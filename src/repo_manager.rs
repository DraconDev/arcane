use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub path: PathBuf,
    pub name: String,
    pub last_checked: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoManager {
    pub watched_repos: Vec<RepoConfig>,
    #[serde(skip)]
    config_path: PathBuf,
}

impl RepoManager {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let config_dir = home.join(".arcane");

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        let config_path = config_dir.join("repos.json");

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            match serde_json::from_str::<RepoManager>(&content) {
                Ok(mut manager) => {
                    manager.config_path = config_path;
                    Ok(manager)
                }
                Err(e) => {
                    eprintln!(
                        "⚠️ Failed to parse repos.json: {}. Starting with empty list.",
                        e
                    );
                    Ok(Self {
                        watched_repos: Vec::new(),
                        config_path,
                    })
                }
            }
        } else {
            Ok(Self {
                watched_repos: Vec::new(),
                config_path,
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    pub fn add_repo(&mut self, path: String) -> Result<RepoConfig> {
        let path_buf = PathBuf::from(&path);
        let canonical = fs::canonicalize(&path_buf).context("Invalid path")?;

        // simple name derivation
        let name = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let config = RepoConfig {
            path: canonical,
            name,
            last_checked: None, // Could set to now
        };

        // Check for duplicates
        if !self.watched_repos.iter().any(|r| r.path == config.path) {
            self.watched_repos.push(config.clone());
            self.save()?;
        }

        Ok(config)
    }

    pub fn remove_repo(&mut self, path: String) -> Result<()> {
        let path_buf = PathBuf::from(&path);
        // Try to match by path string or canonical path
        // For simplicity, just string matching the input or the stored path
        self.watched_repos
            .retain(|r| r.path.to_string_lossy() != path && r.path != path_buf);
        self.save()?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoStatus {
    pub path: PathBuf,
    pub name: String,
    pub last_checked: Option<String>,
    pub is_secured: bool,
    pub is_pinned: bool,
    pub root_path: Option<PathBuf>,
}

impl RepoManager {
    pub fn list_repos(&self) -> Vec<RepoStatus> {
        let mut results = std::collections::HashMap::new();

        // 1. Add pinned (explicitly watched) repos
        for config in &self.watched_repos {
            let is_secured = config
                .path
                .join(".git")
                .join("arcane")
                .join("keys")
                .exists();
            results.insert(
                config.path.clone(),
                RepoStatus {
                    path: config.path.clone(),
                    name: config.name.clone(),
                    last_checked: config.last_checked.clone(),
                    is_secured,
                    is_pinned: true,
                    root_path: None,
                },
            );
        }

        // 2. Scan watch roots
        if let Ok(config) = crate::config::ArcaneConfig::load() {
            for root in config.daemon.watch_roots {
                if let Ok(entries) = fs::read_dir(&root) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && path.join(".git").exists() {
                            // If not already in results (pinned), add as discovered
                            if !results.contains_key(&path) {
                                let name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();

                                let is_secured =
                                    path.join(".git").join("arcane").join("keys").exists();

                                results.insert(
                                    path.clone(),
                                    RepoStatus {
                                        path: path.clone(),
                                        name: name.clone(),
                                        last_checked: None,
                                        is_secured,
                                        is_pinned: false,
                                        root_path: Some(root.clone()),
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        let mut final_list: Vec<RepoStatus> = results.into_values().collect();
        final_list.sort_by(|a, b| a.name.cmp(&b.name));
        final_list
    }

    pub fn list_watch_roots(&self) -> Vec<PathBuf> {
        match crate::config::ArcaneConfig::load() {
            Ok(config) => config.daemon.watch_roots,
            Err(_) => Vec::new(),
        }
    }
}
