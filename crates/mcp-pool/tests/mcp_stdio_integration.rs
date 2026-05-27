//! Real MCP stdio integration tests.
//!
//! Uses the openwand-echo-mcp-server fixture to test real MCP lifecycle:
//! - Server start/stop
//! - Tool discovery
//! - Tool execution

use openwand_mcp_pool::{
    McpServerConfig, McpToolGateway, McpTransportConfig,
};
use openwand_mcp_pool::pool::McpServerPool;
use std::collections::HashMap;
use std::sync::Arc;

/// Path to the compiled echo server binary.
fn echo_server_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = std::path::Path::new(&manifest_dir)
        .ancestors()
        .nth(2)
        .unwrap()
        .join("target/release/openwand-echo-mcp-server.exe");

    assert!(
        path.exists(),
        "Echo server binary not found at {:?}. Run: cargo build -p openwand-echo-mcp-server --release",
        path
    );
    path.to_string_lossy().to_string()
}

pub fn echo_server_config() -> McpServerConfig {
    McpServerConfig {
        name: "echo".into(),
        transport: McpTransportConfig::Stdio {
            command: echo_server_bin(),
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
async fn mcp_server_start_stop_cleanly() {
    let config = echo_server_config();
    let pool = McpServerPool::new(vec![config]);

    pool.ensure_started("echo").await.unwrap();
    pool.ensure_started("echo").await.unwrap();
}

#[tokio::test]
async fn mcp_discover_tools_real_stdio_fixture() {
    let config = echo_server_config();
    let pool = McpServerPool::new(vec![config]);

    let tools = pool.discover_all_tools().await.unwrap();

    assert!(
        tools.len() >= 2,
        "Expected at least 2 tools, got: {:?}",
        tools.iter().map(|t| &t.remote_name).collect::<Vec<_>>()
    );

    let names: Vec<&str> = tools.iter().map(|t| t.remote_name.as_str()).collect();
    assert!(names.contains(&"echo_read"), "Missing echo_read, got: {:?}", names);
    assert!(names.contains(&"echo_list"), "Missing echo_list, got: {:?}", names);

    for tool in &tools {
        assert_eq!("echo", tool.server_name);
    }
}

#[tokio::test]
async fn mcp_call_tool_real_stdio_fixture() {
    let config = echo_server_config();
    let pool = McpServerPool::new(vec![config]);

    let result = pool
        .execute_tool("echo", "echo_read", serde_json::json!({"text": "hello world"}))
        .await
        .unwrap();

    assert!(!result.is_error);
    assert!(
        result.output.contains("echo: hello world"),
        "Expected 'echo: hello world', got: {}",
        result.output
    );
}

#[tokio::test]
async fn mcp_call_list_tool() {
    let config = echo_server_config();
    let pool = McpServerPool::new(vec![config]);

    let result = pool
        .execute_tool("echo", "echo_list", serde_json::json!({}))
        .await
        .unwrap();

    assert!(!result.is_error);
    assert!(
        result.output.contains("file1.rs"),
        "Expected file1.rs in output, got: {}",
        result.output
    );
}

#[tokio::test]
async fn mcp_call_nonexistent_tool_returns_error() {
    let config = echo_server_config();
    let pool = McpServerPool::new(vec![config]);

    let result = pool
        .execute_tool("echo", "nonexistent_tool", serde_json::json!({}))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn composite_executor_lists_local_plus_real_mcp() {
    use openwand_tools::composite::CompositeToolExecutor;
    use openwand_tools::local::batch1_local_tools;
    use openwand_tools::ToolExecutor;

    let config = echo_server_config();
    let gateway: Arc<dyn McpToolGateway> = Arc::new(McpServerPool::new(vec![config.clone()]));
    let mcp_configs = {
        let mut m = HashMap::new();
        m.insert(config.name.clone(), config);
        m
    };

    let executor = CompositeToolExecutor::new(batch1_local_tools(), gateway, mcp_configs);
    let report = executor.refresh_mcp_tools().await.unwrap();
    assert!(report.tools_added >= 2, "Expected >= 2 MCP tools, got: {}", report.tools_added);

    let tools = executor.available_tools();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    assert!(names.contains(&"local__file_read"), "Missing local__file_read");
    assert!(names.contains(&"mcp__echo__echo_read"), "Missing mcp__echo__echo_read, got: {:?}", names);
    assert!(names.contains(&"mcp__echo__echo_list"), "Missing mcp__echo__echo_list");
}

#[tokio::test]
async fn composite_executor_executes_mcp_canonical_name() {
    use openwand_tools::composite::CompositeToolExecutor;
    use openwand_tools::executor::ToolCall;
    use openwand_tools::result::ToolCallContext;
    use openwand_tools::local::batch1_local_tools;
    use openwand_tools::ToolExecutor;
    use openwand_core::{SessionId, ToolCallId};
    use tokio_util::sync::CancellationToken;

    let config = echo_server_config();
    let gateway: Arc<dyn McpToolGateway> = Arc::new(McpServerPool::new(vec![config.clone()]));
    let mcp_configs = {
        let mut m = HashMap::new();
        m.insert(config.name.clone(), config);
        m
    };

    let executor = CompositeToolExecutor::new(batch1_local_tools(), gateway, mcp_configs);
    executor.refresh_mcp_tools().await.unwrap();

    let ctx = ToolCallContext {
        working_directory: ".".into(),
        session_id: SessionId::new(),
        cancellation: CancellationToken::new(),
    };

    let call = ToolCall {
        id: ToolCallId("tc_mcp_1".into()),
        name: "mcp__echo__echo_read".into(),
        arguments: serde_json::json!({"text": "MCP integration test"}),
    };

    let result = executor.execute(&call, &ctx).await;
    assert!(!result.is_error, "MCP tool call failed: {}", result.output);
    assert!(
        result.output.contains("echo: MCP integration test"),
        "Expected 'echo: MCP integration test', got: {}",
        result.output
    );
}
