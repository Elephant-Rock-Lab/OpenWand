use openwand_core::tool_vocab::ToolEffect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransportConfig,
    pub default_effect: Option<ToolEffect>,
    #[serde(default)]
    pub tool_effects: HashMap<String, ToolEffect>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransportConfig {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        cwd: Option<PathBuf>,
    },

    #[cfg(feature = "http")]
    StreamableHttp {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dto_roundtrip_mcp_server_config() {
        let config = McpServerConfig {
            name: "echo".into(),
            transport: McpTransportConfig::Stdio {
                command: "echo-server".into(),
                args: vec!["--port".into(), "8080".into()],
                env: HashMap::new(),
                cwd: None,
            },
            default_effect: Some(ToolEffect::Read),
            tool_effects: {
                let mut m = HashMap::new();
                m.insert("dangerous_tool".into(), ToolEffect::Delete);
                m
            },
            enabled: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: McpServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.name, restored.name);
        assert!(restored.enabled);
        assert_eq!(Some(ToolEffect::Read), restored.default_effect);
    }
}
