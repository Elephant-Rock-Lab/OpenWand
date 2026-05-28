//! Wave 03f-hardening: Trace failure injection tests.
//!
//! Proves that tool.called is the execution boundary and terminal
//! event failure surfaces visibly.

use openwand_core::mode::InteractionMode;
use openwand_session::config::RunConfig;
use openwand_session::testing::harness::SessionHarness;
use openwand_session::ApprovalDecision;

fn direct_config() -> RunConfig {
    RunConfig {
        mode: InteractionMode::Direct,
        ..Default::default()
    }
}

fn _conversational_config() -> RunConfig {
    RunConfig {
        mode: InteractionMode::Conversational,
        ..Default::default()
    }
}

#[tokio::test]
async fn tool_called_append_failure_prevents_execution() {
    // Build a harness where trace append fails on "tool.called"
    let base = SessionHarness::read_file_tool_turn();
    let harness = SessionHarness::with_failing_trace(base, "tool.called");

    // Run should fail — tool.called append is the execution boundary
    let result = harness
        .runner
        .run_turn("Read README.md".into(), direct_config())
        .await;

    assert!(result.is_err(), "Run must fail when tool.called append fails");

    // Tool must NOT have executed — tool.called is before execute
    assert_eq!(
        0,
        harness.tools.execution_count().await,
        "Tool must not execute when tool.called append fails"
    );
}

#[tokio::test]
async fn tool_terminal_append_failure_surfaces_as_error() {
    // Build a harness where trace append fails on "tool.completed"
    let base = SessionHarness::read_file_tool_turn();
    let harness = SessionHarness::with_failing_trace(base, "tool.completed");

    // Run should fail — terminal event is mandatory after execution
    let result = harness
        .runner
        .run_turn("Read README.md".into(), direct_config())
        .await;

    assert!(
        result.is_err(),
        "Run must fail when terminal event append fails"
    );

    // Tool DID execute (called succeeded, completed failed)
    assert_eq!(
        1,
        harness.tools.execution_count().await,
        "Tool should have executed once (called succeeded)"
    );
}
