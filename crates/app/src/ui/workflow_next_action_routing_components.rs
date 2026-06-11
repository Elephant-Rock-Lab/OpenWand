//! Next-action routing desktop UI components.
//!
//! Read-only display of recorded next-action routing state using Wave 52A
//! design-system tokens. Displays routing status, predicates, route link,
//! and explicit no-execution copy (Patch 2).
//!
//! Components accept already-prepared state. They do not choose routes,
//! execute actions, create proposals, advance workflow state, append trace,
//! write memory, or create workflow/evidence records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_next_action_routing_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for routing status.
pub fn routing_status_tone(status: &str) -> UiTone {
    match status {
        "routed" => UiTone::Info,
        "blocked" => UiTone::Error,
        "pending" => UiTone::Warning,
        _ => UiTone::Neutral,
    }
}

/// Human-readable label.
pub fn routing_status_label(status: &str) -> String {
    match status {
        "routed" => "Routed (recorded)".into(),
        "blocked" => "Blocked (recorded)".into(),
        "pending" => "Pending (recorded)".into(),
        _ => format!("Recorded: {}", status),
    }
}

/// Patch 2: suggested, not executable.
pub fn routing_suggested_note() -> String {
    "Recorded next-action routing state.".into()
}

pub fn routing_no_execution_note() -> String {
    "No action execution is available here.".into()
}

pub fn routing_no_proposal_created_note() -> String {
    "No proposal is created by this UI.".into()
}

/// Safety text.
pub fn routing_safety_text() -> String {
    workflow_next_action_routing_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    
    use dioxus::prelude::*;

    /// Empty state.
    pub fn render_next_action_routing_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No next-action routing records"
            }
        }
    }

    /// Loading state.
    pub fn render_next_action_routing_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading next-action routing…"
            }
        }
    }

    /// Error state.
    pub fn render_next_action_routing_error_state(message: &str) -> Element {
        let style = banner_style(UiTone::Error);
        let safe: String = message.chars().take(200).collect();
        rsx! {
            div { style: "{style}",
                "Routing load error: {safe}"
            }
        }
    }

    /// Safety banner.
    pub fn render_next_action_routing_safety_banner() -> Element {
        let text = routing_safety_text();
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                "⚠ {text}"
            }
        }
    }

    /// Routing summary card.
    pub fn render_next_action_routing_summary(row: &WorkflowNextActionRoutingSummaryRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let tone = routing_status_tone(&row.status);
        let label = routing_status_label(&row.status);
        let badge_s = badge_style(tone);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_SM, typo::TEXT_SM,
        );
        let label_s = format!("min-width: 140px; color: {};", colors::TEXT_PRIMARY);
        let _value_s = format!("color: {};", colors::TEXT_MUTED);
        let note1 = routing_suggested_note();
        let note2 = routing_no_execution_note();
        let note3 = routing_no_proposal_created_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            typo::TEXT_XS, colors::TEXT_MUTED, spacing::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Suggested next action ({row.routing_id})"
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Status" }
                    span { style: "{badge_s}", "{label}" }
                }
                div { style: "{note_s}", "{note1}" }
                div { style: "{note_s}", "{note2}" }
                div { style: "{note_s}", "{note3}" }
            }
        }
    }

    /// Predicate rows (Patch 5).
    pub fn render_next_action_routing_predicate_rows(predicates: &[WorkflowNextActionRoutingPredicateRow]) -> Element {
        if predicates.is_empty() {
            return rsx! { div {} };
        }
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_SM, typo::TEXT_SM,
        );
        let name_s = format!("min-width: 260px; color: {};", colors::TEXT_PRIMARY);

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Next-action routing predicates"
                }
                for pred in predicates {
                    div { style: "{row_s}",
                        span { style: "{name_s}", "{pred.predicate}" }
                        {
                            let pred_color = if pred.passed { "#2d6a2d" } else { "#721c24" };
                            let pred_label = if pred.passed { "Passed" } else { "Failed" };
                            rsx! { span { style: "min-width: 80px; color: {pred_color};", "{pred_label}" } }
                        }
                        span { style: "color: #888;", "{pred.reason}" }
                    }
                }
            }
        }
    }

    /// Route link card.
    pub fn render_route_link(link: &WorkflowNextActionRouteLinkRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_SM, typo::TEXT_SM,
        );
        let label_s = format!("min-width: 140px; color: {};", colors::TEXT_PRIMARY);
        let value_s = format!("color: {};", colors::TEXT_MUTED);

        rsx! {
            div { style: "{card_s}",
                div { style: "{row_s}",
                    span { style: "{label_s}", "Routing" }
                    span { style: "{value_s}", "{link.routing_id}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Created route" }
                    span { style: "{value_s}", "{link.route_id}" }
                }
            }
        }
    }

    /// Full routing panel with predicates and route link.
    pub fn render_next_action_routing_panel(state: &WorkflowNextActionRoutingUiState) -> Element {
        rsx! {
            div {
                if let Some(ref row) = state.latest_routing {
                    { render_next_action_routing_summary(row) }
                } else {
                    { render_next_action_routing_empty_state() }
                }
                if !state.predicates.is_empty() {
                    { render_next_action_routing_predicate_rows(&state.predicates) }
                }
                if let Some(ref link) = state.route_link {
                    { render_route_link(link) }
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
    fn routing_status_tone_info_for_routed() {
        assert_eq!(UiTone::Info, routing_status_tone("routed"));
    }

    #[test]
    fn routing_status_tone_error_for_blocked() {
        assert_eq!(UiTone::Error, routing_status_tone("blocked"));
    }

    #[test]
    fn routing_status_label_says_recorded() {
        assert!(routing_status_label("routed").contains("recorded"));
    }

    // ── Patch 2 guard tests ──

    #[test]
    fn next_action_ui_copy_says_suggested_not_executable() {
        let note = routing_no_execution_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("no action execution"), "got: {note}");
    }

    #[test]
    fn next_action_ui_copy_says_no_proposal_created() {
        let note = routing_no_proposal_created_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("no proposal"), "got: {note}");
    }

    #[test]
    fn next_action_ui_copy_contains_no_execution_verbs() {
        let all_copy = vec![
            routing_status_label("routed"),
            routing_suggested_note(),
            routing_no_execution_note(),
            routing_no_proposal_created_note(),
            routing_safety_text(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("run now"), "copy: {text}");
            assert!(!lower.contains("execute now"), "copy: {text}");
            assert!(!lower.contains("perform"), "copy: {text}");
            assert!(!lower.contains("submit action"), "copy: {text}");
        }
    }
}
