use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DoctorCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
    pub overall_health: CheckStatus,
}

pub struct ArcaneDoctor;

impl ArcaneDoctor {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, repo_path: &Path) -> DoctorReport {
        let mut checks = Vec::new();

        // 1. Check .env protection
        checks.push(self.check_env_protection(repo_path));

        // 2. Check Key Configuration
        checks.push(self.check_key_configuration(repo_path));

        // Determine overall health
        let overall_health = if checks.iter().any(|c| matches!(c.status, CheckStatus::Fail)) {
            CheckStatus::Fail
        } else if checks
            .iter()
            .any(|c| matches!(c.status, CheckStatus::Warning))
        {
            CheckStatus::Warning
        } else {
            CheckStatus::Pass
        };

        DoctorReport {
            checks,
            overall_health,
        }
    }

    fn check_env_protection(&self, repo_path: &Path) -> DoctorCheck {
        let env_path = repo_path.join(".env");
        if !env_path.exists() {
            return DoctorCheck {
                name: ".env Protection".to_string(),
                status: CheckStatus::Pass,
                message: "No .env file present (safe)".to_string(),
            };
        }

        // Run git check-attr
        // git check-attr filter .env
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(&["check-attr", "filter", ".env"])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                // Expected format: ".env: filter: git-arcane"
                if stdout.contains("filter: git-arcane") {
                    DoctorCheck {
                        name: ".env Protection".to_string(),
                        status: CheckStatus::Pass,
                        message: ".env is correctly protected by git-arcane filter".to_string(),
                    }
                } else {
                    DoctorCheck {
                        name: ".env Protection".to_string(),
                        status: CheckStatus::Fail,
                        message: "CRITICAL: .env exists but is NOT using git-arcane filter!"
                            .to_string(),
                    }
                }
            }
            _ => DoctorCheck {
                name: ".env Protection".to_string(),
                status: CheckStatus::Warning,
                message: "Could not verify git attributes".to_string(),
            },
        }
    }

    fn check_key_configuration(&self, repo_path: &Path) -> DoctorCheck {
        let keys_dir = repo_path.join(".git").join("arcane").join("keys");

        if !keys_dir.exists() {
            return DoctorCheck {
                name: "Key Configuration".to_string(),
                status: CheckStatus::Fail,
                message: "Arcane keys directory missing. Run 'arcane init'.".to_string(),
            };
        }

        match keys_dir.read_dir() {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    DoctorCheck {
                        name: "Key Configuration".to_string(),
                        status: CheckStatus::Pass,
                        message: "Repository keys found.".to_string(),
                    }
                } else {
                    DoctorCheck {
                        name: "Key Configuration".to_string(),
                        status: CheckStatus::Fail,
                        message: "Keys directory is empty.".to_string(),
                    }
                }
            }
            Err(_) => DoctorCheck {
                name: "Key Configuration".to_string(),
                status: CheckStatus::Warning,
                message: "Could not access keys directory".to_string(),
            },
        }
    }
}
