mod config;
mod deploy;
mod init;
mod install;
mod join;
mod rk8s;

use rk8s::run_command;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("rk8s started");
    run_command();
}
