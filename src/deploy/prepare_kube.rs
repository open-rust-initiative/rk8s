use crate::config::Config;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn start(config: &Config) {
    tracing::info!("Start preparing kubernetes binaries...");
    tracing::info!("Change working directory into `k8s`");
    let prev_dir = Path::new("/rk8s");
    let work_dir = Path::new("/rk8s/k8s");
    env::set_current_dir(work_dir).expect("Error happened when trying to change into `k8s`");
    tracing::info!("Changed to {}", env::current_dir().unwrap().display());

    // Prepare directory to be sent.
    let cfg_path = PathBuf::from("to_send/kubernetes/cfg");
    check_dir_exist_or_create(cfg_path);
    let bin_path = PathBuf::from("to_send/kubernetes/bin");
    check_dir_exist_or_create(bin_path);
    let ssl_path = PathBuf::from("to_send/kubernetes/ssl");
    check_dir_exist_or_create(ssl_path);
    let logs_path = PathBuf::from("to_send/kubernetes/logs");
    check_dir_exist_or_create(logs_path);

    tracing::info!("Untaring prepared kubernetes binary file");
    Command::new("tar")
        .arg("-zxf")
        .arg("/rk8s/preparation/kubernetes-server-linux-amd64.tar.gz")
        .status()
        .expect("Error happened when trying to untar `kubernetes` executable");

    tracing::info!("Copying binaries to to_send/");
    Command::new("cp")
        // Needed by master.
        .arg("kubernetes/server/bin/kube-apiserver")
        .arg("kubernetes/server/bin/kube-controller-manager")
        .arg("kubernetes/server/bin/kube-scheduler")
        .arg("kubernetes/server/bin/kubectl")
        // Needed by worker.
        .arg("kubernetes/server/bin/kubelet")
        .arg("kubernetes/server/bin/kube-proxy")
        .arg("to_send/")
        .status()
        .expect("Error happened when trying to copy binaries to `to_send/etcd/bin`");
    tracing::info!("Binaries prepared");

    for (ip, name) in &config.instance_hosts {
        tracing::info!(
            "Found node: {} on {}, sending kubernetes skeleton and worker binaries...",
            name,
            ip
        );
        Command::new("scp")
            .arg("-r")
            .arg("to_send/kubernetes")
            .arg(format!("root@{}:/opt/", ip))
            .status()
            .expect("Error happened when trying to send files to other nodes");
        Command::new("scp")
            .arg("to_send/kubelet")
            .arg("to_send/kube-proxy")
            .arg(format!("root@{}:/opt/kubernetes/bin", ip))
            .status()
            .expect("Error happened when trying to send files to other nodes");

        let path = PathBuf::from("to_send").join(ip);
        if name.contains("master") {
            // Only master need apiserver.
            let apiserver_path = path.join("apiserver");
            check_dir_exist_or_create(apiserver_path);

            // Only master need controller manager.
            let controller_path = path.join("controller_manager");
            check_dir_exist_or_create(controller_path);

            // Only master need scheduler.
            let scheduler_path = path.join("scheduler");
            check_dir_exist_or_create(scheduler_path);

            tracing::info!("Found master: {} on {}, sending kubernetes apiserver, controller-manager, scheduler, kubectl...", name, ip);
            Command::new("scp")
                .arg("to_send/kube-apiserver")
                .arg("to_send/kube-controller-manager")
                .arg("to_send/kube-scheduler")
                .arg(format!("root@{}:/opt/kubernetes/bin", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            Command::new("scp")
                .arg("to_send/kubectl")
                .arg(format!("root@{}:/usr/bin", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
        }

        let kubelet_path = path.join("kubelet");
        check_dir_exist_or_create(kubelet_path);

        let kube_proxy_path = path.join("kube_proxy");
        check_dir_exist_or_create(kube_proxy_path);
    }

    env::set_current_dir(prev_dir).expect("Error happened when trying to change into `/rk8s`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}

fn check_dir_exist_or_create(path: PathBuf) {
    if !path.is_dir() {
        fs::create_dir_all(path).expect("Error happened when trying to create path");
    }
}
