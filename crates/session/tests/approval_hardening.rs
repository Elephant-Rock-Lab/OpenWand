//! Wave 03c — Approval Hardening
//!
//! 1. Hostile ordering test: failed tool.resumed append prevents execution
//! 2. LLM rejection feedback: denied tool result injected, model can continue
//! 3. Multi-tool batch behavior: only first pending tool suspends
//! 4. Unresolved suspension detection and query helper
//! 5. State cleanup after approval/rejection

use openwand_core::mode::InteractionMode;
use openwand_core::SessionId;
use openwand_llm::LlmClient;
use openwand_llm::LlmMessage;
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

// ==========================================================================
// 1. Hostile ordering test
// ==========================================================================

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

    // CHANGE 3: Also assert no execution events leaked
    assert!(
        !kinds.iter().any(|k| k == "tool.called"),
        "tool.called should NOT exist — no execution happened"
    );
    assert!(
        !kinds.iter().any(|k| k == "tool.completed"),
        "tool.completed should NOT exist — no execution happened"
    );
}

// ==========================================================================
// 2. Multi-tool batch behavior
// ==========================================================================

#[tokio::test]
async fn multi_tool_batch_only_first_suspends() {
    let harness = SessionHarness::multi_tool_batch();

    let result = harness
        .runner
        .run_turn("Write files A and B, read C".into(), conversational_config())
        .await
        .unwrap();

    assert_eq!(
        RunStopReason::AwaitingApproval,
        result.stop_reason,
        "Should stop for approval when first tool requires it"
    );

    // Only write_A should be pending
    let pending = harness.runner.pending_approval().await;
    assert!(pending.is_some(), "Should have a pending approval");
    let pending = pending.unwrap();
    assert_eq!(
        "local__write_A", pending.tool_call.name,
        "First confirmation-requiring tool should be pending"
    );

    // No tool execution happened
    assert_eq!(
        0,
        harness.tools.execution_count().await,
        "No tools should execute during suspension"
    );

    // Trace should have exactly 1 tool.suspended
    let kinds = harness.trace.event_kinds().await;
    let suspended_count = kinds.iter().filter(|k| *k == "tool.suspended").count();
    assert_eq!(
        1, suspended_count,
        "Should have exactly 1 tool.suspended event"
    );
}

#[tokio::test]
async fn multi_tool_batch_allowed_tools_do_not_execute_during_suspension() {
    let harness = SessionHarness::multi_tool_batch();

    harness
        .runner
        .run_turn("Write files A and B, read C".into(), conversational_config())
        .await
        .unwrap();

    // read_C is allowed by policy but should NOT execute — batch is frozen
    assert_eq!(
        0,
        harness.tools.execution_count().await,
        "Allowed tools must not execute while batch is suspended"
    );

    // No tool.called or tool.completed events for any tool
    let kinds = harness.trace.event_kinds().await;
    assert!(
        !kinds.iter().any(|k| k == "tool.called"),
        "tool.called should NOT exist — no execution during suspension"
    );
    assert!(
        !kinds.iter().any(|k| k == "tool.completed"),
        "tool.completed should NOT exist — no execution during suspension"
    );
}

#[tokio::test]
async fn multi_tool_batch_after_approval_only_approved_tool_executes() {
    let harness = SessionHarness::multi_tool_batch();

    harness
        .runner
        .run_turn("Write files A and B, read C".into(), conversational_config())
        .await
        .unwrap();

    // Approve write_A
    let approval_result = harness
        .runner
        .resume_with_approval(ApprovalDecision::Approved, conversational_config())
        .await
        .unwrap();

    assert_eq!("local__write_A", approval_result.tool_name);

    // Only write_A should have executed
    assert_eq!(
        1,
        harness.tools.execution_count().await,
        "Only the approved tool should execute"
    );

    let calls = harness.tools.calls().await;
    assert_eq!(1, calls.len());
    assert_eq!("local__write_A", calls[0].name);

    // write_B and read_C should have no execution events
    // Note: the runner doesn't record tool.called/completed in trace,
    // so we verify via mock executor calls instead.
    let calls = harness.tools.calls().await;
    assert_eq!(1, calls.len(), "Only one tool should have been executed");
    assert_eq!("local__write_A", calls[0].name, "Only write_A should execute");
}

#[tokio::test]
async fn multi_tool_batch_deferred_tools_not_in_pending() {
    let harness = SessionHarness::multi_tool_batch();

    harness
        .runner
        .run_turn("Write files A and B, read C".into(), conversational_config())
        .await
        .unwrap();

    let pending = harness.runner.pending_approval().await.unwrap();

    // Only one pending tool, and it's write_A (not write_B or read_C)
    assert_eq!("local__write_A", pending.tool_call.name);

    // There is no mechanism to resume write_B — it's genuinely deferred
    // After approving A, pending is cleared
    harness
        .runner
        .resume_with_approval(ApprovalDecision::Approved, conversational_config())
        .await
        .unwrap();

    assert!(
        harness.runner.pending_approval().await.is_none(),
        "Pending should be cleared after approval"
    );
}

// ==========================================================================
// 3. Model continuation after rejection
// ==========================================================================

#[tokio::test]
async fn denied_approval_reaches_llm_as_tool_error_and_model_continues() {
    let harness = SessionHarness::write_tool_then_text_after_denial();

    // Turn 1: LLM requests write tool → suspended
    let result = harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    assert_eq!(
        RunStopReason::AwaitingApproval,
        result.stop_reason,
        "First turn should suspend for approval"
    );

    // Reject the tool
    harness
        .runner
        .resume_with_approval(ApprovalDecision::Rejected, conversational_config())
        .await
        .unwrap();

    // Turn 2: LLM should see the denied tool result and respond with text
    let result = harness
        .runner
        .run_turn("OK what else can you do?".into(), conversational_config())
        .await
        .unwrap();

    assert_eq!(
        RunStopReason::Natural,
        result.stop_reason,
        "Second turn should complete naturally after LLM adjusts"
    );

    // Verify the second LLM request contains the denied tool result
    let requests = harness.llm.requests().await;
    assert_eq!(2, requests.len(), "LLM should have been called twice");

    let second_request = &requests[1];
    let tool_messages: Vec<_> = second_request
        .messages
        .iter()
        .filter(|m| matches!(m, LlmMessage::Tool { is_error: true, .. }))
        .collect();

    assert_eq!(
        1,
        tool_messages.len(),
        "Second LLM request should contain the denied tool result"
    );

    match &tool_messages[0] {
        LlmMessage::Tool {
            tool_call_id,
            content,
            is_error,
        } => {
            assert!(*is_error, "Denied tool result should be an error");
            assert!(
                content.to_lowercase().contains("denied"),
                "Denied tool result should mention 'denied', got: {content}"
            );
            // tool_call_id should match the original tool call
            assert!(
                !tool_call_id.is_empty(),
                "tool_call_id should be present in denied result"
            );
        }
        _ => panic!("Expected Tool message"),
    }

    // Trace should show: gate.evaluated → tool.suspended → tool.denied
    // Then second turn completes naturally
    let kinds = harness.trace.event_kinds().await;
    assert!(
        kinds.iter().any(|k| k == "tool.suspended"),
        "Trace should show tool.suspended"
    );
    assert!(
        kinds.iter().any(|k| k == "tool.denied"),
        "Trace should show tool.denied"
    );
}

// ==========================================================================
// 4. State cleanup and basic behavior
// ==========================================================================

#[tokio::test]
async fn rejection_clears_state_for_next_run() {
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

    assert!(harness.runner.pending_approval().await.is_none());

    let result = harness
        .runner
        .run_turn("Try again".into(), conversational_config())
        .await
        .unwrap();

    assert!(matches!(
        result.stop_reason,
        RunStopReason::Natural | RunStopReason::AwaitingApproval
    ));
}

#[tokio::test]
async fn approval_then_clear_allows_new_run() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    harness
        .runner
        .resume_with_approval(ApprovalDecision::Approved, conversational_config())
        .await
        .unwrap();

    assert!(harness.runner.pending_approval().await.is_none());

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

    let messages = harness.runner.loro_state().messages().unwrap();

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

// ==========================================================================
// 5. Unresolved suspension helper
// ==========================================================================

#[tokio::test]
async fn unresolved_suspensions_returns_suspended_without_resolution() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    // Run suspends but don't approve or reject
    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    let unresolved = harness.runner.unresolved_suspensions().await.unwrap();
    assert_eq!(1, unresolved.len(), "Should find one unresolved suspension");
    assert_eq!(
        "local__file_write", unresolved[0].tool_name,
        "Tool name should match"
    );
}

#[tokio::test]
async fn unresolved_suspensions_excludes_resumed_tools() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    // Approve → tool.resumed is recorded
    harness
        .runner
        .resume_with_approval(ApprovalDecision::Approved, conversational_config())
        .await
        .unwrap();

    let unresolved = harness.runner.unresolved_suspensions().await.unwrap();
    assert!(
        unresolved.is_empty(),
        "Approved tools should not be unresolved, got: {:?}",
        unresolved
    );
}

#[tokio::test]
async fn unresolved_suspensions_excludes_denied_tools() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    // Reject → tool.denied is recorded
    harness
        .runner
        .resume_with_approval(ApprovalDecision::Rejected, conversational_config())
        .await
        .unwrap();

    let unresolved = harness.runner.unresolved_suspensions().await.unwrap();
    assert!(
        unresolved.is_empty(),
        "Denied tools should not be unresolved, got: {:?}",
        unresolved
    );
}

#[tokio::test]
async fn unresolved_suspensions_empty_when_no_suspensions() {
    let harness = SessionHarness::text_only();

    harness
        .runner
        .run_turn("Hello".into(), conversational_config())
        .await
        .unwrap();

    let unresolved = harness.runner.unresolved_suspensions().await.unwrap();
    assert!(
        unresolved.is_empty(),
        "No suspensions in text-only run, got: {:?}",
        unresolved
    );
}
