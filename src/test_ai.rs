#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_service::{AIConfig, AIProvider, AIService};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_ai_commit_message() {
        // Get API key from environment - skip test if not available
        let api_key = match std::env::var("GEMINI_API_KEY") {
            Ok(key) if !key.is_empty() => key,
            _ => {
                println!("⏭️ Skipping AI test - GEMINI_API_KEY not set");
                return;
            }
        };

        // Create a test configuration with the Gemini API key from env
        let mut provider_models = HashMap::new();
        provider_models.insert(AIProvider::Gemini, "gemini-1.5-flash-latest".to_string());

        let mut api_keys = HashMap::new();
        api_keys.insert(AIProvider::Gemini, api_key);

        let config = AIConfig {
            primary_provider: AIProvider::Gemini,
            backup_providers: vec![AIProvider::OpenRouter, AIProvider::OpenAI],
            provider_models,
            api_keys,
        };

        let ai_service = AIService::new(config);

        // Test with a simple diff
        let diff = "diff --git a/test.txt b/test.txt\nindex 1234567..abcdefg 100644\n--- a/test.txt\n+++ b/test.txt\n@@ -1 +1 @@\n-test content\n+modified content for AI testing";

        let result = ai_service.generate_commit_message(diff).await;

        match result {
            Ok(message) => {
                println!("✅ AI commit message generated: {}", message);
                assert!(!message.is_empty());
            }
            Err(e) => {
                println!("❌ AI commit message failed: {}", e);
                panic!("AI commit message generation failed: {}", e);
            }
        }
    }
}
