//! Workflow readiness desktop UI components.
//!
//! Read-only display of recorded workflow readiness state using Wave 52A
//! design-system tokens. Displays readiness summary, predicates, tool intents,
//! approval markers, environment, rollback/abort state, and safety copy.
//!
//! Components accept already-prepared state. They do not start workflow runs,
//! execute tools, create approval requests, mutate memory, or grant execution.

use crate::ui::design_tokens::*;
use crate::ui::workflow_readiness_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

pub fn readiness_status_tone(status: &str) -> UiTone {
    match status.to_lowercase().as_str() {
        "ready" | "proceed" => UiTone::Success,
        "blocked" | "notready" => UiTone::Error,
        "inconclusive" => UiTone::Warning,
        _ => UiTone::Neutral,
    }
}

pub fn readiness_status_label(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "ready" => "Ready (recorded)".into(),
        "proceed" => "Proceed (recorded)".into(),
        "blocked" => "Blocked (recorded)".into(),
        "notready" => "Not ready (recorded)".into(),
        "inconclusive" => "Inconclusive (recorded)".into(),
        _ => format!("Recorded: {}", status),
    }
}

pub fn intent_status_tone(status: &str) -> UiTone {
    match status {
        "resolved" => UiTone::Success,
        "unresolved" => UiTone::Error,
        "ambiguous" => UiTone::Warning,
        "rejected" => UiTone::Neutral,
        _ => UiTone::Neutral,
    }
}

pub fn readiness_safety_text() -> String {
    workflow_readiness_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use dioxus::prelude::*;

    pub fn render_workflow_readiness_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! { div { style: "{style}", "No workflow readiness records" } }
    }

    pub fn render_workflow_readiness_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM, colors::TEXT_MUTED,
        );
        rsx! { div { style: "{style}", "Loading workflow readiness\u{2026}" } }
    }

    pub fn render_workflow_readiness_panel(state: &WorkflowReadinessUiState) -> Element {
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
                div { style: "{title_style}", "Workflow Readiness" }

                if let Some(r) = &state.latest_readiness {
                    {render_summary(r)}
                }

                if !state.predicates.is_empty() {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Predicates"
                        }
                        {render_predicates(&state.predicates)}
                    }
                }

                if !state.tool_intents.is_empty() {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Tool Intents ({state.tool_intents.len()})"
                        }
                        for ti in &state.tool_intents {
                            {render_tool_intent(ti)}
                        }
                    }
                }

                if let Some(env) = &state.environment {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};", "Environment" }
                        div { style: "font-size: {typo::TEXT_SM};",
                            div { style: "color: {if env.workspace_observed {colors::STATUS_SUCCESS} else {colors::STATUS_ERROR}};",
                                if env.workspace_observed { "Workspace: observed" } else { "Workspace: not observed" }
                            }
                            div { style: "color: {if env.provider_config_available {colors::STATUS_SUCCESS} else {colors::STATUS_ERROR}};",
                                if env.provider_config_available { "Provider: available" } else { "Provider: unavailable" }
                            }
                            div { style: "color: {if env.session_runtime_available {colors::STATUS_SUCCESS} else {colors::STATUS_ERROR}};",
                                if env.session_runtime_available { "Runtime: available" } else { "Runtime: unavailable" }
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

    fn render_summary(r: &WorkflowReadinessSummaryRow) -> Element {
        let tone = readiness_status_tone(&r.status);
        let badge = tone_to_badge_color(tone);
        let label = readiness_status_label(&r.status);
        let card = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD, spacing::SPACE_MD, spacing::SPACE_MD,
        );
        let pct = if r.predicates_total > 0 {
            r.predicates_passed * 100 / r.predicates_total
        } else { 100 };
        let bar_color = if pct >= 80 { colors::STATUS_SUCCESS } else if pct >= 50 { colors::STATUS_WARN } else { colors::STATUS_ERROR };

        rsx! {
            div { style: "{card}",
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: {spacing::SPACE_SM};",
                    span { style: "background: {badge}; color: white; padding: 2px 8px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS};",
                        "{label}"
                    }
                    span { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED};",
                        "Proposal: {r.proposal_id}"
                    }
                }
                div { style: "margin-bottom: {spacing::SPACE_SM};",
                    div { style: "display: flex; justify-content: space-between; font-size: {typo::TEXT_XS}; color: {colors::TEXT_MUTED}; margin-bottom: 2px;",
                        span { "Predicates" }
                        span { "{r.predicates_passed}/{r.predicates_total} ({pct}%)" }
                    }
                    div { style: "height: 4px; background: {colors::BORDER_LIGHT}; border-radius: 2px;",
                        div { style: "height: 4px; width: {pct}%; background: {bar_color}; border-radius: 2px;" }
                    }
                }
                div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_FAINT}; font-family: monospace;",
                    "{r.readiness_id}"
                }
            }
        }
    }

    fn render_predicates(preds: &[WorkflowReadinessPredicateRow]) -> Element {
        let pass_color = tone_to_badge_color(UiTone::Success);
        let fail_color = tone_to_badge_color(UiTone::Error);
        let row_style = format!(
            "display: flex; justify-content: space-between; padding: {} 0; border-bottom: 1px solid {}; font-size: {};",
            spacing::SPACE_XS, colors::BORDER_LIGHT, typo::TEXT_SM,
        );
        rsx! {
            div {
                for pred in preds {
                    {
                        let bg = if pred.passed { pass_color.clone() } else { fail_color.clone() };
                        let status_text = if pred.passed { "PASS" } else { "FAIL" };
                        rsx! {
                            div { style: "{row_style}",
                                span { style: "color: {colors::TEXT_BODY}; flex: 1;", "{pred.predicate}" }
                                span { style: "background: {bg}; color: white; padding: 1px 6px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS}; margin-right: {spacing::SPACE_SM};", "{status_text}" }
                                span { style: "color: {colors::TEXT_MUTED}; flex: 2;", "{pred.reason}" }
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_tool_intent(ti: &ToolIntentResolutionRow) -> Element {
        let tone = intent_status_tone(&ti.status);
        let badge = tone_to_badge_color(tone);
        rsx! {
            div { style: "font-size: {typo::TEXT_SM}; padding: {spacing::SPACE_XS} 0; border-bottom: 1px solid {colors::BORDER_LIGHT}; display: flex; align-items: center; gap: {spacing::SPACE_SM};",
                span { style: "background: {badge}; color: white; padding: 1px 6px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS};",
                    "{ti.status}"
                }
                span { style: "color: {colors::TEXT_BODY};", "{ti.capability}" }
                span { style: "color: {colors::TEXT_MUTED}; flex: 1;", "{ti.reason}" }
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
        let text = readiness_safety_text();
        rsx! { div { style: "{style}", "{text}" } }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;
