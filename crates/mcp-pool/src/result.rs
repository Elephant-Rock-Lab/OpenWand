use serde::{Deserialize, Serialize};

/// Result from an MCP tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub server_name: String,
    pub remote_name: String,
    pub output: String,
    pub is_error: bool,
}
