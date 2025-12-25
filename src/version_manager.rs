use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SemVerBump {
    Major,
    Minor,
    Patch,
    None,
}

pub struct VersionManager;

impl VersionManager {
    // Ported from VersionCoreService.ts (git-ai-committer)
    const VERSION_FILES: &'static [&'static str] = &[
        "package.json",
        "Cargo.toml",
        "pyproject.toml",
        "version.txt",
    ];

    pub fn detect_version_file(root: &Path) -> Option<PathBuf> {
        for &file_name in Self::VERSION_FILES {
            let path = root.join(file_name);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    pub fn get_current_version(file_path: &Path) -> Result<String> {
        let content = fs::read_to_string(file_path)?;
        let file_name = file_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        if file_name == "package.json" {
            let v: serde_json::Value = serde_json::from_str(&content)?;
            v["version"]
                .as_str()
                .map(|s| s.to_string())
                .context("No version field in package.json")
        } else if file_name == "Cargo.toml" || file_name == "pyproject.toml" {
            // Regex match: version = "x.y.z"
            let re = Regex::new(r#"version\s*=\s*["']([^"']+)["']"#).unwrap();
            re.captures(&content)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .context("No version found in TOML file")
        } else {
            Ok(content.trim().to_string())
        }
    }

    pub fn apply_bump(file_path: &Path, bump: SemVerBump) -> Result<(String, String)> {
        if bump == SemVerBump::None {
            let v = Self::get_current_version(file_path)?;
            return Ok((v.clone(), v));
        }

        let old_content = fs::read_to_string(file_path)?;
        let current_ver = Self::get_current_version(file_path)?;
        let new_ver = Self::bump_string(&current_ver, bump)?;

        let new_content = if file_path.ends_with("package.json") {
            // Regex replace for JSON to preserve formatting
            // "version": "1.2.3"
            let escaped_ver = regex::escape(&current_ver);
            let re = Regex::new(&format!(r#"("version":\s*)"{}""#, escaped_ver)).unwrap();

            // Check if it matches first
            if !re.is_match(&old_content) {
                // Fallback to less strict regex if formatting is weird
                // This might be risky if there are multiple "version" keys, but package.json usually has one top-level
                // Attempting generic replacement
                let re_generic = Regex::new(&format!(r#""{}""#, escaped_ver)).unwrap();
                re_generic
                    .replace(&old_content, format!("\"{}\"", new_ver))
                    .to_string()
            } else {
                re.replace(&old_content, |caps: &regex::Captures| {
                    format!("{}\"{}\"", &caps[1], new_ver)
                })
                .to_string()
            }
        } else if file_path.ends_with("Cargo.toml") || file_path.ends_with("pyproject.toml") {
            // version = "1.2.3"
            let escaped_ver = regex::escape(&current_ver);
            let re = Regex::new(&format!(r#"(version\s*=\s*["']){}(["'])"#, escaped_ver)).unwrap();
            re.replace(&old_content, |caps: &regex::Captures| {
                format!("{}{}{}", &caps[1], new_ver, &caps[2])
            })
            .to_string()
        } else {
            // Text file
            new_ver.clone()
        };

        if new_content != old_content {
            fs::write(file_path, &new_content)?;
        }

        Ok((current_ver, new_ver))
    }

    fn bump_string(ver: &str, bump: SemVerBump) -> Result<String> {
        // Strip v prefix if present
        let clean_ver = ver.strip_prefix('v').unwrap_or(ver);

        let parts: Vec<&str> = clean_ver.split('.').collect();
        if parts.len() < 3 {
            // Fallback for non-semver compliant strings?
            // Or just try to parse what we can.
            // For now error out to be safe
            anyhow::bail!("Version string '{}' is not X.Y.Z format", ver);
        }

        let mut major: u32 = parts[0].parse().unwrap_or(0);
        let mut minor: u32 = parts[1].parse().unwrap_or(0);
        let mut patch: u32 = parts[2].parse().unwrap_or(0);

        match bump {
            SemVerBump::Major => {
                major += 1;
                minor = 0;
                patch = 0;
            }
            SemVerBump::Minor => {
                minor += 1;
                patch = 0;
            }
            SemVerBump::Patch => {
                patch += 1;
            }
            SemVerBump::None => {}
        }

        let new_ver = format!("{}.{}.{}", major, minor, patch);

        // Restore v prefix if it was there
        if ver.starts_with('v') {
            Ok(format!("v{}", new_ver))
        } else {
            Ok(new_ver)
        }
    }
}
