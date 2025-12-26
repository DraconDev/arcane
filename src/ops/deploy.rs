use crate::ops::config::{OpsConfig, ServerConfig};
use crate::ops::shell::Shell;
use crate::security::ArcaneSecurity;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

pub struct ArcaneDeployer;

impl ArcaneDeployer {
    /// Deploy to a target (server or group).
    /// Supports Single Image (Blue/Green compatible) or Docker Compose (Rolling).
    pub async fn deploy(
        target_name: &str,
        // The primary reference: Image Name (for single) or App Name (for compose)
        deployment_ref: &str,
        env_name: &str,
        ports: Option<Vec<u16>>,
        compose_path: Option<String>,
        dry_run: bool,
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
                if let Err(e) = Self::deploy_target(
                    server_name,
                    deployment_ref,
                    env_name,
                    ports.clone(),
                    compose_path.clone(),
                    dry_run,
                )
                .await
                {
                    eprintln!("❌ Failed to deploy to {}: {}", server_name, e);
                    // Atomic mentality: stop on first error to prevent massive partial failure state
                    return Err(e);
                }
            }
            return Ok(());
        }

        // 2. Otherwise assume it's a single server
        Self::deploy_target(
            target_name,
            deployment_ref,
            env_name,
            ports,
            compose_path,
            dry_run,
        )
        .await
    }

    /// Internal helper for deploying to a single server.
    /// Dispatches to Compose or Single Image strategy.
    async fn deploy_target(
        server_name: &str,
        deployment_ref: &str,
        env_name: &str,
        ports: Option<Vec<u16>>,
        compose_path: Option<String>,
        dry_run: bool,
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

        // 2. Environment Safeguard
        if let Some(server_env) = &server.env {
            if server_env != env_name {
                println!(
                    "⚠️  WARNING: Server '{}' is configured for environment '{}', but you are deploying '{}'.",
                    server.name, server_env, env_name
                );
                // In dry run, we just warn. In real execution, we pause/prompt (handled by caller typically, but here specifically)
                if !dry_run {
                    println!("   Proceeding in 3 seconds... (Ctrl+C to cancel)");
                    std::thread::sleep(std::time::Duration::from_secs(3));
                }
            }
        }

        // 3. Decrypt Environment
        // We do this BEFORE lock to fail early if keys are missing
        println!("🔓 Decrypting environment '{}'...", env_name);

        // MOCK for Dry Run? No, we should test decryption even in dry run to validate config.
        let security = ArcaneSecurity::new(None)?;
        let repo_key = security.load_repo_key()?;
        let project_root = ArcaneSecurity::find_repo_root()?;
        let env =
            arcane::config::env::Environment::load(env_name, &project_root, &security, &repo_key)?;

        if dry_run {
            println!(
                "   [DRY RUN] Decryption successful. Loaded {} variables.",
                env.variables.len()
            );
        } else {
            println!(
                "   Decryption successful. Loaded {} variables.",
                env.variables.len()
            );
        }

        // 4. Acquire Lock
        println!("🔒 Acquiring distributed lock on {}...", server.host);
        let _lock_guard = DeployLock::acquire(server, dry_run).await?;
        if dry_run {
            println!("   [DRY RUN] Would hold lock.");
        } else {
            println!("   ✅ Lock acquired. Team safe.");
        }

        // 5. Build/Push & Deploy
        if let Some(compose_file) = compose_path {
            Self::deploy_compose(server, compose_file, deployment_ref, env.variables, dry_run)
                .await?;
        } else {
            Self::deploy_single_image(server, deployment_ref, env.variables, ports, dry_run)
                .await?;
        }

        Ok(())
    }

    /// Strategy: Docker Compose
    async fn deploy_compose(
        server: &ServerConfig,
        compose_path: String,
        app_name: &str, // used for folder name
        env_vars: HashMap<String, String>,
        dry_run: bool,
    ) -> Result<()> {
        println!("🚀 Initiating Compose Deploy for '{}'...", app_name);

        // 1. Prepare Remote Directory
        let remote_dir = format!("arcane/apps/{}", app_name);
        let mkdir_cmd = format!("mkdir -p {}", remote_dir);
        Shell::exec_remote(server, &mkdir_cmd, dry_run)?;

        // 2. Copy Docker Compose File
        println!("   📄 Uploading {}...", compose_path);
        if dry_run {
            println!(
                "   [DRY RUN] Would scp {} to {}/{}/docker-compose.yaml",
                compose_path, server.host, remote_dir
            );
        } else {
            // Read local file
            let content = fs::read(&compose_path).context("Failed to read local compose file")?;

            // Pipe content to remote file
            // ssh user@host 'cat > remote_path'
            Self::copy_bytes_to_remote(
                server,
                &content,
                &format!("{}/docker-compose.yaml", remote_dir),
            )?;
        }

        // 3. Helper: Create .env content
        let mut env_content = String::new();
        env_content.push_str("# Generated by Arcane\n");
        for (k, v) in env_vars {
            env_content.push_str(&format!("{}='{}'\n", k, v.replace("'", "'\\''")));
        }

        println!(
            "   🔑 Uploading .env ({} vars)...",
            env_content.lines().count()
        );
        if dry_run {
            println!(
                "   [DRY RUN] Would upload .env to {}/{}",
                server.host, remote_dir
            );
        } else {
            Self::copy_bytes_to_remote(
                server,
                env_content.as_bytes(),
                &format!("{}/.env", remote_dir),
            )?;
        }

        // 4. Run Docker Compose
        println!("   🐳 Running Docker Compose...");
        // Command: docker compose up -d --remove-orphans
        // We run inside the dir
        let up_cmd = format!("cd {} && docker compose up -d --remove-orphans", remote_dir);
        Shell::exec_remote(server, &up_cmd, dry_run)?;

        println!("✅ Compose Deployment Complete!");
        Ok(())
    }

    /// Strategy: Single Image (Supports Blue/Green)
    async fn deploy_single_image(
        server: &ServerConfig,
        image: &str,
        env_vars: HashMap<String, String>,
        ports: Option<Vec<u16>>,
        dry_run: bool,
    ) -> Result<()> {
        // 1.5 Auto-Build & Smoke Test (Garage Mode)
        // Only if NOT dry run? Or verify build in dry run too?
        // Let's Skip build in dry run to keep it fast.
        if !dry_run {
            println!("🏗️  Garage Mode: Building '{}' locally...", image);
            if let Err(e) = Shell::exec_local(&format!("docker build -t {} .", image), false) {
                return Err(anyhow::anyhow!("❌ Build Failed: {}", e));
            }

            println!("🩺 Running local smoke test...");
            // (Simple Smoke Test logic omitted for brevity/speed in dry run, kept for integration)
            let smoke_id = format!("smoke-{}", uuid::Uuid::new_v4());
            if let Err(e) = Shell::exec_local(
                &format!("docker run -d --name {} {}", smoke_id, image),
                false,
            ) {
                return Err(anyhow::anyhow!("❌ Smoke Test Start Failed: {}", e));
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
            let status = Shell::exec_local(
                &format!("docker inspect -f '{{{{.State.Running}}}}' {}", smoke_id),
                false,
            )
            .unwrap_or("false".into());
            let _ = Shell::exec_local(&format!("docker rm -f {}", smoke_id), false);
            if status.trim().replace("'", "") != "true" {
                return Err(anyhow::anyhow!("❌ Smoke Test Failed."));
            }
            println!("   ✅ Smoke Test Passed.");
        } else {
            println!("   [DRY RUN] Would build and smoke test image '{}'.", image);
        }

        // Push
        println!("   🚀 Pushing image via Warp Drive (Zstd)...");
        Shell::push_compressed_image(server, image, dry_run)?;

        // Construct Env Flags
        let mut env_flags: String = String::new();
        for (k, v) in env_vars {
            let safe_v = v.replace("'", "'\\''");
            env_flags.push_str(&format!(" -e {}='{}'", k, safe_v));
        }

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
                // Blue/Green Logic
                return Self::deploy_blue_green(
                    server, image, base_name, env_flags, ports, dry_run,
                )
                .await;
            }
        }

        // Standard Rolling Logic (stop, rename backup, start)
        Self::deploy_standard(server, image, base_name, env_flags, ports, dry_run).await
    }

    async fn deploy_blue_green(
        server: &ServerConfig,
        image: &str,
        base_name: &str,
        env_flags: String,
        ports: &Vec<u16>,
        dry_run: bool,
    ) -> Result<()> {
        // Enterprise Mode: Zero Downtime (Blue/Green + Caddy)
        let (blue_port, green_port) = (ports[0], ports[1]);
        let blue_name = format!("{}-blue", base_name);
        let green_name = format!("{}-green", base_name);

        // Check running status
        let blue_running_str = Shell::exec_remote(
            server,
            &format!("docker inspect -f '{{{{.State.Running}}}}' {}", blue_name),
            dry_run,
        )
        .unwrap_or_else(|_| "false".into());
        let blue_running = blue_running_str.trim() == "true";

        // If dry run, we assume Blue is running for simulation? Or just picking one.
        if dry_run {
            println!("   [DRY RUN] Assuming Blue status: {}", blue_running);
        }

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
        let _ = Shell::exec_remote(server, &format!("docker rm -f {}", target_name), dry_run);

        // Start Target
        let run_cmd = format!(
            "docker run -d --name {} -p {}:3000 --restart unless-stopped {} {}",
            target_name, target_port, env_flags, image
        );
        Shell::exec_remote(server, &run_cmd, dry_run)?;

        // Verify
        if !dry_run {
            println!("   🏥 Verifying health (5s)...");
            std::thread::sleep(std::time::Duration::from_secs(5));
            let check = Shell::exec_remote(
                server,
                &format!("docker inspect -f '{{{{.State.Running}}}}' {}", target_name),
                false,
            );
            if !matches!(check, Ok(ref s) if s.trim() == "true") {
                println!("   ❌ New version failed to start. Rolling back.");
                let _ = Shell::exec_remote(server, &format!("docker rm -f {}", target_name), false);
                return Err(anyhow::anyhow!(
                    "Deployment failed. Traffic stays on {}.",
                    old_name
                ));
            }
            println!("   ✅ {} is HEALTHY.", target_name);
        }

        // Swap Caddy
        println!(
            "   🔀 Swapping Caddy Upstream from :{} to :{}...",
            old_port, target_port
        );
        let caddy_cmd = format!(
            "sed -i 's/:{}/:{}/g' /etc/caddy/Caddyfile && caddy reload",
            old_port, target_port
        );
        Shell::exec_remote(server, &caddy_cmd, dry_run)?;

        // Cleanup Old
        println!("   🛑 Stopping {}...", old_name);
        let _ = Shell::exec_remote(server, &format!("docker rm -f {}", old_name), dry_run);

        Ok(())
    }

    async fn deploy_standard(
        server: &ServerConfig,
        image: &str,
        container_name: &str,
        env_flags: String,
        ports: Option<Vec<u16>>,
        dry_run: bool,
    ) -> Result<()> {
        // Construct Port Flag
        let port_flag = if let Some(p) = ports.as_ref().and_then(|v| v.first()) {
            format!("-p {}:3000", p)
        } else {
            String::new()
        };

        let backup_name = format!("{}_old", container_name);

        println!(
            "   📦 Backing up existing container to '{}'...",
            backup_name
        );
        // Rename if exists
        // Check if exists
        let check = Shell::exec_remote(
            server,
            &format!("docker inspect --type container {}", container_name),
            dry_run,
        );
        let has_existing = check.is_ok(); // Roughly accurate (if success, it exists)

        if has_existing {
            let _ = Shell::exec_remote(server, &format!("docker rm -f {}", backup_name), dry_run);
            Shell::exec_remote(
                server,
                &format!("docker rename {} {}", container_name, backup_name),
                dry_run,
            )?;
            Shell::exec_remote(server, &format!("docker stop {}", backup_name), dry_run)?;
        }

        println!("   ✨ Starting new container '{}'...", container_name);
        let run_cmd = format!(
            "docker run -d --name {} {} --restart unless-stopped {} {}",
            container_name, port_flag, env_flags, image
        );
        Shell::exec_remote(server, &run_cmd, dry_run)?;

        if !dry_run {
            println!("   🏥 Verifying health (5s)...");
            std::thread::sleep(std::time::Duration::from_secs(5));
            let check = Shell::exec_remote(
                server,
                &format!(
                    "docker inspect -f '{{{{.State.Running}}}}' {}",
                    container_name
                ),
                false,
            );
            // If failed... rollback (omitted for strict brevity but conceptually similar to original)
            if !matches!(check, Ok(ref s) if s.trim() == "true") {
                println!("   ❌ Failed.");
                // Rollback logic
                if has_existing {
                    let _ = Shell::exec_remote(
                        server,
                        &format!("docker rm -f {}", container_name),
                        false,
                    );
                    let _ = Shell::exec_remote(
                        server,
                        &format!("docker rename {} {}", backup_name, container_name),
                        false,
                    );
                    let _ = Shell::exec_remote(
                        server,
                        &format!("docker start {}", container_name),
                        false,
                    );
                }
                return Err(anyhow::anyhow!("Start failed. Rolled back."));
            }
            if has_existing {
                // cleanup backup
                let _ = Shell::exec_remote(server, &format!("docker rm -f {}", backup_name), false);
            }
        }

        println!("✅ Deployment Complete!");
        Ok(())
    }

    /// Helper to copy bytes to remote file via SSH pipe
    fn copy_bytes_to_remote(
        server: &ServerConfig,
        content: &[u8],
        remote_path: &str,
    ) -> Result<()> {
        let mut ssh = Command::new("ssh");
        if server.port > 0 {
            ssh.arg("-p").arg(server.port.to_string());
        }
        if let Some(key) = &server.key_path {
            ssh.arg("-i").arg(key);
        }
        ssh.arg(format!("{}@{}", server.user, server.host));
        ssh.arg(format!("cat > {}", remote_path));

        ssh.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = ssh.spawn().context("Failed to spawn ssh for file copy")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(content)
                .context("Failed to write content to ssh stdin")?;
        }

        let output = child.wait_with_output().context("Failed to wait on ssh")?;
        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "SCP (cat) failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }
}

// Helper struct for RAII locking
struct DeployLock<'a> {
    server: &'a ServerConfig,
    dry_run: bool,
}

impl<'a> DeployLock<'a> {
    async fn acquire(server: &'a ServerConfig, dry_run: bool) -> Result<Self> {
        let cmd = "mkdir /var/lock/arcane.deploy";
        match Shell::exec_remote(server, cmd, dry_run) {
            Ok(_) => Ok(Self { server, dry_run }),
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
        if self.dry_run {
            println!("   [DRY RUN] Would release lock.");
            return;
        }
        println!("🔓 Releasing lock...");
        let _ = Shell::exec_remote(self.server, "rmdir /var/lock/arcane.deploy", false);
    }
}
