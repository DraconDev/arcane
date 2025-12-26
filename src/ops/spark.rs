//! Arcane Spark - Push-to-Deploy Webhook Server
//!
//! A lightweight daemon that listens for GitHub/GitLab webhooks and triggers deploys.

use crate::ops::config::OpsConfig;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

type HmacSha256 = Hmac<Sha256>;

/// Spark configuration
#[derive(Clone)]
pub struct SparkConfig {
    pub port: u16,
    pub secret: String,
    pub repos: HashMap<String, RepoConfig>,
}

#[derive(Clone)]
pub struct RepoConfig {
    pub branch: String,
    pub deploy_target: String,
    pub env: String,
}

/// Build state for debounce + latest wins
struct BuildState {
    pending_commit: Option<String>,
    last_push_time: Instant,
    build_in_progress: bool,
}

impl Default for BuildState {
    fn default() -> Self {
        Self {
            pending_commit: None,
            last_push_time: Instant::now() - Duration::from_secs(60),
            build_in_progress: false,
        }
    }
}

/// Shared state across all webhook handlers
#[derive(Clone)]
struct AppState {
    config: SparkConfig,
    builds: Arc<RwLock<HashMap<String, BuildState>>>,
    deploy_tx: mpsc::Sender<DeployJob>,
}

struct DeployJob {
    repo_name: String,
    commit: String,
    target: String,
    env: String,
}

/// Verify GitHub webhook signature
fn verify_signature(secret: &str, signature: &str, body: &[u8]) -> bool {
    let sig_parts: Vec<&str> = signature.split('=').collect();
    if sig_parts.len() != 2 || sig_parts[0] != "sha256" {
        return false;
    }

    let expected = match hex::decode(sig_parts[1]) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take any key");
    mac.update(body);
    mac.verify_slice(&expected).is_ok()
}

/// Handle incoming webhook
async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<&'static str, StatusCode> {
    // Get signature header
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Verify signature
    if !verify_signature(&state.config.secret, signature, &body) {
        eprintln!("❌ Invalid webhook signature");
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Parse payload
    let payload: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Extract ref and repo
    let git_ref = payload["ref"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let repo_url = payload["repository"]["clone_url"]
        .as_str()
        .or_else(|| payload["repository"]["html_url"].as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let commit = payload["after"].as_str().unwrap_or("HEAD").to_string();

    // Extract repo name from URL
    let repo_name = repo_url
        .split('/')
        .last()
        .unwrap_or("unknown")
        .trim_end_matches(".git")
        .to_string();

    println!(
        "📥 Webhook received: {} ({})",
        repo_name,
        &commit[..7.min(commit.len())]
    );

    // Check if repo is in whitelist
    let repo_config = state
        .config
        .repos
        .get(&repo_name)
        .ok_or_else(|| {
            eprintln!("⚠️  Repo '{}' not in whitelist, ignoring", repo_name);
            StatusCode::ACCEPTED
        })?
        .clone();

    // Check branch
    let expected_ref = format!("refs/heads/{}", repo_config.branch);
    if git_ref != expected_ref {
        println!("   ℹ️  Branch {} != {}, ignoring", git_ref, expected_ref);
        return Ok("ignored");
    }

    // Update build state (debounce + latest wins)
    {
        let mut builds = state.builds.write().unwrap();
        let build_state = builds.entry(repo_name.clone()).or_default();

        // If build in progress, we'll cancel and restart (latest wins)
        build_state.pending_commit = Some(commit.clone());
        build_state.last_push_time = Instant::now();
    }

    // Schedule deploy after debounce
    let repo_name_clone = repo_name.clone();
    let state_clone = state.clone();
    tokio::spawn(async move {
        // Wait for debounce period
        tokio::time::sleep(Duration::from_secs(10)).await;

        // Check if this commit is still the latest
        let should_build = {
            let mut builds = state_clone.builds.write().unwrap();
            let build_state = builds.entry(repo_name_clone.clone()).or_default();

            // Only build if no newer push came in during debounce
            if build_state.last_push_time.elapsed() >= Duration::from_secs(10) {
                if let Some(pending) = build_state.pending_commit.take() {
                    if pending == commit {
                        build_state.build_in_progress = true;
                        true
                    } else {
                        false // Newer commit came in
                    }
                } else {
                    false
                }
            } else {
                false // Still in debounce window
            }
        };

        if should_build {
            let _ = state_clone
                .deploy_tx
                .send(DeployJob {
                    repo_name: repo_name_clone,
                    commit,
                    target: repo_config.deploy_target,
                    env: repo_config.env,
                })
                .await;
        }
    });

    Ok("accepted")
}

/// Deploy worker - runs builds sequentially per repo
async fn deploy_worker(
    mut rx: mpsc::Receiver<DeployJob>,
    builds: Arc<RwLock<HashMap<String, BuildState>>>,
) {
    while let Some(job) = rx.recv().await {
        println!(
            "🚀 Starting deploy for {} ({})",
            job.repo_name,
            &job.commit[..7.min(job.commit.len())]
        );

        // Run arcane deploy
        let result = Command::new("arcane")
            .args(["deploy", "--target", &job.target, "--env", &job.env])
            .status();

        match result {
            Ok(status) if status.success() => {
                println!("✅ Deploy successful for {}", job.repo_name);
            }
            Ok(status) => {
                eprintln!(
                    "❌ Deploy failed for {} (exit: {:?})",
                    job.repo_name,
                    status.code()
                );
            }
            Err(e) => {
                eprintln!("❌ Deploy error for {}: {}", job.repo_name, e);
            }
        }

        // Mark build as complete
        {
            let mut builds = builds.write().unwrap();
            if let Some(state) = builds.get_mut(&job.repo_name) {
                state.build_in_progress = false;
            }
        }
    }
}

/// Start the Spark server
pub async fn start_server(port: u16, secret: String) -> anyhow::Result<()> {
    // Load repo config from servers.toml or spark.toml
    let _ops_config = OpsConfig::load();
    let repos = HashMap::new();

    // For MVP, allow all repos with default config
    // TODO: Load from spark.toml
    println!("⚡ Arcane Spark starting on port {}", port);
    println!("   Webhook URL: http://0.0.0.0:{}/webhook", port);
    println!(
        "   Secret configured: {}",
        if secret.is_empty() {
            "❌ NO"
        } else {
            "✅ YES"
        }
    );

    let (deploy_tx, deploy_rx) = mpsc::channel(32);
    let builds = Arc::new(RwLock::new(HashMap::new()));

    let state = AppState {
        config: SparkConfig {
            port,
            secret,
            repos,
        },
        builds: builds.clone(),
        deploy_tx,
    };

    // Spawn deploy worker
    tokio::spawn(deploy_worker(deploy_rx, builds));

    let app = Router::new()
        .route("/webhook", post(handle_webhook))
        .route("/health", axum::routing::get(|| async { "ok" }))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("🎯 Spark is ready! Waiting for webhooks...\n");

    axum::serve(listener, app).await?;

    Ok(())
}
