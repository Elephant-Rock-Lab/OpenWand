//! Workflow loop controller desktop UI components.
//!
//! Read-only display of recorded loop controller state using Wave 52A
//! design-system tokens. Displays controller summary, detected state,
//! recommendation, predicates, evidence links, and safety copy.
//!
//! Components accept already-prepared state. They do not route actions,
//! resolve approvals, reconcile outcomes, execute tools, append trace,
//! or mutate workflow state.

use crate::ui::design_tokens::*;
use crate::ui::workflow_loop_controller_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

pub fn loop_controller_status_tone(status: &str) -> UiTone {
    match status {
        "recommendationready" | "complete" => UiTone::Success,
        "blocked" | "error" => UiTone::Error,
        "pending" | "inconclusive" => UiTone::Warning,
        _ => UiTone::Neutral,
    }
}

pub fn recommendation_operation_label(op: &str) -> String {
    match op {
        "createcontinuationproposal" => "Create continuation proposal".into(),
        "createroutingreadiness" => "Create routing readiness".into(),
        "createnextactionrouting" => "Create next-action routing".into(),
        "createmanualresult" => "Create manual result".into(),
        "createreconciliation" => "Create reconciliation".into(),
        "createcommandcomposer" => "Create command composer".into(),
        "createcommandreview" => "Create command review".into(),
        "waitforexternalevidence" => "Wait for external evidence".into(),
        "terminal" => "Terminal state".into(),
        _ => format!("Operation: {}", op),
    }
}

pub fn detected_state_label(state: &str) -> String {
    match state {
        s if s.contains("needsinitial") => "Needs initial proposal".into(),
        s if s.contains("suspended") => "Suspended mid-stage".into(),
        s if s.contains("awaiting") => "Awaiting evidence".into(),
        s if s.contains("terminal") | s.contains("complete") => "Terminal / complete".into(),
        _ => format!("State: {}", state),
    }
}

pub fn loop_controller_safety_text() -> String {
    workflow_loop_controller_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use dioxus::prelude::*;

    pub fn render_loop_controller_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! { div { style: "{style}", "No loop controller records" } }
    }

    pub fn render_loop_controller_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM, colors::TEXT_MUTED,
        );
        rsx! { div { style: "{style}", "Loading loop controller\u{2026}" } }
    }

    pub fn render_loop_controller_panel(state: &WorkflowLoopControllerUiState) -> Element {
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
                div { style: "{title_style}", "Loop Controller" }

                // Controller summary
                if let Some(ctrl) = &state.latest_controller {
                    div { style: "{card}",
                        div { style: "display: flex; align-items: center; gap: {spacing::SPACE_SM}; margin-bottom: {spacing::SPACE_XS};",
                            {
                                let tone = loop_controller_status_tone(&ctrl.status);
                                let badge = tone_to_badge_color(tone);
                                rsx! {
                                    span { style: "background: {badge}; color: white; padding: 2px 8px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS};",
                                        "{ctrl.status}"
                                    }
                                }
                            }
                        }
                        div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_FAINT}; font-family: monospace;",
                            "{ctrl.controller_id}"
                        }
                    }
                }

                // Detected state
                if let Some(ds) = &state.detected_state {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Detected State"
                        }
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY};",
                            "{detected_state_label(ds)}"
                        }
                    }
                }

                // Recommendation
                if let Some(rec) = &state.recommendation {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Recommendation"
                        }
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_STRONG}; font-weight: 600; margin-bottom: {spacing::SPACE_XS};",
                            "{recommendation_operation_label(&rec.operation)}"
                        }
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED};",
                            "{rec.reason}"
                        }
                    }
                }

                // Predicates
                if !state.predicates.is_empty() {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};", "Predicates" }
                        {render_predicates(&state.predicates)}
                    }
                }

                // Evidence links
                if !state.evidence_links.is_empty() {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Evidence Links ({state.evidence_links.len()})"
                        }
                        for ev in &state.evidence_links {
                            div { style: "font-size: {typo::TEXT_SM}; padding: {spacing::SPACE_XS} 0; border-bottom: 1px solid {colors::BORDER_LIGHT}; display: flex; justify-content: space-between;",
                                span { style: "color: {colors::TEXT_BODY};",
                                    "{ev.link_kind}: {ev.record_id}"
                                }
                                span { style: "color: {colors::TEXT_MUTED};",
                                    "{ev.summary}"
                                }
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

    fn render_predicates(preds: &[WorkflowLoopPredicateRow]) -> Element {
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
        let text = loop_controller_safety_text();
        rsx! { div { style: "{style}", "{text}" } }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_controller_status_tone_maps_known() {
        assert!(matches!(loop_controller_status_tone("recommendationready"), UiTone::Success));
        assert!(matches!(loop_controller_status_tone("blocked"), UiTone::Error));
        assert!(matches!(loop_controller_status_tone("pending"), UiTone::Warning));
    }

    #[test]
    fn recommendation_operation_label_formats_known_ops() {
        assert_eq!("Create continuation proposal", recommendation_operation_label("createcontinuationproposal"));
        assert_eq!("Terminal state", recommendation_operation_label("terminal"));
    }

    #[test]
    fn recommendation_operation_label_falls_back() {
        let label = recommendation_operation_label("customOp");
        assert!(label.contains("customOp"));
    }

    #[test]
    fn detected_state_label_formats_known_states() {
        assert_eq!("Needs initial proposal", detected_state_label("needsinitialcontinuationproposal"));
        assert_eq!("Terminal / complete", detected_state_label("terminal"));
    }

    #[test]
    fn loop_controller_safety_text_matches_warning() {
        let text = loop_controller_safety_text();
        assert!(text.contains("recommends the next manual operation"));
        assert!(text.contains("does not route"));
    }
}
