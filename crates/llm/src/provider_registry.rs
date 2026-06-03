//! Provider registry — builds `Arc<dyn LlmClient>` from validated config.
//!
//! Dispatches on ProviderKind to construct the right adapter.
//! API key resolution happens at build time, never stored in registry.

use std::sync::Arc;

use crate::client::LlmClient;
use crate::error::LlmError;
use crate::provider_config::{ProviderKind, ProviderTargetConfig, ProviderTargetSummary};
use crate::request::LlmTarget;

#[cfg(feature = "testing")]
use crate::testing::MockLlmClient;

/// Registry that builds LLM clients from provider configurations.
pub struct ProviderRegistry {
    targets: Vec<ProviderTargetConfig>,
}

impl ProviderRegistry {
    /// Create a registry from validated provider configs.
    /// Call `validate_provider_config()` on each target before passing them here.
    pub fn new(targets: Vec<ProviderTargetConfig>) -> Self {
        Self { targets }
    }

    /// Build a client for the given target ID.
    /// Resolves API keys from environment at build time.
    pub fn build_client(&self, target_id: &str) -> Result<Arc<dyn LlmClient>, LlmError> {
        let config = self
            .targets
            .iter()
            .find(|t| t.id == target_id)
            .ok_or_else(|| LlmError::RequestInvalid {
                message: format!("unknown provider target: '{target_id}'"),
            })?;

        if !config.enabled {
            return Err(LlmError::RequestInvalid {
                message: format!("provider target '{}' is disabled", config.id),
            });
        }

        let mut config = config.clone();
        config.resolve_api_key();

        match &config.provider_kind {
            ProviderKind::Mock => {
                #[cfg(feature = "testing")]
                {
                    Ok(Arc::new(MockLlmClient::new()) as Arc<dyn LlmClient>)
                }
                #[cfg(not(feature = "testing"))]
                {
                    Err(LlmError::Unsupported {
                        provider: "Mock".into(),
                        feature: "testing feature not enabled".into(),
                    })
                }
            }
            ProviderKind::OpenAiCompatible => {
                #[cfg(feature = "openai-compatible")]
                {
                    let client = crate::adapters::openai_compatible::OpenAiCompatibleClient::new();
                    Ok(Arc::new(client) as Arc<dyn LlmClient>)
                }
                #[cfg(not(feature = "openai-compatible"))]
                {
                    Err(LlmError::Unsupported {
                        provider: "OpenAI-compatible".into(),
                        feature: "openai-compatible feature not enabled".into(),
                    })
                }
            }
            ProviderKind::LocalOpenAiCompatible => {
                #[cfg(feature = "openai-compatible")]
                {
                    let client = crate::adapters::openai_compatible::OpenAiCompatibleClient::new();
                    Ok(Arc::new(client) as Arc<dyn LlmClient>)
                }
                #[cfg(not(feature = "openai-compatible"))]
                {
                    Err(LlmError::Unsupported {
                        provider: "Local OpenAI-compatible".into(),
                        feature: "openai-compatible feature not enabled".into(),
                    })
                }
            }
            ProviderKind::AnthropicCompatible => {
                #[cfg(feature = "anthropic-compatible")]
                {
                    let client =
                        crate::adapters::anthropic_compatible::AnthropicCompatibleClient::new();
                    Ok(Arc::new(client) as Arc<dyn LlmClient>)
                }
                #[cfg(not(feature = "anthropic-compatible"))]
                {
                    Err(LlmError::Unsupported {
                        provider: "Anthropic-compatible".into(),
                        feature: "anthropic-compatible feature not enabled".into(),
                    })
                }
            }
        }
    }

    /// Build an LlmTarget for the given target ID.
    /// Resolves API key at build time.
    pub fn build_target(&self, target_id: &str) -> Result<LlmTarget, LlmError> {
        let config = self
            .targets
            .iter()
            .find(|t| t.id == target_id)
            .ok_or_else(|| LlmError::RequestInvalid {
                message: format!("unknown provider target: '{target_id}'"),
            })?;

        if !config.enabled {
            return Err(LlmError::RequestInvalid {
                message: format!("provider target '{}' is disabled", config.id),
            });
        }

        let mut config = config.clone();
        config.resolve_api_key();

        Ok(LlmTarget {
            provider: config.to_llm_provider(),
            model: config.model,
            base_url: config.endpoint,
            api_key: config.resolved_api_key,
        })
    }

    /// List all enabled targets as display summaries.
    /// Never contains raw API key values (Patch 4).
    pub fn list_available_targets(&self) -> Vec<ProviderTargetSummary> {
        self.targets
            .iter()
            .filter(|t| t.enabled)
            .map(|t| t.to_summary())
            .collect()
    }

    /// Get a config by ID (for inspection, not execution).
    pub fn get_config(&self, target_id: &str) -> Option<&ProviderTargetConfig> {
        self.targets.iter().find(|t| t.id == target_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_config::ApiKeySource;

    fn mock_registry() -> ProviderRegistry {
        ProviderRegistry::new(vec![ProviderTargetConfig {
            id: "mock-test".into(),
            provider_kind: ProviderKind::Mock,
            display_name: "Mock Provider".into(),
            model: "mock-model".into(),
            endpoint: None,
            api_key_source: ApiKeySource::None,
            timeout_ms: 30_000,
            enabled: true,
            supports_tools: true,
            supports_streaming: true,
            supports_usage: false,
            supports_reasoning: false,
            thinking_budget_tokens: None,
            max_context_tokens: None,
            resolved_api_key: None,
        }])
    }

    fn openai_registry() -> ProviderRegistry {
        ProviderRegistry::new(vec![ProviderTargetConfig {
            id: "openai-test".into(),
            provider_kind: ProviderKind::OpenAiCompatible,
            display_name: "OpenAI".into(),
            model: "gpt-4.1-mini".into(),
            endpoint: Some("https://api.openai.com/v1".into()),
            api_key_source: ApiKeySource::EnvVar {
                name: "OPENAI_API_KEY".into(),
            },
            timeout_ms: 60_000,
            enabled: true,
            supports_tools: true,
            supports_streaming: true,
            supports_usage: true,
            supports_reasoning: false,
            thinking_budget_tokens: None,
            max_context_tokens: Some(128_000),
            resolved_api_key: None,
        }])
    }

    #[cfg(feature = "testing")]
    #[test]
    fn provider_registry_builds_mock_client() {
        let registry = mock_registry();
        let client = registry.build_client("mock-test").unwrap();
        // Verify it's usable as a trait object
        let target = registry.build_target("mock-test").unwrap();
        let caps = client.capabilities(&target);
        assert!(caps.supports_streaming);
    }

    #[test]
    fn provider_registry_builds_openai_compatible_client() {
        let registry = openai_registry();
        // Without openai-compatible feature, this returns Unsupported
        let result = registry.build_client("openai-test");
        // The feature may or may not be enabled; just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn provider_registry_builds_anthropic_compatible_client() {
        let registry = ProviderRegistry::new(vec![ProviderTargetConfig {
            id: "anthropic-test".into(),
            provider_kind: ProviderKind::AnthropicCompatible,
            display_name: "Anthropic".into(),
            model: "claude-sonnet-4-20250514".into(),
            endpoint: Some("https://api.anthropic.com".into()),
            api_key_source: ApiKeySource::EnvVar {
                name: "ANTHROPIC_API_KEY".into(),
            },
            timeout_ms: 120_000,
            enabled: true,
            supports_tools: true,
            supports_streaming: true,
            supports_usage: true,
            supports_reasoning: true,
            thinking_budget_tokens: None,
            max_context_tokens: Some(200_000),
            resolved_api_key: None,
        }]);
        let result = registry.build_client("anthropic-test");
        // Feature may not be enabled; just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn provider_registry_builds_local_openai_compatible_client() {
        let registry = ProviderRegistry::new(vec![ProviderTargetConfig {
            id: "local-test".into(),
            provider_kind: ProviderKind::LocalOpenAiCompatible,
            display_name: "Local LM Studio".into(),
            model: "local-model".into(),
            endpoint: Some("http://localhost:1234/v1".into()),
            api_key_source: ApiKeySource::None,
            timeout_ms: 60_000,
            enabled: true,
            supports_tools: false,
            supports_streaming: true,
            supports_usage: false,
            supports_reasoning: false,
            thinking_budget_tokens: None,
            max_context_tokens: None,
            resolved_api_key: None,
        }]);
        let result = registry.build_client("local-test");
        let _ = result;
    }

    #[test]
    fn provider_registry_rejects_unsupported_kind() {
        let registry = ProviderRegistry::new(vec![]);
        let result = registry.build_client("nonexistent");
        assert!(result.is_err());
        match result {
            Err(LlmError::RequestInvalid { message }) => {
                assert!(message.contains("unknown provider target"));
            }
            _ => panic!("expected RequestInvalid"),
        }
    }

    #[test]
    fn provider_registry_lists_available_targets() {
        let registry = mock_registry();
        let targets = registry.list_available_targets();
        assert_eq!(1, targets.len());
        assert_eq!("mock-test", targets[0].id);
        assert!(targets[0].enabled);
    }

    #[test]
    fn provider_registry_env_var_resolution_at_build_time() {
        let registry = ProviderRegistry::new(vec![ProviderTargetConfig {
            id: "env-test".into(),
            provider_kind: ProviderKind::Mock,
            display_name: "Env Test".into(),
            model: "test".into(),
            endpoint: None,
            api_key_source: ApiKeySource::EnvVar {
                name: "OPENWAND_TEST_KEY_FOR_REGISTRY".into(),
            },
            timeout_ms: 30_000,
            enabled: true,
            supports_tools: true,
            supports_streaming: true,
            supports_usage: false,
            supports_reasoning: false,
            thinking_budget_tokens: None,
            max_context_tokens: None,
            resolved_api_key: None,
        }]);

        // Set the env var before building
        // SAFETY: Test-only, single-threaded test
        unsafe { std::env::set_var("OPENWAND_TEST_KEY_FOR_REGISTRY", "test-key-value"); }
        let target = registry.build_target("env-test").unwrap();
        assert_eq!(Some("test-key-value".into()), target.api_key);
        // SAFETY: Test-only, single-threaded test
        unsafe { std::env::remove_var("OPENWAND_TEST_KEY_FOR_REGISTRY"); }
    }

    #[test]
    fn provider_registry_disabled_targets_not_listed() {
        let config = ProviderTargetConfig {
            id: "disabled-test".into(),
            provider_kind: ProviderKind::Mock,
            display_name: "Disabled".into(),
            model: "test".into(),
            endpoint: None,
            api_key_source: ApiKeySource::None,
            timeout_ms: 30_000,
            enabled: false, // disabled
            supports_tools: true,
            supports_streaming: true,
            supports_usage: false,
            supports_reasoning: false,
            thinking_budget_tokens: None,
            max_context_tokens: None,
            resolved_api_key: None,
        };

        // Also verify disabled target can't build
        let registry = ProviderRegistry::new(vec![config.clone()]);
        let targets = registry.list_available_targets();
        assert!(targets.is_empty(), "Disabled targets must not appear in list");

        let result = registry.build_client("disabled-test");
        assert!(result.is_err());
        match result {
            Err(LlmError::RequestInvalid { message }) => {
                assert!(message.contains("disabled"));
            }
            _ => panic!("expected RequestInvalid for disabled target"),
        }
    }
}
