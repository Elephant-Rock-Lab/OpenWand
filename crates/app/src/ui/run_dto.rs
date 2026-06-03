//! Live run DTOs for the UI layer.
//!
//! These represent the streaming state of an active run.
//! Separate from static session DTOs (dto.rs) because
//! run state is transient and high-frequency.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Overall run status shown in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiRunStatus {
    Idle,
    Starting,
    Running,
    WaitingForApproval,
    Blocked,
    Completed,
    Failed,
    Error,
    Cancelled,
}

/// A single event in the live run stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiRunEvent {
    RunStarted {
        session_id: String,
    },
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
    ToolPendingApproval {
        tool_call_id: String,
        tool_name: String,
        reason: String,
    },
    ToolApprovalResolved {
        tool_call_id: String,
        approved: bool,
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

/// Pending tool approval shown in the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiPendingApproval {
    pub tool_call_id: String,
    pub tool_name: String,
    pub reason: String,
}

/// Memory context summary for the session status area.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiMemoryContextSummary {
    pub retrieved_count: usize,
    pub included_count: usize,
    pub excluded_count: usize,
    pub report_available: bool,
}

/// Trace/persistence summary for the session status area.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiTraceSummary {
    pub latest_trace_id: Option<String>,
    pub event_count: usize,
    pub last_event_kind: Option<String>,
}

/// A message in the session transcript.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiSessionMessage {
    pub role: UiMessageRole,
    pub content: String,
    pub trace_id: Option<String>,
    pub tool_call_id: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
}

/// Message role for session transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiMessageRole {
    User,
    Assistant,
    Tool,
    System,
}

/// Snapshot of the current run state for UI rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiRunState {
    pub status: UiRunStatus,
    pub session_id: Option<String>,
    pub phase: Option<String>,
    pub step: u64,
    /// Accumulated assistant text for this turn.
    pub streamed_text: String,
    /// Tool call/result events in order.
    pub tool_events: Vec<UiRunEvent>,
    /// Pending tool approval, if any.
    pub pending_approval: Option<UiPendingApproval>,
    /// Completed transcript messages (flushed from streamed_text on step end).
    pub messages: Vec<UiSessionMessage>,
    /// Memory context summary, updated when memory retrieval occurs.
    pub memory_context: Option<UiMemoryContextSummary>,
    /// Trace summary, updated as events are processed.
    pub trace_summary: Option<UiTraceSummary>,
    /// Error message if failed.
    pub error: Option<String>,
}

impl Default for UiRunState {
    fn default() -> Self {
        Self {
            status: UiRunStatus::Idle,
            session_id: None,
            phase: None,
            step: 0,
            streamed_text: String::new(),
            tool_events: Vec::new(),
            pending_approval: None,
            messages: Vec::new(),
            memory_context: None,
            trace_summary: None,
            error: None,
        }
    }
}

impl UiRunState {
    pub fn new_running() -> Self {
        Self {
            status: UiRunStatus::Running,
            session_id: None,
            phase: Some("RunStart".into()),
            step: 0,
            streamed_text: String::new(),
            tool_events: Vec::new(),
            pending_approval: None,
            messages: Vec::new(),
            memory_context: None,
            trace_summary: None,
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
            UiRunEvent::RunStarted { session_id } => {
                self.status = UiRunStatus::Running;
                self.session_id = Some(session_id.clone());
            }
            UiRunEvent::TextDelta { delta } => {
                self.streamed_text.push_str(delta);
            }
            UiRunEvent::ToolCallStarted { .. } | UiRunEvent::ToolCallCompleted { .. } => {
                self.tool_events.push(event);
                // Update trace summary on tool events
                self.trace_summary = Some(UiTraceSummary {
                    latest_trace_id: self.trace_summary.as_ref().and_then(|t| t.latest_trace_id.clone()),
                    event_count: self.trace_summary.as_ref().map(|t| t.event_count).unwrap_or(0) + 1,
                    last_event_kind: Some("tool".into()),
                });
            }
            UiRunEvent::ToolPendingApproval { tool_call_id, tool_name, reason } => {
                self.status = UiRunStatus::WaitingForApproval;
                self.pending_approval = Some(UiPendingApproval {
                    tool_call_id: tool_call_id.clone(),
                    tool_name: tool_name.clone(),
                    reason: reason.clone(),
                });
            }
            UiRunEvent::ToolApprovalResolved { approved, .. } => {
                if *approved {
                    self.status = UiRunStatus::Running;
                } else {
                    // Rejection doesn't auto-resume; status goes to running
                    // when the next text delta or tool event arrives
                }
                self.pending_approval = None;
            }
            UiRunEvent::PhaseChanged { phase, step } => {
                // Flush assistant text into transcript on step boundary
                if *step > self.step && !self.streamed_text.is_empty() {
                    self.messages.push(UiSessionMessage {
                        role: UiMessageRole::Assistant,
                        content: std::mem::take(&mut self.streamed_text),
                        trace_id: None,
                        tool_call_id: None,
                        timestamp: Some(Utc::now()),
                    });
                }
                self.phase = Some(phase.clone());
                self.step = *step;
            }
            UiRunEvent::Completed { .. } => {
                // Flush remaining streamed text
                if !self.streamed_text.is_empty() {
                    self.messages.push(UiSessionMessage {
                        role: UiMessageRole::Assistant,
                        content: std::mem::take(&mut self.streamed_text),
                        trace_id: None,
                        tool_call_id: None,
                        timestamp: Some(Utc::now()),
                    });
                }
                // Don't overwrite WaitingForApproval — the run completes
                // but approval may still be pending
                if self.status != UiRunStatus::WaitingForApproval {
                    self.status = UiRunStatus::Completed;
                }
            }
            UiRunEvent::Error { message } => {
                self.status = UiRunStatus::Failed;
                self.error = Some(message.clone());
            }
        }
    }

    /// Record that a user message was sent (for transcript display).
    pub fn record_user_message(&mut self, text: String) {
        self.messages.push(UiSessionMessage {
            role: UiMessageRole::User,
            content: text,
            trace_id: None,
            tool_call_id: None,
            timestamp: Some(Utc::now()),
        });
    }
}
