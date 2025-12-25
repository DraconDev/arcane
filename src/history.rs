use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

#[allow(dead_code)]
pub struct HistoryManager;

#[allow(dead_code)]
impl HistoryManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_file_history(repo_path: &Path, file_path: &Path) -> Result<Vec<CommitInfo>> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("log")
            .arg("--follow")
            .arg("--pretty=format:%H|%an|%ad|%s")
            .arg("--date=iso")
            .arg("--")
            .arg(file_path)
            .output()
            .await?;

        let output_str = String::from_utf8(output.stdout)?;
        let mut history = Vec::new();

        for line in output_str.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                history.push(CommitInfo {
                    hash: parts[0].to_string(),
                    author: parts[1].to_string(),
                    date: parts[2].to_string(),
                    message: parts[3].to_string(),
                });
            }
        }

        Ok(history)
    }
    pub async fn search_commits(repo_path: &Path, query: &str) -> Result<Vec<CommitInfo>> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("log")
            .arg(format!("--grep={}", query))
            .arg("--regexp-ignore-case")
            .arg("--pretty=format:%H|%an|%ad|%s")
            .arg("--date=iso")
            .output()
            .await?;

        let output_str = String::from_utf8(output.stdout)?;
        let mut history = Vec::new();

        for line in output_str.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                history.push(CommitInfo {
                    hash: parts[0].to_string(),
                    author: parts[1].to_string(),
                    date: parts[2].to_string(),
                    message: parts[3].to_string(),
                });
            }
        }

        Ok(history)
    }

    pub async fn get_repo_history(repo_path: &Path) -> Result<Vec<CommitInfo>> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("log")
            .arg("--pretty=format:%H|%an|%ad|%s")
            .arg("--date=iso")
            .arg("-n")
            .arg("50") // Limit defaults
            .output()
            .await?;

        let output_str = String::from_utf8(output.stdout)?;
        let mut history = Vec::new();

        for line in output_str.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                history.push(CommitInfo {
                    hash: parts[0].to_string(),
                    author: parts[1].to_string(),
                    date: parts[2].to_string(),
                    message: parts[3].to_string(),
                });
            }
        }

        Ok(history)
    }

    pub async fn get_commit_graph(repo_path: &Path) -> Result<Vec<GraphCommitInfo>> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("log")
            .arg("--all")
            .arg("--pretty=format:%H|%p|%an|%ad|%D|%s")
            .arg("--date=iso-strict")
            .arg("-n")
            .arg("100")
            .output()
            .await?;

        let output_str = String::from_utf8(output.stdout)?;
        let mut graph = Vec::new();

        for line in output_str.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            // Expected: Hash | Parents (space sep) | Author | Date | Refs | Message
            if parts.len() >= 6 {
                let parents: Vec<String> =
                    parts[1].split_whitespace().map(|s| s.to_string()).collect();

                graph.push(GraphCommitInfo {
                    hash: parts[0].to_string(),
                    parents,
                    author: parts[2].to_string(),
                    date: parts[3].to_string(),
                    refs: parts[4].to_string(),
                    message: parts[5].to_string(),
                });
            }
        }

        Ok(graph)
    }
}

#[derive(Debug, Serialize)]
pub struct GraphCommitInfo {
    pub hash: String,
    pub parents: Vec<String>,
    pub author: String,
    pub date: String,
    pub refs: String,
    pub message: String,
}
