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

        // Build SSH command (simple version for stdbuf)
        // We'll trust the caller provided valid server details to exec_remote logic
        // but re-implement minimal here for streaming.

        // Actually, to avoid code duplication and complex pipe handling in 3 different ways,
        // let's stick to the simplest spawning implementation.

        let mut ssh = Command::new("ssh");
        if server.port > 0 {
            ssh.arg("-p").arg(server.port.to_string());
        }
        if let Some(key) = &server.key_path {
            ssh.arg("-i").arg(key);
        }
        let target = format!("{}@{}", server.user, server.host);
        ssh.arg(target);
        ssh.arg(cmd);

        // Spawn thread to read stdout
        std::thread::spawn(move || {
            // ... (existing implementation)
            if let Ok(mut child) = ssh.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
                // ... handle stdout
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        if let Ok(l) = line {
                            let _ = tx.send(l);
                        }
                    }
                }
            }
        });

        rx
    }

    /// Push a local Docker image to a remote server using Zstd compression.
    /// Pipeline: docker save <image> | zstd -T0 -3 | ssh <server> 'zstd -d | docker load'
    pub fn push_compressed_image(server: &ServerConfig, image: &str) -> Result<()> {
        // 1. Check local zstd
        if Command::new("zstd").arg("--version").output().is_err() {
            return Err(anyhow::anyhow!("'zstd' not found locally. Please install it: sudo apt install zstd / brew install zstd"));
        }

        // 2. Build SSH Command string for sh -c
        let mut ssh_base = String::from("ssh");
        if server.port > 0 {
            ssh_base.push_str(&format!(" -p {}", server.port));
        }
        if let Some(key) = &server.key_path {
            ssh_base.push_str(&format!(" -i {}", key));
        }
        let target = format!("{}@{}", server.user, server.host);

        // 3. Construct Pipeline
        // Note: We use -T0 to use all cores for compression. -3 is standard level.
        // On remote: zstd -d (decompress) | docker load
        let pipeline = format!(
            "docker save {} | zstd -T0 -3 | {} {} 'zstd -d | docker load'",
            image, ssh_base, target
        );

        // 4. Exec via shell
        println!("   ⚡ Executing Warp Drive: {}", pipeline);
        let output = Command::new("sh")
            .arg("-c")
            .arg(&pipeline)
            .output()
            .context("Failed to execute push pipeline")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Push Failed: {}", err));
        }

        Ok(())
    }
}
