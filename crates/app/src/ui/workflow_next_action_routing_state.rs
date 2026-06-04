//! UI next-action routing state — read-only display helpers.

use openwand_workflow::workflow_next_action_routing_gate::*;

#[derive(Debug, Clone)]
pub struct WorkflowNextActionRoutingSummaryRow { pub routing_id: String, pub status: String }
#[derive(Debug, Clone)]
pub struct WorkflowNextActionRoutingPredicateRow { pub predicate: String, pub passed: bool, pub reason: String }
#[derive(Debug, Clone)]
pub struct WorkflowNextActionRouteLinkRow { pub routing_id: String, pub route_id: String }

#[derive(Debug, Clone)]
pub struct WorkflowNextActionRoutingUiState {
    pub latest_routing: Option<WorkflowNextActionRoutingSummaryRow>,
    pub predicates: Vec<WorkflowNextActionRoutingPredicateRow>,
    pub route_link: Option<WorkflowNextActionRouteLinkRow>,
    pub warnings: Vec<String>,
}

pub fn workflow_next_action_routing_summary_lines(record: &WorkflowNextActionRoutingRecord) -> WorkflowNextActionRoutingSummaryRow {
    WorkflowNextActionRoutingSummaryRow {
        routing_id: record.routing_id.0.clone(),
        status: format!("{:?}", record.status).to_lowercase(),
    }
}

pub fn workflow_next_action_routing_predicate_rows(record: &WorkflowNextActionRoutingRecord) -> Vec<WorkflowNextActionRoutingPredicateRow> {
    record.predicates.iter().map(|p| WorkflowNextActionRoutingPredicateRow {
        predicate: format!("{:?}", p.predicate), passed: p.passed, reason: p.reason.clone(),
    }).collect()
}

pub fn workflow_next_action_route_link_lines(record: &WorkflowNextActionRoutingRecord) -> Option<WorkflowNextActionRouteLinkRow> {
    record.created_route_id.as_ref().map(|rid| WorkflowNextActionRouteLinkRow {
        routing_id: record.routing_id.0.clone(),
        route_id: rid.0.clone(),
    })
}

pub fn workflow_next_action_routing_safety_warning() -> String {
    "Reviewed next-action routing creates at most one workflow action route record \
     through the existing routing path. It does not execute tools, resolve approvals, \
     append trace, or mutate workflow state directly.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_action_route::WorkflowActionRouteId;
    use openwand_workflow::workflow_routing_readiness::WorkflowRoutingReadinessId;
    use openwand_workflow::workflow_continuation::WorkflowNextActionProposalId;
    use openwand_workflow::workflow_next_action_review::WorkflowNextActionReviewId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;
    use chrono::Utc;

    fn test_record() -> WorkflowNextActionRoutingRecord {
        WorkflowNextActionRoutingRecord {
            routing_id: WorkflowNextActionRoutingId("wnaroute_t".into()),
            routing_readiness_id: WorkflowRoutingReadinessId("wrrd_t".into()),
            next_action_proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            next_action_review_id: WorkflowNextActionReviewId("wnar_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            status: WorkflowNextActionRoutingStatus::Routed,
            decision: WorkflowNextActionRoutingDecision::Routed {
                route_id: WorkflowActionRouteId("war_t".into()), summary: "ok".into(),
            },
            predicates: vec![WorkflowNextActionRoutingPredicateResult {
                predicate: WorkflowNextActionRoutingPredicate::RoutingReadinessExists,
                passed: true, reason: "ok".into(),
            }],
            route_request_preview_hash: "ph".into(),
            created_route_id: Some(WorkflowActionRouteId("war_t".into())),
            created_at: Utc::now(), completed_at: Some(Utc::now()),
        }
    }

    #[test]
    fn ui_state_loads_latest_next_action_routing() {
        let state = WorkflowNextActionRoutingUiState {
            latest_routing: Some(workflow_next_action_routing_summary_lines(&test_record())),
            predicates: vec![], route_link: None, warnings: vec![],
        };
        assert!(state.latest_routing.is_some());
        assert_eq!("routed", state.latest_routing.unwrap().status);
    }

    #[test]
    fn routing_predicate_rows_show_pass_fail_reason() {
        let rows = workflow_next_action_routing_predicate_rows(&test_record());
        assert!(!rows.is_empty()); assert!(rows[0].passed);
    }

    #[test]
    fn route_link_lines_show_created_route_id() {
        let link = workflow_next_action_route_link_lines(&test_record()).unwrap();
        assert_eq!("wnaroute_t", link.routing_id);
        assert_eq!("war_t", link.route_id);
    }

    #[test]
    fn safety_warning_mentions_existing_route_path_and_no_execution() {
        let w = workflow_next_action_routing_safety_warning();
        assert!(w.contains("existing routing path"));
        assert!(w.contains("does not execute tools"));
        assert!(w.contains("resolve approvals"));
        assert!(w.contains("append trace"));
    }
}
