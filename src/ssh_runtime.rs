use anyhow::{Error, Result};
use russh::ChannelMsg;
use regex::Regex;

use crate::info;
use crate::ssh_session::{SSHConfig, Session};

pub struct SSHRuntimeManager {
    ssh_config: SSHConfig,
    ssh_session: Session,
}

impl SSHRuntimeManager {
    pub async fn new(ssh_config: SSHConfig) -> Self {
        let ssh_result = Session::connect(&ssh_config).await;

        let ssh_session = match ssh_result {
            Ok(ssh) => ssh,
            Err(error) => panic!("Failed to connect to the host. {}", error),
        };
        info!("Connected");

        Self{ssh_config: ssh_config, ssh_session: ssh_session}
    }

    pub async fn start_new_process<'a>(&self, command: &str) -> Result<String, Error> {
        info!("starting a new process");
        let mut channel = self.ssh_session.open_channel().await;
        channel.exec(true, format!("setsid {}", command)).await?;

        let pid_str = "";
        let mut pid_opt = None;

        loop {
            let Some(msg) = channel.wait().await else {
                break;
            };
            match msg {
                ChannelMsg::Data { ref data } => {
                    // let mut msg_str: &'a str;
                    let msg_str = String::from_utf8(data.to_ascii_lowercase()).unwrap();
                    let re = Regex::new(r"server is on host (.*) on pid (\d+)").unwrap();
                    if let Some(cap) = re.captures(msg_str.as_str()) {
                        if let Some(pid) = cap.get(2) {
                            info!("started a new process, PID: {}", pid.as_str());
                            return Ok(pid.as_str().to_string());
                        }
                    }
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    if exit_status == 0 {
                        pid_opt = Some(String::from(pid_str));
                    }
                }
                _ => {}
            }
        }

        Ok(pid_opt.expect("program did not exit cleanly"))
    }

    pub async fn kill_process(&mut self, pid: &str) -> Result<()> {
        let res = self.ssh_session.exec_command(format!("kill {}", pid).as_str()).await;
        match res {
            Ok(_) => {
                info!("killed process {}", pid);
            },
            Err(err) => {
                info!("failed to kill process {}\n{}", pid, err);
            },
        }
        
        Ok(())
    }

    // pub async fn pwd(&mut self) {
    //     let _ = self.ssh_session.exec_command("pwd").await;
    // }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.ssh_session.close().await?;
        Ok(())
    }
}