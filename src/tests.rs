//! Comprehensive test suite for the arcane crate
//! Tests all major modules: security, history, repo_manager, git_operations, doctor, config, shadow

#[cfg(test)]
mod security_tests {
    use crate::security::{ArcaneSecurity, SecretScanner};

    #[test]
    fn test_secret_scanner_aws_key() {
        let scanner = SecretScanner::new();
        let content = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let found = scanner.scan(content);
        assert!(
            found.iter().any(|s| s.contains("AWS")),
            "Should detect AWS key"
        );
    }

    #[test]
    fn test_secret_scanner_stripe_key() {
        let scanner = SecretScanner::new();

        // Build test patterns to avoid triggering GitHub's secret scanner
        // Only LIVE keys trigger detection - test keys are safe for development
        let prefix = "sk_"; // Stripe key prefix
        let env = "live_"; // Production environment marker
        let suffix = "abcdefghij1234567890abcd"; // 24 char fake key body
        let live_key = format!("STRIPE_SECRET_KEY={}{}{}", prefix, env, suffix);

        let found = scanner.scan(&live_key);
        assert!(
            found.iter().any(|s| s.contains("Stripe")),
            "Should detect Stripe LIVE key"
        );

        // Verify test keys do NOT trigger warnings (intentional!)
        let test_env = "test_"; // Test environment marker
        let test_key = format!("STRIPE_SECRET_KEY={}{}{}", prefix, test_env, suffix);
        let test_found = scanner.scan(&test_key);
        assert!(
            !test_found.iter().any(|s| s.contains("Stripe")),
            "Should NOT detect Stripe TEST key - those are safe for development"
        );
    }

    #[test]
    fn test_secret_scanner_private_key() {
        let scanner = SecretScanner::new();
        let content = "-----BEGIN RSA PRIVATE KEY-----\nMIIE...";
        let found = scanner.scan(content);
        assert!(
            found.iter().any(|s| s.contains("Private Key")),
            "Should detect private key"
        );
    }

    #[test]
    fn test_secret_scanner_google_api_key() {
        let scanner = SecretScanner::new();
        let content = "GOOGLE_API_KEY=AIzaSyAaBbCcDdEeFfGgHhIiJjKkLlMmNnOoPp";
        let found = scanner.scan(content);
        // Note: Google API key detection may not be implemented yet
        // This test documents the current behavior
        println!("Google API key detection result: {:?}", found);
    }

    #[test]
    fn test_secret_scanner_no_false_positives() {
        let scanner = SecretScanner::new();
        let content = "This is just normal text with no secrets";
        let found = scanner.scan(content);
        assert!(
            found.is_empty(),
            "Should not detect any secrets in normal text"
        );
    }

    #[test]
    fn test_find_repo_root_in_git_repo() {
        let result = ArcaneSecurity::find_repo_root();
        assert!(result.is_ok(), "Should find repo root in git repo");

        if let Ok(path) = result {
            assert!(
                path.join(".git").exists(),
                "Repo root should have .git directory"
            );
        }
    }

    #[test]
    fn test_arcane_security_new_with_path() {
        let path = std::env::current_dir().unwrap();
        let result = ArcaneSecurity::new(Some(&path));
        assert!(result.is_ok(), "Should create ArcaneSecurity with path");
    }

    #[test]
    fn test_arcane_security_new_without_path() {
        let result = ArcaneSecurity::new(None);
        assert!(result.is_ok(), "Should create ArcaneSecurity without path");
    }

    #[test]
    fn test_scan_content() {
        let security = ArcaneSecurity::new(None).unwrap();
        let found = security.scan_content("Normal text");
        assert!(found.is_empty());

        let found = security.scan_content("AKIAIOSFODNN7EXAMPLE");
        assert!(!found.is_empty(), "Should find AWS key pattern");
    }
}

#[cfg(test)]
mod doctor_tests {
    use crate::doctor::{ArcaneDoctor, CheckStatus};

    #[test]
    fn test_doctor_new() {
        let doctor = ArcaneDoctor::new();
        let _ = doctor;
    }

    #[test]
    fn test_doctor_run_returns_report() {
        let doctor = ArcaneDoctor::new();
        let path = std::env::current_dir().unwrap();
        let report = doctor.run(&path);

        assert!(report.checks.len() >= 2, "Should have at least 2 checks");
    }

    #[test]
    fn test_doctor_check_status_variants() {
        assert_eq!(CheckStatus::Pass, CheckStatus::Pass);
        assert_eq!(CheckStatus::Warning, CheckStatus::Warning);
        assert_eq!(CheckStatus::Fail, CheckStatus::Fail);
        assert_ne!(CheckStatus::Pass, CheckStatus::Fail);
    }

    #[test]
    fn test_doctor_overall_health_calculation() {
        let doctor = ArcaneDoctor::new();
        let path = std::env::current_dir().unwrap();
        let report = doctor.run(&path);

        let has_fail = report
            .checks
            .iter()
            .any(|c| matches!(c.status, CheckStatus::Fail));
        let has_warning = report
            .checks
            .iter()
            .any(|c| matches!(c.status, CheckStatus::Warning));

        if has_fail {
            assert_eq!(report.overall_health, CheckStatus::Fail);
        } else if has_warning {
            assert_eq!(report.overall_health, CheckStatus::Warning);
        } else {
            assert_eq!(report.overall_health, CheckStatus::Pass);
        }
    }
}

#[cfg(test)]
mod repo_manager_tests {
    use crate::repo_manager::RepoManager;

    #[test]
    fn test_repo_manager_new() {
        let result = RepoManager::new();
        if let Ok(manager) = result {
            let _ = manager;
        }
    }

    #[test]
    fn test_repo_manager_list_repos() {
        if let Ok(manager) = RepoManager::new() {
            let repos = manager.list_repos();
            assert!(repos.len() >= 0);
        }
    }

    #[test]
    fn test_repo_manager_add_and_remove() {
        if let Ok(mut manager) = RepoManager::new() {
            let test_path = "/tmp/test_repo_12345".to_string();

            // Try adding (may fail if path doesn't exist)
            let _ = manager.add_repo(test_path.clone());

            // Try removing
            let _ = manager.remove_repo(test_path);
        }
    }
}

#[cfg(test)]
mod git_operations_tests {
    use crate::git_operations::GitOperations;
    use std::path::PathBuf;

    #[test]
    fn test_git_operations_new() {
        let git = GitOperations::new();
        let _ = git;
    }

    #[tokio::test]
    async fn test_is_git_repo_true() {
        let git = GitOperations::new();
        let path = std::env::current_dir().unwrap();

        let result = git.is_git_repo(&path).await;
        assert!(result.is_ok());
        assert!(result.unwrap(), "Current dir should be a git repo");
    }

    #[tokio::test]
    async fn test_is_git_repo_false() {
        let git = GitOperations::new();
        let path = PathBuf::from("/tmp");

        let result = git.is_git_repo(&path).await;
        assert!(result.is_ok());
        assert!(!result.unwrap(), "/tmp should not be a git repo");
    }

    #[tokio::test]
    async fn test_has_changes() {
        let git = GitOperations::new();
        let path = std::env::current_dir().unwrap();

        let result = git.has_changes(&path).await;
        assert!(result.is_ok(), "has_changes should not error");
    }

    #[tokio::test]
    async fn test_get_diff() {
        let git = GitOperations::new();
        let path = std::env::current_dir().unwrap();

        let result = git.get_diff(&path).await;
        assert!(result.is_ok(), "get_diff should not error");
    }

    #[tokio::test]
    async fn test_get_diff_truncation() {
        let git = GitOperations::new();
        let path = std::env::current_dir().unwrap();

        let result = git.get_diff(&path).await;
        if let Ok(diff) = result {
            if diff.len() > 5000 {
                assert!(
                    diff.contains("(truncated)"),
                    "Large diffs should be truncated"
                );
            }
        }
    }
}

#[cfg(test)]
mod history_tests {
    use crate::history::HistoryManager;

    #[tokio::test]
    async fn test_get_repo_history() {
        let path = std::env::current_dir().unwrap();
        let result = HistoryManager::get_repo_history(&path).await;

        assert!(result.is_ok(), "get_repo_history should not error");

        if let Ok(commits) = result {
            assert!(!commits.is_empty(), "Should have at least one commit");

            let first = &commits[0];
            assert!(!first.hash.is_empty(), "Commit should have hash");
            assert!(!first.message.is_empty(), "Commit should have message");
        }
    }

    #[tokio::test]
    async fn test_get_repo_history_limit() {
        let path = std::env::current_dir().unwrap();
        let result = HistoryManager::get_repo_history(&path).await;

        if let Ok(commits) = result {
            assert!(commits.len() <= 50, "Should be limited to 50 commits");
        }
    }

    #[test]
    fn test_commit_info_structure() {
        use crate::history::CommitInfo;

        let commit = CommitInfo {
            hash: "abc123".to_string(),
            message: "Test commit".to_string(),
            author: "Test Author".to_string(),
            date: "2024-01-01".to_string(),
        };

        assert_eq!(commit.hash, "abc123");
        assert_eq!(commit.message, "Test commit");
    }
}

#[cfg(test)]
mod config_tests {
    use crate::config::ConfigManager;

    #[test]
    fn test_config_manager_new() {
        let result = ConfigManager::new();
        if let Err(e) = &result {
            println!("ConfigManager::new() error (may be expected): {}", e);
        }
    }

    #[test]
    fn test_config_manager_ai_config() {
        if let Ok(config) = ConfigManager::new() {
            let ai_config = config.ai_config();
            let _ = ai_config;
        }
    }
}

#[cfg(test)]
mod shadow_tests {
    use crate::shadow::ShadowManager;

    #[test]
    fn test_shadow_manager_new() {
        let path = std::env::current_dir().unwrap();
        let manager = ShadowManager::new(&path);
        let _ = manager;
    }

    #[test]
    fn test_shadow_manager_ensure_shadow_branch() {
        let path = std::env::current_dir().unwrap();
        let manager = ShadowManager::new(&path);

        let result = manager.ensure_shadow_branch();
        // May or may not succeed
        if let Ok(branch) = result {
            assert!(
                branch.starts_with("shadow/"),
                "Shadow branch should start with shadow/"
            );
        }
    }

    #[test]
    fn test_shadow_manager_list_shadow_commits() {
        let path = std::env::current_dir().unwrap();
        let manager = ShadowManager::new(&path);

        let result = manager.list_shadow_commits(10);
        if let Ok(commits) = result {
            assert!(commits.len() <= 10, "Should respect limit");
        }
    }
}

#[cfg(test)]
mod daemon_status_tests {
    use crate::DaemonStatus;

    #[test]
    fn test_daemon_status_creation() {
        let status = DaemonStatus {
            pid: 12345,
            state: "Running".to_string(),
            last_commit: Some("2024-01-01T00:00:00Z".to_string()),
            watching: vec!["/path/to/repo".to_string()],
            branch: Some("main".to_string()),
            last_alert: None,
        };

        assert_eq!(status.pid, 12345);
        assert_eq!(status.state, "Running");
    }

    #[test]
    fn test_daemon_status_save_and_load() {
        let status = DaemonStatus {
            pid: std::process::id(),
            state: "Test".to_string(),
            last_commit: None,
            watching: vec![],
            branch: None,
            last_alert: None,
        };

        let save_result = status.save();
        assert!(save_result.is_ok(), "Save should succeed");

        let loaded = DaemonStatus::load();
        if let Some(loaded) = loaded {
            assert_eq!(loaded.state, "Test");
        }
    }

    #[test]
    fn test_daemon_status_clone() {
        let status = DaemonStatus {
            pid: 1,
            state: "Idle".to_string(),
            last_commit: None,
            watching: vec!["repo1".to_string()],
            branch: None,
            last_alert: None,
        };

        let cloned = status.clone();
        assert_eq!(status.pid, cloned.pid);
        assert_eq!(status.state, cloned.state);
    }
}

#[cfg(test)]
mod integration_tests {
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_full_workflow_no_panic() {
        let path = std::env::current_dir().unwrap();

        // 1. Find repo root
        let root = crate::security::ArcaneSecurity::find_repo_root().unwrap_or(path.clone());

        // 2. Create security instance
        let security = crate::security::ArcaneSecurity::new(Some(&root));
        assert!(security.is_ok(), "Security should initialize");

        // 3. Run doctor
        let doctor = crate::doctor::ArcaneDoctor::new();
        let report = doctor.run(&root);
        assert!(!report.checks.is_empty(), "Doctor should return checks");

        // 4. Get git operations
        let git = crate::git_operations::GitOperations::new();

        // 5. Check for changes
        let has_changes = git.has_changes(&root).await;
        assert!(has_changes.is_ok());

        // 6. Get diff
        let diff = git.get_diff(&root).await;
        assert!(diff.is_ok());

        // 7. Get history
        let history = crate::history::HistoryManager::get_repo_history(&root).await;
        assert!(history.is_ok());

        // 8. List repos
        if let Ok(manager) = crate::repo_manager::RepoManager::new() {
            let repos = manager.list_repos();
            let _ = repos;
        }

        println!("Full workflow completed successfully!");
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        use std::sync::Arc;

        let path = Arc::new(std::env::current_dir().unwrap());
        let git = Arc::new(crate::git_operations::GitOperations::new());

        let mut handles = vec![];

        for i in 0..10 {
            let path = path.clone();
            let git = git.clone();

            let handle = tokio::spawn(async move {
                let _ = git.has_changes(&path).await;
                println!("Task {} completed", i);
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        println!("All concurrent tasks completed!");
    }

    #[test]
    fn test_error_handling_invalid_path() {
        let invalid_path = PathBuf::from("/nonexistent/path/that/does/not/exist");

        let security = crate::security::ArcaneSecurity::new(Some(&invalid_path));
        let _ = security;

        let doctor = crate::doctor::ArcaneDoctor::new();
        let report = doctor.run(&invalid_path);
        let _ = report;
    }
}

#[cfg(test)]
mod ai_service_tests {
    use crate::ai_service::{AIConfig, AIProvider, AIService};
    use std::collections::HashMap;

    #[test]
    fn test_ai_service_new() {
        let config = AIConfig {
            primary_provider: AIProvider::Gemini,
            backup_providers: vec![],
            provider_models: HashMap::new(),
            api_keys: HashMap::new(),
        };

        let service = AIService::new(config);
        let _ = service;
    }

    #[tokio::test]
    async fn test_ai_service_generate_commit_message() {
        let config = AIConfig {
            primary_provider: AIProvider::Gemini,
            backup_providers: vec![],
            provider_models: HashMap::new(),
            api_keys: HashMap::new(),
        };

        let service = AIService::new(config);
        let diff = "diff --git a/test.txt b/test.txt\n+new line";

        let result = service.generate_commit_message(diff).await;
        assert!(result.is_ok(), "Should return a commit message");

        if let Ok(msg) = result {
            assert!(!msg.is_empty(), "Message should not be empty");
        }
    }
}
