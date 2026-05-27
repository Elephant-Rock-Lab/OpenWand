//! Live run DTOs for the UI layer.
//!
//! These represent the streaming state of an active run.
//! Separate from static session DTOs (dto.rs) because
//! run state is transient and high-frequency.

use serde::{Deserialize, Serialize};

/// Overall run status shown in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiRunStatus {
    Idle,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// A single event in the live run stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiRunEvent {
    TextDelta { delta: String },
    ToolCallStarted {
        id: String,
        name: String,
    },
    ToolCallCompleted {
        id: String,
        name: String,
        output: String,
        is_error: bool,
    },
    PhaseChanged {
        phase: String,
        step: u64,
    },
    Completed {
        steps: u64,
        tools: u64,
        reason: String,
    },
    Error {
        message: String,
    },
}

/// Snapshot of the current run state for UI rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiRunState {
    pub status: UiRunStatus,
    pub phase: Option<String>,
    pub step: u64,
    /// Accumulated assistant text for this turn.
    pub streamed_text: String,
    /// Tool call/result events in order.
    pub tool_events: Vec<UiRunEvent>,
    /// Error message if failed.
    pub error: Option<String>,
}

impl Default for UiRunState {
    fn default() -> Self {
        Self {
            status: UiRunStatus::Idle,
            phase: None,
            step: 0,
            streamed_text: String::new(),
            tool_events: Vec::new(),
            error: None,
        }
    }
}

impl UiRunState {
    pub fn new_running() -> Self {
        Self {
            status: UiRunStatus::Running,
            phase: Some("RunStart".into()),
            step: 0,
            streamed_text: String::new(),
            tool_events: Vec::new(),
            error: None,
        }
    }

    /// Apply a run event to this state.
    /// Follows backpressure rules:
    /// - Text deltas: append to streamed_text
    /// - Phase updates: keep latest
    /// - Tool calls/results/errors/completion: never dropped
    pub fn apply(&mut self, event: UiRunEvent) {
        match &event {
            UiRunEvent::TextDelta { delta } => {
                self.streamed_text.push_str(delta);
            }
            UiRunEvent::ToolCallStarted { .. } | UiRunEvent::ToolCallCompleted { .. } => {
                self.tool_events.push(event);
            }
            UiRunEvent::PhaseChanged { phase, step } => {
                self.phase = Some(phase.clone());
                self.step = *step;
            }
            UiRunEvent::Completed { .. } => {
                self.status = UiRunStatus::Completed;
            }
            UiRunEvent::Error { message } => {
                self.status = UiRunStatus::Failed;
                self.error = Some(message.clone());
            }
        }
    }
}
