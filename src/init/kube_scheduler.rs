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
        let mut scheduler_conf = File::create("/opt/kubernetes/cfg/kube-scheduler.conf")
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
        .expect("Error happened when trying to write `kube-controller-manager.conf`");
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
        let mut scheduler_unit = File::create("/usr/lib/systemd/system/kube-scheduler.service")
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
    // kube-scheduler
    tracing::info!("kube_apiserver phase started");
    tracing::info!("Change working directory into `k8s`");
    let prev_dir = Path::new("/rk8s");
    let work_dir = Path::new("/rk8s/k8s");
    env::set_current_dir(&work_dir).expect("Error happened when trying to change into `k8s`");
    tracing::info!("Changed to {}", env::current_dir().unwrap().display());

    tracing::info!("Generating `kube-scheduler.conf` to /opt/kubernetes/cfg...");
    KubeSchedulerCfg::generate();
    tracing::info!("`kube-scheduler.conf` generated");

    tracing::info!("Start generating `kube-scheduler-csr.json`...");
    let kube_scheduler_csr = KubeSchedulerCsr::from(config);
    let content = serde_json::to_string_pretty(&kube_scheduler_csr)
        .expect("Error happened when trying to serialize `kube-scheduler-csr.json`");
    let mut kube_scheduler_csr_file = File::create("kube-scheduler-csr.json")
        .expect("Error happened when trying to create `kube-scheduler-csr.json`");
    kube_scheduler_csr_file
        .write_all(content.as_bytes())
        .expect(
            "Error happened when trying to write content to `kube-scheduler--csr.json`",
        );
    tracing::info!("`kube-scheduler-csr.json` generated");

    tracing::info!("Generating self-signed kube_controller_manager https certificate...");
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

    tracing::info!("Generating `kubeconfig` using `kubectl`");
    Command::new("kubectl")
        .arg("config")
        .arg("set-cluster")
        .arg("kubernetes")
        .arg("--certificate-authority=/opt/kubernetes/ssl/ca.pem")
        .arg("--embed-certs=true")
        .arg(format!("--server=https://{}:6443", config.instance_ip))
        .arg("--kubeconfig=/opt/kubernetes/cfg/kube-scheduler.kubeconfig")
        .status()
        .expect("Error happened when trying to execute kubectl");
    Command::new("kubectl")
        .arg("config")
        .arg("set-credentials")
        .arg("kube-scheduler")
        .arg("--client-certificate=./kube-scheduler.pem")
        .arg("--client-key=./kube-scheduler-key.pem")
        .arg("--embed-certs=true")
        .arg("--kubeconfig=/opt/kubernetes/cfg/kube-scheduler.kubeconfig")
        .status()
        .expect("Error happened when trying to execute kubectl");
    Command::new("kubectl")
        .arg("config")
        .arg("set-context")
        .arg("default")
        .arg("--cluster=kubernetes")
        .arg("--user=kube-scheduler")
        .arg("--kubeconfig=/opt/kubernetes/cfg/kube-scheduler.kubeconfig")
        .status()
        .expect("Error happened when trying to execute kubectl");
    Command::new("kubectl")
        .arg("config")
        .arg("use-context")
        .arg("default")
        .arg("--kubeconfig=/opt/kubernetes/cfg/kube-scheduler.kubeconfig")
        .status()
        .expect("Error happened when trying to execute kubectl");

    tracing::info!("Generating `kube-scheduler.service` to /usr/lib/systemd/system/");
    KubeSchedulerUnit::generate();
    tracing::info!("`kube-scheduler.service` generated");

    Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .expect("Error happened when trying to reload systemd daemons");
    Command::new("systemctl")
        .arg("enable")
        .arg("kube-scheduler")
        .status()
        .expect("Error happened when trying to enable `kube-scheduler.service`");
    Command::new("systemctl")
        .arg("start")
        .arg("kube-scheduler")
        .status()
        .expect("Error happened when trying to start `kube-scheduler.service`");
    tracing::info!("Master's scheduler is now set");

    env::set_current_dir(&prev_dir).expect("Error happened when trying to change into `/rk8s`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}