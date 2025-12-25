use anyhow::{anyhow, Context, Result};
use chrono::Local;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::version_manager::SemVerBump;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub enum AIProvider {
    Gemini,
    OpenRouter,
    OpenAI,
    Anthropic,
    Copilot,
    Ollama,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AIConfig {
    pub primary_provider: AIProvider,
    pub backup_providers: Vec<AIProvider>,
    pub provider_models: std::collections::HashMap<AIProvider, String>,
    pub api_keys: std::collections::HashMap<AIProvider, String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AIAttempt {
    pub provider: AIProvider,
    pub model: Option<String>,
    pub duration: Duration,
    pub success: bool,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct AIService {
    config: AIConfig,
    client: Client,
    retry_policy: RetryPolicy,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RetryPolicy {
    max_retries: usize,
    base_delay: Duration,
}

impl RetryPolicy {
    pub fn exponential_backoff(base_delay: Duration, max_retries: usize) -> Self {
        Self {
            max_retries,
            base_delay,
        }
    }
}

#[allow(dead_code)]
impl AIService {
    pub fn new(config: AIConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            retry_policy: RetryPolicy::exponential_backoff(Duration::from_millis(100), 3),
        }
    }

    pub async fn analyze_semver(&self, diff: &str) -> anyhow::Result<SemVerBump> {
        let prompt = format!(
            "You are a Release Manager. Analyze the following code changes (git diff) and determine the Semantic Versioning bump required.\n\
            Return ONLY one of the following words: 'Major', 'Minor', 'Patch', 'None'.\n\
            \n\
            Rules:\n\
            - Major: Breaking API changes (incompatible).\n\
            - Minor: New features (backward compatible functionality).\n\
            - Patch: Bug fixes, refactoring, docs, performance, chores (backward compatible).\n\
            - None: No version bump needed (e.g. CI config only, no code).\n\
            \n\
            Diff:\n\
            {}\n\
            \n\
            Response:",
            diff
        );

        let result = self
            .try_providers_for_prompt(&prompt)
            .await
            .context("Failed to analyze semver")?;
        let clean_res = result.trim().to_lowercase();

        if clean_res.contains("major") {
            Ok(SemVerBump::Major)
        } else if clean_res.contains("minor") {
            Ok(SemVerBump::Minor)
        } else if clean_res.contains("patch") {
            Ok(SemVerBump::Patch)
        } else {
            Ok(SemVerBump::None)
        }
    }

    pub async fn generate_commit_message(&self, diff: &str) -> Result<String> {
        let simplified_diff = self.simplify_diff(diff);
        let mut attempts = Vec::new();

        // Try providers in order: primary, backup1, backup2
        let providers = self.get_provider_order();

        for provider in providers {
            let attempt = self.try_provider(provider, &simplified_diff).await;
            attempts.push(attempt.clone());

            if let Some(message) = attempt.message {
                let cleaned = self.clean_response(&message);
                if !cleaned.is_empty() {
                    return Ok(cleaned);
                }
            }
        }

        // All failed - return fallback
        Ok(self.generate_fallback_message())
    }

    fn clean_response(&self, raw: &str) -> String {
        // 1. Remove Markdown code blocks if present
        let mut text = raw.to_string();
        if let Some(start) = text.find("```") {
            if let Some(end) = text[start + 3..].find("```") {
                let content = &text[start + 3..start + 3 + end];
                let lines: Vec<&str> = content.lines().collect();
                if lines.len() > 1 && !lines[0].contains(' ') {
                    text = lines[1..].join("\n");
                } else {
                    text = content.to_string();
                }
            } else {
                text = text.replace("```", "");
            }
        }

        // 2. Strip "COMMIT_MESSAGE:" prefix (from our system prompt)
        if let Some(stripped) = text.strip_prefix("COMMIT_MESSAGE:") {
            text = stripped.to_string();
        }

        // 3. Scan for Conventional Commit Header (Strong Heuristic)
        // Regex: ^[a-z]+(\([a-z0-9-]+\))?: .+$
        // If we find a line matching this, we discard everything before it.
        // We use a simplified check to avoid heavy regex if possible, but regex is safer.
        // Let's use basic string matching for common types.
        let common_types = [
            "feat", "fix", "docs", "style", "refactor", "perf", "test", "chore", "build", "ci",
            "revert",
        ];
        let lines: Vec<&str> = text.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let lower = line.trim().to_lowercase();
            // Check if line starts with specific type
            for t in common_types {
                // e.g. "feat:" or "feat("
                if lower.starts_with(&format!("{}:", t)) || lower.starts_with(&format!("{}(", t)) {
                    return lines[i..].join("\n").trim().to_string();
                }
            }
        }

        fn goto_extraction(_idx: usize, _lines: &[&str]) {} // dummy closure to break simple loops

        // 4. Aggressive fallback: Strip conversational preamble
        // Common patterns:
        //   "Here's a concise and descriptive commit message for this change:\n\nfeat: ..."
        //   "Here's a concise commit message:\n\n**Commit Message:**\n\nfeat: ..."
        //   "Sure! Here's a commit message:\n\nfeat: ..."
        let lower = text.to_lowercase();

        // List of garbage prefixes the AI loves to add
        let garbage_prefixes = [
            "here's a concise",
            "here is a concise",
            "here's a commit",
            "here is a commit",
            "here's the commit",
            "here is the commit",
            "here's a descriptive",
            "here is a descriptive",
            "sure!",
            "sure,",
            "okay,",
            "certainly!",
            "**commit message:**",
            "commit message:",
        ];

        // If text starts with garbage, find the first conventional commit line
        let starts_garbage = garbage_prefixes.iter().any(|p| lower.starts_with(p));

        if starts_garbage {
            // Re-scan lines for first conventional commit
            for line in text.lines() {
                let trimmed = line.trim();
                let lower_line = trimmed.to_lowercase();
                for t in common_types {
                    if lower_line.starts_with(&format!("{}:", t))
                        || lower_line.starts_with(&format!("{}(", t))
                    {
                        // Found it! Return from this line onwards
                        let idx = text.find(trimmed).unwrap_or(0);
                        return text[idx..].trim().to_string();
                    }
                }
            }
            // If we still haven't found a conventional commit, take first non-empty line after a colon
            if let Some(colon_idx) = text.find(':') {
                let after_colon = &text[colon_idx + 1..];
                for line in after_colon.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with("*") && !trimmed.starts_with("-")
                    {
                        return trimmed.to_string();
                    }
                }
            }
        }

        text.trim().trim_matches('"').trim_matches('\'').to_string()
    }

    fn simplify_diff(&self, diff: &str) -> String {
        let lines: Vec<&str> = diff.lines().collect();
        if lines.len() > 200 {
            let truncated: Vec<&str> = lines.into_iter().take(200).collect();
            format!("{}\n... (truncated)", truncated.join("\n"))
        } else {
            lines.join("\n")
        }
    }

    async fn try_providers_for_prompt(&self, prompt: &str) -> Result<String> {
        let providers = self.get_provider_order();

        for provider in providers {
            let model = self.config.provider_models.get(&provider);

            let result = match provider {
                AIProvider::Gemini => self.call_gemini(prompt, model).await,
                AIProvider::OpenRouter => self.call_openrouter(prompt, model).await,
                AIProvider::OpenAI => self.call_openai(prompt, model).await,
                AIProvider::Anthropic => self.call_anthropic(prompt, model).await,
                AIProvider::Copilot => self.call_copilot(prompt, model).await,
                AIProvider::Ollama => self.call_ollama(prompt, model).await,
            };

            if let Ok(msg) = result {
                return Ok(msg);
            }
        }
        anyhow::bail!("All providers failed")
    }

    async fn try_provider(&self, provider: AIProvider, diff: &str) -> AIAttempt {
        let model = self.config.provider_models.get(&provider);
        let start_time = Instant::now();

        // Construct Commit Prompt
        // Check for System Prompt
        let system_instruction = if let Ok(config) = crate::config::ArcaneConfig::load() {
            config.system_prompt
        } else {
            // Default Fallback
            r#"You are a Security Auditor and Git Committer.
1. Analyze the diff for SECRETS (keys, tokens, passwords) and VULNERABILITIES (CWEs).
2. If DANGEROUS issues are found, output ONLY: SECURITY_ALERT: <brief reason>
3. If clean, output ONLY: COMMIT_MESSAGE: <conventional commit message>

Format: type(scope): short description
Types: feat, fix, docs, style, refactor, perf, test, chore, build, ci, revert
Max 50 chars. Lowercase. No period."#
                .to_string()
        };

        let prompt = format!("{}\n\nDiff:\n{}", system_instruction, diff);
        let result = match provider {
            AIProvider::Gemini => self.call_gemini(&prompt, model).await,
            AIProvider::OpenRouter => self.call_openrouter(&prompt, model).await,
            AIProvider::OpenAI => self.call_openai(&prompt, model).await,
            AIProvider::Anthropic => self.call_anthropic(&prompt, model).await,
            AIProvider::Copilot => self.call_copilot(&prompt, model).await,
            AIProvider::Ollama => self.call_ollama(&prompt, model).await,
        };

        let (message, error) = match &result {
            Ok(msg) => (Some(msg.clone()), None),
            Err(e) => (None, Some(e.to_string())),
        };

        AIAttempt {
            provider,
            model: model.cloned(),
            duration: start_time.elapsed(),
            success: result.is_ok(),
            message,
            error,
        }
    }

    pub async fn check_connectivity(
        &self,
        provider: AIProvider,
        model: Option<String>,
    ) -> AIAttempt {
        let start_time = Instant::now();
        let prompt = "Say 'OK' and nothing else.";

        let result = match provider {
            AIProvider::Gemini => self.call_gemini(prompt, model.as_ref()).await,
            AIProvider::OpenRouter => self.call_openrouter(prompt, model.as_ref()).await,
            AIProvider::OpenAI => self.call_openai(prompt, model.as_ref()).await,
            AIProvider::Anthropic => self.call_anthropic(prompt, model.as_ref()).await,
            AIProvider::Copilot => self.call_copilot(prompt, model.as_ref()).await,
            AIProvider::Ollama => self.call_ollama(prompt, model.as_ref()).await,
        };

        let (message, error) = match &result {
            Ok(msg) => (Some(msg.clone()), None),
            Err(e) => (None, Some(e.to_string())),
        };

        AIAttempt {
            provider,
            model,
            duration: start_time.elapsed(),
            success: result.is_ok(),
            message,
            error,
        }
    }

    fn generate_fallback_message(&self) -> String {
        format!("arcane: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))
    }

    fn get_provider_order(&self) -> Vec<AIProvider> {
        let mut providers = vec![self.config.primary_provider.clone()];
        providers.extend(self.config.backup_providers.clone());
        providers
    }

    async fn call_gemini(&self, prompt: &str, model: Option<&String>) -> Result<String> {
        let api_key = self
            .config
            .api_keys
            .get(&AIProvider::Gemini)
            .ok_or_else(|| anyhow!("Gemini API key not configured"))?;

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model.unwrap_or(&"gemini-1.5-flash".to_string()),
            api_key
        );

        let body = serde_json::json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }]
        });

        let response = self.client.post(&url).json(&body).send().await?;
        let status = response.status();

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!(
                "Gemini API error: {} - Body: {}",
                status,
                error_text
            ));
        }

        let json: serde_json::Value = response.json().await?;
        let text = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid Gemini response format"))?
            .trim()
            .to_string();

        Ok(text)
    }

    async fn call_openrouter(&self, prompt: &str, model: Option<&String>) -> Result<String> {
        let api_key = self
            .config
            .api_keys
            .get(&AIProvider::OpenRouter)
            .ok_or_else(|| anyhow!("OpenRouter API key not configured"))?;

        // Model cascade: try primary, then backups (code-focused free models)
        // Note: xiaomi/mimo-v2-flash is super smart but may not stay free
        let models = [
            model
                .map(|s| s.as_str())
                .unwrap_or("xiaomi/mimo-v2-flash:free"),
            "qwen/qwen3-coder:free",
            "mistralai/devstral-2512:free",
            "google/gemini-2.0-flash-exp:free",
        ];

        let mut last_error = anyhow!("No models tried");

        for model_name in models {
            let body = serde_json::json!({
                "model": model_name,
                "messages": [{"role": "user", "content": prompt}]
            });

            let response = self
                .client
                .post("https://openrouter.ai/api/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(text) = json["choices"][0]["message"]["content"].as_str() {
                            // Don't clean message for generic prompts, only generic trim
                            return Ok(text.trim().to_string());
                        }
                    }
                }
                Ok(resp) => {
                    last_error = anyhow!("OpenRouter {} error: {}", model_name, resp.status());
                }
                Err(e) => {
                    last_error = anyhow!("OpenRouter {} request failed: {}", model_name, e);
                }
            }
        }

        Err(last_error)
    }

    fn clean_commit_message(&self, text: &str) -> String {
        text.lines()
            .next()
            .unwrap_or(text)
            .trim()
            .trim_matches('`')
            .trim_matches('"')
            .trim_matches('\'')
            .chars()
            .take(72)
            .collect()
    }

    async fn call_openai(&self, prompt: &str, model: Option<&String>) -> Result<String> {
        let api_key = self
            .config
            .api_keys
            .get(&AIProvider::OpenAI)
            .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;

        let body = serde_json::json!({
            "model": model.unwrap_or(&"gpt-4o".to_string()),
            "messages": [{"role": "user", "content": prompt}]
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("OpenAI API error: {}", response.status()));
        }

        let json: serde_json::Value = response.json().await?;
        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid OpenAI response format"))?
            .trim()
            .to_string();

        Ok(text)
    }

    async fn call_anthropic(&self, prompt: &str, model: Option<&String>) -> Result<String> {
        let api_key = self
            .config
            .api_keys
            .get(&AIProvider::Anthropic)
            .ok_or_else(|| anyhow!("Anthropic API key not configured"))?;

        let body = serde_json::json!({
            "model": model.unwrap_or(&"claude-3-sonnet-20240229".to_string()),
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": prompt}]
        });

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Anthropic API error: {}", response.status()));
        }

        let json: serde_json::Value = response.json().await?;
        let text = json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid Anthropic response format"))?
            .trim()
            .to_string();

        Ok(text)
    }

    async fn call_copilot(&self, prompt: &str, model: Option<&String>) -> Result<String> {
        let api_key = self
            .config
            .api_keys
            .get(&AIProvider::Copilot)
            .ok_or_else(|| anyhow!("Copilot API key not configured"))?;

        let body = serde_json::json!({
            "model": model.unwrap_or(&"copilot-gpt-4".to_string()),
            "messages": [{"role": "user", "content": prompt}]
        });

        let response = self
            .client
            .post("https://api.githubcopilot.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Copilot API error: {}", response.status()));
        }

        let json: serde_json::Value = response.json().await?;
        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid Copilot response format"))?
            .trim()
            .to_string();

        Ok(text)
    }

    async fn call_ollama(&self, prompt: &str, model: Option<&String>) -> Result<String> {
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());

        let url = format!("{}/api/generate", base_url);

        let body = serde_json::json!({
            "model": model.unwrap_or(&"llama3".to_string()),
            "prompt": prompt,
            "stream": false
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Ollama API error: {}", response.status()));
        }

        let json: serde_json::Value = response.json().await?;
        let text = json["response"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid Ollama response format (missing 'response')"))?
            .trim()
            .to_string();

        Ok(text)
    }
    pub async fn analyze_commits_for_squash(
        &self,
        commits: &[crate::git_operations::CommitInfo],
    ) -> Result<SquashPlan> {
        let commit_list: Vec<String> = commits
            .iter()
            .map(|c| {
                format!(
                    "{} {}",
                    c.hash.chars().take(7).collect::<String>(),
                    c.message
                )
            })
            .collect();
        let commit_block = commit_list.join("\n");

        let prompt = format!(
            r#"You are a Git Historian. I have a list of unpushed commits.
Please group them into LOGICAL ATOMIC SETS to be squashed together.

Commits (Newest First):
{}

Rules:
1. Contiguous commits that fix the same thing (e.g. "Fix typo", "Fix typo again") should be 1 group.
2. Distinct features MUST remain separate groups. DO NOT SQUASH UNRELATED FEATURES TOGETHER.
3. Ideally, you should output MULTIPLE small groups rather than one large group, unless every single commit is about the exact same atomic change.
4. Groups must respect chronological order (you can only squash adjacent commits safely).
5. If a commit stands alone and is distinct, it is a group of size 1. Keep it separate.
6. Output specific JSON format.

JSON Format:
{{
  "groups": [
    {{
      "target_message": "feat(auth): implement login flow",
      "commits": ["hash1", "hash2"]
    }},
    {{
      "target_message": "fix(ui): correct padding",
      "commits": ["hash3"]
    }}
  ]
}}

Response ONLY VALID JSON."#,
            commit_block
        );

        let response = self.try_providers_for_prompt(&prompt).await?;
        let json_str = self.clean_json_response(&response);
        let plan: SquashPlan =
            serde_json::from_str(&json_str).context("Failed to parse AI Squash Plan JSON")?;

        Ok(plan)
    }

    pub async fn analyze_commits_for_lazy_squash(
        &self,
        commits: &[crate::git_operations::CommitInfo],
        use_minor: bool,
    ) -> Result<SquashPlan> {
        let commit_list: Vec<String> = commits
            .iter()
            .map(|c| {
                format!(
                    "{} {}",
                    c.hash.chars().take(7).collect::<String>(),
                    c.message
                )
            })
            .collect();
        // Limit context if too large, but for summary we want as much as possible.
        // If > 100 commits, maybe just send subject lines?
        let commit_block = if commit_list.len() > 200 {
            commit_list
                .iter()
                .take(200)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
                + "\n... (truncated)"
        } else {
            commit_list.join("\n")
        };

        let version_type = if use_minor { "MINOR" } else { "MAJOR" };
        let message_example = if use_minor {
            "feat: consolidate all changes including <top 3 features>"
        } else {
            "feat!: major overhaul of auth and ui systems"
        };

        let prompt = format!(
            r#"You are a Release Manager. I have {} commits.
I want to SQUASH ALL OF THEM into A SINGLE COMMIT.
This is a {} update.

Commits:
{}

Instructions:
1. Create ONE single group containing ALL provided commit hashes.
2. The target_message MUST be a Conventional Commit.
   - For MINOR: use "feat: ..." (no bang). 
   - For MAJOR: use "feat!: ..." (with bang for breaking).
   Example: "{}"
3. Summarize the high-level impact.

JSON Format:
{{{{
  "groups": [
    {{{{
      "target_message": "...",
      "commits": ["<all_hashes_in_order>"]
    }}}}
  ]
}}}}

Response ONLY VALID JSON."#,
            commits.len(),
            version_type,
            commit_block,
            message_example
        );

        let response = self.try_providers_for_prompt(&prompt).await?;
        let json_str = self.clean_json_response(&response);

        // AI might miss some hashes if the list is long.
        // FORCE the plan to include ALL hashes from input, using the AI's message.
        // We trust AI for the message, but we enforce the hash list to ensure no data loss.

        let mut plan: SquashPlan =
            serde_json::from_str(&json_str).context("Failed to parse AI Lazy Plan")?;

        if let Some(group) = plan.groups.first_mut() {
            // Overwrite commits with ALL input hashes to be safe
            group.commits = commits.iter().map(|c| c.hash.clone()).collect();
        } else {
            // If AI returned empty groups?!
            plan.groups.push(crate::ai_service::SquashGroup {
                target_message: "feat!: major update".to_string(),
                commits: commits.iter().map(|c| c.hash.clone()).collect(),
            });
        }

        // Ensure only 1 group
        plan.groups.truncate(1);

        Ok(plan)
    }

    fn clean_json_response(&self, raw: &str) -> String {
        let mut text = raw.trim();
        if text.starts_with("```json") {
            text = text.trim_start_matches("```json");
        } else if text.starts_with("```") {
            text = text.trim_start_matches("```");
        }
        if text.ends_with("```") {
            text = text.trim_end_matches("```");
        }
        text.trim().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquashPlan {
    pub groups: Vec<SquashGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquashGroup {
    pub target_message: String,
    pub commits: Vec<String>, // Hashes
}
