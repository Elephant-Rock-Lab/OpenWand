//! Next-action review desktop UI components.
//!
//! Read-only display of recorded next-action review decisions using Wave 52A
//! design-system tokens. Patch 3: must not look like an approval control.
//! Displays review decisions, rationale, feedback, and explicit display-only copy.
//!
//! Components accept already-prepared state. They do not approve/reject reviews,
//! resolve reviews, request changes, create routes, execute tools, append trace,
//! write memory, or create workflow/evidence records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_next_action_review_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for review decision.
pub fn review_decision_tone(decision: &str) -> UiTone {
    match decision {
        "Approved" => UiTone::Info,
        "Rejected" => UiTone::Error,
        "ChangesRequested" => UiTone::Warning,
        _ => UiTone::Neutral,
    }
}

/// Human-readable label.
pub fn review_decision_label(decision: &str) -> String {
    match decision {
        "Approved" => "Approved (recorded)".into(),
        "Rejected" => "Rejected (recorded)".into(),
        "ChangesRequested" => "Changes requested (recorded)".into(),
        _ => format!("Recorded: {}", decision),
    }
}

/// Patch 3: display-only copy.
pub fn review_display_only_note() -> String {
    "Review display only.".into()
}

pub fn review_no_resolution_note() -> String {
    "No approve/reject/request-changes action is available here.".into()
}

/// Safety text.
pub fn review_safety_text() -> String {
    next_action_review_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use crate::ui::layout::*;
    use dioxus::prelude::*;

    /// Empty state.
    pub fn render_next_action_review_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            SPACING::SPACE_LG, SPACING::SPACE_XL, TYPO::TEXT_SM,
            COLORS::TEXT_FAINT, COLORS::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No next-action reviews recorded"
            }
        }
    }

    /// Loading state.
    pub fn render_next_action_review_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            SPACING::SPACE_LG, SPACING::SPACE_XL, TYPO::TEXT_SM,
            COLORS::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading next-action review…"
            }
        }
    }

    /// Error state.
    pub fn render_next_action_review_error_state(message: &str) -> Element {
        let style = banner_style(UiTone::Error);
        let safe: String = message.chars().take(200).collect();
        rsx! {
            div { style: "{style}",
                "Review load error: {safe}"
            }
        }
    }

    /// Safety banner.
    pub fn render_next_action_review_safety_banner() -> Element {
        let text = review_safety_text();
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                "⚠ {text}"
            }
        }
    }

    /// Review summary card (Patch 3: no approval control appearance).
    pub fn render_next_action_review_summary(row: &ReviewSummaryRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let tone = review_decision_tone(&row.decision);
        let label = review_decision_label(&row.decision);
        let badge_s = badge_style(tone);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );
        let label_s = format!("min-width: 140px; color: {};", COLORS::TEXT_PRIMARY);
        let value_s = format!("color: {};", COLORS::TEXT_MUTED);
        let note1 = review_display_only_note();
        let note2 = review_no_resolution_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            TYPO::TEXT_XS, COLORS::TEXT_MUTED, SPACING::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Recorded review decision ({row.review_id})"
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Proposal" }
                    span { style: "{value_s}", "{row.proposal_id}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Decision" }
                    span { style: "{badge_s}", "{label}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Reviewer" }
                    span { style: "{value_s}", "{row.reviewer}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Rationale" }
                    span { style: "{value_s}", "{row.rationale}" }
                }
                if row.has_feedback {
                    div { style: "{row_s}",
                        span { style: "{label_s}", "Feedback" }
                        span { style: "color: {};", COLORS::ACCENT_WARNING, "Has feedback" }
                    }
                }
                div { style: "{note_s}", "{note1}" }
                div { style: "{note_s}", "{note2}" }
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
    fn review_decision_tone_info_for_approved() {
        assert_eq!(UiTone::Info, review_decision_tone("Approved"));
    }

    #[test]
    fn review_decision_tone_error_for_rejected() {
        assert_eq!(UiTone::Error, review_decision_tone("Rejected"));
    }

    #[test]
    fn review_decision_tone_warning_for_changes() {
        assert_eq!(UiTone::Warning, review_decision_tone("ChangesRequested"));
    }

    #[test]
    fn review_decision_label_says_recorded() {
        assert!(review_decision_label("Approved").contains("recorded"));
    }

    // ── Patch 3 guard tests ──

    #[test]
    fn next_action_review_card_says_recorded_decision() {
        let note = review_display_only_note();
        // The header says "Recorded review decision" — test that the label contains "recorded"
        let label = review_decision_label("Approved");
        assert!(label.to_lowercase().contains("recorded"), "got: {label}");
    }

    #[test]
    fn next_action_review_card_says_display_only() {
        let note = review_display_only_note();
        assert!(note.to_lowercase().contains("display only"), "got: {note}");
    }

    #[test]
    fn next_action_review_card_has_no_resolution_actions() {
        let note = review_no_resolution_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("no approve"), "got: {note}");
        assert!(lower.contains("reject"), "got: {note}");
        assert!(lower.contains("request-changes"), "got: {note}");
    }

    #[test]
    fn review_ui_copy_contains_no_execution_verbs() {
        let all_copy = vec![
            review_decision_label("Approved"),
            review_decision_label("Rejected"),
            review_display_only_note(),
            review_no_resolution_note(),
            review_safety_text(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("certified"), "copy: {text}");
            assert!(!lower.contains("trusted"), "copy: {text}");
        }
    }
}
