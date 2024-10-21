use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use client::Msg;
use russh::keys::*;
use russh::*;
use tokio::io::AsyncWriteExt;

pub struct SSHConfig {
    pub hostname: String,
    pub username: String,
    pub port: u16,
    pub private_key_path: PathBuf,
}

struct Client {}

#[async_trait]
impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct Session {
    session: client::Handle<Client>,
}

impl Session {
    pub async fn connect(ssh_config: &SSHConfig) -> Result<Self> {
        let key_pair = load_secret_key(ssh_config.private_key_path.clone(), None)?;
        let config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(120)),
            keepalive_interval: Some(Duration::from_secs(30)),
            ..<_>::default()
        };

        let config = Arc::new(config);
        let sh = Client {};

        let addrs = (ssh_config.hostname.clone(), ssh_config.port);
        let mut session = client::connect(config, addrs, sh).await?;
        let auth_res = session
            .authenticate_publickey(ssh_config.username.clone(), Arc::new(key_pair))
            .await?;

        if !auth_res {
            anyhow::bail!("Authentication failed");
        }

        Ok(Self { session })
    }

    pub async fn open_channel(&self) -> Channel<Msg> {
        let channel = self.session.channel_open_session().await;
        return Result::expect(channel, "Failed to open channel");
    }

    pub async fn exec_command(&mut self, command: &str) -> Result<u32> {
        let mut channel = self.session.channel_open_session().await?;
        channel.exec(true, command).await?;

        let mut code = None;
        let mut stdout = tokio::io::stdout();

        loop {
            let Some(msg) = channel.wait().await else {
                break;
            };
            match msg {
                ChannelMsg::Data { ref data } => {
                    stdout.write_all(data).await?;
                    stdout.flush().await?;
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    code = Some(exit_status);
                }
                _ => {}
            }
        }
        Ok(code.expect("program did not exit cleanly"))
    }

    pub async fn close(&mut self) -> Result<()> {
        self.session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await?;
        Ok(())
    }
}