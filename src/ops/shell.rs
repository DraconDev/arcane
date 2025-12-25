use crate::ops::config::ServerConfig;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub struct Shell;

impl Shell {
    /// Execute a command locally and return output
    pub fn exec_local(cmd: &str) -> Result<String> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(String::new());
        }

        let output = Command::new(parts[0])
            .args(&parts[1..])
            .output()
            .context("Failed to exec local command")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Command failed: {}", err));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Execute a command on a remote server via SSH
    pub fn exec_remote(server: &ServerConfig, cmd: &str) -> Result<String> {
        // Build SSH command: ssh -p <port> -i <key> <user>@<host> <cmd>
        let mut ssh = Command::new("ssh");

        // Port
        if server.port > 0 {
            ssh.arg("-p").arg(server.port.to_string());
        }

        // Identity file
        if let Some(key) = &server.key_path {
            ssh.arg("-i").arg(key);
        }

        // Target
        let target = format!("{}@{}", server.user, server.host);
        ssh.arg(target);

        // Command
        ssh.arg(cmd);

        let output = ssh.output().context("SSH connection failed")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Remote command failed: {}", err));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Stream logs from a remote command (e.g. docker logs -f)
    /// Returns a Receiver channel that yields lines.
    pub fn stream_remote(server: &ServerConfig, cmd: &str) -> std::sync::mpsc::Receiver<String> {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut ssh = Command::new("ssh");
        // Port
        if server.port > 0 {
            ssh.arg("-p").arg(server.port.to_string());
        }
        // Identity file
        if let Some(key) = &server.key_path {
            ssh.arg("-i").arg(key);
        }
        // Target
        let target = format!("{}@{}", server.user, server.host);
        ssh.arg(target);
        // Command
        ssh.arg(cmd);

        // Spawn thread to read stdout
        std::thread::spawn(move || {
            if let Ok(mut child) = ssh.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        if let Ok(l) = line {
                            if tx.send(l).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        rx
    }
}
