//! UI operator console state — display-only helpers.
//!
//! Wave 48A: Extended with per-section summaries, attestation display,
//! verification readiness eligibility, and detected state explanation.

use openwand_workflow::workflow_operator_console::*;

#[derive(Debug, Clone)]
pub struct OperatorConsoleSummaryRow {
    pub workflow_execution_id: String,
    pub run_status: String,
    pub detected_state: String,
    pub detected_state_explanation: Option<String>,
    pub evidence_chain_count: usize,
    pub chain_consistent: bool,
    pub warning_count: usize,
    pub section_count: usize,
    pub attestation_group_count: usize,
    pub readiness_summary_count: usize,
}

pub fn console_summary_lines(state: &WorkflowOperatorConsoleState) -> OperatorConsoleSummaryRow {
    OperatorConsoleSummaryRow {
        workflow_execution_id: state.workflow_execution_id.0.clone(),
        run_status: state.run_status.clone(),
        detected_state: state.detected_state.clone(),
        detected_state_explanation: state.detected_state_explanation.clone(),
        evidence_chain_count: state.evidence_chain.len(),
        chain_consistent: state.evidence_chain_consistent,
        warning_count: state.chain_warnings.len(),
        section_count: state.sections.len(),
        attestation_group_count: state.attestation_groups.len(),
        readiness_summary_count: state.verification_readiness_summary.len(),
    }
}

/// Per-section summary for display.
#[derive(Debug, Clone)]
pub struct SectionDisplayRow {
    pub section: String,
    pub present: usize,
    pub missing: usize,
    pub total: usize,
}

pub fn section_display_rows(state: &WorkflowOperatorConsoleState) -> Vec<SectionDisplayRow> {
    state.sections.iter().map(|s| SectionDisplayRow {
        section: format!("{:?}", s.section),
        present: s.present_count,
        missing: s.missing_count,
        total: s.link_count,
    }).collect()
}

/// Attestation display summary.
#[derive(Debug, Clone)]
pub struct AttestationDisplayRow {
    pub target: String,
    pub count: usize,
    pub all_unverified: bool,
}

pub fn attestation_display_rows(state: &WorkflowOperatorConsoleState) -> Vec<AttestationDisplayRow> {
    state.attestation_groups.iter().map(|g| AttestationDisplayRow {
        target: format!("{}:{}", g.target_kind, g.target_id),
        count: g.attestations.len(),
        all_unverified: g.attestations.iter().all(|a| !a.verified_by_openwand && !a.promotes_trust),
    }).collect()
}

pub fn console_safety_warning() -> String {
    "The operator console observes, summarizes, groups, explains, and links recorded evidence. \
     It does not route actions, execute tools, verify external state, certify evidence, \
     promote trust, schedule verification, resolve approvals, reconcile outcomes, \
     or mutate workflow state.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_loop_state::WorkflowDetectedLoopState;
    use openwand_workflow::workflow_operator_console::*;

    fn test_state() -> WorkflowOperatorConsoleState {
        WorkflowOperatorConsoleState {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            run_status: "suspended".into(),
            stages: vec![],
            detected_state: "inconclusive".into(),
            detected_state_explanation: Some("test explanation".into()),
            recommendation: None,
            evidence_chain: vec![ConsoleEvidenceLink {
                link_kind: "test".into(), record_id: "id_1".into(),
                status: "found".into(), summary: "test".into(),
            }],
            sections: vec![ConsoleSectionSummary {
                section: ConsoleEvidenceSection::UpstreamSpine,
                link_count: 5, present_count: 3, missing_count: 2, warnings_count: 0,
            }],
            attestation_groups: vec![ConsoleAttestationGroup {
                target_kind: "manual_result".into(),
                target_id: "wmr_1".into(),
                attestations: vec![ConsoleAttestationRow {
                    attestation_id: "watt_1".into(),
                    kind: "code_review".into(),
                    source_name: "Bob".into(),
                    claim: "LGTM".into(),
                    verified_by_openwand: false,
                    promotes_trust: false,
                }],
            }],
            verification_readiness_summary: vec![ConsoleReadinessEligibilitySummary {
                readiness_id: "wvr_1".into(),
                target_kind: "manual_result".into(),
                target_id: "wmr_1".into(),
                status: "ready".into(),
                is_eligibility_only: true,
            }],
            chain_warnings: vec![],
            evidence_chain_consistent: true,
            warnings: vec![],
            computed_at: chrono::Utc::now(),
            creates_route: false, executes_tool: false, verifies_external_state: false,
            resolves_approval: false, reconciles_outcome: false, mutates_workflow_state: false,
            creates_run_revision: false, appends_trace: false, writes_memory: false,
            certifies_evidence: false, promotes_trust: false, schedules_verification: false,
        }
    }

    #[test]
    fn console_summary_shows_evidence_count() {
        let row = console_summary_lines(&test_state());
        assert_eq!(1, row.evidence_chain_count);
    }

    #[test]
    fn console_summary_shows_chain_consistency() {
        let row = console_summary_lines(&test_state());
        assert!(row.chain_consistent);
    }

    #[test]
    fn console_summary_shows_sections() {
        let row = console_summary_lines(&test_state());
        assert_eq!(1, row.section_count);
    }

    #[test]
    fn console_summary_shows_attestation_groups() {
        let row = console_summary_lines(&test_state());
        assert_eq!(1, row.attestation_group_count);
    }

    #[test]
    fn console_summary_shows_readiness_summaries() {
        let row = console_summary_lines(&test_state());
        assert_eq!(1, row.readiness_summary_count);
    }

    #[test]
    fn console_summary_shows_detected_state_explanation() {
        let row = console_summary_lines(&test_state());
        assert!(row.detected_state_explanation.is_some());
    }

    #[test]
    fn section_display_rows_show_counts() {
        let rows = section_display_rows(&test_state());
        assert_eq!(1, rows.len());
        assert_eq!(3, rows[0].present);
        assert_eq!(2, rows[0].missing);
    }

    #[test]
    fn attestation_display_rows_show_unverified() {
        let rows = attestation_display_rows(&test_state());
        assert_eq!(1, rows.len());
        assert!(rows[0].all_unverified);
    }

    #[test]
    fn safety_warning_mentions_observe() {
        let w = console_safety_warning();
        assert!(w.contains("observes"));
    }

    #[test]
    fn safety_warning_mentions_no_certify_trust_schedule() {
        let w = console_safety_warning();
        assert!(w.contains("certify evidence") || w.contains("certifies"));
        assert!(w.contains("promote trust") || w.contains("promotes trust"));
        assert!(w.contains("schedule verification") || w.contains("schedules"));
    }
}
