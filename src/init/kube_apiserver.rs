use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Serialize, Deserialize, Debug)]
struct CAConfig {
    signing: Signing,
}

#[derive(Serialize, Deserialize, Debug)]
struct Signing {
    default: SignDefault,
    profiles: SignProfiles,
}

#[derive(Serialize, Deserialize, Debug)]
struct SignDefault {
    expiry: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SignProfiles {
    kubernetes: KubernetesProfile,
}

#[derive(Serialize, Deserialize, Debug)]
struct KubernetesProfile {
    expiry: String,
    usages: Vec<String>,
}

impl CAConfig {
    fn from(config: &Config) -> CAConfig {
        CAConfig {
            signing: Signing {
                default: SignDefault {
                    expiry: config.kube_apiserver_expiry.to_owned(),
                },
                profiles: SignProfiles {
                    kubernetes: KubernetesProfile {
                        expiry: config.kube_apiserver_expiry.to_owned(),
                        usages: config.kube_apiserver_usages.to_owned(),
                    },
                },
            },
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct CACsr {
    CN: String,
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

impl CACsr {
    fn from(config: &Config) -> CACsr {
        CACsr {
            CN: config.kube_apiserver_CN.to_owned(),
            key: Key {
                algo: config.kube_apiserver_key_algo.to_owned(),
                size: config.kube_apiserver_key_size.to_owned(),
            },
            names: vec![Name {
                C: config.kube_apiserver_names_C.to_owned(),
                L: config.kube_apiserver_names_L.to_owned(),
                ST: config.kube_apiserver_names_ST.to_owned(),
                O: config.kube_apiserver_names_O.to_owned(),
                OU: config.kube_apiserver_names_OU.to_owned(),
            }],
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct ServerCsr {
    CN: String,
    hosts: Vec<String>,
    key: Key,
    names: Vec<Name>,
}

impl ServerCsr {
    fn from(config: &Config) -> ServerCsr {
        ServerCsr {
            CN: config.kube_apiserver_CN.to_owned(),
            hosts: {
                let mut hosts = vec![
                    "10.0.0.1".to_string(),
                    "127.0.0.1".to_string(),
                    "kubernetes".to_string(),
                    "kubernetes.default".to_string(),
                    "kubernetes.default.svc".to_string(),
                    "kubernetes.default.svc.cluster".to_string(),
                    "kubernetes.default.svc.cluster.local".to_string(),
                ];
                for ip in config.instance_hosts.keys() {
                    hosts.push(ip.to_owned());
                }
                hosts
            },
            key: Key {
                algo: config.kube_apiserver_key_algo.to_owned(),
                size: config.kube_apiserver_key_size.to_owned(),
            },
            names: vec![Name {
                C: config.kube_apiserver_names_C.to_owned(),
                L: config.kube_apiserver_names_L.to_owned(),
                ST: config.kube_apiserver_names_ST.to_owned(),
                O: config.kube_apiserver_names_O.to_owned(),
                OU: config.kube_apiserver_names_OU.to_owned(),
            }],
        }
    }
}

struct KubeApiserverCfg;

impl KubeApiserverCfg {
    fn generate(config: &Config) {
        let mut apiserver_conf = File::create("/opt/kubernetes/cfg/kube-apiserver.conf")
            .expect("Error happened when trying to create kube-apiserver configuration file");

        writeln!(
            &mut apiserver_conf,
            r#"KUBE_APISERVER_OPTS="--logtostderr=false \
--v=2 \
--log-dir=/opt/kubernetes/logs \"#
        )
        .expect("Error happened when trying to write `kube-apiserver.conf`");
        let mut buffer = String::new();
        for ip in config.instance_hosts.keys() {
            buffer.push_str(format!("https://{}:2379,", ip).as_str());
        }
        buffer.pop();
        writeln!(&mut apiserver_conf, "--etcd-servers={}", buffer)
            .expect("Error happened when trying to write `kube-apiserver.conf`");
        writeln!(&mut apiserver_conf, "--bind-address={}", config.instance_ip)
            .expect("Error happened when trying to write `kube-apiserver.conf`");
        writeln!(&mut apiserver_conf, "--secure-port=6443")
            .expect("Error happened when trying to write `kube-apiserver.conf`");
        writeln!(
            &mut apiserver_conf,
            "--advertise-address={}",
            config.instance_ip
        )
        .expect("Error happened when trying to write `kube-apiserver.conf`");
        writeln!(
            &mut apiserver_conf,
r#"--allow-privileged=true \
--service-cluster-ip-range=10.0.0.0/24 \
--enable-admission-plugins=NamespaceLifecycle,LimitRanger,ServiceAccount,ResourceQuota,NodeRestriction \
--authorization-mode=RBAC,Node \
--enable-bootstrap-token-auth=true \
--token-auth-file=/opt/kubernetes/cfg/token.csv \
--service-node-port-range=30000-32767 \
--kubelet-client-certificate=/opt/kubernetes/ssl/server.pem \
--kubelet-client-key=/opt/kubernetes/ssl/server-key.pem \
--tls-cert-file=/opt/kubernetes/ssl/server.pem  \
--tls-private-key-file=/opt/kubernetes/ssl/server-key.pem \
--client-ca-file=/opt/kubernetes/ssl/ca.pem \
--service-account-key-file=/opt/kubernetes/ssl/ca-key.pem \
--service-account-issuer=api \
--service-account-signing-key-file=/opt/kubernetes/ssl/server-key.pem \
--etcd-cafile=/opt/etcd/ssl/ca.pem \
--etcd-certfile=/opt/etcd/ssl/server.pem \
--etcd-keyfile=/opt/etcd/ssl/server-key.pem \
--requestheader-client-ca-file=/opt/kubernetes/ssl/ca.pem \
--proxy-client-cert-file=/opt/kubernetes/ssl/server.pem \
--proxy-client-key-file=/opt/kubernetes/ssl/server-key.pem \
--requestheader-allowed-names=kubernetes \
--requestheader-extra-headers-prefix=X-Remote-Extra- \
--requestheader-group-headers=X-Remote-Group \
--requestheader-username-headers=X-Remote-User \
--enable-aggregator-routing=true \
--audit-log-maxage=30 \
--audit-log-maxbackup=3 \
--audit-log-maxsize=100 \
--audit-log-path=/opt/kubernetes/logs/k8s-audit.log""#,
        )
        .expect("Error happened when trying to write `kube-apiserver.conf`");
    }
}

struct KubeApiserverUnit;

impl KubeApiserverUnit {
    fn generate() {
        let mut kube_apiserver_unit =
            File::create("/usr/lib/systemd/system/kube-apiserver.service")
                .expect("Error happened when trying to create kube-apiserver unit file");
        let content = r#"[Unit]
Description=Kubernetes API Server
Documentation=https://github.com/kubernetes/kubernetes

[Service]
EnvironmentFile=/opt/kubernetes/cfg/kube-apiserver.conf
ExecStart=/opt/kubernetes/bin/kube-apiserver $KUBE_APISERVER_OPTS
Restart=on-failure

[Install]
WantedBy=multi-user.target
"#;
        kube_apiserver_unit
            .write_all(content.as_bytes())
            .expect("Error happened when trying to write kube-apiserver unit file");
    }
}

pub fn start(config: &Config) {
    tracing::info!("kube_apiserver phase started");
    tracing::info!("Change working directory into `k8s`");
    let prev_dir = Path::new("/rk8s");
    let work_dir = Path::new("/rk8s/k8s");
    env::set_current_dir(work_dir).expect("Error happened when trying to change into `k8s`");
    tracing::info!("Changed to {}", env::current_dir().unwrap().display());

    tracing::info!("Start generating `ca-config.json`...");
    let ca_config = CAConfig::from(config);
    let content = serde_json::to_string_pretty(&ca_config)
        .expect("Error happened when trying to serialize `ca-config.json`");
    let mut ca_config_file = File::create("ca-config.json")
        .expect("Error happened when trying to create `ca-config.json`");
    ca_config_file
        .write_all(content.as_bytes())
        .expect("Error happened when trying to write content to `ca-config.json`");
    tracing::info!("`ca-config.json` generated");

    tracing::info!("Start generating `ca-csr.json`...");
    let ca_csr = CACsr::from(config);
    let content = serde_json::to_string_pretty(&ca_csr)
        .expect("Error happened when trying to serialize `ca-csr.json`");
    let mut ca_csr_file =
        File::create("ca-csr.json").expect("Error happened when trying to create `ca-csr.json`");
    ca_csr_file
        .write_all(content.as_bytes())
        .expect("Error happened when trying to write content to `ca-csr.json`");
    tracing::info!("`ca-csr.json` generated");

    tracing::info!("Generating self-signed CA certificate...");
    let cfssl_ca = Command::new("cfssl")
        .arg("gencert")
        .arg("-initca")
        .arg("ca-csr.json")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Error happened when trying to execute `cfssl` command");
    Command::new("cfssljson")
        .arg("-bare")
        .arg("ca")
        .arg("-")
        .stdin(Stdio::from(cfssl_ca.stdout.unwrap()))
        .status()
        .expect("Error happened when trying to execute `cfssljson`");
    tracing::info!("Self-signed CA certificate generated");

    tracing::info!("Start generating `server-csr.json`...");
    let server_csr = ServerCsr::from(config);
    let content = serde_json::to_string_pretty(&server_csr)
        .expect("Error happened when trying to serialize `server-csr.json`");
    let mut server_csr_file = File::create("server-csr.json")
        .expect("Error happened when trying to create `server-csr.json`");
    server_csr_file
        .write_all(content.as_bytes())
        .expect("Error happened when trying to write content to `server-csr.json`");
    tracing::info!("`server-csr.json` generated");

    tracing::info!("Generating self-signed kube_apiserver https certificate...");
    let cfssl_kube = Command::new("cfssl")
        .arg("gencert")
        .arg("-ca=ca.pem")
        .arg("-ca-key=ca-key.pem")
        .arg("-config=ca-config.json")
        .arg("-profile=kubernetes")
        .arg("server-csr.json")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Error happened when trying to execute `cfssl` command");
    Command::new("cfssljson")
        .arg("-bare")
        .arg("server")
        .stdin(Stdio::from(cfssl_kube.stdout.unwrap()))
        .status()
        .expect("Error happened when trying to execute `cfssljson`");
    tracing::info!("Self-signed CA certificate generated");

    tracing::info!("Copying certificates to /opt/kubernetes/ssl...");
    Command::new("cp")
        .arg("ca.pem")
        .arg("ca-key.pem")
        .arg("server-key.pem")
        .arg("server.pem")
        .arg("/opt/kubernetes/ssl")
        .status()
        .expect("Error happened when trying to copy certificates to `/opt/kubernetes/ssl`");
    tracing::info!("ertificates copied");

    tracing::info!("Generating `kube-apiserver.conf` to /opt/kubernetes/cfg...");
    KubeApiserverCfg::generate(config);
    tracing::info!("`kube-apiserver.conf` generated");

    tracing::info!("Generating `token.csv` to /opt/kubernetes/cfg/token.csv...");
    let mut token = File::create("/opt/kubernetes/cfg/token.csv")
        .expect("Error happened when trying to write token file");
    token.write_all(b"4136692876ad4b01bb9dd0988480ebba,kubelet-bootstrap,10001,\"system:node-bootstrapper\"").expect("Error happened when trying to write `token.csv`");
    tracing::info!("`token.csv` generated");

    tracing::info!("Generating `kube-apiserver.service` to /usr/lib/systemd/system/");
    KubeApiserverUnit::generate();
    tracing::info!("`kube-apiserver.service` generated");

    // tracing::info!("Sending etcd to worker nodes...");
    // for (ip, _) in &config.instance_hosts {
    //     if *ip != config.instance_ip {
    //         Command::new("scp").arg("-r").arg("/opt/etcd").arg(format!("root@{}:/opt/", ip)).status().expect("Error happened when trying to send files to other worker nodes");
    //         Command::new("scp").arg("/usr/lib/systemd/system/etcd.service").arg(format!("root@{}:/usr/lib/systemd/system/", ip)).status().expect("Error happened when trying to send files to other worker nodes");
    //     }
    // }
    // tracing::info!("Files sent to other worker nodes");

    Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .expect("Error happened when trying to reload systemd daemons");
    Command::new("systemctl")
        .arg("enable")
        .arg("kube-apiserver")
        .status()
        .expect("Error happened when trying to enable `kube-apiserver.service`");
    Command::new("systemctl")
        .arg("start")
        .arg("kube-apiserver")
        .status()
        .expect("Error happened when trying to start `kube-apiserver.service`");
    tracing::info!("Master's apiserver is now set");

    env::set_current_dir(prev_dir).expect("Error happened when trying to change into `/rk8s`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}
