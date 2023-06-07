use std::process::Command;
use std::path::{Path, PathBuf};
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;

use crate::config::Config;

struct DockerCfg;

impl DockerCfg {
    fn generate() {
        // Send to /etc/docker/daemon.json
        let mut docker_cfg = File::create("to_send/daemon.json")
            .expect("Error happened when trying to create docker config file");
        // Set youki (pre-built) as the default runtime.
        let content = r#"{
    "default-runtime": "youki",
    "runtimes": {
        "youki": {
            "path": "/usr/bin/youki"
        }
    },
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
        let mut docker_unit = File::create("to_send/docker.service")
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
    // Deploy docker to all hosts according to their name.
    // Docker does not distinguish masters or workers.
    tracing::info!("Preparing mutual .json, .service and docker binaries...");
    tracing::info!("Change working directory into `docker`");
    let prev_dir = Path::new("/rk8s");
    let work_dir = Path::new("/rk8s/docker");
    env::set_current_dir(&work_dir).expect("Error happened when trying to change into `etcd`");
    tracing::info!("Changed to {}", env::current_dir().unwrap().display());

    // Prepare directory to be sent.
    let path = PathBuf::from("to_send/");
    check_dir_exist_or_create(path);

    tracing::info!("untaring docker binaries...");
    Command::new("tar")
        .arg("-zxf")
        .arg("/rk8s/preparation/docker-20.10.9.tgz")
        .status()
        .expect("Error happened when trying to untar `docker` executable");

    tracing::info!("Generating docker.service file to to_send/");
    DockerUnit::generate();
    tracing::info!("docker.service generated");

    tracing::info!("Generating daemon.json to to_send/");
    DockerCfg::generate();
    tracing::info!("daemon.json generated");

    for (ip, name) in &config.instance_hosts {
        tracing::info!("Found instance {} on {},", name, ip);

        Command::new("scp")
            .arg("docker/containerd")
            .arg("docker/containerd-shim")
            .arg("docker/containerd-shim-runc-v2")
            .arg("docker/ctr")
            .arg("docker/docker")
            .arg("docker/dockerd")
            .arg("docker/docker-init")
            .arg("docker/docker-proxy")
            .arg("docker/runc")
            // Send youki along with the process
            .arg("/rk8s/preparation/youki")
            .arg(format!("root@{}:/usr/bin", ip))
            .status()
            .expect("Error happened when trying to send files to other nodes");

        Command::new("ssh")
            .arg(format!("root@{}", ip))
            .arg("mkdir /etc/docker")
            .status()
            .expect("Error happened when trying to create config directory on other nodes");

        Command::new("scp")
            .arg("to_send/daemon.json")
            .arg(format!("root@{}:/etc/docker", ip))
            .status()
            .expect("Error happened when trying to send files to other nodes");

        Command::new("scp")
            .arg("to_send/docker.service")
            .arg(format!("root@{}:/usr/lib/systemd/system/", ip))
            .status()
            .expect("Error happened when trying to send files to other nodes");

        tracing::info!("Docker installed on {}, starting...", name);
        Command::new("ssh")
            .arg(format!("root@{}", ip))
            .arg("systemctl daemon-reload")
            .status()
            .expect("Error happened when trying to reload systemd daemons");
        Command::new("ssh")
            .arg(format!("root@{}", ip))
            .arg("systemctl start docker")
            .status()
            .expect("Error happened when trying to start docker");
        Command::new("ssh")
            .arg(format!("root@{}", ip))
            .arg("systemctl enable docker")
            .status()
            .expect("Error happened when trying to enable docker");
        tracing::info!("Docker started on {}", ip);
    }

    env::set_current_dir(&prev_dir).expect("Error happened when trying to change into `etcd`");
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
