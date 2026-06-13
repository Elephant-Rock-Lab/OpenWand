//! Workflow reconciliation desktop UI components.
//!
//! Read-only display of recorded workflow reconciliation state using Wave 52A
//! design-system tokens. Displays reconciliation summary, stage progression,
//! run revision, predicates, lifecycle events, and safety copy.
//!
//! Components accept already-prepared state. They do not route actions,
//! resolve approvals, execute tools, append trace, or mutate session state.

use crate::ui::design_tokens::*;
use crate::ui::workflow_reconciliation_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

pub fn reconciliation_status_tone(status: &str) -> UiTone {
    match status {
        "reconciled" | "applied" => UiTone::Success,
        "skipped" => UiTone::Neutral,
        "failed" | "error" => UiTone::Error,
        "pending" | "inconclusive" => UiTone::Warning,
        _ => UiTone::Neutral,
    }
}

pub fn reconciliation_decision_tone(decision: &str) -> UiTone {
    if decision.starts_with("reconciled") || decision.starts_with("applied") {
        UiTone::Success
    } else if decision.starts_with("skipped") {
        UiTone::Neutral
    } else if decision.starts_with("failed") {
        UiTone::Error
    } else {
        UiTone::Neutral
    }
}

pub fn reconciliation_safety_text() -> String {
    workflow_reconciliation_safety_warning()
}

pub fn stage_status_arrow(previous: &str, new: &str) -> String {
    format!("{} \u{2192} {}", previous, new)
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use dioxus::prelude::*;

    pub fn render_reconciliation_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! { div { style: "{style}", "No workflow reconciliation records" } }
    }

    pub fn render_reconciliation_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM, colors::TEXT_MUTED,
        );
        rsx! { div { style: "{style}", "Loading reconciliation\u{2026}" } }
    }

    pub fn render_reconciliation_panel(state: &WorkflowReconciliationUiState) -> Element {
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
                div { style: "{title_style}", "Workflow Reconciliation" }

                // Summary
                if let Some(rec) = &state.latest_reconciliation {
                    {render_summary(rec)}
                }

                // Stage progression
                if let Some(prog) = &state.progression {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Stage Progression"
                        }
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY}; margin-bottom: {spacing::SPACE_XS};",
                            "{prog.stage_id}"
                        }
                        div { style: "display: flex; align-items: center; gap: {spacing::SPACE_SM}; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            span { style: "color: {colors::TEXT_MUTED};", "{prog.previous_status}" }
                            span { style: "color: {colors::TEXT_FAINT};", "\u{2192}" }
                            span { style: "color: {colors::TEXT_STRONG}; font-weight: 600;", "{prog.new_status}" }
                        }
                        div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_MUTED};",
                            "{prog.summary}"
                        }
                    }
                }

                // Run revision
                if let Some(rev) = &state.latest_run_revision {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Run Revision"
                        }
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY}; margin-bottom: {spacing::SPACE_XS};",
                            "Revision: {rev.revision_id}"
                        }
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED}; margin-bottom: {spacing::SPACE_XS};",
                            "Stages: {rev.stage_count}"
                        }
                        if let Some(agg) = &rev.aggregate_status {
                            div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_MUTED};",
                                "Aggregate: {agg}"
                            }
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

                // Lifecycle event
                if let Some(evt) = &state.lifecycle_event {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Lifecycle Event"
                        }
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY};",
                            "{evt.event_kind}"
                        }
                        div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_MUTED};",
                            "{evt.summary}"
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

    fn render_summary(rec: &WorkflowReconciliationSummaryRow) -> Element {
        let tone = reconciliation_status_tone(&rec.status);
        let badge = tone_to_badge_color(tone);
        let card = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD, spacing::SPACE_MD, spacing::SPACE_MD,
        );
        rsx! {
            div { style: "{card}",
                div { style: "display: flex; align-items: center; gap: {spacing::SPACE_SM}; margin-bottom: {spacing::SPACE_SM};",
                    span { style: "background: {badge}; color: white; padding: 2px 8px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS};",
                        "{rec.status}"
                    }
                    span { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED};",
                        "Decision: {rec.decision}"
                    }
                }
                div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_FAINT}; font-family: monospace;",
                    "{rec.reconciliation_id}"
                }
            }
        }
    }

    fn render_predicates(preds: &[WorkflowReconciliationPredicateRow]) -> Element {
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
        let text = reconciliation_safety_text();
        rsx! { div { style: "{style}", "{text}" } }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconciliation_status_tone_maps_known_statuses() {
        assert!(matches!(reconciliation_status_tone("reconciled"), UiTone::Success));
        assert!(matches!(reconciliation_status_tone("failed"), UiTone::Error));
        assert!(matches!(reconciliation_status_tone("pending"), UiTone::Warning));
        assert!(matches!(reconciliation_status_tone("unknown"), UiTone::Neutral));
    }

    #[test]
    fn reconciliation_decision_tone_maps_known_decisions() {
        assert!(matches!(reconciliation_decision_tone("reconciled"), UiTone::Success));
        assert!(matches!(reconciliation_decision_tone("skipped"), UiTone::Neutral));
        assert!(matches!(reconciliation_decision_tone("failed"), UiTone::Error));
    }

    #[test]
    fn stage_status_arrow_formats_transition() {
        let arrow = stage_status_arrow("suspended", "completed");
        assert!(arrow.contains("suspended"));
        assert!(arrow.contains("completed"));
        assert!(arrow.contains("\u{2192}"));
    }

    #[test]
    fn reconciliation_safety_text_matches_warning() {
        let text = reconciliation_safety_text();
        assert!(text.contains("does not route actions"));
        assert!(text.contains("execute tools"));
    }
}
