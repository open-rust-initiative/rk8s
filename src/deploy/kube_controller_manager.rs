use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};

struct KubeControllerManagerCfg;

impl KubeControllerManagerCfg {
    fn generate() {
        let mut controller_conf = File::create("to_send/kube-controller-manager.conf")
            .expect(
                "Error happened when trying to create kube-controller-manager configuration file",
            );

        writeln!(
            &mut controller_conf,
r#"KUBE_CONTROLLER_MANAGER_OPTS="--logtostderr=false \
--v=2 \
--log-dir=/opt/kubernetes/logs \
--leader-elect=true \
--kubeconfig=/opt/kubernetes/cfg/kube-controller-manager.kubeconfig \
--bind-address=127.0.0.1 \
--allocate-node-cidrs=true \
--cluster-cidr=10.244.0.0/16 \
--service-cluster-ip-range=10.0.0.0/24 \
--cluster-signing-cert-file=/opt/kubernetes/ssl/ca.pem \
--cluster-signing-key-file=/opt/kubernetes/ssl/ca-key.pem  \
--root-ca-file=/opt/kubernetes/ssl/ca.pem \
--service-account-private-key-file=/opt/kubernetes/ssl/ca-key.pem \
--cluster-signing-duration=87600h0m0s"
"#
        )
        .expect("Error happened when trying to write `kube-controller-manager.conf`");
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct KubeControllerManagerCsr {
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

impl KubeControllerManagerCsr {
    fn from(config: &Config) -> KubeControllerManagerCsr {
        KubeControllerManagerCsr {
            CN: config.kube_controller_manager_CN.to_owned(),
            hosts: vec![],
            key: Key {
                algo: config.kube_controller_manager_key_algo.to_owned(),
                size: config.kube_controller_manager_key_size.to_owned(),
            },
            names: vec![Name {
                C: config.kube_controller_manager_names_C.to_owned(),
                L: config.kube_controller_manager_names_L.to_owned(),
                ST: config.kube_controller_manager_names_ST.to_owned(),
                O: config.kube_controller_manager_names_O.to_owned(),
                OU: config.kube_controller_manager_names_OU.to_owned(),
            }],
        }
    }
}

struct KubeControllerManagerUnit;

impl KubeControllerManagerUnit {
    fn generate() {
        let mut kube_controller_unit = File::create("to_send/kube-controller-manager.service")
            .expect("Error happened when trying to create kube-controller-manager unit file");
        let content = r#"[Unit]
Description=Kubernetes Controller Manager
Documentation=https://github.com/kubernetes/kubernetes

[Service]
EnvironmentFile=/opt/kubernetes/cfg/kube-controller-manager.conf
ExecStart=/opt/kubernetes/bin/kube-controller-manager $KUBE_CONTROLLER_MANAGER_OPTS
Restart=on-failure

[Install]
WantedBy=multi-user.target
"#;
        kube_controller_unit
            .write_all(content.as_bytes())
            .expect("Error happened when trying to write kube-controller manager unit file");
    }
}

pub fn start(config: &Config) {
    tracing::info!("kube_controller_manager phase started");
    tracing::info!("Change working directory into `k8s`");
    let prev_dir = Path::new("/rk8s");
    let work_dir = Path::new("/rk8s/k8s");
    env::set_current_dir(&work_dir).expect("Error happened when trying to change into `k8s`");
    tracing::info!("Changed to {}", env::current_dir().unwrap().display());

    tracing::info!("Start generating `kube-controller-manager-csr.json`...");
    let kube_controller_manager_csr = KubeControllerManagerCsr::from(config);
    let content = serde_json::to_string_pretty(&kube_controller_manager_csr)
        .expect("Error happened when trying to serialize `kube-controller-manager-csr.json`");
    let mut kube_controller_manager_csr_file = File::create("kube-controller-manager-csr.json")
        .expect("Error happened when trying to create `kube-controller-manager-csr.json`");
    kube_controller_manager_csr_file
        .write_all(content.as_bytes())
        .expect(
            "Error happened when trying to write content to `kube-controller-manager-csr.json`",
        );
    tracing::info!("`kube-controller-manager-csr.json` generated");

    tracing::info!("Generating self-signed kube_controller_manager https certificate...");
    let cfssl_kube_controller = Command::new("cfssl")
        .arg("gencert")
        .arg("-ca=ca.pem")
        .arg("-ca-key=ca-key.pem")
        .arg("-config=ca-config.json")
        .arg("-profile=kubernetes")
        .arg("kube-controller-manager-csr.json")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Error happened when trying to execute `cfssl` command");
    Command::new("cfssljson")
        .arg("-bare")
        .arg("kube-controller-manager")
        .stdin(Stdio::from(cfssl_kube_controller.stdout.unwrap()))
        .status()
        .expect("Error happened when trying to execute `cfssljson`");
    tracing::info!("Self-signed kube_controller_manager CA certificate generated");

    tracing::info!("Generating `kube-controller-manager.conf` to to_send/...");
    KubeControllerManagerCfg::generate();
    tracing::info!("`kube-controller-manager.conf` generated");

    tracing::info!("Generating `kube-controller-manager.service` to to_send/");
    KubeControllerManagerUnit::generate();
    tracing::info!("`kube-controller-manager.service` generated");

    for (ip, name) in &config.instance_hosts {
        if name.contains("master") {
            Command::new("scp")
                .arg("kube-controller-manager.pem")
                .arg("kube-controller-manager-key.pem")
                .arg(format!("root@{}:/opt/kubernetes/ssl", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Certificates sent to master on {}", ip);

            Command::new("scp")
                .arg("to_send/kube-controller-manager.conf")
                .arg(format!("root@{}:/opt/kubernetes/cfg", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Configurations sent to master on {}", ip);

            Command::new("scp")
                .arg("to_send/kube-controller-manager.service")
                .arg(format!("root@{}:/usr/lib/systemd/system/", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Systemd service sent to master on {}", ip);

            // Generate kubeconfig on remote master.
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg(format!("kubectl config set-cluster kubernetes --certificate-authority=/opt/kubernetes/ssl/ca.pem --embed-certs=true \
                --server=https://{}:6443 --kubeconfig=/opt/kubernetes/cfg/kube-controller-manager.kubeconfig", ip))
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config set-credentials kube-controller-manager --client-certificate=/opt/kubernetes/ssl/kube-controller-manager.pem \
                --client-key=/opt/kubernetes/ssl/kube-controller-manager-key.pem --embed-certs=true --kubeconfig=/opt/kubernetes/cfg/kube-controller-manager.kubeconfig")
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config set-context default --cluster=kubernetes --user=kube-controller-manager \
                --kubeconfig=/opt/kubernetes/cfg/kube-controller-manager.kubeconfig")
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config use-context default --kubeconfig=/opt/kubernetes/cfg/kube-controller-manager.kubeconfig")
                .status()
                .expect("Error happened when trying to execute kubectl");

            // Starting controller manager...
            tracing::info!("kube-controller-manager installed on {}, starting...", name);
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("systemctl daemon-reload")
                .status()
                .expect("Error happened when trying to reload systemd daemons");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("systemctl start kube-controller-manager")
                .status()
                .expect("Error happened when trying to start controller-manager");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("systemctl enable kube-controller-manager")
                .status()
                .expect("Error happened when trying to enable controller-manager");
            tracing::info!("kube-controller-manager started on {}", ip);
        }
    }

    env::set_current_dir(&prev_dir).expect("Error happened when trying to change into `/rk8s`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}