use crate::ops::config::ServerConfig;
use crate::ops::shell::Shell;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub image: String,
    pub name: String,
    pub status: String,
    pub ports: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerStats {
    pub id: String,
    pub name: String,
    pub cpu: String,
    pub mem: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerStatus {
    pub name: String,
    pub is_online: bool,
    pub docker_version: Option<String>,
    pub load_avg: Option<String>,
    pub last_updated: String,
}

pub struct Monitor;

impl Monitor {
    pub fn list_containers(server: &ServerConfig) -> Result<Vec<ContainerInfo>> {
        // format: {{.ID}}|{{.Image}}|{{.Names}}|{{.Status}}|{{.Ports}}
        let cmd = "docker ps --format '{{.ID}}|{{.Image}}|{{.Names}}|{{.Status}}|{{.Ports}}'";
        let output = Shell::exec_remote(server, cmd, false)?;

        let mut containers = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 5 {
                containers.push(ContainerInfo {
                    id: parts[0].to_string(),
                    image: parts[1].to_string(),
                    name: parts[2].to_string(),
                    status: parts[3].to_string(),
                    ports: parts[4].to_string(),
                });
            }
        }
        Ok(containers)
    }

    pub fn get_stats(server: &ServerConfig) -> Result<Vec<ContainerStats>> {
        // docker stats --no-stream --format "{{.ID}}|{{.Name}}|{{.CPUPerc}}|{{.MemUsage}}"
        let cmd =
            "docker stats --no-stream --format '{{.ID}}|{{.Name}}|{{.CPUPerc}}|{{.MemUsage}}'";
        let output = Shell::exec_remote(server, cmd, false)?;

        let mut stats = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                stats.push(ContainerStats {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    cpu: parts[2].to_string(),
                    mem: parts[3].to_string(),
                });
            }
        }
        Ok(stats)
    }
}
