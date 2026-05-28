//! Tests proving the runner emits tool.deferred events for multi-tool batches.

use openwand_core::mode::InteractionMode;
use openwand_session::config::RunConfig;
use openwand_session::testing::harness::SessionHarness;
use openwand_session::ApprovalDecision;

fn conversational_config() -> RunConfig {
    RunConfig {
        mode: InteractionMode::Conversational,
        ..Default::default()
    }
}

#[tokio::test]
async fn multi_tool_batch_emits_deferred_for_remaining_tools() {
    let harness = SessionHarness::multi_tool_batch();

    harness
        .runner
        .run_turn("Write files A and B, read C".into(), conversational_config())
        .await
        .unwrap();

    // Check the recovery index for deferred events
    let index = harness.runner.approval_recovery_index().await.unwrap();

    // Should have exactly 1 pending (write_A) and 2 deferred (write_B, read_C)
    assert_eq!(1, index.pending.len(), "Should have 1 pending");
    assert_eq!(
        "local__write_A", index.pending[0].tool_name,
        "First tool should be pending"
    );

    assert_eq!(2, index.deferred.len(), "Should have 2 deferred tools");

    let deferred_names: Vec<&str> = index.deferred.iter().map(|d| d.tool_name.as_str()).collect();
    assert!(
        deferred_names.contains(&"local__write_B"),
        "write_B should be deferred, got: {deferred_names:?}"
    );
    assert!(
        deferred_names.contains(&"local__read_C"),
        "read_C should be deferred, got: {deferred_names:?}"
    );

    // Each deferred event should reference the blocking approval
    let blocking_arid = &index.pending[0].context.approval_request_id;
    for deferred in &index.deferred {
        assert_eq!(
            Some(blocking_arid.clone()),
            deferred.blocked_by_approval_request_id,
            "Deferred tool should reference blocking approval_request_id"
        );
    }
}

#[tokio::test]
async fn deferred_events_survive_in_trace_after_approval() {
    let harness = SessionHarness::multi_tool_batch();

    harness
        .runner
        .run_turn("Write files".into(), conversational_config())
        .await
        .unwrap();

    // Approve write_A
    harness
        .runner
        .resume_with_approval(ApprovalDecision::Approved, conversational_config())
        .await
        .unwrap();

    // The deferred events should still be in trace (they're audit records)
    let index = harness.runner.approval_recovery_index().await.unwrap();

    // write_A is no longer pending (it was approved and executed)
    assert!(index.pending.is_empty(), "No pending after approval");

    // Deferred events should still be there (they're historical, not pending)
    assert_eq!(2, index.deferred.len(), "Deferred events should persist in trace");
}
