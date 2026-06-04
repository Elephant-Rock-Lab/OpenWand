//! UI workflow loop controller state — read-only display helpers.

use openwand_workflow::workflow_loop_controller::*;
use openwand_workflow::workflow_loop_recommendation::*;
use openwand_workflow::workflow_loop_state::*;

#[derive(Debug, Clone)]
pub struct WorkflowLoopControllerSummaryRow { pub controller_id: String, pub status: String }
#[derive(Debug, Clone)]
pub struct WorkflowLoopRecommendationRow { pub operation: String, pub reason: String }
#[derive(Debug, Clone)]
pub struct WorkflowLoopPredicateRow { pub predicate: String, pub passed: bool, pub reason: String }
#[derive(Debug, Clone)]
pub struct WorkflowLoopEvidenceRow { pub link_kind: String, pub record_id: String, pub summary: String }

#[derive(Debug, Clone)]
pub struct WorkflowLoopControllerUiState {
    pub latest_controller: Option<WorkflowLoopControllerSummaryRow>,
    pub detected_state: Option<String>,
    pub recommendation: Option<WorkflowLoopRecommendationRow>,
    pub predicates: Vec<WorkflowLoopPredicateRow>,
    pub evidence_links: Vec<WorkflowLoopEvidenceRow>,
    pub warnings: Vec<String>,
}

pub fn workflow_loop_controller_summary_lines(record: &WorkflowLoopControllerRecord) -> WorkflowLoopControllerSummaryRow {
    WorkflowLoopControllerSummaryRow {
        controller_id: record.controller_id.0.clone(),
        status: format!("{:?}", record.status).to_lowercase(),
    }
}

pub fn workflow_loop_detected_state_lines(state: &WorkflowLoopState) -> String {
    format!("{:?}", state.detected_state).to_lowercase()
}

pub fn workflow_loop_recommendation_lines(rec: &WorkflowLoopRecommendation) -> WorkflowLoopRecommendationRow {
    WorkflowLoopRecommendationRow {
        operation: format!("{:?}", rec.operation).to_lowercase(),
        reason: rec.reason.clone(),
    }
}

pub fn workflow_loop_predicate_rows(record: &WorkflowLoopControllerRecord) -> Vec<WorkflowLoopPredicateRow> {
    record.predicates.iter().map(|p| WorkflowLoopPredicateRow {
        predicate: format!("{:?}", p.predicate), passed: p.passed, reason: p.reason.clone(),
    }).collect()
}

pub fn workflow_loop_evidence_rows(record: &WorkflowLoopControllerRecord) -> Vec<WorkflowLoopEvidenceRow> {
    record.evidence_links.iter().map(|e| WorkflowLoopEvidenceRow {
        link_kind: e.link_kind.clone(), record_id: e.record_id.clone(), summary: e.summary.clone(),
    }).collect()
}

pub fn workflow_loop_controller_safety_warning() -> String {
    "Workflow loop controller recommends the next manual operation only. It does not \
     route actions, resolve approvals, reconcile outcomes, execute tools, append \
     trace, or mutate workflow state.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_loop_recommendation::WorkflowManualOperationKind;
    use chrono::Utc;

    fn test_record() -> WorkflowLoopControllerRecord {
        WorkflowLoopControllerRecord {
            controller_id: WorkflowLoopControllerId("wlc_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            latest_run_revision_id: None,
            status: WorkflowLoopControllerStatus::RecommendationReady,
            decision: WorkflowLoopControllerDecision::Recommend {
                operation: WorkflowManualOperationKind::CreateContinuationProposal,
                summary: "test".into(),
            },
            loop_state: Some(WorkflowLoopState {
                workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
                latest_run_revision_id: None, run_status: "suspended".into(),
                stage_summary: vec![], latest_route_id: None, latest_outcome_id: None,
                latest_reconciliation_id: None, latest_continuation_readiness_id: None,
                latest_next_action_proposal_id: None, latest_next_action_review_id: None,
                latest_routing_readiness_id: None, latest_next_action_routing_id: None,
                detected_state: WorkflowDetectedLoopState::NeedsInitialContinuationProposal,
            }),
            recommendation: Some(WorkflowLoopRecommendation {
                operation: WorkflowManualOperationKind::CreateContinuationProposal,
                command_hint: "display only".into(),
                reason: "No continuation proposal".into(),
                required_inputs: vec![], evidence_links: vec![],
            }),
            predicates: vec![WorkflowLoopPredicateResult {
                predicate: WorkflowLoopPredicate::WorkflowRunExists, passed: true, reason: "ok".into(),
            }],
            evidence_links: vec![],
            creates_route: false, resolves_approval: false, reconciles_outcome: false,
            executes_tool: false, mutates_workflow_state: false,
            schedules_work: false, starts_worker: false, queues_operation: false,
            retries_operation: false, resumes_workflow: false,
            created_at: Utc::now(),
        }
    }

    #[test] fn ui_state_loads_latest_workflow_loop_controller() {
        let state = WorkflowLoopControllerUiState {
            latest_controller: Some(workflow_loop_controller_summary_lines(&test_record())),
            detected_state: None, recommendation: None, predicates: vec![],
            evidence_links: vec![], warnings: vec![],
        };
        assert!(state.latest_controller.is_some());
        assert!(state.latest_controller.unwrap().status.contains("recommendation"));
    }
    #[test] fn detected_state_lines_show_state() {
        let rec = test_record();
        let state = rec.loop_state.unwrap();
        let lines = workflow_loop_detected_state_lines(&state);
        assert!(lines.contains("needsinitialcontinuationproposal"));
    }
    #[test] fn recommendation_lines_show_operation_and_reason() {
        let rec = test_record();
        let row = workflow_loop_recommendation_lines(rec.recommendation.as_ref().unwrap());
        assert!(row.operation.contains("create"));
        assert!(row.reason.contains("No continuation"));
    }
    #[test] fn predicate_rows_show_pass_fail_reason() {
        let rows = workflow_loop_predicate_rows(&test_record());
        assert!(!rows.is_empty()); assert!(rows[0].passed);
    }
    #[test] fn evidence_rows_show_link_kind_and_summary() {
        let mut rec = test_record();
        rec.evidence_links.push(openwand_workflow::workflow_loop_recommendation::WorkflowLoopEvidenceLink {
            link_kind: "route".into(), record_id: "war_t".into(), summary: "routed".into(),
        });
        let rows = workflow_loop_evidence_rows(&rec);
        assert_eq!(1, rows.len());
        assert_eq!("route", rows[0].link_kind);
    }
    #[test] fn safety_warning_mentions_manual_only() {
        let w = workflow_loop_controller_safety_warning();
        assert!(w.contains("recommends the next manual operation"));
        assert!(w.contains("does not route"));
        assert!(w.contains("resolve approvals"));
        assert!(w.contains("execute tools"));
    }
    // Patch 5: UI guard
    #[test] fn loop_controller_ui_does_not_expose_worker_schedule_queue() {
        let src = include_str!("../ui/workflow_loop_controller_state.rs");
        // Check function definitions and pub fn signatures only
        let pub_fns: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn ")).collect();
        assert!(!pub_fns.iter().any(|l| l.contains("schedule")));
        assert!(!pub_fns.iter().any(|l| l.contains("queue")));
        assert!(!pub_fns.iter().any(|l| l.contains("worker")));
    }
}
