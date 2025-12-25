use crate::ops::config::OpsConfig;
use crate::ops::shell::Shell;
use crate::security::ArcaneSecurity;
use anyhow::{Context, Result};
use std::path::Path;

pub struct ArcaneDeployer;

impl ArcaneDeployer {
    /// Deploy an image to a specific server with secrets injected from a local environment.
    pub async fn deploy(
        server_name: &str,
        image: &str,
        env_name: &str,
        ports: Option<Vec<u16>>,
    ) -> Result<()> {
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

        let env_str = if !env_path.exists() {
            println!(
                "   ⚠️  Environment file not found: {} (Deploying without extra secrets)",
                env_path.display()
            );
            String::new()
        } else {
            let content = std::fs::read(&env_path)?;
            let decrypted = security.decrypt_with_repo_key(&repo_key, &content)?;
            String::from_utf8(decrypted)?
        };

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

        // 0. Acquire Distributed Lock (RAII)
        println!("🔒 Acquiring distributed lock...");
        // Define scope for lock to ensure it drops at end of function
        let _lock_guard = DeployLock::acquire(server).await?;
        println!("   ✅ Lock acquired. Team safe.");

        // Push (Zstd Warp Drive)
        println!("   🚀 Pushing image via Warp Drive (Zstd)...");
        Shell::push_compressed_image(server, image)?;

        // Smart Swap Logic
        let base_name = image
            .split('/')
            .last()
            .unwrap_or("app")
            .split(':')
            .next()
            .unwrap_or("app");

        if let Some(ports) = ports {
            // Enterprise Mode: Zero Downtime (Blue/Green + Caddy)
            if ports.len() != 2 {
                return Err(anyhow::anyhow!(
                    "Zero Downtime requires exactly 2 ports (e.g. 8001,8002)"
                ));
            }
            let (blue_port, green_port) = (ports[0], ports[1]);
            let blue_name = format!("{}-blue", base_name);
            let green_name = format!("{}-green", base_name);

            // Check running status
            let blue_running = Shell::exec_remote(
                server,
                &format!("docker inspect -f '{{{{.State.Running}}}}' {}", blue_name),
            )
            .unwrap_or_else(|_| "false".into())
            .trim()
                == "true";

            // Determine Target
            let (target_color, target_port, target_name, old_name, old_port) = if blue_running {
                ("green", green_port, &green_name, &blue_name, blue_port)
            } else {
                ("blue", blue_port, &blue_name, &green_name, green_port)
            };

            println!(
                "   🔄 Zero Downtime Mode: Active is {}. Deploying to {} (:{})...",
                if blue_running { "Blue" } else { "Green" },
                target_color,
                target_port
            );

            // Cleanup target (in case of failed previous deploy)
            let _ = Shell::exec_remote(server, &format!("docker rm -f {}", target_name));

            // Start Target
            // NOTE: Assuming internal port 3000 for standard ARCANE Apps.
            let run_cmd = format!(
                "docker run -d --name {} -p {}:3000 --restart unless-stopped {} {}",
                target_name, target_port, env_flags, image
            );
            Shell::exec_remote(server, &run_cmd)?;

            // Verify
            println!("   🏥 Verifying health (5s)...");
            std::thread::sleep(std::time::Duration::from_secs(5));
            let check = Shell::exec_remote(
                server,
                &format!("docker inspect -f '{{{{.State.Running}}}}' {}", target_name),
            );

            if matches!(check, Ok(ref s) if s.trim() == "true") {
                println!("   ✅ {} is HEALTHY.", target_name);

                // Swap Caddy
                println!(
                    "   🔀 Swapping Caddy Upstream from :{} to :{}...",
                    old_port, target_port
                );
                // Safest approach: Replace OLD port with NEW port in global Caddyfile
                // Also ensures we replace ALL instances if multiple blocks exist? Risky.
                // We assume User has set up Caddyfile strictly for this app.
                // We also assume old_port is present. If bootstrapping (first ever deploy), user must set Caddyfile manually once?
                // Or we can try to replace EITHER port (Blue or Green) with Target.
                // sed -i 's/:<old>/:<new>/g; s/:<new>/:<new>/g' -- redundant but safe?
                // If Bootstrapping and Caddyfile points to 8000 (wrong), this won't help.
                // Bootstrapping: User must point Caddy to Blue or Green initially.

                let caddy_cmd = format!(
                    "sed -i 's/:{}/:{}/g' /etc/caddy/Caddyfile && caddy reload",
                    old_port, target_port
                );

                if let Err(e) = Shell::exec_remote(server, &caddy_cmd) {
                    println!("   ⚠️  Caddy Swap failed (Is Caddy installed?): {}", e);
                    println!("   ⚠️  Traffic MIGHT still be on Old version.");
                } else {
                    println!("   ✅ Caddy Reloaded.");
                    // Kill Old
                    println!("   🛑 Stopping {}...", old_name);
                    let _ = Shell::exec_remote(server, &format!("docker rm -f {}", old_name));

                    // Also kill legacy if exists
                    let _ = Shell::exec_remote(server, &format!("docker rm -f {}", base_name));
                }
            } else {
                // Rollback (Cleanup failed target)
                println!("   ❌ New version failed to start. Rolling back (cleaning up).");
                let _ = Shell::exec_remote(server, &format!("docker rm -f {}", target_name));
                return Err(anyhow::anyhow!(
                    "Deployment failed. Traffic stays on {}.",
                    old_name
                ));
            }
        } else {
            // Standard Smart Swap (Rename)
            // Fallback for non-Caddy deploys
            let container_name = base_name;
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
                                let _ = Shell::exec_remote(
                                    server,
                                    &format!("docker rm {}", backup_name),
                                );
                            }
                        }
                        _ => {
                            println!("   ❌ Container FAILED to start/stay running!");
                            println!("   Running rollback...");

                            // Stop new (failed)
                            let _ = Shell::exec_remote(
                                server,
                                &format!("docker rm -f {}", container_name),
                            );

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
                        let _ =
                            Shell::exec_remote(server, &format!("docker start {}", container_name));
                    }
                    return Err(e);
                }
            }
        }

        println!("✅ Deployment Complete!");
        Ok(())
    }
}

// Helper struct for RAII locking
struct DeployLock<'a> {
    server: &'a crate::ops::config::ServerConfig,
}

impl<'a> DeployLock<'a> {
    async fn acquire(server: &'a crate::ops::config::ServerConfig) -> Result<Self> {
        let cmd = "mkdir /var/lock/arcane.deploy";
        match Shell::exec_remote(server, cmd) {
            Ok(_) => Ok(Self { server }),
            Err(e) => Err(anyhow::anyhow!(
                "⚠️  Deployment Locked! (or SSH Error): {}\n   If you are sure no one is deploying, run: ssh {} 'rmdir /var/lock/arcane.deploy'",
                e,
                server.host
            )),
        }
    }
}

impl<'a> Drop for DeployLock<'a> {
    fn drop(&mut self) {
        println!("🔓 Releasing lock...");
        // Best effort cleanup. We use 'rmdir' to remove the directory we created.
        // We do this synchronously in drop, which technically blocks the thread slightly if we were async,
        // but Drop is sync. Ideally we'd spawn a task, but we want to ensure it runs ensuring we don't leave locks.
        // Since Shell::exec_remote is synchronous (std::process), this is fine.
        let _ = Shell::exec_remote(self.server, "rmdir /var/lock/arcane.deploy");
    }
}
