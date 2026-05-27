use async_trait::async_trait;
use openwand_core::ToolCallId;
use openwand_tools::descriptor::{ToolAnnotations, ToolDef, ToolSource};
use openwand_tools::error::ToolError;
use openwand_tools::executor::{ToolCall, ToolExecutor, ToolRefreshReport};
use openwand_tools::result::{ToolCallContext, ToolResult};
use openwand_core::tool_vocab::ToolEffect;
use std::collections::HashMap;
use tokio::sync::Mutex;

/// Mock tool executor for deterministic testing.
/// Panics if `refresh_mcp_tools()` is called — session must not refresh MCP.
pub struct MockToolExecutor {
    tools: Vec<ToolDef>,
    results: HashMap<String, ToolResult>,
    calls: Mutex<Vec<ToolCall>>,
}

impl MockToolExecutor {
    pub fn empty() -> Self {
        Self {
            tools: vec![],
            results: HashMap::new(),
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn with_success(tool_name: &str, output: &str) -> Self {
        let mut results = HashMap::new();
        results.insert(
            tool_name.to_string(),
            ToolResult::success(
                ToolCallId::new(),
                tool_name.to_string(),
                output.to_string(),
                42,
            ),
        );
        Self {
            tools: vec![make_local_def(tool_name)],
            results,
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn with_error(tool_name: &str, error_msg: &str) -> Self {
        let mut results = HashMap::new();
        results.insert(
            tool_name.to_string(),
            ToolResult::error(
                ToolCallId::new(),
                tool_name.to_string(),
                error_msg.to_string(),
                1,
            ),
        );
        Self {
            tools: vec![make_local_def(tool_name)],
            results,
            calls: Mutex::new(Vec::new()),
        }
    }

    pub async fn calls(&self) -> Vec<ToolCall> {
        self.calls.lock().await.clone()
    }
}

fn make_local_def(name: &str) -> ToolDef {
    ToolDef {
        name: format!("local__{name}"),
        display_name: Some(name.to_string()),
        description: format!("Mock tool: {name}"),
        parameters_schema: serde_json::json!({"type": "object"}),
        output_schema: None,
        source: ToolSource::Local,
        declared_effect: ToolEffect::Read,
        risk_hints: vec![],
        tags: vec!["mock".into()],
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
    }
}

#[async_trait]
impl ToolExecutor for MockToolExecutor {
    fn available_tools(&self) -> Vec<ToolDef> {
        self.tools.clone()
    }

    fn get_descriptor(&self, name: &str) -> Option<ToolDef> {
        self.tools.iter().find(|t| t.name == name).cloned()
    }

    async fn execute(
        &self,
        call: &ToolCall,
        _context: &ToolCallContext,
    ) -> ToolResult {
        self.calls.lock().await.push(call.clone());

        self.results
            .get(&call.name)
            .cloned()
            .unwrap_or_else(|| {
                ToolResult::error(
                    call.id.clone(),
                    call.name.clone(),
                    format!("mock tool not found: {}", call.name),
                    0,
                )
            })
    }

    async fn refresh_mcp_tools(&self) -> Result<ToolRefreshReport, ToolError> {
        panic!("session must not call refresh_mcp_tools — MCP awareness is forbidden")
    }
}
