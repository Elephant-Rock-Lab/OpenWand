//! Mock MCP gateway for testing.
//! Simulates MCP server behavior without requiring real processes.

use crate::{McpDiscoveredTool, McpPoolError, McpToolGateway, McpToolResult, McpToolAnnotations};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A mock MCP tool definition.
pub struct MockTool {
    pub name: String,
    pub description: String,
    pub annotations: Option<McpToolAnnotations>,
    pub handler: Arc<dyn Fn(serde_json::Value) -> Result<String, String> + Send + Sync>,
}

impl std::fmt::Debug for MockTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockTool")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// A mock MCP server.
pub struct MockMcpServer {
    pub name: String,
    pub tools: Vec<MockTool>,
}

/// Mock MCP gateway for integration testing.
pub struct MockMcpGateway {
    servers: HashMap<String, MockMcpServer>,
    started: RwLock<bool>,
}

impl MockMcpGateway {
    pub fn new(servers: Vec<MockMcpServer>) -> Self {
        let map: HashMap<String, MockMcpServer> = servers
            .into_iter()
            .map(|s| (s.name.clone(), s))
            .collect();
        Self {
            servers: map,
            started: RwLock::new(false),
        }
    }

    pub fn empty() -> Self {
        Self {
            servers: HashMap::new(),
            started: RwLock::new(false),
        }
    }
}

#[async_trait]
impl McpToolGateway for MockMcpGateway {
    async fn ensure_started(&self, _server_name: &str) -> Result<(), McpPoolError> {
        let mut started = self.started.write().await;
        *started = true;
        Ok(())
    }

    async fn discover_all_tools(&self) -> Result<Vec<McpDiscoveredTool>, McpPoolError> {
        let mut tools = Vec::new();
        for (name, server) in &self.servers {
            for tool in &server.tools {
                tools.push(McpDiscoveredTool {
                    server_name: name.clone(),
                    remote_name: tool.name.clone(),
                    title: None,
                    description: tool.description.clone(),
                    input_schema: serde_json::json!({"type": "object"}),
                    output_schema: None,
                    annotations: tool.annotations.clone(),
                });
            }
        }
        Ok(tools)
    }

    async fn execute_tool(
        &self,
        server_name: &str,
        remote_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpPoolError> {
        let server = self.servers.get(server_name).ok_or_else(|| {
            McpPoolError::CallFailed {
                server: server_name.to_string(),
                tool: remote_name.to_string(),
                reason: "Server not found".into(),
            }
        })?;

        let tool = server
            .tools
            .iter()
            .find(|t| t.name == remote_name)
            .ok_or_else(|| McpPoolError::CallFailed {
                server: server_name.to_string(),
                tool: remote_name.to_string(),
                reason: "Tool not found".into(),
            })?;

        match (tool.handler)(arguments) {
            Ok(output) => Ok(McpToolResult {
                server_name: server_name.to_string(),
                remote_name: remote_name.to_string(),
                output,
                is_error: false,
            }),
            Err(e) => Ok(McpToolResult {
                server_name: server_name.to_string(),
                remote_name: remote_name.to_string(),
                output: e,
                is_error: true,
            }),
        }
    }
}
