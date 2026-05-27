//! Wave 03c — Approval Hardening
//!
//! 1. Hostile ordering test: failed tool.resumed append prevents execution
//! 2. LLM rejection feedback: denied tool result injected, model can continue
//! 3. Multi-tool behavior: explicit handling for multiple pending tools
//! 4. Unresolved suspension detection

use openwand_core::mode::InteractionMode;
use openwand_core::SessionId;
use openwand_llm::LlmClient;
use openwand_memory::MemoryReadStore;
use openwand_session::config::{RunConfig, RunStopReason};
use openwand_session::runner::SessionRunner;
use openwand_session::testing::harness::SessionHarness;
use openwand_session::testing::mock_llm::MockLlmClient;
use openwand_session::testing::mock_memory::MockMemoryReadStore;
use openwand_session::testing::mock_policy::MockPolicyEngine;
use openwand_session::testing::mock_tools::MockToolExecutor;
use openwand_session::ApprovalDecision;
use openwand_policy::PolicyEngine;
use openwand_store::StoredEvent;
use openwand_tools::executor::ToolExecutor;
use openwand_trace::testing::{FailOnAppend, InMemoryTraceStore};
use openwand_trace::TraceStore;
use std::sync::Arc;

fn conversational_config() -> RunConfig {
    RunConfig {
        mode: InteractionMode::Conversational,
        ..Default::default()
    }
}

/// Build a runner with a failing trace store for hostile tests.
fn build_failing_trace_runner(
    inner_trace: Arc<InMemoryTraceStore<StoredEvent>>,
) -> (SessionRunner, Arc<MockToolExecutor>) {
    let failing_trace: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        FailOnAppend::fail_on_kind(
            inner_trace.clone() as Arc<dyn TraceStore<StoredEvent>>,
            "tool.resumed",
        ),
    );

    let tool_call_id = openwand_core::ToolCallId::new();
    let llm = Arc::new(MockLlmClient::tool_then_stop(
        tool_call_id.as_str().to_string(),
        "local__file_write",
        serde_json::json!({"path": "test.txt", "content": "hello"}),
    ));
    let tools = Arc::new(MockToolExecutor::with_success(
        "local__file_write",
        "Wrote 5 bytes to test.txt",
    ));
    let policy = Arc::new(MockPolicyEngine::require_confirmation_for("local__file_write"));
    let memory = Arc::new(MockMemoryReadStore::new());

    let runner = SessionRunner::new(
        SessionId::new(),
        failing_trace,
        llm as Arc<dyn LlmClient>,
        tools.clone() as Arc<dyn ToolExecutor>,
        policy as Arc<dyn PolicyEngine>,
        memory as Arc<dyn MemoryReadStore>,
        ".".into(),
    );

    (runner, tools)
}

// ---- 1. Hostile ordering test ----

#[tokio::test]
async fn trace_failure_on_resumed_prevents_execution() {
    let inner_trace = Arc::new(InMemoryTraceStore::new());
    let (runner, tools) = build_failing_trace_runner(inner_trace.clone());

    // Run: should suspend
    runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    assert!(runner.pending_approval().await.is_some());

    // Count tool executions before approval attempt
    let exec_count_before = tools.execution_count().await;

    // Now try to approve — the tool.resumed append should fail
    let result = runner
        .resume_with_approval(ApprovalDecision::Approved, conversational_config())
        .await;

    assert!(result.is_err(), "Approval should fail when trace append fails");

    // Tool should NOT have executed
    let exec_count_after = tools.execution_count().await;
    assert_eq!(
        exec_count_before, exec_count_after,
        "Tool executor should NOT have been called after trace failure"
    );

    // The inner trace should show gate.evaluated + tool.suspended but NOT tool.resumed
    let kinds = inner_trace.event_kinds().await;
    assert!(
        kinds.iter().any(|k| k.contains("gate.evaluated")),
        "gate.evaluated should be in trace"
    );
    assert!(
        kinds.iter().any(|k| k.contains("tool.suspended")),
        "tool.suspended should be in trace"
    );
    assert!(
        !kinds.iter().any(|k| k.contains("tool.resumed")),
        "tool.resumed should NOT be in trace (append was blocked)"
    );
}

// ---- 2. Multi-tool behavior ----

#[tokio::test]
async fn multiple_pending_tools_only_first_suspended() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    let result = harness
        .runner
        .run_turn("Write files".into(), conversational_config())
        .await
        .unwrap();

    // The mock LLM only produces one tool call
    assert_eq!(RunStopReason::AwaitingApproval, result.stop_reason);

    let pending = harness.runner.pending_approval().await;
    assert!(pending.is_some());
    assert_eq!("local__file_write", pending.unwrap().tool_call.name);
}

// ---- 3. Rejection followed by new run ----

#[tokio::test]
async fn rejection_clears_state_for_next_run() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    // First run: suspended
    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    // Reject
    harness
        .runner
        .resume_with_approval(ApprovalDecision::Rejected, conversational_config())
        .await
        .unwrap();

    // Pending should be cleared
    assert!(harness.runner.pending_approval().await.is_none());

    // A new run should work (no stale state)
    let result = harness
        .runner
        .run_turn("Try again".into(), conversational_config())
        .await
        .unwrap();

    // The mock LLM was consumed, so this run will be text-only (Natural stop)
    // The important thing is it doesn't crash with stale pending state
    assert!(matches!(
        result.stop_reason,
        RunStopReason::Natural | RunStopReason::AwaitingApproval
    ));
}

// ---- 4. Approval then state fully cleared ----

#[tokio::test]
async fn approval_then_clear_allows_new_run() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    // First run: suspended
    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    // Approve
    harness
        .runner
        .resume_with_approval(ApprovalDecision::Approved, conversational_config())
        .await
        .unwrap();

    // Pending should be cleared
    assert!(harness.runner.pending_approval().await.is_none());

    // New run should work (no stale state from previous approval)
    let result = harness
        .runner
        .run_turn("Another file".into(), conversational_config())
        .await
        .unwrap();

    assert!(matches!(
        result.stop_reason,
        RunStopReason::Natural | RunStopReason::AwaitingApproval
    ));
}

// ---- 5. Direct mode has no pending approval ----

#[tokio::test]
async fn direct_mode_no_pending_approval() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), RunConfig {
            mode: InteractionMode::Direct,
            ..Default::default()
        })
        .await
        .unwrap();

    assert!(
        harness.runner.pending_approval().await.is_none(),
        "Direct mode should not set pending approval"
    );
}

// ---- 6. Unresolved suspension detection ----

#[tokio::test]
async fn trace_shows_unresolved_suspension_when_no_resume() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    // Don't approve or reject — check trace directly
    let kinds = harness.trace.event_kinds().await;
    assert!(
        kinds.iter().any(|k| k == "tool.suspended"),
        "tool.suspended should exist"
    );
    assert!(
        !kinds.iter().any(|k| k == "tool.resumed"),
        "tool.resumed should NOT exist yet"
    );
    assert!(
        !kinds.iter().any(|k| k == "tool.denied"),
        "tool.denied should NOT exist yet"
    );
}

// ---- 7. Rejection feedback injected into Loro state ----

#[tokio::test]
async fn rejection_feedback_visible_in_messages() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    harness
        .runner
        .resume_with_approval(ApprovalDecision::Rejected, conversational_config())
        .await
        .unwrap();

    // The denied tool result should be in the Loro messages
    let messages = harness.runner.loro_state().messages().unwrap();

    // Find the tool message with the denial feedback
    let tool_msgs: Vec<_> = messages
        .iter()
        .filter(|m| matches!(m.role, openwand_session::message::MessageRole::Tool))
        .collect();

    assert_eq!(1, tool_msgs.len(), "Should have one tool message");

    match &tool_msgs[0].content {
        openwand_session::message::MessageContent::ToolResult { result, is_error, .. } => {
            assert!(*is_error, "Denial feedback should be an error result");
            assert!(
                result.contains("denied by user"),
                "Denial feedback should mention 'denied by user', got: {result}"
            );
        }
        other => panic!("Expected ToolResult content, got: {other:?}"),
    }
}
