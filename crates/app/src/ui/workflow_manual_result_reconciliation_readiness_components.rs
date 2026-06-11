//! Reconciliation readiness desktop UI components.
//!
//! Read-only display of recorded reconciliation readiness state using Wave 52A
//! design-system tokens. Displays readiness status, textual predicate rows
//! (Patch 5: name/passed/reason, not just color), hashes, and explicit
//! not-execution-permission copy.
//!
//! Components accept already-prepared state. They do not reconcile outcomes,
//! verify truth, create run revisions, execute tools, append trace, write memory,
//! mutate workflow state, or create workflow/evidence records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_evidence_chain_inspector_components::chain_hash_display;
use crate::ui::workflow_manual_result_reconciliation_readiness_state::*;

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

/// Human-readable label for readiness status.
pub fn readiness_status_label(status: &str) -> String {
    match status {
        "ready" => "Ready (recorded)".into(),
        "blocked" => "Blocked (recorded)".into(),
        "inconclusive" => "Inconclusive (recorded)".into(),
        _ => format!("Recorded: {}", status),
    }
}

/// Explicit not-execution-permission copy (Patch 2).
pub fn readiness_not_execution_permission_note() -> String {
    "Recorded eligibility, not execution permission.".into()
}

/// Predicate status label.
pub fn predicate_status_label(passed: bool) -> String {
    if passed { "Passed".into() } else { "Failed".into() }
}

/// Safety text.
pub fn readiness_safety_text() -> String {
    workflow_reconciliation_readiness_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use crate::ui::layout::*;
    use dioxus::prelude::*;

    /// Empty state.
    pub fn render_readiness_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No reconciliation readiness records"
            }
        }
    }

    /// Loading state.
    pub fn render_readiness_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading reconciliation readiness…"
            }
        }
    }

    /// Error state.
    pub fn render_readiness_error_state(message: &str) -> Element {
        let style = banner_style(UiTone::Error);
        let safe: String = message.chars().take(200).collect();
        rsx! {
            div { style: "{style}",
                "Readiness load error: {safe}"
            }
        }
    }

    /// Safety banner.
    pub fn render_readiness_safety_banner() -> Element {
        let text = readiness_safety_text();
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                "⚠ {text}"
            }
        }
    }

    /// Readiness summary card (Patch 3: labeled "Latest recorded" when single).
    pub fn render_readiness_summary(row: &WorkflowManualResultReconciliationReadinessSummaryRow, is_latest: bool) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let tone = readiness_status_tone(&row.status);
        let label = readiness_status_label(&row.status);
        let badge_s = badge_style(tone);
        let header_label = if is_latest { "Latest recorded readiness" } else { "Reconciliation readiness" };
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_SM, typo::TEXT_SM,
        );
        let label_s = format!("min-width: 140px; color: {};", colors::TEXT_PRIMARY);
        let value_s = format!("color: {};", colors::TEXT_MUTED);
        let note = readiness_not_execution_permission_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            typo::TEXT_XS, colors::TEXT_MUTED, spacing::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "{header_label} ({row.readiness_id})"
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Evaluator" }
                    span { style: "{value_s}", "{row.evaluator}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Status" }
                    span { style: "{badge_s}", "{label}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Manual result" }
                    span { style: "{value_s}", "{row.manual_result_id}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Review" }
                    span { style: "{value_s}", "{row.manual_result_review_id}" }
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
                div { style: "{note_s}", "{note}" }
            }
        }
    }

    /// Predicate rows — textual, not just color-coded (Patch 5).
    pub fn render_predicate_rows(predicates: &[ReadinessPredicateDisplayRow]) -> Element {
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
        let status_s = |passed: bool| {
            format!("min-width: 80px; color: {};", if passed { colors::SUCCESS_TEXT } else { colors::ERROR_TEXT })
        };

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Readiness predicates"
                }
                for pred in predicates {
                    div { style: "{row_s}",
                        span { style: "{name_s}", "{pred.name}" }
                        span { style: "{status_s(pred.passed)}", "{predicate_status_label(pred.passed)}" }
                        span { style: "color: #888;", "{pred.reason}" }
                    }
                }
            }
        }
    }

    /// Multiple readiness records.
    pub fn render_readiness_list(records: &[WorkflowManualResultReconciliationReadinessSummaryRow]) -> Element {
        if records.is_empty() {
            return render_readiness_empty_state();
        }
        rsx! {
            div {
                for (i, row) in records.iter().enumerate() {
                    { render_readiness_summary(row, i == 0) }
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
    fn readiness_status_tone_warning_for_inconclusive() {
        assert_eq!(UiTone::Warning, readiness_status_tone("inconclusive"));
    }

    #[test]
    fn readiness_status_label_says_recorded() {
        let label = readiness_status_label("ready");
        assert!(label.contains("recorded"), "got: {label}");
    }

    #[test]
    fn readiness_not_execution_permission_note_contents() {
        let note = readiness_not_execution_permission_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("not execution permission"), "got: {note}");
    }

    #[test]
    fn predicate_status_label_text() {
        assert_eq!("Passed", predicate_status_label(true));
        assert_eq!("Failed", predicate_status_label(false));
    }

    // ── Guard tests (Patch 2) ──

    #[test]
    fn readiness_ui_copy_says_readiness_not_execution_permission() {
        let note = readiness_not_execution_permission_note();
        assert!(note.to_lowercase().contains("not execution permission"));
    }

    #[test]
    fn readiness_ui_copy_contains_no_action_verbs() {
        let all_copy = vec![
            readiness_status_label("ready"),
            readiness_status_label("blocked"),
            readiness_not_execution_permission_note(),
            readiness_safety_text(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("certified"), "copy contains 'certified': {text}");
            assert!(!lower.contains("approved"), "copy contains 'approved': {text}");
            assert!(!lower.contains("trusted"), "copy contains 'trusted': {text}");
        }
    }
}
