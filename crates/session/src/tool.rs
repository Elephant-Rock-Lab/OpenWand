use openwand_core::ToolCallId;
use serde::{Deserialize, Serialize};

/// Internal tool call representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: ToolCallId,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Internal tool result representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
    pub output: String,
    pub is_error: bool,
    pub duration_ms: u64,
}
