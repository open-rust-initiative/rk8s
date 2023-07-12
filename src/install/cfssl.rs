use std::env;
use std::path::Path;
use std::process::Command;

pub fn start() {
    tracing::info!("Start Installing cfssl...");
    let prev_dir = Path::new("/rk8s");
    let work_dir = Path::new("/rk8s/preparation");
    env::set_current_dir(work_dir)
        .expect("Error happened when trying to change into `praparation`");
    tracing::info!("Changed to {}", env::current_dir().unwrap().display());
    Command::new("cp")
        .arg("cfssl")
        .arg("/usr/local/bin/cfssl")
        .status()
        .expect("Error happened when trying to copy `cfssl`");
    tracing::info!("cfssl copied");
    Command::new("chmod")
        .arg("+x")
        .arg("/usr/local/bin/cfssl")
        .status()
        .expect("Error happened when trying to set `cfssl` executable");
    tracing::info!("cfssl is ready");

    tracing::info!("Start installing cfssljson...");
    Command::new("cp")
        .arg("cfssljson")
        .arg("/usr/local/bin/cfssljson")
        .status()
        .expect("Error happened when trying to copy `cfssljson`");
    tracing::info!("cfssljson copied");
    Command::new("chmod")
        .arg("+x")
        .arg("/usr/local/bin/cfssljson")
        .status()
        .expect("Error happened when trying to set `cfssljson` executable");
    tracing::info!("cfssljson is ready");

    tracing::info!("Start installing cfsslcertinfo...");
    Command::new("cp")
        .arg("cfssl-certinfo")
        .arg("/usr/local/bin/cfssl-certinfo")
        .status()
        .expect("Error happened when trying to copy `cfssl-certinfo`");
    tracing::info!("cfssl-certinfo copied");
    Command::new("chmod")
        .arg("+x")
        .arg("/usr/local/bin/cfssl-certinfo")
        .status()
        .expect("Error happened when trying to set `cfssl-certinfo` executable");
    tracing::info!("cfssl-certinfo is ready");

    env::set_current_dir(prev_dir)
        .expect("Error happened when trying to change into `praparation`");
    tracing::info!(
        "Change working directory back to {}",
        env::current_dir().unwrap().display()
    );
}
