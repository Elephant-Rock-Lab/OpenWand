//! Session view helpers and Dioxus render functions.
//!
//! Pure helpers extract display data without Dioxus dependency.
//! Render functions consume helpers for UI display.
//! Neither mutates any state or calls backends.

use crate::ui::run_dto::{
    UiMemoryContextSummary, UiPendingApproval, UiRunState, UiRunStatus,
    UiSessionMessage, UiTraceSummary,
};

// ── Status bar ──────────────────────────────────────────────────────────────

pub fn status_bar_text(status: UiRunStatus) -> String {
    match status {
        UiRunStatus::Idle => "Idle".into(),
        UiRunStatus::Starting => "Starting…".into(),
        UiRunStatus::Running => "Running".into(),
        UiRunStatus::WaitingForApproval => "⏳ Waiting for approval".into(),
        UiRunStatus::Blocked => "🚫 Blocked".into(),
        UiRunStatus::Completed => "✅ Completed".into(),
        UiRunStatus::Failed => "❌ Failed".into(),
        UiRunStatus::Error => "⚠ Error".into(),
        UiRunStatus::Cancelled => "⊘ Cancelled".into(),
    }
}

// ── Chat transcript ─────────────────────────────────────────────────────────

pub fn chat_transcript_lines(messages: &[UiSessionMessage], streaming: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for msg in messages {
        let prefix = match msg.role {
            crate::ui::run_dto::UiMessageRole::User => "You",
            crate::ui::run_dto::UiMessageRole::Assistant => "Assistant",
            crate::ui::run_dto::UiMessageRole::Tool => "Tool",
            crate::ui::run_dto::UiMessageRole::System => "System",
        };
        lines.push(format!("{}: {}", prefix, msg.content));
    }
    if !streaming.is_empty() {
        lines.push(format!("Assistant: {}…", streaming));
    }
    lines
}

// ── Approval panel ──────────────────────────────────────────────────────────

pub fn approval_panel_text(approval: &UiPendingApproval) -> Vec<String> {
    vec![
        format!("Tool: {}", approval.tool_name),
        format!("ID: {}", approval.tool_call_id),
        format!("Reason: {}", approval.reason),
        "Awaiting your decision.".into(),
    ]
}

// ── Memory context indicator ────────────────────────────────────────────────

pub fn memory_context_text(ctx: &UiMemoryContextSummary) -> String {
    format!(
        "Memory: {} retrieved, {} included, {} excluded{}",
        ctx.retrieved_count,
        ctx.included_count,
        ctx.excluded_count,
        if ctx.report_available { " (report)" } else { "" }
    )
}

// ── Trace summary ───────────────────────────────────────────────────────────

pub fn trace_summary_text(summary: &UiTraceSummary) -> String {
    let id = summary
        .latest_trace_id
        .as_deref()
        .unwrap_or("(none)");
    let kind = summary
        .last_event_kind
        .as_deref()
        .unwrap_or("(none)");
    format!("Trace: {} events, last: {}, ID: {}", summary.event_count, kind, id)
}

// ── Error panel ─────────────────────────────────────────────────────────────

pub fn error_panel_text(error: &str) -> String {
    // Sanitize: don't expose internal paths or stack traces
    let safe: String = error
        .chars()
        .take(200)
        .collect();
    format!("Error: {}", safe)
}

// ── Dioxus render functions ─────────────────────────────────────────────────

#[cfg(feature = "desktop")]
use dioxus::prelude::*;

#[cfg(feature = "desktop")]
pub fn render_status_bar(state: &UiRunState) -> Element {
    let text = status_bar_text(state.status);
    rsx! {
        div { style: "padding: 8px; font-family: monospace; font-size: 12px; border-bottom: 1px solid #ddd;",
            "{text}"
        }
    }
}

#[cfg(feature = "desktop")]
pub fn render_chat_transcript(state: &UiRunState) -> Element {
    let lines = chat_transcript_lines(&state.messages, &state.streamed_text);
    rsx! {
        div { style: "padding: 16px; font-family: monospace; font-size: 13px; white-space: pre-wrap;",
            for line in lines {
                div { "{line}" }
            }
        }
    }
}

#[cfg(feature = "desktop")]
pub fn render_approval_panel(state: &UiRunState) -> Element {
    match &state.pending_approval {
        Some(approval) => {
            let lines = approval_panel_text(approval);
            rsx! {
                div { style: "background: #fff3cd; border: 1px solid #ffc107; border-radius: 4px; padding: 12px; margin: 8px 0;",
                    for line in lines {
                        div { style: "font-size: 12px; font-family: monospace;", "{line}" }
                    }
                }
            }
        }
        None => rsx! { div {} },
    }
}

#[cfg(feature = "desktop")]
pub fn render_memory_indicator(state: &UiRunState) -> Element {
    match &state.memory_context {
        Some(ctx) => {
            let text = memory_context_text(ctx);
            rsx! {
                div { style: "font-size: 11px; color: #666; padding: 4px 8px; font-family: monospace;",
                    "{text}"
                }
            }
        }
        None => rsx! { div {} },
    }
}

#[cfg(feature = "desktop")]
pub fn render_error_panel(state: &UiRunState) -> Element {
    match &state.error {
        Some(err) => {
            let text = error_panel_text(err);
            rsx! {
                div { style: "background: #f8d7da; border: 1px solid #f5c6cb; border-radius: 4px; padding: 12px; color: #721c24; font-size: 12px; font-family: monospace;",
                    "{text}"
                }
            }
        }
        None => rsx! { div {} },
    }
}
