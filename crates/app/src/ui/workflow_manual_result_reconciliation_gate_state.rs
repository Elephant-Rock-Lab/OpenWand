//! UI manual reconciliation gate state — display-only helpers.

use openwand_workflow::workflow_manual_result_reconciliation_gate::*;

#[derive(Debug, Clone)]
pub struct WorkflowManualResultReconciliationGateSummaryRow {
    pub gate_id: String,
    pub status: String,
    pub reconciled_by: String,
    pub manual_result_id: String,
    pub stage_id: String,
    pub revision_id: Option<String>,
    pub readiness_id: String,
    pub readiness_hash: String,
    pub manual_result_review_hash: String,
    pub manual_result_hash: String,
    pub command_review_hash: String,
    pub command_composer_hash: String,
    pub command_descriptor_hash: String,
    pub loop_controller_hash: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowManualResultReconciliationGateUiState {
    pub latest_gate: Option<WorkflowManualResultReconciliationGateSummaryRow>,
    pub warnings: Vec<String>,
}

pub fn gate_summary_lines(record: &WorkflowManualResultReconciliationGateRecord) -> WorkflowManualResultReconciliationGateSummaryRow {
    let revision = record.new_run_revision_id.as_ref().map(|r| r.0.clone());
    WorkflowManualResultReconciliationGateSummaryRow {
        gate_id: record.gate_id.0.clone(),
        status: serde_json::to_string(&record.status).unwrap().trim_matches('"').to_string(),
        reconciled_by: record.reconciled_by.clone(),
        manual_result_id: record.manual_result_id.0.clone(),
        stage_id: record.stage_id.clone(),
        revision_id: revision,
        readiness_id: record.reconciliation_readiness_id.0.clone(),
        readiness_hash: record.reconciliation_readiness_hash.clone(),
        manual_result_review_hash: record.manual_result_review_hash.clone(),
        manual_result_hash: record.manual_result_hash.clone(),
        command_review_hash: record.command_review_hash.clone(),
        command_composer_hash: record.command_composer_hash.clone(),
        command_descriptor_hash: record.command_descriptor_hash.clone(),
        loop_controller_hash: record.loop_controller_hash.clone(),
    }
}

pub fn gate_safety_warning() -> String {
    "Manual result reconciliation creates a new workflow run revision from \
     accepted operator-reported evidence. It does not execute commands, does not \
     verify external truth, does not mutate the original workflow run, and does not \
     route continuation.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_manual_result::WorkflowManualResultId;
    use openwand_workflow::workflow_manual_result_review::WorkflowManualResultReviewId;
    use openwand_workflow::workflow_manual_result_reconciliation_readiness::WorkflowManualResultReconciliationReadinessId;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;
    use chrono::Utc;

    fn test_record() -> WorkflowManualResultReconciliationGateRecord {
        WorkflowManualResultReconciliationGateRecord {
            gate_id: WorkflowManualResultReconciliationGateId("wmrrg_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            manual_result_review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            reconciliation_readiness_id: WorkflowManualResultReconciliationReadinessId("wmrrr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            stage_id: "stage_1".into(),
            workflow_run_hash: "wrh".into(), reconciliation_readiness_hash: "rrh".into(),
            manual_result_review_hash: "mrrh".into(), manual_result_hash: "mrh".into(),
            command_review_hash: "crh".into(), command_composer_hash: "cch".into(),
            command_descriptor_hash: "cdh".into(), loop_controller_hash: "lch".into(),
            status: WorkflowManualResultReconciliationGateStatus::Reconciled,
            decision: WorkflowManualResultReconciliationGateDecision::Reconciled {
                revision_id: Some("wrr_t".into()), summary: "ok".into(),
            },
            predicates: vec![], progression: None,
            new_run_revision_id: Some(WorkflowRunRevisionId("wrr_t".into())),
            creates_run_revision: true, mutates_original_workflow_run: false,
            verifies_external_truth: false, executes_command: false,
            routes_continuation: false, appends_trace: false, writes_memory: false,
            creates_execution_grant: false, execution_allowed_now: false,
            reconciled_by: "test".into(), reconciled_at: Utc::now(),
        }
    }

    #[test]
    fn ui_state_loads_latest_gate() {
        let state = WorkflowManualResultReconciliationGateUiState {
            latest_gate: Some(gate_summary_lines(&test_record())),
            warnings: vec![],
        };
        assert!(state.latest_gate.is_some());
        assert_eq!("reconciled", state.latest_gate.unwrap().status);
    }

    #[test]
    fn summary_lines_show_revision() {
        let row = gate_summary_lines(&test_record());
        assert_eq!("reconciled", row.status);
        assert_eq!(Some("wrr_t".into()), row.revision_id);
    }

    #[test]
    fn safety_warning_mentions_no_mutation() {
        let w = gate_safety_warning();
        assert!(w.contains("does not execute"));
        assert!(w.contains("not mutate"));
        assert!(w.contains("does not route"));
    }

    #[test]
    fn safety_warning_mentions_revision() {
        let w = gate_safety_warning();
        assert!(w.contains("revision"));
    }
}
