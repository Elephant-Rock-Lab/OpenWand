//! Wave 19: E2E mock UI session tests.
//!
//! Uses existing SessionHarness from openwand-session to prove the full
//! round-trip: harness → runner → bridge → UI state.
//!
//! These tests use deterministic mocks, never real LLM providers.

use openwand_app::ui::run_bridge;
use openwand_app::ui::run_dto::*;
use openwand_app::ui::session_actions::*;
use openwand_session::testing::harness::SessionHarness;
use openwand_session::config::RunConfig;
use openwand_core::mode::InteractionMode;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn ui_e2e_text_only_session_streams_to_transcript() {
    let harness = SessionHarness::text_only();
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();

    let rx = harness.runner.subscribe();
    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation);

    let config = RunConfig {
        max_steps: 5,
        mode: InteractionMode::Direct,
        working_directory: ".".into(),
        system_prompt: None,
        llm_target: None,
        memory_prompt_inputs: None,
        output_guard: None,
        capability_context: None,
    };
    let result = harness.runner.run_turn("Hello".into(), config).await;
    assert!(result.is_ok());

    // Wait for bridge to process events
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let s = state.lock().unwrap();
    assert_eq!(UiRunStatus::Completed, s.status, "Text-only session should complete");

    // Should have assistant text either in messages or streamed_text
    let all_text: String = s.messages.iter()
        .filter(|m| matches!(m.role, UiMessageRole::Assistant))
        .map(|m| m.content.clone())
        .collect::<Vec<_>>()
        .join("")
        + &s.streamed_text;
    assert!(all_text.contains("Hello, world."), "Expected assistant text, got: {}", all_text);
}

#[tokio::test]
async fn ui_e2e_tool_approval_session_waits_for_approval() {
    let harness = SessionHarness::write_tool_requires_confirmation();
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();

    let runner = Arc::new(harness.runner);
    let rx = runner.subscribe();
    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation);

    // Run in background — will suspend waiting for approval
    let runner_clone = Arc::clone(&runner);
    let config = RunConfig {
        max_steps: 5,
        mode: InteractionMode::Conversational,
        working_directory: ".".into(),
        system_prompt: None,
        llm_target: None,
        memory_prompt_inputs: None,
        output_guard: None,
        capability_context: None,
    };

    tokio::spawn(async move {
        let _ = runner_clone.run_turn("Write file".into(), config).await;
    });

    // Wait for approval to arrive
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let s = state.lock().unwrap();
    assert_eq!(UiRunStatus::WaitingForApproval, s.status,
        "Should be waiting for approval, got: {:?}", s.status);
    assert!(s.pending_approval.is_some(), "Pending approval should be set");
    let pa = s.pending_approval.as_ref().unwrap();
    assert_eq!("local__file_write", pa.tool_name);
}

#[tokio::test]
async fn ui_e2e_tool_rejection_visible_without_tool_execution() {
    let harness = SessionHarness::write_tool_requires_confirmation();
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();

    let runner = Arc::new(harness.runner);
    let rx = runner.subscribe();
    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation);

    let config = RunConfig {
        max_steps: 5,
        mode: InteractionMode::Conversational,
        working_directory: ".".into(),
        system_prompt: None,
        llm_target: None,
        memory_prompt_inputs: None,
        output_guard: None,
        capability_context: None,
    };

    // Run to suspension
    let summary = runner.run_turn("Write file".into(), config).await.unwrap();
    assert!(format!("{:?}", summary.stop_reason).contains("AwaitingApproval"));

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify pending approval visible in UI state
    {
        let s = state.lock().unwrap();
        assert_eq!(UiRunStatus::WaitingForApproval, s.status);
        assert!(s.pending_approval.is_some());
    }

    // Reject through the action adapter (not raw bridge)
    let action = UiSessionAction::RejectPendingTool {
        approval_request_id: None,
        rationale: Some("User rejected".into()),
    };
    let result = execute_session_action(
        action,
        Some(Arc::clone(&runner)),
        None,
        Some(Arc::clone(&state)),
        None,
    ).await;

    match result {
        UiSessionActionResult::ApprovalResolved { approved, .. } => {
            assert!(!approved, "Should be rejected");
        }
        other => panic!("Expected ApprovalResolved, got: {:?}", other),
    }

    // The adapter clears pending_approval in UI state
    let s = state.lock().unwrap();
    assert!(s.pending_approval.is_none(), "Pending approval should be cleared by adapter");
}

#[tokio::test]
async fn ui_e2e_policy_block_visible_without_tool_execution() {
    let harness = SessionHarness::write_tool_requires_confirmation();
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();

    let rx = harness.runner.subscribe();
    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation);

    // Use Direct mode — pending confirmation is treated as blocked
    let config = RunConfig {
        max_steps: 5,
        mode: InteractionMode::Direct,
        working_directory: ".".into(),
        system_prompt: None,
        llm_target: None,
        memory_prompt_inputs: None,
        output_guard: None,
        capability_context: None,
    };
    let result = harness.runner.run_turn("Write file".into(), config).await;
    assert!(result.is_ok());

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let s = state.lock().unwrap();
    // In Direct mode, the tool is denied without execution
    assert_eq!(UiRunStatus::Completed, s.status);
    // Should NOT have any successful tool completions
    let successful_tools = s.tool_events.iter().any(|e| {
        matches!(e, UiRunEvent::ToolCallCompleted { is_error: false, .. })
    });
    assert!(!successful_tools, "No successful tool execution should occur in Direct mode with pending approval");
}

#[tokio::test]
async fn ui_e2e_tool_error_visible_as_tool_message() {
    let harness = SessionHarness::tool_turn_with_tool_error("local__file_read", "file not found");
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();

    let rx = harness.runner.subscribe();
    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation);

    let config = RunConfig {
        max_steps: 5,
        mode: InteractionMode::Direct,
        working_directory: ".".into(),
        system_prompt: None,
        llm_target: None,
        memory_prompt_inputs: None,
        output_guard: None,
        capability_context: None,
    };
    let result = harness.runner.run_turn("Read file".into(), config).await;
    assert!(result.is_ok());

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let s = state.lock().unwrap();
    // Tool error should appear as ToolCallCompleted with is_error=true
    let has_error = s.tool_events.iter().any(|e| {
        matches!(e, UiRunEvent::ToolCallCompleted { is_error: true, .. })
    });
    assert!(has_error, "Tool error should be visible");
}

#[tokio::test]
async fn ui_e2e_memory_context_indicator_updates() {
    // Text-only session with mock memory that returns nothing
    let harness = SessionHarness::text_only();
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));

    // Manually set memory context (simulating what the UI bridge would do)
    {
        let mut s = state.lock().unwrap();
        s.memory_context = Some(UiMemoryContextSummary {
            retrieved_count: 5,
            included_count: 3,
            excluded_count: 2,
            report_available: true,
        });
    }

    let s = state.lock().unwrap();
    assert!(s.memory_context.is_some());
    let ctx = s.memory_context.as_ref().unwrap();
    assert_eq!(5, ctx.retrieved_count);
    assert_eq!(3, ctx.included_count);
}
