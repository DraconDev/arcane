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
    // We only care about file/folder creation
    if let EventKind::Create(_) = event.kind {
        for path in event.paths {
            // Check if it's a .git directory being created
            if path.file_name().and_then(|s| s.to_str()) == Some(".git") {
                if let Some(parent) = path.parent() {
                    log_event(&format!("✨ Detected new git repo: {:?}", parent));
                    // Trigger Auto-Init
                    if let Err(e) = auto_init_repo(parent) {
                        log_event(&format!("❌ Failed to auto-init: {:?}", e));
                    } else {
                        log_event(&format!("✅ Auto-Init successful for {:?}", parent));
                    }
                }
            }
        }
    }
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
