//! Audit packet review desktop UI components.
//!
//! Read-only display of audit packet review records using Wave 52A design-system
//! tokens. The review surface displays recorded review metadata, decisions,
//! caveats, and explicit no-certification copy.
//!
//! Components accept already-prepared state. They do not record, create, export,
//! send, upload, verify receipt, certify truth, prove delivery, or mutate records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_audit_packet_review_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for a review decision.
/// Reviewed/Acknowledged/Noted → Info (not Success-as-truth).
pub fn review_decision_tone(decision: &str) -> UiTone {
    match decision {
        "ReviewedWithCaveats" => UiTone::Warning,
        "Reviewed" | "Acknowledged" | "Noted" => UiTone::Info,
        "Rejected" => UiTone::Error,
        _ => UiTone::Neutral,
    }
}

/// Human-readable label for a review decision.
pub fn review_decision_label(decision: &str) -> String {
    match decision {
        "Reviewed" => "Reviewed".into(),
        "ReviewedWithCaveats" => "Reviewed with caveats".into(),
        "Acknowledged" => "Acknowledged".into(),
        "Noted" => "Noted".into(),
        "Rejected" => "Rejected".into(),
        _ => format!("Recorded: {}", decision),
    }
}

/// Explicit no-certification copy.
pub fn review_not_certification_note() -> String {
    "Review recorded, not truth certification.".into()
}

/// Safety text.
pub fn review_safety_text() -> String {
    review_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use crate::ui::layout::*;
    use dioxus::prelude::*;

    /// Empty state when no review records exist.
    pub fn render_review_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            SPACING::SPACE_LG, SPACING::SPACE_XL, TYPO::TEXT_SM,
            COLORS::TEXT_FAINT, COLORS::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No audit packet reviews recorded"
            }
        }
    }

    /// Loading state.
    pub fn render_review_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            SPACING::SPACE_LG, SPACING::SPACE_XL, TYPO::TEXT_SM,
            COLORS::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading audit packet reviews…"
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

    /// Review summary card.
    pub fn render_review_summary(row: &ReviewSummaryRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let tone = review_decision_tone(&row.decision);
        let label = review_decision_label(&row.decision);
        let badge_s = badge_style(tone);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );
        let label_s = format!("min-width: 100px; color: {};", COLORS::TEXT_PRIMARY);
        let value_s = format!("color: {};", COLORS::TEXT_MUTED);
        let note = review_not_certification_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            TYPO::TEXT_XS, COLORS::TEXT_MUTED, SPACING::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Audit Packet Review ({row.review_id})"
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
                    span { style: "{label_s}", "Scope" }
                    span { style: "{value_s}", "{row.scope}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Inspection" }
                    span { style: "{value_s}", "{row.inspection_id}" }
                }
                if row.caveat_count > 0 {
                    div { style: "{row_s}",
                        span { style: "{label_s}", "Caveats" }
                        span { style: "{value_s}", "{row.caveat_count}" }
                    }
                }
                div { style: "{note_s}", "{note}" }
            }
        }
    }

    /// Multiple review records as a list of cards.
    pub fn render_review_list(reviews: &[ReviewSummaryRow]) -> Element {
        if reviews.is_empty() {
            return render_review_empty_state();
        }
        rsx! {
            div {
                for row in reviews {
                    { render_review_summary(row) }
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
    fn review_decision_tone_info_for_reviewed() {
        assert_eq!(UiTone::Info, review_decision_tone("Reviewed"));
        assert_eq!(UiTone::Info, review_decision_tone("Acknowledged"));
        assert_eq!(UiTone::Info, review_decision_tone("Noted"));
    }

    #[test]
    fn review_decision_tone_warning_for_caveats() {
        assert_eq!(UiTone::Warning, review_decision_tone("ReviewedWithCaveats"));
    }

    #[test]
    fn review_decision_tone_error_for_rejected() {
        assert_eq!(UiTone::Error, review_decision_tone("Rejected"));
    }

    #[test]
    fn review_decision_tone_neutral_for_unknown() {
        assert_eq!(UiTone::Neutral, review_decision_tone("SomeFutureVariant"));
    }

    #[test]
    fn review_decision_label_text() {
        assert_eq!("Reviewed with caveats", review_decision_label("ReviewedWithCaveats"));
        assert_eq!("Acknowledged", review_decision_label("Acknowledged"));
        assert_eq!("Rejected", review_decision_label("Rejected"));
    }

    #[test]
    fn review_not_certification_note_says_not_certification() {
        let note = review_not_certification_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("not truth certification") || lower.contains("not certification"), "got: {note}");
    }

    // ── Guard tests ──

    #[test]
    fn review_ui_copy_says_not_truth_certification() {
        let note = review_not_certification_note();
        assert!(note.to_lowercase().contains("not truth certification") || note.to_lowercase().contains("not certification"));
    }

    #[test]
    fn review_distribution_ui_copy_contains_no_authority_verbs() {
        let all_copy = vec![
            review_decision_label("Reviewed"),
            review_decision_label("Acknowledged"),
            review_decision_label("ReviewedWithCaveats"),
            review_not_certification_note(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("certified"), "copy contains 'certified': {text}");
            assert!(!lower.contains("approved"), "copy contains 'approved': {text}");
            assert!(!lower.contains("verified "), "copy contains 'verified ': {text}");
            assert!(!lower.contains("trusted"), "copy contains 'trusted': {text}");
            assert!(!lower.contains("delivered"), "copy contains 'delivered': {text}");
            assert!(!lower.contains("uploaded"), "copy contains 'uploaded': {text}");
        }
    }
}
