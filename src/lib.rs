pub mod ai_service;
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
