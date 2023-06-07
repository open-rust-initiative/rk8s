use crate::config::Config;
use regex::Regex;
use std::env;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::process::Command;

struct KubeletCfg;

impl KubeletCfg {
    fn generate(config: &Config) {
        let mut kubelet_conf = File::create("/opt/kubernetes/cfg/kubelet.conf")
            .expect(
                "Error happened when trying to create kubelet configuration file",
            );

        writeln!(
            &mut kubelet_conf,
r#"KUBELET_OPTS="--logtostderr=false \
--v=2 \
--log-dir=/opt/kubernetes/logs \"#,
        )
        .expect("Error happened when trying to write `kubelet.conf`");
        writeln!(
            &mut kubelet_conf,
            "--hostname-override={} \\",
            config.instance_name
        )
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
        let mut kubelet_config = File::create("/opt/kubernetes/cfg/kubelet-config.yml")
            .expect(
                "Error happened when trying to create kubelet configuration file",
            );

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
        let mut kubelet_unit = File::create("/usr/lib/systemd/system/kubelet.service")
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
    env::set_current_dir(&work_dir).expect("Error happened when trying to change into `k8s`");
    tracing::info!("Changed to {}", env::current_dir().unwrap().display());

    tracing::info!("Generating `kubelet.conf` to /opt/kubernetes/cfg...");
    KubeletCfg::generate(config);
    tracing::info!("`kubelet.conf` generated");

    tracing::info!("Generating `kubelet-config.yml` to /opt/kubernetes/cfg...");
    KubeletConfig::generate();
    tracing::info!("`kubelet-config.yml` generated");

    tracing::info!("Generating `kubeconfig` using `kubectl`");
    Command::new("kubectl")
        .arg("config")
        .arg("set-cluster")
        .arg("kubernetes")
        .arg("--certificate-authority=/opt/kubernetes/ssl/ca.pem")
        .arg("--embed-certs=true")
        .arg(format!("--server=https://{}:6443", config.instance_ip))
        .arg("--kubeconfig=/opt/kubernetes/cfg/bootstrap.kubeconfig")
        .status()
        .expect("Error happened when trying to execute kubectl");
    Command::new("kubectl")
        .arg("config")
        .arg("set-credentials")
        .arg("kubelet-bootstrap")
        .arg("--token=4136692876ad4b01bb9dd0988480ebba")
        .arg("--kubeconfig=/opt/kubernetes/cfg/bootstrap.kubeconfig")
        .status()
        .expect("Error happened when trying to execute kubectl");
    Command::new("kubectl")
        .arg("config")
        .arg("set-context")
        .arg("default")
        .arg("--cluster=kubernetes")
        .arg("--user=kubelet-bootstrap")
        .arg("--kubeconfig=/opt/kubernetes/cfg/bootstrap.kubeconfig")
        .status()
        .expect("Error happened when trying to execute kubectl");
    Command::new("kubectl")
        .arg("config")
        .arg("use-context")
        .arg("default")
        .arg("--kubeconfig=/opt/kubernetes/cfg/bootstrap.kubeconfig")
        .status()
        .expect("Error happened when trying to execute kubectl");

    tracing::info!("Generating `kubelet.service` to /usr/lib/systemd/system/");
    KubeletUnit::generate();
    tracing::info!("`kubelet.service` generated");

    Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .expect("Error happened when trying to reload systemd daemons");
    Command::new("systemctl")
        .arg("enable")
        .arg("kubelet")
        .status()
        .expect("Error happened when trying to enable `kubelet.service`");
    Command::new("systemctl")
        .arg("start")
        .arg("kubelet")
        .status()
        .expect("Error happened when trying to start `kubelet.service`");
    tracing::info!("Master's kubelet is now set");
    
    loop {
        let output = Command::new("kubectl").arg("get").arg("csr").output().unwrap().stdout;
        if !(output.len() == 0) {
            break;
        }
        tracing::info!("Waiting other etcd nodes to join cluster");
    }

    // kubectl approve master node csr.
    let output = Command::new("kubectl")
        .arg("get")
        .arg("csr")
        .output()
        .unwrap()
        .stdout;
    let output = String::from_utf8(output).unwrap();
    tracing::info!("Output of kubectl is: {}", output);
    let csr = Regex::new(r"node-csr-\S*").unwrap();
    let mut res = "";
    for word in output.split_whitespace() {
        if csr.is_match(word) {
            res = word.clone();
            break;
        }
    }
    tracing::info!("Retrieved csr is: {}", res);
    Command::new("kubectl")
        .arg("certificate")
        .arg("approve")
        .arg(res)
        .status()
        .expect("Error happened when trying to approve csr from node");

    env::set_current_dir(&prev_dir).expect("Error happened when trying to change into `/rk8s`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}