use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
    pub variables: HashMap<String, String>,
}

impl Environment {
    /// Load an environment by name (e.g., "staging", "production")
    /// Merges base.env with [name].env
    pub fn load(name: &str, project_root: &Path) -> Result<Self> {
        let envs_dir = project_root.join("config").join("envs");

        // 1. Load base.env (if exists)
        let mut variables = HashMap::new();
        let base_path = envs_dir.join("base.env");
        if base_path.exists() {
            let base_vars = load_env_file(&base_path)?;
            variables.extend(base_vars);
        }

        // 2. Load specific env file (e.g., staging.env)
        // Note: In a real implementation, this file might be encrypted (.age)
        // For now, we assume plaintext .env for the MVP structure logic,
        // encryption hooks will be added in step 2.
        let env_path = envs_dir.join(format!("{}.env", name));
        if env_path.exists() {
            let env_vars = load_env_file(&env_path)?;
            variables.extend(env_vars); // Overwrite base vars
        } else if name != "staging" && name != "production" {
            // If user asks for a specific env that doesn't exist, warn or error?
            // For now, we allow it (might be purely base config)
        }

        Ok(Self {
            name: name.to_string(),
            variables,
        })
    }
}

fn load_env_file(path: &Path) -> Result<HashMap<String, String>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read env file: {:?}", path))?;

    let mut map = HashMap::new();
    for line in content.lines() {
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
