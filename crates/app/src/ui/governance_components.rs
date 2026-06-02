//! Governance console view-model helpers and Dioxus render functions.
//!
//! Pure helpers extract display data without Dioxus dependency.
//! Render functions consume helpers for UI display.
//! Neither mutates any state or calls backends.

use crate::ui::governance_state::{
    GovernanceConsoleState, GovernanceFeedbackSummary, GovernancePredicateSummary,
    GovernanceRecordSummary,
};

// ── Pure view-model helpers ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct PredicateRow {
    pub predicate: String,
    pub passed: bool,
    pub reason: String,
    pub source_record_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedbackRow {
    pub review_id: String,
    pub summary: String,
    pub blocking_reasons: Vec<String>,
    pub requested_changes: Vec<String>,
    pub evidence_gaps: Vec<String>,
}

pub fn record_card_lines(summary: &GovernanceRecordSummary) -> Vec<String> {
    let mut lines = vec![
        format!("{:?}", summary.kind),
        format!("ID: {}", summary.id),
        format!("Status: {}", summary.status),
    ];
    if let Some(ref decision) = summary.decision {
        lines.push(format!("Decision: {}", decision));
    }
    if let Some(ref hash) = summary.hash {
        lines.push(format!("Hash: {}", hash));
    }
    for (key, val) in &summary.linked_ids {
        lines.push(format!("→ {}: {}", key, val));
    }
    if let Some(ref ts) = summary.created_at {
        lines.push(format!("Created: {}", ts.to_rfc3339()));
    }
    lines.push(summary.summary.clone());
    lines
}

pub fn predicate_panel_rows(predicates: &[GovernancePredicateSummary]) -> Vec<PredicateRow> {
    predicates
        .iter()
        .map(|p| PredicateRow {
            predicate: p.predicate.clone(),
            passed: p.passed,
            reason: p.reason.clone(),
            source_record_id: p.source_record_id.clone(),
        })
        .collect()
}

pub fn feedback_panel_rows(feedback: &[GovernanceFeedbackSummary]) -> Vec<FeedbackRow> {
    feedback
        .iter()
        .map(|f| FeedbackRow {
            review_id: f.review_id.clone(),
            summary: f.summary.clone(),
            blocking_reasons: f.blocking_reasons.clone(),
            requested_changes: f.requested_changes.clone(),
            evidence_gaps: f.evidence_gaps.clone(),
        })
        .collect()
}

pub fn safety_banner_text() -> String {
    "This console reviews and displays governed records. Execution still goes through the backend gates. UI approval is not execution.".to_string()
}

pub fn overview_status_lines(state: &GovernanceConsoleState) -> Vec<String> {
    let status_of = |opt: &Option<GovernanceRecordSummary>| -> String {
        match opt {
            Some(s) => format!("{}: {}", s.id, s.status),
            None => "Missing".to_string(),
        }
    };

    let mut lines = vec![
        format!("Local proposal:        {}", status_of(&state.local_proposal)),
        format!("Local review:          {}", status_of(&state.local_review)),
        format!("Local execution:       {}", status_of(&state.local_execution)),
        format!("Post-commit verify:    {}", status_of(&state.post_commit_verification)),
        format!("Push readiness:        {}", status_of(&state.push_readiness)),
        format!("Push proposal:         {}", status_of(&state.push_proposal)),
        format!("Push review:           {}", status_of(&state.push_review)),
        format!("Push execution:        {}", status_of(&state.push_execution)),
    ];

    if !state.chain_warnings.is_empty() {
        lines.push(String::new());
        lines.push("Chain warnings:".to_string());
        for w in &state.chain_warnings {
            lines.push(format!("⚠ {}", w));
        }
    }

    lines
}

// ── Dioxus render functions ─────────────────────────────────────────────────
// Only compiled when desktop feature is active.

#[cfg(feature = "desktop")]
use dioxus::prelude::*;

#[cfg(feature = "desktop")]
pub fn render_governance_overview(state: &GovernanceConsoleState) -> Element {
    let lines = overview_status_lines(state);
    rsx! {
        div { style: "padding: 16px; font-family: monospace; font-size: 13px;",
            for line in lines {
                div { "{line}" }
            }
        }
    }
}

#[cfg(feature = "desktop")]
pub fn render_record_card(summary: &GovernanceRecordSummary) -> Element {
    let lines = record_card_lines(summary);
    rsx! {
        div { style: "border: 1px solid #ddd; border-radius: 4px; padding: 12px; margin: 8px 0; font-family: monospace; font-size: 12px;",
            for line in lines {
                div { "{line}" }
            }
        }
    }
}

#[cfg(feature = "desktop")]
pub fn render_predicate_panel(predicates: &[GovernancePredicateSummary]) -> Element {
    let rows = predicate_panel_rows(predicates);
    rsx! {
        div { style: "padding: 8px;",
            div { style: "font-weight: 600; margin-bottom: 8px;", "Predicates" }
            for row in &rows {
                div { style: "display: flex; gap: 8px; padding: 2px 0; font-size: 12px; font-family: monospace;",
                    span { style: if row.passed { "color: green" } else { "color: red" },
                        if row.passed { "✓" } else { "✗" }
                    }
                    span { "{row.predicate}" }
                    span { style: "color: #666;", "{row.reason}" }
                }
            }
        }
    }
}

#[cfg(feature = "desktop")]
pub fn render_feedback_panel(feedback: &[GovernanceFeedbackSummary]) -> Element {
    let rows = feedback_panel_rows(feedback);
    rsx! {
        div { style: "padding: 8px;",
            div { style: "font-weight: 600; margin-bottom: 8px;", "Feedback" }
            for row in &rows {
                div { style: "border: 1px solid #eee; padding: 8px; margin: 4px 0;",
                    div { style: "font-weight: 600; font-size: 12px;", "Review: {row.review_id}" }
                    div { style: "font-size: 12px;", "{row.summary}" }
                    for reason in &row.blocking_reasons {
                        div { style: "color: red; font-size: 11px;", "🚫 {reason}" }
                    }
                    for change in &row.requested_changes {
                        div { style: "color: #c90; font-size: 11px;", "📝 {change}" }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "desktop")]
pub fn render_safety_banner() -> Element {
    let text = safety_banner_text();
    rsx! {
        div { style: "background: #fff3cd; border: 1px solid #ffc107; border-radius: 4px; padding: 12px; margin: 8px 0; font-size: 12px; color: #856404;",
            "⚠ {text}"
        }
    }
}
