use crate::config::Config;
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;

pub fn start(config: &Config) {
    tracing::info!("Pre check started");
    fs::create_dir("pre_check").unwrap();
    let mut k8s_conf =
        File::create("k8s.conf").expect("Error happened when trying to create `k8s.conf` file");
    k8s_conf
        .write_all(
            b"net.bridge.bridge-nf-call-ip6tables = 1\nnet.bridge.bridge-nf-call-iptables = 1\n",
        )
        .expect("Error happened when trying to write to `/etc/sysctl.d/k8s.conf`");
    let mut k8s_module = File::create("pre_check/k8s.conf")
        .expect("Error happened when trying to create `k8s.conf` file");
    k8s_module
        .write_all(b"br_netfilter\n")
        .expect("Error happened when trying to write to `/etc/sysctl.d/k8s.conf`");

    for (ip, name) in &config.instance_hosts {
        tracing::info!("Found {} on {}, start pre-setting", name, ip);
        // Stop `firewalld` daemon.
        tracing::info!("Stopping `firewalld` daemon...");
        Command::new("ssh")
            .arg(format!("root@{}", ip))
            .arg("systemctl stop firewalld")
            .status()
            .expect("Error happened when trying to stop firewalld daemon");
        tracing::info!("`firewalld` daemon stopped");

        // Disable `firewalld` daemon.
        tracing::info!("Disabling `firewalld` daemon...");
        Command::new("ssh")
            .arg(format!("root@{}", ip))
            .arg("systemctl disable firewalld")
            .status()
            .expect("Error happened when trying to disable firewalld daemon");
        tracing::info!("`firewalld` daemon disabled");

        // Turn off selinux.
        tracing::info!("Disabling selinux...");
        Command::new("ssh")
            .arg(format!("root@{}", ip))
            .arg("sed -i s/enforcing/disabled/ /etc/selinux/config")
            .status()
            .expect("Error happened when trying to disable swap partition");

        // Turn off swap.
        tracing::info!("Disabling swap partition...");
        Command::new("ssh")
            .arg(format!("root@{}", ip))
            .arg("sed -ri \"s/.*swap.*/#&/\" /etc/fstab")
            .status()
            .expect("Error happened when trying to disable swap partition");
        tracing::info!("swap partition disabled");

        // Set Master `hostname`.
        tracing::info!("Setting master's `hostname` to master01...");
        // Acquire ip and name of this instance.
        Command::new("ssh")
            .arg(format!("root@{}", ip))
            .arg(format!("hostnamectl set-hostname {}", name))
            .status()
            .expect("Error happened when trying to set hostname of master");
        tracing::info!("`hostname` set to master01");

        // Set `/etc/hosts` file.
        tracing::info!("Setting `/etc/hosts` according to configuration...");
        let mut buffer = String::new();
        for (ip, name) in &config.instance_hosts {
            buffer.push_str(format!("{} {}\n", ip, name).as_str());
        }
        Command::new("ssh")
            .arg(format!("root@{}", ip))
            .arg(format!("echo \"{}\" >> /etc/hosts", buffer))
            .status()
            .expect("Error happened when trying to write hosts");
        tracing::info!("`/etc/hosts` set");

        tracing::info!("Setting `/etc/sysctl.d/k8s.conf` according to configuration...");
        Command::new("scp")
            .arg("k8s.conf")
            .arg(format!("root@{}:/etc/sysctl.d", ip))
            .status()
            .expect("Error happened when trying to send files to other nodes");
        Command::new("scp")
            .arg("pre_check/k8s.conf")
            .arg(format!("root@{}:/etc/modules-load.d", ip))
            .status()
            .expect("Error happened when trying to send files to other nodes");

        Command::new("ssh")
            .arg(format!("root@{}", ip))
            .arg("sysctl --system")
            .status()
            .expect("Error happened when activating `k8s.conf`");
        tracing::info!("`/etc/sysctl.d/k8s.conf` set");
    }
}
