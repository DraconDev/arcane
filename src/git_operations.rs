use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Clone)]
pub struct GitOperations;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffFile {
    pub path: String,
    pub status: FileStatus,
    pub hunks: Vec<DiffHunk>,
}

#[allow(dead_code)]
impl GitOperations {
    pub fn new() -> Self {
        Self
    }

    // ... existing methods ...

    pub async fn get_current_branch(&self, repo_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .output()
            .await?;

        if !output.status.success() {
            return Ok("DETACHED".to_string());
        }

        let branch = String::from_utf8(output.stdout)?;
        Ok(branch.trim().to_string())
    }

    pub async fn get_diff_entries(&self, repo_path: &Path) -> Result<Vec<DiffFile>> {
        // Use `git status --porcelain` to get all changed files (staged, unstaged, untracked)
        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("status")
            .arg("--porcelain")
            .output()
            .await?;

        let stdout = String::from_utf8(output.stdout)?;
        let mut entries = Vec::new();

        for line in stdout.lines() {
            if line.len() < 4 {
                continue;
            }

            // Porcelain format: XY PATH
            // X = staging status, Y = worktree status
            let x = line.chars().nth(0).unwrap_or(' ');
            let y = line.chars().nth(1).unwrap_or(' ');
            let path_str = &line[3..];

            // Determine effective status
            // If either X or Y is 'A' or '?', it's an add/untracked
            // If either is 'M', it's modified
            // 'D' is deleted
            // 'R' is renamed
            let status = if x == '?' || y == '?' {
                FileStatus::Unknown // Untracked
            } else if x == 'A' || y == 'A' {
                FileStatus::Added
            } else if x == 'D' || y == 'D' {
                FileStatus::Deleted
            } else if x == 'R' || y == 'R' {
                FileStatus::Renamed
            } else {
                FileStatus::Modified
            };

            entries.push(DiffFile {
                path: path_str.to_string(),
                status,
                hunks: Vec::new(),
            });
        }

        Ok(entries)
    }

    pub async fn get_file_diff(&self, repo_path: &Path, file_path: &str) -> Result<String> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("diff")
            .arg("HEAD")
            .arg("--")
            .arg(file_path)
            .output()
            .await?;

        Ok(String::from_utf8(output.stdout)?)
    }

    pub async fn is_git_repo(&self, path: &Path) -> Result<bool> {
        let git_dir = path.join(".git");
        Ok(git_dir.exists())
    }

    pub async fn has_changes(&self, repo_path: &Path) -> Result<bool> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("status")
            .arg("--porcelain")
            .output()
            .await?;

        Ok(!output.stdout.is_empty())
    }

    pub async fn get_diff(&self, repo_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("diff")
            .arg("HEAD")
            .output()
            .await?;

        let text = String::from_utf8(output.stdout)?;
        if text.len() > 5000 {
            Ok(format!("{}\n... (truncated)", &text[..5000]))
        } else {
            Ok(text)
        }
    }

    pub async fn add_paths(&self, repo_path: &Path, paths: &[PathBuf]) -> Result<()> {
        let mut command = Command::new("git");
        command.current_dir(repo_path).arg("add");

        for path in paths {
            command.arg(path);
        }

        command.output().await?;
        Ok(())
    }

    pub async fn commit(&self, repo_path: &Path, message: &str) -> Result<()> {
        Command::new("git")
            .current_dir(repo_path)
            .arg("commit")
            .arg("-m")
            .arg(message)
            .output()
            .await?;

        Ok(())
    }

    /// Get the current HEAD commit SHA
    pub async fn get_head_sha(&self, repo_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get HEAD SHA"));
        }

        let sha = String::from_utf8(output.stdout)?;
        Ok(sha.trim().to_string())
    }
}
