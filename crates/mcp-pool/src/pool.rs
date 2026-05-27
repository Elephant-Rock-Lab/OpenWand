//! MCP Server Pool — manages MCP server lifecycles and dispatches tool calls via rmcp.

use crate::{
    McpDiscoveredTool, McpPoolError, McpServerConfig, McpServerState, McpToolAnnotations,
    McpToolGateway, McpToolResult, McpTransportConfig,
};
use async_trait::async_trait;
use rmcp::model::{CallToolRequestParams, ClientCapabilities, Implementation};
use rmcp::service::{RoleClient, RunningService};
use rmcp::serve_client;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// A started MCP server connection.
struct McpConnection {
    peer: rmcp::service::Peer<RoleClient>,
    // Hold the RunningService alive so the background task keeps running.
    _running: RunningService<RoleClient, rmcp::model::InitializeRequestParams>,
    #[allow(dead_code)] // Will be used for health monitoring in Wave 02
    state: McpServerState,
}

/// Real implementation of McpToolGateway that manages MCP server processes.
pub struct McpServerPool {
    configs: HashMap<String, McpServerConfig>,
    connections: RwLock<HashMap<String, McpConnection>>,
}

impl McpServerPool {
    pub fn new(configs: Vec<McpServerConfig>) -> Self {
        let config_map: HashMap<String, McpServerConfig> = configs
            .into_iter()
            .map(|c| (c.name.clone(), c))
            .collect();
        Self {
            configs: config_map,
            connections: RwLock::new(HashMap::new()),
        }
    }

    fn build_client_info() -> rmcp::model::InitializeRequestParams {
        rmcp::model::InitializeRequestParams::new(
            ClientCapabilities::default(),
            Implementation::new("openwand", env!("CARGO_PKG_VERSION")),
        )
    }

    async fn start_server(
        &self,
        config: &McpServerConfig,
    ) -> Result<McpConnection, McpPoolError> {
        match &config.transport {
            McpTransportConfig::Stdio {
                command,
                args,
                env,
                cwd,
            } => {
                let mut cmd = tokio::process::Command::new(command);
                cmd.args(args);
                cmd.envs(env);
                if let Some(dir) = cwd {
                    cmd.current_dir(dir);
                }

                let transport =
                    rmcp::transport::child_process::TokioChildProcess::new(cmd).map_err(
                        |e| McpPoolError::StartFailed {
                            server: config.name.clone(),
                            reason: format!("Failed to spawn process: {e}"),
                        },
                    )?;

                let client_info = Self::build_client_info();
                let running = serve_client(client_info, transport)
                    .await
                    .map_err(|e| McpPoolError::StartFailed {
                        server: config.name.clone(),
                        reason: format!("Handshake failed: {e}"),
                    })?;

                let peer = running.peer().clone();
                let now = chrono::Utc::now();

                Ok(McpConnection {
                    peer,
                    _running: running,
                    state: McpServerState::Ready {
                        started_at: now,
                        last_discovered_at: None,
                    },
                })
            }

            #[cfg(feature = "http")]
            McpTransportConfig::StreamableHttp { .. } => Err(McpPoolError::Transport(
                "HTTP transport not yet supported".into(),
            )),
        }
    }
}

#[async_trait]
impl McpToolGateway for McpServerPool {
    async fn ensure_started(&self, server_name: &str) -> Result<(), McpPoolError> {
        let config = self
            .configs
            .get(server_name)
            .ok_or_else(|| McpPoolError::ServerNotConfigured {
                server: server_name.to_string(),
            })?;

        if !config.enabled {
            return Err(McpPoolError::ServerDisabled {
                server: server_name.to_string(),
            });
        }

        {
            let connections = self.connections.read().await;
            if connections.contains_key(server_name) {
                return Ok(());
            }
        }

        let conn = self.start_server(config).await?;
        let mut connections = self.connections.write().await;
        connections.insert(server_name.to_string(), conn);

        Ok(())
    }

    async fn discover_all_tools(&self) -> Result<Vec<McpDiscoveredTool>, McpPoolError> {
        let mut all_tools = Vec::new();

        for (name, config) in &self.configs {
            if !config.enabled {
                continue;
            }

            if let Err(e) = self.ensure_started(name).await {
                tracing::warn!(server = %name, "Skipping MCP server: {e}");
                continue;
            }

            let connections = self.connections.read().await;
            let conn = match connections.get(name) {
                Some(c) => c,
                None => continue,
            };

            match conn.peer.list_tools(None).await {
                Ok(tools_result) => {
                    for tool in tools_result.tools {
                        let discovered = McpDiscoveredTool {
                            server_name: name.clone(),
                            remote_name: tool.name.to_string(),
                            title: tool.title,
                            description: tool
                                .description
                                .as_ref()
                                .map(|d| d.to_string())
                                .unwrap_or_default(),
                            input_schema: serde_json::Value::Object(
                                tool.input_schema.as_ref().clone(),
                            ),
                            output_schema: None,
                            annotations: tool.annotations.map(|a| McpToolAnnotations {
                                read_only_hint: a.read_only_hint,
                                destructive_hint: a.destructive_hint,
                                idempotent_hint: a.idempotent_hint,
                                open_world_hint: a.open_world_hint,
                            }),
                        };
                        all_tools.push(discovered);
                    }
                }
                Err(e) => {
                    tracing::warn!(server = %name, "Tool discovery failed: {e}");
                }
            }
        }

        Ok(all_tools)
    }

    async fn execute_tool(
        &self,
        server_name: &str,
        remote_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpPoolError> {
        self.ensure_started(server_name).await?;

        let connections = self.connections.read().await;
        let conn = connections.get(server_name).ok_or_else(|| {
            McpPoolError::CallFailed {
                server: server_name.to_string(),
                tool: remote_name.to_string(),
                reason: "Server not connected".into(),
            }
        })?;

        let mut call_param = CallToolRequestParams::new(remote_name.to_string());
        if let Some(obj) = arguments.as_object() {
            call_param = call_param.with_arguments(obj.clone());
        }

        match conn.peer.call_tool(call_param).await {
            Ok(result) => {
                let output = result
                    .content
                    .into_iter()
                    .map(|c| {
                        if let rmcp::model::RawContent::Text(text) = c.raw {
                            text.text
                        } else {
                            "(non-text content)".to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(McpToolResult {
                    server_name: server_name.to_string(),
                    remote_name: remote_name.to_string(),
                    output,
                    is_error: result.is_error.unwrap_or(false),
                })
            }
            Err(e) => Err(McpPoolError::CallFailed {
                server: server_name.to_string(),
                tool: remote_name.to_string(),
                reason: format!("{e}"),
            }),
        }
    }
}
