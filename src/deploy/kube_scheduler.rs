use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};

struct KubeSchedulerCfg;

impl KubeSchedulerCfg {
    fn generate() {
        let mut scheduler_conf = File::create("to_send/kube-scheduler.conf")
            .expect(
                "Error happened when trying to create kube-scheduler configuration file",
            );

        writeln!(
            &mut scheduler_conf,
r#"KUBE_SCHEDULER_OPTS="--logtostderr=false \
--v=2 \
--log-dir=/opt/kubernetes/logs \
--leader-elect \
--kubeconfig=/opt/kubernetes/cfg/kube-scheduler.kubeconfig \
--bind-address=127.0.0.1"
"#
        )
        .expect("Error happened when trying to write `kube-scheduler.conf`");
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct KubeSchedulerCsr {
    CN: String,
    hosts: Vec<String>,
    key: Key,
    names: Vec<Name>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Key {
    algo: String,
    size: i64,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct Name {
    C: String,
    L: String,
    ST: String,
    O: String,
    OU: String,
}

impl KubeSchedulerCsr {
    fn from(config: &Config) -> KubeSchedulerCsr {
        KubeSchedulerCsr {
            CN: config.kube_scheduler_CN.to_owned(),
            hosts: vec![],
            key: Key {
                algo: config.kube_scheduler_key_algo.to_owned(),
                size: config.kube_scheduler_key_size.to_owned(),
            },
            names: vec![Name {
                C: config.kube_scheduler_names_C.to_owned(),
                L: config.kube_scheduler_names_L.to_owned(),
                ST: config.kube_scheduler_names_ST.to_owned(),
                O: config.kube_scheduler_names_O.to_owned(),
                OU: config.kube_scheduler_names_OU.to_owned(),
            }],
        }
    }
}

struct KubeSchedulerUnit;

impl KubeSchedulerUnit {
    fn generate() {
        let mut scheduler_unit = File::create("to_send/kube-scheduler.service")
            .expect("Error happened when trying to create kube-scheduler unit file");
        let content = r#"[Unit]
Description=Kubernetes Scheduler
Documentation=https://github.com/kubernetes/kubernetes

[Service]
EnvironmentFile=/opt/kubernetes/cfg/kube-scheduler.conf
ExecStart=/opt/kubernetes/bin/kube-scheduler $KUBE_SCHEDULER_OPTS
Restart=on-failure

[Install]
WantedBy=multi-user.target
"#;
        scheduler_unit
            .write_all(content.as_bytes())
            .expect("Error happened when trying to write kube-scheduler unit file");
    }
}

pub fn start(config: &Config) {
    tracing::info!("kube_apiserver phase started");
    tracing::info!("Change working directory into `k8s`");
    let prev_dir = Path::new("/rk8s");
    let work_dir = Path::new("/rk8s/k8s");
    env::set_current_dir(&work_dir).expect("Error happened when trying to change into `k8s`");
    tracing::info!("Changed to {}", env::current_dir().unwrap().display());

    tracing::info!("Start generating `kube-scheduler-csr.json`...");
    let kube_scheduler_csr = KubeSchedulerCsr::from(config);
    let content = serde_json::to_string_pretty(&kube_scheduler_csr)
        .expect("Error happened when trying to serialize `kube-scheduler-csr.json`");
    let mut kube_scheduler_csr_file = File::create("kube-scheduler-csr.json")
        .expect("Error happened when trying to create `kube-scheduler-csr.json`");
    kube_scheduler_csr_file
        .write_all(content.as_bytes())
        .expect(
            "Error happened when trying to write content to `kube-scheduler-csr.json`",
        );
    tracing::info!("`kube-scheduler-csr.json` generated");

    tracing::info!("Generating self-signed kube_scheduler certificate...");
    let cfssl_kube_scheduler = Command::new("cfssl")
        .arg("gencert")
        .arg("-ca=ca.pem")
        .arg("-ca-key=ca-key.pem")
        .arg("-config=ca-config.json")
        .arg("-profile=kubernetes")
        .arg("kube-scheduler-csr.json")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Error happened when trying to execute `cfssl` command");
    Command::new("cfssljson")
        .arg("-bare")
        .arg("kube-scheduler")
        .stdin(Stdio::from(cfssl_kube_scheduler.stdout.unwrap()))
        .status()
        .expect("Error happened when trying to execute `cfssljson`");
    tracing::info!("Self-signed kube_scheduler CA certificate generated");

    tracing::info!("Generating `kube-scheduler.conf` to to_send/...");
    KubeSchedulerCfg::generate();
    tracing::info!("`kube-scheduler.conf` generated");

    tracing::info!("Generating `kube-scheduler.service` to /usr/lib/systemd/system/");
    KubeSchedulerUnit::generate();
    tracing::info!("`kube-scheduler.service` generated");

    for (ip, name) in &config.instance_hosts {
        if name.contains("master") {
            Command::new("scp")
                .arg("kube-scheduler.pem")
                .arg("kube-scheduler-key.pem")
                .arg(format!("root@{}:/opt/kubernetes/ssl", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Certificates sent to master on {}", ip);

            Command::new("scp")
                .arg("to_send/kube-scheduler.conf")
                .arg(format!("root@{}:/opt/kubernetes/cfg", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Configurations sent to master on {}", ip);

            Command::new("scp")
                .arg("to_send/kube-scheduler.service")
                .arg(format!("root@{}:/usr/lib/systemd/system/", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Systemd service sent to master on {}", ip);

            // Generate kubeconfig on remote master.
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg(format!("kubectl config set-cluster kubernetes --certificate-authority=/opt/kubernetes/ssl/ca.pem --embed-certs=true \
                --server=https://{}:6443 --kubeconfig=/opt/kubernetes/cfg/kube-scheduler.kubeconfig", ip))
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config set-credentials kube-scheduler --client-certificate=/opt/kubernetes/ssl/kube-scheduler.pem \
                --client-key=/opt/kubernetes/ssl/kube-scheduler-key.pem --embed-certs=true --kubeconfig=/opt/kubernetes/cfg/kube-scheduler.kubeconfig")
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config set-context default --cluster=kubernetes --user=kube-scheduler \
                --kubeconfig=/opt/kubernetes/cfg/kube-scheduler.kubeconfig")
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config use-context default --kubeconfig=/opt/kubernetes/cfg/kube-scheduler.kubeconfig")
                .status()
                .expect("Error happened when trying to execute kubectl");

            // Starting scheduler...
            tracing::info!("kube-scheduler installed on {}, starting...", name);
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("systemctl daemon-reload")
                .status()
                .expect("Error happened when trying to reload systemd daemons");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("systemctl start kube-scheduler")
                .status()
                .expect("Error happened when trying to start scheduler");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("systemctl enable kube-scheduler")
                .status()
                .expect("Error happened when trying to enable scheduler");
            tracing::info!("kube-scheduler started on {}", ip);
        }
    }

    env::set_current_dir(&prev_dir).expect("Error happened when trying to change into `/rk8s`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}