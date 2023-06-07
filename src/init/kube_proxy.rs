use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};

struct KubeProxyCfg;

impl KubeProxyCfg {
    fn generate() {
        let mut kube_proxy_conf = File::create("/opt/kubernetes/cfg/kube-proxy.conf")
            .expect(
                "Error happened when trying to create kube-proxy configuration file",
            );

        writeln!(
            &mut kube_proxy_conf,
r#"KUBE_PROXY_OPTS="--logtostderr=false \
--v=2 \
--log-dir=/opt/kubernetes/logs \
--config=/opt/kubernetes/cfg/kube-proxy-config.yml"
"#
        )
        .expect("Error happened when trying to write `kube-proxy.conf`");
    }
}

struct KubeProxyConfig;

impl KubeProxyConfig {
    fn generate(config: &Config) {
        let mut kube_proxy_config = File::create("/opt/kubernetes/cfg/kube-proxy-config.yml")
            .expect(
                "Error happened when trying to create kube-proxy configuration file",
            );

        writeln!(
            &mut kube_proxy_config,
r#"kind: KubeProxyConfiguration
apiVersion: kubeproxy.config.k8s.io/v1alpha1
bindAddress: 0.0.0.0
metricsBindAddress: 0.0.0.0:10249
clientConnection:
  kubeconfig: /opt/kubernetes/cfg/kube-proxy.kubeconfig"#
        )
        .expect("Error happened when trying to write `kube-proxy-config.yml`");
        writeln!(
            &mut kube_proxy_config,
            "hostnameOverride: {} \\",
            config.instance_name
        )
        .expect("Error happened when trying to write `kube-proxy-config.yml`");
        writeln!(
            &mut kube_proxy_config,
r#"clusterCIDR: 10.244.0.0/16
"#,
        )
        .expect("Error happened when trying to write `kube-proxy-config.yml`");
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct KubeProxyCsr {
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

impl KubeProxyCsr {
    fn from(config: &Config) -> KubeProxyCsr {
        KubeProxyCsr {
            CN: config.kube_proxy_CN.to_owned(),
            hosts: vec![],
            key: Key {
                algo: config.kube_proxy_key_algo.to_owned(),
                size: config.kube_proxy_key_size.to_owned(),
            },
            names: vec![Name {
                C: config.kube_proxy_names_C.to_owned(),
                L: config.kube_proxy_names_L.to_owned(),
                ST: config.kube_proxy_names_ST.to_owned(),
                O: config.kube_proxy_names_O.to_owned(),
                OU: config.kube_proxy_names_OU.to_owned(),
            }],
        }
    }
}

struct KubeProxyUnit;

impl KubeProxyUnit {
    fn generate() {
        let mut proxy_unit = File::create("/usr/lib/systemd/system/kube-proxy.service")
            .expect("Error happened when trying to create kube-proxy unit file");
        let content = r#"[Unit]
Description=Kubernetes Proxy
After=network.target

[Service]
EnvironmentFile=/opt/kubernetes/cfg/kube-proxy.conf
ExecStart=/opt/kubernetes/bin/kube-proxy $KUBE_PROXY_OPTS
Restart=on-failure
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
"#;
        proxy_unit
            .write_all(content.as_bytes())
            .expect("Error happened when trying to write kube-proxy unit file");
    }
}

pub fn start(config: &Config) {
    // kube-proxy
    tracing::info!("kube_proxy phase started");
    tracing::info!("Change working directory into `k8s`");
    let prev_dir = Path::new("/rk8s");
    let work_dir = Path::new("/rk8s/k8s");
    env::set_current_dir(&work_dir).expect("Error happened when trying to change into `k8s`");
    tracing::info!("Changed to {}", env::current_dir().unwrap().display());

    tracing::info!("Start generating `kube-proxy-csr.json`...");
    let kube_proxy_csr = KubeProxyCsr::from(config);
    let content = serde_json::to_string_pretty(&kube_proxy_csr)
        .expect("Error happened when trying to serialize `kube-proxy-csr.json`");
    let mut kube_proxy_csr_file = File::create("kube-proxy-csr.json")
        .expect("Error happened when trying to create `kube-proxy-csr.json`");
    kube_proxy_csr_file
        .write_all(content.as_bytes())
        .expect(
            "Error happened when trying to write content to `kube-proxy-csr.json`",
        );
    tracing::info!("`kube-proxy-csr.json` generated");

    tracing::info!("Generating self-signed kube_proxy https certificate...");
    let cfssl_kube_proxy= Command::new("cfssl")
        .arg("gencert")
        .arg("-ca=ca.pem")
        .arg("-ca-key=ca-key.pem")
        .arg("-config=ca-config.json")
        .arg("-profile=kubernetes")
        .arg("kube-proxy-csr.json")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Error happened when trying to execute `cfssl` command");
    Command::new("cfssljson")
        .arg("-bare")
        .arg("kube-proxy")
        .stdin(Stdio::from(cfssl_kube_proxy.stdout.unwrap()))
        .status()
        .expect("Error happened when trying to execute `cfssljson`");
    tracing::info!("Self-signed kube_proxy CA certificate generated");

    tracing::info!("Generating `kube-proxy.conf` to /opt/kubernetes/cfg...");
    KubeProxyCfg::generate();
    tracing::info!("`kube-proxy.conf` generated");

    tracing::info!("Generating `kube-proxy-config.yml` to /opt/kubernetes/cfg...");
    KubeProxyConfig::generate(config);
    tracing::info!("`kube-proxy-config.yml` generated");

    tracing::info!("Generating `kubeconfig` using `kubectl`");
    Command::new("kubectl")
        .arg("config")
        .arg("set-cluster")
        .arg("kubernetes")
        .arg("--certificate-authority=/opt/kubernetes/ssl/ca.pem")
        .arg("--embed-certs=true")
        .arg(format!("--server=https://{}:6443", config.instance_ip))
        .arg("--kubeconfig=/opt/kubernetes/cfg/kube-proxy.kubeconfig")
        .status()
        .expect("Error happened when trying to execute kubectl");
    Command::new("kubectl")
        .arg("config")
        .arg("set-credentials")
        .arg("kube-proxy")
        .arg("--client-certificate=./kube-proxy.pem")
        .arg("--client-key=./kube-proxy-key.pem")
        .arg("--embed-certs=true")
        .arg("--kubeconfig=/opt/kubernetes/cfg/kube-proxy.kubeconfig")
        .status()
        .expect("Error happened when trying to execute kubectl");
    Command::new("kubectl")
        .arg("config")
        .arg("set-context")
        .arg("default")
        .arg("--cluster=kubernetes")
        .arg("--user=kube-proxy")
        .arg("--kubeconfig=/opt/kubernetes/cfg/kube-proxy.kubeconfig")
        .status()
        .expect("Error happened when trying to execute kubectl");
    Command::new("kubectl")
        .arg("config")
        .arg("use-context")
        .arg("default")
        .arg("--kubeconfig=/opt/kubernetes/cfg/kube-proxy.kubeconfig")
        .status()
        .expect("Error happened when trying to execute kubectl");

    tracing::info!("Generating `kube-proxy.service` to /usr/lib/systemd/system/");
    KubeProxyUnit::generate();
    tracing::info!("`kube-proxy.service` generated");

    Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .expect("Error happened when trying to reload systemd daemons");
    Command::new("systemctl")
        .arg("enable")
        .arg("kube-proxy")
        .status()
        .expect("Error happened when trying to enable `kube-proxy.service`");
    Command::new("systemctl")
        .arg("start")
        .arg("kube-proxy")
        .status()
        .expect("Error happened when trying to start `kube-proxy.service`");
    tracing::info!("Master's proxy is now set");

    tracing::info!("Deploying Calico...");
    Command::new("curl")
        .arg("https://docs.projectcalico.org/v3.20/manifests/calico.yaml")
        .arg("-o")
        .arg("calico.yaml")
        .status()
        .expect("Error happened when trying to download Calico");
    Command::new("kubectl")
        .arg("apply")
        .arg("-f")
        .arg("calico.yaml")
        .status()
        .expect("Error happened when trying to apply calico.yaml");

    env::set_current_dir(&prev_dir).expect("Error happened when trying to change into `/rk8s`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}