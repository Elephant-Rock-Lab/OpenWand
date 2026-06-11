//! Manual result desktop UI components.
//!
//! Read-only display of operator-reported manual result records using Wave 52A
//! design-system tokens. Displays reported outcomes, validation snapshots,
//! artifact references, hashes, and explicit not-verified copy.
//!
//! Components accept already-prepared state. They do not capture results,
//! verify truth, certify evidence, execute tools, append trace, write memory,
//! mutate workflow state, or create workflow/evidence records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_evidence_chain_inspector_components::chain_hash_display;
use crate::ui::workflow_manual_result_state::*;
use crate::ui::workflow_manual_result_review_state::WorkflowManualResultReviewSummaryRow;
use crate::ui::workflow_manual_result_reconciliation_readiness_state::WorkflowManualResultReconciliationReadinessSummaryRow;
use crate::ui::workflow_manual_result_reconciliation_gate_state::WorkflowManualResultReconciliationGateSummaryRow;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for a manual result status.
pub fn result_status_tone(status: &str) -> UiTone {
    match status {
        "reported_succeeded" => UiTone::Info,
        "reported_failed" => UiTone::Warning,
        "reported_partial" => UiTone::Warning,
        "reported_unknown" => UiTone::Neutral,
        _ => UiTone::Neutral,
    }
}

/// Human-readable label for result status.
pub fn result_status_label(status: &str) -> String {
    match status {
        "reported_succeeded" => "Reported succeeded".into(),
        "reported_failed" => "Reported failed".into(),
        "reported_partial" => "Reported partial".into(),
        "reported_unknown" => "Reported unknown".into(),
        _ => format!("Reported: {}", status),
    }
}

/// Explicit not-verified copy (Patch 2).
pub fn result_reported_not_verified_note() -> String {
    "Reported outcome, not verified execution.".into()
}

/// Validation check label.
pub fn validation_check_label(passed: bool) -> String {
    if passed { "Matched".into() } else { "Mismatch".into() }
}

/// Safety text.
pub fn result_safety_text() -> String {
    workflow_manual_result_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use crate::ui::layout::*;
    use dioxus::prelude::*;

    /// Empty state when no result record exists.
    pub fn render_result_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No manual results recorded"
            }
        }
    }

    /// Loading state.
    pub fn render_result_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading manual results…"
            }
        }
    }

    /// Error state.
    pub fn render_result_error_state(message: &str) -> Element {
        let style = banner_style(UiTone::Error);
        let safe: String = message.chars().take(200).collect();
        rsx! {
            div { style: "{style}",
                "Result load error: {safe}"
            }
        }
    }

    /// Safety banner.
    pub fn render_result_safety_banner() -> Element {
        let text = result_safety_text();
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                "⚠ {text}"
            }
        }
    }

    /// Manual result summary card (Patch 3: labeled "Latest reported" when single).
    pub fn render_result_summary(row: &WorkflowManualResultSummaryRow, is_latest: bool) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let tone = result_status_tone(&row.status);
        let label = result_status_label(&row.status);
        let badge_s = badge_style(tone);
        let header_label = if is_latest { "Latest reported manual result" } else { "Manual result" };
        let header_s = section_title_style();
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_SM, typo::TEXT_SM,
        );
        let label_s = format!("min-width: 140px; color: {};", colors::TEXT_PRIMARY);
        let value_s = format!("color: {};", colors::TEXT_MUTED);
        let note = result_reported_not_verified_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            typo::TEXT_XS, colors::TEXT_MUTED, spacing::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "{header_label} ({row.result_id})"
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Operator" }
                    span { style: "{value_s}", "{row.operator}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Status" }
                    span { style: "{badge_s}", "{label}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Caveat" }
                    span { style: "{value_s}", "{row.caveat}" }
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

    /// Artifact reference card.
    pub fn render_artifact_reference(artifact: &WorkflowManualArtifactReferenceRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_SM, typo::TEXT_SM,
        );
        let label_s = format!("min-width: 100px; color: {};", colors::TEXT_PRIMARY);
        let value_s = format!("color: {};", colors::TEXT_MUTED);

        rsx! {
            div { style: "{card_s}",
                div { style: "{row_s}",
                    span { style: "{label_s}", "Artifact" }
                    span { style: "{value_s}", "{artifact.artifact_id}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Label" }
                    span { style: "{value_s}", "{artifact.label}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Kind" }
                    span { style: "{value_s}", "{artifact.kind}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Reference" }
                    span { style: "{value_s}", "{artifact.reference}" }
                }
                if let Some(ref hash) = artifact.hash {
                    div { style: "{row_s}",
                        span { style: "{label_s}", "Operator hash" }
                        span { style: "{value_s}", "{chain_hash_display(hash)}" }
                    }
                }
            }
        }
    }

    /// Validation snapshot display.
    pub fn render_validation_snapshot(validation: &WorkflowManualResultValidationRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_title_style();
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            spacing::SPACE_MD, spacing::SPACE_SM, typo::TEXT_SM,
        );
        let label_s = format!("min-width: 180px; color: {};", colors::TEXT_PRIMARY);

        let checks = [
            ("Review acknowledged", validation.review_acknowledged),
            ("Review hash", validation.review_hash_matched),
            ("Composer hash", validation.composer_hash_matched),
            ("Descriptor hash", validation.descriptor_hash_matched),
            ("Loop hash", validation.loop_hash_matched),
        ];

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Validation snapshot"
                }
                for (name, passed) in checks {
                    div { style: "{row_s}",
                        span { style: "{label_s}", "{name}" }
                        {
                            let check_color = if passed { "#2d6a2d" } else { "#721c24" };
                            rsx! { span { style: "color: {check_color};", "{validation_check_label(passed)}" } }
                        }
                    }
                }
            }
        }
    }

    /// Multiple result records.
    pub fn render_result_list(results: &[WorkflowManualResultSummaryRow]) -> Element {
        if results.is_empty() {
            return render_result_empty_state();
        }
        rsx! {
            div {
                for (i, row) in results.iter().enumerate() {
                    { render_result_summary(row, i == 0) }
                }
            }
        }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;

// ── Linkage checking (Patch 1) ─────────────────────────────────────────

/// Check ladder linkage consistency. Returns warnings for mismatches.
/// Does NOT hide mismatched records — only warns.
pub fn check_ladder_linkage(
    results: &[WorkflowManualResultSummaryRow],
    reviews: &[WorkflowManualResultReviewSummaryRow],
    readiness: &[WorkflowManualResultReconciliationReadinessSummaryRow],
    gates: &[WorkflowManualResultReconciliationGateSummaryRow],
    wfx_id: &str,
) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check cross-workflow records
    for r in results {
        if r.workflow_execution_id != wfx_id {
            warnings.push(format!(
                "Result {} belongs to workflow {} (expected {})",
                r.result_id, r.workflow_execution_id, wfx_id
            ));
        }
    }

    // Check review→result linkage
    if let (Some(result), Some(review)) = (results.first(), reviews.first())
        && !review.manual_result_id.is_empty() && review.manual_result_id != result.result_id {
            warnings.push(format!(
                "Review {} references result {} but latest result is {}",
                review.review_id, review.manual_result_id, result.result_id
            ));
        }

    // Check readiness→result linkage
    if let (Some(result), Some(ready)) = (results.first(), readiness.first())
        && !ready.manual_result_id.is_empty() && ready.manual_result_id != result.result_id {
            warnings.push(format!(
                "Readiness {} references result {} but latest result is {}",
                ready.readiness_id, ready.manual_result_id, result.result_id
            ));
        }

    // Check gate→readiness linkage
    if let (Some(ready), Some(gate)) = (readiness.first(), gates.first())
        && !gate.readiness_id.is_empty() && gate.readiness_id != ready.readiness_id {
            warnings.push(format!(
                "Gate {} references readiness {} but latest readiness is {}",
                gate.gate_id, gate.readiness_id, ready.readiness_id
            ));
        }

    warnings
}

// ── Desktop-gated combined panel ─────────────────────────────────────────

#[cfg(feature = "desktop")]
pub mod panel_render {
    use super::*;
    use crate::ui::workflow_manual_result_review_components as review_comp;
    use crate::ui::workflow_manual_result_reconciliation_readiness_components as readiness_comp;
    use crate::ui::workflow_manual_result_reconciliation_gate_components as gate_comp;
    use crate::ui::workflow_manual_result_review_state::WorkflowManualResultReviewSummaryRow;
    use crate::ui::workflow_manual_result_reconciliation_readiness_state::*;
    use crate::ui::workflow_manual_result_reconciliation_gate_state::*;
    use crate::ui::layout::section_title_style;
    use dioxus::prelude::*;

    /// Combined manual-result ladder panel for Inspector tab.
    /// Handles all four rungs with linkage warnings (Patch 1).
    pub fn render_manual_result_ladder_panel(
        results: &[WorkflowManualResultSummaryRow],
        reviews: &[WorkflowManualResultReviewSummaryRow],
        readiness_records: &[WorkflowManualResultReconciliationReadinessSummaryRow],
        gates: &[WorkflowManualResultReconciliationGateSummaryRow],
        predicates: &[ReadinessPredicateDisplayRow],
        wfx_id: &str,
    ) -> Element {
        let linkage_warnings = check_ladder_linkage(results, reviews, readiness_records, gates, wfx_id);

        let header_s = section_title_style();
        let warn_s = crate::ui::components::banner_style(UiTone::Warning);

        rsx! {
            div {
                div { style: "{header_s}",
                    "Manual Result Ladder"
                }
                if !linkage_warnings.is_empty() {
                    div { style: "{warn_s}",
                        "⚠ Ladder linkage warnings:"
                    }
                    for w in &linkage_warnings {
                        div { style: "font-size: 12px; color: #856404; padding: 2px 0;",
                            "{w}"
                        }
                    }
                }
                { render_result_list(results) }
                { render_validation_if_present(results) }
                { render_artifacts_if_present(results) }
                { review_comp::render_review_list(reviews) }
                { render_acceptance_if_present(reviews) }
                { readiness_comp::render_readiness_list(readiness_records) }
                { readiness_comp::render_predicate_rows(predicates) }
                { gate_comp::render_gate_list(gates) }
            }
        }
    }

    fn render_validation_if_present(results: &[WorkflowManualResultSummaryRow]) -> Element {
        // Validation snapshot rendering is state-dependent; placeholder for now
        let _ = results;
        rsx! { div {} }
    }

    fn render_artifacts_if_present(results: &[WorkflowManualResultSummaryRow]) -> Element {
        // Artifact rendering is state-dependent; placeholder for now
        let _ = results;
        rsx! { div {} }
    }

    fn render_acceptance_if_present(reviews: &[WorkflowManualResultReviewSummaryRow]) -> Element {
        // Acceptance snapshot rendering is state-dependent; placeholder for now
        let _ = reviews;
        rsx! { div {} }
    }
}

#[cfg(feature = "desktop")]
pub use panel_render::render_manual_result_ladder_panel;

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_status_tone_info_for_succeeded() {
        assert_eq!(UiTone::Info, result_status_tone("reported_succeeded"));
    }

    #[test]
    fn result_status_tone_warning_for_failed() {
        assert_eq!(UiTone::Warning, result_status_tone("reported_failed"));
        assert_eq!(UiTone::Warning, result_status_tone("reported_partial"));
    }

    #[test]
    fn result_status_tone_neutral_for_unknown() {
        assert_eq!(UiTone::Neutral, result_status_tone("reported_unknown"));
        assert_eq!(UiTone::Neutral, result_status_tone("something_else"));
    }

    #[test]
    fn result_status_label_says_reported() {
        let label = result_status_label("reported_succeeded");
        assert!(label.starts_with("Reported"), "got: {label}");
    }

    #[test]
    fn result_reported_not_verified_note_contents() {
        let note = result_reported_not_verified_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("reported"), "got: {note}");
        assert!(lower.contains("not verified"), "got: {note}");
    }

    #[test]
    fn validation_check_label_text() {
        assert_eq!("Matched", validation_check_label(true));
        assert_eq!("Mismatch", validation_check_label(false));
    }

    // ── Guard tests (Patch 2) ──

    #[test]
    fn manual_result_ui_copy_says_reported_not_verified() {
        let note = result_reported_not_verified_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("not verified"), "got: {note}");
    }

    #[test]
    fn manual_ladder_ui_copy_contains_no_action_verbs() {
        let all_copy = vec![
            result_status_label("reported_succeeded"),
            result_status_label("reported_failed"),
            result_reported_not_verified_note(),
            result_safety_text(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("certified"), "copy contains 'certified': {text}");
            assert!(!lower.contains("approved"), "copy contains 'approved': {text}");
            assert!(!lower.contains("trusted"), "copy contains 'trusted': {text}");
            assert!(!lower.contains("delivered"), "copy contains 'delivered': {text}");
        }
    }

    // ── Patch 4: hash display ──

    #[test]
    fn manual_ladder_hash_display_preserves_identity() {
        let short = chain_hash_display("abc123");
        assert_eq!("abc123", short);
        let long = chain_hash_display("abcdef0123456789abcdef0123456789abcdef01");
        assert!(long.contains("\u{2026}"), "long hash should be truncated: {long}");
    }

    // ── Patch 1: Linkage warning tests ──

    fn sample_result(wfx: &str) -> WorkflowManualResultSummaryRow {
        WorkflowManualResultSummaryRow {
            result_id: "wmr_1".into(), status: "reported_succeeded".into(),
            operator: "test".into(), caveat: "".into(),
            workflow_execution_id: wfx.into(),
            command_review_hash: "h".into(), command_composer_hash: "h".into(),
            command_descriptor_hash: "h".into(), loop_controller_hash: "h".into(),
        }
    }

    fn sample_review(result_id: &str) -> crate::ui::workflow_manual_result_review_state::WorkflowManualResultReviewSummaryRow {
        crate::ui::workflow_manual_result_review_state::WorkflowManualResultReviewSummaryRow {
            review_id: "wmrr_1".into(), decision: "accepted".into(),
            reviewer: "test".into(), manual_result_id: result_id.into(),
            manual_result_hash: "h".into(), command_review_hash: "h".into(),
            command_composer_hash: "h".into(), command_descriptor_hash: "h".into(),
            loop_controller_hash: "h".into(),
        }
    }

    fn sample_readiness(result_id: &str) -> crate::ui::workflow_manual_result_reconciliation_readiness_state::WorkflowManualResultReconciliationReadinessSummaryRow {
        crate::ui::workflow_manual_result_reconciliation_readiness_state::WorkflowManualResultReconciliationReadinessSummaryRow {
            readiness_id: "wmrrr_1".into(), status: "ready".into(),
            evaluator: "test".into(), manual_result_id: result_id.into(),
            manual_result_review_id: "wmrr_1".into(),
            manual_result_hash: "h".into(), manual_result_review_hash: "h".into(),
            command_review_hash: "h".into(), command_composer_hash: "h".into(),
            command_descriptor_hash: "h".into(), loop_controller_hash: "h".into(),
        }
    }

    fn sample_gate(readiness_id: &str) -> crate::ui::workflow_manual_result_reconciliation_gate_state::WorkflowManualResultReconciliationGateSummaryRow {
        crate::ui::workflow_manual_result_reconciliation_gate_state::WorkflowManualResultReconciliationGateSummaryRow {
            gate_id: "wmrrg_1".into(), status: "reconciled".into(),
            reconciled_by: "test".into(), manual_result_id: "wmr_1".into(),
            stage_id: "s1".into(), revision_id: None,
            readiness_id: readiness_id.into(), readiness_hash: "h".into(),
            manual_result_review_hash: "h".into(), manual_result_hash: "h".into(),
            command_review_hash: "h".into(), command_composer_hash: "h".into(),
            command_descriptor_hash: "h".into(), loop_controller_hash: "h".into(),
        }
    }

    #[test]
    fn manual_ladder_warns_on_cross_workflow_record() {
        let result = sample_result("wfx_other");
        let warnings = check_ladder_linkage(&[result], &[], &[], &[], "wfx_1");
        assert!(!warnings.is_empty(), "should warn on cross-workflow result");
        assert!(warnings[0].contains("wfx_other"));
    }

    #[test]
    fn manual_ladder_warns_on_review_result_mismatch() {
        let result = sample_result("wfx_1");
        let review = sample_review("wmr_other");
        let warnings = check_ladder_linkage(&[result], &[review], &[], &[], "wfx_1");
        assert!(!warnings.is_empty(), "should warn on review-result mismatch");
    }

    #[test]
    fn manual_ladder_warns_on_readiness_result_mismatch() {
        let result = sample_result("wfx_1");
        let readiness = sample_readiness("wmr_other");
        let warnings = check_ladder_linkage(&[result], &[], &[readiness], &[], "wfx_1");
        assert!(!warnings.is_empty(), "should warn on readiness-result mismatch");
    }

    #[test]
    fn manual_ladder_warns_on_gate_readiness_mismatch() {
        let readiness = sample_readiness("wmr_1");
        let gate = sample_gate("wmrrr_other");
        let warnings = check_ladder_linkage(&[], &[], &[readiness], &[gate], "wfx_1");
        assert!(!warnings.is_empty(), "should warn on gate-readiness mismatch");
    }

    #[test]
    fn manual_ladder_does_not_present_mismatched_latest_records_as_coherent() {
        let result = sample_result("wfx_1");
        let review = sample_review("wmr_other");
        let warnings = check_ladder_linkage(&[result], &[review], &[], &[], "wfx_1");
        assert!(!warnings.is_empty());
        let combined = warnings.join(" ");
        assert!(combined.contains("wmr_other"));
    }

    #[test]
    fn manual_ladder_no_warnings_when_linkage_matches() {
        let result = sample_result("wfx_1");
        let review = sample_review("wmr_1");
        let readiness = sample_readiness("wmr_1");
        let gate = sample_gate("wmrrr_1");
        let warnings = check_ladder_linkage(&[result], &[review], &[readiness], &[gate], "wfx_1");
        assert!(warnings.is_empty(), "should have no warnings: {warnings:?}");
    }

    #[test]
    fn manual_ladder_handles_no_records() {
        let warnings = check_ladder_linkage(&[], &[], &[], &[], "wfx_1");
        assert!(warnings.is_empty());
    }

    #[test]
    fn manual_ladder_handles_result_without_review() {
        let result = sample_result("wfx_1");
        let warnings = check_ladder_linkage(&[result], &[], &[], &[], "wfx_1");
        assert!(warnings.is_empty());
    }

    #[test]
    fn manual_ladder_handles_multiple_results_if_available() {
        let r1 = sample_result("wfx_1");
        let r2 = WorkflowManualResultSummaryRow {
            result_id: "wmr_2".into(), ..sample_result("wfx_1")
        };
        let warnings = check_ladder_linkage(&[r1, r2], &[], &[], &[], "wfx_1");
        assert!(warnings.is_empty());
    }
}
