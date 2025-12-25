use crate::ops::config::OpsConfig;
use crate::ops::shell::Shell;
use crate::security::ArcaneSecurity;
use anyhow::{Context, Result};
use std::path::Path;

pub struct ArcaneDeployer;

impl ArcaneDeployer {
    /// Deploy an image to a specific server with secrets injected from a local environment.
    pub async fn deploy(server_name: &str, image: &str, env_name: &str) -> Result<()> {
        // 1. Load Server Config
        let config = OpsConfig::load();
        let server = config
            .servers
            .iter()
            .find(|s| s.name == server_name)
            .context(format!(
                "Server '{}' not found in configuration",
                server_name
            ))?;

        println!("🚀 Initiating Sovereign Deploy to {}", server.host);
        println!("   Image: {}", image);
        println!("   Secrets Environment: {}", env_name);

        // 2. Decrypt Secrets in Memory
        println!("🔓 Decrypting secrets...");
        let security = ArcaneSecurity::new(None)?;
        let repo_key = security.load_repo_key()?;

        let env_path = Path::new("config")
            .join("envs")
            .join(format!("{}.env", env_name));
        if !env_path.exists() {
            return Err(anyhow::anyhow!(
                "Environment file not found: {}",
                env_path.display()
            ));
        }

        let content = std::fs::read(&env_path)?;
        let decrypted = security.decrypt_with_repo_key(&repo_key, &content)?;
        let env_str = String::from_utf8(decrypted)?;

        // 3. Construct Docker Flags
        let mut env_flags = String::new();
        // Always inject ARCANE_MACHINE_KEY if found (we might need a designated one or just skip if using runtime decryption)
        // For this version, let's inject ALL decrypted variables as -e flags.
        // This is "Ram Injection" mode.

        // Also inject the Machine Key itself if we have one for this server?
        // Actually, the "Sovereign Handshake" implies we pass the Key to allow the APP to decrypt other files.
        // But here we are decrypting LOCALLY and injecting. That's simpler for "Phase 1".

        let mut count = 0;
        for line in env_str.lines() {
            if let Some((k, v)) = line.split_once('=') {
                let k = k.trim();
                let v = v.trim();
                if !k.is_empty() && !k.starts_with('#') {
                    // Escape single quotes for shell safety
                    let safe_v = v.replace("'", "'\\''");
                    env_flags.push_str(&format!(" -e {}='{}'", k, safe_v));
                    count += 1;
                }
            }
        }
        println!("   Injected {} secrets into RAM payload.", count);

        // 4. Execute Remote Commands
        println!("📡 Connecting to {}...", server.host);

        // Pull
        println!("   Pulling image...");
        Shell::exec_remote(server, &format!("docker pull {}", image))?;

        // Smart Swap Logic
        let container_name = image
            .split('/')
            .last()
            .unwrap_or("app")
            .split(':')
            .next()
            .unwrap_or("app");
        let backup_name = format!("{}_old", container_name);

        // 1. Rename existing to backup (if exists)
        println!(
            "   📦 Backing up existing container to '{}'...",
            backup_name
        );
        // Ignore error if it doesn't exist (fresh deploy), but if it exists, rename it.
        // We check if it exists via 'docker inspect'. If fails, we assume valid fresh deploy.
        let check_exists =
            Shell::exec_remote(server, &format!("docker inspect {}", container_name));
        let has_existing = check_exists.is_ok();

        if has_existing {
            // Check if backup already exists from a broken run? clean it up
            let _ = Shell::exec_remote(server, &format!("docker rm -f {}", backup_name));

            // Rename current -> backup
            Shell::exec_remote(
                server,
                &format!("docker rename {} {}", container_name, backup_name),
            )?;
            // Stop backup to free ports
            Shell::exec_remote(server, &format!("docker stop {}", backup_name))?;
        }

        // 2. Start New Container
        println!("   ✨ Starting new container '{}'...", container_name);
        let run_cmd = format!(
            "docker run -d --name {} --restart unless-stopped {} {}",
            container_name, env_flags, image
        );

        match Shell::exec_remote(server, &run_cmd) {
            Ok(_) => {
                // 3. Health Check / Verification
                println!("   🏥 Verifying health (5s)...");
                std::thread::sleep(std::time::Duration::from_secs(5));

                let check = Shell::exec_remote(
                    server,
                    &format!(
                        "docker inspect -f '{{{{.State.Running}}}}' {}",
                        container_name
                    ),
                );

                match check {
                    Ok(output) if output.trim() == "true" => {
                        println!("   ✅ Container is HEALTHY.");
                        if has_existing {
                            println!("   🗑️  Removing backup '{}'...", backup_name);
                            let _ =
                                Shell::exec_remote(server, &format!("docker rm {}", backup_name));
                        }
                    }
                    _ => {
                        println!("   ❌ Container FAILED to start/stay running!");
                        println!("   Running rollback...");

                        // Stop new (failed)
                        let _ =
                            Shell::exec_remote(server, &format!("docker rm -f {}", container_name));

                        if has_existing {
                            println!("   🔄 Restoring '{}'...", backup_name);
                            let _ = Shell::exec_remote(
                                server,
                                &format!("docker rename {} {}", backup_name, container_name),
                            );
                            let _ = Shell::exec_remote(
                                server,
                                &format!("docker start {}", container_name),
                            );
                            println!(
                                "   ✅ Rollback Complete. Service restored to previous version."
                            );
                        }
                        return Err(anyhow::anyhow!(
                            "Deployment failed verification. Rolled back."
                        ));
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Docker Run Failed: {}", e);
                if has_existing {
                    println!("   🔄 Restoring backup...");
                    let _ = Shell::exec_remote(
                        server,
                        &format!("docker rename {} {}", backup_name, container_name),
                    );
                    let _ = Shell::exec_remote(server, &format!("docker start {}", container_name));
                }
                return Err(e);
            }
        }

        println!("✅ Deployment Complete!");
        Ok(())
    }
}
