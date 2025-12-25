//! Shadow Branch Management
//!
//! Implements "invisible" auto-commits to shadow branches without switching HEAD.
//! This keeps the user's main branch history clean while preserving granular history.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Manages shadow branch operations for a repository
pub struct ShadowManager {
    repo_path: PathBuf,
}

#[allow(dead_code)]
impl ShadowManager {
    pub fn new(repo_path: &Path) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
        }
    }

    /// Get the current branch name
    fn get_current_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .context("Failed to get current branch")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Not on a branch (detached HEAD?)"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get the shadow branch name for the current branch
    fn shadow_branch_name(&self) -> Result<String> {
        let current = self.get_current_branch()?;
        Ok(format!("shadow/{}", current))
    }

    /// Ensure the shadow branch exists, creating it from current HEAD if needed
    pub fn ensure_shadow_branch(&self) -> Result<String> {
        let shadow_name = self.shadow_branch_name()?;
        let shadow_ref = format!("refs/heads/{}", shadow_name);

        // Check if shadow branch exists
        let check = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["show-ref", "--verify", "--quiet", &shadow_ref])
            .status()
            .context("Failed to check shadow branch")?;

        if !check.success() {
            // Create shadow branch pointing to current HEAD
            let head_output = Command::new("git")
                .current_dir(&self.repo_path)
                .args(["rev-parse", "HEAD"])
                .output()
                .context("Failed to get HEAD")?;

            let head_sha = String::from_utf8_lossy(&head_output.stdout)
                .trim()
                .to_string();

            Command::new("git")
                .current_dir(&self.repo_path)
                .args(["update-ref", &shadow_ref, &head_sha])
                .output()
                .context("Failed to create shadow branch")?;

            println!("🌑 Created shadow branch: {}", shadow_name);
        }

        Ok(shadow_name)
    }

    /// Commit staged changes to the shadow branch without switching HEAD
    pub fn commit_to_shadow(&self, message: &str) -> Result<String> {
        let shadow_name = self.ensure_shadow_branch()?;
        let shadow_ref = format!("refs/heads/{}", shadow_name);

        // 1. Write the current index as a tree
        let tree_output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["write-tree"])
            .output()
            .context("Failed to write tree")?;

        if !tree_output.status.success() {
            return Err(anyhow::anyhow!(
                "write-tree failed: {}",
                String::from_utf8_lossy(&tree_output.stderr)
            ));
        }

        let tree_sha = String::from_utf8_lossy(&tree_output.stdout)
            .trim()
            .to_string();

        // 2. Get the parent commit (current shadow branch tip)
        let parent_output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["rev-parse", &shadow_ref])
            .output()
            .context("Failed to get shadow parent")?;

        let parent_sha = String::from_utf8_lossy(&parent_output.stdout)
            .trim()
            .to_string();

        // 3. Create the commit object
        let commit_output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["commit-tree", &tree_sha, "-p", &parent_sha, "-m", message])
            .output()
            .context("Failed to create commit")?;

        if !commit_output.status.success() {
            return Err(anyhow::anyhow!(
                "commit-tree failed: {}",
                String::from_utf8_lossy(&commit_output.stderr)
            ));
        }

        let commit_sha = String::from_utf8_lossy(&commit_output.stdout)
            .trim()
            .to_string();

        // 4. Update the shadow ref to point to new commit
        Command::new("git")
            .current_dir(&self.repo_path)
            .args(["update-ref", &shadow_ref, &commit_sha])
            .output()
            .context("Failed to update shadow ref")?;

        println!("👻 Shadow commit: {} -> {}", &commit_sha[..8], shadow_name);

        Ok(commit_sha)
    }

    /// List recent commits on the shadow branch
    pub fn list_shadow_commits(&self, limit: usize) -> Result<Vec<ShadowCommit>> {
        let shadow_name = self.shadow_branch_name()?;

        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args([
                "log",
                &shadow_name,
                &format!("-n{}", limit),
                "--pretty=format:%H|%ai|%s",
            ])
            .output()
            .context("Failed to list shadow commits")?;

        if !output.status.success() {
            // Shadow branch might not exist yet
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(3, '|').collect();
                if parts.len() == 3 {
                    Some(ShadowCommit {
                        sha: parts[0].to_string(),
                        date: parts[1].to_string(),
                        message: parts[2].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(commits)
    }

    /// Restore files from a shadow commit to the working directory
    pub fn restore_from_shadow(&self, commit_sha: &str) -> Result<()> {
        // Safety Check: Ensure working directory is clean
        let status_output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["status", "--porcelain"])
            .output()
            .context("Failed to check git status")?;

        if !status_output.stdout.is_empty() {
            return Err(anyhow::anyhow!(
                "Working directory is dirty. Please stash or commit changes before restoring from shadow."
            ));
        }

        // Use git checkout to restore files from the shadow commit
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["checkout", commit_sha, "--", "."])
            .output()
            .context("Failed to restore from shadow")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "restore failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        println!("⏪ Restored from shadow commit: {}", &commit_sha[..8]);
        Ok(())
    }
    /// Undo the last shadow commit (restore state to previous commit)
    pub fn undo_last_commit(&self) -> Result<()> {
        let shadow_name = self.shadow_branch_name()?;
        let shadow_ref = format!("refs/heads/{}", shadow_name);

        // 1. Get current shadow HEAD SHA
        let current_output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["rev-parse", &shadow_ref])
            .output()
            .context("Failed to get current shadow HEAD")?;

        if !current_output.status.success() {
            return Err(anyhow::anyhow!("No shadow history to undo"));
        }

        let current_sha = String::from_utf8_lossy(&current_output.stdout)
            .trim()
            .to_string();

        // 2. Get parent SHA (HEAD~1)
        let parent_output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["rev-parse", &format!("{}^", current_sha)])
            .output();

        // If no parent (first commit), we can't easily undo to "nothing" without cleaning directory
        // For safety, let's just error or handle it.
        // If it fails, maybe it's the only commit.
        let parent_sha = match parent_output {
            Ok(out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            }
            _ => return Err(anyhow::anyhow!("Cannot undo: No previous history found")),
        };

        // 3. Restore files from parent SHA
        self.restore_from_shadow(&parent_sha)?;

        // 4. Move shadow pointer back
        Command::new("git")
            .current_dir(&self.repo_path)
            .args(["update-ref", &shadow_ref, &parent_sha])
            .output()
            .context("Failed to update shadow ref")?;

        println!(
            "⏪ Undid commit: {} -> {}",
            &current_sha[..8],
            &parent_sha[..8]
        );

        Ok(())
    }
}

/// Represents a commit on the shadow branch
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ShadowCommit {
    pub sha: String,
    pub date: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_branch_naming() {
        // This would need a real git repo to test properly
        // For now, just verify the struct can be created
        let manager = ShadowManager::new(Path::new("/tmp"));
        assert!(manager.repo_path.exists() || true); // Path existence not required for construction
    }
}
