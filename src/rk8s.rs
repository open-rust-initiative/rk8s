use crate::config;
use crate::config::Config;
use crate::deploy;
use crate::init;
use crate::install;
use crate::join;
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "rk8s")]
#[command(author = "Ruoqing He <timeprinciple@gmail.com>")]
#[command(version = "0.1.0")]
#[command(
    about = "A rust implementation of Kubeadm, easily bootstrap a secure Kubernetes cluster",
    long_about = "
    
Introduction:

    ┌──────────────────────────────────────────────────────────┐
    │ RK8S                                                     │
    │ A kubeadm implementation written in Rust                 │ 
    │ Easily bootstrap a secure Kubernetes cluster             │
    │                                                          │
    │ Please give us feedback at:                              │
    │                                                          │  
    └──────────────────────────────────────────────────────────┘

Example usage:

    Create a two-machine cluster with one control-plane node
    (which controls the cluster), and one worker node
    (where your workloads, like Pods and Deployments run).

    ┌──────────────────────────────────────────────────────────┐
    │ On the first machine:                                    │
    ├──────────────────────────────────────────────────────────┤
    │ control-plane# kubeadm init                              │
    └──────────────────────────────────────────────────────────┘

    ┌──────────────────────────────────────────────────────────┐
    │ On the second machine:                                   │
    ├──────────────────────────────────────────────────────────┤
    │ worker# kubeadm join <arguments-returned-from-init>      │
    └──────────────────────────────────────────────────────────┘

    You can then repeat the second step on as many other machines as you like.
"
)]
struct Cli {
    #[arg(long)]
    flag1: Option<String>,
    #[arg(long)]
    flag2: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Deploy,
    Precheck,
    Init,
    Join,
    Install { target: String },
    Generate { target: String },
}

pub fn run_command() {
    // Change working directory
    let work_root = Path::new("/rk8s");
    if !work_root.is_dir() {
        fs::create_dir("/rk8s").expect("Error happened when trying to create `/rk8s` directory");
        fs::create_dir("/rk8s/cfg")
            .expect("Error happened when trying to create `/rk8s/cfg` directory");
        fs::create_dir("/rk8s/etcd")
            .expect("Error happened when trying to create `/rk8s/etcd` directory");
        fs::create_dir("/rk8s/docker")
            .expect("Error happened when trying to create `/rk8s/docker` directory");
        fs::create_dir("/rk8s/k8s")
            .expect("Error happened when trying to create `/rk8s/k8s` directory");
        fs::create_dir("/rk8s/preparation")
            .expect("Error happened when trying to create `/rk8s/preparation` directory");
    }
    env::set_current_dir(work_root)
        .expect("Error happened when trying to change working directory");

    let cli = Cli::parse();

    match &cli.command {
        Commands::Deploy => {
            // Read configuration file.
            let adm_config = Config::init();
            deploy::pre_check::start(&adm_config);
            deploy::etcd::start(&adm_config);
            deploy::docker::start(&adm_config);
            deploy::prepare_kube::start(&adm_config);
            deploy::kube_apiserver::start(&adm_config);
            deploy::kube_controller_manager::start(&adm_config);
            deploy::kube_scheduler::start(&adm_config);
            deploy::kubectl::start(&adm_config);
            deploy::kubelet::start(&adm_config);
            deploy::kube_proxy::start(&adm_config);
        }
        Commands::Precheck => {
            // Read configuration file.
            let adm_config = Config::init();
            init::pre_check::start(&adm_config);
        }
        Commands::Init => {
            // Read configuration file.
            let adm_config = Config::init();
            tracing::info!("Init subcommand invoked.");
            init::etcd::start(&adm_config);
            init::kube_apiserver::start(&adm_config);
            init::kube_controller_manager::start(&adm_config);
            init::kube_scheduler::start(&adm_config);
            init::kube_ctl::start(&adm_config);
            init::kube_let::start(&adm_config);
            init::kube_proxy::start(&adm_config);
        }
        Commands::Join => {
            // Read configuration file.
            let adm_config = Config::init();
            tracing::info!("Join subcommand invoked.");
            join::etcd::start(&adm_config);
        }
        Commands::Install { target } => {
            // Read configuration file.
            let adm_config = Config::init();
            match target.as_str() {
                "etcd" => {
                    tracing::info!("Installing etcd...");
                    install::etcd::start(&adm_config);
                }
                "cfssl" => {
                    tracing::info!("Installing cfssl...");
                    install::cfssl::start();
                    tracing::info!("cfssl installation complete");
                }
                "docker" => {
                    tracing::info!("Installing docker...");
                    install::docker::start(&adm_config);
                    tracing::info!("docker installation complete");
                }
                "kubernetes" => {
                    tracing::info!("Installing kubernetes...");
                    install::kubernetes::start(&adm_config);
                    tracing::info!("kubernetes installation complete");
                }
                _ => {
                    tracing::info!("Unknown target");
                }
            }
        }
        Commands::Generate { target } => {
            // Generate `config_template` do not require reading configuration.
            match target.as_str() {
                "config" => {
                    tracing::info!("Generating `config-template.yaml`...");
                    config::generate_config_template();
                    tracing::info!("`config_template.yaml` generated under `rk8s/cfg`");
                }
                _ => {
                    tracing::info!("Unknown target");
                }
            }
        }
    }
}
