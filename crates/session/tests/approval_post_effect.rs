//! Wave 70A (Patch 3): Approval post-effect E2E validation.
//!
//! Proves that approval causes real tool execution with correct trace ordering:
//!   gate.evaluated → tool.suspended → tool.resumed → tool.called → tool.completed
//!
//! Also proves:
//!   - tool was actually executed (not just mock success)
//!   - tool.completed (not tool.failed) appears in trace
//!   - tool.resumed appears BEFORE tool.called in trace ordering

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

/// Patch 3: Approve a write request, verify tool executed, verify trace ordering.
#[tokio::test]
async fn approval_post_effect_tool_executes_with_correct_trace_order() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    // Step 1: Run turn — should suspend
    let turn = harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .expect("turn should run");
    assert_eq!(RunStopReason::AwaitingApproval, turn.stop_reason);

    // Step 2: Approve
    let result = harness
        .runner
        .resolve_approval(ApprovalDecision::approve(), conversational_config())
        .await
        .expect("resume should succeed");

    // Step 3: Verify approval resolution
    assert!(
        matches!(result.resolution, ApprovalResolution::Approve),
        "resolution should be Approve"
    );
    assert_eq!("local__file_write", result.tool_name);
    assert!(result.tool_result.is_some(), "Tool should have executed");

    // Step 4: Verify tool was actually called (not just a placeholder)
    let calls = harness.tools.calls().await;
    assert_eq!(1, calls.len(), "Tool should have been called exactly once");
    assert_eq!("local__file_write", calls[0].name);

    // Step 5: Verify trace ordering
    let kinds = harness.trace.event_kinds().await;

    // Find positions
    let gate_pos = kinds.iter().position(|k| k == "gate.evaluated");
    let suspended_pos = kinds.iter().position(|k| k == "tool.suspended");
    let resumed_pos = kinds.iter().position(|k| k == "tool.resumed");
    let called_pos = kinds.iter().position(|k| k == "tool.called");
    let completed_pos = kinds.iter().position(|k| k == "tool.completed");
    let failed_pos = kinds.iter().position(|k| k == "tool.failed");

    assert!(gate_pos.is_some(), "gate.evaluated should be in trace");
    assert!(suspended_pos.is_some(), "tool.suspended should be in trace");
    assert!(resumed_pos.is_some(), "tool.resumed should be in trace");
    assert!(called_pos.is_some(), "tool.called should be in trace");

    // tool.resumed BEFORE tool.called
    assert!(
        resumed_pos.unwrap() < called_pos.unwrap(),
        "tool.resumed must appear before tool.called in trace: resumed={} called={}",
        resumed_pos.unwrap(),
        called_pos.unwrap()
    );

    // tool.completed (not tool.failed)
    assert!(
        completed_pos.is_some(),
        "tool.completed should be in trace"
    );
    assert!(
        failed_pos.is_none(),
        "tool.failed should NOT be in trace — tool succeeded"
    );

    // Full ordering: gate.evaluated → tool.suspended → tool.resumed → tool.called → tool.completed
    assert!(gate_pos.unwrap() < suspended_pos.unwrap());
    assert!(suspended_pos.unwrap() < resumed_pos.unwrap());
    assert!(called_pos.unwrap() < completed_pos.unwrap());
}

/// Patch 3 variant: rejection does NOT produce tool.called or tool.completed.
#[tokio::test]
async fn rejection_does_not_execute_tool() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .expect("turn should run");

    let result = harness
        .runner
        .resolve_approval(ApprovalDecision::reject(), conversational_config())
        .await
        .expect("resolve should succeed");

    assert!(matches!(result.resolution, ApprovalResolution::Reject { .. }));

    // Tool should NOT have been called
    let calls = harness.tools.calls().await;
    assert_eq!(0, calls.len(), "Tool should NOT have been called on rejection");

    // Trace should NOT have tool.called or tool.completed
    let kinds = harness.trace.event_kinds().await;
    assert!(
        !kinds.iter().any(|k| k == "tool.called"),
        "tool.called should NOT appear after rejection"
    );
    assert!(
        !kinds.iter().any(|k| k == "tool.completed"),
        "tool.completed should NOT appear after rejection"
    );
    assert!(
        kinds.iter().any(|k| k == "tool.denied"),
        "tool.denied SHOULD appear after rejection"
    );
}
