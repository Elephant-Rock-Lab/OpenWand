//! Wave 70B: Real filesystem approval-effect E2E.
//!
//! Proves that approved file writes produce the expected on-disk effect:
//!   - Approved write creates the file with expected contents
//!   - Rejected write creates no file
//!   - Trace records tool.resumed → tool.called → tool.completed for approval
//!   - No tool.failed on successful approved write
//!
//! Uses the session test harness with a file-writing mock tool executor.

use std::path::PathBuf;
use std::sync::Arc;

use openwand_core::mode::InteractionMode;
use openwand_core::ToolCallId;
use openwand_session::config::{RunConfig, RunStopReason};
use openwand_session::testing::harness::SessionHarness;
use openwand_session::testing::mock_tools::MockToolExecutor;
use openwand_session::{ApprovalDecision, ApprovalResolution};
use openwand_tools::executor::{ToolCall, ToolExecutor};
use openwand_tools::result::{ToolCallContext, ToolResult};
use tempfile::TempDir;
use tokio::fs;

fn conversational_config() -> RunConfig {
    RunConfig {
        mode: InteractionMode::Conversational,
        ..Default::default()
    }
}

/// A tool executor that actually writes files to a temp directory.
/// Verifies real filesystem I/O, not just mock success.
struct RealFileWriteExecutor {
    workspace: PathBuf,
    tool_name: String,
    calls: std::sync::Mutex<Vec<ToolCall>>,
}

impl RealFileWriteExecutor {
    fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            tool_name: "local__file_write".into(),
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl ToolExecutor for RealFileWriteExecutor {
    fn available_tools(&self) -> Vec<openwand_tools::descriptor::ToolDef> {
        vec![openwand_tools::descriptor::ToolDef {
            name: self.tool_name.clone(),
            display_name: Some("file_write".into()),
            description: "Write a file".into(),
            parameters_schema: serde_json::json!({"type": "object"}),
            output_schema: None,
            source: openwand_tools::descriptor::ToolSource::Local,
            declared_effect: openwand_core::tool_vocab::ToolEffect::Write,
            risk_hints: vec![],
            tags: vec!["test".into()],
            annotations: None,
        }]
    }

    fn get_descriptor(&self, name: &str) -> Option<openwand_tools::descriptor::ToolDef> {
        if name == self.tool_name {
            self.available_tools().into_iter().next()
        } else {
            None
        }
    }

    async fn execute(&self, call: &ToolCall, _context: &ToolCallContext) -> ToolResult {
        self.calls.lock().unwrap().push(call.clone());

        let path = call.arguments.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown.txt");
        let content = call.arguments.get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let full_path = self.workspace.join(path);
        match std::fs::write(&full_path, content) {
            Ok(()) => ToolResult::success(
                call.id.clone(),
                call.name.clone(),
                format!("Wrote {} bytes to {}", content.len(), path),
                content.len() as u64,
            ),
            Err(e) => ToolResult::error(
                call.id.clone(),
                call.name.clone(),
                format!("Write failed: {}", e),
                0,
            ),
        }
    }

    async fn refresh_mcp_tools(&self) -> Result<openwand_tools::executor::ToolRefreshReport, openwand_tools::error::ToolError> {
        Ok(openwand_tools::executor::ToolRefreshReport::default())
    }
}

/// Build a harness with the real file-write executor.
fn real_write_harness(workspace: PathBuf) -> (SessionHarness, Arc<RealFileWriteExecutor>) {
    use openwand_session::testing::mock_llm::MockLlmClient;
    use openwand_session::testing::mock_memory::MockMemoryReadStore;
    use openwand_session::testing::mock_policy::MockPolicyEngine;
    use openwand_trace::testing::InMemoryTraceStore;
    use openwand_store::StoredEvent;
    use openwand_trace::TraceStore;
    use openwand_session::runner::SessionRunner;
    use openwand_core::SessionId;
    use openwand_llm::LlmClient;

    let trace: Arc<InMemoryTraceStore<StoredEvent>> = Arc::new(InMemoryTraceStore::new());
    let tool_call_id = ToolCallId::new();
    let real_exec = Arc::new(RealFileWriteExecutor::new(workspace));
    let tools: Arc<dyn ToolExecutor> = real_exec.clone();

    let llm = Arc::new(MockLlmClient::tool_then_stop(
        tool_call_id.as_str().to_string(),
        "local__file_write",
        serde_json::json!({"path": "approval_real.txt", "content": "Real I/O verified!"}),
    ));

    let policy = Arc::new(MockPolicyEngine::require_confirmation_for("local__file_write"));
    let memory = Arc::new(MockMemoryReadStore::new());

    let runner = SessionRunner::new(
        SessionId::new(),
        trace.clone() as Arc<dyn TraceStore<StoredEvent>>,
        llm.clone() as Arc<dyn LlmClient>,
        tools,
        policy.clone() as Arc<dyn openwand_policy::PolicyEngine>,
        memory.clone() as Arc<dyn openwand_memory::MemoryReadStore>,
        ".".into(),
    );

    let harness = SessionHarness {
        runner,
        trace,
        llm,
        policy,
        tools: Arc::new(MockToolExecutor::empty()), // placeholder — real exec is separate
        memory,
    };

    (harness, real_exec)
}

// ---- Tests ----

/// Approved write creates the file with expected contents.
/// Trace shows tool.resumed → tool.called → tool.completed (no tool.failed).
#[tokio::test]
async fn approved_write_creates_file_with_expected_contents() {
    let dir = TempDir::new().expect("temp dir");
    let workspace = dir.path().to_path_buf();

    let (harness, real_exec) = real_write_harness(workspace.clone());

    // Step 1: Run turn — should suspend for approval
    let turn = harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .expect("turn should run");
    assert_eq!(
        RunStopReason::AwaitingApproval, turn.stop_reason,
        "Should suspend for file_write approval"
    );

    // Step 2: Approve
    let result = harness
        .runner
        .resolve_approval(ApprovalDecision::approve(), conversational_config())
        .await
        .expect("resume should succeed");

    // Step 3: Verify approval resolution
    assert!(
        matches!(result.resolution, ApprovalResolution::Approve),
        "Resolution should be Approve"
    );

    // Step 4: File exists on disk
    let file_path = workspace.join("approval_real.txt");
    assert!(
        file_path.exists(),
        "Approved file should exist on disk at {:?}",
        file_path
    );

    // Step 5: File contents match expected payload
    let contents = fs::read_to_string(&file_path)
        .await
        .expect("should read approved file");
    assert_eq!(
        "Real I/O verified!", contents,
        "File contents should match expected payload"
    );

    // Step 6: Tool was actually called
    let calls = real_exec.calls.lock().unwrap();
    assert_eq!(1, calls.len(), "Tool should have been called exactly once");
    assert_eq!("local__file_write", calls[0].name);

    // Step 7: Trace ordering
    let kinds = harness.trace.event_kinds().await;
    let resumed_pos = kinds.iter().position(|k| k == "tool.resumed");
    let called_pos = kinds.iter().position(|k| k == "tool.called");
    let completed_pos = kinds.iter().position(|k| k == "tool.completed");
    let failed_pos = kinds.iter().position(|k| k == "tool.failed");

    assert!(resumed_pos.is_some(), "tool.resumed should be in trace");
    assert!(called_pos.is_some(), "tool.called should be in trace");
    assert!(
        resumed_pos.unwrap() < called_pos.unwrap(),
        "tool.resumed must appear before tool.called: resumed={} called={}",
        resumed_pos.unwrap(),
        called_pos.unwrap()
    );
    assert!(
        completed_pos.is_some(),
        "tool.completed should be in trace"
    );
    assert!(
        failed_pos.is_none(),
        "tool.failed should NOT be in trace — write succeeded"
    );

    // Full ordering: gate.evaluated → tool.suspended → tool.resumed → tool.called → tool.completed
    let gate_pos = kinds.iter().position(|k| k == "gate.evaluated");
    let suspended_pos = kinds.iter().position(|k| k == "tool.suspended");
    assert!(gate_pos.is_some(), "gate.evaluated should be in trace");
    assert!(suspended_pos.is_some(), "tool.suspended should be in trace");
    assert!(gate_pos.unwrap() < suspended_pos.unwrap());
    assert!(suspended_pos.unwrap() < resumed_pos.unwrap());
    assert!(called_pos.unwrap() < completed_pos.unwrap());
}

/// Rejected write does NOT create the file.
#[tokio::test]
async fn rejected_write_creates_no_file() {
    let dir = TempDir::new().expect("temp dir");
    let workspace = dir.path().to_path_buf();

    let (harness, real_exec) = real_write_harness(workspace.clone());

    // Step 1: Run turn — suspends
    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .expect("turn should run");

    // Step 2: Reject
    let result = harness
        .runner
        .resolve_approval(ApprovalDecision::reject(), conversational_config())
        .await
        .expect("resolve should succeed");

    assert!(
        matches!(result.resolution, ApprovalResolution::Reject { .. }),
        "Resolution should be Reject"
    );

    // Step 3: File does NOT exist on disk
    let file_path = workspace.join("approval_real.txt");
    assert!(
        !file_path.exists(),
        "Rejected file should NOT exist on disk at {:?}",
        file_path
    );

    // Step 4: Tool was NOT called
    let calls = real_exec.calls.lock().unwrap();
    assert_eq!(0, calls.len(), "Tool should NOT have been called on rejection");

    // Step 5: Trace has tool.denied, not tool.called
    let kinds = harness.trace.event_kinds().await;
    assert!(
        kinds.iter().any(|k| k == "tool.denied"),
        "tool.denied should be in trace after rejection"
    );
    assert!(
        !kinds.iter().any(|k| k == "tool.called"),
        "tool.called should NOT be in trace after rejection"
    );
    assert!(
        !kinds.iter().any(|k| k == "tool.completed"),
        "tool.completed should NOT be in trace after rejection"
    );
}
