use crate::config::Config;
use std::fs::File;
use std::io::Write;
use std::process::Command;

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

pub fn start(config: &Config) {
    tracing::info!("Overwriting `/opt/etcd/cfg/etcd.conf` according to rk8s configuration");
    ETCDCfg::generate(config);
    tracing::info!("`/opt/etcd/cfg/etcd.conf` is now set");

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
    tracing::info!("etcd_{} is now etcd set", config.instance_name);
}
