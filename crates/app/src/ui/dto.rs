//! UI-facing session DTOs.
//!
//! These are the types the Dioxus UI renders. They are projections from
//! the store layer, not authority. The UI never sees raw SQL rows.

use serde::{Deserialize, Serialize};

/// Summary for session list rendering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiSessionSummary {
    pub session_id: String,
    pub title: Option<String>,
    pub status: String,
    pub updated_at: i64,
    pub last_message_preview: Option<String>,
    pub model: Option<String>,
    pub current_phase: Option<String>,
}

/// Full session view for detail pane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSessionView {
    pub summary: UiSessionSummary,
    pub messages: Vec<UiMessage>,
    pub interaction_mode: String,
    pub current_step: i64,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub working_directory: Option<String>,
}

/// A single message in the session view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiMessage {
    pub role: UiMessageRole,
    pub text: String,
    pub trace_id: Option<String>,
    pub timestamp: Option<i64>,
    pub is_error: bool,
}

/// Message role for UI rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiMessageRole {
    User,
    Assistant,
    Tool,
}

/// Request to create a new session.
#[derive(Debug, Clone)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub provider: Option<String>,
    pub working_directory: Option<String>,
    pub interaction_mode: String,
}
