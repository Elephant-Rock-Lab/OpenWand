//! Session view helpers and Dioxus render functions.
//!
//! Pure helpers extract display data without Dioxus dependency.
//! Render functions consume helpers for UI display.
//! Neither mutates any state or calls backends.
//!
//! Wave 52A: migrated to design-system tokens and component style builders.

#[cfg(feature = "desktop")]
use crate::ui::components::*;
#[cfg(feature = "desktop")]
use crate::ui::design_tokens::*;
#[cfg(feature = "desktop")]
use crate::ui::layout::*;
use crate::ui::run_dto::{
    UiMemoryContextSummary, UiPendingApproval, UiRunStatus,
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
    let style = status_bar_style();
    rsx! {
        div { style: "{style}",
            "{text}"
        }
    }
}

#[cfg(feature = "desktop")]
pub fn render_chat_transcript(state: &UiRunState) -> Element {
    let lines = chat_transcript_lines(&state.messages, &state.streamed_text);
    let style = scroll_area_style();
    rsx! {
        div { style: "{style}",
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
            let style = banner_style(UiTone::Warning);
            let line_style = format!(
                "font-size: {}; font-family: {};",
                typo::TEXT_BASE,
                typo::FONT_MONO,
            );
            rsx! {
                div { style: "{style}",
                    for line in lines {
                        div { style: "{line_style}", "{line}" }
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
            let style = format!(
                "font-size: {}; color: {}; padding: {} {}; font-family: {};",
                typo::TEXT_SM,
                colors::TEXT_SECONDARY,
                spacing::SPACE_SM,
                spacing::SPACE_MD,
                typo::FONT_MONO,
            );
            rsx! {
                div { style: "{style}",
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
            let style = banner_style(UiTone::Error);
            rsx! {
                div { style: "{style}",
                    "{text}"
                }
            }
        }
        None => rsx! { div {} },
    }
}
