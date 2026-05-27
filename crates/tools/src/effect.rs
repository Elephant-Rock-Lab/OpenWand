//! MCP effect resolution.
//!
//! Precedence: config tool override → server default → annotation hints → Unknown.

use crate::descriptor::ToolAnnotations;
use crate::descriptor::ToolSource;
use openwand_core::tool_vocab::ToolEffect;
use openwand_mcp_pool::McpServerConfig;

/// Resolve the declared effect for an MCP-backed tool.
/// Annotations are hints, not authority.
pub fn resolve_mcp_effect(
    source: &ToolSource,
    annotations: Option<&ToolAnnotations>,
    config: &McpServerConfig,
) -> ToolEffect {
    let ToolSource::Mcp {
        server: _,
        remote_name,
    } = source
    else {
        return ToolEffect::Unknown;
    };

    // 1. Per-tool override in config
    if let Some(effect) = config.tool_effects.get(remote_name) {
        return effect.clone();
    }

    // 2. Server-wide default
    if let Some(effect) = &config.default_effect {
        return effect.clone();
    }

    // 3. Annotation hints
    if let Some(ann) = annotations {
        if ann.read_only_hint.unwrap_or(false) {
            return ToolEffect::Read;
        }
        if ann.destructive_hint.unwrap_or(false) {
            return ToolEffect::Delete;
        }
        if ann.open_world_hint.unwrap_or(false) {
            return ToolEffect::Network;
        }
    }

    // 4. Unknown
    ToolEffect::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mcp_source(remote: &str) -> ToolSource {
        ToolSource::Mcp {
            server: "test".into(),
            remote_name: remote.into(),
        }
    }

    #[test]
    fn effect_resolution_mcp_tool_override_wins() {
        let source = mcp_source("danger");
        let config = McpServerConfig {
            name: "test".into(),
            transport: openwand_mcp_pool::McpTransportConfig::Stdio {
                command: "test".into(),
                args: vec![],
                env: HashMap::new(),
                cwd: None,
            },
            default_effect: Some(ToolEffect::Read),
            tool_effects: {
                let mut m = HashMap::new();
                m.insert("danger".into(), ToolEffect::Delete);
                m
            },
            enabled: true,
        };
        let result = resolve_mcp_effect(&source, None, &config);
        assert_eq!(ToolEffect::Delete, result);
    }

    #[test]
    fn effect_resolution_mcp_server_default_wins() {
        let source = mcp_source("unknown_tool");
        let config = McpServerConfig {
            name: "test".into(),
            transport: openwand_mcp_pool::McpTransportConfig::Stdio {
                command: "test".into(),
                args: vec![],
                env: HashMap::new(),
                cwd: None,
            },
            default_effect: Some(ToolEffect::Read),
            tool_effects: HashMap::new(),
            enabled: true,
        };
        let result = resolve_mcp_effect(&source, None, &config);
        assert_eq!(ToolEffect::Read, result);
    }

    #[test]
    fn effect_resolution_mcp_annotations_are_hints() {
        let source = mcp_source("tool");
        let config = McpServerConfig {
            name: "test".into(),
            transport: openwand_mcp_pool::McpTransportConfig::Stdio {
                command: "test".into(),
                args: vec![],
                env: HashMap::new(),
                cwd: None,
            },
            default_effect: None,
            tool_effects: HashMap::new(),
            enabled: true,
        };
        let ann = ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: None,
            idempotent_hint: None,
            open_world_hint: None,
        };
        let result = resolve_mcp_effect(&source, Some(&ann), &config);
        assert_eq!(ToolEffect::Read, result);
    }

    #[test]
    fn effect_resolution_unknown_when_no_signal() {
        let source = mcp_source("mystery");
        let config = McpServerConfig {
            name: "test".into(),
            transport: openwand_mcp_pool::McpTransportConfig::Stdio {
                command: "test".into(),
                args: vec![],
                env: HashMap::new(),
                cwd: None,
            },
            default_effect: None,
            tool_effects: HashMap::new(),
            enabled: true,
        };
        let result = resolve_mcp_effect(&source, None, &config);
        assert_eq!(ToolEffect::Unknown, result);
    }

    #[test]
    fn effect_resolution_local_source_returns_unknown() {
        let source = ToolSource::Local;
        let config = McpServerConfig {
            name: "test".into(),
            transport: openwand_mcp_pool::McpTransportConfig::Stdio {
                command: "test".into(),
                args: vec![],
                env: HashMap::new(),
                cwd: None,
            },
            default_effect: Some(ToolEffect::Read),
            tool_effects: HashMap::new(),
            enabled: true,
        };
        let result = resolve_mcp_effect(&source, None, &config);
        assert_eq!(ToolEffect::Unknown, result);
    }
}
