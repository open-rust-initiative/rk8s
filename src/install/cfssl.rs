use std::process::Command;

use crate::config::Config;

pub fn start(config: &Config) {
    tracing::info!("Start downloading cfssl...");
    Command::new("curl")
        .arg("-L")
        .arg(&config.cfssl_url)
        .arg("-o")
        .arg("/usr/local/bin/cfssl")
        .status()
        .expect("Error happened when trying to download `cfssl`");
    tracing::info!("cfssl downloaded");
    Command::new("chmod")
        .arg("+x")
        .arg("/usr/local/bin/cfssl")
        .status()
        .expect("Error happened when trying to set `cfssl` executable");
    tracing::info!("cfssl is ready");

    tracing::info!("Start downloading cfssljson...");
    Command::new("curl")
        .arg("-L")
        .arg(&config.cfssljson_url)
        .arg("-o")
        .arg("/usr/local/bin/cfssljson")
        .status()
        .expect("Error happened when trying to download `cfssljson`");
    tracing::info!("cfssljson downloaded");
    Command::new("chmod")
        .arg("+x")
        .arg("/usr/local/bin/cfssljson")
        .status()
        .expect("Error happened when trying to set `cfssljson` executable");
    tracing::info!("cfssljson is ready");

    tracing::info!("Start downloading cfsslcertinfo...");
    Command::new("curl")
        .arg("-L")
        .arg(&config.cfsslcertinfo_url)
        .arg("-o")
        .arg("/usr/local/bin/cfssl-certinfo")
        .status()
        .expect("Error happened when trying to download `cfssl-certinfo`");
    tracing::info!("cfssl-certinfo downloaded");
    Command::new("chmod")
        .arg("+x")
        .arg("/usr/local/bin/cfssl-certinfo")
        .status()
        .expect("Error happened when trying to set `cfssl-certinfo` executable");
    tracing::info!("cfssl-certinfo is ready");
}
