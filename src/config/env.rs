use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
    pub variables: HashMap<String, String>,
}

use crate::security::{ArcaneSecurity, RepoKey};

impl Environment {
    /// Load an environment by name (e.g., "staging", "production")
    /// Merges base.env with [name].env. Supports encrypted files.
    pub fn load(
        name: &str,
        project_root: &Path,
        security: &ArcaneSecurity,
        repo_key: &RepoKey,
    ) -> Result<Self> {
        let envs_dir = project_root.join("config").join("envs");

        // 1. Load base.env (if exists)
        let mut variables = HashMap::new();
        let base_path = envs_dir.join("base.env");
        if base_path.exists() {
            let base_vars = load_and_decrypt(&base_path, security, repo_key)?;
            variables.extend(base_vars);
        }

        // 2. Load specific env file (e.g. staging.env)
        let env_path = envs_dir.join(format!("{}.env", name));
        if env_path.exists() {
            let env_vars = load_and_decrypt(&env_path, security, repo_key)?;
            variables.extend(env_vars);
        } else if name != "staging" && name != "production" {
            // Check if it exists in root (legacy support for simple .env)
            let legacy_path = project_root.join(format!("{}.env", name));
            if legacy_path.exists() {
                let env_vars = load_and_decrypt(&legacy_path, security, repo_key)?;
                variables.extend(env_vars);
            }
        }

        Ok(Self {
            name: name.to_string(),
            variables,
        })
    }
}

fn load_and_decrypt(
    path: &Path,
    security: &ArcaneSecurity,
    repo_key: &RepoKey,
) -> Result<HashMap<String, String>> {
    let content = fs::read(path).with_context(|| format!("Failed to read env file: {:?}", path))?;

    // Hybrid Mode: Try decrypt, fallback to plaintext
    let decrypted_bytes = match security.decrypt_with_repo_key(repo_key, &content) {
        Ok(d) => d,
        Err(_) => content, // Assume plaintext if decryption fails
    };

    let content_str = String::from_utf8(decrypted_bytes)
        .context(format!("File {:?} is not valid UTF-8 text", path))?;

    let mut map = HashMap::new();
    for line in content_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(map)
}
