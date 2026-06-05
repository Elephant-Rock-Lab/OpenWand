//! UI evidence chain inspector state — display-only helpers.

use openwand_workflow::workflow_evidence_chain_inspector::*;

#[derive(Debug, Clone)]
pub struct EvidenceChainSummaryRow {
    pub inspection_id: String,
    pub workflow_execution_id: String,
    pub present_links: usize,
    pub missing_links: usize,
    pub not_yet_applicable: usize,
    pub chain_hash: String,
    pub warning_count: usize,
}

pub fn chain_summary_lines(state: &EvidenceChainInspectionState) -> EvidenceChainSummaryRow {
    EvidenceChainSummaryRow {
        inspection_id: state.inspection_id.clone(),
        workflow_execution_id: state.workflow_execution_id.clone(),
        present_links: state.coverage_summary.present_links,
        missing_links: state.coverage_summary.missing_expected_links,
        not_yet_applicable: state.coverage_summary.not_yet_applicable_links,
        chain_hash: state.chain_hash.clone(),
        warning_count: state.coverage_summary.warnings,
    }
}

pub fn chain_safety_warning() -> String {
    "The evidence chain inspector observes recorded evidence. \
     It does not certify external truth, verify artifacts, execute commands, \
     route actions, resolve approvals, reconcile outcomes, or mutate workflow state.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_evidence_chain_inspector::EvidenceLinkPresence;

    fn test_state() -> EvidenceChainInspectionState {
        let links = vec![
            EvidenceChainLink {
                record_type: "run".into(),
                record_id: "wfx_1".into(),
                presence: EvidenceLinkPresence::Present,
                record_hash: "h1".into(),
                source_path_hint: None,
            },
        ];
        openwand_workflow::workflow_evidence_chain_inspector::build_inspection_state(
            "wfx_1", links, vec![], false,
        )
    }

    #[test]
    fn summary_row_extracts_correct_counts() {
        let state = test_state();
        let row = chain_summary_lines(&state);
        assert_eq!(1, row.present_links);
        assert_eq!(0, row.missing_links);
        assert!(row.inspection_id.starts_with("weci_"));
    }

    #[test]
    fn safety_warning_does_not_overclaim() {
        let w = chain_safety_warning();
        assert!(w.contains("does not certify"));
        assert!(w.contains("verify artifacts"));
        assert!(w.contains("does not") || w.contains("does not certify"));
        // Should not contain affirmative verification claims
        assert!(!w.contains("certifies"));
    }

    #[test]
    fn safety_warning_mentions_inspector_not_verifier() {
        let w = chain_safety_warning();
        assert!(w.contains("inspector"));
        assert!(!w.contains("verifier"));
    }
}
