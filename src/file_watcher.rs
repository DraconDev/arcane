use crate::auto_gitignore::AutoGitIgnore;
use anyhow::Result;
use chrono::Local;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify_debouncer_mini::{new_debouncer, notify::*, DebouncedEvent};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::ai_service::AIService;
use crate::git_operations::GitOperations;
use crate::security::ArcaneSecurity;
use crate::shadow::ShadowManager;

use crate::DaemonStatus;

#[allow(dead_code)]
pub struct FileWatcher {
    root_path: PathBuf,
    git_operations: GitOperations,
    ai_service: AIService,
    #[allow(dead_code)]
    security: ArcaneSecurity,
    shadow_manager: ShadowManager,
    shadow_mode: bool,
    change_queue: Arc<Mutex<Vec<PathBuf>>>,
    last_commit_time: Arc<Mutex<chrono::DateTime<Local>>>,
    processing: Arc<Mutex<bool>>,
    gitignore: Gitignore,
    status_tx: Option<tokio::sync::broadcast::Sender<DaemonStatus>>,
}

#[allow(dead_code)]
impl FileWatcher {
    pub fn new(
        root_path: PathBuf,
        git_operations: GitOperations,
        ai_service: AIService,
        security: ArcaneSecurity,
    ) -> Self {
        let shadow_manager = ShadowManager::new(&root_path);

        // Load .gitignore
        let mut builder = GitignoreBuilder::new(&root_path);
        // Look for .gitignore in the root
        let gitignore_path = root_path.join(".gitignore");
        if let Some(err) = builder.add(&gitignore_path) {
            eprintln!("⚠️ Error loading .gitignore: {}", err);
        }
        // Also add common build directories to ignores manually
        let _ = builder.add_line(None, ".git/");
        let _ = builder.add_line(None, "target/");
        let _ = builder.add_line(None, "node_modules/");

        let gitignore = builder.build().unwrap_or_else(|_| Gitignore::empty());

        Self {
            root_path,
            git_operations,
            ai_service,
            security,
            shadow_manager,
            shadow_mode: false, // Default to REAL commits (Direct Mode)
            change_queue: Arc::new(Mutex::new(Vec::new())),
            last_commit_time: Arc::new(Mutex::new(Local::now())),
            processing: Arc::new(Mutex::new(false)),
            gitignore,
            status_tx: None,
        }
    }

    pub fn with_status_channel(mut self, tx: tokio::sync::broadcast::Sender<DaemonStatus>) -> Self {
        self.status_tx = Some(tx);
        self
    }

    pub async fn start_watching(&mut self) -> Result<()> {
        self.update_status("Starting").await?;
        println!("👀 Starting file watcher for: {}", self.root_path.display());

        // Auto-ensure safe gitignore defaults
        if let Err(e) = AutoGitIgnore::new(&self.root_path).ensure_defaults() {
            eprintln!("⚠️ Failed to update .gitignore: {}", e);
        }

        // Verify it's a git repository
        if !self.git_operations.is_git_repo(&self.root_path).await? {
            return Err(anyhow::anyhow!(
                "Not a git repository: {}",
                self.root_path.display()
            ));
        }

        // Setup file watcher with debouncing
        let (tx, rx) = channel();
        let mut debouncer = new_debouncer(Duration::from_secs(5), tx)?;

        // Add paths selectively, skipping heavy directories
        // This avoids hitting OS inotify limits
        let skip_dirs = [
            "target",
            "node_modules",
            ".git",
            "dist",
            "build",
            ".next",
            "__pycache__",
        ];

        // Watch root directory (non-recursive)
        debouncer
            .watcher()
            .watch(&self.root_path, RecursiveMode::NonRecursive)?;

        // Recursively add subdirectories, skipping excluded ones
        fn add_dir_recursive(
            watcher: &mut dyn notify::Watcher,
            path: &PathBuf,
            skip_dirs: &[&str],
            depth: usize,
        ) -> Result<usize> {
            if depth > 10 {
                return Ok(0);
            } // Max depth to prevent infinite loops

            let mut count = 0;
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        let name = entry_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");

                        // Skip excluded directories
                        if skip_dirs.iter().any(|s| *s == name) {
                            continue;
                        }

                        // Skip hidden directories (except .arcane)
                        if name.starts_with('.') && name != ".arcane" {
                            continue;
                        }

                        if watcher
                            .watch(&entry_path, RecursiveMode::NonRecursive)
                            .is_ok()
                        {
                            count += 1;
                            count += add_dir_recursive(watcher, &entry_path, skip_dirs, depth + 1)?;
                        }
                    }
                }
            }
            Ok(count)
        }

        let dir_count = add_dir_recursive(debouncer.watcher(), &self.root_path, &skip_dirs, 0)?;
        println!(
            "📁 Watching {} directories in: {}",
            dir_count + 1,
            self.root_path.display()
        );

        self.update_status("Idle").await?;

        // Create an async channel to bridge the blocking sync channel
        let (async_tx, mut async_rx) = tokio::sync::mpsc::channel(100);

        // Spawn a blocking task to pump events from sync to async
        tokio::task::spawn_blocking(move || {
            while let Ok(events) = rx.recv() {
                if async_tx.blocking_send(events).is_err() {
                    break;
                }
            }
        });

        // Process events asynchronously
        while let Some(events) = async_rx.recv().await {
            match events {
                Ok(events) => {
                    if let Err(e) = self.handle_events(events).await {
                        eprintln!("⚠️ processing error: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("🔴 Watch error: {:?}", e);
                }
            }
        }

        Ok(())
    }

    async fn handle_events(&self, events: Vec<DebouncedEvent>) -> Result<()> {
        let mut queue = self.change_queue.lock().await;

        for event in events {
            let path = event.path;
            // println!("🔍 Debug: Event for {:?}", path); // Uncomment for raw event spam

            // Check against .gitignore
            if self.should_ignore(&path) {
                // println!("🚫 Ignored: {:?}", path);
                continue;
            }

            // Check if this is a file change we care about
            if let Some(relative_path) = self.get_relative_path(&path) {
                println!("📝 Change detected: {}", relative_path.display());
                queue.push(relative_path);
            }
        }

        // Process the queue if we have changes and not already processing
        if !queue.is_empty() && !*self.processing.lock().await {
            let changes = queue.clone();
            queue.clear();

            drop(queue); // Release the lock

            self.process_changes(changes).await?;
        }

        Ok(())
    }

    // Helper to auto-encrypt any unencrypted .env files found in changes
    async fn auto_encrypt_env_files(&self, changes: &[PathBuf]) -> Result<()> {
        let simpler_ignore = AutoGitIgnore::new(&self.root_path);

        for path in changes {
            if simpler_ignore.is_unencrypted_env(path) {
                let full_path = self.root_path.join(path);
                println!("🔒 Auto-encrypting detected env file: {}", path.display());

                // Read plaintext
                let plaintext = tokio::fs::read(&full_path).await?;

                // Encrypt with repo key
                match self.security.load_repo_key() {
                    Ok(repo_key) => {
                        match self.security.encrypt_with_repo_key(&repo_key, &plaintext) {
                            Ok(ciphertext) => {
                                // Overwrite with encrypted content + Age header
                                let mut final_content = b"age-encryption.org/v1\n".to_vec();
                                final_content.extend_from_slice(&ciphertext);
                                tokio::fs::write(&full_path, final_content).await?;
                                println!("✅ Encrypted {}", path.display());
                            }
                            Err(e) => {
                                eprintln!("❌ Encryption failed for {}: {}", path.display(), e)
                            }
                        }
                    }
                    Err(_) => {
                        eprintln!("⚠️ Cannot auto-encrypt {}: Master identity not loaded or repo key missing.", path.display());
                    }
                }
            }
        }
        Ok(())
    }

    async fn process_changes(&self, changes: Vec<PathBuf>) -> Result<()> {
        *self.processing.lock().await = true;
        self.update_status("Processing Changes").await?;

        // Auto-encrypt any env files *before* checking for git changes
        if let Err(e) = self.auto_encrypt_env_files(&changes).await {
            eprintln!("⚠️ Auto-encrypt error: {}", e);
        }

        println!("🔄 Processing {} changes...", changes.len());

        // Check if we should commit based on timing
        let now = Local::now();
        let last_commit = *self.last_commit_time.lock().await;

        if now - last_commit < chrono::Duration::seconds(2) {
            println!("⏳ Too soon since last commit, skipping");
            *self.processing.lock().await = false;
            self.update_status("Idle").await?;
            return Ok(());
        }

        // Check if there are actual git changes
        if !self.git_operations.has_changes(&self.root_path).await? {
            println!("📋 No actual git changes detected");
            *self.processing.lock().await = false;
            return Ok(());
        }

        // Generate commit message using AI
        let diff = self.git_operations.get_diff(&self.root_path).await?;

        // --- Smart Versioning Logic (Auto-Commit Hook) ---
        let mut final_diff = diff.clone();

        // Re-load config to check if versioning is enabled
        let config = crate::config::ArcaneConfig::load().unwrap_or_default();
        if config.version_bumping {
            println!("🧠 Analyzing SemVer for auto-commit...");

            match self.ai_service.analyze_semver(&diff).await {
                Ok(bump) => {
                    use crate::version_manager::{SemVerBump, VersionManager};
                    println!("📊 AI suggests: {:?}", bump);

                    match bump {
                        SemVerBump::Patch => {
                            // Apply Patch Bump Automatically
                            if let Some(ver_file) =
                                VersionManager::detect_version_file(&self.root_path)
                            {
                                println!("📦 Applying Patch bump to {}", ver_file.display());
                                match VersionManager::apply_bump(&ver_file, SemVerBump::Patch) {
                                    Ok((old, new)) => {
                                        println!("✅ Bumped {} -> {}", old, new);
                                        // Stage the version file
                                        if let Err(e) = self
                                            .git_operations
                                            .add_paths(&self.root_path, &vec![ver_file])
                                            .await
                                        {
                                            eprintln!("⚠️ Failed to stage version file: {}", e);
                                        }
                                        // Refresh Diff to include the version bump in the AI commit context
                                        if let Ok(new_diff) =
                                            self.git_operations.get_diff(&self.root_path).await
                                        {
                                            final_diff = new_diff;
                                        }
                                    }
                                    Err(e) => eprintln!("⚠️ Version bump failed: {}", e),
                                }
                            }
                        }
                        SemVerBump::Minor | SemVerBump::Major => {
                            // Log but skip auto-bump for larger changes
                            println!("⚠️  Skipping auto-bump for {:?} change. Manual review recommended.", bump);
                        }
                        SemVerBump::None => {
                            println!("✨ No version bump required.");
                        }
                    }
                }
                Err(e) => eprintln!("⚠️ SemVer analysis failed: {}", e),
            }
        }
        // -------------------------------------------------

        let commit_message = self.ai_service.generate_commit_message(&final_diff).await?;

        // Perform the commit (shadow or regular)
        self.git_operations
            .add_paths(&self.root_path, &changes)
            .await?;

        if self.shadow_mode {
            // Shadow mode: commit to shadow branch
            match self.shadow_manager.commit_to_shadow(&commit_message) {
                Ok(sha) => println!("👻 Shadow commit: {}", &sha[..8]),
                Err(e) => eprintln!("⚠️ Shadow commit failed, falling back to regular: {}", e),
            }
        } else {
            // Regular mode: commit to HEAD
            self.git_operations
                .commit(&self.root_path, &commit_message)
                .await?;
            println!("✅ Committed: {}", commit_message);
        }

        // Update last commit time
        *self.last_commit_time.lock().await = Local::now();

        *self.processing.lock().await = false;
        self.update_status("Idle").await?;
        Ok(())
    }

    async fn update_status(&self, state: &str) -> Result<()> {
        let branch = self
            .git_operations
            .get_current_branch(&self.root_path)
            .await
            .ok();

        let status = DaemonStatus {
            pid: std::process::id(),
            state: state.to_string(),
            last_commit: Some(self.last_commit_time.lock().await.to_rfc3339()),
            watching: vec![self.root_path.to_string_lossy().to_string()],
            branch,
        };

        // Broadcast to in-memory listeners
        if let Some(tx) = &self.status_tx {
            let _ = tx.send(status.clone());
        }

        // Save to disk (legacy/persistence)
        // We ignore errors here to not crash the daemon on status write failure
        let _ = status.save();
        Ok(())
    }

    fn should_ignore(&self, path: &Path) -> bool {
        // Use the ignore crate's Gitignore matcher
        // matched returns Match::Ignore, Match::Whitelist, or Match::None
        let is_dir = path.is_dir();
        self.gitignore.matched(path, is_dir).is_ignore()
    }

    fn get_relative_path(&self, path: &Path) -> Option<PathBuf> {
        path.strip_prefix(&self.root_path)
            .ok()
            .map(|p| p.to_path_buf())
    }
}
