use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use crate::config::Config;

struct DockerCfg;

impl DockerCfg {
    fn generate() {
        let mut docker_cfg = File::create("/etc/docker/daemon.json")
            .expect("Error happened when trying to create docker config file");
        let content = r#"{
    "exec-opts": ["native.cgroupdriver=systemd"],
    "log-driver": "json-file",
    "log-opts": {
        "max-size": "100m"
    },
    "storage-driver": "overlay2"
}
"#;
        docker_cfg
            .write_all(content.as_bytes())
            .expect("Error happened when trying to write docker unit file");
    }
}

struct DockerUnit;

impl DockerUnit {
    fn generate() {
        let mut docker_unit = File::create("/usr/lib/systemd/system/docker.service")
            .expect("Error happened when trying to create docker unit file");
        let content = r#"[Unit]
Description=Docker Application Container Engine
Documentation=https://docs.docker.com
After=network-online.target firewalld.service
Wants=network-online.target

[Service]
Type=notify
ExecStart=/usr/bin/dockerd
ExecReload=/bin/kill -s HUP $MAINPID
LimitNOFILE=infinity
LimitNPROC=infinity
LimitCORE=infinity
TimeoutStartSec=0
Delegate=yes
KillMode=process
Restart=on-failure
StartLimitBurst=3
StartLimitInterval=60s

[Install]
WantedBy=multi-user.target
"#;
        docker_unit
            .write_all(content.as_bytes())
            .expect("Error happened when trying to write docker unit file");
    }
}

pub fn start(config: &Config) {
    tracing::info!("Start installing docker");
    tracing::info!("Downloading docker binary from docker_URL");
    Command::new("curl")
        .arg("-L")
        .arg(&config.docker_url)
        .arg("-o")
        .arg("docker/docker-20.10.9.tgz")
        .status()
        .expect("Error happened when trying to download `etcd`");

    if PathBuf::from("/rk8s/docker/docker-20.10.9.tgz").is_file() {
        tracing::info!("docker downloaded");

        tracing::info!("untaring downloaded file");
        Command::new("tar")
            .arg("-zxf")
            .arg("docker/docker-20.10.9.tgz")
            .arg("--directory")
            .arg("docker")
            .status()
            .expect("Error happened when trying to untar `docker` executable");

        Command::new("cp")
            .arg("docker/docker/containerd")
            .arg("docker/docker/containerd-shim")
            .arg("docker/docker/containerd-shim-runc-v2")
            .arg("docker/docker/ctr")
            .arg("docker/docker/docker")
            .arg("docker/docker/dockerd")
            .arg("docker/docker/docker-init")
            .arg("docker/docker/docker-proxy")
            .arg("docker/docker/runc")
            .arg("/usr/bin")
            .status()
            .expect("Error happened when trying to copy `docker` executable to `/usr/bin`");

        tracing::info!("Generating docker.service file to /usr/lib/systemd/system/");
        DockerUnit::generate();
        tracing::info!("docker.service generated");

        let config_dir = PathBuf::from("/etc/docker");
        check_dir_exist_or_create(config_dir);
        tracing::info!("Generating daemon.json to /etc/docker/");
        DockerCfg::generate();
        tracing::info!("daemon.json generated");

        tracing::info!("docker is ready");
    } else {
        tracing::error!("docker not downloaded, please try again");
    }

    Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .expect("Error happened when trying to reload systemd daemons");
    Command::new("systemctl")
        .arg("enable")
        .arg("docker")
        .status()
        .expect("Error happened when trying to enable `docker.service`");
    Command::new("systemctl")
        .arg("start")
        .arg("docker")
        .status()
        .expect("Error happened when trying to start `docker.service`");

    tracing::info!("docker is now installed");
}

fn check_dir_exist_or_create(path: PathBuf) {
    if !path.is_dir() {
        fs::create_dir_all(path).expect("Error happened when trying to create path");
    }
}
