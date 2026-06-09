//! Provider settings — externalized configuration.
//!
//! Reads from ~/.openwand/settings.toml, falls back to defaults.
//! Wave 50A, FIX-01.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Provider settings loaded from settings.toml.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderSettings {
    /// Base URL for the LLM provider (e.g., "http://localhost:1234/v1").
    pub base_url: Option<String>,
    /// Model name (e.g., "qwen/qwen3-4b-2507").
    pub model: Option<String>,
    /// API key (optional — some local providers don't require one).
    pub api_key: Option<String>,
    /// Provider name (e.g., "lm-studio", "openai").
    pub provider: Option<String>,
}

/// Full settings file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub provider: ProviderSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            provider: ProviderSettings {
                base_url: Some("http://localhost:1234/v1".into()),
                model: Some("qwen/qwen3-4b-2507".into()),
                api_key: Some("lm-studio".into()),
                provider: Some("lm-studio".into()),
            },
        }
    }
}

/// Returns the path to the settings file: ~/.openwand/settings.toml
pub fn settings_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openwand")
        .join("settings.toml")
}

/// Load settings from disk. Returns defaults if file is missing or unparseable.
pub fn load_settings() -> Settings {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            match toml::from_str(&content) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to parse {}: {} — using defaults", path.display(), e);
                    Settings::default()
                }
            }
        }
        Err(_) => {
            // File doesn't exist yet — use defaults
            Settings::default()
        }
    }
}

/// Resolve the base URL: settings > fallback.
pub fn resolve_base_url(settings: &Settings) -> String {
    settings.provider.base_url.clone()
        .unwrap_or_else(|| "http://localhost:1234/v1".into())
}

/// Resolve the model name: settings > fallback.
pub fn resolve_model(settings: &Settings) -> String {
    settings.provider.model.clone()
        .unwrap_or_else(|| "qwen/qwen3-4b-2507".into())
}

/// Resolve the API key: settings > fallback.
pub fn resolve_api_key(settings: &Settings) -> String {
    settings.provider.api_key.clone()
        .unwrap_or_else(|| "lm-studio".into())
}

/// Resolve the provider name: settings > fallback.
pub fn resolve_provider(settings: &Settings) -> String {
    settings.provider.provider.clone()
        .unwrap_or_else(|| "lm-studio".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_have_no_hardcoded_private_ip() {
        let s = Settings::default();
        // Default must use localhost, NOT a private IP like 100.64.0.1
        let url = s.provider.base_url.unwrap();
        assert!(url.contains("localhost"), "Default URL must use localhost, got: {}", url);
        assert!(!url.contains("100.64"), "Default URL must not contain private IP");
    }

    #[test]
    fn resolve_uses_settings_when_present() {
        let mut s = Settings::default();
        s.provider.base_url = Some("http://my-server:8080/v1".into());
        s.provider.model = Some("gpt-4".into());
        assert_eq!("http://my-server:8080/v1", resolve_base_url(&s));
        assert_eq!("gpt-4", resolve_model(&s));
    }

    #[test]
    fn resolve_uses_defaults_when_none() {
        let mut s = Settings::default();
        s.provider.base_url = None;
        s.provider.model = None;
        assert!(resolve_base_url(&s).contains("localhost"));
        assert!(resolve_model(&s).contains("qwen"));
    }

    #[test]
    fn load_settings_missing_file_returns_defaults() {
        // Settings path may not exist — should return defaults without panic
        let s = load_settings();
        assert!(s.provider.base_url.is_some());
    }

    #[test]
    fn settings_path_is_under_home_openwand() {
        let path = settings_path();
        assert!(path.to_string_lossy().contains(".openwand"));
        assert!(path.to_string_lossy().ends_with("settings.toml"));
    }
}
