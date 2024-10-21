/// Run with
/// ssh-runtime -n [host_name] -p [port] -u [user_name] -k [path_to_private_key]

use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use anyhow::Result;
use clap::Parser;
use log::info;
use ssh_runtime::SSHRuntimeManager;

mod ssh_session;
mod ssh_runtime;

#[derive(clap::Parser)]
pub struct Cli {
    #[clap(long, short = 'n')]
    host: String,

    #[clap(long, short = 'p', default_value_t = 22)]
    port: u16,

    #[clap(long, short = 'u')]
    username: String,

    #[clap(long, short = 'k')]
    private_key: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cli = Cli::parse();

    info!("Connecting to {}:{}", cli.host, cli.port);
    info!("Key path: {:?}", cli.private_key);

    let ssh_config = ssh_session::SSHConfig {
        hostname: cli.host,
        username: cli.username,
        port: cli.port,
        private_key_path: cli.private_key,
    };

    let mut runtime = SSHRuntimeManager::new(ssh_config).await;
    let pid = runtime.start_new_process("./launch.sh").await;
    sleep(Duration::from_secs(15));
    runtime.kill_process(pid.unwrap().as_str()).await?;
    runtime.shutdown().await?;

    Ok(())
}
