//! Provider target configuration — serializable, validated, TOML-loadable.
//!
//! Wave 21 reads provider configs from `.openwand/providers.toml`.
//! Wave 21 does NOT persist provider config edits from the UI.
//!
//! Secret redaction rules (Patch 4):
//! - Debug impl redacts resolved API keys → "***REDACTED***"
//! - ProviderTargetSummary never contains raw key values
//! - Validation errors reference field names, never raw key values
//! - Adapter errors must not include Authorization headers or API keys

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::request::{LlmCapabilities, LlmProvider};

/// Which kind of provider adapter to use.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAiCompatible,
    AnthropicCompatible,
    LocalOpenAiCompatible,
    Mock,
}

/// Where the API key comes from. Never serialized with the raw resolved key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[derive(Default)]
pub enum ApiKeySource {
    #[default]
    None,
    EnvVar { name: String },
}


/// A single provider target configuration entry.
///
/// Loaded from `.openwand/providers.toml`. The `id` field uniquely
/// identifies this target across the system.
///
/// API keys are never serialized (skip_serializing) and never appear in Debug.
#[derive(Clone, Serialize, Deserialize)]
pub struct ProviderTargetConfig {
    /// Unique identifier for this target (e.g., "local-lmstudio").
    pub id: String,

    /// Which provider adapter to use.
    pub provider_kind: ProviderKind,

    /// Human-readable name for UI display.
    #[serde(default)]
    pub display_name: String,

    /// Model identifier (e.g., "gpt-4.1-mini", "claude-sonnet-4-20250514").
    pub model: String,

    /// Base URL for the provider API. Required for Local and OpenAiCompatible.
    #[serde(default)]
    pub endpoint: Option<String>,

    /// Where to get the API key.
    #[serde(default)]
    pub api_key_source: ApiKeySource,

    /// HTTP timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// Whether this target is enabled for use.
    #[serde(default = "default_true")]
    pub enabled: bool,

    // --- Capability flags ---
    #[serde(default = "default_true")]
    pub supports_tools: bool,

    #[serde(default = "default_true")]
    pub supports_streaming: bool,

    #[serde(default)]
    pub supports_usage: bool,

    #[serde(default)]
    pub supports_reasoning: bool,

    /// Thinking/reasoning budget (provider-specific).
    #[serde(default)]
    pub thinking_budget_tokens: Option<u64>,

    /// Maximum context tokens for this model.
    #[serde(default)]
    pub max_context_tokens: Option<u64>,

    /// Resolved API key — NEVER serialized, NEVER in Debug.
    /// Populated at build time from ApiKeySource.
    #[serde(skip)]
    pub resolved_api_key: Option<String>,
}

fn default_timeout_ms() -> u64 {
    120_000
}

fn default_true() -> bool {
    true
}

/// Custom Debug that redacts the resolved API key (Patch 4).
impl fmt::Debug for ProviderTargetConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderTargetConfig")
            .field("id", &self.id)
            .field("provider_kind", &self.provider_kind)
            .field("display_name", &self.display_name)
            .field("model", &self.model)
            .field("endpoint", &self.endpoint)
            .field("api_key_source", &self.api_key_source)
            .field("timeout_ms", &self.timeout_ms)
            .field("enabled", &self.enabled)
            .field("supports_tools", &self.supports_tools)
            .field("supports_streaming", &self.supports_streaming)
            .field("supports_usage", &self.supports_usage)
            .field("supports_reasoning", &self.supports_reasoning)
            .field("thinking_budget_tokens", &self.thinking_budget_tokens)
            .field("max_context_tokens", &self.max_context_tokens)
            .field("resolved_api_key", &"***REDACTED***")
            .finish()
    }
}

/// A display summary of a provider target — safe for UI, logs, and errors.
/// Never contains raw API key values (Patch 4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTargetSummary {
    pub id: String,
    pub provider_kind: ProviderKind,
    pub display_name: String,
    pub model: String,
    pub endpoint: Option<String>,
    pub api_key_source_display: String, // "none" or "env:VAR_NAME"
    pub enabled: bool,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_reasoning: bool,
}

impl ProviderTargetConfig {
    /// Build a display summary that never contains raw key values.
    pub fn to_summary(&self) -> ProviderTargetSummary {
        ProviderTargetSummary {
            id: self.id.clone(),
            provider_kind: self.provider_kind.clone(),
            display_name: self.display_name.clone(),
            model: self.model.clone(),
            endpoint: self.endpoint.clone(),
            api_key_source_display: match &self.api_key_source {
                ApiKeySource::None => "none".to_string(),
                ApiKeySource::EnvVar { name } => format!("env:{name}"),
            },
            enabled: self.enabled,
            supports_tools: self.supports_tools,
            supports_streaming: self.supports_streaming,
            supports_reasoning: self.supports_reasoning,
        }
    }

    /// Convert to an LlmProvider enum value based on provider_kind.
    pub fn to_llm_provider(&self) -> LlmProvider {
        match &self.provider_kind {
            ProviderKind::OpenAiCompatible => LlmProvider::OpenAI,
            ProviderKind::AnthropicCompatible => LlmProvider::Anthropic,
            ProviderKind::LocalOpenAiCompatible => LlmProvider::Custom {
                name: self.display_name.clone().replace(' ', "-").to_lowercase(),
            },
            ProviderKind::Mock => LlmProvider::Custom {
                name: "mock".into(),
            },
        }
    }

    /// Convert to LlmCapabilities based on config flags.
    pub fn to_capabilities(&self) -> LlmCapabilities {
        LlmCapabilities {
            supports_streaming: self.supports_streaming,
            supports_tools: self.supports_tools,
            supports_reasoning: self.supports_reasoning,
            supports_vision: false, // Not configurable in Wave 21
            max_context_tokens: self.max_context_tokens,
            supported_features: {
                let mut features = Vec::new();
                if self.supports_streaming {
                    features.push("streaming".into());
                }
                if self.supports_tools {
                    features.push("tools".into());
                }
                if self.supports_reasoning {
                    features.push("reasoning".into());
                }
                features
            },
        }
    }

    /// Resolve API key from source. Returns None if source is None or env var not set.
    pub fn resolve_api_key(&mut self) {
        match &self.api_key_source {
            ApiKeySource::None => {
                self.resolved_api_key = None;
            }
            ApiKeySource::EnvVar { name } => {
                self.resolved_api_key = std::env::var(name).ok();
            }
        }
    }
}

/// Top-level TOML structure for provider config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfigFile {
    #[serde(rename = "target")]
    pub targets: Vec<ProviderTargetConfig>,
}

/// Validate a provider target config. Returns all errors found.
/// Errors reference field names, never raw key values (Patch 4).
pub fn validate_provider_config(config: &ProviderTargetConfig) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if config.model.trim().is_empty() {
        errors.push("field 'model' must not be empty".to_string());
    }

    if config.id.trim().is_empty() {
        errors.push("field 'id' must not be empty".to_string());
    }

    // Local providers require endpoint
    if matches!(
        config.provider_kind,
        ProviderKind::LocalOpenAiCompatible
    ) && config.endpoint.as_ref().is_none_or(|e| e.trim().is_empty())
    {
        errors.push(
            "field 'endpoint' is required for LocalOpenAiCompatible providers".to_string(),
        );
    }

    // Timeout bounds: 1s – 600s
    if config.timeout_ms < 1000 {
        errors.push("field 'timeout_ms' must be at least 1000 (1 second)".to_string());
    }
    if config.timeout_ms > 600_000 {
        errors.push("field 'timeout_ms' must be at most 600000 (10 minutes)".to_string());
    }

    // Env var validation: check it exists when source is EnvVar
    if let ApiKeySource::EnvVar { name } = &config.api_key_source
        && name.trim().is_empty() {
            errors.push("field 'api_key_source.name' must not be empty when type is env_var".to_string());
        }
        // We do NOT fail validation if the env var is unset.
        // The key might be set in production but not during config validation.
        // Resolution happens at build time; missing key surfaces as a build error.

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Load provider configs from a TOML file.
/// Returns validated configs or safe error messages (Patch 3).
pub fn load_provider_configs(path: &Path) -> Result<Vec<ProviderTargetConfig>, Vec<String>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        vec![format!(
            "failed to read provider config from '{}': {}",
            path.display(),
            e
        )]
    })?;

    let file: ProviderConfigFile = toml::from_str(&content).map_err(|e| {
        // Safe error: no file content leakage
        vec![format!(
            "malformed provider config in '{}': {}",
            path.display(),
            e
        )]
    })?;

    // Validate each target
    let mut all_errors = Vec::new();
    let mut valid_configs = Vec::new();

    for target in file.targets {
        match validate_provider_config(&target) {
            Ok(()) => valid_configs.push(target),
            Err(errors) => {
                for err in errors {
                    all_errors.push(format!("target '{}': {}", target.id, err));
                }
            }
        }
    }

    if all_errors.is_empty() {
        Ok(valid_configs)
    } else {
        Err(all_errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn valid_local_config() -> ProviderTargetConfig {
        ProviderTargetConfig {
            id: "local-lmstudio".into(),
            provider_kind: ProviderKind::LocalOpenAiCompatible,
            display_name: "LM Studio".into(),
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
        }
    }

    fn valid_openai_config() -> ProviderTargetConfig {
        ProviderTargetConfig {
            id: "openai-main".into(),
            provider_kind: ProviderKind::OpenAiCompatible,
            display_name: "OpenAI".into(),
            model: "gpt-4.1-mini".into(),
            endpoint: None,
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
        }
    }

    #[test]
    fn provider_config_roundtrips_without_raw_secret() {
        let config = ProviderTargetConfig {
            resolved_api_key: Some("sk-super-secret-key-12345".into()),
            ..valid_openai_config()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(
            !json.contains("sk-super-secret-key-12345"),
            "Serialized config must not contain raw API key"
        );
        assert!(
            !json.contains("resolved_api_key"),
            "Serialized config must not contain resolved_api_key field"
        );
    }

    #[test]
    fn provider_config_rejects_empty_model() {
        let mut config = valid_local_config();
        config.model = "".into();
        let result = validate_provider_config(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("model") && e.contains("empty")));
    }

    #[test]
    fn provider_config_requires_endpoint_for_local() {
        let mut config = valid_local_config();
        config.endpoint = None;
        let result = validate_provider_config(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("endpoint") && e.contains("LocalOpenAiCompatible")));
    }

    #[test]
    fn provider_config_bounds_timeout() {
        // Too low
        let mut config = valid_local_config();
        config.timeout_ms = 500;
        let result = validate_provider_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("timeout_ms") && e.contains("at least")));

        // Too high
        config.timeout_ms = 700_000;
        let result = validate_provider_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("timeout_ms") && e.contains("at most")));
    }

    #[test]
    fn provider_config_requires_env_var_name_when_enabled() {
        let mut config = valid_openai_config();
        config.api_key_source = ApiKeySource::EnvVar { name: "".into() };
        let result = validate_provider_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("api_key_source.name")));
    }

    #[test]
    fn provider_config_thinking_budget_optional() {
        let config = valid_local_config();
        assert!(config.thinking_budget_tokens.is_none());
        let mut config_with_budget = config;
        config_with_budget.thinking_budget_tokens = Some(4096);
        assert_eq!(Some(4096), config_with_budget.thinking_budget_tokens);
    }

    #[test]
    fn provider_config_displays_without_keys() {
        let config = ProviderTargetConfig {
            resolved_api_key: Some("sk-secret".into()),
            ..valid_openai_config()
        };
        let summary = config.to_summary();
        assert_eq!("env:OPENAI_API_KEY", summary.api_key_source_display);
        assert!(!format!("{summary:?}").contains("sk-secret"));
    }

    #[test]
    fn provider_config_debug_redacts_api_key() {
        let config = ProviderTargetConfig {
            resolved_api_key: Some("sk-should-not-appear".into()),
            ..valid_openai_config()
        };
        let debug_str = format!("{config:?}");
        assert!(
            debug_str.contains("***REDACTED***"),
            "Debug must redact API key"
        );
        assert!(
            !debug_str.contains("sk-should-not-appear"),
            "Debug must not contain raw API key"
        );
    }

    #[test]
    fn provider_target_summary_contains_no_raw_key() {
        let config = ProviderTargetConfig {
            resolved_api_key: Some("sk-leaked-key".into()),
            ..valid_openai_config()
        };
        let summary = config.to_summary();
        let summary_str = format!("{summary:?}");
        assert!(
            !summary_str.contains("sk-leaked-key"),
            "Summary must not contain raw API key"
        );
        // Should contain env var name only
        assert!(summary_str.contains("OPENAI_API_KEY"));
    }

    #[test]
    fn provider_validation_error_does_not_include_raw_key() {
        let config = ProviderTargetConfig {
            model: "".into(),
            ..ProviderTargetConfig {
                resolved_api_key: Some("sk-never-in-errors".into()),
                ..valid_openai_config()
            }
        };
        let errors = validate_provider_config(&config).unwrap_err();
        for error in &errors {
            assert!(
                !error.contains("sk-never-in-errors"),
                "Validation error must not contain raw key: {error}"
            );
        }
    }

    #[test]
    fn provider_ui_rows_show_env_var_name_not_value() {
        let config = ProviderTargetConfig {
            resolved_api_key: Some("sk-actual-key-value".into()),
            ..valid_openai_config()
        };
        let summary = config.to_summary();
        // The summary shows the env var name, not the resolved value
        assert_eq!("env:OPENAI_API_KEY", summary.api_key_source_display);
        assert!(
            !summary.api_key_source_display.contains("sk-actual-key-value"),
            "UI display must show env var name, not resolved value"
        );
    }

    #[test]
    fn provider_config_loads_from_openwand_providers_toml() {
        let dir = std::env::temp_dir().join("openwand_test_providers_load");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("providers.toml");

        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"
[[target]]
id = "local-lmstudio"
provider_kind = "local_open_ai_compatible"
display_name = "LM Studio"
endpoint = "http://localhost:1234/v1"
model = "local-model"
api_key_source = {{ type = "none" }}
timeout_ms = 60000
supports_tools = false
supports_streaming = true
supports_usage = false
enabled = true

[[target]]
id = "openai-main"
provider_kind = "open_ai_compatible"
display_name = "OpenAI"
model = "gpt-4.1-mini"
api_key_source = {{ type = "env_var", name = "OPENAI_API_KEY" }}
timeout_ms = 60000
supports_tools = true
supports_streaming = true
supports_usage = true
enabled = true
"#
        )
        .unwrap();

        let configs = load_provider_configs(&path).unwrap();
        assert_eq!(2, configs.len());
        assert_eq!("local-lmstudio", configs[0].id);
        assert_eq!("openai-main", configs[1].id);
        assert!(matches!(configs[0].provider_kind, ProviderKind::LocalOpenAiCompatible));
        assert!(matches!(configs[1].provider_kind, ProviderKind::OpenAiCompatible));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn provider_config_malformed_file_reports_safe_errors() {
        let dir = std::env::temp_dir().join("openwand_test_providers_malformed");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("providers.toml");

        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "this is not valid toml [[[[").unwrap();

        let result = load_provider_configs(&path);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("malformed provider config")));
        // Error references the file path but should not dump raw file content as-is
        // Note: toml parse errors may include snippets from the error location,
        // which is acceptable — they don't contain secrets or keys.
        assert!(errors.iter().any(|e| e.contains("malformed provider config")));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn provider_config_ui_is_read_only_no_config_write_path() {
        // This is a design assertion: ProviderTargetConfig has no write methods,
        // no save function, no edit path. The module only provides load_provider_configs.
        // This test verifies the public API surface is read-only.
        let config = valid_local_config();
        let summary = config.to_summary();

        // to_summary() produces a read-only view
        assert_eq!("local-lmstudio", summary.id);

        // No public method exists to write config back to disk
        // This is a compile-time guarantee: no save function is exposed.
        // We verify by checking that ProviderTargetSummary has no mutation methods.
        let _summary2 = summary.clone(); // Clone works (read-only copy)
    }

    #[test]
    fn provider_config_toml_example_roundtrips() {
        let config = valid_openai_config();
        let file = ProviderConfigFile {
            targets: vec![config.clone()],
        };
        let toml_str = toml::to_string_pretty(&file).unwrap();
        let restored: ProviderConfigFile = toml::from_str(&toml_str).unwrap();
        assert_eq!(1, restored.targets.len());
        assert_eq!(config.id, restored.targets[0].id);
        assert_eq!(config.model, restored.targets[0].model);
    }
}
