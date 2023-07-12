use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct KubectlCsr {
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

impl KubectlCsr {
    fn from(config: &Config) -> KubectlCsr {
        KubectlCsr {
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
    tracing::info!("kube_apiserver phase started");
    tracing::info!("Change working directory into `k8s`");
    let prev_dir = Path::new("/rk8s");
    let work_dir = Path::new("/rk8s/k8s");
    env::set_current_dir(work_dir).expect("Error happened when trying to change into `k8s`");
    tracing::info!("Changed to {}", env::current_dir().unwrap().display());

    tracing::info!("Start generating `admin-csr.json`...");
    let admin_csr = KubectlCsr::from(config);
    let content = serde_json::to_string_pretty(&admin_csr)
        .expect("Error happened when trying to serialize `admin-csr.json`");
    let mut admin_csr_file = File::create("admin-csr.json")
        .expect("Error happened when trying to create `admin-csr.json`");
    admin_csr_file
        .write_all(content.as_bytes())
        .expect("Error happened when trying to write content to `admin-csr.json`");
    tracing::info!("`admin-csr.json` generated");

    tracing::info!("Generating self-signed kubectl https certificate...");
    let cfssl_kubectl = Command::new("cfssl")
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
        .stdin(Stdio::from(cfssl_kubectl.stdout.unwrap()))
        .status()
        .expect("Error happened when trying to execute `cfssljson`");
    tracing::info!("Self-signed kubectl CA certificate generated");

    for (ip, name) in &config.instance_hosts {
        if name.contains("master") {
            Command::new("scp")
                .arg("admin.pem")
                .arg("admin-key.pem")
                .arg(format!("root@{}:/opt/kubernetes/ssl", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Certificates sent to master on {}", ip);

            // Create .kube directory under /root
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("mkdir /root/.kube")
                .status()
                .expect("Error happened when trying to create directory");

            // Generate kubeconfig on remote master.
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg(format!("kubectl config set-cluster kubernetes --certificate-authority=/opt/kubernetes/ssl/ca.pem --embed-certs=true \
                --server=https://{}:6443 --kubeconfig=/root/.kube/config", ip))
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config set-credentials cluster-admin --client-certificate=/opt/kubernetes/ssl/admin.pem \
                --client-key=/opt/kubernetes/ssl/admin-key.pem --embed-certs=true --kubeconfig=/root/.kube/config")
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg(
                    "kubectl config set-context default --cluster=kubernetes --user=cluster-admin \
                --kubeconfig=/root/.kube/config",
                )
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config use-context default --kubeconfig=/root/.kube/config")
                .status()
                .expect("Error happened when trying to execute kubectl");

            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl create clusterrolebinding kubelet-bootstrap --clusterrole=system:node-bootstrapper --user=kubelet-bootstrap")
                .status()
                .expect("Error happened when trying to execute kubectl");

            tracing::info!("kubectl is now ready on {}", ip);
        }
    }

    env::set_current_dir(prev_dir).expect("Error happened when trying to change into `/rk8s`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}
