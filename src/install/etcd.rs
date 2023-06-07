use crate::config::Config;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn start(config: &Config) {
    tracing::info!("Start downloading etcd...");
    // Retrieve file name from URL.
    let fields: Vec<&str> = config.etcd_url.as_str().split('/').collect();
    let filename = fields[fields.len() - 1];
    let fields: Vec<&str> = filename.split(".tar.").collect();
    let folder_name = fields[0];
    Command::new("curl")
        .arg("-L")
        .arg(&config.etcd_url)
        .arg("-o")
        .arg(format!("etcd/{}.tar.gz", folder_name))
        .status()
        .expect("Error happened when trying to download `etcd`");
    if PathBuf::from(format!("/rk8s/etcd/{}.tar.gz", folder_name)).is_file() {
        tracing::info!("etcd downloaded");

        tracing::info!("untaring downloaded file");
        Command::new("tar")
            .arg("-zxf")
            .arg(format!("etcd/{}.tar.gz", folder_name))
            .arg("--directory")
            .arg("etcd")
            .status()
            .expect("Error happened when trying to untar `etcd` executable");

        let cfg_path = PathBuf::from("/opt/etcd/cfg");
        check_dir_exist_or_create(cfg_path);
        let bin_path = PathBuf::from("/opt/etcd/bin");
        check_dir_exist_or_create(bin_path);
        let ssl_path = PathBuf::from("/opt/etcd/ssl");
        check_dir_exist_or_create(ssl_path);

        Command::new("cp")
            .arg(format!("etcd/{}/etcd", folder_name))
            .arg(format!("etcd/{}/etcdctl", folder_name))
            .arg("/opt/etcd/bin")
            .status()
            .expect("Error happened when trying to copy `etcd` executable to `/opt/etcd/bin`");

        tracing::info!("etcd is ready");
    } else {
        tracing::error!("etcd not downloaded, please try again");
    }
}

fn check_dir_exist_or_create(path: PathBuf) {
    if !path.is_dir() {
        fs::create_dir_all(path).expect("Error happened when trying to create path");
    }
}
