//! Workflow proposal desktop UI components.
//!
//! Read-only display of recorded workflow proposal state using Wave 52A
//! design-system tokens. Displays proposal summary, stages, tool intents,
//! risks, approval markers, abort/rollback notes, review, and safety copy.
//!
//! Components accept already-prepared state. They do not execute tools,
//! start workflow runs, schedule work, mutate memory, or create execution grants.

use crate::ui::design_tokens::*;
use crate::ui::workflow_proposal_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for proposal status.
pub fn proposal_status_tone(status: &str) -> UiTone {
    match status {
        "active" | "ready" => UiTone::Info,
        "approved" => UiTone::Success,
        "rejected" => UiTone::Error,
        "changes_requested" => UiTone::Warning,
        "superseded" | "withdrawn" => UiTone::Neutral,
        _ => UiTone::Neutral,
    }
}

pub fn proposal_status_label(status: &str) -> String {
    match status {
        "active" => "Active (recorded)".into(),
        "approved" => "Approved (recorded)".into(),
        "rejected" => "Rejected (recorded)".into(),
        "changes_requested" => "Changes requested (recorded)".into(),
        "superseded" => "Superseded (recorded)".into(),
        "withdrawn" => "Withdrawn (recorded)".into(),
        _ => format!("Recorded: {}", status),
    }
}

pub fn review_decision_tone(decision: &str) -> UiTone {
    match decision {
        "approved" => UiTone::Success,
        "rejected" => UiTone::Error,
        "changes_requested" => UiTone::Warning,
        _ => UiTone::Neutral,
    }
}

pub fn proposal_safety_text() -> String {
    workflow_proposal_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use dioxus::prelude::*;

    pub fn render_proposal_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! { div { style: "{style}", "No workflow proposal records" } }
    }

    pub fn render_proposal_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM, colors::TEXT_MUTED,
        );
        rsx! { div { style: "{style}", "Loading workflow proposal\u{2026}" } }
    }

    pub fn render_proposal_panel(state: &WorkflowProposalUiState) -> Element {
        let title_style = format!(
            "font-size: {}; font-weight: 600; color: {}; margin-bottom: {};",
            typo::TEXT_BASE, colors::TEXT_STRONG, spacing::SPACE_SM,
        );
        let card = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD, spacing::SPACE_MD, spacing::SPACE_MD,
        );

        rsx! {
            div {
                div { style: "{title_style}", "Workflow Proposal" }

                // Summary
                if let Some(p) = &state.latest_proposal {
                    {render_summary(p)}
                }

                // Review
                if let Some(r) = &state.latest_review {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Latest Review"
                        }
                        {render_review(r)}
                    }
                }

                // Stages
                if !state.stages.is_empty() {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Stages ({state.stages.len()})"
                        }
                        for stage in &state.stages {
                            {render_stage(stage)}
                        }
                    }
                }

                // Risks
                if !state.risks.is_empty() {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Risks ({state.risks.len()})"
                        }
                        for risk in &state.risks {
                            div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY}; padding: {spacing::SPACE_XS} 0; border-bottom: 1px solid {colors::BORDER_LIGHT};",
                                span { style: "font-weight: 600; margin-right: {spacing::SPACE_SM};", "{risk.risk_level}" }
                                span { style: "color: {colors::TEXT_MUTED};", "{risk.summary}" }
                            }
                        }
                    }
                }

                // Approval markers
                if !state.approvals.is_empty() {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Required Approvals ({state.approvals.len()})"
                        }
                        for m in &state.approvals {
                            div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY}; padding: {spacing::SPACE_XS} 0; border-bottom: 1px solid {colors::BORDER_LIGHT};",
                                "{m.stage_id}: {m.reason}"
                            }
                        }
                    }
                }

                for w in &state.warnings {
                    {render_warning(w)}
                }

                {render_safety_footer()}
            }
        }
    }

    fn render_summary(p: &WorkflowProposalSummaryRow) -> Element {
        let tone = proposal_status_tone(&p.status);
        let badge = tone_to_badge_color(tone);
        let label = proposal_status_label(&p.status);
        let card = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD, spacing::SPACE_MD, spacing::SPACE_MD,
        );
        rsx! {
            div { style: "{card}",
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: {spacing::SPACE_SM};",
                    span { style: "font-weight: 600; font-size: {typo::TEXT_BASE}; color: {colors::TEXT_STRONG};",
                        "{p.title}"
                    }
                    span { style: "background: {badge}; color: white; padding: 2px 8px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS};",
                        "{label}"
                    }
                }
                div { style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: {spacing::SPACE_MD}; font-size: {typo::TEXT_SM};",
                    div {
                        div { style: "color: {colors::TEXT_MUTED}; font-size: {typo::TEXT_XS};", "Stages" }
                        div { style: "color: {colors::TEXT_STRONG}; font-weight: 600;", "{p.stage_count}" }
                    }
                    div {
                        div { style: "color: {colors::TEXT_MUTED}; font-size: {typo::TEXT_XS};", "Risks" }
                        div { style: "color: {colors::TEXT_STRONG}; font-weight: 600;", "{p.risk_count}" }
                    }
                    div {
                        div { style: "color: {colors::TEXT_MUTED}; font-size: {typo::TEXT_XS};", "Source Plan" }
                        div { style: "color: {colors::TEXT_MUTED}; font-size: {typo::TEXT_XS}; font-family: monospace;", "{p.source_task_plan_id}" }
                    }
                }
                div { style: "margin-top: {spacing::SPACE_SM}; font-size: {typo::TEXT_XS}; color: {colors::TEXT_FAINT}; font-family: monospace;",
                    "{p.proposal_id}"
                }
            }
        }
    }

    fn render_review(r: &WorkflowProposalReviewRow) -> Element {
        let tone = review_decision_tone(&r.decision);
        let badge = tone_to_badge_color(tone);
        let grant_color = if r.creates_execution_grant { colors::STATUS_ERROR } else { colors::TEXT_MUTED };
        rsx! {
            div {
                div { style: "display: flex; align-items: center; gap: {spacing::SPACE_SM}; margin-bottom: {spacing::SPACE_XS};",
                    span { style: "background: {badge}; color: white; padding: 2px 8px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS};",
                        "{r.decision}"
                    }
                    span { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED};",
                        "by {r.reviewer}"
                    }
                }
                div { style: "font-size: {typo::TEXT_XS}; color: {grant_color};",
                    if r.creates_execution_grant {
                        "Creates execution grant"
                    } else {
                        "No execution grant"
                    }
                }
            }
        }
    }

    fn render_stage(s: &WorkflowStageRow) -> Element {
        let approval_badge = if s.requires_approval { "_requires approval_" } else { "" };
        rsx! {
            div { style: "font-size: {typo::TEXT_SM}; padding: {spacing::SPACE_XS} 0; border-bottom: 1px solid {colors::BORDER_LIGHT}; display: flex; justify-content: space-between;",
                span { style: "color: {colors::TEXT_BODY};",
                    "{s.order}. {s.title}"
                }
                span { style: "color: {colors::TEXT_MUTED}; font-size: {typo::TEXT_XS};",
                    "{s.kind}{approval_badge} · {s.tool_intent_count} intents"
                }
            }
        }
    }

    fn render_warning(text: &str) -> Element {
        let style = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {}; font-size: {}; color: {};",
            colors::BG_WARN, colors::BORDER_WARN, radius::RADIUS_SM,
            spacing::SPACE_SM, spacing::SPACE_SM, typo::TEXT_SM, colors::TEXT_WARN,
        );
        rsx! { div { style: "{style}", "{text}" } }
    }

    fn render_safety_footer() -> Element {
        let style = format!(
            "font-size: {}; color: {}; padding: {} 0; border-top: 1px solid {}; margin-top: {};",
            typo::TEXT_XS, colors::TEXT_FAINT, spacing::SPACE_SM,
            colors::BORDER_LIGHT, spacing::SPACE_MD,
        );
        let text = proposal_safety_text();
        rsx! { div { style: "{style}", "{text}" } }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;
