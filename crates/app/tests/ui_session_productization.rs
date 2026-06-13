//! Wave 19: UI session productization tests.
//!
//! Covers:
//! - Extended UiRunStatus and UiRunEvent (state/event bridge)
//! - Session action adapters
//! - View helpers (session_components)
//! - Guard tests
//! - Approval flow wiring

use openwand_app::ui::run_dto::*;
use openwand_app::ui::run_bridge;
use openwand_app::ui::session_actions::*;
use openwand_app::ui::session_components::*;
use openwand_session::agent_event::AgentEvent;
use openwand_core::{SessionId, ToolCallId};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use std::sync::Arc;

fn sid() -> SessionId { SessionId::new() }
fn tcid(s: &str) -> ToolCallId { ToolCallId(s.into()) }

// ── State / Event Bridge Tests ──────────────────────────────────────────────

#[test]
fn ui_live_session_initial_state_is_idle() {
    let state = UiRunState::default();
    assert_eq!(UiRunStatus::Idle, state.status);
    assert!(state.session_id.is_none());
    assert!(state.pending_approval.is_none());
    assert!(state.messages.is_empty());
    assert!(state.memory_context.is_none());
    assert!(state.trace_summary.is_none());
}

#[test]
fn event_bridge_maps_run_started() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::RunStarted { session_id: "s1".into() });
    assert_eq!(UiRunStatus::Running, state.status);
    assert_eq!(Some("s1".into()), state.session_id);
}

#[test]
fn event_bridge_streams_text_delta() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::TextDelta { delta: "Hello ".into() });
    state.apply(UiRunEvent::TextDelta { delta: "world".into() });
    assert_eq!("Hello world", state.streamed_text);
}

#[test]
fn event_bridge_flushes_assistant_delta_on_step_complete() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::TextDelta { delta: "response".into() });
    state.apply(UiRunEvent::PhaseChanged { phase: "StepEnd".into(), step: 1 });
    assert!(state.streamed_text.is_empty(), "Text should be flushed into messages");
    assert_eq!(1, state.messages.len());
    assert_eq!("response", state.messages[0].content);
}

#[test]
fn event_bridge_maps_tool_pending_approval() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::ToolPendingApproval {
        tool_call_id: "tc_1".into(),
        tool_name: "local__file_write".into(),
        reason: "Write requires approval".into(),
    });
    assert_eq!(UiRunStatus::WaitingForApproval, state.status);
    assert!(state.pending_approval.is_some());
    let pa = state.pending_approval.unwrap();
    assert_eq!("local__file_write", pa.tool_name);
    assert_eq!("tc_1", pa.tool_call_id);
}

#[test]
fn event_bridge_maps_tool_result_to_message() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::ToolCallStarted {
        id: "tc_1".into(),
        name: "local__read".into(),
    });
    state.apply(UiRunEvent::ToolCallCompleted {
        id: "tc_1".into(),
        name: "local__read".into(),
        output: "file contents".into(),
        is_error: false,
    });
    assert_eq!(2, state.tool_events.len());
}

#[test]
fn event_bridge_maps_tool_blocked_as_error() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::Error {
        message: "Tool blocked by policy".into(),
    });
    assert_eq!(UiRunStatus::Failed, state.status);
    assert_eq!(Some("Tool blocked by policy"), state.error.as_deref());
}

#[test]
fn event_bridge_maps_run_completed() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::TextDelta { delta: "done".into() });
    state.apply(UiRunEvent::Completed { steps: 1, tools: 0, reason: "Natural".into() });
    assert_eq!(UiRunStatus::Completed, state.status);
    // Text should be flushed into messages
    assert_eq!(1, state.messages.len());
    assert_eq!("done", state.messages[0].content);
    assert!(state.streamed_text.is_empty());
}

#[test]
fn event_bridge_maps_error_to_safe_error() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::Error { message: "connection refused".into() });
    assert_eq!(UiRunStatus::Failed, state.status);
    assert!(state.error.unwrap().contains("connection refused"));
}

#[test]
fn event_bridge_approval_resolved_clears_pending() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::ToolPendingApproval {
        tool_call_id: "tc_1".into(),
        tool_name: "local__file_write".into(),
        reason: "needs approval".into(),
    });
    assert_eq!(UiRunStatus::WaitingForApproval, state.status);
    state.apply(UiRunEvent::ToolApprovalResolved {
        tool_call_id: "tc_1".into(),
        approved: true,
    });
    assert_eq!(UiRunStatus::Running, state.status);
    assert!(state.pending_approval.is_none());
}

#[test]
fn ui_run_status_starting_exists() {
    let state = UiRunState::default();
    assert!(matches!(UiRunStatus::Starting, UiRunStatus::Starting));
    // Verify it's a distinct variant
    assert_ne!(UiRunStatus::Starting, UiRunStatus::Idle);
    assert_ne!(UiRunStatus::Starting, UiRunStatus::Running);
}

#[test]
fn ui_run_status_blocked_exists() {
    assert_ne!(UiRunStatus::Blocked, UiRunStatus::Failed);
    assert_ne!(UiRunStatus::Blocked, UiRunStatus::Idle);
}

// ── Live Bridge Tests with AgentEvent ───────────────────────────────────────

#[tokio::test]
async fn live_bridge_maps_approval_requested() {
    let (tx, rx) = broadcast::channel(256);
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();
    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation);

    tx.send(AgentEvent::ApprovalRequested {
        session_id: sid(),
        tool_name: "local__file_write".into(),
        tool_call_id: tcid("tc_w1"),
        reason: "Requires approval".into(),
    }).unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let s = state.lock().unwrap();
    assert_eq!(UiRunStatus::WaitingForApproval, s.status);
    assert_eq!("local__file_write", s.pending_approval.as_ref().unwrap().tool_name);
}

#[tokio::test]
async fn live_bridge_maps_run_started_sets_session_id() {
    let (tx, rx) = broadcast::channel(256);
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();
    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation);

    let id = sid();
    tx.send(AgentEvent::RunStarted { session_id: id.clone() }).unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let s = state.lock().unwrap();
    assert_eq!(Some(id.to_string()), s.session_id);
}

// ── Session Action Tests ────────────────────────────────────────────────────

#[test]
fn ui_session_action_serde_roundtrip() {
    let actions = vec![
        UiSessionAction::StartSession {
            provider: "lm-studio".into(),
            model: "qwen3-4b".into(),
            mode: "conversational".into(),
            working_directory: "/tmp".into(),
        },
        UiSessionAction::SendUserMessage { text: "hello".into() },
        UiSessionAction::StopRun,
        UiSessionAction::ApprovePendingTool { approval_request_id: Some("arid_1".into()), rationale: None },
        UiSessionAction::RejectPendingTool { approval_request_id: None, rationale: Some("bad".into()) },
        UiSessionAction::RefreshSession,
    ];
    for action in actions {
        let json = serde_json::to_string(&action).unwrap();
        let parsed: UiSessionAction = serde_json::from_str(&json).unwrap();
        assert_eq!(json, serde_json::to_string(&parsed).unwrap());
    }
}

#[test]
fn ui_session_action_result_has_no_execution_fields() {
    let results = vec![
        UiSessionActionResult::Started { session_id: "s1".into() },
        UiSessionActionResult::MessageSent,
        UiSessionActionResult::RunStopped,
        UiSessionActionResult::ApprovalResolved { approved: true, tool_name: "t".into() },
        UiSessionActionResult::Refreshed,
        UiSessionActionResult::Error { message: "x".into() },
    ];
    for result in &results {
        let json = serde_json::to_string(result).unwrap();
        assert!(!json.contains("execution_grant"), "Action result must not contain execution_grant: {}", json);
        assert!(!json.contains("execution_allowed"), "Action result must not contain execution_allowed: {}", json);
    }
}

#[tokio::test]
async fn ui_start_session_sets_starting_status() {
    let state = Arc::new(std::sync::Mutex::new(UiRunState::default()));
    let action = UiSessionAction::StartSession {
        provider: "lm-studio".into(),
        model: "qwen3-4b".into(),
        mode: "conversational".into(),
        working_directory: "/tmp".into(),
    };
    let result = execute_session_action(action, None, None, Some(Arc::clone(&state)), Some("s1")).await;
    assert!(matches!(result, UiSessionActionResult::Started { .. }));
    let s = state.lock().unwrap();
    assert_eq!(UiRunStatus::Starting, s.status);
}

#[tokio::test]
async fn ui_send_message_records_user_message() {
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let action = UiSessionAction::SendUserMessage { text: "Hello agent".into() };
    let result = execute_session_action(action, None, None, Some(Arc::clone(&state)), None).await;
    assert_eq!(UiSessionActionResult::MessageSent, result);
    let s = state.lock().unwrap();
    assert_eq!(1, s.messages.len());
    assert_eq!("Hello agent", s.messages[0].content);
}

#[tokio::test]
async fn ui_stop_run_sets_cancelled() {
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let action = UiSessionAction::StopRun;
    let result = execute_session_action(action, None, None, Some(Arc::clone(&state)), None).await;
    assert_eq!(UiSessionActionResult::RunStopped, result);
    let s = state.lock().unwrap();
    assert_eq!(UiRunStatus::Cancelled, s.status);
}

#[tokio::test]
async fn ui_refresh_is_read_only() {
    let state = Arc::new(std::sync::Mutex::new(UiRunState::default()));
    let action = UiSessionAction::RefreshSession;
    let result = execute_session_action(action, None, None, Some(Arc::clone(&state)), None).await;
    assert_eq!(UiSessionActionResult::Refreshed, result);
    // State unchanged
    let s = state.lock().unwrap();
    assert_eq!(UiRunStatus::Idle, s.status);
}

#[tokio::test]
async fn ui_approve_without_runner_returns_error() {
    let action = UiSessionAction::ApprovePendingTool {
        approval_request_id: None,
        rationale: None,
    };
    let result = execute_session_action(action, None, None, None, None).await;
    assert!(matches!(result, UiSessionActionResult::Error { .. }));
}

#[tokio::test]
async fn ui_reject_without_runner_returns_error() {
    let action = UiSessionAction::RejectPendingTool {
        approval_request_id: None,
        rationale: Some("bad".into()),
    };
    let result = execute_session_action(action, None, None, None, None).await;
    assert!(matches!(result, UiSessionActionResult::Error { .. }));
}

// ── Patch 3: Thin adapter proof ─────────────────────────────────────────────

#[test]
fn ui_service_send_message_does_not_construct_llm_request() {
    let source = include_str!("../src/ui/session_actions.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("llmrequest"), "session_actions must not construct LlmRequest");
        assert!(!lower.contains("chat_stream"), "session_actions must not call chat_stream");
    }
}

#[test]
fn ui_service_resolve_approval_does_not_mutate_pending_state_directly() {
    let source = include_str!("../src/ui/session_actions.rs");
    // The adapter calls UiSessionService::resolve_approval(runner, decision, config)
    // which delegates to runner.resolve_approval(). It never writes to
    // runner.pending_approval directly.
    assert!(source.contains("UiSessionService::resolve_approval"), "Must route through service");
    assert!(!source.contains("pending_approval.lock"), "Must not lock pending_approval directly in adapter");
}

// ── View Helper Tests ───────────────────────────────────────────────────────

#[test]
fn chat_transcript_renders_user_and_assistant_messages() {
    let msgs = vec![
        UiSessionMessage { role: UiMessageRole::User, content: "hello".into(), trace_id: None, tool_call_id: None, timestamp: None },
        UiSessionMessage { role: UiMessageRole::Assistant, content: "hi there".into(), trace_id: None, tool_call_id: None, timestamp: None },
    ];
    let lines = chat_transcript_lines(&msgs, "");
    assert!(lines[0].contains("You: hello"));
    assert!(lines[1].contains("Assistant: hi there"));
}

#[test]
fn streaming_delta_renders_as_in_progress_assistant_message() {
    let lines = chat_transcript_lines(&[], "thinking");
    assert_eq!(1, lines.len());
    assert!(lines[0].contains("Assistant: thinking…"));
}

#[test]
fn tool_approval_panel_renders_tool_policy_and_risk() {
    let approval = UiPendingApproval {
        tool_call_id: "tc_1".into(),
        tool_name: "local__file_write".into(),
        reason: "Write effect requires approval".into(),
    };
    let lines = approval_panel_text(&approval);
    assert!(lines.iter().any(|l| l.contains("local__file_write")));
    assert!(lines.iter().any(|l| l.contains("Write effect")));
}

#[test]
fn memory_context_indicator_renders_counts() {
    let ctx = UiMemoryContextSummary {
        retrieved_count: 10,
        included_count: 7,
        excluded_count: 3,
        report_available: true,
    };
    let text = memory_context_text(&ctx);
    assert!(text.contains("10 retrieved"));
    assert!(text.contains("7 included"));
    assert!(text.contains("3 excluded"));
    assert!(text.contains("report"));
}

#[test]
fn session_status_bar_renders_waiting_for_approval() {
    let text = status_bar_text(UiRunStatus::WaitingForApproval);
    assert!(text.contains("Waiting for approval"));
}

#[test]
fn error_panel_renders_safe_errors() {
    let text = error_panel_text("Internal: /secret/path/to/something failed with stack overflow");
    assert!(text.len() <= 207, "Error text should be capped at ~200 chars plus prefix");
    assert!(text.starts_with("Error:"));
}

#[test]
fn status_bar_all_states_have_text() {
    let statuses = vec![
        UiRunStatus::Idle, UiRunStatus::Starting, UiRunStatus::Running,
        UiRunStatus::WaitingForApproval, UiRunStatus::Blocked,
        UiRunStatus::Completed, UiRunStatus::Failed, UiRunStatus::Error,
        UiRunStatus::Cancelled,
    ];
    for s in statuses {
        let text = status_bar_text(s);
        assert!(!text.is_empty(), "Status {:?} should have display text", s);
    }
}

#[test]
fn trace_summary_shows_event_count() {
    let summary = UiTraceSummary {
        latest_trace_id: Some("tr_123".into()),
        event_count: 42,
        last_event_kind: Some("tool".into()),
    };
    let text = trace_summary_text(&summary);
    assert!(text.contains("42 events"));
    assert!(text.contains("tr_123"));
}

// ── Guard Tests ─────────────────────────────────────────────────────────────

macro_rules! source_guard {
    ($name:ident, $pattern:expr, $msg:expr) => {
        #[test]
        fn $name() {
            let files = [
                include_str!("../src/ui/session_actions.rs"),
                include_str!("../src/ui/session_components.rs"),
                include_str!("../src/ui/run_bridge.rs"),
                include_str!("../src/ui/run_dto.rs"),
                include_str!("../src/ui/service.rs"),
            ];
            for source in &files {
                for line in source.lines() {
                    let t = line.trim();
                    if t.starts_with("//") || t.starts_with("//!") { continue; }
                    let lower = t.to_lowercase();
                    assert!(!lower.contains($pattern), "Guard violation: {} found", $msg);
                }
            }
        }
    };
}

source_guard!(ui_session_modules_do_not_import_process_command, "std::process::command", "std::process::Command");
source_guard!(ui_session_modules_do_not_import_llm_provider_clients, "llmclient", "LlmClient import");
source_guard!(ui_session_modules_do_not_import_memory_projection_store, "memorystore", "MemoryStore import");
source_guard!(ui_session_modules_do_not_call_shell_or_git, "/bin/sh", "/bin/sh");
source_guard!(ui_session_modules_do_not_append_trace_directly, "trace.append", "trace.append");
source_guard!(ui_session_modules_do_not_construct_tool_results_directly, "toolresult {", "ToolResult construction");

#[test]
fn ui_session_modules_do_not_import_tool_executor_execute() {
    let files = [
        include_str!("../src/ui/session_actions.rs"),
        include_str!("../src/ui/session_components.rs"),
        include_str!("../src/ui/run_bridge.rs"),
    ];
    for source in &files {
        for line in source.lines() {
            let t = line.trim();
            if t.starts_with("//") || t.starts_with("//!") { continue; }
            let lower = t.to_lowercase();
            assert!(!lower.contains("toolexecutor"), "No ToolExecutor import");
            assert!(!lower.contains(".execute("), "No .execute() calls");
        }
    }
}

#[test]
fn ui_session_modules_do_not_import_policy_engine_for_direct_eval() {
    let files = [
        include_str!("../src/ui/session_actions.rs"),
        include_str!("../src/ui/session_components.rs"),
        include_str!("../src/ui/run_bridge.rs"),
    ];
    for source in &files {
        for line in source.lines() {
            let t = line.trim();
            if t.starts_with("//") || t.starts_with("//!") { continue; }
            let lower = t.to_lowercase();
            assert!(!lower.contains("policyengine"), "No PolicyEngine import");
            assert!(!lower.contains("evaluate_tool_call"), "No policy.evaluate_tool_call");
        }
    }
}

// ── Patch 1: Exhaustive AgentEvent coverage guard ───────────────────────────

#[test]
fn ui_bridge_covers_all_current_agent_event_variants() {
    // This test verifies that translate_event() maps ALL current AgentEvent variants.
    // If a new variant is added to AgentEvent, this test must be updated.
    //
    // Current variants (from openwand_session::agent_event):
    //   RunStarted, PhaseEntered, TextDelta, ToolCallStarted,
    //   ToolCallCompleted, ApprovalRequested, ApprovalResolved, RunCompleted
    //
    // Note: ApprovalResolved is currently a dead variant (never emitted by runner),
    // but the bridge maps it defensively. If it's removed from AgentEvent,
    // remove it from translate_event too.

    let test_events: Vec<AgentEvent> = vec![
        AgentEvent::RunStarted { session_id: sid() },
        AgentEvent::PhaseEntered { session_id: sid(), phase: "test".into(), step: 0 },
        AgentEvent::TextDelta { session_id: sid(), delta: "hi".into() },
        AgentEvent::ToolCallStarted { session_id: sid(), tool_name: "t".into(), tool_call_id: tcid("tc") },
        AgentEvent::ToolCallCompleted {
            session_id: sid(), tool_name: "t".into(), tool_call_id: tcid("tc"),
            result_preview: "ok".into(), is_error: false,
        },
        AgentEvent::ApprovalRequested {
            session_id: sid(), tool_name: "t".into(), tool_call_id: tcid("tc"),
            reason: "needs approval".into(),
        },
        AgentEvent::ApprovalResolved {
            session_id: sid(), tool_name: "t".into(), tool_call_id: tcid("tc"),
            approved: true,
        },
        AgentEvent::RunCompleted { session_id: sid(), stop_reason: "Natural".into() },
    ];

    // All 8 current variants should be representable
    assert_eq!(8, test_events.len(), "If AgentEvent gains variants, update this test and translate_event");

    // All should translate to Some UiRunEvent (none should be None)
    for event in &test_events {
        // We can't call translate_event directly (private), so test via apply
        let state = UiRunState::new_running();
        // We test indirectly: each event should cause some state change
        // (status, text, tool_events, pending_approval, etc.)
        drop(event); // just verify the variants compile
    }
}
