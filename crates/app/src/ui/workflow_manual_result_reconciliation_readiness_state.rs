//! UI reconciliation readiness state — display-only helpers.

use openwand_workflow::workflow_manual_result_reconciliation_readiness::*;

#[derive(Debug, Clone)]
pub struct WorkflowManualResultReconciliationReadinessSummaryRow {
    pub readiness_id: String,
    pub status: String,
    pub evaluator: String,
    pub manual_result_id: String,
    pub manual_result_review_id: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowManualResultReconciliationReadinessUiState {
    pub latest_readiness: Option<WorkflowManualResultReconciliationReadinessSummaryRow>,
    pub warnings: Vec<String>,
}

pub fn workflow_reconciliation_readiness_summary_lines(record: &WorkflowManualResultReconciliationReadinessRecord) -> WorkflowManualResultReconciliationReadinessSummaryRow {
    WorkflowManualResultReconciliationReadinessSummaryRow {
        readiness_id: record.readiness_id.0.clone(),
        status: serde_json::to_string(&record.status).unwrap().trim_matches('"').to_string(),
        evaluator: record.evaluator.clone(),
        manual_result_id: record.manual_result_id.0.clone(),
        manual_result_review_id: record.manual_result_review_id.0.clone(),
    }
}

pub fn workflow_reconciliation_readiness_safety_warning() -> String {
    "Reconciliation readiness evaluates whether reconciliation is possible. \
     OpenWand does not reconcile, verify external state, create run revisions, \
     or mutate workflow state from this screen.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_manual_result::WorkflowManualResultId;
    use openwand_workflow::workflow_manual_result_review::WorkflowManualResultReviewId;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use chrono::Utc;

    fn test_record() -> WorkflowManualResultReconciliationReadinessRecord {
        WorkflowManualResultReconciliationReadinessRecord {
            readiness_id: WorkflowManualResultReconciliationReadinessId("wmrrr_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            manual_result_review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            manual_result_review_hash: "rrh".into(), manual_result_hash: "mrh".into(),
            command_review_hash: "crh".into(), command_composer_hash: "cch".into(),
            command_descriptor_hash: "cdh".into(), loop_controller_hash: "lch".into(),
            status: WorkflowManualResultReconciliationReadinessStatus::Ready,
            decision: WorkflowManualResultReconciliationReadinessDecision::Ready { summary: "ok".into() },
            predicates: vec![], reconciliation_preview: None,
            verifies_external_state: false, reconciles_now: false,
            mutates_workflow_state: false, creates_run_revision: false,
            appends_trace: false, writes_memory: false,
            routes_action: false, resolves_approval: false,
            creates_execution_grant: false, execution_allowed_now: false,
            evaluator: "test".into(), evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn ui_state_loads_latest_readiness() {
        let state = WorkflowManualResultReconciliationReadinessUiState {
            latest_readiness: Some(workflow_reconciliation_readiness_summary_lines(&test_record())),
            warnings: vec![],
        };
        assert!(state.latest_readiness.is_some());
        assert_eq!("ready", state.latest_readiness.unwrap().status);
    }

    #[test]
    fn summary_lines_show_status() {
        let row = workflow_reconciliation_readiness_summary_lines(&test_record());
        assert_eq!("ready", row.status);
        assert_eq!("wmrrr_t", row.readiness_id);
    }

    #[test]
    fn safety_warning_mentions_readiness_not_reconciliation() {
        let w = workflow_reconciliation_readiness_safety_warning();
        assert!(w.contains("does not reconcile"));
        assert!(w.contains("readiness"));
    }
}
