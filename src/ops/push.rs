use crate::ops::config::OpsConfig;
use crate::security::ArcaneSecurity;
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

pub struct PushDeploy;

impl PushDeploy {
    /// Pushes the current repo to the target server defined in servers.toml.
    pub fn deploy(target_alias: &str) -> Result<()> {
        let config = OpsConfig::load();
        let server = config
            .servers
            .iter()
            .find(|s| s.name == target_alias)
            .ok_or_else(|| anyhow!("Server '{}' not found in servers.toml", target_alias))?;

        println!(
            "🚀 Preparing deployment for '{}' ({})",
            target_alias, server.host
        );

        // 1. Prepare Staging Temp Dir
        let temp_dir = std::env::temp_dir().join(format!("arcane-deploy-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir)?;

        let result = Self::stage_and_push(&temp_dir, &server.user, &server.host);

        let _ = std::fs::remove_dir_all(&temp_dir);
        result
    }

    fn stage_and_push(temp_dir: &Path, user: &str, host: &str) -> Result<()> {
        let archive_path = temp_dir.join("bundle.tar");

        // 2a. Git Archive (Export HEAD -> bundle.tar)
        println!("📦 Bundling repository (HEAD)...");
        let status = Command::new("git")
            .args(&[
                "archive",
                "--format=tar",
                "-o",
                archive_path.to_str().unwrap(),
                "HEAD",
            ])
            .status()
            .context("git archive failed")?;

        if !status.success() {
            return Err(anyhow!("git archive failed"));
        }

        // 2b. Inject Decrypted Secrets (.env)
        println!("🔓 Injecting decrypted secrets...");
        let security = ArcaneSecurity::new(None)?;
        let env_path = Path::new(".env");
        if env_path.exists() {
            let temp_env_path = temp_dir.join(".env");
            if let Ok(repo_key) = security.load_repo_key() {
                if let Ok(ciphertext) = std::fs::read(env_path) {
                    match security.decrypt_with_repo_key(&repo_key, &ciphertext) {
                        Ok(plaintext) => {
                            std::fs::write(&temp_env_path, plaintext)?;
                            println!("   - Decrypted .env successfully");

                            // Append to tar: tar -rf bundle.tar -C temp_dir .env
                            let status = Command::new("tar")
                                .args(&[
                                    "-rf",
                                    archive_path.to_str().unwrap(),
                                    "-C",
                                    temp_dir.to_str().unwrap(),
                                    ".env",
                                ])
                                .status()?;

                            if !status.success() {
                                eprintln!("   ⚠️ Failed to inject .env into tarball.");
                            }
                        }
                        Err(e) => {
                            println!("   ⚠️ .env decryption failed ({}), skipping injection.", e);
                        }
                    }
                }
            }
        }

        // 3. Compress (Gzip) & Ship (SSH)
        println!("🚚 Shipping to {}@{}...", user, host);
        let remote_dir = "arcane_deploy";

        let cat_cmd = Command::new("cat")
            .arg(&archive_path)
            .stdout(Stdio::piped())
            .spawn()?;

        let gzip_cmd = Command::new("gzip")
            .stdin(cat_cmd.stdout.unwrap())
            .stdout(Stdio::piped())
            .spawn()?;

        let ssh_remote_cmd = format!("mkdir -p {} && tar -xz -C {}", remote_dir, remote_dir);
        let mut ssh_process = Command::new("ssh")
            .args(&[&format!("{}@{}", user, host), &ssh_remote_cmd])
            .stdin(gzip_cmd.stdout.unwrap())
            .spawn()?;

        let status = ssh_process.wait()?;
        if !status.success() {
            return Err(anyhow!("SSH transfer failed"));
        }

        // 4. Execute Start Script
        println!("🔥 Executing startup script...");
        let start_cmd = format!("cd {} && if [ -f ./start.sh ]; then chmod +x ./start.sh && ./start.sh; else echo 'No start.sh found'; fi", remote_dir);

        let status = Command::new("ssh")
            .args(&[&format!("{}@{}", user, host), &start_cmd])
            .status()?;

        if status.success() {
            println!("✅ Deployment Complete!");
            Ok(())
        } else {
            Err(anyhow!("Startup script failed"))
        }
    }
}
