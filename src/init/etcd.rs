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
    www: WWWProfile,
}

#[derive(Serialize, Deserialize, Debug)]
struct WWWProfile {
    expiry: String,
    usages: Vec<String>,
}

impl CAConfig {
    fn from(config: &Config) -> CAConfig {
        CAConfig {
            signing: Signing {
                default: SignDefault {
                    expiry: config.etcd_expiry.to_owned(),
                },
                profiles: SignProfiles {
                    www: WWWProfile {
                        expiry: config.etcd_expiry.to_owned(),
                        usages: config.etcd_usages.to_owned(),
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
}

impl CACsr {
    fn from(config: &Config) -> CACsr {
        CACsr {
            CN: config.etcd_ca_CN.to_owned(),
            key: Key {
                algo: config.etcd_key_algo.to_owned(),
                size: config.etcd_key_size.to_owned(),
            },
            names: vec![Name {
                C: config.etcd_names_C.to_owned(),
                L: config.etcd_names_L.to_owned(),
                ST: config.etcd_names_ST.to_owned(),
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
            CN: config.etcd_CN.to_owned(),
            hosts: {
                let mut hosts = Vec::new();
                for (ip, _) in &config.instance_hosts {
                    hosts.push(ip.to_owned());
                }
                hosts
            },
            key: Key {
                algo: config.etcd_key_algo.to_owned(),
                size: config.etcd_key_size.to_owned(),
            },
            names: vec![Name {
                C: config.etcd_names_C.to_owned(),
                L: config.etcd_names_L.to_owned(),
                ST: config.etcd_names_ST.to_owned(),
            }],
        }
    }
}

struct ETCDCfg;

impl ETCDCfg {
    fn generate(config: &Config) {
        let mut etcd_conf = File::create("/opt/etcd/cfg/etcd.conf")
            .expect("Error happened when trying to create etcd configuration file");

        writeln!(&mut etcd_conf, "#[Member]")
            .expect("Error happened when trying to write `etcd.conf`");
        writeln!(
            &mut etcd_conf,
            "ETCD_NAME=\"etcd_{}\"",
            config.instance_name
        )
        .expect("Error happened when trying to write `etcd.conf`");
        writeln!(
            &mut etcd_conf,
            "ETCD_DATA_DIR=\"/var/lib/etcd/default.etcd\""
        )
        .expect("Error happened when trying to write `etcd.conf`");
        writeln!(
            &mut etcd_conf,
            "ETCD_LISTEN_PEER_URLS=\"https://{}:2380\"",
            config.instance_ip
        )
        .expect("Error happened when trying to write `etcd.conf`");
        writeln!(
            &mut etcd_conf,
            "ETCD_LISTEN_CLIENT_URLS=\"https://{}:2379\"",
            config.instance_ip
        )
        .expect("Error happened when trying to write `etcd.conf`");
        writeln!(&mut etcd_conf, "").expect("Error happened when trying to write `etcd.conf`");
        writeln!(&mut etcd_conf, "#[Clustering]")
            .expect("Error happened when trying to write `etcd.conf`");
        writeln!(
            &mut etcd_conf,
            "ETCD_INITIAL_ADVERTISE_PEER_URLS=\"https://{}:2380\"",
            config.instance_ip
        )
        .expect("Error happened when trying to write `etcd.conf`");
        writeln!(
            &mut etcd_conf,
            "ETCD_ADVERTISE_CLIENT_URLS=\"https://{}:2379\"",
            config.instance_ip
        )
        .expect("Error happened when trying to write `etcd.conf`");
        let mut buffer = String::new();
        for (ip, name) in &config.instance_hosts {
            buffer.push_str(format!("etcd_{}=https://{}:2380,", name, ip).as_str());
        }
        buffer.pop();
        writeln!(&mut etcd_conf, "ETCD_INITIAL_CLUSTER=\"{}\"", buffer)
            .expect("Error happened when trying to write `etcd.conf`");
        writeln!(
            &mut etcd_conf,
            "ETCD_INITIAL_CLUSTER_TOKEN=\"etcd-cluster\""
        )
        .expect("Error happened when trying to write `etcd.conf`");
        writeln!(&mut etcd_conf, "ETCD_INITIAL_CLUSTER_STATE=\"new\"")
            .expect("Error happened when trying to write `etcd.conf`");
    }
}

struct ETCDUnit;

impl ETCDUnit {
    fn generate() {
        let mut etcd_unit = File::create("/usr/lib/systemd/system/etcd.service")
            .expect("Error happened when trying to create etcd unit file");
        let content = r#"[Unit]
Description=Etcd Server
After=network.target
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
EnvironmentFile=/opt/etcd/cfg/etcd.conf
ExecStart=/opt/etcd/bin/etcd \
--cert-file=/opt/etcd/ssl/server.pem \
--key-file=/opt/etcd/ssl/server-key.pem \
--peer-cert-file=/opt/etcd/ssl/server.pem \
--peer-key-file=/opt/etcd/ssl/server-key.pem \
--trusted-ca-file=/opt/etcd/ssl/ca.pem \
--peer-trusted-ca-file=/opt/etcd/ssl/ca.pem \
--logger=zap
Restart=on-failure
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
"#;
        etcd_unit
            .write_all(content.as_bytes())
            .expect("Error happened when trying to write etcd unit file");
    }
}

pub fn start(config: &Config) {
    tracing::info!("etcd phase started");
    tracing::info!("Change working directory into `etcd`");
    let prev_dir = Path::new("/rk8s");
    let work_dir = Path::new("/rk8s/etcd");
    env::set_current_dir(&work_dir).expect("Error happened when trying to change into `etcd`");
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

    tracing::info!("Start generating `ca-config.json`...");
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

    tracing::info!("Generating self-signed etcd https certificate...");
    let cfssl_etcd = Command::new("cfssl")
        .arg("gencert")
        .arg("-ca=ca.pem")
        .arg("-ca-key=ca-key.pem")
        .arg("-config=ca-config.json")
        .arg("-profile=www")
        .arg("server-csr.json")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Error happened when trying to execute `cfssl` command");
    Command::new("cfssljson")
        .arg("-bare")
        .arg("server")
        .stdin(Stdio::from(cfssl_etcd.stdout.unwrap()))
        .status()
        .expect("Error happened when trying to execute `cfssljson`");
    tracing::info!("Self-signed CA certificate generated");

    tracing::info!("Copying certificates to /opt/etcd/ssl...");
    Command::new("cp")
        .arg("ca.pem")
        .arg("ca-key.pem")
        .arg("server-key.pem")
        .arg("server.pem")
        .arg("/opt/etcd/ssl")
        .status()
        .expect("Error happened when trying to copy certificates to `/opt/etcd/ssl`");
    tracing::info!("ertificates copied");

    tracing::info!("Generating `etcd.conf` to /opt/etcd/cfg...");
    ETCDCfg::generate(config);
    tracing::info!("`etcd.conf` generated");

    tracing::info!("Generating `etcd.service` to /usr/lib/systemd/system/...");
    ETCDUnit::generate();
    tracing::info!("`etcd.service` generated");

    tracing::info!("Sending etcd to worker nodes...");
    for (ip, _) in &config.instance_hosts {
        if *ip != config.instance_ip {
            Command::new("scp")
                .arg("-r")
                .arg("/opt/etcd")
                .arg(format!("root@{}:/opt/", ip))
                .status()
                .expect("Error happened when trying to send files to other worker nodes");
            Command::new("scp")
                .arg("/usr/lib/systemd/system/etcd.service")
                .arg(format!("root@{}:/usr/lib/systemd/system/", ip))
                .status()
                .expect("Error happened when trying to send files to other worker nodes");
        }
    }
    tracing::info!("Files sent to other worker nodes");

    Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .expect("Error happened when trying to reload systemd daemons");
    Command::new("systemctl")
        .arg("enable")
        .arg("etcd")
        .status()
        .expect("Error happened when trying to enable `etcd.service`");
    Command::new("systemctl")
        .arg("start")
        .arg("etcd")
        .status()
        .expect("Error happened when trying to start `etcd.service`");
    tracing::info!("Master is now etcd set");

    env::set_current_dir(&prev_dir).expect("Error happened when trying to change into `etcd`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}
