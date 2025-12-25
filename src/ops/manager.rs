use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: Option<String>,
}

#[derive(Debug)]
pub struct OpsManager {
    pub servers: Vec<Server>,
}

impl OpsManager {
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
        }
    }
}
