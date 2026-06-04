//! UI command review state — display-only helpers.

use openwand_workflow::workflow_command_review::*;

#[derive(Debug, Clone)]
pub struct WorkflowCommandReviewSummaryRow { pub review_id: String, pub decision: String, pub reviewer: String }
#[derive(Debug, Clone)]
pub struct WorkflowCommandAcknowledgmentSnapshotRow {
    pub display_command: String,
    pub copyable_text_hash: String,
    pub display_only: bool,
    pub executable: bool,
    pub missing_inputs: Vec<String>,
    pub review_only: bool,
    pub performed: bool,
}

#[derive(Debug, Clone)]
pub struct WorkflowCommandReviewUiState {
    pub latest_review: Option<WorkflowCommandReviewSummaryRow>,
    pub acknowledgment_snapshot: Option<WorkflowCommandAcknowledgmentSnapshotRow>,
    pub feedback: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn workflow_command_review_summary_lines(record: &WorkflowCommandReview) -> WorkflowCommandReviewSummaryRow {
    WorkflowCommandReviewSummaryRow {
        review_id: record.review_id.0.clone(),
        decision: format!("{:?}", record.decision).to_lowercase(),
        reviewer: record.reviewer.clone(),
    }
}

pub fn workflow_command_acknowledgment_snapshot_lines(snapshot: &WorkflowCommandAcknowledgmentSnapshot) -> WorkflowCommandAcknowledgmentSnapshotRow {
    WorkflowCommandAcknowledgmentSnapshotRow {
        display_command: snapshot.descriptor_display_command.clone(),
        copyable_text_hash: snapshot.descriptor_copyable_text_hash.clone(),
        display_only: snapshot.descriptor_display_only,
        executable: snapshot.descriptor_executable,
        missing_inputs: snapshot.descriptor_missing_inputs.clone(),
        review_only: snapshot.acknowledges_review_only,
        performed: snapshot.command_performed_now,
    }
}

pub fn workflow_command_review_feedback_lines(record: &WorkflowCommandReview) -> Vec<String> {
    match &record.feedback {
        Some(fb) => {
            let mut lines = vec![format!("Summary: {}", fb.summary)];
            for r in &fb.blocking_reasons { lines.push(format!("Blocking: {}", r)); }
            for c in &fb.requested_changes { lines.push(format!("Change: {}", c)); }
            for g in &fb.evidence_gaps { lines.push(format!("Gap: {}", g)); }
            lines
        }
        None => vec![],
    }
}

pub fn workflow_command_review_safety_warning() -> String {
    "Command review records operator acknowledgment only. It does not execute the \
     command, route actions, resolve approvals, reconcile outcomes, schedule work, or \
     mutate workflow state.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use chrono::Utc;

    fn test_review() -> WorkflowCommandReview {
        WorkflowCommandReview {
            review_id: WorkflowCommandReviewId("wcrv_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            command_composer_hash: "ch".into(),
            command_descriptor_hash: "dh".into(),
            loop_controller_hash: "lh".into(),
            decision: WorkflowCommandReviewDecision::Acknowledged,
            reviewer: "tester".into(),
            rationale: "ok".into(),
            feedback: Some(WorkflowCommandReviewFeedback {
                summary: "test feedback".into(),
                blocking_reasons: vec!["reason1".into()],
                requested_changes: vec!["change1".into()],
                evidence_gaps: vec![],
            }),
            acknowledgment_snapshot: WorkflowCommandAcknowledgmentSnapshot {
                descriptor_display_command: "openwand test".into(),
                descriptor_copyable_text_hash: "cth".into(),
                descriptor_display_only: true, descriptor_executable: false,
                descriptor_missing_inputs: vec!["decision".into()],
                loop_detected_state: "idle".into(),
                loop_recommended_operation: "none".into(),
                acknowledges_review_only: true, command_performed_now: false,
            },
            executes_command: false, invokes_shell: false, invokes_git: false,
            routes_action: false, resolves_approval: false, reconciles_outcome: false,
            mutates_workflow_state: false, schedules_work: false, starts_worker: false,
            queues_operation: false, creates_execution_grant: false,
            execution_allowed_now: false, reviewed_at: Utc::now(),
        }
    }

    #[test]
    fn ui_state_loads_latest_command_review() {
        let state = WorkflowCommandReviewUiState {
            latest_review: Some(workflow_command_review_summary_lines(&test_review())),
            acknowledgment_snapshot: None, feedback: vec![], warnings: vec![],
        };
        assert!(state.latest_review.is_some());
        assert_eq!("acknowledged", state.latest_review.unwrap().decision);
    }

    #[test]
    fn review_summary_lines_show_decision_and_reviewer() {
        let row = workflow_command_review_summary_lines(&test_review());
        assert_eq!("acknowledged", row.decision);
        assert_eq!("tester", row.reviewer);
    }

    #[test]
    fn acknowledgment_snapshot_lines_show_descriptor_binding() {
        let row = workflow_command_acknowledgment_snapshot_lines(
            &test_review().acknowledgment_snapshot);
        assert!(row.display_command.contains("openwand"));
        assert!(row.display_only);
        assert!(!row.executable);
    }

    // Patch 4: snapshot lines say not performed
    #[test]
    fn acknowledgment_snapshot_lines_say_not_performed() {
        let row = workflow_command_acknowledgment_snapshot_lines(
            &test_review().acknowledgment_snapshot);
        assert!(row.review_only);
        assert!(!row.performed);
    }

    #[test]
    fn feedback_lines_show_blocking_reasons_and_requested_changes() {
        let lines = workflow_command_review_feedback_lines(&test_review());
        assert!(lines.iter().any(|l| l.contains("Blocking: reason1")));
        assert!(lines.iter().any(|l| l.contains("Change: change1")));
    }

    #[test]
    fn safety_warning_mentions_no_execution_or_schedule() {
        let w = workflow_command_review_safety_warning();
        assert!(w.contains("does not execute"));
        assert!(w.contains("schedule"));
        assert!(w.contains("route actions"));
    }
}
