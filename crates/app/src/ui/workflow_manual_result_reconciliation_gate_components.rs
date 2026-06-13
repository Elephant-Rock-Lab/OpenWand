//! Reconciliation gate desktop UI components.
//!
//! Read-only display of recorded reconciliation gate state using Wave 52A
//! design-system tokens. Displays gate status, revision info, hashes, and
//! explicit no-reconciliation-action copy (Patch 6).
//!
//! Components accept already-prepared state. They do not reconcile outcomes,
//! create run revisions, verify truth, certify evidence, execute tools,
//! append trace, write memory, mutate workflow state, or create
//! workflow/evidence records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_manual_result_reconciliation_gate_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for gate status.
pub fn gate_status_tone(status: &str) -> UiTone {
    match status {
        "reconciled" => UiTone::Info,
        "blocked" => UiTone::Error,
        "pending" => UiTone::Warning,
        "skipped" => UiTone::Neutral,
        _ => UiTone::Neutral,
    }
}

/// Human-readable label for gate status.
pub fn gate_status_label(status: &str) -> String {
    match status {
        "reconciled" => "Reconciled (recorded)".into(),
        "blocked" => "Blocked (recorded)".into(),
        "pending" => "Pending (recorded)".into(),
        "skipped" => "Skipped (recorded)".into(),
        _ => format!("Recorded: {}", status),
    }
}

/// Patch 6: explicit no-action copy.
pub fn gate_recorded_state_note() -> String {
    "Recorded gate state.".into()
}

pub fn gate_no_reconciliation_action_note() -> String {
    "No reconciliation action is available here.".into()
}

pub fn gate_no_revision_created_note() -> String {
    "No run revision is created by this UI.".into()
}

/// Safety text.
pub fn gate_safety_text() -> String {
    gate_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::workflow_evidence_chain_inspector_components::chain_hash_display;
    use crate::ui::components::*;
    
    use dioxus::prelude::*;

    /// Empty state.
    pub fn render_gate_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No reconciliation gate records"
            }
        }
    }

    /// Loading state.
    pub fn render_gate_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading reconciliation gate…"
            }
        }
    }

    /// Error state.
    pub fn render_gate_error_state(message: &str) -> Element {
        let style = banner_style(UiTone::Error);
        let safe: String = message.chars().take(200).collect();
        rsx! {
            div { style: "{style}",
                "Gate load error: {safe}"
            }
        }
    }

    /// Safety banner.
    pub fn render_gate_safety_banner() -> Element {
        let text = gate_safety_text();
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                "⚠ {text}"
            }
        }
    }

    /// Gate summary card (Patch 3: labeled "Latest recorded" when single).
    /// Patch 6: explicit no-action copy.
    pub fn render_gate_summary(row: &WorkflowManualResultReconciliationGateSummaryRow, is_latest: bool) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let tone = gate_status_tone(&row.status);
        let label = gate_status_label(&row.status);
        let badge_s = badge_style(tone);
        let header_label = if is_latest { "Latest recorded gate" } else { "Reconciliation gate" };
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_SM, typo::TEXT_SM,
        );
        let label_s = format!("min-width: 140px; color: {};", colors::TEXT_PRIMARY);
        let value_s = format!("color: {};", colors::TEXT_MUTED);
        let note1 = gate_recorded_state_note();
        let note2 = gate_no_reconciliation_action_note();
        let note3 = gate_no_revision_created_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            typo::TEXT_XS, colors::TEXT_MUTED, spacing::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "{header_label} ({row.gate_id})"
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Status" }
                    span { style: "{badge_s}", "{label}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Reconciled by" }
                    span { style: "{value_s}", "{row.reconciled_by}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Manual result" }
                    span { style: "{value_s}", "{row.manual_result_id}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Stage" }
                    span { style: "{value_s}", "{row.stage_id}" }
                }
                if let Some(ref rev) = row.revision_id {
                    div { style: "{row_s}",
                        span { style: "{label_s}", "Revision" }
                        span { style: "{value_s}", "{rev}" }
                    }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Readiness" }
                    span { style: "{value_s}", "{row.readiness_id}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Readiness hash" }
                    span { style: "{value_s}", "{chain_hash_display(&row.readiness_hash)}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Result hash" }
                    span { style: "{value_s}", "{chain_hash_display(&row.manual_result_hash)}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Review hash" }
                    span { style: "{value_s}", "{chain_hash_display(&row.manual_result_review_hash)}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Cmd review hash" }
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
                div { style: "{note_s}", "{note1}" }
                div { style: "{note_s}", "{note2}" }
                div { style: "{note_s}", "{note3}" }
            }
        }
    }

    /// Multiple gate records.
    pub fn render_gate_list(records: &[WorkflowManualResultReconciliationGateSummaryRow]) -> Element {
        if records.is_empty() {
            return render_gate_empty_state();
        }
        rsx! {
            div {
                for (i, row) in records.iter().enumerate() {
                    { render_gate_summary(row, i == 0) }
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
    fn gate_status_tone_info_for_reconciled() {
        assert_eq!(UiTone::Info, gate_status_tone("reconciled"));
    }

    #[test]
    fn gate_status_tone_error_for_blocked() {
        assert_eq!(UiTone::Error, gate_status_tone("blocked"));
    }

    #[test]
    fn gate_status_tone_warning_for_pending() {
        assert_eq!(UiTone::Warning, gate_status_tone("pending"));
    }

    #[test]
    fn gate_status_label_says_recorded() {
        let label = gate_status_label("reconciled");
        assert!(label.contains("recorded"), "got: {label}");
    }

    // ── Patch 6 guard tests ──

    #[test]
    fn gate_card_says_recorded_state() {
        let note = gate_recorded_state_note();
        assert!(note.to_lowercase().contains("recorded"), "got: {note}");
    }

    #[test]
    fn gate_card_says_no_reconciliation_action_available() {
        let note = gate_no_reconciliation_action_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("no reconciliation action"), "got: {note}");
        assert!(lower.contains("available"), "got: {note}");
    }

    #[test]
    fn gate_card_says_no_run_revision_created_by_ui() {
        let note = gate_no_revision_created_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("no run revision"), "got: {note}");
        assert!(lower.contains("created"), "got: {note}");
    }

    #[test]
    fn gate_ui_copy_contains_no_action_verbs() {
        let all_copy = vec![
            gate_status_label("reconciled"),
            gate_status_label("blocked"),
            gate_recorded_state_note(),
            gate_no_reconciliation_action_note(),
            gate_no_revision_created_note(),
            gate_safety_text(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("certified"), "copy contains 'certified': {text}");
            assert!(!lower.contains("trusted"), "copy contains 'trusted': {text}");
            assert!(!lower.contains("approved"), "copy contains 'approved': {text}");
        }
    }
}
