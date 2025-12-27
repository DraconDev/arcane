//! Arcane Spark - Push-to-Deploy Webhook Server
//!
//! A lightweight daemon that listens for GitHub/GitLab webhooks and triggers deploys.

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use serde_yaml::Value as YamlValue;
use sha2::Sha256;
use std::collections::HashMap;
use std::fs;
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
    pub github_token: Option<String>,
    pub repos: HashMap<String, RepoConfig>,
}

#[derive(Clone, Deserialize)]
pub struct RepoConfig {
    pub url: String,
    pub branch: String,
    pub deploy_target: String,
    pub env: String,
}

#[derive(Deserialize)]
struct SparkToml {
    repos: Vec<RepoEntry>,
}

#[derive(Deserialize)]
struct RepoEntry {
    name: String,
    #[serde(flatten)]
    config: RepoConfig,
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
    repo_url: String,
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
    // Verify signature (only if secret is configured)
    if !state.config.secret.is_empty() {
        let signature = headers
            .get("x-hub-signature-256")
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        if !verify_signature(&state.config.secret, signature, &body) {
            eprintln!("❌ Invalid webhook signature");
            return Err(StatusCode::UNAUTHORIZED);
        }
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
                    repo_url: repo_config.url.clone(),
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
    github_token: Option<String>,
) {
    // Create base repos directory
    let home = std::env::var("HOME").expect("HOME not set");
    let base_dir = std::path::Path::new(&home).join(".arcane/spark/repos");
    std::fs::create_dir_all(&base_dir).expect("Failed to create repos dir");
    let client = Client::new();

    while let Some(job) = rx.recv().await {
        println!(
            "🚀 Starting deploy for {} ({})",
            job.repo_name,
            &job.commit[..7.min(job.commit.len())]
        );

        if let Some(token) = &github_token {
            set_commit_status(
                &client,
                token,
                &job.repo_url,
                &job.commit,
                "pending",
                "Deploy started...",
            )
            .await;
        }

        let repo_dir = base_dir.join(&job.repo_name);

        // 1. Git Sync
        let git_res = if repo_dir.exists() {
            // Reset and Pull
            println!("   🔄 Updating repo in {}", repo_dir.display());
            let status = Command::new("git")
                .current_dir(&repo_dir)
                .args(["fetch", "--all"])
                .status()
                .and_then(|_| {
                    Command::new("git")
                        .current_dir(&repo_dir)
                        .args(["reset", "--hard", &job.commit])
                        .status()
                });
            status
        } else {
            // Clone
            println!("   📥 Cloning {} to {}", job.repo_url, repo_dir.display());
            Command::new("git")
                .current_dir(&base_dir)
                .args(["clone", &job.repo_url, &job.repo_name])
                .status()
        };

        if let Ok(status) = git_res {
            if !status.success() {
                eprintln!("❌ Git sync failed");
                mark_complete(&builds, &job.repo_name);
                continue;
            }
        } else {
            eprintln!("❌ Git command failed");
            mark_complete(&builds, &job.repo_name);
            continue;
        }

        // 2. Arcane Deploy
        let mut cmd = Command::new("arcane");
        cmd.current_dir(&repo_dir)
            .args(["deploy", "--target", &job.target, "--env", &job.env]);

        // Auto-detect compose file
        let mut compose_file = None;
        if repo_dir.join("compose.yml").exists() {
            compose_file = Some("compose.yml");
        } else if repo_dir.join("docker-compose.yml").exists() {
            compose_file = Some("docker-compose.yml");
        }

        if let Some(file) = compose_file {
            cmd.args(["--compose", file]);

            // Auto-inject Traefik labels
            if let Err(e) = inject_traefik_labels(&repo_dir.join(file), &job.repo_name) {
                eprintln!("⚠️ Failed to inject Traefik labels: {}", e);
            }
        }

        let result = cmd.status();

        match result {
            Ok(status) if status.success() => {
                println!("✅ Deploy successful for {}", job.repo_name);
                if let Some(token) = &github_token {
                    set_commit_status(
                        &client,
                        token,
                        &job.repo_url,
                        &job.commit,
                        "success",
                        "Deploy successful!",
                    )
                    .await;
                }
            }
            Ok(status) => {
                eprintln!(
                    "❌ Deploy failed for {} (exit: {:?})",
                    job.repo_name,
                    status.code()
                );
                if let Some(token) = &github_token {
                    set_commit_status(
                        &client,
                        token,
                        &job.repo_url,
                        &job.commit,
                        "failure",
                        "Deploy failed",
                    )
                    .await;
                }
            }
            Err(e) => {
                eprintln!("❌ Deploy error for {}: {}", job.repo_name, e);
                if let Some(token) = &github_token {
                    set_commit_status(
                        &client,
                        token,
                        &job.repo_url,
                        &job.commit,
                        "error",
                        &format!("Error: {}", e),
                    )
                    .await;
                }
            }
        }

        mark_complete(&builds, &job.repo_name);
    }
}

fn mark_complete(builds: &Arc<RwLock<HashMap<String, BuildState>>>, repo_name: &str) {
    let mut builds = builds.write().unwrap();
    if let Some(state) = builds.get_mut(repo_name) {
        state.build_in_progress = false;
    }
}

/// Start the Spark server
pub async fn start_server(port: u16, secret: String) -> anyhow::Result<()> {
    // Load repo config from spark.toml
    let mut repos = HashMap::new();

    match fs::read_to_string("spark.toml") {
        Ok(content) => match toml::from_str::<SparkToml>(&content) {
            Ok(config) => {
                println!("📄 Loaded spark.toml with {} repos", config.repos.len());
                for entry in config.repos {
                    println!("   - {}", entry.name);
                    repos.insert(entry.name, entry.config);
                }
            }
            Err(e) => eprintln!("❌ Failed to parse spark.toml: {}", e),
        },
        Err(_) => println!("⚠️  spark.toml not found, running with empty whitelist"),
    }

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
            github_token: std::env::var("GITHUB_TOKEN").ok(),
            repos,
        },
        builds: builds.clone(),
        deploy_tx,
    };

    // Spawn deploy worker
    tokio::spawn(deploy_worker(
        deploy_rx,
        builds,
        state.config.github_token.clone(),
    ));

    let app = Router::new()
        .route("/webhook", post(handle_webhook))
        .route("/health", axum::routing::get(|| async { "ok" }))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("🎯 Spark is ready! Waiting for webhooks...\n");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn set_commit_status(
    client: &Client,
    token: &str,
    repo_url: &str,
    sha: &str,
    state: &str,
    desc: &str,
) {
    if let Some((owner, repo)) = parse_github_repo(repo_url) {
        let url = format!(
            "https://api.github.com/repos/{}/{}/statuses/{}",
            owner, repo, sha
        );
        let body = json!({
            "state": state,
            "description": desc,
            "context": "arcane/spark",
            "target_url": ""
        });

        let _ = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "arcane-spark")
            .header("Accept", "application/vnd.github.v3+json")
            .json(&body)
            .send()
            .await;
    }
}

fn parse_github_repo(url: &str) -> Option<(String, String)> {
    let url = url.trim_end_matches(".git");
    if let Some(path) = url.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }
    if let Some(path) = url.strip_prefix("git@github.com:") {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }
    None
}

fn inject_traefik_labels(path: &std::path::Path, repo_name: &str) -> anyhow::Result<()> {
    let content = fs::read_to_string(path)?;
    let mut doc: YamlValue = serde_yaml::from_str(&content)?;

    if let Some(services) = doc.get_mut("services").and_then(|v| v.as_mapping_mut()) {
        for (service_name, config) in services.iter_mut() {
            // Check if service should be exposed (has ports or is 'web'/'app')
            let service_name_str = service_name.as_str().unwrap_or_default();
            let is_web = service_name_str == "web" || service_name_str == "app";
            let has_ports = config.get("ports").is_some();

            if is_web || has_ports {
                let mut port = "80".to_string(); // Default

                // Extract port if configured and REMOVE explicit mapping to avoid conflict
                if let Some(ports) = config.get_mut("ports").and_then(|p| p.as_sequence_mut()) {
                    if let Some(first) = ports.first() {
                        let p_str = match first {
                            YamlValue::String(s) => s.clone(),
                            YamlValue::Number(n) => n.to_string(),
                            _ => "80:80".to_string(),
                        };
                        // Parse "8080:80" -> internal port 80
                        if let Some((_, internal)) = p_str.split_once(':') {
                            port = internal.to_string(); // Use internal port
                        } else {
                            port = p_str; // "80" -> 80
                        }
                    }
                    // Remove ports section to prevent binding conflict
                    if let Some(mapping) = config.as_mapping_mut() {
                        mapping.remove("ports");
                    }
                }

                // Add Labels
                let labels = config
                    .as_mapping_mut()
                    .unwrap()
                    .entry(YamlValue::String("labels".to_string()))
                    .or_insert(YamlValue::Sequence(Vec::new()));

                if let YamlValue::Sequence(seq) = labels {
                    // Check if already enabled
                    let has_traefik = seq
                        .iter()
                        .any(|l| l.as_str().unwrap_or("").contains("traefik.enable=true"));

                    if !has_traefik {
                        println!(
                            "   🏷️  Injecting Traefik labels for service '{}' (port {})",
                            service_name_str, port
                        );
                        let host_rule = format!(
                            "traefik.http.routers.{}.rule=Host(`{}.dracon.uk`)",
                            repo_name, repo_name
                        );
                        let port_rule = format!(
                            "traefik.http.services.{}.loadbalancer.server.port={}",
                            repo_name, port
                        );

                        seq.push(YamlValue::String("traefik.enable=true".to_string()));
                        seq.push(YamlValue::String(host_rule));
                        seq.push(YamlValue::String(
                            "traefik.http.routers.tls.certresolver=letsencrypt".to_string(),
                        ));
                        seq.push(YamlValue::String(port_rule));

                        // Add network
                        let networks = config
                            .as_mapping_mut()
                            .unwrap()
                            .entry(YamlValue::String("networks".to_string()))
                            .or_insert(YamlValue::Sequence(Vec::new()));

                        if let YamlValue::Sequence(net_seq) = networks {
                            net_seq.push(YamlValue::String("traefik-public".to_string()));
                        }
                    }
                }

                // Ensure traefik-public network is defined at top level
                if let Some(mapping) = doc.as_mapping_mut() {
                    let networks = mapping
                        .entry(YamlValue::String("networks".to_string()))
                        .or_insert(YamlValue::Mapping(serde_yaml::Mapping::new()));

                    if let YamlValue::Mapping(net_map) = networks {
                        net_map
                            .entry(YamlValue::String("traefik-public".to_string()))
                            .or_insert(serde_yaml::from_str("external: true").unwrap());
                    }
                }

                break; // Only inject for one service
            }
        }
    }

    // Save back
    let new_content = serde_yaml::to_string(&doc)?;
    fs::write(path, new_content)?;

    Ok(())
}
