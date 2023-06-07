use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use std::fs::File;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

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
            CN: config.kube_ctl_CN.to_owned(),
            hosts: vec![],
            key: Key {
                algo: config.kube_ctl_key_algo.to_owned(),
                size: config.kube_ctl_key_size.to_owned(),
            },
            names: vec![Name {
                C: config.kube_ctl_names_C.to_owned(),
                L: config.kube_ctl_names_L.to_owned(),
                ST: config.kube_ctl_names_ST.to_owned(),
                O: config.kube_ctl_names_O.to_owned(),
                OU: config.kube_ctl_names_OU.to_owned(),
            }],
        }
    }
}

pub fn start(config: &Config) {
    // kube-ctl
    tracing::info!("kube_apiserver phase started");
    tracing::info!("Change working directory into `k8s`");
    let prev_dir = Path::new("/rk8s");
    let work_dir = Path::new("/rk8s/k8s");
    env::set_current_dir(&work_dir).expect("Error happened when trying to change into `k8s`");
    tracing::info!("Changed to {}", env::current_dir().unwrap().display());

    tracing::info!("Start generating `admin-csr.json`...");
    let admin_csr = KubeSchedulerCsr::from(config);
    let content = serde_json::to_string_pretty(&admin_csr)
        .expect("Error happened when trying to serialize `admin-csr.json`");
    let mut admin_csr_file = File::create("admin-csr.json")
        .expect("Error happened when trying to create `admin-csr.json`");
    admin_csr_file
        .write_all(content.as_bytes())
        .expect(
            "Error happened when trying to write content to `admin-csr.json`",
        );
    tracing::info!("`admin-csr.json` generated");

    tracing::info!("Generating self-signed kubectl https certificate...");
    let cfssl_kube_scheduler = Command::new("cfssl")
        .arg("gencert")
        .arg("-ca=ca.pem")
        .arg("-ca-key=ca-key.pem")
        .arg("-config=ca-config.json")
        .arg("-profile=kubernetes")
        .arg("admin-csr.json")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Error happened when trying to execute `cfssl` command");
    Command::new("cfssljson")
        .arg("-bare")
        .arg("admin")
        .stdin(Stdio::from(cfssl_kube_scheduler.stdout.unwrap()))
        .status()
        .expect("Error happened when trying to execute `cfssljson`");
    tracing::info!("Self-signed kubectl CA certificate generated");

    // Check /root/.kube directory exist or not
    let dot_kube= Path::new("/root/.kube");
    if !dot_kube.is_dir() {
        fs::create_dir_all("/root/.kube")
            .expect("Error happened when trying to create `.kube` directory");
    }

    tracing::info!("Generating `kubeconfig` using `kubectl`");
    Command::new("kubectl")
        .arg("config")
        .arg("set-cluster")
        .arg("kubernetes")
        .arg("--certificate-authority=/opt/kubernetes/ssl/ca.pem")
        .arg("--embed-certs=true")
        .arg(format!("--server=https://{}:6443", config.instance_ip))
        .arg("--kubeconfig=/root/.kube/config")
        .status()
        .expect("Error happened when trying to execute kubectl");
    Command::new("kubectl")
        .arg("config")
        .arg("set-credentials")
        .arg("cluster-admin")
        .arg("--client-certificate=./admin.pem")
        .arg("--client-key=./admin-key.pem")
        .arg("--embed-certs=true")
        .arg("--kubeconfig=/root/.kube/config")
        .status()
        .expect("Error happened when trying to execute kubectl");
    Command::new("kubectl")
        .arg("config")
        .arg("set-context")
        .arg("default")
        .arg("--cluster=kubernetes")
        .arg("--user=cluster-admin")
        .arg("--kubeconfig=/root/.kube/config")
        .status()
        .expect("Error happened when trying to execute kubectl");
    Command::new("kubectl")
        .arg("config")
        .arg("use-context")
        .arg("default")
        .arg("--kubeconfig=/root/.kube/config")
        .status()
        .expect("Error happened when trying to execute kubectl");

    Command::new("kubectl")
        .arg("create")
        .arg("clusterrolebinding")
        .arg("kubelet-bootstrap")
        .arg("--clusterrole=system:node-bootstrapper")
        .arg("--user=kubelet-bootstrap")
        .status()
        .expect("Error happened when trying to execute kubectl");
    tracing::info!("Master's scheduler is now set");

    env::set_current_dir(&prev_dir).expect("Error happened when trying to change into `/rk8s`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}