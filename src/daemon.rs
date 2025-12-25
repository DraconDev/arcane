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
                            if let Err(e) = perform_auto_commit(&root_clone) {
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
    use crate::ai_service::{AIConfig, AIService};
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

        let message = if diff.trim().is_empty() {
            format!("Auto-save: {}", chrono::Local::now().format("%H:%M:%S"))
        } else {
            ai.generate_commit_message(&diff).await.unwrap_or_else(|_| {
                format!("Auto-save: {}", chrono::Local::now().format("%H:%M:%S"))
            })
        };

        git.commit(repo_path, &message).await?;

        let mut action_msg = format!(
            "🤖 Auto-committed in {:?}: {}",
            repo_path.file_name().unwrap_or_default(),
            message
        );

        if auto_push {
            match git.push(repo_path).await {
                Ok(_) => {
                    action_msg.push_str(" (Pushed 🚀)");
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
