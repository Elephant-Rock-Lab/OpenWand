//! Integration tests for MCP pool + tools composite.

use openwand_core::{SessionId, ToolCallId};
use openwand_mcp_pool::testing::{MockMcpGateway, MockMcpServer, MockTool};
use openwand_mcp_pool::{McpServerConfig, McpTransportConfig, McpToolAnnotations};
use openwand_tools::composite::CompositeToolExecutor;
use openwand_tools::executor::{ToolCall, ToolExecutor};
use openwand_tools::result::ToolCallContext;
use openwand_tools::local::batch1_local_tools;
use openwand_tools::naming::canonical_mcp_tool_name;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

fn test_context(dir: &TempDir) -> ToolCallContext {
    ToolCallContext {
        working_directory: dir.path().to_string_lossy().to_string(),
        session_id: SessionId::new(),
        cancellation: CancellationToken::new(),
    }
}

fn echo_config() -> McpServerConfig {
    McpServerConfig {
        name: "echo".into(),
        transport: McpTransportConfig::Stdio {
            command: "echo".into(),
            args: vec![],
            env: HashMap::new(),
            cwd: None,
        },
        default_effect: Some(openwand_core::tool_vocab::ToolEffect::Read),
        tool_effects: HashMap::new(),
        enabled: true,
    }
}

#[tokio::test]
async fn mcp_discovery_populates_composite() {
    let gateway = MockMcpGateway::new(vec![MockMcpServer {
        name: "echo".into(),
        tools: vec![MockTool {
            name: "echo".into(),
            description: "Echo back input".into(),
            annotations: Some(McpToolAnnotations {
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: None,
                open_world_hint: None,
            }),
            handler: std::sync::Arc::new(|args| {
                let msg = args
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                Ok(format!("echo: {msg}"))
            }),
        }],
    }]);

    let mut mcp_configs = HashMap::new();
    mcp_configs.insert("echo".into(), echo_config());

    let executor =
        CompositeToolExecutor::new(batch1_local_tools(), std::sync::Arc::new(gateway), mcp_configs);

    // Before refresh — only local tools
    let tools = executor.available_tools();
    assert_eq!(3, tools.len());

    // Refresh MCP tools
    let report = executor.refresh_mcp_tools().await.unwrap();
    assert_eq!(1, report.servers_checked);
    assert_eq!(1, report.tools_added);

    // After refresh — local + MCP
    let tools = executor.available_tools();
    assert_eq!(4, tools.len());

    let mcp_name = canonical_mcp_tool_name("echo", "echo");
    assert!(tools.iter().any(|t| t.name == mcp_name));
}

#[tokio::test]
async fn mcp_tool_call_through_composite() {
    let gateway = MockMcpGateway::new(vec![MockMcpServer {
        name: "echo".into(),
        tools: vec![MockTool {
            name: "echo".into(),
            description: "Echo back input".into(),
            annotations: None,
            handler: std::sync::Arc::new(|args| {
                let msg = args
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                Ok(format!("echo: {msg}"))
            }),
        }],
    }]);

    let mut mcp_configs = HashMap::new();
    mcp_configs.insert("echo".into(), echo_config());

    let executor =
        CompositeToolExecutor::new(batch1_local_tools(), std::sync::Arc::new(gateway), mcp_configs);

    executor.refresh_mcp_tools().await.unwrap();

    let dir = TempDir::new().unwrap();
    let ctx = test_context(&dir);
    let mcp_name = canonical_mcp_tool_name("echo", "echo");

    let call = ToolCall {
        id: ToolCallId("tc_mcp_1".into()),
        name: mcp_name,
        arguments: serde_json::json!({"message": "hello"}),
    };

    let result = executor.execute(&call, &ctx).await;
    assert!(!result.is_error, "Expected success, got: {}", result.output);
    assert_eq!("echo: hello", result.output);
}

#[tokio::test]
async fn local_plus_mcp_composite_listing() {
    let gateway = MockMcpGateway::new(vec![MockMcpServer {
        name: "echo".into(),
        tools: vec![MockTool {
            name: "echo".into(),
            description: "Echo tool".into(),
            annotations: None,
            handler: std::sync::Arc::new(|_| Ok("ok".into())),
        }],
    }]);

    let mut mcp_configs = HashMap::new();
    mcp_configs.insert("echo".into(), echo_config());

    let executor =
        CompositeToolExecutor::new(batch1_local_tools(), std::sync::Arc::new(gateway), mcp_configs);

    executor.refresh_mcp_tools().await.unwrap();

    let tools = executor.available_tools();
    let local_count = tools
        .iter()
        .filter(|t| t.name.starts_with("local__"))
        .count();
    let mcp_count = tools
        .iter()
        .filter(|t| t.name.starts_with("mcp__"))
        .count();
    assert_eq!(3, local_count);
    assert_eq!(1, mcp_count);
}

#[tokio::test]
async fn mcp_tool_descriptor_has_correct_source() {
    let gateway = MockMcpGateway::new(vec![MockMcpServer {
        name: "echo".into(),
        tools: vec![MockTool {
            name: "echo".into(),
            description: "Echo tool".into(),
            annotations: None,
            handler: std::sync::Arc::new(|_| Ok("ok".into())),
        }],
    }]);

    let mut mcp_configs = HashMap::new();
    mcp_configs.insert("echo".into(), echo_config());

    let executor =
        CompositeToolExecutor::new(batch1_local_tools(), std::sync::Arc::new(gateway), mcp_configs);

    executor.refresh_mcp_tools().await.unwrap();

    let mcp_name = canonical_mcp_tool_name("echo", "echo");
    let desc = executor.get_descriptor(&mcp_name).unwrap();

    match desc.source {
        openwand_tools::descriptor::ToolSource::Mcp { server, remote_name } => {
            assert_eq!("echo", server);
            assert_eq!("echo", remote_name);
        }
        _ => panic!("Expected MCP source"),
    }
}
