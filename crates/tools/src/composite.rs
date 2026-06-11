//! Composite tool executor — unifies local + MCP tools behind one seam.

use async_trait::async_trait;
use openwand_mcp_pool::{McpToolGateway, McpToolResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::descriptor::ToolDef;
use crate::error::ToolError;
use crate::result::ToolCallContext;
use crate::{ToolCall, ToolExecutor, ToolRefreshReport};
use crate::local::BuiltinToolProvider;
use crate::naming::{parse_canonical_tool_name, ParsedToolName};
use crate::result::ToolResult;

/// Unified tool executor combining local and MCP tools.
pub struct CompositeToolExecutor {
    local: BuiltinToolProvider,
    gateway: Arc<dyn McpToolGateway>,
    mcp_configs: HashMap<String, openwand_mcp_pool::McpServerConfig>,
    mcp_cache: RwLock<HashMap<String, ToolDef>>,
}

impl CompositeToolExecutor {
    pub fn new(
        local: BuiltinToolProvider,
        gateway: Arc<dyn McpToolGateway>,
        mcp_configs: HashMap<String, openwand_mcp_pool::McpServerConfig>,
    ) -> Self {
        Self {
            local,
            gateway,
            mcp_configs,
            mcp_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Create with only local tools (no MCP servers).
    pub fn local_only(local: BuiltinToolProvider) -> Self {
        Self {
            local,
            gateway: Arc::new(NoopMcpGateway),
            mcp_configs: HashMap::new(),
            mcp_cache: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ToolExecutor for CompositeToolExecutor {
    fn available_tools(&self) -> Vec<ToolDef> {
        let mut tools = self.local.available_descriptors();
        // For sync access we read the cache without blocking.
        // This is safe because we only populate in refresh_mcp_tools (async).
        // Use try_read to avoid blocking the runtime.
        if let Ok(cache) = self.mcp_cache.try_read() {
            tools.extend(cache.values().cloned());
        }
        tools
    }

    fn get_descriptor(&self, name: &str) -> Option<ToolDef> {
        // Check local first
        if let Some(desc) = self.local.get_descriptor(name) {
            return Some(desc);
        }
        // Then MCP cache
        if let Ok(cache) = self.mcp_cache.try_read() {
            cache.get(name).cloned()
        } else {
            None
        }
    }

    async fn execute(&self, call: &ToolCall, context: &ToolCallContext) -> ToolResult {
        let start = std::time::Instant::now();
        let parsed = match parse_canonical_tool_name(&call.name) {
            Some(p) => p,
            None => {
                return ToolResult::error(
                    call.id.clone(),
                    call.name.clone(),
                    format!("Invalid tool name format: '{}'", call.name),
                    start.elapsed().as_millis() as u64,
                );
            }
        };

        match parsed {
            ParsedToolName::Local { name: _ } => {
                match self
                    .local
                    .execute(&call.name, call.arguments.clone(), context.clone())
                    .await
                {
                    Some(result) => result,
                    None => ToolResult::error(
                        call.id.clone(),
                        call.name.clone(),
                        format!("Unknown local tool: '{}'", call.name),
                        start.elapsed().as_millis() as u64,
                    ),
                }
            }
            ParsedToolName::Mcp {
                server,
                remote_name,
            } => {
                let mcp_result = self
                    .gateway
                    .execute_tool(&server, &remote_name, call.arguments.clone())
                    .await;

                match mcp_result {
                    Ok(McpToolResult {
                        output,
                        is_error,
                        ..
                    }) => {
                        
                        if is_error {
                            ToolResult::error(
                                call.id.clone(),
                                call.name.clone(),
                                output,
                                start.elapsed().as_millis() as u64,
                            )
                        } else {
                            ToolResult::success(
                                call.id.clone(),
                                call.name.clone(),
                                output,
                                start.elapsed().as_millis() as u64,
                            )
                        }
                    }
                    Err(e) => ToolResult::error(
                        call.id.clone(),
                        call.name.clone(),
                        format!("MCP tool call failed: {e}"),
                        start.elapsed().as_millis() as u64,
                    ),
                }
            }
        }
    }

    async fn refresh_mcp_tools(&self) -> Result<ToolRefreshReport, ToolError> {
        let mut report = ToolRefreshReport::default();

        let discovered = self
            .gateway
            .discover_all_tools()
            .await
            .map_err(|e| ToolError::McpRefresh(e.to_string()))?;

        report.servers_checked = self.mcp_configs.len() as u32;

        let mut new_cache = HashMap::new();
        for tool in discovered {
            let config = match self.mcp_configs.get(&tool.server_name) {
                Some(c) => c,
                None => {
                    report
                        .errors
                        .push(format!("Unknown server: {}", tool.server_name));
                    continue;
                }
            };

            let tool_def = ToolDef::from_mcp_discovered(tool, config);
            report.tools_added += 1;
            new_cache.insert(tool_def.name.clone(), tool_def);
        }

        // Count removed
        let old_cache = self.mcp_cache.read().await;
        report.tools_removed = old_cache
            .keys()
            .filter(|k| !new_cache.contains_key(*k))
            .count() as u32;

        drop(old_cache);
        let mut cache = self.mcp_cache.write().await;
        *cache = new_cache;

        Ok(report)
    }
}

/// A no-op MCP gateway for local-only mode.
struct NoopMcpGateway;

#[async_trait]
impl McpToolGateway for NoopMcpGateway {
    async fn ensure_started(&self, _server_name: &str) -> Result<(), openwand_mcp_pool::McpPoolError> {
        Ok(())
    }

    async fn discover_all_tools(&self) -> Result<Vec<openwand_mcp_pool::McpDiscoveredTool>, openwand_mcp_pool::McpPoolError> {
        Ok(vec![])
    }

    async fn execute_tool(
        &self,
        server_name: &str,
        remote_name: &str,
        _arguments: serde_json::Value,
    ) -> Result<McpToolResult, openwand_mcp_pool::McpPoolError> {
        Err(openwand_mcp_pool::McpPoolError::CallFailed {
            server: server_name.to_string(),
            tool: remote_name.to_string(),
            reason: "No MCP gateway configured".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local::batch1_local_tools;
    use openwand_core::{SessionId, ToolCallId};
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    fn test_context(dir: &TempDir) -> ToolCallContext {
        ToolCallContext {
            working_directory: dir.path().to_string_lossy().to_string(),
            session_id: SessionId::new(),
            cancellation: CancellationToken::new(),
        }
    }

    #[tokio::test]
    async fn composite_local_file_read() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("test.txt"), "composite works")
            .await
            .unwrap();

        let executor = CompositeToolExecutor::local_only(batch1_local_tools());
        let ctx = test_context(&dir);
        let call = ToolCall {
            id: ToolCallId("tc_1".into()),
            name: "local__file_read".into(),
            arguments: serde_json::json!({"path": "test.txt"}),
        };

        let result = executor.execute(&call, &ctx).await;
        assert!(!result.is_error);
        assert_eq!("composite works", result.output);
    }

    #[tokio::test]
    async fn composite_unknown_tool_returns_error_result() {
        let dir = TempDir::new().unwrap();
        let executor = CompositeToolExecutor::local_only(batch1_local_tools());
        let ctx = test_context(&dir);

        let call = ToolCall {
            id: ToolCallId("tc_2".into()),
            name: "local__nonexistent".into(),
            arguments: serde_json::json!({}),
        };

        let result = executor.execute(&call, &ctx).await;
        assert!(result.is_error);
        assert!(result.output.contains("Unknown local tool"));
    }

    #[tokio::test]
    async fn composite_invalid_name_returns_error_result() {
        let dir = TempDir::new().unwrap();
        let executor = CompositeToolExecutor::local_only(batch1_local_tools());
        let ctx = test_context(&dir);

        let call = ToolCall {
            id: ToolCallId("tc_3".into()),
            name: "bad_name_no_prefix".into(),
            arguments: serde_json::json!({}),
        };

        let result = executor.execute(&call, &ctx).await;
        assert!(result.is_error);
        assert!(result.output.contains("Invalid tool name format"));
    }

    #[tokio::test]
    async fn composite_lists_local_tools() {
        let executor = CompositeToolExecutor::local_only(batch1_local_tools());
        let tools = executor.available_tools();
        assert!(tools.len() >= 3);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"local__file_read"));
        assert!(names.contains(&"local__file_list"));
        assert!(names.contains(&"local__file_search"));
    }

    #[tokio::test]
    async fn composite_get_descriptor_finds_local_tool() {
        let executor = CompositeToolExecutor::local_only(batch1_local_tools());
        let desc = executor.get_descriptor("local__file_read").unwrap();
        assert_eq!("local__file_read", desc.name);
    }

    #[tokio::test]
    async fn composite_mcp_missing_tool_returns_error_result() {
        let dir = TempDir::new().unwrap();
        let executor = CompositeToolExecutor::local_only(batch1_local_tools());
        let ctx = test_context(&dir);

        let call = ToolCall {
            id: ToolCallId("tc_4".into()),
            name: "mcp__nonexistent__some_tool".into(),
            arguments: serde_json::json!({}),
        };

        let result = executor.execute(&call, &ctx).await;
        assert!(result.is_error);
        assert!(result.output.contains("MCP tool call failed"));
    }
}
