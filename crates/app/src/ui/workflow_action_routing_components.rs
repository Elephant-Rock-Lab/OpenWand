//! Workflow action routing desktop UI components.
//!
//! Read-only display of recorded workflow action route state using Wave 52A
//! design-system tokens. Displays route status, predicates, session route
//! metadata, and explicit no-route-selection copy (Patch 1).
//!
//! Components accept already-prepared state. They do not choose routes,
//! advance workflow state, approve actions, execute actions, append trace,
//! write memory, or create workflow/evidence records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_action_routing_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for route status.
pub fn route_status_tone(status: &str) -> UiTone {
    match status {
        "completed" => UiTone::Info,
        "pending" => UiTone::Warning,
        "failed" => UiTone::Error,
        "cancelled" => UiTone::Neutral,
        _ => UiTone::Neutral,
    }
}

/// Human-readable label for route status.
pub fn route_status_label(status: &str) -> String {
    match status {
        "completed" => "Completed (recorded)".into(),
        "pending" => "Pending (recorded)".into(),
        "failed" => "Failed (recorded)".into(),
        "cancelled" => "Cancelled (recorded)".into(),
        _ => format!("Recorded: {}", status),
    }
}

/// Patch 1: no-route-selection copy.
pub fn route_not_selected_note() -> String {
    "Displayed route, not selected by this UI.".into()
}

pub fn route_no_advancement_note() -> String {
    "No workflow advancement is available here.".into()
}

/// Safety text.
pub fn route_safety_text() -> String {
    workflow_action_route_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use crate::ui::layout::*;
    use dioxus::prelude::*;

    /// Empty state.
    pub fn render_route_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            SPACING::SPACE_LG, SPACING::SPACE_XL, TYPO::TEXT_SM,
            COLORS::TEXT_FAINT, COLORS::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No workflow action routes recorded"
            }
        }
    }

    /// Loading state.
    pub fn render_route_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            SPACING::SPACE_LG, SPACING::SPACE_XL, TYPO::TEXT_SM,
            COLORS::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading action route…"
            }
        }
    }

    /// Error state.
    pub fn render_route_error_state(message: &str) -> Element {
        let style = banner_style(UiTone::Error);
        let safe: String = message.chars().take(200).collect();
        rsx! {
            div { style: "{style}",
                "Route load error: {safe}"
            }
        }
    }

    /// Safety banner.
    pub fn render_route_safety_banner() -> Element {
        let text = route_safety_text();
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                "⚠ {text}"
            }
        }
    }

    /// Route summary card.
    pub fn render_route_summary(row: &WorkflowActionRouteSummaryRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let tone = route_status_tone(&row.status);
        let label = route_status_label(&row.status);
        let badge_s = badge_style(tone);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );
        let label_s = format!("min-width: 140px; color: {};", COLORS::TEXT_PRIMARY);
        let value_s = format!("color: {};", COLORS::TEXT_MUTED);
        let note1 = route_not_selected_note();
        let note2 = route_no_advancement_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            TYPO::TEXT_XS, COLORS::TEXT_MUTED, SPACING::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Recorded route decision ({row.route_id})"
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Status" }
                    span { style: "{badge_s}", "{label}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Stage" }
                    span { style: "{value_s}", "{row.stage_id}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Action request" }
                    span { style: "{value_s}", "{row.action_request_id}" }
                }
                div { style: "{note_s}", "{note1}" }
                div { style: "{note_s}", "{note2}" }
            }
        }
    }

    /// Session route card.
    pub fn render_session_route(session: &WorkflowSessionRouteRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );
        let label_s = format!("min-width: 140px; color: {};", COLORS::TEXT_PRIMARY);
        let value_s = format!("color: {};", COLORS::TEXT_MUTED);

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Session route (observed)"
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Session" }
                    span { style: "{value_s}", "{session.session_id}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Status" }
                    span { style: "{value_s}", "{session.session_status}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Trace count" }
                    span { style: "{value_s}", "{session.trace_count}" }
                }
                if session.pending_approval {
                    div { style: "{row_s}",
                        span { style: "{label_s}", "Pending approval" }
                        span { style: "color: {};", COLORS::ACCENT_WARNING, "Yes" }
                    }
                }
            }
        }
    }

    /// Route predicate rows (Patch 5: textual-first).
    pub fn render_route_predicate_rows(predicates: &[WorkflowActionRoutePredicateRow]) -> Element {
        if predicates.is_empty() {
            return rsx! { div {} };
        }
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );
        let name_s = format!("min-width: 220px; color: {};", COLORS::TEXT_PRIMARY);

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Route predicates"
                }
                for pred in predicates {
                    div { style: "{row_s}",
                        span { style: "{name_s}", "{pred.predicate}" }
                        span {
                            style: "min-width: 80px; color: {};",
                            if pred.passed { COLORS::ACCENT_INFO } else { COLORS::ACCENT_ERROR },
                            if pred.passed { "Passed" } else { "Failed" }
                        }
                        span { style: "color: {};", COLORS::TEXT_MUTED, "{pred.reason}" }
                    }
                }
            }
        }
    }

    /// Route prompt display (descriptive only).
    pub fn render_route_prompt(prompt: &WorkflowActionRoutePromptRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );
        let label_s = format!("min-width: 140px; color: {};", COLORS::TEXT_PRIMARY);
        let value_s = format!("color: {};", COLORS::TEXT_MUTED);

        rsx! {
            div { style: "{card_s}",
                div { style: "{row_s}",
                    span { style: "{label_s}", "Capability" }
                    span { style: "{value_s}", "{prompt.capability}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Purpose" }
                    span { style: "{value_s}", "{prompt.purpose}" }
                }
                if prompt.governance_constraint {
                    div { style: "{row_s}",
                        span { style: "{label_s}", "Governance" }
                        span { style: "color: {};", COLORS::ACCENT_WARNING, "Constrained" }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_status_tone_info_for_completed() {
        assert_eq!(UiTone::Info, route_status_tone("completed"));
    }

    #[test]
    fn route_status_tone_warning_for_pending() {
        assert_eq!(UiTone::Warning, route_status_tone("pending"));
    }

    #[test]
    fn route_status_tone_error_for_failed() {
        assert_eq!(UiTone::Error, route_status_tone("failed"));
    }

    #[test]
    fn route_status_label_says_recorded() {
        assert!(route_status_label("completed").contains("recorded"));
    }

    // ── Patch 1 guard tests ──

    #[test]
    fn routing_ui_copy_says_recorded_not_selected() {
        let note = route_not_selected_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("not selected"), "got: {note}");
    }

    #[test]
    fn routing_ui_copy_says_no_workflow_advancement() {
        let note = route_no_advancement_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("no workflow advancement"), "got: {note}");
    }

    #[test]
    fn routing_ui_copy_contains_no_route_action_verbs() {
        let all_copy = vec![
            route_status_label("completed"),
            route_status_label("pending"),
            route_not_selected_note(),
            route_no_advancement_note(),
            route_safety_text(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("choose"), "copy contains 'choose': {text}");
            assert!(!lower.contains("select route"), "copy contains 'select route': {text}");
            assert!(!lower.contains("dispatch"), "copy contains 'dispatch': {text}");
            assert!(!lower.contains("route now"), "copy contains 'route now': {text}");
        }
    }
}
