//! Inspector view helpers and Dioxus render functions.
//!
//! Pure helpers extract display data without Dioxus dependency.
//! Render functions consume helpers for UI display.
//! Neither mutates any state or calls backends.

use crate::ui::inspector_state::*;

// ── Trace Timeline Rows ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TraceTimelineRow {
    pub event_kind: String,
    pub actor: String,
    pub time: String,
    pub summary: String,
    pub family: String,
}

pub fn trace_timeline_rows(state: &LiveInspectorState) -> Vec<TraceTimelineRow> {
    state.trace_timeline.iter().map(|item| TraceTimelineRow {
        event_kind: item.event_kind.clone(),
        actor: item.actor.clone(),
        time: item.timestamp.clone(),
        summary: item.summary.clone(),
        family: item.event_family.clone(),
    }).collect()
}

// ── Gate / Tool Rows ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct GateToolRow {
    pub kind_display: String,
    pub status: String,
    pub risk: Option<String>,
    pub tool: Option<String>,
    pub reason: Option<String>,
}

pub fn gate_tool_rows(state: &LiveInspectorState) -> Vec<GateToolRow> {
    state.gate_tool_events.iter().map(|item| GateToolRow {
        kind_display: item.kind.display().to_string(),
        status: item.status.clone(),
        risk: item.risk_level.clone(),
        tool: item.tool_name.clone(),
        reason: item.reason.clone(),
    }).collect()
}

// ── Memory Evidence Rows ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryEvidenceRow {
    pub claim: String,
    pub status_display: String,
    pub reason: String,
    pub trace_count: usize,
    pub confidence: Option<String>,
}

pub fn memory_evidence_rows(state: &LiveInspectorState) -> Vec<MemoryEvidenceRow> {
    state.memory_evidence.iter().map(|item| MemoryEvidenceRow {
        claim: item.claim_summary.clone(),
        status_display: item.status.display().to_string(),
        reason: item.reason.clone(),
        trace_count: item.source_trace_ids.len(),
        confidence: item.confidence.map(|c| format!("{:.0}%", c * 100.0)),
    }).collect()
}

// ── Trace Relation Rows ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TraceRelationRow {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub direction: String,
}

pub fn trace_relation_rows(detail: &TraceEventDetail) -> Vec<TraceRelationRow> {
    detail.relations.iter().map(|r| TraceRelationRow {
        from: r.from_trace_id.clone(),
        to: r.to_trace_id.clone(),
        kind: r.relation_kind.clone(),
        direction: match r.direction {
            RelationDirection::Outgoing => "→ outgoing".into(),
            RelationDirection::Incoming => "← incoming".into(),
        },
    }).collect()
}

// ── Event Detail Lines ─────────────────────────────────────────────────────

pub fn event_detail_lines(detail: &TraceEventDetail) -> Vec<String> {
    let mut lines = vec![
        format!("ID: {}", detail.trace_id),
        format!("Kind: {}", detail.event_kind),
        format!("Actor: {}", detail.actor),
        format!("Time: {}", detail.timestamp),
    ];
    if !detail.payload_summary.is_empty() {
        lines.push(String::new());
        lines.push("Payload:".into());
        for (key, val) in &detail.payload_summary {
            lines.push(format!("  {}: {}", key, val));
        }
    }
    if !detail.relations.is_empty() {
        lines.push(String::new());
        lines.push(format!("Relations ({}):", detail.relations.len()));
        for row in trace_relation_rows(detail) {
            lines.push(format!("  {} {} {} ({})", row.from, row.direction, row.to, row.kind));
        }
    }
    lines
}

// ── Inspector Warnings ─────────────────────────────────────────────────────

pub fn inspector_warning_lines(state: &LiveInspectorState) -> Vec<String> {
    state.warnings.iter().map(|w| format!("⚠ {}", w)).collect()
}

// ── Dioxus Render Functions ─────────────────────────────────────────────────

#[cfg(feature = "desktop")]
use dioxus::prelude::*;

#[cfg(feature = "desktop")]
pub fn render_trace_timeline(state: &LiveInspectorState) -> Element {
    let rows = trace_timeline_rows(state);
    rsx! {
        div { style: "padding: 8px; font-family: monospace; font-size: 11px;",
            div { style: "font-weight: 600; margin-bottom: 8px;", "Trace Timeline ({})", rows.len() }
            for row in &rows {
                div { style: "display: flex; gap: 8px; padding: 1px 0;",
                    span { style: "color: #888; min-width: 120px;", "{row.time}" }
                    span { style: "color: #666; min-width: 100px;", "{row.event_kind}" }
                    span { "{row.summary}" }
                }
            }
        }
    }
}

#[cfg(feature = "desktop")]
pub fn render_gate_tool_history(state: &LiveInspectorState) -> Element {
    let rows = gate_tool_rows(state);
    rsx! {
        div { style: "padding: 8px; font-family: monospace; font-size: 11px;",
            div { style: "font-weight: 600; margin-bottom: 8px;", "Gate/Tool History ({})", rows.len() }
            for row in &rows {
                div { style: "display: flex; gap: 8px; padding: 2px 0; border-bottom: 1px solid #eee;",
                    span { style: "min-width: 100px; font-weight: 500;", "{row.kind_display}" }
                    span { style: if row.status == "passed" || row.status == "completed" || row.status == "resumed" { "color: green" } else if row.status == "failed" || row.status == "denied" || row.status == "blocked" { "color: red" } else { "color: #c90" },
                        "{row.status}"
                    }
                    if let Some(tool) = &row.tool {
                        span { style: "color: #555;", "{tool}" }
                    }
                    if let Some(reason) = &row.reason {
                        span { style: "color: #888; font-size: 10px;", "{reason}" }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "desktop")]
pub fn render_memory_inspector(state: &LiveInspectorState) -> Element {
    let rows = memory_evidence_rows(state);
    let ctx = &state.memory_context;
    rsx! {
        div { style: "padding: 8px; font-family: monospace; font-size: 11px;",
            if let Some(ctx) = ctx {
                div { style: "font-weight: 600; margin-bottom: 8px;",
                    "Memory: {ctx.included_count} included, {ctx.excluded_count} excluded, {ctx.conflicts_count} conflicts"
                }
            }
            for row in &rows {
                div { style: "padding: 2px 0; border-bottom: 1px solid #f0f0f0;",
                    span { style: if row.status_display.starts_with("Included") { "color: green" } else { "color: red" },
                        "{row.status_display}"
                    }
                    span { style: "margin-left: 8px;", "{row.claim}" }
                }
            }
        }
    }
}

#[cfg(feature = "desktop")]
pub fn render_inspector_warnings(state: &LiveInspectorState) -> Element {
    if state.warnings.is_empty() {
        return rsx! { div {} };
    }
    let lines = inspector_warning_lines(state);
    rsx! {
        div { style: "background: #fff3cd; border: 1px solid #ffc107; border-radius: 4px; padding: 8px; margin: 8px 0; font-size: 11px; font-family: monospace;",
            for line in lines {
                div { "{line}" }
            }
        }
    }
}
