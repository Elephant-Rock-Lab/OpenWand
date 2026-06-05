//! UI operator console state — display-only helpers.

use openwand_workflow::workflow_operator_console::*;

#[derive(Debug, Clone)]
pub struct OperatorConsoleSummaryRow {
    pub workflow_execution_id: String,
    pub run_status: String,
    pub detected_state: String,
    pub evidence_chain_count: usize,
    pub chain_consistent: bool,
    pub warning_count: usize,
}

pub fn console_summary_lines(state: &WorkflowOperatorConsoleState) -> OperatorConsoleSummaryRow {
    OperatorConsoleSummaryRow {
        workflow_execution_id: state.workflow_execution_id.0.clone(),
        run_status: state.run_status.clone(),
        detected_state: state.detected_state.clone(),
        evidence_chain_count: state.evidence_chain.len(),
        chain_consistent: state.evidence_chain_consistent,
        warning_count: state.chain_warnings.len(),
    }
}

pub fn console_safety_warning() -> String {
    "The operator console observes, summarizes, and links evidence. \
     It does not route actions, does not execute tools, does not verify external state, \
     does not resolve approvals, does not reconcile outcomes, and does not mutate workflow state.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_loop_state::WorkflowDetectedLoopState;

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
            sections: vec![],
            attestation_groups: vec![],
            verification_readiness_summary: vec![],
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
    fn safety_warning_mentions_no_mutation() {
        let w = console_safety_warning();
        assert!(w.contains("does not route"));
        assert!(w.contains("does not execute"));
        assert!(w.contains("does not mutate"));
    }

    #[test]
    fn safety_warning_mentions_observe() {
        let w = console_safety_warning();
        assert!(w.contains("observes"));
    }
}
