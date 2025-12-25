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

        let output = command.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to add paths: {}", stderr));
        }
        Ok(())
    }

    pub async fn commit(&self, repo_path: &Path, message: &str) -> Result<()> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("commit")
            .arg("-m")
            .arg(message)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "nothing to commit" errors, but report others
            if !stderr.contains("nothing to commit") && !stderr.contains("clean") {
                return Err(anyhow::anyhow!("Failed to commit: {}", stderr));
            }
        }

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
    pub async fn push(&self, repo_path: &Path, refspec: Option<&str>) -> Result<()> {
        let mut command = Command::new("git");
        command.current_dir(repo_path).arg("push");

        if let Some(r) = refspec {
            command.arg("origin").arg(r);
        }

        let output = command.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("Everything up-to-date") {
                return Err(anyhow::anyhow!("Failed to push: {}", stderr));
            }
        }
        Ok(())
    }
    pub async fn get_unpushed_commits(&self, repo_path: &Path) -> Result<Vec<CommitInfo>> {
        // Try @{u} (upstream) first
        let has_upstream = self.has_upstream(repo_path).await;
        let range = if has_upstream {
            "@{u}..HEAD"
        } else {
            // If no upstream, we might be on a local branch.
            // Try "master..HEAD" or "main..HEAD"? Or just return all?
            // Safer: assume everything is unpushed if no upstream?
            // Or maybe we just return an error asking to push first?
            // Let's assume generic "HEAD" for now (all history) if no upstream, but that's too much.
            // Let's try to find the "fork point" from main/master.
            "origin/master..HEAD"
        };

        let output = Command::new("git")
            .current_dir(repo_path)
            .args(&["log", range, "--pretty=format:%H|%an|%ad|%s"])
            .output()
            .await;

        let stdout = match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
            _ => {
                // Formatting might fail if range is invalid.
                // Fallback: just 50 recent commits?
                let output = Command::new("git")
                    .current_dir(repo_path)
                    .args(&["log", "-n", "20", "--pretty=format:%H|%an|%ad|%s"])
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
        };

        let mut commits = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                commits.push(CommitInfo {
                    hash: parts[0].to_string(),
                    author: parts[1].to_string(),
                    date: parts[2].to_string(),
                    message: parts[3..].join("|"),
                });
            }
        }
        Ok(commits)
    }

    pub async fn create_backup_branch(&self, repo_path: &Path, prefix: &str) -> Result<String> {
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let branch_name = format!("{}-backup-{}", prefix, timestamp);

        let output = Command::new("git")
            .current_dir(repo_path)
            .args(&["branch", &branch_name])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to create backup branch"));
        }
        Ok(branch_name)
    }

    async fn has_upstream(&self, repo_path: &Path) -> bool {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(&["rev-parse", "--abbrev-ref", "@{u}"])
            .output()
            .await;
        matches!(output, Ok(out) if out.status.success())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}
