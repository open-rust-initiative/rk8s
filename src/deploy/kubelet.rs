use crate::config::Config;
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::{thread, time};

struct KubeletCfg;

impl KubeletCfg {
    fn generate(current_ip: &String, current_name: &String) {
        let mut kubelet_conf = File::create(format!("to_send/{}/kubelet/kubelet.conf", current_ip))
            .expect("Error happened when trying to create kubelet configuration file");

        writeln!(
            &mut kubelet_conf,
            r#"KUBELET_OPTS="--logtostderr=false \
--v=2 \
--log-dir=/opt/kubernetes/logs \"#,
        )
        .expect("Error happened when trying to write `kubelet.conf`");
        writeln!(&mut kubelet_conf, "--hostname-override={} \\", current_name)
            .expect("Error happened when trying to write `kubelet.conf`");
        writeln!(
            &mut kubelet_conf,
            r#"--network-plugin=cni \
--kubeconfig=/opt/kubernetes/cfg/kubelet.kubeconfig \
--bootstrap-kubeconfig=/opt/kubernetes/cfg/bootstrap.kubeconfig \
--config=/opt/kubernetes/cfg/kubelet-config.yml \
--cert-dir=/opt/kubernetes/ssl \
--pod-infra-container-image=registry.cn-hangzhou.aliyuncs.com/google-containers/pause-amd64:3.0"
"#
        )
        .expect("Error happened when trying to write `kubelet.conf`");
    }
}

struct KubeletConfig;

impl KubeletConfig {
    fn generate() {
        let mut kubelet_config = File::create("to_send/kubelet-config.yml")
            .expect("Error happened when trying to create kubelet configuration file");

        writeln!(
            &mut kubelet_config,
            r#"kind: KubeletConfiguration
apiVersion: kubelet.config.k8s.io/v1beta1
address: 0.0.0.0
port: 10250
readOnlyPort: 10255
cgroupDriver: systemd
clusterDNS:
- 10.0.0.2
clusterDomain: cluster.local 
failSwapOn: false
authentication:
  anonymous:
    enabled: false
  webhook:
    cacheTTL: 2m0s
    enabled: true
  x509:
    clientCAFile: /opt/kubernetes/ssl/ca.pem 
authorization:
  mode: Webhook
  webhook:
    cacheAuthorizedTTL: 5m0s
    cacheUnauthorizedTTL: 30s
evictionHard:
  imagefs.available: 15%
  memory.available: 100Mi
  nodefs.available: 10%
  nodefs.inodesFree: 5%
maxOpenFiles: 1000000
maxPods: 110
"#,
        )
        .expect("Error happened when trying to write `kubelet-config.yml`");
    }
}

struct KubeletUnit;

impl KubeletUnit {
    fn generate() {
        let mut kubelet_unit = File::create("to_send/kubelet.service")
            .expect("Error happened when trying to create kubelet unit file");
        let content = r#"[Unit]
Description=Kubernetes Kubelet
After=docker.service

[Service]
EnvironmentFile=/opt/kubernetes/cfg/kubelet.conf
ExecStart=/opt/kubernetes/bin/kubelet $KUBELET_OPTS
Restart=on-failure
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
"#;
        kubelet_unit
            .write_all(content.as_bytes())
            .expect("Error happened when trying to write kubelet unit file");
    }
}

pub fn start(config: &Config) {
    // kube-scheduler
    tracing::info!("kube_apiserver phase started");
    tracing::info!("Change working directory into `k8s`");
    let prev_dir = Path::new("/rk8s");
    let work_dir = Path::new("/rk8s/k8s");
    env::set_current_dir(work_dir).expect("Error happened when trying to change into `k8s`");
    tracing::info!("Changed to {}", env::current_dir().unwrap().display());

    tracing::info!("Generating `kubelet-config.yml` to to_send/...");
    KubeletConfig::generate();
    tracing::info!("`kubelet-config.yml` generated");

    tracing::info!("Generating `kubelet.service` to to_send/...");
    KubeletUnit::generate();
    tracing::info!("`kubelet.service` generated");

    let mut master_ip = "";
    for (ip, name) in &config.instance_hosts {
        tracing::info!("Found instance {} on {},", name, ip);
        if name.contains("master") {
            master_ip = ip;
            tracing::info!("Generating `kubelet.conf`...");
            KubeletCfg::generate(ip, name);
            tracing::info!("`kubelet.conf` generated");

            Command::new("scp")
                .arg(format!("to_send/{}/kubelet/kubelet.conf", ip))
                .arg("to_send/kubelet-config.yml")
                .arg(format!("root@{}:/opt/kubernetes/cfg", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Configurations sent to master on {}", ip);

            Command::new("scp")
                .arg("to_send/kubelet.service")
                .arg(format!("root@{}:/usr/lib/systemd/system/", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Systemd service sent to master on {}", ip);

            // Generate kubeconfig on remote master.
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg(format!("kubectl config set-cluster kubernetes --certificate-authority=/opt/kubernetes/ssl/ca.pem --embed-certs=true \
                --server=https://{}:6443 --kubeconfig=/opt/kubernetes/cfg/bootstrap.kubeconfig", ip))
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config set-credentials kubelet-bootstrap --token=4136692876ad4b01bb9dd0988480ebba \
                --kubeconfig=/opt/kubernetes/cfg/bootstrap.kubeconfig")
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config set-context default --cluster=kubernetes --user=kubelet-bootstrap \
                --kubeconfig=/opt/kubernetes/cfg/bootstrap.kubeconfig")
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config use-context default --kubeconfig=/opt/kubernetes/cfg/bootstrap.kubeconfig")
                .status()
                .expect("Error happened when trying to execute kubectl");

            // Starting kubelet...
            tracing::info!("kubelet installed on {}, starting...", name);
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("systemctl daemon-reload")
                .status()
                .expect("Error happened when trying to reload systemd daemons");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("systemctl start kubelet")
                .status()
                .expect("Error happened when trying to start kubelet");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("systemctl enable kubelet")
                .status()
                .expect("Error happened when trying to enable kubelet");
            tracing::info!("kubelet started on {}", ip);
        }
    }

    loop {
        thread::sleep(time::Duration::from_secs(1));
        let output = Command::new("ssh")
            .arg(format!("root@{}", master_ip))
            .arg("kubectl get csr")
            .output()
            .unwrap()
            .stdout;
        if !output.is_empty() {
            break;
        }
        tracing::info!("Waiting other etcd nodes to join cluster");
    }

    // kubectl approve master node csr.
    let output = Command::new("ssh")
        .arg(format!("root@{}", master_ip))
        .arg("kubectl get csr")
        .output()
        .unwrap()
        .stdout;
    let output = String::from_utf8(output).unwrap();
    tracing::info!("Output of kubectl is: {}", output);
    let csr = Regex::new(r"node-csr-\S*").unwrap();
    let mut res = "";
    for word in output.split_whitespace() {
        if csr.is_match(word) {
            res = word;
            break;
        }
    }
    tracing::info!("Retrieved csr is: {}", res);
    Command::new("ssh")
        .arg(format!("root@{}", master_ip))
        .arg(format!("kubectl certificate approve {}", res))
        .status()
        .expect("Error happened when trying to approve csr from node");

    env::set_current_dir(prev_dir).expect("Error happened when trying to change into `/rk8s`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}
