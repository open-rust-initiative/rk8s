use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    // The machine running this program.
    pub instance_name: String,
    // The ip address of running server.
    pub instance_ip: String,
    pub instance_hosts: HashMap<String, String>,

    // Fields needed by `install cfssl` command.
    pub cfssl_url: String,
    pub cfssljson_url: String,
    pub cfsslcertinfo_url: String,
    // Fields needed by `install etcd` command.
    pub etcd_url: String,
    // Fields needed by `install docker` command.
    pub docker_url: String,
    // Fields needed by `install kubernetes` command.
    pub kubernetes_url: String,

    // Fields needed by `etcd` phase.
    pub etcd_ca_CN: String,
    pub etcd_CN: String,
    pub etcd_key_algo: String,
    pub etcd_key_size: i64,
    pub etcd_expiry: String,
    pub etcd_usages: Vec<String>,
    pub etcd_names_C: String,
    pub etcd_names_L: String,
    pub etcd_names_ST: String,

    // Fields needed by `kube_apiserver` phase.
    pub kube_apiserver_CN: String,
    pub kube_apiserver_key_algo: String,
    pub kube_apiserver_key_size: i64,
    pub kube_apiserver_expiry: String,
    pub kube_apiserver_usages: Vec<String>,
    pub kube_apiserver_names_C: String,
    pub kube_apiserver_names_L: String,
    pub kube_apiserver_names_ST: String,
    pub kube_apiserver_names_O: String,
    pub kube_apiserver_names_OU: String,

    // Fields needed by `kube_controller_manager` phase.
    pub kube_controller_manager_CN: String,
    pub kube_controller_manager_key_algo: String,
    pub kube_controller_manager_key_size: i64,
    pub kube_controller_manager_names_C: String,
    pub kube_controller_manager_names_L: String,
    pub kube_controller_manager_names_ST: String,
    pub kube_controller_manager_names_O: String,
    pub kube_controller_manager_names_OU: String,

    // Fields needed by `kube_scheduler` phase.
    pub kube_scheduler_CN: String,
    pub kube_scheduler_key_algo: String,
    pub kube_scheduler_key_size: i64,
    pub kube_scheduler_names_C: String,
    pub kube_scheduler_names_L: String,
    pub kube_scheduler_names_ST: String,
    pub kube_scheduler_names_O: String,
    pub kube_scheduler_names_OU: String,

    // Fields needed by `kube_ctl` phase.
    pub kube_ctl_CN: String,
    pub kube_ctl_key_algo: String,
    pub kube_ctl_key_size: i64,
    pub kube_ctl_names_C: String,
    pub kube_ctl_names_L: String,
    pub kube_ctl_names_ST: String,
    pub kube_ctl_names_O: String,
    pub kube_ctl_names_OU: String,

    // Fields needed by `kube_proxy` phase.
    pub kube_proxy_CN: String,
    pub kube_proxy_key_algo: String,
    pub kube_proxy_key_size: i64,
    pub kube_proxy_names_C: String,
    pub kube_proxy_names_L: String,
    pub kube_proxy_names_ST: String,
    pub kube_proxy_names_O: String,
    pub kube_proxy_names_OU: String,
}

impl Config {
    pub fn init() -> Config {
        tracing::info!("Reading config file...");
        let mut file = File::open("cfg/config.yaml").expect("File `config.yaml` does not exist!");
        let mut content = vec![];
        file.read_to_end(&mut content)
            .expect("Error happened when trying to read content of `config.yaml`");
        let config = serde_yaml::from_slice(&content)
            .expect("Something went wrong while parsing config.yaml");
        tracing::info!("Config read");
        config
    }
}

pub fn generate_config_template() {
    let config = Config {
        instance_name: "master01".to_owned(),
        instance_ip: "192.168.221.143".to_owned(),
        instance_hosts: {
            let mut map = HashMap::new();
            map.insert(
                "192.168.221.143".to_owned(),
                "master01".to_owned(),
            );
            map.insert(
                "192.168.221.147".to_owned(),
                "worker01".to_owned(),
            );
            map.insert(
                "192.168.221.148".to_owned(),
                "worker02".to_owned(),
            );
            map
        },

        cfssl_url: "https://pkg.cfssl.org/R1.2/cfssl_linux-amd64".to_owned(),
        cfssljson_url: "https://pkg.cfssl.org/R1.2/cfssljson_linux-amd64".to_owned(),
        cfsslcertinfo_url: "https://pkg.cfssl.org/R1.2/cfssl-certinfo_linux-amd64".to_owned(),
        etcd_url: "https://github.com/etcd-io/etcd/releases/download/v3.4.9/etcd-v3.4.9-linux-amd64.tar.gz".to_owned(),
        docker_url: "https://download.docker.com/linux/static/stable/x86_64/docker-20.10.9.tgz".to_owned(),
        kubernetes_url: "https://dl.k8s.io/v1.20.15/kubernetes-server-linux-amd64.tar.gz".to_owned(),

        etcd_ca_CN: "etcd CA".to_owned(),
        etcd_CN: "etcd".to_owned(),
        etcd_key_algo: "rsa".to_owned(),
        etcd_key_size: 2048,
        etcd_expiry: "87600h".to_owned(),
        etcd_usages: vec![
            "signing".to_owned(),
            "key encipherment".to_owned(),
            "server auth".to_owned(),
            "client auth".to_owned(),
        ],
        etcd_names_C: "CN".to_owned(),
        etcd_names_L: "Beijing".to_owned(),
        etcd_names_ST: "Beijing".to_owned(),

        kube_apiserver_CN: "kubernetes".to_owned(),
        kube_apiserver_key_algo: "rsa".to_owned(),
        kube_apiserver_key_size: 2048,
        kube_apiserver_expiry: "87600h".to_owned(),
        kube_apiserver_usages: vec![
            "signing".to_owned(),
            "key encipherment".to_owned(),
            "server auth".to_owned(),
            "client auth".to_owned(),
        ],
        kube_apiserver_names_C: "CN".to_owned(),
        kube_apiserver_names_L: "Beijing".to_owned(),
        kube_apiserver_names_ST: "Beijing".to_owned(),
        kube_apiserver_names_O: "k8s".to_owned(),
        kube_apiserver_names_OU: "System".to_owned(),

        kube_controller_manager_CN: "system:kube-controller-manager".to_owned(),
        kube_controller_manager_key_algo: "rsa".to_owned(),
        kube_controller_manager_key_size: 2048,
        kube_controller_manager_names_C: "CN".to_owned(),
        kube_controller_manager_names_L: "Beijing".to_owned(),
        kube_controller_manager_names_ST: "Beijing".to_owned(),
        kube_controller_manager_names_O: "system:masters".to_owned(),
        kube_controller_manager_names_OU: "System".to_owned(),

        kube_scheduler_CN: "system:kube-scheduler".to_owned(),
        kube_scheduler_key_algo: "rsa".to_owned(),
        kube_scheduler_key_size: 2048,
        kube_scheduler_names_C: "CN".to_owned(),
        kube_scheduler_names_L: "Beijing".to_owned(),
        kube_scheduler_names_ST: "Beijing".to_owned(),
        kube_scheduler_names_O: "system:masters".to_owned(),
        kube_scheduler_names_OU: "System".to_owned(),

        kube_ctl_CN: "admin".to_owned(),
        kube_ctl_key_algo: "rsa".to_owned(),
        kube_ctl_key_size: 2048,
        kube_ctl_names_C: "CN".to_owned(),
        kube_ctl_names_L: "Beijing".to_owned(),
        kube_ctl_names_ST: "Beijing".to_owned(),
        kube_ctl_names_O: "system:masters".to_owned(),
        kube_ctl_names_OU: "System".to_owned(),

        kube_proxy_CN: "system:kube-proxy".to_owned(),
        kube_proxy_key_algo: "rsa".to_owned(),
        kube_proxy_key_size: 2048,
        kube_proxy_names_C: "CN".to_owned(),
        kube_proxy_names_L: "Beijing".to_owned(),
        kube_proxy_names_ST: "Beijing".to_owned(),
        kube_proxy_names_O: "k8s".to_owned(),
        kube_proxy_names_OU: "System".to_owned(),
    };

    let yaml = serde_yaml::to_string(&config).unwrap();
    let mut file = File::create("cfg/config.yaml").unwrap();
    file.write_all(yaml.as_bytes()).unwrap();
}
