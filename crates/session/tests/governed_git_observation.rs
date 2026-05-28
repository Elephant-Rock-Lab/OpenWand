//! Wave 04b session tests — governed git observation lifecycle.
//!
//! Proves git observation tools close the tool lifecycle and don't
//! require approval beyond Inform when policy allows.

use openwand_core::mode::InteractionMode;
use openwand_session::config::RunConfig;
use openwand_session::testing::harness::SessionHarness;

fn conversational_config() -> RunConfig {
    RunConfig {
        mode: InteractionMode::Conversational,
        ..Default::default()
    }
}

/// Helper: get event_kind strings from trace.
async fn trace_kinds(harness: &SessionHarness) -> Vec<String> {
    harness.trace.event_kinds().await
}

#[tokio::test]
async fn git_observation_records_closed_lifecycle() {
    // Use an AllowAll policy — git observation is allowed directly
    let harness = SessionHarness::tool_turn_with_policy(
        openwand_session::testing::mock_policy::MockPolicyBehavior::AllowAll,
    );

    let result = harness
        .runner
        .run_turn("Check git status".into(), conversational_config())
        .await
        .expect("turn should run");

    // Should complete naturally (tool executed)
    assert_eq!(
        openwand_session::config::RunStopReason::Natural,
        result.stop_reason
    );

    // Lifecycle: gate.evaluated → tool.called → tool.completed
    let kinds = trace_kinds(&harness).await;
    assert!(kinds.contains(&"gate.evaluated".to_string()), "missing gate.evaluated");
    assert!(kinds.contains(&"tool.called".to_string()), "missing tool.called");
    assert!(kinds.contains(&"tool.completed".to_string()), "missing tool.completed");
    // Should NOT suspend
    assert!(!kinds.contains(&"tool.suspended".to_string()), "should not suspend");
}

#[tokio::test]
async fn git_status_non_repo_records_failed_lifecycle() {
    // Use a mock tool that returns an error (simulating non-repo)
    let harness = SessionHarness::tool_turn_with_tool_error(
        "local__git_status",
        "working directory is not inside a git worktree",
    );

    let result = harness
        .runner
        .run_turn("Check git status".into(), conversational_config())
        .await
        .expect("turn should run");

    // Run completes — error fed back to LLM, not session crash
    assert_eq!(
        openwand_session::config::RunStopReason::Natural,
        result.stop_reason
    );

    // Lifecycle: gate.evaluated → tool.called → tool.failed
    let kinds = trace_kinds(&harness).await;
    assert!(kinds.contains(&"tool.failed".to_string()), "should have tool.failed");
    // Should NOT have tool.completed
    assert!(!kinds.contains(&"tool.completed".to_string()), "should not have tool.completed for failure");
}
