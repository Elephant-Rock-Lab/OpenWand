//! Manual result review desktop UI components.
//!
//! Read-only display of recorded manual result review decisions using Wave 52A
//! design-system tokens. Displays review decisions, acceptance snapshots,
//! hashes, and explicit not-certification copy.
//!
//! Components accept already-prepared state. They do not accept/reject reviews,
//! verify truth, certify evidence, execute tools, append trace, write memory,
//! mutate workflow state, or create workflow/evidence records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_manual_result_review_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for a review decision.
pub fn review_decision_tone(decision: &str) -> UiTone {
    match decision {
        "accepted" => UiTone::Info,
        "rejected" => UiTone::Error,
        "changes_requested" => UiTone::Warning,
        "noted" => UiTone::Neutral,
        _ => UiTone::Neutral,
    }
}

/// Human-readable label for review decision.
pub fn review_decision_label(decision: &str) -> String {
    match decision {
        "accepted" => "Accepted (recorded)".into(),
        "rejected" => "Rejected (recorded)".into(),
        "changes_requested" => "Changes requested (recorded)".into(),
        "noted" => "Noted (recorded)".into(),
        _ => format!("Recorded: {}", decision),
    }
}

/// Explicit not-certification copy (Patch 2).
pub fn review_not_certification_note() -> String {
    "Recorded review decision, not truth certification.".into()
}

/// Safety text.
pub fn review_safety_text() -> String {
    workflow_manual_result_review_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    
    use dioxus::prelude::*;

    /// Empty state.
    pub fn render_review_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No manual result reviews recorded"
            }
        }
    }

    /// Loading state.
    pub fn render_review_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading manual result reviews…"
            }
        }
    }

    /// Error state.
    pub fn render_review_error_state(message: &str) -> Element {
        let style = banner_style(UiTone::Error);
        let safe: String = message.chars().take(200).collect();
        rsx! {
            div { style: "{style}",
                "Review load error: {safe}"
            }
        }
    }

    /// Safety banner.
    pub fn render_review_safety_banner() -> Element {
        let text = review_safety_text();
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                "⚠ {text}"
            }
        }
    }

    /// Review summary card (Patch 3: labeled "Latest recorded" when single).
    pub fn render_review_summary(row: &WorkflowManualResultReviewSummaryRow, is_latest: bool) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let tone = review_decision_tone(&row.decision);
        let label = review_decision_label(&row.decision);
        let badge_s = badge_style(tone);
        let header_label = if is_latest { "Latest recorded review" } else { "Manual result review" };
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_SM, typo::TEXT_SM,
        );
        let label_s = format!("min-width: 140px; color: {};", colors::TEXT_PRIMARY);
        let value_s = format!("color: {};", colors::TEXT_MUTED);
        let note = review_not_certification_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            typo::TEXT_XS, colors::TEXT_MUTED, spacing::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "{header_label} ({row.review_id})"
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Reviewer" }
                    span { style: "{value_s}", "{row.reviewer}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Decision" }
                    span { style: "{badge_s}", "{label}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Manual result" }
                    span { style: "{value_s}", "{row.manual_result_id}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Result hash" }
                    span { style: "{value_s}", "{chain_hash_display(&row.manual_result_hash)}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Review hash" }
                    span { style: "{value_s}", "{chain_hash_display(&row.command_review_hash)}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Composer hash" }
                    span { style: "{value_s}", "{chain_hash_display(&row.command_composer_hash)}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Descriptor hash" }
                    span { style: "{value_s}", "{chain_hash_display(&row.command_descriptor_hash)}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Loop hash" }
                    span { style: "{value_s}", "{chain_hash_display(&row.loop_controller_hash)}" }
                }
                div { style: "{note_s}", "{note}" }
            }
        }
    }

    /// Acceptance snapshot card.
    pub fn render_acceptance_snapshot(acceptance: &WorkflowManualResultReviewAcceptanceRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_SM, typo::TEXT_SM,
        );
        let label_s = format!("min-width: 200px; color: {};", colors::TEXT_PRIMARY);

        let flags = [
            ("Accepts reported evidence", acceptance.accepts_reported_evidence),
            ("Verifies external state", acceptance.verifies_external_state),
            ("Reconciles workflow state", acceptance.reconciles_workflow_state),
            ("Result verified by OpenWand", acceptance.result_verified_by_openwand),
        ];

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Acceptance snapshot"
                }
                for (name, value) in flags {
                    div { style: "{row_s}",
                        span { style: "{label_s}", "{name}" }
                        {
                            let val_color = if value { "#2d6a2d" } else { "#888" };
                            let val_label = if value { "Yes" } else { "No" };
                            rsx! { span { style: "color: {val_color};", "{val_label}" } }
                        }
                    }
                }
            }
        }
    }

    /// Multiple review records.
    pub fn render_review_list(reviews: &[WorkflowManualResultReviewSummaryRow]) -> Element {
        if reviews.is_empty() {
            return render_review_empty_state();
        }
        rsx! {
            div {
                for (i, row) in reviews.iter().enumerate() {
                    { render_review_summary(row, i == 0) }
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
    fn review_decision_tone_info_for_accepted() {
        assert_eq!(UiTone::Info, review_decision_tone("accepted"));
    }

    #[test]
    fn review_decision_tone_error_for_rejected() {
        assert_eq!(UiTone::Error, review_decision_tone("rejected"));
    }

    #[test]
    fn review_decision_tone_warning_for_changes_requested() {
        assert_eq!(UiTone::Warning, review_decision_tone("changes_requested"));
    }

    #[test]
    fn review_decision_label_says_recorded() {
        let label = review_decision_label("accepted");
        assert!(label.contains("recorded"), "got: {label}");
    }

    #[test]
    fn review_not_certification_note_contents() {
        let note = review_not_certification_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("not truth certification"), "got: {note}");
    }

    // ── Guard tests (Patch 2) ──

    #[test]
    fn manual_review_ui_copy_says_recorded_not_certification() {
        let note = review_not_certification_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("recorded"), "got: {note}");
        assert!(lower.contains("not truth certification"), "got: {note}");
    }

    #[test]
    fn review_ui_copy_contains_no_action_verbs() {
        let all_copy = vec![
            review_decision_label("accepted"),
            review_decision_label("rejected"),
            review_not_certification_note(),
            review_safety_text(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("certified"), "copy contains 'certified': {text}");
            assert!(!lower.contains("trusted"), "copy contains 'trusted': {text}");
            assert!(!lower.contains("approved"), "copy contains 'approved': {text}");
        }
    }
}
