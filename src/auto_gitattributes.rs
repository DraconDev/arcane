//! Auto GitAttributes Module
//!
//! Automatically maintains .gitattributes with safe defaults and Managed Blocks.
//! Implements "Smart Enforce" (Last Match Wins).

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct AutoGitAttributes {
    repo_root: PathBuf,
}

impl AutoGitAttributes {
    pub fn new(repo_root: &Path) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
        }
    }

    fn gitattributes_path(&self) -> PathBuf {
        self.repo_root.join(".gitattributes")
    }

    /// Enforce patterns in .gitattributes using a Managed Block
    pub fn ensure_managed_block(&self, patterns: &[String]) -> Result<()> {
        let path = self.gitattributes_path();
        let current_content = if path.exists() {
            fs::read_to_string(&path)?
        } else {
            String::new()
        };

        const BLOCK_START: &str = "# --- BEGIN ARCANE MANAGED BLOCK ---";
        const BLOCK_END: &str = "# --- END ARCANE MANAGED BLOCK ---";

        let mut lines: Vec<String> = current_content.lines().map(|s| s.to_string()).collect();

        // 1. Remove existing block if present
        let mut in_block = false;
        lines.retain(|line| {
            if line.trim() == BLOCK_START {
                in_block = true;
                return false;
            }
            if line.trim() == BLOCK_END {
                in_block = false;
                return false;
            }
            !in_block
        });

        // Trim trailing newlines
        while let Some(last) = lines.last() {
            if last.trim().is_empty() {
                lines.pop();
            } else {
                break;
            }
        }

        // 2. Append new block
        lines.push("".to_string());
        lines.push(BLOCK_START.to_string());
        lines.push("# Content inside this block is managed by Arcane.".to_string());
        lines.push(
            "# It is appended to the bottom to override previous rules (Last Match Wins)."
                .to_string(),
        );

        for pattern in patterns {
            lines.push(pattern.clone());
        }
        lines.push(BLOCK_END.to_string());

        // 3. Write back
        let new_content = lines.join("\n");
        let final_content = format!("{}\n", new_content.trim());

        fs::write(&path, final_content)?;
        Ok(())
    }
}
