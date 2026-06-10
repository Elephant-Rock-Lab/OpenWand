//! UI manual result state — display-only helpers.

use openwand_workflow::workflow_manual_result::*;

#[derive(Debug, Clone)]
pub struct WorkflowManualResultSummaryRow {
    pub result_id: String, pub status: String, pub operator: String,
    pub caveat: String,
    pub workflow_execution_id: String,
    pub command_review_hash: String, pub command_composer_hash: String,
    pub command_descriptor_hash: String, pub loop_controller_hash: String,
}
#[derive(Debug, Clone)]
pub struct WorkflowManualResultValidationRow {
    pub review_acknowledged: bool, pub review_hash_matched: bool,
    pub composer_hash_matched: bool, pub descriptor_hash_matched: bool,
    pub loop_hash_matched: bool,
}
#[derive(Debug, Clone)]
pub struct WorkflowManualArtifactReferenceRow {
    pub artifact_id: String, pub label: String, pub kind: String,
    pub reference: String, pub hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowManualResultUiState {
    pub latest_result: Option<WorkflowManualResultSummaryRow>,
    pub validation_snapshot: Option<WorkflowManualResultValidationRow>,
    pub artifact_references: Vec<WorkflowManualArtifactReferenceRow>,
    pub warnings: Vec<String>,
}

pub fn workflow_manual_result_summary_lines(record: &WorkflowManualResult) -> WorkflowManualResultSummaryRow {
    WorkflowManualResultSummaryRow {
        result_id: record.result_id.0.clone(),
        status: serde_json::to_string(&record.status).unwrap().trim_matches('"').to_string(),
        operator: record.operator.clone(),
        caveat: record.summary.caveat.clone(),
        workflow_execution_id: record.workflow_execution_id.0.clone(),
        command_review_hash: record.command_review_hash.clone(),
        command_composer_hash: record.command_composer_hash.clone(),
        command_descriptor_hash: record.command_descriptor_hash.clone(),
        loop_controller_hash: record.loop_controller_hash.clone(),
    }
}

pub fn workflow_manual_result_validation_lines(snapshot: &WorkflowManualResultValidationSnapshot) -> WorkflowManualResultValidationRow {
    WorkflowManualResultValidationRow {
        review_acknowledged: snapshot.command_review_was_acknowledged,
        review_hash_matched: snapshot.command_review_hash_matched,
        composer_hash_matched: snapshot.command_composer_hash_matched,
        descriptor_hash_matched: snapshot.command_descriptor_hash_matched,
        loop_hash_matched: snapshot.loop_controller_hash_matched,
    }
}

pub fn workflow_manual_artifact_reference_rows(artifacts: &[WorkflowManualArtifactReference]) -> Vec<WorkflowManualArtifactReferenceRow> {
    artifacts.iter().map(|a| WorkflowManualArtifactReferenceRow {
        artifact_id: a.artifact_id.clone(),
        label: a.label.clone(),
        kind: serde_json::to_string(&a.kind).unwrap().trim_matches('"').to_string(),
        reference: a.reference.clone(),
        hash: a.operator_supplied_hash.clone(),
    }).collect()
}

pub fn workflow_manual_result_safety_warning() -> String {
    "Manual result capture records operator-reported evidence only. OpenWand does not \
     execute commands, verify shell/git state, inspect artifact contents, append \
     trace, or mutate workflow state from this screen.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use chrono::Utc;

    fn test_result() -> WorkflowManualResult {
        WorkflowManualResult {
            result_id: WorkflowManualResultId("wmr_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            command_review_hash: "rh".into(), command_composer_hash: "ch".into(),
            command_descriptor_hash: "dh".into(), loop_controller_hash: "lh".into(),
            status: WorkflowManualResultStatus::ReportedSucceeded,
            operator: "tester".into(),
            summary: WorkflowManualResultSummary {
                operator_summary: "done".into(), operator_details: None,
                reported_status: WorkflowManualResultStatus::ReportedSucceeded,
                caveat: "Operator-reported, not verified by OpenWand.".into(),
            },
            artifact_references: vec![WorkflowManualArtifactReference {
                artifact_id: "art_1".into(), label: "log".into(),
                kind: WorkflowManualArtifactKind::LogExcerpt,
                reference: "/tmp/build.log".into(),
                operator_supplied_hash: Some("abc123".into()),
                description: Some("Build log".into()),
            }],
            validation_snapshot: WorkflowManualResultValidationSnapshot {
                command_review_was_acknowledged: true,
                command_review_hash_matched: true, command_composer_hash_matched: true,
                command_descriptor_hash_matched: true, loop_controller_hash_matched: true,
                command_review_marked_not_performed_by_openwand: true,
            },
            reported_by_operator: true,
            verified_by_openwand: false, command_executed_by_openwand: false,
            mutates_workflow_state: false, reconciles_outcome: false,
            routes_action: false, resolves_approval: false,
            appends_trace: false, writes_memory: false,
            invokes_shell: false, invokes_git: false,
            creates_execution_grant: false, execution_allowed_now: false,
            captured_at: Utc::now(),
        }
    }

    #[test]
    fn ui_state_loads_latest_manual_result() {
        let state = WorkflowManualResultUiState {
            latest_result: Some(workflow_manual_result_summary_lines(&test_result())),
            validation_snapshot: None, artifact_references: vec![], warnings: vec![],
        };
        assert!(state.latest_result.is_some());
        assert_eq!("reported_succeeded", state.latest_result.unwrap().status);
    }

    #[test]
    fn manual_result_summary_lines_show_reported_status() {
        let row = workflow_manual_result_summary_lines(&test_result());
        assert_eq!("reported_succeeded", row.status);
        assert!(row.caveat.contains("not verified"));
    }

    #[test]
    fn validation_lines_show_hash_binding() {
        let row = workflow_manual_result_validation_lines(&test_result().validation_snapshot);
        assert!(row.review_acknowledged);
        assert!(row.review_hash_matched);
        assert!(row.composer_hash_matched);
        assert!(row.descriptor_hash_matched);
        assert!(row.loop_hash_matched);
    }

    #[test]
    fn artifact_rows_show_reference_without_verification_claim() {
        let rows = workflow_manual_artifact_reference_rows(&test_result().artifact_references);
        assert_eq!(1, rows.len());
        assert_eq!("art_1", rows[0].artifact_id);
        // No verification claim — hash is operator-supplied
        assert!(rows[0].hash.is_some());
    }

    #[test]
    fn safety_warning_mentions_reported_not_verified() {
        let w = workflow_manual_result_safety_warning();
        assert!(w.contains("does not execute"));
        assert!(w.contains("verify"));
        assert!(w.contains("not"));
    }
}
