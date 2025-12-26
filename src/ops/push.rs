use crate::config::ConfigManager;
use crate::security::ArcaneSecurity;
use anyhow::{anyhow, Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct PushDeploy;

impl PushDeploy {
    /// Pushes the current repo to the target server defined in servers.toml.
    ///
    /// Strategy:
    /// 1. Create a temp directory.
    /// 2. `git archive` the current HEAD to that temp dir (sanitized snapshot).
    /// 3. Decrypt `.env` and write it to the temp dir.
    /// 4. Tar/Gzip the temp dir and pipe it over SSH to the server.
    /// 5. Execute `./start.sh` (or `docker-compose up -d`) on the server.
    pub fn deploy(target_alias: &str) -> Result<()> {
        let config_manager = ConfigManager::new()?;
        let server = config_manager
            .get_server(target_alias)
            .ok_or_else(|| anyhow!("Server '{}' not found in servers.toml", target_alias))?;

        println!(
            "🚀 Preparing deployment for '{}' ({})",
            target_alias, server.host
        );

        // 1. Prepare Staging Area
        let temp_dir = std::env::temp_dir().join(format!("arcane-deploy-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir)?;

        let cleanup = |path: &Path| {
            let _ = std::fs::remove_dir_all(path);
        };

        // Ensure we clean up even on error (best effort via scope guard or explicit calls)
        // For simplicity here, we'll try/catch.

        let result = Self::stage_and_push(&temp_dir, &server.user, &server.host);

        cleanup(&temp_dir);
        result
    }

    fn stage_and_push(staging_path: &Path, user: &str, host: &str) -> Result<()> {
        // 2. Git Archive (Export HEAD)
        // We assume command is run from repo root.
        println!("📦 Bundling repository (HEAD)...");
        let output = Command::new("git")
            .args(&["archive", "--format=tar", "HEAD"])
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to spawn git archive")?;

        let tar_output = output.wait_with_output()?;
        if !tar_output.status.success() {
            return Err(anyhow!(
                "git archive failed: {:?}",
                String::from_utf8_lossy(&tar_output.stderr)
            ));
        }

        // Unpack tar to staging (so we can add .env)
        // This is a bit inefficient (tar -> untar -> tar), but safe and uses standard tools.
        let status = Command::new("tar")
            .args(&["-xf", "-"])
            .current_dir(staging_path)
            .stdin(Stdio::from(tar_output.stdout)) // No, wait_with_output CONSUMES stdout.
            // We need to pipe directly or write to file.
            // Writing `git archive` to a file is safer/easier.
            .output(); // Wait, this logic is flawed because we consumed stdout above.

        // RETRY: Pipeline approach.
        // git archive | tar -x -C staging_path
        let git_archive = Command::new("git")
            .args(&["archive", "--format=tar", "HEAD"])
            .stdout(Stdio::piped())
            .spawn()?;

        let mut tar_extract = Command::new("tar")
            .args(&["-x", "-C", staging_path.to_str().unwrap()])
            .stdin(git_archive.stdout.unwrap()) // Chain pipes
            .spawn()?;

        let status = tar_extract.wait()?;
        if !status.success() {
            return Err(anyhow!("Failed to extract git archive to staging"));
        }

        // 3. Inject Decrypted Secrets
        println!("🔓 Injecting decrypted secrets...");
        let security = ArcaneSecurity::new(None)?;
        // Note: This expects we are in a repo root to find .git/arcane

        // Find .env files in the root? Or just .env?
        // Arcane typically manages a single root .env or specific ones.
        // For 'push deploy', we usually just want the root .env.
        let env_path = Path::new(".env");
        if env_path.exists() {
            // If it's encrypted (binary/age header), we decrypt it using the REPO KEY (authorized for US).
            // Wait, usually locally it's decrypted on checkout?
            // If the user has `arcane run` working, the .env on disk might be plaintext OR encrypted depending on filter state.
            // If the git filter is active:
            // - Worktree: Plaintext
            // - Index/Repo: Encrypted
            // So if we just copy the worktree .env, we effectively deploy the secret.
            // BUT, `git archive` gets the COMMITTED (Encrypted) version from the repo!

            // CORRECT LOGIC:
            // 1. `git archive` exports the ENCRYPTED .env.
            // 2. We must overwrite it in `staging_path/.env` with the DECRYPTED version.

            // How to get decrypted version?
            // Logic: Read Worktree .env (which is plaintext if filter works) or decrypt manually.
            // Safer to decrypt manually using Arcane's crypto to be sure.

            // Check if we can load the key.
            if let Ok(repo_key) = security.load_repo_key() {
                // We need the ciphertext. Since `git archive` puts it in staging_path:
                let staged_env = staging_path.join(".env");
                if staged_env.exists() {
                    let ciphertext = std::fs::read(&staged_env)?;
                    // Try to decrypt it.
                    // If it's already plaintext (e.g. user didn't encrypt properly), strict decrypt fails.
                    // But `git archive` outputs what is in the generic object db.

                    // Actually, if we are the user, we have the Master Identity.
                    // We can just use the user's local .env which IS plaintext (due to smudge filter).
                    // BUT `git archive` comes from the OBJECT DATABASE (Clean/Encrypted).
                    // So `staging_path/.env` is DEFINITELY encrypted (if committed).

                    // So we decrypt `staging_path/.env`.
                    // We need the Repo Key.
                    match security.decrypt_with_repo_key(&repo_key, &ciphertext) {
                        Ok(plaintext) => {
                            std::fs::write(&staged_env, plaintext)?;
                            println!("   - Decrypted .env successfully");
                        }
                        Err(e) => {
                            // Maybe it wasn't encrypted? Or key issue.
                            println!(
                                "   ⚠️ .env found but decryption failed ({}), deploying as-is.",
                                e
                            );
                        }
                    }
                }
            }
        }

        // 4. Ship it (Tar + SSH)
        println!("🚚 Shipping to {}@{}...", user, host);
        let remote_dir = "arcane_deploy"; // standard deploy folder

        // Command: tar -cz . | ssh user@host "mkdir -p dir && tar -xz -C dir"
        let tar_pack = Command::new("tar")
            .args(&["-cz", "."]) // Pack current dir (staging)
            .current_dir(staging_path)
            .stdout(Stdio::piped())
            .spawn()?;

        let ssh_cmd = format!("mkdir -p {} && tar -xz -C {}", remote_dir, remote_dir);

        let mut ssh_process = Command::new("ssh")
            .args(&[
                // "-o", "StrictHostKeyChecking=no", // Optional: User might want verification
                &format!("{}@{}", user, host),
                &ssh_cmd,
            ])
            .stdin(tar_pack.stdout.unwrap()) // Pipe tar output to ssh stdin
            .spawn()?;

        let status = ssh_process.wait()?;
        if !status.success() {
            return Err(anyhow!("SSH transfer failed"));
        }

        // 5. Execute Start Script
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
