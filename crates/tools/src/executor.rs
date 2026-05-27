//! ToolExecutor trait — the session-facing seam.

use crate::result::ToolCallContext;
use crate::{ToolDef, ToolError, ToolResult};
use async_trait::async_trait;
use openwand_core::ToolCallId;

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: ToolCallId,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct ToolRefreshReport {
    pub servers_checked: u32,
    pub tools_added: u32,
    pub tools_removed: u32,
    pub errors: Vec<String>,
}

/// The unified tool executor seam.
/// Session calls this. Never knows about local vs MCP.
///
/// `execute()` is infallible — always returns `ToolResult`, never `Err`.
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    fn available_tools(&self) -> Vec<ToolDef>;

    fn get_descriptor(&self, name: &str) -> Option<ToolDef>;

    async fn execute(&self, call: &ToolCall, context: &ToolCallContext) -> ToolResult;

    async fn refresh_mcp_tools(&self) -> Result<ToolRefreshReport, ToolError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prove ToolExecutor is object-safe.
    #[test]
    fn tool_executor_trait_object_compiles() {
        fn _uses_arc_dyn(_exec: std::sync::Arc<dyn ToolExecutor>) {}
    }
}
