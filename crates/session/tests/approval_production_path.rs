//! Wave 71B: Production-path approval E2E.
//!
//! Proves that approved file writes through the PRODUCTION tool executor
//! (CompositeToolExecutor + file_write_handler + schema validation + sandbox)
//! produce the expected on-disk effect:
//!   - Approved write creates the file with expected contents
//!   - Rejected write creates no file
//!   - Trace records tool.resumed -> tool.called -> tool.completed
//!   - No tool.failed on successful approved write
//!
//! Production path exercised:
//!   - CompositeToolExecutor::local_only(batch2_local_tools())
//!   - file_write_handler with JSON schema validation
//!   - resolve_workspace_path() sandbox containment

use std::path::PathBuf;
use std::sync::Arc;

use openwand_core::mode::InteractionMode;
use openwand_core::ToolCallId;
use openwand_session::config::{RunConfig, RunStopReason};
use openwand_session::runner::SessionRunner;
use openwand_session::testing::mock_llm::MockLlmClient;
use openwand_session::testing::mock_memory::MockMemoryReadStore;
use openwand_session::testing::mock_policy::MockPolicyEngine;
use openwand_session::{ApprovalDecision, ApprovalResolution};
use openwand_store::StoredEvent;
use openwand_tools::composite::CompositeToolExecutor;
use openwand_tools::local::batch2_local_tools;
use openwand_trace::testing::InMemoryTraceStore;
use openwand_trace::TraceStore;
use tempfile::TempDir;
use tokio::fs;

fn conversational_config(workspace: &std::path::Path) -> RunConfig {
    RunConfig {
        mode: InteractionMode::Conversational,
        working_directory: workspace.to_string_lossy().into(),
        ..Default::default()
    }
}

/// Build a runner with the PRODUCTION tool executor.
fn production_runner(
    workspace: PathBuf,
    file_name: &str,
    file_content: &str,
) -> (
    SessionRunner,
    Arc<InMemoryTraceStore<StoredEvent>>,
) {
    let trace: Arc<InMemoryTraceStore<StoredEvent>> = Arc::new(InMemoryTraceStore::new());
    let tool_call_id = ToolCallId::new();

    // Production tool executor: full schema validation + sandbox
    let local_tools = batch2_local_tools();
    let tools: Arc<dyn openwand_tools::executor::ToolExecutor> =
        Arc::new(CompositeToolExecutor::local_only(local_tools));

    let llm = Arc::new(MockLlmClient::tool_then_stop(
        tool_call_id.as_str().to_string(),
        "local__file_write",
        serde_json::json!({
            "path": file_name,
            "content": file_content,
        }),
    ));

    let policy = Arc::new(MockPolicyEngine::require_confirmation_for("local__file_write"));
    let memory = Arc::new(MockMemoryReadStore::new());

    let runner = SessionRunner::new(
        openwand_core::SessionId::new(),
        trace.clone() as Arc<dyn TraceStore<StoredEvent>>,
        llm.clone() as Arc<dyn openwand_llm::LlmClient>,
        tools,
        policy.clone() as Arc<dyn openwand_policy::PolicyEngine>,
        memory.clone() as Arc<dyn openwand_memory::MemoryReadStore>,
        workspace.to_string_lossy().into(),
    );

    (runner, trace)
}

// ---- Tests ----

/// Production-path approved write: file created with expected contents.
/// Exercises real file_write_handler, schema validation, and sandbox.
#[tokio::test]
async fn production_approved_write_creates_file_via_sandbox() {
    let dir = TempDir::new().expect("temp dir");
    let workspace = dir.path().to_path_buf();

    let (runner, trace) = production_runner(
        workspace.clone(),
        "prod_test.txt",
        "Production path verified!",
    );

    // Step 1: Run turn — should suspend for approval
    let turn = runner
        .run_turn("Write a file".into(), conversational_config(&workspace))
        .await
        .expect("turn should run");
    assert_eq!(
        RunStopReason::AwaitingApproval, turn.stop_reason,
        "Should suspend for file_write approval"
    );

    // Step 2: Approve
    let result = runner
        .resolve_approval(ApprovalDecision::approve(), conversational_config(&workspace))
        .await
        .expect("resume should succeed");

    assert!(
        matches!(result.resolution, ApprovalResolution::Approve),
        "Resolution should be Approve"
    );

    // Step 3: File exists on disk
    let file_path = workspace.join("prod_test.txt");
    assert!(
        file_path.exists(),
        "Approved file should exist on disk at {:?}",
        file_path
    );

    // Step 4: File contents match
    let contents = fs::read_to_string(&file_path)
        .await
        .expect("should read approved file");
    assert_eq!(
        "Production path verified!", contents,
        "File contents should match expected payload"
    );

    // Step 5: Trace ordering
    let kinds = trace.event_kinds().await;
    assert!(
        kinds.iter().any(|k| k == "tool.resumed"),
        "tool.resumed should be in trace"
    );
    assert!(
        kinds.iter().any(|k| k == "tool.called"),
        "tool.called should be in trace"
    );
    assert!(
        kinds.iter().any(|k| k == "tool.completed"),
        "tool.completed should be in trace"
    );
    assert!(
        !kinds.iter().any(|k| k == "tool.failed"),
        "tool.failed should NOT be in trace — write succeeded"
    );

    let resumed_pos = kinds.iter().position(|k| k == "tool.resumed").unwrap();
    let called_pos = kinds.iter().position(|k| k == "tool.called").unwrap();
    let completed_pos = kinds.iter().position(|k| k == "tool.completed").unwrap();
    assert!(resumed_pos < called_pos, "resumed must precede called");
    assert!(called_pos < completed_pos, "called must precede completed");
}

/// Production-path rejected write: no file created.
#[tokio::test]
async fn production_rejected_write_creates_no_file() {
    let dir = TempDir::new().expect("temp dir");
    let workspace = dir.path().to_path_buf();

    let (runner, trace) = production_runner(
        workspace.clone(),
        "prod_reject.txt",
        "This should never be written",
    );

    // Step 1: Run turn — suspends
    runner
        .run_turn("Write a file".into(), conversational_config(&workspace))
        .await
        .expect("turn should run");

    // Step 2: Reject
    let result = runner
        .resolve_approval(ApprovalDecision::reject(), conversational_config(&workspace))
        .await
        .expect("resolve should succeed");

    assert!(
        matches!(result.resolution, ApprovalResolution::Reject { .. }),
        "Resolution should be Reject"
    );

    // Step 3: File does NOT exist
    let file_path = workspace.join("prod_reject.txt");
    assert!(
        !file_path.exists(),
        "Rejected file should NOT exist on disk at {:?}",
        file_path
    );

    // Step 4: Trace has tool.denied
    let kinds = trace.event_kinds().await;
    assert!(
        kinds.iter().any(|k| k == "tool.denied"),
        "tool.denied should be in trace after rejection"
    );
    assert!(
        !kinds.iter().any(|k| k == "tool.called"),
        "tool.called should NOT be in trace after rejection"
    );
}

/// Production sandbox rejects traversal paths even when policy approves.
#[tokio::test]
async fn production_sandbox_blocks_traversal_even_when_approved() {
    let dir = TempDir::new().expect("temp dir");
    let workspace = dir.path().to_path_buf();

    let trace: Arc<InMemoryTraceStore<StoredEvent>> = Arc::new(InMemoryTraceStore::new());
    let tool_call_id = ToolCallId::new();

    let local_tools = batch2_local_tools();
    let tools: Arc<dyn openwand_tools::executor::ToolExecutor> =
        Arc::new(CompositeToolExecutor::local_only(local_tools));

    // Request a file with traversal path
    let llm = Arc::new(MockLlmClient::tool_then_stop(
        tool_call_id.as_str().to_string(),
        "local__file_write",
        serde_json::json!({
            "path": "../../../etc/escape.txt",
            "content": "Escape attempt",
        }),
    ));

    let policy = Arc::new(MockPolicyEngine::require_confirmation_for("local__file_write"));
    let memory = Arc::new(MockMemoryReadStore::new());

    let runner = SessionRunner::new(
        openwand_core::SessionId::new(),
        trace.clone() as Arc<dyn TraceStore<StoredEvent>>,
        llm.clone() as Arc<dyn openwand_llm::LlmClient>,
        tools,
        policy.clone() as Arc<dyn openwand_policy::PolicyEngine>,
        memory.clone() as Arc<dyn openwand_memory::MemoryReadStore>,
        workspace.to_string_lossy().into(),
    );

    // Step 1: Run turn — suspends
    let turn = runner
        .run_turn("Write traversal".into(), conversational_config(&workspace))
        .await
        .expect("turn should run");
    assert_eq!(RunStopReason::AwaitingApproval, turn.stop_reason);

    // Step 2: Approve (policy allows, but sandbox should block the path)
    let result = runner
        .resolve_approval(ApprovalDecision::approve(), conversational_config(&workspace))
        .await
        .expect("resolve should succeed");

    assert!(
        matches!(result.resolution, ApprovalResolution::Approve),
        "Resolution is Approve (policy allows)"
    );

    // The tool result should be an error (sandbox blocked traversal)
    if let Some(tool_result) = &result.tool_result {
        assert!(
            tool_result.is_error,
            "Tool result should be an error because sandbox blocks traversal: {}",
            tool_result.output
        );
    }
}
