//! Workflow operator console desktop UI components.
//!
//! Read-only display of the workflow operator console using Wave 52A design-system
//! tokens. The console displays summary, evidence sections, chain warnings,
//! attestations, and verification-readiness eligibility.
//!
//! Components accept already-prepared state. They do not load, assemble, query,
//! or call services. The desktop shell may call a read-only loader/assembler;
//! this file remains pure presentation.
//!
//! This module imports no backend crates, performs no persistence, executes
//! no tools, verifies nothing, certifies nothing, mutates no trace or memory
//! state, resolves no approvals, routes no actions, and creates no workflow
//! records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_operator_console_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Textual progress for an evidence section: "3 / 5 present, 2 missing"
pub fn section_progress_text(section: &SectionDisplayRow) -> String {
    format!(
        "{} / {} present, {} missing",
        section.present, section.total, section.missing
    )
}

/// Semantic tone for an evidence section based on completeness.
pub fn section_tone(section: &SectionDisplayRow) -> UiTone {
    if section.missing == 0 {
        UiTone::Success
    } else if section.present == 0 {
        UiTone::Warning
    } else {
        UiTone::Info
    }
}

/// Semantic tone for a detected state string.
pub fn detected_state_tone(state: &str) -> UiTone {
    match state {
        "workflowcomplete" => UiTone::Success,
        "workflowblocked" => UiTone::Error,
        "inconclusive" => UiTone::Warning,
        _ => UiTone::Neutral,
    }
}

/// Semantic tone for chain consistency.
pub fn chain_consistency_tone(consistent: bool) -> UiTone {
    if consistent { UiTone::Success } else { UiTone::Error }
}

/// Note text for attestation display — always unverified.
pub fn attestation_unverified_note() -> String {
    "Reported, not verified by OpenWand".into()
}

/// Note text for verification readiness — always eligibility-only.
pub fn readiness_eligibility_note() -> String {
    "Eligibility display only — not a verification".into()
}

/// Status label for an evidence link.
pub fn evidence_link_status_label(status: &str) -> String {
    match status {
        "found" => "Present".into(),
        "missing" => "Missing".into(),
        _ => status.to_string(),
    }
}

/// Tone for an evidence link.
pub fn evidence_link_tone(status: &str) -> UiTone {
    match status {
        "found" => UiTone::Success,
        "missing" => UiTone::Warning,
        _ => UiTone::Neutral,
    }
}

/// Safety warning for operator console display.
pub fn operator_console_safety_text() -> String {
    console_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use crate::ui::layout::*;
    use dioxus::prelude::*;
    use openwand_workflow::workflow_operator_console::{
        ConsoleAttestationGroup, ConsoleEvidenceLink,
        ConsoleReadinessEligibilitySummary, ConsoleSectionSummary,
        WorkflowOperatorConsoleState,
    };

    /// Empty state when no workflow execution is selected.
    pub fn render_operator_console_empty_state() -> Element {
        let style = empty_state_style();
        rsx! {
            div { style: "{style}",
                div { style: "text-align: center;",
                    div { style: "font-size: 16px; color: #999; margin-bottom: 8px;",
                        "No workflow execution selected"
                    }
                    div { style: "font-size: 12px; color: #bbb;",
                        "Run a governed workflow to see the operator console."
                    }
                }
            }
        }
    }

    /// Loading state while console state is being assembled.
    pub fn render_operator_console_loading_state() -> Element {
        let style = empty_state_style();
        rsx! {
            div { style: "{style}",
                div { style: "font-size: 14px; color: {colors::TEXT_MUTED};",
                    "Loading operator console…"
                }
            }
        }
    }

    /// Error state when console assembly fails.
    pub fn render_operator_console_error_state(message: &str) -> Element {
        let style = banner_style(UiTone::Error);
        let safe: String = message.chars().take(200).collect();
        rsx! {
            div { style: "{style}",
                div { style: "font-weight: 600; margin-bottom: 4px;",
                    "Operator console error"
                }
                div { "{safe}" }
            }
        }
    }

    /// Safety banner for the operator console.
    pub fn render_safety_banner() -> Element {
        let text = operator_console_safety_text();
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                "⚠ {text}"
            }
        }
    }

    /// Console summary header with run ID, status, detected state, and explanation.
    pub fn render_console_summary(
        summary: &OperatorConsoleSummaryRow,
    ) -> Element {
        let header_style = header_bar_style();
        let detected_tone = detected_state_tone(&summary.detected_state);
        let status_badge_style = badge_style(detected_tone);
        let id_style = format!("font-size: {}; color: {};", typo::TEXT_SM, colors::TEXT_MUTED);
        let title_style = format!("margin: 0 0 2px 0; font-size: {};", typo::TEXT_XL);
        let chain_tone = chain_consistency_tone(summary.chain_consistent);
        let chain_label = if summary.chain_consistent {
            "Chain consistent"
        } else {
            "Chain has warnings"
        };
        let chain_badge_style = badge_style(chain_tone);
        let explanation_style = format!(
            "font-size: {}; color: {}; margin-top: {}; line-height: 1.4;",
            typo::TEXT_SM, colors::TEXT_SECONDARY, spacing::SPACE_SM,
        );

        rsx! {
            div { style: "{header_style}",
                div {
                    div { style: "{title_style}",
                        "Workflow {summary.workflow_execution_id}"
                    }
                    div { style: "{id_style}",
                        "Status: {summary.run_status}"
                    }
                    if let Some(ref explanation) = summary.detected_state_explanation {
                        div { style: "{explanation_style}",
                            "{explanation}"
                        }
                    }
                }
                div { style: "display: flex; gap: 8px; align-items: center;",
                    span { style: "{status_badge_style}",
                        "{summary.detected_state}"
                    }
                    span { style: "{chain_badge_style}",
                        "{chain_label}"
                    }
                }
            }
        }
    }

    /// Section grid showing 5 evidence sections with textual progress.
    pub fn render_section_grid(sections: &[ConsoleSectionSummary]) -> Element {
        let section_rows: Vec<(String, String, UiTone, String)> = sections
            .iter()
            .map(|s| {
                let display = SectionDisplayRow {
                    section: format!("{:?}", s.section),
                    present: s.present_count,
                    missing: s.missing_count,
                    total: s.link_count,
                };
                let tone = section_tone(&display);
                let progress = section_progress_text(&display);
                (display.section.clone(), progress, tone, format!("{}", s.present_count))
            })
            .collect();

        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Evidence Sections ({section_rows.len()})"
                }
                for (name, progress, tone, _count) in &section_rows {
                    {
                        let dot_s = status_dot_style(*tone, UiSize::Sm);
                        let row_s = format!(
                            "display: flex; align-items: center; gap: {}; padding: {} 0; \
                             border-bottom: 1px solid {}; font-size: {};",
                            spacing::SPACE_MD, spacing::SPACE_SM,
                            colors::BORDER_SUBTLE, typo::TEXT_SM,
                        );
                        let name_s = format!(
                            "min-width: 160px; font-weight: 500; color: {};",
                            colors::TEXT_PRIMARY,
                        );
                        let progress_s = format!("color: {};", colors::TEXT_MUTED);
                        rsx! {
                            div { style: "{row_s}",
                                div { style: "{dot_s}" }
                                span { style: "{name_s}", "{name}" }
                                span { style: "{progress_s}", "{progress}" }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Scrollable evidence chain list with status dots.
    pub fn render_evidence_chain(links: &[ConsoleEvidenceLink]) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);
        let scroll_s = format!(
            "max-height: 200px; overflow-y: auto; font-family: {};",
            typo::FONT_MONO,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Evidence Chain ({links.len()})"
                }
                div { style: "{scroll_s}",
                    for link in links {
                        {
                            let tone = evidence_link_tone(&link.status);
                            let dot_s = status_dot_style(tone, UiSize::Xs);
                            let row_s = format!(
                                "display: flex; align-items: center; gap: {}; padding: {} 0; \
                                 font-size: {}; border-bottom: 1px solid {};",
                                spacing::SPACE_SM, spacing::SPACE_XS,
                                typo::TEXT_SM, colors::BORDER_SUBTLE,
                            );
                            let kind_s = format!("min-width: 140px; color: {};", colors::TEXT_PRIMARY);
                            let id_s = format!("color: {};", colors::TEXT_MUTED);
                            let status_label = evidence_link_status_label(&link.status);
                            rsx! {
                                div { style: "{row_s}",
                                    div { style: "{dot_s}" }
                                    span { style: "{kind_s}", "{link.link_kind}" }
                                    span { style: "{id_s}", "{link.record_id}" }
                                    span { style: "{id_s}", " — {status_label}" }
                                }
                            }
                        }
                    }
                    if links.is_empty() {
                        div { style: "padding: 8px; font-size: 12px; color: #999; text-align: center;",
                            "No evidence links recorded"
                        }
                    }
                }
            }
        }
    }

    /// Attestation panel showing grouped attestations, all marked unverified.
    pub fn render_attestation_panel(groups: &[ConsoleAttestationGroup]) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);
        let note = attestation_unverified_note();

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "External Attestations ({groups.len()})"
                }
                for group in groups {
                    {
                        let target_s = format!(
                            "font-weight: 600; font-size: {}; color: {}; padding: {} 0 {} 0;",
                            typo::TEXT_SM, colors::TEXT_PRIMARY,
                            spacing::SPACE_SM, spacing::SPACE_XS,
                        );
                        let row_s = format!(
                            "padding: {} {}; font-size: {}; color: {};",
                            spacing::SPACE_XS, spacing::SPACE_MD,
                            typo::TEXT_XS, colors::TEXT_MUTED,
                        );
                        let note_s = format!(
                            "font-size: {}; color: {}; font-style: italic;",
                            typo::TEXT_XS, colors::TEXT_MUTED,
                        );
                        rsx! {
                            div {
                                div { style: "{target_s}",
                                    "{group.target_kind}:{group.target_id} ({group.attestations.len()})"
                                }
                                for att in &group.attestations {
                                    div { style: "{row_s}",
                                        "{att.source_name}: {att.claim} ({att.kind})"
                                    }
                                }
                                div { style: "{note_s}", "{note}" }
                            }
                        }
                    }
                }
                if groups.is_empty() {
                    div { style: "padding: 8px; font-size: 12px; color: #999; text-align: center;",
                        "No external attestations recorded"
                    }
                }
            }
        }
    }

    /// Readiness panel showing verification readiness as eligibility-only.
    pub fn render_readiness_panel(summaries: &[ConsoleReadinessEligibilitySummary]) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Info);
        let note = readiness_eligibility_note();

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Verification Readiness ({summaries.len()})"
                }
                for summary in summaries {
                    {
                        let row_s = format!(
                            "display: flex; gap: {}; padding: {} 0; font-size: {}; \
                             border-bottom: 1px solid {};",
                            spacing::SPACE_MD, spacing::SPACE_SM,
                            typo::TEXT_SM, colors::BORDER_SUBTLE,
                        );
                        let label_s = format!("min-width: 120px; color: {};", colors::TEXT_PRIMARY);
                        let value_s = format!("color: {};", colors::TEXT_MUTED);
                        rsx! {
                            div { style: "{row_s}",
                                span { style: "{label_s}", "{summary.target_kind}:{summary.target_id}" }
                                span { style: "{value_s}", "{summary.status}" }
                            }
                        }
                    }
                }
                if summaries.is_empty() {
                    div { style: "padding: 8px; font-size: 12px; color: #999; text-align: center;",
                        "No verification readiness records"
                    }
                }
                if !summaries.is_empty() {
                    {
                        let note_s = format!(
                            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
                            typo::TEXT_XS, colors::TEXT_MUTED, spacing::SPACE_SM,
                        );
                        rsx! { div { style: "{note_s}", "{note}" } }
                    }
                }
            }
        }
    }

    /// Chain warnings display.
    pub fn render_chain_warnings(warnings: &[String], consistent: bool) -> Element {
        if consistent && warnings.is_empty() {
            return rsx! { div {} };
        }
        let tone = if consistent { UiTone::Warning } else { UiTone::Error };
        let style = banner_style(tone);
        rsx! {
            div { style: "{style}",
                div { style: "font-weight: 600; margin-bottom: 4px;",
                    if consistent { "Warnings" } else { "Chain Inconsistency" }
                }
                for warning in warnings {
                    div { style: "font-size: 12px;", "⚠ {warning}" }
                }
            }
        }
    }

    /// Full operator console surface.
    pub fn render_operator_console(state: &WorkflowOperatorConsoleState) -> Element {
        let summary = console_summary_lines(state);
        let scroll_s = scroll_area_style();
        let _section_rows = state.sections.iter().map(|s| SectionDisplayRow {
            section: format!("{:?}", s.section),
            present: s.present_count,
            missing: s.missing_count,
            total: s.link_count,
        }).collect::<Vec<_>>();

        rsx! {
            div { style: "flex: 1; display: flex; flex-direction: column; min-width: 0;",
                { render_console_summary(&summary) }

                div { style: "{scroll_s}",
                    { render_chain_warnings(&state.chain_warnings.iter().map(|w| w.reason.clone()).collect::<Vec<_>>(), state.evidence_chain_consistent) }
                    { render_section_grid(&state.sections) }
                    { render_evidence_chain(&state.evidence_chain) }
                    { render_attestation_panel(&state.attestation_groups) }
                    { render_readiness_panel(&state.verification_readiness_summary) }
                }

                { render_safety_banner() }
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

    // ── Pure helper tests ──

    #[test]
    fn section_progress_text_includes_present_missing_total() {
        let row = SectionDisplayRow {
            section: "UpstreamSpine".into(),
            present: 3,
            missing: 2,
            total: 5,
        };
        let text = section_progress_text(&row);
        assert!(text.contains("3 / 5 present"), "got: {text}");
        assert!(text.contains("2 missing"), "got: {text}");
    }

    #[test]
    fn section_progress_tone_does_not_replace_textual_status() {
        // Tone is auxiliary, not replacement
        let row = SectionDisplayRow { section: "Test".into(), present: 3, missing: 0, total: 3 };
        let tone = section_tone(&row);
        assert_eq!(UiTone::Success, tone);
        let text = section_progress_text(&row);
        assert!(text.contains("3 / 3"), "textual counts must still be present");
    }

    #[test]
    fn section_tone_success_when_no_missing() {
        let row = SectionDisplayRow { section: "Test".into(), present: 5, missing: 0, total: 5 };
        assert_eq!(UiTone::Success, section_tone(&row));
    }

    #[test]
    fn section_tone_warning_when_all_missing() {
        let row = SectionDisplayRow { section: "Test".into(), present: 0, missing: 5, total: 5 };
        assert_eq!(UiTone::Warning, section_tone(&row));
    }

    #[test]
    fn section_tone_info_when_partial() {
        let row = SectionDisplayRow { section: "Test".into(), present: 3, missing: 2, total: 5 };
        assert_eq!(UiTone::Info, section_tone(&row));
    }

    #[test]
    fn detected_state_tone_maps_correctly() {
        assert_eq!(UiTone::Success, detected_state_tone("workflowcomplete"));
        assert_eq!(UiTone::Error, detected_state_tone("workflowblocked"));
        assert_eq!(UiTone::Warning, detected_state_tone("inconclusive"));
        assert_eq!(UiTone::Neutral, detected_state_tone("needs_command_descriptor"));
    }

    #[test]
    fn chain_consistency_tone_success_when_consistent() {
        assert_eq!(UiTone::Success, chain_consistency_tone(true));
        assert_eq!(UiTone::Error, chain_consistency_tone(false));
    }

    #[test]
    fn attestation_unverified_note_says_unverified() {
        let note = attestation_unverified_note();
        assert!(note.to_lowercase().contains("unverified") || note.to_lowercase().contains("not verified"));
    }

    #[test]
    fn readiness_eligibility_note_says_eligibility() {
        let note = readiness_eligibility_note();
        assert!(note.to_lowercase().contains("eligibility"));
    }

    #[test]
    fn evidence_link_status_label_found() {
        assert_eq!("Present", evidence_link_status_label("found"));
        assert_eq!("Missing", evidence_link_status_label("missing"));
    }

    #[test]
    fn evidence_link_tone_found_is_success() {
        assert_eq!(UiTone::Success, evidence_link_tone("found"));
        assert_eq!(UiTone::Warning, evidence_link_tone("missing"));
    }

    // ── Guard tests ──

    #[test]
    fn operator_console_ui_copy_contains_no_authority_verbs() {
        // Safety warning is excluded because it explicitly lists forbidden actions
        // (which naturally contain authority verbs in negated form).
        let all_copy = vec![
            section_progress_text(&SectionDisplayRow {
                section: "Test".into(), present: 3, missing: 2, total: 5,
            }),
            attestation_unverified_note(),
            readiness_eligibility_note(),
            evidence_link_status_label("found"),
            evidence_link_status_label("missing"),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("execute"), "copy contains 'execute': {text}");
            assert!(!lower.contains("approve"), "copy contains 'approve': {text}");
            assert!(!lower.contains("resolve"), "copy contains 'resolve': {text}");
            assert!(!lower.contains("certify"), "copy contains 'certify': {text}");
            assert!(!lower.contains("promote trust"), "copy contains 'promote trust': {text}");
            assert!(!lower.contains("reconcile now"), "copy contains 'reconcile now': {text}");
            assert!(!lower.contains("route now"), "copy contains 'route now': {text}");
            assert!(!lower.contains("schedule verification"), "copy contains 'schedule verification': {text}");
        }
    }

    #[test]
    fn operator_console_readiness_copy_says_eligibility_not_verification() {
        let note = readiness_eligibility_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("eligibility"));
        assert!(!lower.contains("verify your"), "should not imply user action");
    }

    #[test]
    fn operator_console_attestation_copy_says_unverified() {
        let note = attestation_unverified_note();
        assert!(
            note.to_lowercase().contains("unverified") || note.to_lowercase().contains("not verified"),
            "attestation copy must say unverified: {note}"
        );
    }

    // ── Allowed wording test ──

    #[test]
    fn operator_console_uses_allowed_read_only_wording() {
        let allowed = vec!["reported", "recorded", "present", "missing", "warning", "eligibility", "unverified", "read-only"];
        let all_copy = vec![
            attestation_unverified_note(),
            readiness_eligibility_note(),
            evidence_link_status_label("found"),
            evidence_link_status_label("missing"),
        ];
        let combined = all_copy.join(" ").to_lowercase();
        // At least some allowed words should appear
        let found = allowed.iter().filter(|w| combined.contains(*w)).count();
        assert!(found >= 3, "Expected at least 3 allowed words in copy, found {found}");
    }
}
