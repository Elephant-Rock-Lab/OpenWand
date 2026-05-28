//! Wave 03b acceptance tests — approval lifecycle + trace recording.
//!
//! Proves:
//! - RequireConfirmation suspends runner before mutation
//! - gate.evaluated recorded for every gate decision
//! - tool.suspended recorded before pausing
//! - tool.resumed recorded before ToolExecutor on approval
//! - tool.denied recorded (no execution) on rejection
//! - Direct mode treats RequireConfirmation as blocked
//! - Enforceable trace order: gate.evaluated → tool.suspended → tool.resumed/denied

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
async fn conversational_write_suspends_with_awaiting_approval() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    let result = harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .expect("turn should run");

    assert_eq!(RunStopReason::AwaitingApproval, result.stop_reason);
    assert_eq!(0, result.tools_executed);
}

#[tokio::test]
async fn direct_write_blocked_when_confirmation_required() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    let result = harness
        .runner
        .run_turn("Write a file".into(), direct_config())
        .await
        .expect("turn should run");

    assert_eq!(RunStopReason::ToolBlocked, result.stop_reason);
    assert_eq!(0, result.tools_executed);
}

#[tokio::test]
async fn pending_approval_available_after_suspension() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    let pending = harness.runner.pending_approval().await;
    assert!(pending.is_some(), "Pending approval should be set");
    let pending = pending.unwrap();
    assert_eq!("local__file_write", pending.tool_call.name);
}

// ---- Approval path ----

#[tokio::test]
async fn approved_tool_executes_after_resume() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    let result = harness
        .runner
        .resolve_approval(ApprovalDecision::approve(), conversational_config())
        .await
        .expect("resume should succeed");

    assert!(matches!(result.resolution, ApprovalResolution::Approve));
    assert_eq!("local__file_write", result.tool_name);
    assert!(result.tool_result.is_some(), "Tool should have executed");
}

#[tokio::test]
async fn approved_tool_clears_pending_approval() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    harness
        .runner
        .resolve_approval(ApprovalDecision::approve(), conversational_config())
        .await
        .unwrap();

    let pending = harness.runner.pending_approval().await;
    assert!(pending.is_none(), "Pending approval should be cleared");
}

// ---- Rejection path ----

#[tokio::test]
async fn rejected_tool_does_not_execute() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    let result = harness
        .runner
        .resolve_approval(ApprovalDecision::reject(), conversational_config())
        .await
        .expect("resume should succeed");

    assert!(matches!(result.resolution, ApprovalResolution::Reject { .. }));
    assert_eq!("local__file_write", result.tool_name);
    assert!(result.tool_result.is_none(), "Tool should NOT have executed");
}

#[tokio::test]
async fn rejected_tool_clears_pending_approval() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    harness
        .runner
        .resolve_approval(ApprovalDecision::reject(), conversational_config())
        .await
        .unwrap();

    let pending = harness.runner.pending_approval().await;
    assert!(pending.is_none(), "Pending approval should be cleared");
}

// ---- No-pending-approval error ----

#[tokio::test]
async fn resume_without_pending_approval_fails() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    let result = harness
        .runner
        .resolve_approval(ApprovalDecision::approve(), conversational_config())
        .await;

    assert!(result.is_err(), "Resume without pending should fail");
}

// ---- Trace content tests ----

#[tokio::test]
async fn trace_contains_gate_evaluated_for_write() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    let kinds = trace_kinds(&harness).await;
    assert!(
        kinds.iter().any(|k| k == "gate.evaluated"),
        "Trace must contain gate.evaluated, got: {kinds:?}"
    );
}

#[tokio::test]
async fn trace_contains_tool_suspended_for_write() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    let kinds = trace_kinds(&harness).await;
    assert!(
        kinds.iter().any(|k| k == "tool.suspended"),
        "Trace must contain tool.suspended, got: {kinds:?}"
    );
}

// ---- Enforceable trace order ----

#[tokio::test]
async fn approval_trace_shows_suspended_then_resumed() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    harness
        .runner
        .resolve_approval(ApprovalDecision::approve(), conversational_config())
        .await
        .unwrap();

    let kinds = trace_kinds(&harness).await;

    let gate_pos = kinds.iter().position(|k| k == "gate.evaluated");
    let suspended_pos = kinds.iter().position(|k| k == "tool.suspended");
    let resumed_pos = kinds.iter().position(|k| k == "tool.resumed");

    assert!(gate_pos.is_some(), "gate.evaluated missing from: {kinds:?}");
    assert!(suspended_pos.is_some(), "tool.suspended missing");
    assert!(resumed_pos.is_some(), "tool.resumed missing");

    // Enforce: gate.evaluated → tool.suspended → tool.resumed
    assert!(
        gate_pos.unwrap() < suspended_pos.unwrap(),
        "gate.evaluated must precede tool.suspended"
    );
    assert!(
        suspended_pos.unwrap() < resumed_pos.unwrap(),
        "tool.suspended must precede tool.resumed"
    );
}

#[tokio::test]
async fn rejection_trace_shows_suspended_then_denied_no_resumed() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    harness
        .runner
        .resolve_approval(ApprovalDecision::reject(), conversational_config())
        .await
        .unwrap();

    let kinds = trace_kinds(&harness).await;

    assert!(kinds.iter().any(|k| k == "tool.suspended"), "tool.suspended missing");
    assert!(kinds.iter().any(|k| k == "tool.denied"), "tool.denied missing");
    assert!(!kinds.iter().any(|k| k == "tool.resumed"), "tool.resumed should not exist on rejection");

    let suspended_pos = kinds.iter().position(|k| k == "tool.suspended").unwrap();
    let denied_pos = kinds.iter().position(|k| k == "tool.denied").unwrap();
    assert!(
        denied_pos > suspended_pos,
        "tool.denied must come after tool.suspended"
    );
}

#[tokio::test]
async fn direct_mode_trace_shows_denied_no_suspended() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), direct_config())
        .await
        .unwrap();

    let kinds = trace_kinds(&harness).await;

    assert!(kinds.iter().any(|k| k == "gate.evaluated"), "gate.evaluated missing");
    assert!(kinds.iter().any(|k| k == "tool.denied"), "tool.denied missing");
    assert!(!kinds.iter().any(|k| k == "tool.suspended"), "Direct mode should not produce tool.suspended");
    assert!(!kinds.iter().any(|k| k == "tool.resumed"), "Direct mode should not produce tool.resumed");
}
