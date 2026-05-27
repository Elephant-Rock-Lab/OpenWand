use serde::{Deserialize, Serialize};

/// A tool discovered from an MCP server.
/// This is the pool-owned DTO — never `ToolDef` (which lives in openwand-tools).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDiscoveredTool {
    pub server_name: String,
    pub remote_name: String,
    pub title: Option<String>,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub annotations: Option<McpToolAnnotations>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpToolAnnotations {
    pub read_only_hint: Option<bool>,
    pub destructive_hint: Option<bool>,
    pub idempotent_hint: Option<bool>,
    pub open_world_hint: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dto_roundtrip_mcp_discovered_tool() {
        let tool = McpDiscoveredTool {
            server_name: "filesystem".into(),
            remote_name: "read_file".into(),
            title: Some("Read File".into()),
            description: "Read a file from disk".into(),
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: None,
            annotations: Some(McpToolAnnotations {
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: None,
                open_world_hint: None,
            }),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let restored: McpDiscoveredTool = serde_json::from_str(&json).unwrap();
        assert_eq!(tool.server_name, restored.server_name);
        assert_eq!(tool.remote_name, restored.remote_name);
        assert_eq!(Some(true), restored.annotations.unwrap().read_only_hint);
    }
}
