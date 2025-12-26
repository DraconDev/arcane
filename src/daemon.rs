use crate::config::ConfigManager;
use crate::security::ArcaneSecurity;
use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

pub fn start_daemon() -> Result<()> {
    let config_manager = ConfigManager::new()?;
    let roots = config_manager.config.daemon.watch_roots;

    if roots.is_empty() {
        println!("⚠️ No watch roots configured. Run 'arcane daemon config --add <path>'");
        return Ok(());
    }

    println!("🔮 Sovereign Guardian watching directories:");
    for root in &roots {
        println!("   - {:?}", root);
    }

    // Channel for receiving file events
    let (tx, rx) = channel();

    // Create a recommended watcher
    let mut watcher = notify::recommended_watcher(tx)?;

    // Add each root to the watcher
    for root in &roots {
        if root.exists() {
            watcher
                .watch(root, RecursiveMode::Recursive)
                .with_context(|| format!("Failed to watch {:?}", root))?;
        } else {
            println!("⚠️ Watch root does not exist (skipping): {:?}", root);
        }
    }

    log_event("⚡ Daemon is active. Waiting for new repositories...");

    // Save Status to disk so TUI can see it
    let status = crate::DaemonStatus {
        pid: std::process::id(),
        state: "Running".to_string(),
        last_commit: None,
        watching: roots.iter().map(|p| p.display().to_string()).collect(),
        branch: None,
        last_alert: None,
    };
    if let Err(e) = status.save() {
        log_event(&format!("❌ Failed to save daemon status: {}", e));
    }

    // Event loop
    for res in rx {
        match res {
            Ok(event) => handle_event(event),
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}

pub fn log_event(message: &str) {
    if let Some(home) = home::home_dir() {
        let log_path = home.join(".arcane").join("daemon.log");
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
        {
            let _ = writeln!(
                file,
                "[{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                message
            );
        }
    }
    println!("{}", message);
}

fn handle_event(event: Event) {
    match event.kind {
        EventKind::Create(_) => {
            for path in event.paths {
                if path.file_name().and_then(|s| s.to_str()) == Some(".git") {
                    if let Some(parent) = path.parent() {
                        log_event(&format!("✨ Detected new git repo: {:?}", parent));
                        if let Err(e) = auto_init_repo(parent) {
                            log_event(&format!("❌ Failed to auto-init: {:?}", e));
                        } else {
                            log_event(&format!("✅ Auto-Init successful for {:?}", parent));
                        }
                    }
                }
            }
        }
        EventKind::Modify(_) => {
            // Check if auto-commit is enabled
            if let Ok(config_manager) = ConfigManager::new() {
                if !config_manager.config.auto_commit_enabled {
                    return;
                }

                // Debounce/Throttle could go here

                for path in event.paths {
                    // Ignore modifications inside .git folder
                    if path.to_string_lossy().contains(".git") {
                        continue;
                    }

                    // Find repo root
                    let repo_root = find_git_root(&path);
                    if let Some(root) = repo_root {
                        // Spin up a thread to handle commit to avoid blocking watcher
                        let root_clone = root.clone();
                        std::thread::spawn(move || {
                            if let Err(_e) = perform_auto_commit(&root_clone) {
                                // log_event(&format!("❌ Auto-commit failed: {:?}", e));
                                // Silence frequent errors to avoid log spam, or log only criticals
                            }
                        });
                    }
                }
            }
        }
        _ => {}
    }
}

fn find_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = path;
    if current.is_file() {
        if let Some(p) = current.parent() {
            current = p;
        } else {
            return None;
        }
    }

    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            return None;
        }
    }
}

fn perform_auto_commit(repo_path: &Path) -> Result<()> {
    use crate::ai_service::AIService;
    use crate::git_operations::GitOperations;

    let git = GitOperations::new();

    // Since we are in a sync thread, we need a runtime for async calls
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        if !git.has_changes(repo_path).await? {
            return Ok(());
        }

        // Add all
        git.add_paths(repo_path, &[PathBuf::from(".")]).await?;

        // Generate Message
        // Load config for AI
        let config_manager = ConfigManager::new()?;
        let ai_config = config_manager.ai_config();

        let auto_push = config_manager.config.auto_push_enabled;

        // Use AI Service
        let ai = AIService::new(ai_config);
        let diff = git.get_diff(repo_path).await?;

        if diff.trim().is_empty() {
            return Ok(());
        }

        // 1. FAST REGEX SCAN (Local)
        // We scan the diff content to catch secrets *before* sending to AI (privacy + speed)
        // Only scan ADDED lines (starting with '+') to avoid false positives on removed secrets
        // Skip lines from examples/ directories (demo files with fake secrets)

        // Parse diff to find current file being modified
        let mut current_file = String::new();
        let mut added_lines = Vec::new();

        for line in diff.lines() {
            if line.starts_with("+++ ") {
                // Extract file path from diff header line: +++ b/path/to/file
                current_file = line
                    .trim_start_matches("+++ ")
                    .trim_start_matches("b/")
                    .to_string();
            } else if line.starts_with('+') && !line.starts_with("+++") {
                // Skip scanning for:
                // - examples/ directories (demo files with fake secrets)
                // - config/envs/ (managed by Arcane encryption)
                // - .env files (will be encrypted by Arcane)
                let is_arcane_managed = current_file.starts_with("examples/")
                    || current_file.starts_with("config/envs/")
                    || current_file.ends_with(".env")
                    || current_file.contains("/examples/")
                    || current_file.contains("demo");

                if !is_arcane_managed {
                    added_lines.push(line.to_string());
                }
            }
        }

        let added_content = added_lines.join("\n");

        let scanner = crate::security::SecretScanner::new();
        let matches = scanner.scan(&added_content);
        if !matches.is_empty() {
            // Build detailed, actionable alert
            let secret_list: Vec<String> = matches
                .iter()
                .take(3) // Show max 3 to keep readable
                .map(|m| format!("• Line {}: {} - \"{}\"", m.line, m.name, 
                    if m.snippet.len() > 40 { format!("{}...", &m.snippet[..40]) } else { m.snippet.clone() }
                ))
                .collect();
            
            let more_msg = if matches.len() > 3 {
                format!("\n  ...and {} more", matches.len() - 3)
            } else {
                String::new()
            };
            
            // Log detailed alert
            let log_msg = format!(
                "🛑 BLOCKED: Secrets detected in source code!\n  {}{}\n  \n  ⚠️  Move secrets to .env (encrypted by Arcane)\n  ⚠️  Or use test keys (sk_test_* not sk_live_*)",
                secret_list.join("\n  "),
                more_msg
            );
            crate::daemon::log_event(&log_msg);

            // Desktop notification (brief)
            notify_user(
                "🛑 Secret Detected - Commit Blocked",
                &format!("{} secret(s) found in source code. Check TUI for details.", matches.len()),
            );

            // Persist Alert to Status
            if let Some(mut status) = crate::DaemonStatus::load() {
                status.last_alert = Some(format!(
                    "{} - {} secret(s) blocked",
                    chrono::Local::now().format("%H:%M:%S"),
                    matches.len()
                ));
                let _ = status.save();
            }

            return Ok(());
        }

        // 2. AI ANALYSIS (Smart)
        let response = ai
            .generate_commit_message(&diff)
            .await
            .unwrap_or_else(|_| format!("Auto-save: {}", chrono::Local::now().format("%H:%M:%S")));

        if response.trim().is_empty() {
            return Ok(());
        }

        // Check for specific alert protocols
        if response.starts_with("SECURITY_ALERT:") {
            let reason = response.replace("SECURITY_ALERT:", "").trim().to_string();
            let alert_msg = format!(
                "🛑 AI SECURITY ALERT: Blocked commit for {:?}. Reason: {}",
                repo_path.file_name().unwrap_or_default(),
                reason
            );

            crate::daemon::log_event(&alert_msg);
            notify_user(
                "Arcane Security Alert",
                &format!("Blocked commit: {}", reason),
            );

            // Persist Alert to Status
            if let Some(mut status) = crate::DaemonStatus::load() {
                status.last_alert = Some(format!(
                    "{} - {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    reason
                ));
                let _ = status.save();
            }

            return Ok(());
        }

        let commit_msg = if let Some(stripped) = response.strip_prefix("COMMIT_MESSAGE:") {
            stripped.trim().to_string()
        } else {
            response
        };

        if commit_msg.is_empty() {
            return Ok(());
        }

        git.commit(repo_path, &commit_msg).await?;

        // Clear Alert on success
        if let Some(mut status) = crate::DaemonStatus::load() {
            if status.last_alert.is_some() {
                status.last_alert = None;
                let _ = status.save();
            }
        }

        let mut action_msg = format!(
            "🤖 Auto-committed in {:?}: {}",
            repo_path.file_name().unwrap_or_default(),
            commit_msg
        );

        if auto_push {
            let push_result = if config_manager.config.shadow_branches {
                // Shadow Mode: Push to shadow/<branch>
                if let Ok(current_branch) = git.get_current_branch(repo_path).await {
                    let refspec = format!("HEAD:refs/heads/shadow/{}", current_branch);
                    git.push(repo_path, Some(&refspec)).await.map(|_| "Shadow")
                } else {
                    // Fallback to normal if can't get branch? Or error?
                    Err(anyhow::anyhow!(
                        "Could not determine branch for shadow push"
                    ))
                }
            } else {
                // Normal Mode: Push current branch to upstream
                git.push(repo_path, None).await.map(|_| "Upstream")
            };

            match push_result {
                Ok(target) => {
                    action_msg.push_str(&format!(" (Pushed {} 🚀)", target));
                }
                Err(e) => {
                    action_msg.push_str(&format!(" (Push Failed: {})", e));
                }
            }
        }

        log_event(&action_msg);

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

fn auto_init_repo(path: &Path) -> Result<()> {
    // 1. Check/Write .gitattributes
    let attr_file = path.join(".gitattributes");
    let needs_config = if !attr_file.exists() {
        true
    } else {
        let content = std::fs::read_to_string(&attr_file)?;
        !content.contains("filter=git-arcane")
    };

    if needs_config {
        println!("   📝 Injecting .gitattributes...");
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(attr_file)?;
        use std::io::Write;
        writeln!(file, "\n# Arcane Transparent Encryption")?;
        writeln!(file, "*.env filter=git-arcane diff=git-arcane")?;
    }

    // 2. Initialize Keys (Arcane Init)
    println!("   🔐 Initializing Arcane Encryption Keys...");
    let security = ArcaneSecurity::new(Some(path))?;

    match security.init_repo() {
        Ok(_) => println!("   ✅ Repo keys generated."),
        Err(e) => {
            if e.to_string().contains("already initialized") {
                println!("   (Repo already initialized, skipping)");
            } else {
                return Err(e);
            }
        }
    }

    Ok(())
}

pub fn add_watch_root(path: PathBuf) -> Result<()> {
    let mut manager = ConfigManager::new()?;
    println!("✅ Added watch root: {:?}", path);
    manager.add_watch_root(path)?;
    Ok(())
}

fn notify_user(title: &str, body: &str) {
    #[cfg(target_os = "linux")]
    {
        use notify_rust::{Hint, Notification, Urgency};
        use std::process::Command;
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};

        // Debounce: Only send one notification per 10 seconds
        static LAST_NOTIFY: AtomicU64 = AtomicU64::new(0);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last = LAST_NOTIFY.load(Ordering::Relaxed);

        if now - last < 10 {
            return; // Skip - too soon since last notification
        }
        LAST_NOTIFY.store(now, Ordering::Relaxed);

        let result = Notification::new()
            .summary(title)
            .body(body)
            .appname("Arcane")
            .icon("security-high")
            .urgency(Urgency::Critical)
            .hint(Hint::Resident(true)) // Keep in notification center
            .timeout(0) // Persistent until user dismisses
            .action("default", "Open Arcane")
            .show();

        // Spawn a thread to wait for click action (non-blocking)
        if let Ok(handle) = result {
            std::thread::spawn(move || {
                handle.wait_for_action(|action| {
                    if action == "default" {
                        // Use absolute path to ensure we run the correct binary
                        let arcane_path = "/home/dracon/.cargo/bin/arcane";

                        // Try gnome-terminal first (most common on Ubuntu)
                        if Command::new("gnome-terminal")
                            .args(["--", arcane_path])
                            .spawn()
                            .is_err()
                        {
                            // Fallback to x-terminal-emulator
                            let _ = Command::new("x-terminal-emulator")
                                .args(["-e", arcane_path])
                                .spawn();
                        }
                    }
                });
            });
        }
    }
}
