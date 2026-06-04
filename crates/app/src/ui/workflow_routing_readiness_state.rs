//! UI routing readiness state — read-only display helpers.

use openwand_workflow::workflow_next_action_review::*;
use openwand_workflow::workflow_routing_readiness::*;
use openwand_workflow::workflow_continuation::WorkflowNextActionCandidate;

#[derive(Debug, Clone)]
pub struct WorkflowNextActionReviewRow { pub review_id: String, pub decision: String, pub reviewer: String }
#[derive(Debug, Clone)]
pub struct WorkflowRoutingReadinessRow { pub readiness_id: String, pub status: String }
#[derive(Debug, Clone)]
pub struct WorkflowRoutingReadinessPredicateRow { pub predicate: String, pub passed: bool, pub reason: String }
#[derive(Debug, Clone)]
pub struct WorkflowNextActionCandidateRow { pub stage_id: String, pub candidate_kind: String }
#[derive(Debug, Clone)]
pub struct WorkflowRouteRequestPreviewRow { pub stage_id: String, pub action_request_id: String, pub descriptive_only: bool }

#[derive(Debug, Clone)]
pub struct WorkflowRoutingReadinessUiState {
    pub latest_review: Option<WorkflowNextActionReviewRow>,
    pub latest_readiness: Option<WorkflowRoutingReadinessRow>,
    pub predicates: Vec<WorkflowRoutingReadinessPredicateRow>,
    pub candidate: Option<WorkflowNextActionCandidateRow>,
    pub route_preview: Option<WorkflowRouteRequestPreviewRow>,
    pub feedback: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn workflow_routing_readiness_safety_warning() -> String {
    "Routing readiness is evidence only. It does not route actions, create session \
     turns, approve tools, execute tools, append trace, or mutate workflow state.".into()
}

pub fn workflow_next_action_review_lines(review: &WorkflowNextActionReview) -> WorkflowNextActionReviewRow {
    WorkflowNextActionReviewRow {
        review_id: review.review_id.0.clone(),
        decision: format!("{:?}", review.decision).to_lowercase(),
        reviewer: review.reviewer.clone(),
    }
}

pub fn workflow_next_action_feedback_lines(review: &WorkflowNextActionReview) -> Vec<String> {
    review.feedback.as_ref().map(|fb| {
        let mut lines = vec![format!("Summary: {}", fb.summary)];
        lines.extend(fb.blocking_reasons.iter().map(|r| format!("Blocking: {}", r)));
        lines.extend(fb.requested_changes.iter().map(|c| format!("Change: {}", c)));
        lines.extend(fb.evidence_gaps.iter().map(|g| format!("Gap: {}", g)));
        lines
    }).unwrap_or_default()
}

pub fn workflow_routing_readiness_summary(record: &WorkflowRoutingReadinessRecord) -> WorkflowRoutingReadinessRow {
    WorkflowRoutingReadinessRow {
        readiness_id: record.readiness_id.0.clone(),
        status: format!("{:?}", record.status).to_lowercase(),
    }
}

pub fn workflow_routing_readiness_predicate_rows(record: &WorkflowRoutingReadinessRecord) -> Vec<WorkflowRoutingReadinessPredicateRow> {
    record.predicates.iter().map(|p| WorkflowRoutingReadinessPredicateRow {
        predicate: format!("{:?}", p.predicate), passed: p.passed, reason: p.reason.clone(),
    }).collect()
}

pub fn workflow_route_request_preview_lines(preview: &WorkflowRouteRequestPreview) -> WorkflowRouteRequestPreviewRow {
    WorkflowRouteRequestPreviewRow {
        stage_id: preview.stage_id.clone(),
        action_request_id: preview.action_request_id.clone(),
        descriptive_only: preview.descriptive_only,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_continuation::WorkflowNextActionProposalId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use chrono::Utc;

    fn test_review() -> WorkflowNextActionReview {
        WorkflowNextActionReview {
            review_id: WorkflowNextActionReviewId("wnar_t".into()),
            proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            proposal_hash: "ph".into(),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            source_run_revision_hash: "rh".into(),
            decision: WorkflowNextActionReviewDecision::Approved,
            reviewer: "alice".into(), rationale: "safe".into(),
            feedback: Some(WorkflowNextActionFeedback {
                summary: "issues".into(), blocking_reasons: vec!["risk".into()],
                requested_changes: vec![], evidence_gaps: vec![],
            }),
            creates_route: false, routes_action_now: false,
            executes_tool_now: false, mutates_workflow_state_now: false,
            reviewed_at: Utc::now(),
        }
    }

    fn test_readiness() -> WorkflowRoutingReadinessRecord {
        WorkflowRoutingReadinessRecord {
            readiness_id: WorkflowRoutingReadinessId("wrrd_t".into()),
            proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            review_id: WorkflowNextActionReviewId("wnar_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            proposal_hash: "ph".into(), run_revision_hash: "rh".into(),
            status: WorkflowRoutingReadinessStatus::Ready,
            decision: WorkflowRoutingReadinessDecision::Ready { summary: "ok".into() },
            predicates: vec![WorkflowRoutingReadinessPredicateResult {
                predicate: WorkflowRoutingReadinessPredicate::NextActionProposalExists, passed: true, reason: "ok".into(),
            }],
            candidate: None, route_request_preview: Some(WorkflowRouteRequestPreview {
                workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
                stage_id: "s1".into(), action_request_id: "ar_1".into(),
                source_proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
                source_review_id: WorkflowNextActionReviewId("wnar_t".into()),
                descriptive_only: true, creates_route_now: false,
            }),
            created_at: Utc::now(),
        }
    }

    #[test] fn ui_state_loads_latest_review_and_readiness() {
        let state = WorkflowRoutingReadinessUiState {
            latest_review: Some(workflow_next_action_review_lines(&test_review())),
            latest_readiness: Some(workflow_routing_readiness_summary(&test_readiness())),
            predicates: vec![], candidate: None, route_preview: None, feedback: vec![], warnings: vec![],
        };
        assert!(state.latest_review.is_some());
        assert!(state.latest_readiness.is_some());
    }
    #[test] fn review_lines_show_decision() {
        let row = workflow_next_action_review_lines(&test_review());
        assert_eq!("approved", row.decision);
        assert_eq!("alice", row.reviewer);
    }
    #[test] fn feedback_lines_show_blocking_reasons() {
        let lines = workflow_next_action_feedback_lines(&test_review());
        assert!(lines.iter().any(|l| l.contains("Blocking: risk")));
    }
    #[test] fn readiness_predicate_rows_show_pass_fail_reason() {
        let rows = workflow_routing_readiness_predicate_rows(&test_readiness());
        assert!(!rows.is_empty()); assert!(rows[0].passed);
    }
    #[test] fn route_preview_lines_show_descriptive_non_route() {
        let preview = test_readiness().route_request_preview.unwrap();
        let row = workflow_route_request_preview_lines(&preview);
        assert!(row.descriptive_only);
        assert_eq!("s1", row.stage_id);
    }
    #[test] fn safety_warning_mentions_no_routing_or_execution() {
        let w = workflow_routing_readiness_safety_warning();
        assert!(w.contains("does not route actions"));
        assert!(w.contains("execute tools"));
        assert!(w.contains("append trace"));
    }
}
