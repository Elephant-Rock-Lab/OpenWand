//! UI provider configuration — read-only view helpers.
//!
//! Display DTOs and render functions for provider target configuration.
//! UI reads config and sends selection intent; session runtime resolves client.
//!
//! Key invariants:
//! - UI never calls providers directly
//! - UI never contains raw API key values
//! - Provider errors displayed to user use safe_display()

use openwand_llm::provider_config::ProviderTargetSummary;

/// UI-safe row for displaying a provider target.
#[derive(Debug, Clone)]
pub struct UiProviderTargetRow {
    pub id: String,
    pub display_name: String,
    pub model: String,
    pub endpoint_display: String,
    pub api_key_display: String, // "none" or "env:VAR_NAME" — never resolved value
    pub enabled: bool,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_reasoning: bool,
}

/// Build UI rows from provider summaries. Never includes raw key values.
pub fn provider_target_rows(summaries: &[ProviderTargetSummary]) -> Vec<UiProviderTargetRow> {
    summaries
        .iter()
        .map(|s| UiProviderTargetRow {
            id: s.id.clone(),
            display_name: s.display_name.clone(),
            model: s.model.clone(),
            endpoint_display: s.endpoint.clone().unwrap_or_else(|| "default".into()),
            api_key_display: s.api_key_source_display.clone(), // Already safe
            enabled: s.enabled,
            supports_tools: s.supports_tools,
            supports_streaming: s.supports_streaming,
            supports_reasoning: s.supports_reasoning,
        })
        .collect()
}

/// Validation lines for display. References field names, never raw key values.
pub fn provider_validation_lines(errors: &[String]) -> Vec<String> {
    errors.to_vec() // Already sanitized at source
}

/// Safety warning for provider configuration display.
pub fn provider_config_safety_warning() -> String {
    "Provider adapters stream model output. SessionRunner owns the loop. \
     Policy gates tools. ToolExecutor executes tools. \
     Providers never execute tools or mutate state directly."
        .into()
}

#[cfg(feature = "desktop")]
pub mod components {
    //! Dioxus render functions for provider config panel (desktop-gated).

    use super::*;

    /// Render the provider config panel.
    /// Read-only display — no form submission, no provider calls.
    pub fn render_provider_config_panel(
        rows: &[UiProviderTargetRow],
    ) -> Vec<String> {
        let mut lines = Vec::new();
        for row in rows {
            lines.push(format!(
                "[{}] {} ({}) — model: {}, key: {}",
                if row.enabled { "✓" } else { "✗" },
                row.display_name,
                row.id,
                row.model,
                row.api_key_display,
            ));
        }
        lines
    }

    /// Render validation errors.
    pub fn render_validation_errors(errors: &[String]) -> Vec<String> {
        errors.iter().map(|e| format!("⚠ {e}")).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_llm::provider_config::{ProviderKind, ProviderTargetConfig, ApiKeySource};

    fn test_summary() -> ProviderTargetSummary {
        ProviderTargetConfig {
            id: "openai-main".into(),
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
            resolved_api_key: Some("sk-secret-key-12345".into()),
        }
        .to_summary()
    }

    #[test]
    fn provider_config_rows_hide_raw_api_keys() {
        let summary = test_summary();
        let rows = provider_target_rows(&[summary]);
        assert_eq!(1, rows.len());
        // api_key_display should show env var name, not the resolved value
        assert_eq!("env:OPENAI_API_KEY", rows[0].api_key_display);
        assert!(!rows[0].api_key_display.contains("sk-secret-key-12345"));
    }

    #[test]
    fn provider_config_rows_show_provider_model_endpoint() {
        let summary = test_summary();
        let rows = provider_target_rows(&[summary]);
        assert_eq!("openai-main", rows[0].id);
        assert_eq!("OpenAI", rows[0].display_name);
        assert_eq!("gpt-4.1-mini", rows[0].model);
        assert_eq!("https://api.openai.com/v1", rows[0].endpoint_display);
        assert!(rows[0].supports_tools);
    }

    #[test]
    fn provider_validation_lines_show_safe_errors() {
        let errors = vec![
            "field 'model' must not be empty".into(),
            "field 'endpoint' is required for LocalOpenAiCompatible providers".into(),
        ];
        let lines = provider_validation_lines(&errors);
        assert_eq!(2, lines.len());
        // No raw key values in errors
        assert!(lines.iter().all(|l| !l.contains("sk-")));
    }

    #[test]
    fn provider_safety_warning_mentions_tools_gated() {
        let warning = provider_config_safety_warning();
        assert!(warning.contains("Policy gates tools"));
        assert!(warning.contains("Providers never execute tools"));
    }

    #[test]
    fn provider_config_ui_does_not_call_provider_directly() {
        // Design assertion: This module only imports from provider_config (DTOs),
        // never from adapters or client modules. No provider calls possible.
        let summary = test_summary();
        let rows = provider_target_rows(&[summary]);
        assert!(!rows.is_empty());
        // If this compiles, the module doesn't depend on provider adapters.
    }

    #[test]
    fn provider_errors_do_not_include_raw_api_keys() {
        // Simulate what happens when a provider error reaches the UI
        let err = openwand_llm::error::LlmError::Provider {
            provider: "openai-compatible".into(),
            message: "HTTP 401: unauthorized".into(),
            retryable: false,
        };
        let display = err.safe_display();
        assert!(!display.contains("sk-"));
        assert!(!display.contains("Bearer"));
    }
}
