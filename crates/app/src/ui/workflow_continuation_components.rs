//! Workflow continuation desktop UI components.
//!
//! Read-only display of recorded workflow continuation state using Wave 52A
//! design-system tokens. Displays continuation readiness, next-action proposal,
//! predicates, candidate, evidence links, and explicit no-routing copy.
//!
//! Components accept already-prepared state. They do not route actions,
//! resolve approvals, execute tools, append trace, or mutate workflow state.

use crate::ui::design_tokens::*;
use crate::ui::workflow_continuation_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for continuation status.
pub fn continuation_status_tone(status: &str) -> UiTone {
    match status {
        "proposalready" => UiTone::Success,
        "blocked" => UiTone::Error,
        "inconclusive" => UiTone::Warning,
        "norunnablestage" => UiTone::Neutral,
        _ => UiTone::Neutral,
    }
}

/// Human-readable label for continuation status.
pub fn continuation_status_label(status: &str) -> String {
    match status {
        "proposalready" => "Proposal ready (recorded)".into(),
        "blocked" => "Blocked (recorded)".into(),
        "inconclusive" => "Inconclusive (recorded)".into(),
        "norunnablestage" => "No runnable stage (recorded)".into(),
        _ => format!("Recorded: {}", status),
    }
}

/// Human-readable label for candidate kind.
pub fn candidate_kind_label(kind: &str) -> String {
    match kind {
        "routepreparedaction" => "Route prepared action".into(),
        "manualaction" => "Manual action".into(),
        "continuationproposal" => "Continuation proposal".into(),
        _ => format!("Candidate: {}", kind),
    }
}

/// Safety text.
pub fn continuation_safety_text() -> String {
    workflow_continuation_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;

    use dioxus::prelude::*;

    /// Empty state.
    pub fn render_continuation_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No continuation records"
            }
        }
    }

    /// Loading state.
    pub fn render_continuation_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading workflow continuation\u{2026}"
            }
        }
    }

    /// Full continuation panel from UI state.
    pub fn render_continuation_panel(state: &WorkflowContinuationUiState) -> Element {
        let section_title_style = format!(
            "font-size: {}; font-weight: 600; color: {}; margin-bottom: {};",
            typo::TEXT_BASE, colors::TEXT_STRONG, spacing::SPACE_SM,
        );
        let card_style = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD,
            spacing::SPACE_MD, spacing::SPACE_MD,
        );

        rsx! {
            div {
                // Section header
                div { style: "{section_title_style}",
                    "Workflow Continuation"
                }

                // Readiness summary
                if let Some(readiness) = &state.latest_readiness {
                    {render_readiness_summary(readiness)}
                }

                // Next-action candidate
                if let Some(candidate) = &state.candidate {
                    div { style: "{card_style}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Next Action Candidate"
                        }
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY};",
                            "Stage: {candidate.stage_id}"
                        }
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED};",
                            "Kind: {candidate_kind_label(&candidate.candidate_kind)}"
                        }
                        if let Some(ar) = &candidate.action_request_id {
                            div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_FAINT}; font-family: monospace; margin-top: {spacing::SPACE_XS};",
                                "Action: {ar}"
                            }
                        }
                    }
                }

                // Proposal info
                if let Some(proposal) = &state.latest_proposal {
                    div { style: "{card_style}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Proposal"
                        }
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED};",
                            "ID: {proposal.proposal_id}"
                        }
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED};",
                            "Stage: {proposal.stage_id}"
                        }
                    }
                }

                // Predicates
                if !state.predicates.is_empty() {
                    div { style: "{card_style}",
                        div { style: "font-weight: 600; margin-bottom: {spacing::SPACE_SM}; font-size: {typo::TEXT_SM};",
                            "Predicates"
                        }
                        {render_predicate_table(&state.predicates)}
                    }
                }

                // Evidence links
                if !state.evidence_links.is_empty() {
                    {render_evidence_links(&state.evidence_links)}
                }

                // Warnings
                for w in &state.warnings {
                    {render_warning(w)}
                }

                // Safety notice
                {render_safety_footer()}
            }
        }
    }

    fn render_readiness_summary(readiness: &WorkflowContinuationReadinessRow) -> Element {
        let tone = continuation_status_tone(&readiness.status);
        let badge_color = tone_to_badge_color(tone);
        let label = continuation_status_label(&readiness.status);
        let card_style = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD,
            spacing::SPACE_MD, spacing::SPACE_MD,
        );

        rsx! {
            div { style: "{card_style}",
                div { style: "display: flex; justify-content: space-between; align-items: center;",
                    span { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED};",
                        "Continuation: {readiness.readiness_id}"
                    }
                    span { style: "background: {badge_color}; color: white; padding: 2px 8px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS};",
                        "{label}"
                    }
                }
                div { style: "margin-top: {spacing::SPACE_XS}; font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY};",
                    "Decision: {readiness.decision}"
                }
            }
        }
    }

    fn render_predicate_table(predicates: &[WorkflowContinuationPredicateRow]) -> Element {
        let row_style = format!(
            "display: flex; justify-content: space-between; padding: {} 0; border-bottom: 1px solid {}; font-size: {};",
            spacing::SPACE_XS, colors::BORDER_LIGHT, typo::TEXT_SM,
        );
        let pass_color = tone_to_badge_color(UiTone::Success);
        let fail_color = tone_to_badge_color(UiTone::Error);

        rsx! {
            div {
                for pred in predicates {
                    {
                        let badge_color = if pred.passed { pass_color.clone() } else { fail_color.clone() };
                        let status_text = if pred.passed { "PASS" } else { "FAIL" };
                        let bg = badge_color;
                        rsx! {
                            div { style: "{row_style}",
                                span { style: "color: {colors::TEXT_BODY}; flex: 1;",
                                    "{pred.predicate}"
                                }
                                span { style: "background: {bg}; color: white; padding: 1px 6px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS}; margin-right: {spacing::SPACE_SM};",
                                    "{status_text}"
                                }
                                span { style: "color: {colors::TEXT_MUTED}; flex: 2;",
                                    "{pred.reason}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_evidence_links(links: &[WorkflowContinuationEvidenceRow]) -> Element {
        let card_style = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD,
            spacing::SPACE_MD, spacing::SPACE_MD,
        );
        let row_style = format!(
            "display: flex; gap: {}; padding: {} 0; border-bottom: 1px solid {}; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_XS, colors::BORDER_LIGHT, typo::TEXT_SM,
        );
        let kind_style = format!("color: {}; font-weight: 600; min-width: 80px;", colors::TEXT_BODY);
        let id_style = format!("color: {}; font-family: monospace;", colors::TEXT_MUTED);
        let summary_style = format!("color: {}; flex: 1;", colors::TEXT_MUTED);

        rsx! {
            div { style: "{card_style}",
                div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                    "Evidence Links ({links.len()})"
                }
                for link in links {
                    div { style: "{row_style}",
                        span { style: "{kind_style}", "{link.kind}" }
                        span { style: "{id_style}", "{link.id}" }
                        span { style: "{summary_style}", "{link.summary}" }
                    }
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
        rsx! {
            div { style: "{style}", "{text}" }
        }
    }

    fn render_safety_footer() -> Element {
        let style = format!(
            "font-size: {}; color: {}; padding: {} 0; border-top: 1px solid {}; margin-top: {};",
            typo::TEXT_XS, colors::TEXT_FAINT, spacing::SPACE_SM,
            colors::BORDER_LIGHT, spacing::SPACE_MD,
        );
        let text = continuation_safety_text();
        rsx! {
            div { style: "{style}", "{text}" }
        }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;
