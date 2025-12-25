use crate::ai_service::{AIConfig, AIProvider};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a Security Auditor and Git Committer.
1. Analyze the diff for SECRETS (keys, tokens, passwords) and VULNERABILITIES (CWEs).
2. If DANGEROUS issues are found, output ONLY: SECURITY_ALERT: <brief reason>
3. If clean, output ONLY: COMMIT_MESSAGE: <conventional commit message>

Format: type(scope): short description
Types: feat, fix, docs, style, refactor, perf, test, chore, build, ci, revert
Max 50 chars. Lowercase. No period."#;

pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    // Arcane internal
    ".arcane/shadow/",
    ".arcane/keys/",
    ".gemini/",
    // IDE & Editor
    ".vscode/",
    ".idea/",
    ".vs/",
    "*.swp",
    "*.swo",
    "*~",
    // OS
    ".DS_Store",
    "Thumbs.db",
    // Node.js
    "node_modules/",
    ".npm/",
    "npm-debug.log*",
    "yarn-debug.log*",
    "yarn-error.log*",
    // Rust
    "target/",
    // Build outputs
    "dist/",
    "build/",
    ".next/",
    ".nuxt/",
    ".wxt/",
    // Python
    "__pycache__/",
    "*.pyc",
    ".venv/",
    "venv/",
    // Temp & Cache
    "*.tmp",
    "*.temp",
    "*.log",
    "*.cache",
    // Keys (but .env is handled by git-arcane filter)
    "*.pem",
    "*.key",
    "*.p12",
    // Reference/misc
    "reference/",
    "*.kilocode",
];

pub const DEFAULT_GITATTRIBUTES_PATTERNS: &[&str] = &[
    // Auto-encrypt .env files with git-arcane filter
    "*.env filter=git-arcane diff=git-arcane",
    // Binary files (don't diff)
    "*.lock binary",
    "*.png binary",
    "*.jpg binary",
    "*.mp4 binary",
    "*.gif binary",
    "*.ico binary",
    "*.woff binary",
    "*.woff2 binary",
];

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DaemonConfig {
    pub watch_roots: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TimingConfig {
    #[serde(default = "default_inactivity_delay")]
    pub inactivity_delay: u32, // seconds before commit after file change
    #[serde(default = "default_min_commit_delay")]
    pub min_commit_delay: u32, // minimum seconds between commits
}

fn default_inactivity_delay() -> u32 {
    5
}
fn default_min_commit_delay() -> u32 {
    15
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArcaneConfig {
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub ai_provider: Option<AIProvider>,
    #[serde(default)]
    pub backup_provider_1: Option<AIProvider>,
    #[serde(default)]
    pub backup_provider_2: Option<AIProvider>,
    #[serde(default)]
    pub primary_model: Option<String>,
    #[serde(default)]
    pub backup1_model: Option<String>,
    #[serde(default)]
    pub backup2_model: Option<String>,
    #[serde(default)]
    pub timing: TimingConfig,
    #[serde(default)]
    pub version_bumping: bool,
    #[serde(default)]
    pub auto_commit_enabled: bool,
    #[serde(default)]
    pub auto_push_enabled: bool,
    #[serde(default)]
    pub auto_deploy_enabled: bool,
    #[serde(default)]
    pub model_overrides: HashMap<String, String>, // per-provider defaults
    #[serde(default = "default_ignore_patterns")]
    pub ignore_patterns: Vec<String>,
    #[serde(default = "default_gitattributes_patterns")]
    pub gitattributes_patterns: Vec<String>,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
    #[serde(default)]
    pub shadow_branches: bool, // true = push to shadow/<branch>, false = push to origin/<branch>
    #[serde(default)]
    pub api_keys: HashMap<String, String>, // Provider name -> API key (stored in ~/.arcane/)
}

fn default_ignore_patterns() -> Vec<String> {
    DEFAULT_IGNORE_PATTERNS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn default_gitattributes_patterns() -> Vec<String> {
    DEFAULT_GITATTRIBUTES_PATTERNS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl Default for ArcaneConfig {
    fn default() -> Self {
        Self {
            daemon: DaemonConfig::default(),
            ai_provider: None,
            backup_provider_1: None,
            backup_provider_2: None,
            primary_model: None,
            backup1_model: None,
            backup2_model: None,
            timing: TimingConfig::default(),
            version_bumping: false,
            auto_commit_enabled: false,
            auto_push_enabled: true,
            auto_deploy_enabled: false,
            model_overrides: HashMap::new(),
            ignore_patterns: default_ignore_patterns(),
            gitattributes_patterns: default_gitattributes_patterns(),
            system_prompt: default_system_prompt(),
            shadow_branches: false,

            api_keys: HashMap::new(),
        }
    }
}

fn default_system_prompt() -> String {
    DEFAULT_SYSTEM_PROMPT.to_string()
}

impl ArcaneConfig {
    pub fn load() -> anyhow::Result<Self> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let config_path = home.join(".arcane/config.toml");

        let mut config = if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            toml::from_str(&content)?
        } else {
            Self::default()
        };

        if config.ignore_patterns.is_empty() {
            config.ignore_patterns = DEFAULT_IGNORE_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect();
        }
        if config.gitattributes_patterns.is_empty() {
            config.gitattributes_patterns = DEFAULT_GITATTRIBUTES_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect();
        }

        Ok(config)
    }

    pub fn reset_to_defaults(&mut self, section: &str) {
        match section {
            "gitignore" => {
                self.ignore_patterns = DEFAULT_IGNORE_PATTERNS
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            }
            "gitattributes" => {
                self.gitattributes_patterns = DEFAULT_GITATTRIBUTES_PATTERNS
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            }
            "prompt" => {
                self.system_prompt = DEFAULT_SYSTEM_PROMPT.to_string();
            }
            _ => {}
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let config_dir = home.join(".arcane");
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }
        let config_path = config_dir.join("config.toml");
        let content = toml::to_string_pretty(self)?;
        let mut file = fs::File::create(config_path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConfigManager {
    pub config: ArcaneConfig,
}

impl ConfigManager {
    pub fn new() -> anyhow::Result<Self> {
        let config = ArcaneConfig::load()?;
        Ok(Self { config })
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.config.save()
    }

    pub fn add_watch_root(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let abs_path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(path)
        }
        .canonicalize()?;

        if !self.config.daemon.watch_roots.contains(&abs_path) {
            self.config.daemon.watch_roots.push(abs_path);
            self.save()?;
        }
        Ok(())
    }

    pub fn set_ai_provider(&mut self, provider: AIProvider) -> anyhow::Result<()> {
        self.config.ai_provider = Some(provider);
        self.save()
    }

    pub fn ai_config(&self) -> AIConfig {
        let mut provider_models = HashMap::new();

        // Cloud providers (recommended)
        provider_models.insert(AIProvider::Gemini, "gemini-2.0-flash-lite".to_string());
        provider_models.insert(AIProvider::OpenRouter, "qwen/qwen3-coder:free".to_string());

        // Local fallback (for offline/privacy)
        provider_models.insert(AIProvider::Ollama, "qwen2.5:7b".to_string());

        // Apply overrides from config
        for (provider_name, model_name) in &self.config.model_overrides {
            let provider = match provider_name.to_lowercase().as_str() {
                "gemini" => Some(AIProvider::Gemini),
                "openrouter" => Some(AIProvider::OpenRouter),
                "openai" => Some(AIProvider::OpenAI),
                "anthropic" => Some(AIProvider::Anthropic),
                "copilot" => Some(AIProvider::Copilot),
                "ollama" => Some(AIProvider::Ollama),
                _ => None,
            };
            if let Some(p) = provider {
                provider_models.insert(p, model_name.clone());
            }
        }

        // Load API keys: Config takes priority, then environment variables
        let mut api_keys = HashMap::new();

        // Helper to get key from config or env
        let get_key = |provider: &str,
                       env_var: &str,
                       config_keys: &HashMap<String, String>|
         -> Option<String> {
            // Check config first
            if let Some(key) = config_keys.get(provider) {
                if !key.is_empty() {
                    return Some(key.clone());
                }
            }
            // Fallback to env var
            std::env::var(env_var).ok()
        };

        if let Some(key) = get_key("Gemini", "GEMINI_API_KEY", &self.config.api_keys) {
            api_keys.insert(AIProvider::Gemini, key);
        }
        if let Some(key) = get_key("OpenRouter", "OPENROUTER_API_KEY", &self.config.api_keys) {
            api_keys.insert(AIProvider::OpenRouter, key);
        }
        if let Some(key) = get_key("OpenAI", "OPENAI_API_KEY", &self.config.api_keys) {
            api_keys.insert(AIProvider::OpenAI, key);
        }
        if let Some(key) = get_key("Anthropic", "ANTHROPIC_API_KEY", &self.config.api_keys) {
            api_keys.insert(AIProvider::Anthropic, key);
        }

        // Determine primary provider based on config preference OR available keys
        // Priority: Config Preference > OpenRouter > Gemini > Ollama
        let primary_provider = if let Some(pref) = self.config.ai_provider.clone() {
            pref
        } else if api_keys.contains_key(&AIProvider::OpenRouter) {
            AIProvider::OpenRouter
        } else if api_keys.contains_key(&AIProvider::Gemini) {
            AIProvider::Gemini
        } else {
            // Fallback to Ollama if no cloud keys configured
            AIProvider::Ollama
        };

        // Build backup chain
        let backup_providers = match primary_provider {
            AIProvider::Gemini => vec![AIProvider::OpenRouter, AIProvider::Ollama],
            AIProvider::OpenRouter => vec![AIProvider::Gemini, AIProvider::Ollama],
            AIProvider::Ollama => vec![AIProvider::OpenRouter, AIProvider::Gemini],
            _ => vec![AIProvider::Ollama],
        };

        AIConfig {
            primary_provider,
            backup_providers,
            provider_models,
            api_keys,
        }
    }
}
