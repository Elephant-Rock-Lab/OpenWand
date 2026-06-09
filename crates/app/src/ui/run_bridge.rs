//! Bridge between SessionRunner's broadcast::Sender<AgentEvent>
//! and the Dioxus UI's GlobalSignal<UiRunState>.
//!
//! The bridge runs as a spawned tokio task that:
//! 1. Receives AgentEvents from the session runner
//! 2. Translates them to UiRunEvents
//! 3. Applies them to the shared UiRunState (GlobalSignal)
//!
//! Cleanup: when the CancellationToken is fired, the receiver task stops.
//! No leaked tasks.

use crate::ui::run_dto::{UiRunEvent, UiRunState, UiRunStatus};
use openwand_session::agent_event::AgentEvent;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// Starts the bridge task. Returns a CancellationToken to stop it.
///
/// The caller writes `running` into `state_cell` before calling this.
/// The bridge reads from `rx` until cancelled or the sender drops.
pub fn start_bridge(
    mut rx: broadcast::Receiver<AgentEvent>,
    state: std::sync::Arc<std::sync::Mutex<UiRunState>>,
    cancellation: CancellationToken,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(agent_event) => {
                            if let Some(event) = translate_event(&agent_event) {
                                let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                                state.apply(event);
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(count)) => {
                            // Some events were dropped. Record a warning
                            // but do not crash the bridge.
                            let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                            state.error = Some(format!(
                                "Warning: {count} events lagged (UI may have missed text deltas)"
                            ));
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            // Sender dropped — run is done.
                            let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                            if state.status == UiRunStatus::Running {
                                state.status = UiRunStatus::Completed;
                            }
                            // Don't overwrite WaitingForApproval
                            break;
                        }
                    }
                }
                _ = cancellation.cancelled() => {
                    let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                    state.status = UiRunStatus::Cancelled;
                    break;
                }
            }
        }
    });
}

/// Translate an AgentEvent from the session runner into a UiRunEvent.
/// Returns None for events the UI doesn't need to render.
///
/// Maps actual emitted variants from openwand-session only.
/// See `ui_bridge_covers_all_current_agent_event_variants` guard test.
fn translate_event(event: &AgentEvent) -> Option<UiRunEvent> {
    match event {
        AgentEvent::RunStarted { session_id, .. } => Some(UiRunEvent::RunStarted {
            session_id: session_id.to_string(),
        }),
        AgentEvent::TextDelta { delta, .. } => Some(UiRunEvent::TextDelta {
            delta: delta.clone(),
        }),
        AgentEvent::ToolCallStarted {
            tool_call_id,
            tool_name,
            ..
        } => Some(UiRunEvent::ToolCallStarted {
            id: tool_call_id.0.clone(),
            name: tool_name.clone(),
        }),
        AgentEvent::ToolCallCompleted {
            tool_call_id,
            tool_name,
            result_preview,
            is_error,
            ..
        } => Some(UiRunEvent::ToolCallCompleted {
            id: tool_call_id.0.clone(),
            name: tool_name.clone(),
            output: result_preview.clone(),
            is_error: *is_error,
        }),
        AgentEvent::ApprovalRequested {
            tool_call_id,
            tool_name,
            reason,
            ..
        } => Some(UiRunEvent::ToolPendingApproval {
            tool_call_id: tool_call_id.0.clone(),
            tool_name: tool_name.clone(),
            reason: reason.clone(),
        }),
        AgentEvent::ApprovalResolved {
            tool_call_id,
            approved,
            ..
        } => Some(UiRunEvent::ToolApprovalResolved {
            tool_call_id: tool_call_id.0.clone(),
            approved: *approved,
        }),
        AgentEvent::PhaseEntered { phase, step, .. } => Some(UiRunEvent::PhaseChanged {
            phase: phase.clone(),
            step: *step,
        }),
        AgentEvent::RunCompleted {
            stop_reason, ..
        } => Some(UiRunEvent::Completed {
            steps: 0,
            tools: 0,
            reason: stop_reason.clone(),
        }),
    }
}
