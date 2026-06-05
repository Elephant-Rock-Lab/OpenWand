//! UI manual result review state — display-only helpers.

use openwand_workflow::workflow_manual_result_review::*;

#[derive(Debug, Clone)]
pub struct WorkflowManualResultReviewSummaryRow {
    pub review_id: String,
    pub decision: String,
    pub reviewer: String,
    pub manual_result_id: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowManualResultReviewAcceptanceRow {
    pub accepts_reported_evidence: bool,
    pub verifies_external_state: bool,
    pub reconciles_workflow_state: bool,
    pub result_verified_by_openwand: bool,
}

#[derive(Debug, Clone)]
pub struct WorkflowManualResultReviewUiState {
    pub latest_review: Option<WorkflowManualResultReviewSummaryRow>,
    pub acceptance_snapshot: Option<WorkflowManualResultReviewAcceptanceRow>,
    pub warnings: Vec<String>,
}

pub fn workflow_manual_result_review_summary_lines(record: &WorkflowManualResultReview) -> WorkflowManualResultReviewSummaryRow {
    WorkflowManualResultReviewSummaryRow {
        review_id: record.review_id.0.clone(),
        decision: serde_json::to_string(&record.decision).unwrap().trim_matches('"').to_string(),
        reviewer: record.reviewer.clone(),
        manual_result_id: record.manual_result_id.0.clone(),
    }
}

pub fn workflow_manual_result_review_acceptance_lines(snapshot: &WorkflowManualResultReviewAcceptanceSnapshot) -> WorkflowManualResultReviewAcceptanceRow {
    WorkflowManualResultReviewAcceptanceRow {
        accepts_reported_evidence: snapshot.accepts_reported_evidence,
        verifies_external_state: snapshot.verifies_external_state,
        reconciles_workflow_state: snapshot.reconciles_workflow_state,
        result_verified_by_openwand: snapshot.result_verified_by_openwand,
    }
}

pub fn workflow_manual_result_review_safety_warning() -> String {
    "Manual result review accepts reported evidence only. OpenWand does not \
     verify external state, reconcile workflow state, execute commands, or \
     mutate any existing records from this screen.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_manual_result::WorkflowManualResultId;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use chrono::Utc;

    fn test_review() -> WorkflowManualResultReview {
        WorkflowManualResultReview {
            review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            manual_result_hash: "mrh".into(),
            command_review_hash: "crh".into(),
            command_composer_hash: "cch".into(),
            command_descriptor_hash: "cdh".into(),
            loop_controller_hash: "lch".into(),
            decision: WorkflowManualResultReviewDecision::Accepted,
            reviewer: "reviewer".into(),
            rationale: "ok".into(),
            feedback: None,
            acceptance_snapshot: WorkflowManualResultReviewAcceptanceSnapshot {
                accepts_reported_evidence: true,
                verifies_external_state: false,
                reconciles_workflow_state: false,
                result_verified_by_openwand: false,
            },
            verifies_external_state: false,
            reconciles_workflow_state: false,
            mutates_workflow_state: false,
            executes_command: false,
            invokes_shell: false,
            invokes_git: false,
            routes_action: false,
            resolves_approval: false,
            appends_trace: false,
            writes_memory: false,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        }
    }

    #[test]
    fn ui_state_loads_latest_review() {
        let state = WorkflowManualResultReviewUiState {
            latest_review: Some(workflow_manual_result_review_summary_lines(&test_review())),
            acceptance_snapshot: None,
            warnings: vec![],
        };
        assert!(state.latest_review.is_some());
        assert_eq!("accepted", state.latest_review.unwrap().decision);
    }

    #[test]
    fn review_summary_lines_show_decision() {
        let row = workflow_manual_result_review_summary_lines(&test_review());
        assert_eq!("accepted", row.decision);
        assert_eq!("wmrr_t", row.review_id);
        assert_eq!("wmr_t", row.manual_result_id);
    }

    #[test]
    fn acceptance_lines_show_reported_not_verified() {
        let row = workflow_manual_result_review_acceptance_lines(&test_review().acceptance_snapshot);
        assert!(row.accepts_reported_evidence);
        assert!(!row.verifies_external_state);
        assert!(!row.reconciles_workflow_state);
        assert!(!row.result_verified_by_openwand);
    }

    #[test]
    fn safety_warning_mentions_review_not_verification() {
        let w = workflow_manual_result_review_safety_warning();
        assert!(w.contains("does not verify"));
        assert!(w.contains("review"));
        assert!(w.contains("not"));
    }
}
