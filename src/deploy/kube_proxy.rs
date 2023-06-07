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
        let mut kube_proxy_conf = File::create("to_send/kube-proxy.conf")
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
    fn generate(current_ip: &String, current_name: &String) {
        let mut kube_proxy_config = File::create(format!("to_send/{}/kube_proxy/kube-proxy-config.yml", current_ip))
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
            current_name
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
        let mut proxy_unit = File::create("to_send/kube-proxy.service")
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

    tracing::info!("Generating `kube-proxy.conf` to to_send/...");
    KubeProxyCfg::generate();
    tracing::info!("`kube-proxy.conf` generated");

    tracing::info!("Generating `kube-proxy.service` to to_send/...");
    KubeProxyUnit::generate();
    tracing::info!("`kube-proxy.service` generated");

    for (ip, name) in &config.instance_hosts {
        if name.contains("master") {
            tracing::info!("Generating `kube-proxy-config.yml`...");
            KubeProxyConfig::generate(ip, name);
            tracing::info!("`kube-proxy-config.yml` generated");

            Command::new("scp")
                .arg("kube-proxy.pem")
                .arg("kube-proxy-key.pem")
                .arg(format!("root@{}:/opt/kubernetes/ssl", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Certificates sent to master on {}", ip);

            Command::new("scp")
                .arg("to_send/kube-proxy.conf")
                .arg(format!("to_send/{}/kube_proxy/kube-proxy-config.yml", ip))
                .arg(format!("root@{}:/opt/kubernetes/cfg", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Configurations sent to master on {}", ip);

            Command::new("scp")
                .arg("to_send/kube-proxy.service")
                .arg(format!("root@{}:/usr/lib/systemd/system/", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Systemd service sent to master on {}", ip);

            // Generate kubeconfig on remote master.
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg(format!("kubectl config set-cluster kubernetes --certificate-authority=/opt/kubernetes/ssl/ca.pem --embed-certs=true \
                --server=https://{}:6443 --kubeconfig=/opt/kubernetes/cfg/kube-proxy.kubeconfig", ip))
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config set-credentials kube-proxy --client-certificate=/opt/kubernetes/ssl/kube-proxy.pem \
                --client-key=/opt/kubernetes/ssl/kube-proxy-key.pem --embed-certs=true --kubeconfig=/opt/kubernetes/cfg/kube-proxy.kubeconfig")
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config set-context default --cluster=kubernetes --user=kube-proxy \
                --kubeconfig=/opt/kubernetes/cfg/kube-proxy.kubeconfig")
                .status()
                .expect("Error happened when trying to execute kubectl");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl config use-context default --kubeconfig=/opt/kubernetes/cfg/kube-proxy.kubeconfig")
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
                .arg("systemctl start kube-proxy")
                .status()
                .expect("Error happened when trying to start kube-proxy");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("systemctl enable kube-proxy")
                .status()
                .expect("Error happened when trying to enable kube-proxy");
            tracing::info!("kube-proxy started on {}", ip);

            Command::new("scp")
                .arg("/rk8s/preparation/calico.yaml")
                .arg(format!("root@{}:/root", ip))
                .status()
                .expect("Error happened when trying to send files to other nodes");
            tracing::info!("Systemd service sent to master on {}", ip);
            tracing::info!("Deploying Calico...");
            Command::new("ssh")
                .arg(format!("root@{}", ip))
                .arg("kubectl apply -f /root/calico.yaml")
                .status()
                .expect("Error happened when trying to enable kubelet");
        }
    }

    env::set_current_dir(&prev_dir).expect("Error happened when trying to change into `/rk8s`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}