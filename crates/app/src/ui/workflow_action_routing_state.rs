//! UI workflow action routing state — read-only display helpers.

use openwand_workflow::workflow_action_route::*;

#[derive(Debug, Clone)]
pub struct WorkflowActionRouteSummaryRow { pub route_id: String, pub status: String, pub stage_id: String, pub action_request_id: String }

#[derive(Debug, Clone)]
pub struct WorkflowActionRoutePredicateRow { pub predicate: String, pub passed: bool, pub reason: String }

#[derive(Debug, Clone)]
pub struct WorkflowSessionRouteRow { pub session_id: String, pub session_status: String, pub trace_count: usize, pub pending_approval: bool }

#[derive(Debug, Clone)]
pub struct WorkflowActionRoutePromptRow { pub capability: String, pub purpose: String, pub governance_constraint: bool }

#[derive(Debug, Clone)]
pub struct WorkflowActionRouteUiState {
    pub latest_route: Option<WorkflowActionRouteSummaryRow>,
    pub predicates: Vec<WorkflowActionRoutePredicateRow>,
    pub session_route: Option<WorkflowSessionRouteRow>,
    pub route_prompt: Option<WorkflowActionRoutePromptRow>,
    pub warnings: Vec<String>,
}

pub fn workflow_action_route_safety_warning() -> String {
    "Workflow action routing sends a descriptive action request into the existing SessionRunner path. \
     Workflow does not execute tools, approve tools, append trace, or construct tool calls directly.".into()
}

pub fn workflow_action_route_summary(record: &WorkflowActionRouteRecord) -> WorkflowActionRouteSummaryRow {
    WorkflowActionRouteSummaryRow {
        route_id: record.route_id.0.clone(),
        status: format!("{:?}", record.status).to_lowercase(),
        stage_id: record.stage_id.clone(),
        action_request_id: record.action_request_id.clone(),
    }
}

pub fn workflow_action_route_predicate_rows(record: &WorkflowActionRouteRecord) -> Vec<WorkflowActionRoutePredicateRow> {
    record.predicates.iter().map(|p| WorkflowActionRoutePredicateRow {
        predicate: format!("{:?}", p.predicate), passed: p.passed, reason: p.reason.clone(),
    }).collect()
}

pub fn workflow_session_route_row(record: &WorkflowActionRouteRecord) -> Option<WorkflowSessionRouteRow> {
    record.session_route.as_ref().map(|sr| WorkflowSessionRouteRow {
        session_id: sr.session_id.clone(),
        session_status: sr.session_status.clone(),
        trace_count: sr.trace_ids.len(),
        pending_approval: sr.pending_approval_id.is_some(),
    })
}

pub fn workflow_action_route_prompt_row(record: &WorkflowActionRouteRecord) -> WorkflowActionRoutePromptRow {
    let p = &record.route_prompt;
    let instruction = p.to_session_instruction();
    WorkflowActionRoutePromptRow {
        capability: p.capability_category.clone(),
        purpose: p.purpose.clone(),
        governance_constraint: instruction.contains("Do not treat this workflow action request as a direct tool call"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_readiness::WorkflowReadinessId;
    use openwand_workflow::workflow_proposal::WorkflowProposalId;
    use openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId;
    use chrono::Utc;

    fn test_record() -> WorkflowActionRouteRecord {
        WorkflowActionRouteRecord {
            route_id: WorkflowActionRouteId("war_test".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            readiness_id: WorkflowReadinessId("wfrd_t".into()),
            proposal_id: WorkflowProposalId("wfp_t".into()),
            stage_id: "stage_1".into(),
            action_request_id: "ar_1".into(),
            action_request_hash: "h".into(),
            status: WorkflowActionRouteStatus::Completed,
            decision: WorkflowActionRouteDecision::Completed { summary: "session turn completed".into() },
            predicates: vec![WorkflowActionRoutePredicateResult {
                predicate: WorkflowActionRoutePredicate::WorkflowRunExists, passed: true, reason: "ok".into(),
            }],
            session_route: Some(WorkflowSessionRouteSnapshot {
                session_id: "sess_1".into(), session_run_id: Some("run_1".into()),
                trace_ids: vec!["trace_1".into()], pending_approval_id: None,
                tool_call_id: None, tool_name_observed_from_session: None,
                session_status: "completed".into(),
            }),
            route_prompt: WorkflowActionRoutePrompt {
                capability_category: "file-read".into(), purpose: "Read config".into(),
                expected_input_summary: "path".into(), expected_output_summary: "contents".into(),
                safety_constraints: vec![],
            },
            created_at: Utc::now(), completed_at: Some(Utc::now()),
        }
    }

    #[test]
    fn ui_state_loads_latest_route() {
        let r = test_record();
        let state = WorkflowActionRouteUiState {
            latest_route: Some(workflow_action_route_summary(&r)),
            predicates: workflow_action_route_predicate_rows(&r),
            session_route: workflow_session_route_row(&r),
            route_prompt: Some(workflow_action_route_prompt_row(&r)),
            warnings: vec![],
        };
        assert!(state.latest_route.is_some());
        assert!(!state.predicates.is_empty());
        assert!(state.session_route.is_some());
    }

    #[test]
    fn predicate_rows_show_pass_fail_reason() {
        let rows = workflow_action_route_predicate_rows(&test_record());
        assert!(!rows.is_empty());
        assert!(rows[0].passed);
        assert!(!rows[0].reason.is_empty());
    }

    #[test]
    fn session_route_lines_show_session_trace_links() {
        let row = workflow_session_route_row(&test_record()).unwrap();
        assert_eq!("sess_1", row.session_id);
        assert_eq!(1, row.trace_count);
    }

    #[test]
    fn prompt_lines_show_descriptive_fields_only() {
        let row = workflow_action_route_prompt_row(&test_record());
        assert_eq!("file-read", row.capability);
        assert!(row.governance_constraint);
    }

    #[test]
    fn safety_warning_mentions_session_seams() {
        let w = workflow_action_route_safety_warning();
        assert!(w.contains("SessionRunner"));
        assert!(!w.contains("executes tools directly"));
    }
}
