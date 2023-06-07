use crate::config::Config;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn start(config: &Config) {
    tracing::info!("Start downloading kubernetes...");
    // Retrieve file name from URL.
    let filename = "kubernetes-server-linux-amd64";
    Command::new("curl")
        .arg("-L")
        .arg(&config.kubernetes_url)
        .arg("-o")
        .arg(format!("k8s/{}.tar.gz", filename))
        .status()
        .expect("Error happened when trying to download `kubernetes`");
    if PathBuf::from(format!("/rk8s/k8s/{}.tar.gz", filename)).is_file() {
        tracing::info!("kubernetes downloaded");

        tracing::info!("untaring downloaded file");
        Command::new("tar")
            .arg("-zxf")
            .arg(format!("k8s/{}.tar.gz", filename))
            .arg("--directory")
            .arg("k8s")
            .status()
            .expect("Error happened when trying to untar `kubernetes` executable");

        let cfg_path = PathBuf::from("/opt/kubernetes/cfg");
        check_dir_exist_or_create(cfg_path);
        let bin_path = PathBuf::from("/opt/kubernetes/bin");
        check_dir_exist_or_create(bin_path);
        let ssl_path = PathBuf::from("/opt/kubernetes/ssl");
        check_dir_exist_or_create(ssl_path);
        let logs_path = PathBuf::from("/opt/kubernetes/logs");
        check_dir_exist_or_create(logs_path);

        Command::new("cp")
            .arg("k8s/kubernetes/server/bin/kube-apiserver")
            .arg("k8s/kubernetes/server/bin/kube-scheduler")
            .arg("k8s/kubernetes/server/bin/kube-controller-manager")
            .arg("k8s/kubernetes/server/bin/kube-proxy")
            .arg("k8s/kubernetes/server/bin/kubelet")
            .arg("/opt/kubernetes/bin")
            .status()
            .expect("Error happened when trying to copy `kubernetes` executable to `/opt/kubernetes/bin`");
        
        // Prepare content to be sent to worker nodes.
        let kubernetes_bin = PathBuf::from("to_send/kubernetes/bin");
        check_dir_exist_or_create(kubernetes_bin);
        let kubernetes_cfg= PathBuf::from("to_send/kubernetes/cfg");
        check_dir_exist_or_create(kubernetes_cfg);
        let kubernetes_ssl= PathBuf::from("to_send/kubernetes/ssl");
        check_dir_exist_or_create(kubernetes_ssl);
        let kubernetes_logs= PathBuf::from("to_send/kubernetes/logs");
        check_dir_exist_or_create(kubernetes_logs);
        Command::new("cp")
            .arg("k8s/kubernetes/server/bin/kube-proxy")
            .arg("k8s/kubernetes/server/bin/kubelet")
            .arg("to_send/kubernetes/bin")
            .status()
            .expect("Error happened when trying to copy `kubernetes` executable to `/opt/kubernetes/bin`");

        Command::new("cp")
            .arg("k8s/kubernetes/server/bin/kube-apiserver")
            .arg("k8s/kubernetes/server/bin/kube-scheduler")
            .arg("k8s/kubernetes/server/bin/kube-controller-manager")
            .arg("k8s/kubernetes/server/bin/kube-proxy")
            .arg("k8s/kubernetes/server/bin/kubelet")
            .arg("/opt/kubernetes/bin")
            .status()
            .expect("Error happened when trying to copy `kubernetes` executable to `/opt/kubernetes/bin`");
        Command::new("cp")
            .arg("k8s/kubernetes/server/bin/kubectl")
            .arg("/usr/bin")
            .status()
            .expect("Error happened when trying to copy `kubernetes` executable to `/usr/bin`");

        // tracing::info!("kubernetes is ready");
    } else {
        tracing::error!("kubernetes not downloaded, please try again");
    }
}

fn check_dir_exist_or_create(path: PathBuf) {
    if !path.is_dir() {
        fs::create_dir_all(path).expect("Error happened when trying to create path");
    }
}
