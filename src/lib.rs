pub mod ai_service;
pub mod ai_service;
pub mod auto_gitattributes;
pub mod auto_gitignore;
pub mod config;
pub mod daemon;
pub mod doctor;
pub mod file_watcher;
pub mod git_operations;
pub mod history;
pub mod repo_manager;
pub mod security;
pub mod shadow;
pub mod timeline;
pub mod version_manager;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DaemonStatus {
    pub pid: u32,
    pub state: String,
    pub last_commit: Option<String>,
    pub watching: Vec<String>,
    pub branch: Option<String>,
}

impl DaemonStatus {
    pub fn load() -> Option<Self> {
        let home = home::home_dir()?;
        let content = fs::read_to_string(home.join(".arcane").join("daemon.json")).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let home = home::home_dir().expect("Could not find home directory");
        let status_dir = home.join(".arcane");
        fs::create_dir_all(&status_dir)?;

        let json = serde_json::to_string_pretty(self)?;
        let mut file = fs::File::create(status_dir.join("daemon.json"))?;
        use std::io::Write;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
}

/// A single commit entry in the log
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommitEntry {
    pub timestamp: String,
    pub sha: String,
    pub message: String,
    pub repo: String,
    pub branch: String,
    pub shadow: bool,
    pub files_changed: usize,
}

/// Persistent log of all AI-generated commits
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CommitLog {
    pub entries: Vec<CommitEntry>,
}

impl CommitLog {
    /// Load commit log from ~/.arcane/commit_log.json
    pub fn load() -> Self {
        let home = match home::home_dir() {
            Some(h) => h,
            None => return Self::default(),
        };
        let path = home.join(".arcane").join("commit_log.json");
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save commit log to ~/.arcane/commit_log.json
    pub fn save(&self) -> anyhow::Result<()> {
        let home = home::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir"))?;
        let status_dir = home.join(".arcane");
        fs::create_dir_all(&status_dir)?;

        // Keep only last 1000 entries to prevent unbounded growth
        let trimmed = if self.entries.len() > 1000 {
            CommitLog {
                entries: self.entries[self.entries.len() - 1000..].to_vec(),
            }
        } else {
            self.clone()
        };

        let json = serde_json::to_string_pretty(&trimmed)?;
        let mut file = fs::File::create(status_dir.join("commit_log.json"))?;
        use std::io::Write;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Add a new commit entry and save
    pub fn log_commit(&mut self, entry: CommitEntry) -> anyhow::Result<()> {
        self.entries.push(entry);
        self.save()
    }

    /// Get recent commits (last N)
    pub fn recent(&self, n: usize) -> Vec<&CommitEntry> {
        self.entries.iter().rev().take(n).collect()
    }
}
