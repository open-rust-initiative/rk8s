use crate::config::Config;
use std::fs::File;
use std::io::Write;
use std::process::Command;

pub fn start(config: &Config) {
    tracing::info!("Pre check started");

    // Stop `firewalld` daemon.
    tracing::info!("Stopping `firewalld` daemon...");
    Command::new("systemctl")
        .arg("stop")
        .arg("firewalld")
        .status()
        .expect("Error happened when trying to stop firewalld daemon");
    tracing::info!("`firewalld` daemon stopped");

    // Disable `firewalld` daemon.
    tracing::info!("Disabling `firewalld` daemon...");
    Command::new("systemctl")
        .arg("disable")
        .arg("firewalld")
        .status()
        .expect("Error happened when trying to disable firewalld daemon");
    tracing::info!("`firewalld` daemon disabled");

    // Turn off selinux.
    tracing::info!("Disabling selinux...");
    Command::new("sed")
        .arg("-i")
        .arg("s/enforcing/disabled/")
        .arg("/etc/selinux/config")
        .status()
        .expect("Error happened when trying to disable swap partition");

    // Turn off swap.
    tracing::info!("Disabling swap partition...");
    Command::new("sed")
        .arg("-ri")
        .arg("s/.*swap.*/#&/")
        .arg("/etc/fstab")
        .status()
        .expect("Error happened when trying to disable swap partition");
    tracing::info!("swap partition disabled");

    // Set Master `hostname`.
    tracing::info!("Setting master's `hostname` to master01...");
    // Acquire ip and name of this instance.
    Command::new("hostnamectl")
        .arg("set-hostname")
        .arg(&config.instance_name)
        .status()
        .expect("Error happened when trying to set hostname of master");
    tracing::info!("`hostname` set to master01");

    // Set `/etc/hosts` file.
    tracing::info!("Setting `/etc/hosts` according to configuration...");
    let mut hosts_file = File::options()
        .append(true)
        .open("/etc/hosts")
        .expect("Error happened when trying to append cluster information to `/etc/hosts`");
    for (ip, name) in &config.instance_hosts {
        writeln!(&mut hosts_file, "{} {}", ip, name)
            .expect("Error happened when trying to write `/etc/hosts`");
    }
    tracing::info!("`/etc/hosts` set");

    // Set IPv4 iptables.
    tracing::info!("Setting `/etc/sysctl.d/k8s.conf` according to configuration...");
    let mut k8s_conf = File::create("/etc/sysctl.d/k8s.conf")
        .expect("Error happened when trying to create `k8s.conf` file");
    k8s_conf
        .write_all(
            b"net.bridge.bridge-nf-call-ip6tables = 1\nnet.bridge.bridge-nf-call-iptables = 1\n",
        )
        .expect("Error happened when trying to write to `/etc/sysctl.d/k8s.conf`");
    Command::new("sysctl")
        .arg("--system")
        .status()
        .expect("Error happened when activating `k8s.conf`");
    tracing::info!("`/etc/sysctl.d/k8s.conf` set");

    // Synchronize time and date using `ntpdate`.
    // TODO
    tracing::info!("Synchronizing date and time...");
    tracing::info!("Time and date synchronized");
}
