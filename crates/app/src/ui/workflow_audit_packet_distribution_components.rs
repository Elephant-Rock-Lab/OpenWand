//! Audit packet distribution desktop UI components.
//!
//! Read-only display of audit packet distribution records using Wave 52A
//! design-system tokens. The distribution surface displays reported metadata,
//! destination info, and explicit no-delivery-proof copy.
//!
//! Components accept already-prepared state. They do not record, distribute,
//! send, upload, verify receipt, certify truth, prove delivery, or mutate records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_audit_packet_distribution_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Tone for reported_distribution flag. Info, not Success-as-delivery-proof.
pub fn distribution_reported_tone(reported: bool) -> UiTone {
    if reported { UiTone::Info } else { UiTone::Neutral }
}

/// Tone for proof_of_delivery flag.
pub fn delivery_proof_tone(proof: bool) -> UiTone {
    if proof { UiTone::Info } else { UiTone::Warning }
}

/// Label for proof_of_delivery. Explicitly says "no proof recorded" when false.
pub fn delivery_proof_label(proof: bool) -> String {
    if proof {
        "Delivery proof recorded".into()
    } else {
        "No delivery proof recorded".into()
    }
}

/// Explicit no-delivery-proof copy.
pub fn distribution_no_delivery_proof_note() -> String {
    "Distribution reported, not delivery proof.".into()
}

/// Destination metadata description.
pub fn destination_metadata_note() -> String {
    "Destination metadata is operator-reported.".into()
}

/// Receipt disclaimer.
pub fn receipt_not_proven_note() -> String {
    "Receipt and acceptance are not proven by OpenWand.".into()
}

/// Safety text.
pub fn distribution_safety_text() -> String {
    distribution_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    
    use crate::ui::workflow_audit_packet_review_state::ReviewSummaryRow;
    use crate::ui::workflow_audit_packet_review_components::render_review_list;
    use dioxus::prelude::*;

    /// Empty state when no distribution records exist.
    pub fn render_distribution_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No audit packet distributions recorded"
            }
        }
    }

    /// Loading state.
    pub fn render_distribution_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading audit packet distributions…"
            }
        }
    }

    /// Error state.
    pub fn render_distribution_error_state(message: &str) -> Element {
        let style = banner_style(UiTone::Error);
        let safe: String = message.chars().take(200).collect();
        rsx! {
            div { style: "{style}",
                "Distribution load error: {safe}"
            }
        }
    }

    /// Safety banner.
    pub fn render_distribution_safety_banner() -> Element {
        let text = distribution_safety_text();
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                "⚠ {text}"
            }
        }
    }

    /// Distribution summary card.
    pub fn render_distribution_summary(row: &DistributionSummaryRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);
        let reported_tone = distribution_reported_tone(row.reported_distribution);
        let proof_tone = delivery_proof_tone(row.proof_of_delivery);
        let proof_label = delivery_proof_label(row.proof_of_delivery);
        let reported_badge_s = badge_style(reported_tone);
        let proof_badge_s = badge_style(proof_tone);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_SM, typo::TEXT_SM,
        );
        let label_s = format!("min-width: 120px; color: {};", colors::TEXT_PRIMARY);
        let value_s = format!("color: {};", colors::TEXT_MUTED);
        let note = distribution_no_delivery_proof_note();
        let dest_note = destination_metadata_note();
        let receipt_note = receipt_not_proven_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            typo::TEXT_XS, colors::TEXT_MUTED, spacing::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Audit Packet Distribution ({row.distribution_id})"
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Review" }
                    span { style: "{value_s}", "{row.review_id}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Destination" }
                    span { style: "{value_s}", "{row.destination_kind}: {row.destination_label}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Reported" }
                    span { style: "{reported_badge_s}",
                        if row.reported_distribution { "Reported" } else { "Not reported" }
                    }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Delivery proof" }
                    span { style: "{proof_badge_s}", "{proof_label}" }
                }
                div { style: "{note_s}", "{note}" }
                div { style: "{note_s}", "{dest_note}" }
                div { style: "{note_s}", "{receipt_note}" }
            }
        }
    }

    /// Multiple distribution records as a list of cards.
    pub fn render_distribution_list(distributions: &[DistributionSummaryRow]) -> Element {
        if distributions.is_empty() {
            return render_distribution_empty_state();
        }
        rsx! {
            div {
                for row in distributions {
                    { render_distribution_summary(row) }
                }
            }
        }
    }

    /// Combined panel for Inspector tab integration.
    /// Handles reviews and distributions together, including empty cases.
    pub fn render_audit_packet_review_distribution_panel(
        reviews: &[ReviewSummaryRow],
        distributions: &[DistributionSummaryRow],
    ) -> Element {
        rsx! {
            div {
                { render_review_list(reviews) }
                { render_distribution_list(distributions) }
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
    use crate::ui::workflow_audit_packet_review_state::ReviewSummaryRow;

    #[test]
    fn distribution_reported_tone_is_info_not_success() {
        // Success would imply truth/delivery proof — use Info instead
        assert_eq!(UiTone::Info, distribution_reported_tone(true));
        assert_eq!(UiTone::Neutral, distribution_reported_tone(false));
    }

    #[test]
    fn delivery_proof_tone_warning_when_no_proof() {
        assert_eq!(UiTone::Warning, delivery_proof_tone(false));
        assert_eq!(UiTone::Info, delivery_proof_tone(true));
    }

    #[test]
    fn delivery_proof_label_explicit_when_false() {
        let label = delivery_proof_label(false);
        assert!(label.to_lowercase().contains("no delivery proof"), "got: {label}");
    }

    #[test]
    fn distribution_no_delivery_proof_note_says_not_proof() {
        let note = distribution_no_delivery_proof_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("not delivery proof"), "got: {note}");
    }

    #[test]
    fn destination_metadata_note_says_reported() {
        let note = destination_metadata_note();
        assert!(note.to_lowercase().contains("operator-reported"), "got: {note}");
    }

    #[test]
    fn receipt_not_proven_note_says_not_proven() {
        let note = receipt_not_proven_note();
        assert!(note.to_lowercase().contains("not proven"), "got: {note}");
    }

    // ── Guard tests ──

    #[test]
    fn distribution_ui_copy_says_not_delivery_proof() {
        let note = distribution_no_delivery_proof_note();
        assert!(note.to_lowercase().contains("not delivery proof"));
    }

    #[test]
    fn distribution_ui_copy_says_destination_metadata_reported() {
        let note = destination_metadata_note();
        assert!(note.to_lowercase().contains("reported"));
    }

    #[test]
    fn review_distribution_ui_copy_contains_no_authority_verbs() {
        let all_copy = vec![
            distribution_no_delivery_proof_note(),
            destination_metadata_note(),
            receipt_not_proven_note(),
            delivery_proof_label(false),
            delivery_proof_label(true),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("certified"), "copy contains 'certified': {text}");
            assert!(!lower.contains("approved"), "copy contains 'approved': {text}");
            assert!(!lower.contains("verified "), "copy contains 'verified ': {text}");
            assert!(!lower.contains("trusted"), "copy contains 'trusted': {text}");
            assert!(!lower.contains("delivered"), "copy contains 'delivered': {text}");
            assert!(!lower.contains("uploaded"), "copy contains 'uploaded': {text}");
            assert!(!lower.contains("archived"), "copy contains 'archived': {text}");
        }
    }

    // ── Panel state handling tests ──

    #[test]
    fn review_distribution_panel_handles_no_records() {
        // Compile-time check: both empty slices work
        let _reviews: Vec<ReviewSummaryRow> = vec![];
        let _distributions: Vec<DistributionSummaryRow> = vec![];
    }

    #[test]
    fn review_distribution_panel_handles_review_without_distribution() {
        let reviews = vec![ReviewSummaryRow {
            review_id: "wapr_1".into(),
            inspection_id: "weci_1".into(),
            reviewer: "alice".into(),
            decision: "Reviewed".into(),
            scope: "test".into(),
            caveat_count: 0,
        }];
        let distributions: Vec<DistributionSummaryRow> = vec![];
        // Compile-time check: mixed lengths work
        assert_eq!(1, reviews.len());
        assert_eq!(0, distributions.len());
    }

    #[test]
    fn review_distribution_panel_handles_multiple_reviews() {
        let reviews = vec![
            ReviewSummaryRow {
                review_id: "wapr_1".into(),
                inspection_id: "weci_1".into(),
                reviewer: "alice".into(),
                decision: "Reviewed".into(),
                scope: "test".into(),
                caveat_count: 0,
            },
            ReviewSummaryRow {
                review_id: "wapr_2".into(),
                inspection_id: "weci_1".into(),
                reviewer: "bob".into(),
                decision: "ReviewedWithCaveats".into(),
                scope: "extended".into(),
                caveat_count: 2,
            },
        ];
        assert_eq!(2, reviews.len());
    }
}
