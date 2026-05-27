use crate::{McpDiscoveredTool, McpPoolError, McpToolResult};
use async_trait::async_trait;

/// Gateway trait for MCP server interaction.
/// Implemented by `McpServerPool`. Consumed by `openwand-tools`.
#[async_trait]
pub trait McpToolGateway: Send + Sync {
    /// Ensure a specific MCP server is started and connected.
    async fn ensure_started(&self, server_name: &str) -> Result<(), McpPoolError>;

    /// Discover all tools from all configured and enabled MCP servers.
    async fn discover_all_tools(&self) -> Result<Vec<McpDiscoveredTool>, McpPoolError>;

    /// Execute a tool call on a specific MCP server.
    async fn execute_tool(
        &self,
        server_name: &str,
        remote_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpPoolError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prove McpToolGateway is object-safe.
    #[test]
    fn mcp_tool_gateway_trait_object_compiles() {
        fn _uses_arc_dyn(_gw: std::sync::Arc<dyn McpToolGateway>) {}
    }
}
