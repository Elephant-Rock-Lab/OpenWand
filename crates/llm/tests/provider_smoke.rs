//! Feature-gated smoke tests for real LLM providers.
//!
//! These tests are #[ignore] by default and require specific feature flag + env vars.
//! CI never runs these. Manual run:
//!   cargo test -p openwand-llm --features real-provider-smoke -- --ignored

#[cfg(feature = "real-provider-smoke")]
mod real_smoke {
    use openwand_llm::adapters::openai_compatible::OpenAiCompatibleClient;
    use openwand_llm::client::LlmClient;
    use openwand_llm::request::{
        LlmContent, LlmMessage, LlmProvider, LlmRequest, LlmTarget,
    };

    fn skip_without(var: &str) -> Option<String> {
        std::env::var(var).ok()
    }

    #[tokio::test]
    #[ignore]
    async fn real_model_smoke_openai_compatible() {
        let api_key = match skip_without("OPENAI_API_KEY") {
            Some(k) => k,
            None => {
                eprintln!("Skipping: OPENAI_API_KEY not set");
                return;
            }
        };

        let client = OpenAiCompatibleClient::new();
        let target = LlmTarget {
            provider: LlmProvider::OpenAI,
            model: "gpt-4.1-mini".into(),
            base_url: None,
            api_key: Some(api_key),
        };

        // Health check
        client.health_check(&target).await.expect("Health check failed");

        // Simple completion
        let request = LlmRequest {
            target: target.clone(),
            system_prompt: "Reply with exactly: OK".into(),
            messages: vec![LlmMessage::User {
                content: vec![LlmContent::Text("Say OK".into())],
            }],
            tools: vec![],
            thinking_budget: None,
            max_tokens: Some(10),
            temperature: Some(0.0),
            tool_choice: None,
            provider_options: serde_json::json!({}),
        };

        let response = client.complete(request).await.expect("Completion failed");
        assert!(!response.content.is_empty(), "Response should have content");
    }

    #[tokio::test]
    #[ignore]
    async fn real_model_smoke_anthropic_compatible() {
        let api_key = match skip_without("ANTHROPIC_API_KEY") {
            Some(k) => k,
            None => {
                eprintln!("Skipping: ANTHROPIC_API_KEY not set");
                return;
            }
        };

        #[cfg(feature = "anthropic-compatible")]
        {
            let client =
                openwand_llm::adapters::anthropic_compatible::AnthropicCompatibleClient::new();
            let target = LlmTarget {
                provider: LlmProvider::Anthropic,
                model: "claude-sonnet-4-20250514".into(),
                base_url: None,
                api_key: Some(api_key),
            };

            let request = LlmRequest {
                target: target.clone(),
                system_prompt: "Reply with exactly: OK".into(),
                messages: vec![LlmMessage::User {
                    content: vec![LlmContent::Text("Say OK".into())],
                }],
                tools: vec![],
                thinking_budget: None,
                max_tokens: Some(10),
                temperature: Some(0.0),
                tool_choice: None,
                provider_options: serde_json::json!({}),
            };

            let response = client.complete(request).await.expect("Completion failed");
            assert!(!response.content.is_empty(), "Response should have content");
        }

        #[cfg(not(feature = "anthropic-compatible"))]
        {
            let _ = api_key; // suppress unused warning
            eprintln!("Skipping: anthropic-compatible feature not enabled");
        }
    }

    #[tokio::test]
    #[ignore]
    async fn real_model_smoke_local_openai_compatible() {
        let url = match skip_without("LOCAL_LLM_URL") {
            Some(u) => u,
            None => {
                eprintln!("Skipping: LOCAL_LLM_URL not set");
                return;
            }
        };

        let client = OpenAiCompatibleClient::new();
        let target = LlmTarget {
            provider: LlmProvider::Custom {
                name: "local".into(),
            },
            model: "local-model".into(),
            base_url: Some(url),
            api_key: None,
        };

        let request = LlmRequest {
            target: target.clone(),
            system_prompt: "Reply with exactly: OK".into(),
            messages: vec![LlmMessage::User {
                content: vec![LlmContent::Text("Say OK".into())],
            }],
            tools: vec![],
            thinking_budget: None,
            max_tokens: Some(10),
            temperature: Some(0.0),
            tool_choice: None,
            provider_options: serde_json::json!({}),
        };

        let response = client.complete(request).await.expect("Local completion failed");
        assert!(!response.content.is_empty(), "Response should have content");
    }
}
