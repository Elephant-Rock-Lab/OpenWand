//! Routing readiness desktop UI components.
//!
//! Read-only display of recorded routing readiness state using Wave 52A
//! design-system tokens. Displays readiness status, textual predicate rows,
//! next-action review, route preview, and explicit no-routing-action copy.
//!
//! Components accept already-prepared state. They do not choose routes,
//! advance workflow state, approve actions, execute actions, append trace,
//! write memory, or create workflow/evidence records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_routing_readiness_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for readiness status.
pub fn readiness_status_tone(status: &str) -> UiTone {
    match status {
        "ready" => UiTone::Info,
        "blocked" => UiTone::Error,
        "inconclusive" => UiTone::Warning,
        _ => UiTone::Neutral,
    }
}

/// Human-readable label.
pub fn readiness_status_label(status: &str) -> String {
    match status {
        "ready" => "Ready (recorded)".into(),
        "blocked" => "Blocked (recorded)".into(),
        "inconclusive" => "Inconclusive (recorded)".into(),
        _ => format!("Recorded: {}", status),
    }
}

/// Patch 2: not-executable copy.
pub fn readiness_not_routing_action_note() -> String {
    "Routing readiness is evidence only.".into()
}

/// Safety text.
pub fn readiness_safety_text() -> String {
    workflow_routing_readiness_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    
    use dioxus::prelude::*;

    /// Empty state.
    pub fn render_routing_readiness_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No routing readiness records"
            }
        }
    }

    /// Loading state.
    pub fn render_routing_readiness_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading routing readiness…"
            }
        }
    }

    /// Error state.
    pub fn render_routing_readiness_error_state(message: &str) -> Element {
        let style = banner_style(UiTone::Error);
        let safe: String = message.chars().take(200).collect();
        rsx! {
            div { style: "{style}",
                "Readiness load error: {safe}"
            }
        }
    }

    /// Safety banner.
    pub fn render_routing_readiness_safety_banner() -> Element {
        let text = readiness_safety_text();
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                "⚠ {text}"
            }
        }
    }

    /// Readiness summary card.
    pub fn render_routing_readiness_summary(row: &WorkflowRoutingReadinessRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let tone = readiness_status_tone(&row.status);
        let label = readiness_status_label(&row.status);
        let badge_s = badge_style(tone);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_SM, typo::TEXT_SM,
        );
        let label_s = format!("min-width: 140px; color: {};", colors::TEXT_PRIMARY);
        let _value_s = format!("color: {};", colors::TEXT_MUTED);
        let note = readiness_not_routing_action_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            typo::TEXT_XS, colors::TEXT_MUTED, spacing::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Routing readiness ({row.readiness_id})"
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Status" }
                    span { style: "{badge_s}", "{label}" }
                }
                div { style: "{note_s}", "{note}" }
            }
        }
    }

    /// Predicate rows (Patch 5: textual-first).
    pub fn render_routing_readiness_predicate_rows(predicates: &[WorkflowRoutingReadinessPredicateRow]) -> Element {
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
                    "Routing readiness predicates"
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

    /// Next-action review row inside readiness.
    pub fn render_routing_readiness_review(review: &WorkflowNextActionReviewRow) -> Element {
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
                    span { style: "{label_s}", "Review" }
                    span { style: "{value_s}", "{review.review_id}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Decision" }
                    span { style: "{value_s}", "{review.decision}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Reviewer" }
                    span { style: "{value_s}", "{review.reviewer}" }
                }
            }
        }
    }

    /// Route preview display.
    pub fn render_route_preview(preview: &WorkflowRouteRequestPreviewRow) -> Element {
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
                    span { style: "{label_s}", "Route preview" }
                    span { style: "{value_s}", "{preview.action_request_id}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Stage" }
                    span { style: "{value_s}", "{preview.stage_id}" }
                }
                if preview.descriptive_only {
                    div { style: "font-size: 12px; color: #888; font-style: italic;",
                        "Descriptive only, not a route request"
                    }
                }
            }
        }
    }

    /// Full readiness panel with review, predicates, and preview.
    pub fn render_routing_readiness_panel(state: &WorkflowRoutingReadinessUiState) -> Element {
        rsx! {
            div {
                if let Some(ref row) = state.latest_readiness {
                    { render_routing_readiness_summary(row) }
                }
                if !state.predicates.is_empty() {
                    { render_routing_readiness_predicate_rows(&state.predicates) }
                }
                if let Some(ref review) = state.latest_review {
                    { render_routing_readiness_review(review) }
                }
                if let Some(ref preview) = state.route_preview {
                    { render_route_preview(preview) }
                }
                for fb in &state.feedback {
                    div { style: "font-size: 12px; color: #856404; padding: 2px 0;",
                        "{fb}"
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
    fn readiness_status_tone_info_for_ready() {
        assert_eq!(UiTone::Info, readiness_status_tone("ready"));
    }

    #[test]
    fn readiness_status_tone_error_for_blocked() {
        assert_eq!(UiTone::Error, readiness_status_tone("blocked"));
    }

    #[test]
    fn readiness_status_label_says_recorded() {
        assert!(readiness_status_label("ready").contains("recorded"));
    }

    #[test]
    fn readiness_not_routing_action_note_contents() {
        let note = readiness_not_routing_action_note();
        assert!(note.to_lowercase().contains("evidence only"));
    }

    #[test]
    fn readiness_ui_copy_contains_no_action_verbs() {
        let all_copy = vec![
            readiness_status_label("ready"),
            readiness_status_label("blocked"),
            readiness_not_routing_action_note(),
            readiness_safety_text(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("choose"), "copy: {text}");
            assert!(!lower.contains("dispatch"), "copy: {text}");
        }
    }
}
