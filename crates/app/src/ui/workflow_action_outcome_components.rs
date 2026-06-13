//! Workflow action outcome desktop UI components.
//!
//! Read-only display of recorded workflow action outcome state using Wave 52A
//! design-system tokens. Displays outcome status, approval resolution,
//! predicates, session outcome, trace links, and explicit no-mutation copy.
//!
//! Components accept already-prepared state. They do not approve tools,
//! execute tools, append trace, write memory, or create workflow records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_action_outcome_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for outcome status.
pub fn outcome_status_tone(status: &str) -> UiTone {
    match status {
        "toolcompleted" | "approved" => UiTone::Success,
        "toolfailed" | "rejected" => UiTone::Error,
        "pending" => UiTone::Warning,
        "cancelled" => UiTone::Neutral,
        _ => UiTone::Neutral,
    }
}

/// Human-readable label for outcome status.
pub fn outcome_status_label(status: &str) -> String {
    match status {
        "toolcompleted" => "Tool completed (recorded)".into(),
        "toolfailed" => "Tool failed (recorded)".into(),
        "approved" => "Approved (recorded)".into(),
        "rejected" => "Rejected (recorded)".into(),
        "pending" => "Pending (recorded)".into(),
        "cancelled" => "Cancelled (recorded)".into(),
        _ => format!("Recorded: {}", status),
    }
}

/// Semantic tone for approval resolution.
pub fn approval_resolution_tone(resolution: &str) -> UiTone {
    match resolution {
        "approved" => UiTone::Success,
        "rejected" => UiTone::Error,
        _ => UiTone::Warning,
    }
}

/// Safety text.
pub fn outcome_safety_text() -> String {
    workflow_action_outcome_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;

    use dioxus::prelude::*;

    /// Empty state.
    pub fn render_action_outcome_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No action outcome records"
            }
        }
    }

    /// Loading state.
    pub fn render_action_outcome_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading action outcome\u{2026}"
            }
        }
    }

    /// Full action outcome panel from UI state.
    pub fn render_action_outcome_panel(state: &WorkflowActionOutcomeUiState) -> Element {
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
                    "Action Outcome"
                }

                // Latest outcome summary
                if let Some(outcome) = &state.latest_outcome {
                    {render_outcome_summary(outcome)}
                }

                // Predicates
                if !state.predicates.is_empty() {
                    div { style: "{card_style}",
                        div { style: "font-weight: 600; margin-bottom: {spacing::SPACE_SM};",
                            "Predicates"
                        }
                        {render_predicate_table(&state.predicates)}
                    }
                }

                // Approval resolution
                if let Some(resolution) = &state.approval_resolution {
                    {render_approval_resolution(resolution)}
                }

                // Session outcome
                if let Some(session) = &state.session_outcome {
                    {render_session_outcome(session)}
                }

                // Trace links
                if !state.trace_links.is_empty() {
                    {render_trace_links(&state.trace_links)}
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

    fn render_outcome_summary(outcome: &WorkflowActionOutcomeSummaryRow) -> Element {
        let tone = outcome_status_tone(&outcome.status);
        let label = outcome_status_label(&outcome.status);
        let badge_color = tone_to_badge_color(tone);
        let card_style = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD,
            spacing::SPACE_MD, spacing::SPACE_MD,
        );

        rsx! {
            div { style: "{card_style}",
                div { style: "display: flex; justify-content: space-between; align-items: center;",
                    span { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED};",
                        "Outcome: {outcome.outcome_id}"
                    }
                    span { style: "background: {badge_color}; color: white; padding: 2px 8px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS};",
                        "{label}"
                    }
                }
                div { style: "margin-top: {spacing::SPACE_XS}; font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY};",
                    "Decision: {outcome.decision}"
                }
            }
        }
    }

    fn render_predicate_table(predicates: &[WorkflowActionOutcomePredicateRow]) -> Element {
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

    fn render_approval_resolution(resolution: &WorkflowApprovalResolutionRow) -> Element {
        let tone = approval_resolution_tone(&resolution.resolution);
        let badge_color = tone_to_badge_color(tone);
        let card_style = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD,
            spacing::SPACE_MD, spacing::SPACE_MD,
        );

        rsx! {
            div { style: "{card_style}",
                div { style: "display: flex; align-items: center; gap: {spacing::SPACE_SM}; margin-bottom: {spacing::SPACE_XS};",
                    span { style: "font-weight: 600; font-size: {typo::TEXT_SM};",
                        "Approval"
                    }
                    span { style: "background: {badge_color}; color: white; padding: 2px 8px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS};",
                        "{resolution.resolution}"
                    }
                }
                div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY};",
                    "{resolution.rationale}"
                }
            }
        }
    }

    fn render_session_outcome(session: &WorkflowSessionActionOutcomeRow) -> Element {
        let card_style = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD,
            spacing::SPACE_MD, spacing::SPACE_MD,
        );
        let label_style = format!("color: {}; font-size: {};", colors::TEXT_MUTED, typo::TEXT_XS);
        let value_style = format!("color: {}; font-size: {};", colors::TEXT_BODY, typo::TEXT_SM);

        rsx! {
            div { style: "{card_style}",
                div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                    "Session Outcome"
                }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: {spacing::SPACE_SM};",
                    div {
                        div { style: "{label_style}", "Session" }
                        div { style: "{value_style}", "{session.session_id}" }
                    }
                    if let Some(tool) = &session.tool_name {
                        div {
                            div { style: "{label_style}", "Tool" }
                            div { style: "{value_style}", "{tool}" }
                        }
                    }
                    if let Some(status) = &session.tool_status {
                        div {
                            div { style: "{label_style}", "Status" }
                            div { style: "{value_style}", "{status}" }
                        }
                    }
                    div {
                        div { style: "{label_style}", "Traces" }
                        div { style: "{value_style}", "{session.trace_count}" }
                    }
                }
            }
        }
    }

    fn render_trace_links(links: &[WorkflowOutcomeTraceLinkRow]) -> Element {
        let card_style = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD,
            spacing::SPACE_MD, spacing::SPACE_MD,
        );
        let mono_style = format!(
            "font-family: monospace; font-size: {}; color: {}; padding: {} 0;",
            typo::TEXT_XS, colors::TEXT_BODY, spacing::SPACE_XS,
        );

        rsx! {
            div { style: "{card_style}",
                div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                    "Trace Links ({links.len()})"
                }
                for link in links {
                    div { style: "{mono_style}",
                        "{link.trace_id}"
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
        let text = outcome_safety_text();
        rsx! {
            div { style: "{style}", "{text}" }
        }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;
