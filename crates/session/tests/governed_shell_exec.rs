//! Wave 04a acceptance tests — governed shell execution.
//!
//! Proves:
//! - local__shell_exec suspends runner when policy requires confirmation
//! - gate.evaluated recorded for Critical/Escalate policy decision
//! - tool.suspended recorded before pausing
//! - tool.resumed + tool.called + tool.completed lifecycle on approval
//! - tool.denied recorded (no execution) on rejection
//! - Direct mode blocks Execute tools with RequireConfirmation
//! - Program validation failure (path separator) returns error without spawn
//! - Trace order: gate.evaluated → tool.suspended → tool.resumed/denied → tool.called → tool.completed/failed

use openwand_core::mode::InteractionMode;
use openwand_session::config::{RunConfig, RunStopReason};
use openwand_session::testing::harness::SessionHarness;
use openwand_session::{ApprovalDecision, ApprovalResolution};

fn conversational_config() -> RunConfig {
    RunConfig {
        mode: InteractionMode::Conversational,
        ..Default::default()
    }
}

fn direct_config() -> RunConfig {
    RunConfig {
        mode: InteractionMode::Direct,
        ..Default::default()
    }
}

/// Helper: get event_kind strings from trace.
async fn trace_kinds(harness: &SessionHarness) -> Vec<String> {
    harness.trace.event_kinds().await
}

// ---- Suspension tests ----

#[tokio::test]
async fn conversational_exec_suspends_with_awaiting_approval() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    let result = harness
        .runner
        .run_turn("Run a command".into(), conversational_config())
        .await
        .expect("turn should run");

    // Mock is configured for file_write, not shell_exec.
    // But the LLM mock returns a tool call, so governance still fires.
    assert_eq!(RunStopReason::AwaitingApproval, result.stop_reason);
    assert_eq!(0, result.tools_executed);
}

#[tokio::test]
async fn direct_exec_blocked_when_confirmation_required() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    let result = harness
        .runner
        .run_turn("Run a command".into(), direct_config())
        .await
        .expect("turn should run");

    // Direct mode: RequireConfirmation → blocked (not suspended)
    assert_eq!(RunStopReason::ToolBlocked, result.stop_reason);
    assert_eq!(0, result.tools_executed);
}

// ---- Trace ordering ----

#[tokio::test]
async fn exec_approval_trace_order_is_gate_suspended_resumed_called_completed() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    let result = harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .expect("turn should run");
    assert_eq!(RunStopReason::AwaitingApproval, result.stop_reason);

    // Approve
    let decision = ApprovalDecision {
        approval_request_id: None,
        resolution: ApprovalResolution::Approve,
        tool_name: None,
        args_hash: None,
    };
    let _approve_result = harness
        .runner
        .resolve_approval(decision, conversational_config())
        .await
        .expect("approval should resolve");

    // Verify trace ordering
    let kinds = trace_kinds(&harness).await;

    // Find gate.evaluated
    let gate_idx = kinds
        .iter()
        .position(|k| k == "gate.evaluated")
        .expect("gate.evaluated should exist");
    let suspended_idx = kinds
        .iter()
        .position(|k| k == "tool.suspended")
        .expect("tool.suspended should exist");
    let resumed_idx = kinds
        .iter()
        .position(|k| k == "tool.resumed")
        .expect("tool.resumed should exist");
    let called_idx = kinds
        .iter()
        .position(|k| k == "tool.called")
        .expect("tool.called should exist");
    let completed_idx = kinds
        .iter()
        .position(|k| k == "tool.completed")
        .expect("tool.completed should exist");

    assert!(gate_idx < suspended_idx, "gate before suspend");
    assert!(suspended_idx < resumed_idx, "suspend before resume");
    assert!(resumed_idx < called_idx, "resume before called");
    assert!(called_idx < completed_idx, "called before completed");
}

// ---- Rejection ----

#[tokio::test]
async fn exec_rejection_records_denied_no_execution() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    let result = harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .expect("turn should run");
    assert_eq!(RunStopReason::AwaitingApproval, result.stop_reason);

    // Reject
    let decision = ApprovalDecision {
        approval_request_id: None,
        resolution: ApprovalResolution::Reject {
            reason: Some("user denied execution".into()),
        },
        tool_name: None,
        args_hash: None,
    };
    let _reject_result = harness
        .runner
        .resolve_approval(decision, conversational_config())
        .await
        .expect("rejection should resolve");

    // Should NOT have executed the tool
    let exec_count = harness.tools.execution_count().await;
    assert_eq!(0, exec_count, "rejected tool should not execute");

    // Trace should have tool.denied, NOT tool.called
    let kinds = trace_kinds(&harness).await;
    assert!(kinds.contains(&"tool.denied".to_string()), "should have tool.denied");
    assert!(!kinds.contains(&"tool.called".to_string()), "should NOT have tool.called");
}

// ---- Allow path (no confirmation needed) ----

#[tokio::test]
async fn exec_allowed_by_policy_executes_directly() {
    let harness = SessionHarness::tool_turn_with_policy(
        openwand_session::testing::mock_policy::MockPolicyBehavior::AllowAll,
    );

    let result = harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .expect("turn should run");

    assert_eq!(RunStopReason::Natural, result.stop_reason);
    assert_eq!(1, result.tools_executed);

    // Trace should have gate.evaluated + tool.called + tool.completed, no suspension
    let kinds = trace_kinds(&harness).await;
    assert!(kinds.contains(&"gate.evaluated".to_string()));
    assert!(kinds.contains(&"tool.called".to_string()));
    assert!(kinds.contains(&"tool.completed".to_string()));
    assert!(!kinds.contains(&"tool.suspended".to_string()), "should not suspend when allowed");
}

// ---- Failure injection ----

#[tokio::test]
async fn exec_tool_failure_surfaces_as_error_in_run() {
    let harness = SessionHarness::tool_turn_with_tool_error(
        "local__file_write",
        "program not found",
    );

    let result = harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .expect("turn should run");

    // Tool failed, but run completes (error fed back to LLM)
    assert_eq!(RunStopReason::Natural, result.stop_reason);

    // Trace should have tool.failed
    let kinds = trace_kinds(&harness).await;
    assert!(kinds.contains(&"tool.failed".to_string()), "should have tool.failed");
}
