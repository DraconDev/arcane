use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub name: String,
    pub host: String,
    pub user: String,
    #[serde(default)]
    pub port: u16,
    pub key_path: Option<String>,
    pub env: Option<String>,
    #[serde(default = "default_docker_socket")]
    pub docker_socket: String,
}

impl ServerConfig {
    pub fn ssh_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        if self.port > 0 {
            args.push("-p".to_string());
            args.push(self.port.to_string());
        }
        if let Some(key) = &self.key_path {
            args.push("-i".to_string());
            args.push(key.clone());
        }
        // Strict host checking off for automation stability
        args.push("-o".to_string());
        args.push("StrictHostKeyChecking=no".to_string());
        args.push("-o".to_string());
        args.push("UserKnownHostsFile=/dev/null".to_string());
        args
    }
}

fn default_docker_socket() -> String {
    "/var/run/docker.sock".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ServerGroup {
    pub name: String,
    pub servers: Vec<String>, // List of server names
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct OpsConfig {
    #[serde(default)]
    pub servers: Vec<ServerConfig>,
    #[serde(default)]
    pub groups: Vec<ServerGroup>,
}

impl OpsConfig {
    pub fn load() -> Self {
        let config_path = dirs::home_dir()
            .map(|h| h.join(".arcane").join("servers.toml"))
            .unwrap_or_else(|| PathBuf::from("servers.toml"));

        if !config_path.exists() {
            return Self::default();
        }

        let content = fs::read_to_string(config_path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Home dir not found"))?
            .join(".arcane")
            .join("servers.toml");

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    pub fn find_server(&self, name: &str) -> Option<&ServerConfig> {
        self.servers.iter().find(|s| s.name == name)
    }
}
