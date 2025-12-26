use crate::ops::config::OpsConfig;
use crate::ops::shell::Shell;
use crate::security::ArcaneSecurity;
use anyhow::{Context, Result};
use arcane::config::env::Environment;

pub struct ArcaneDeployer;

impl ArcaneDeployer {
    /// Deploy an image to a target (server or group) with secrets injected from a local environment.
    pub async fn deploy(
        target_name: &str,
        image: &str,
        env_name: &str,
        ports: Option<Vec<u16>>,
    ) -> Result<()> {
        let config = OpsConfig::load();

        // 1. Check if target is a group
        if let Some(group) = config.groups.iter().find(|g| g.name == target_name) {
            println!(
                "🌐 Target is a Group: {}. Deploying to {} servers...",
                group.name,
                group.servers.len()
            );
            for server_name in &group.servers {
                println!("\n--- Deploying to member: {} ---", server_name);
                if let Err(e) =
                    Self::deploy_single(server_name, image, env_name, ports.clone()).await
                {
                    eprintln!("❌ Failed to deploy to {}: {}", server_name, e);
                    // Continue to next server? Or stop?
                    // In a sovereign system, maybe we stop if it's a critical failure, but for "push to many",
                    // we usually want to try all. For now, let's stop on first error to be safe (Atomic mentality).
                    return Err(e);
                }
            }
            return Ok(());
        }

        // 2. Otherwise assume it's a single server
        Self::deploy_single(target_name, image, env_name, ports).await
    }

    /// Internal helper for deploying to a single server.
    pub async fn deploy_single(
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

        if let Some(server_env) = &server.env {
            if server_env != env_name {
                println!("⚠️  WARNING: Server '{}' is configured for environment '{}', but you are deploying '{}'.", server.name, server_env, env_name);
                println!("   Proceeding in 3 seconds... (Ctrl+C to cancel)");
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
        }

        // 1.5 Auto-Build & Smoke Test (Garage Mode)
        println!("🏗️  Garage Mode: Building '{}' locally...", image);
        if let Err(e) = Shell::exec_local(&format!("docker build -t {} .", image)) {
            return Err(anyhow::anyhow!("❌ Build Failed: {}", e));
        }

        println!("🩺 Running local smoke test...");
        let smoke_id = format!("smoke-{}", uuid::Uuid::new_v4());

        // Run with default entrypoint, but force stop it after 3s.
        if let Err(e) = Shell::exec_local(&format!("docker run -d --name {} {}", smoke_id, image)) {
            return Err(anyhow::anyhow!("❌ Smoke Test Start Failed: {}", e));
        }
        std::thread::sleep(std::time::Duration::from_secs(3));

        let status = Shell::exec_local(&format!(
            "docker inspect -f '{{{{.State.Running}}}}' {}",
            smoke_id
        ))
        .unwrap_or("false".into());

        let clean_status = status.trim().replace("'", "");
        if clean_status != "true" {
            let logs = Shell::exec_local(&format!("docker logs --tail 20 {}", smoke_id))
                .unwrap_or_default();
            let _ = Shell::exec_local(&format!("docker rm -f {}", smoke_id));
            return Err(anyhow::anyhow!(
                "❌ Smoke Test Failed: Status='{}'. Logs:\n{}",
                clean_status,
                logs
            ));
        }
        let _ = Shell::exec_local(&format!("docker rm -f {}", smoke_id));
        println!("   ✅ Smoke Test Passed.");

        println!("🚀 Initiating Sovereign Deploy to {}", server.host);
        println!("   Image: {}", image);
        println!("   Secrets Environment: {}", env_name);

        // 2. Decrypt Secrets & Load Environment
        println!("🔓 Decrypting environment '{}'...", env_name);
        let security = ArcaneSecurity::new(None)?;
        let repo_key = security.load_repo_key()?;

        // Find repo root to locate config/
        let project_root = ArcaneSecurity::find_repo_root()?;

        // Load the environment (handles base.env + specific.env + encryption)
        let env =
            arcane::config::env::Environment::load(env_name, &project_root, &security, &repo_key)?;

        // 3. Construct Docker Flags
        let mut env_flags: String = String::new();
        let mut count = 0;
        for (k, v) in env.variables {
            // Escape single quotes for shell safety
            let safe_v = v.replace("'", "'\\''");
            env_flags.push_str(&format!(" -e {}='{}'", k, safe_v));
            count += 1;
        }
        println!("   Injected {} secrets into RAM payload.", count);

        // 4. Execute Remote Commands
        println!("📡 Connecting to {}...", server.host);

        // 0. Acquire Distributed Lock (RAII)
        println!("🔒 Acquiring distributed lock...");
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

        // Logic Split: Blue/Green vs Standard
        if let Some(ports) = &ports {
            if ports.len() == 2 {
                // Enterprise Mode: Zero Downtime (Blue/Green + Caddy)
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

                // Cleanup target
                let _ = Shell::exec_remote(server, &format!("docker rm -f {}", target_name));

                // Start Target
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
                    // Secondary HTTP Check
                    let _ = Shell::exec_remote(
                        server,
                        &format!(
                            "docker exec {} sh -c \"curl -f http://localhost:3000/health || curl -f http://localhost:3000/ || wget -qO- http://localhost:3000/health || wget -qO- http://localhost:3000/\"",
                            target_name
                        ),
                    )
                    .map(|_| println!("   ❇️  HTTP Health Check: PASS"));

                    println!("   ✅ {} is HEALTHY.", target_name);

                    // Swap Caddy
                    println!(
                        "   🔀 Swapping Caddy Upstream from :{} to :{}...",
                        old_port, target_port
                    );

                    let caddy_cmd = format!(
                        "sed -i 's/:{}/:{}/g' /etc/caddy/Caddyfile && caddy reload",
                        old_port, target_port
                    );

                    if let Err(e) = Shell::exec_remote(server, &caddy_cmd) {
                        println!("   ⚠️  Caddy Swap failed (Is Caddy installed?): {}", e);
                        println!("   ⚠️  Traffic MIGHT still be on Old version.");
                    } else {
                        println!("   ✅ Caddy Reloaded.");
                        println!("   🛑 Stopping {}...", old_name);
                        let _ = Shell::exec_remote(server, &format!("docker rm -f {}", old_name));
                        // Also kill legacy if exists
                        let _ = Shell::exec_remote(server, &format!("docker rm -f {}", base_name));
                    }
                    println!("✅ Deployment Complete!");
                    return Ok(());
                } else {
                    // Rollback
                    println!("   ❌ New version failed to start. Rolling back.");
                    let _ = Shell::exec_remote(server, &format!("docker rm -f {}", target_name));
                    return Err(anyhow::anyhow!(
                        "Deployment failed. Traffic stays on {}.",
                        old_name
                    ));
                }
            } else if ports.len() > 1 {
                return Err(anyhow::anyhow!(
                    "Invalid ports: provide 1 (Standard) or 2 (Blue/Green)."
                ));
            }
        }

        // Standard Mode (Fallthrough)
        let container_name = base_name;
        let backup_name = format!("{}_old", container_name);

        // Construct Port Flag (if single port provided)
        let port_flag = if let Some(p) = ports.as_ref().and_then(|v| v.first()) {
            format!("-p {}:3000", p)
        } else {
            String::new()
        };

        // 1. Rename existing to backup (if exists)
        println!(
            "   📦 Backing up existing container to '{}'...",
            backup_name
        );
        let check_exists = Shell::exec_remote(
            server,
            &format!("docker inspect --type container {}", container_name),
        );
        let has_existing = check_exists.is_ok();

        if has_existing {
            let _ = Shell::exec_remote(server, &format!("docker rm -f {}", backup_name));
            Shell::exec_remote(
                server,
                &format!("docker rename {} {}", container_name, backup_name),
            )?;
            Shell::exec_remote(server, &format!("docker stop {}", backup_name))?;
        }

        // 2. Start New Container
        println!("   ✨ Starting new container '{}'...", container_name);
        let run_cmd = format!(
            "docker run -d --name {} {} --restart unless-stopped {} {}",
            container_name, port_flag, env_flags, image
        );

        match Shell::exec_remote(server, &run_cmd) {
            Ok(_) => {
                // 3. Health Check
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
                        let _ = Shell::exec_remote(
                                server,
                                &format!(
                                    "docker exec {} sh -c \"curl -f http://localhost:3000/health || curl -f http://localhost:3000/ || wget -qO- http://localhost:3000/health || wget -qO- http://localhost:3000/\"",
                                    container_name
                                ),
                            ).map(|_| println!("   ❇️  HTTP Health Check: PASS"));

                        println!("   ✅ Container is HEALTHY.");
                        if has_existing {
                            println!("   🗑️  Removing backup '{}'...", backup_name);
                            let _ =
                                Shell::exec_remote(server, &format!("docker rm {}", backup_name));
                        }
                    }
                    _ => {
                        println!("   ❌ Container FAILED to start!");
                        println!("   Running rollback...");
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
                            println!("   ✅ Rollback Complete.");
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
