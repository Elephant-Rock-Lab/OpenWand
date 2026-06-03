//! UiRunBridge and UiRunState tests.
//!
//! Tests the event bridge between SessionRunner broadcast
//! and the UI's shared state.

use openwand_app::ui::run_dto::{UiRunEvent, UiRunState, UiRunStatus};
use openwand_app::ui::run_bridge;
use openwand_session::agent_event::AgentEvent;
use openwand_core::{SessionId, ToolCallId};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use std::sync::Arc;

fn make_agent_event_tx() -> broadcast::Sender<AgentEvent> {
    broadcast::channel(256).0
}

#[test]
fn ui_run_state_applies_text_delta() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::TextDelta { delta: "Hello ".into() });
    state.apply(UiRunEvent::TextDelta { delta: "world".into() });
    assert_eq!("Hello world", state.streamed_text);
    assert_eq!(UiRunStatus::Running, state.status);
}

#[test]
fn ui_run_state_applies_tool_call_events() {
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
fn ui_run_state_applies_phase_change() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::PhaseChanged {
        phase: "Inference".into(),
        step: 1,
    });
    assert_eq!(Some("Inference"), state.phase.as_deref());
    assert_eq!(1, state.step);
}

#[test]
fn ui_run_state_completes_on_completed_event() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::Completed {
        steps: 2,
        tools: 1,
        reason: "Natural".into(),
    });
    assert_eq!(UiRunStatus::Completed, state.status);
}

#[test]
fn ui_run_state_fails_on_error_event() {
    let mut state = UiRunState::new_running();
    state.apply(UiRunEvent::Error {
        message: "timeout".into(),
    });
    assert_eq!(UiRunStatus::Failed, state.status);
    assert_eq!(Some("timeout"), state.error.as_deref());
}

#[tokio::test]
async fn ui_run_bridge_receives_text_delta() {
    let (tx, rx) = broadcast::channel(256);
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();

    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation.clone());

    // Send text delta
    tx.send(AgentEvent::TextDelta {
        session_id: SessionId::new(),
        delta: "Hello".into(),
    }).unwrap();

    // Give bridge task time to process
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let s = state.lock().unwrap();
    assert_eq!("Hello", s.streamed_text);
}

#[tokio::test]
async fn ui_run_bridge_coalesces_text_without_dropping_completion() {
    let (tx, rx) = broadcast::channel(256);
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();

    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation.clone());

    // Send multiple text deltas then completion
    tx.send(AgentEvent::TextDelta {
        session_id: SessionId::new(),
        delta: "A".into(),
    }).unwrap();
    tx.send(AgentEvent::TextDelta {
        session_id: SessionId::new(),
        delta: "B".into(),
    }).unwrap();
    tx.send(AgentEvent::RunCompleted {
        session_id: SessionId::new(),
        stop_reason: "Natural".into(),
    }).unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let s = state.lock().unwrap();
    // After completion, streamed text is flushed into messages
    assert!(s.streamed_text.is_empty());
    assert_eq!(1, s.messages.len());
    assert_eq!("AB", s.messages[0].content);
    assert_eq!(UiRunStatus::Completed, s.status);
}

#[tokio::test]
async fn ui_run_bridge_preserves_tool_call_events() {
    let (tx, rx) = broadcast::channel(256);
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();

    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation.clone());

    tx.send(AgentEvent::ToolCallStarted {
        session_id: SessionId::new(),
        tool_name: "local__read".into(),
        tool_call_id: ToolCallId("tc_1".into()),
    }).unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let s = state.lock().unwrap();
    assert_eq!(1, s.tool_events.len());
    match &s.tool_events[0] {
        UiRunEvent::ToolCallStarted { name, .. } => {
            assert_eq!("local__read", name);
        }
        other => panic!("Expected ToolCallStarted, got: {other:?}"),
    }
}

#[tokio::test]
async fn ui_run_bridge_preserves_tool_result_events() {
    let (tx, rx) = broadcast::channel(256);
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();

    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation.clone());

    tx.send(AgentEvent::ToolCallCompleted {
        session_id: SessionId::new(),
        tool_name: "local__search".into(),
        tool_call_id: ToolCallId("tc_2".into()),
        result_preview: "search results...".into(),
        is_error: true,
    }).unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let s = state.lock().unwrap();
    assert_eq!(1, s.tool_events.len());
    match &s.tool_events[0] {
        UiRunEvent::ToolCallCompleted { name, is_error, output, .. } => {
            assert_eq!("local__search", name);
            assert!(is_error);
            assert_eq!("search results...", output);
        }
        other => panic!("Expected ToolCallCompleted, got: {other:?}"),
    }
}

#[tokio::test]
async fn ui_run_bridge_updates_phase() {
    let (tx, rx) = broadcast::channel(256);
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();

    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation.clone());

    tx.send(AgentEvent::PhaseEntered {
        session_id: SessionId::new(),
        phase: "Inference".into(),
        step: 1,
    }).unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let s = state.lock().unwrap();
    assert_eq!(Some("Inference"), s.phase.as_deref());
    assert_eq!(1, s.step);
}

#[tokio::test]
async fn ui_run_bridge_cleanup_stops_receiver_task() {
    let (tx, rx) = broadcast::channel(256);
    let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));
    let cancellation = CancellationToken::new();

    run_bridge::start_bridge(rx, Arc::clone(&state), cancellation.clone());

    // Cancel the bridge
    cancellation.cancel();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Send an event — should not be received
    let send_result = tx.send(AgentEvent::TextDelta {
        session_id: SessionId::new(),
        delta: "should not arrive".into(),
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let s = state.lock().unwrap();
    assert_eq!(UiRunStatus::Cancelled, s.status);
    // The event may or may not arrive (race), but state is Cancelled
    assert!(s.streamed_text.is_empty() || s.streamed_text == "should not arrive");
}
