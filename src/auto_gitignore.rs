//! Auto GitIgnore Module
//!
//! Automatically maintains .gitignore with safe defaults
//! to prevent accidentally committing sensitive or build files.

use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Common patterns that should always be gitignored
/// Note: .env files are NOT included because Arcane encrypts them
pub const ALWAYS_IGNORE: &[&str] = &[
    // IDE & Editor
    ".vscode",
    ".idea/**",
    ".vs/**",
    "*.swp",
    "*.swo",
    "*~",
    // OS
    ".DS_Store",
    "Thumbs.db",
    // Node.js
    "node_modules/",
    "node_modules/**",
    ".npm/",
    "npm-debug.log*",
    "yarn-debug.log*",
    "yarn-error.log*",
    // Rust
    "target/",
    // Build outputs
    "dist/",
    "dist/**",
    "build/",
    "build/**",
    ".next/",
    ".nuxt/",
    ".wxt",
    // Temp & Cache
    "*.tmp",
    "*.temp",
    "*.log",
    "*.cache",
    // Binaries
    "*.dll",
    "*.exe",
    // Python
    "__pycache__/",
    "*.pyc",
    ".venv/",
    "venv/",
    // Reference/misc
    "reference/**",
    "*.kilocode",
    // Keys (but not .env - Arcane encrypts those)
    "*.pem",
    "*.key",
    "*.p12",
    // Arcane internal
    ".arcane/shadow/",
    ".arcane/keys/",
];

/// Patterns that indicate sensitive files
pub const SENSITIVE_PATTERNS: &[&str] = &[
    "password",
    "secret",
    "api_key",
    "apikey",
    "api-key",
    "private_key",
    "privatekey",
    "credentials",
    "token",
    "auth",
];

pub struct AutoGitIgnore {
    repo_root: PathBuf,
}

impl AutoGitIgnore {
    pub fn new(repo_root: &Path) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
        }
    }

    /// Get the .gitignore path
    fn gitignore_path(&self) -> PathBuf {
        self.repo_root.join(".gitignore")
    }

    /// Read current .gitignore entries
    pub fn read_gitignore(&self) -> HashSet<String> {
        let path = self.gitignore_path();
        if let Ok(content) = fs::read_to_string(&path) {
            content
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                .map(|l| l.trim().to_string())
                .collect()
        } else {
            HashSet::new()
        }
    }

    /// Check if a pattern is already in .gitignore
    pub fn is_ignored(&self, pattern: &str) -> bool {
        self.read_gitignore().contains(pattern)
    }

    /// Add patterns to .gitignore using a Managed Block (Smart Enforce)
    /// This ensures our rules are always at the bottom (Last Match Wins)
    /// without duplicating them or deleting user rules.
    pub fn ensure_managed_block(&self, patterns: &[&str]) -> Result<()> {
        let path = self.gitignore_path();
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
                return false; // Remove the end marker too
            }
            !in_block
        });

        // Trim trailing newlines to keep it clean
        while let Some(last) = lines.last() {
            if last.trim().is_empty() {
                lines.pop();
            } else {
                break;
            }
        }

        // 2. Append new block
        lines.push("".to_string()); // Spacer
        lines.push(BLOCK_START.to_string());
        lines.push("# DANGER: Content inside this block is managed by Arcane.".to_string());
        lines.push(
            "# Manual changes may be overwritten. Add custom rules ABOVE this block.".to_string(),
        );
        for pattern in patterns {
            lines.push(pattern.to_string());
        }
        lines.push(BLOCK_END.to_string());

        // 3. Write back
        let new_content = lines.join("\n");
        // Ensure single trailing newline
        let final_content = format!("{}\n", new_content.trim());

        fs::write(&path, final_content)?;
        Ok(())
    }

    /// Add patterns to .gitignore (Legacy - use ensure_managed_block)
    pub fn add_patterns(&self, patterns: &[&str]) -> Result<Vec<String>> {
        self.ensure_managed_block(patterns)?;
        Ok(Vec::new()) // Return empty as we fully managed it
    }

    /// Ensure patterns are NOT in .gitignore (i.e. force tracking)
    pub fn ensure_tracked(&self, patterns: &[&str]) -> Result<Vec<String>> {
        let path = self.gitignore_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut removed = Vec::new();
        let mut changed = false;

        for pattern in patterns {
            if let Some(pos) = lines.iter().position(|l| l.trim() == *pattern) {
                lines.remove(pos);
                removed.push(pattern.to_string());
                changed = true;
            }
        }

        if changed {
            let new_content = lines.join("\n");
            // Maintain trailing newline
            let final_content = if new_content.ends_with('\n') || new_content.is_empty() {
                new_content
            } else {
                format!("{}\n", new_content)
            };
            fs::write(&path, final_content)?;
        }

        Ok(removed)
    }

    /// Ensure all default patterns are in .gitignore
    pub fn ensure_defaults(&self) -> Result<()> {
        self.ensure_managed_block(ALWAYS_IGNORE)
    }

    /// Check if a file path looks sensitive
    pub fn is_sensitive_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        SENSITIVE_PATTERNS.iter().any(|p| path_str.contains(p))
    }

    /// Check if a file is an unencrypted .env file that should be encrypted
    /// (.env.example files are safe - they're templates)
    pub fn is_unencrypted_env(&self, path: &Path) -> bool {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // .env.example and .env.sample are templates - safe to commit
        if name.contains(".example") || name.contains(".sample") || name.contains(".template") {
            return false;
        }

        // Check if it looks like an env file
        if name.starts_with(".env") || name.ends_with(".env") {
            // Check if file exists and is plaintext (not encrypted)
            let full_path = self.repo_root.join(path);
            if let Ok(content) = fs::read_to_string(&full_path) {
                // Encrypted files start with "age-encryption.org" header
                if content.starts_with("age-encryption.org")
                    || content.starts_with("-----BEGIN AGE")
                {
                    return false; // Already encrypted
                }
                return true; // Plaintext env file - should encrypt!
            }
        }
        false
    }

    /// Scan for files that should be gitignored but aren't
    pub fn scan_unignored(&self) -> Result<Vec<PathBuf>> {
        let mut unignored = Vec::new();
        let existing = self.read_gitignore();

        // Check for common directories/files
        let check_paths = [
            "node_modules",
            "target",
            "__pycache__",
            ".venv",
            ".idea",
            ".DS_Store",
        ];

        for path in check_paths {
            let full_path = self.repo_root.join(path);
            if full_path.exists()
                && !existing.contains(path)
                && !existing.contains(&format!("{}/", path))
            {
                unignored.push(full_path);
            }
        }

        Ok(unignored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sensitive_path() {
        let auto_ignore = AutoGitIgnore::new(Path::new("/tmp"));

        assert!(auto_ignore.is_sensitive_path(Path::new("config/api_key.txt")));
        assert!(auto_ignore.is_sensitive_path(Path::new("secrets/password.env")));
        assert!(auto_ignore.is_sensitive_path(Path::new("auth_token.json")));
        assert!(!auto_ignore.is_sensitive_path(Path::new("src/main.rs")));
        assert!(!auto_ignore.is_sensitive_path(Path::new("README.md")));
    }

    #[test]
    fn test_always_ignore_patterns() {
        // Ensure we have reasonable defaults
        // Note: .env is NOT ignored because Arcane encrypts it instead
        assert!(
            !ALWAYS_IGNORE.contains(&".env"),
            ".env should NOT be in ALWAYS_IGNORE - Arcane encrypts it"
        );
        assert!(ALWAYS_IGNORE.contains(&"node_modules/"));
        assert!(ALWAYS_IGNORE.contains(&"target/"));
    }
}
