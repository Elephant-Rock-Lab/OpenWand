//! Tool descriptor and source types.

use openwand_core::tool_vocab::ToolEffect;
use openwand_mcp_pool::{McpDiscoveredTool, McpServerConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub display_name: Option<String>,
    pub description: String,
    pub parameters_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub source: ToolSource,
    pub declared_effect: ToolEffect,
    pub risk_hints: Vec<String>,
    pub tags: Vec<String>,
    pub annotations: Option<ToolAnnotations>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnnotations {
    pub read_only_hint: Option<bool>,
    pub destructive_hint: Option<bool>,
    pub idempotent_hint: Option<bool>,
    pub open_world_hint: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolSource {
    Local,
    Mcp {
        server: String,
        remote_name: String,
    },
}

impl ToolDef {
    /// Convert an MCP discovered tool into a ToolDef using config for effect resolution.
    pub fn from_mcp_discovered(tool: McpDiscoveredTool, config: &McpServerConfig) -> Self {
        let canonical_name =
            crate::naming::canonical_mcp_tool_name(&config.name, &tool.remote_name);
        let remote_name = tool.remote_name.clone();

        let annotations = tool.annotations.map(|a| ToolAnnotations {
            read_only_hint: a.read_only_hint,
            destructive_hint: a.destructive_hint,
            idempotent_hint: a.idempotent_hint,
            open_world_hint: a.open_world_hint,
        });

        let source = ToolSource::Mcp {
            server: config.name.clone(),
            remote_name,
        };

        let declared_effect =
            crate::effect::resolve_mcp_effect(&source, annotations.as_ref(), config);

        Self {
            name: canonical_name,
            display_name: tool.title,
            description: tool.description,
            parameters_schema: tool.input_schema,
            output_schema: tool.output_schema,
            source,
            declared_effect,
            risk_hints: vec![],
            tags: vec![format!("mcp:{}", config.name)],
            annotations,
        }
    }
}
