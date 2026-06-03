//! UI workflow action outcome state — read-only display helpers.

use openwand_workflow::workflow_action_outcome::*;

#[derive(Debug, Clone)]
pub struct WorkflowActionOutcomeSummaryRow { pub outcome_id: String, pub status: String, pub decision: String }
#[derive(Debug, Clone)]
pub struct WorkflowActionOutcomePredicateRow { pub predicate: String, pub passed: bool, pub reason: String }
#[derive(Debug, Clone)]
pub struct WorkflowApprovalResolutionRow { pub resolution: String, pub rationale: String }
#[derive(Debug, Clone)]
pub struct WorkflowSessionActionOutcomeRow { pub session_id: String, pub tool_name: Option<String>, pub tool_status: Option<String>, pub trace_count: usize }
#[derive(Debug, Clone)]
pub struct WorkflowOutcomeTraceLinkRow { pub trace_id: String }

#[derive(Debug, Clone)]
pub struct WorkflowActionOutcomeUiState {
    pub latest_outcome: Option<WorkflowActionOutcomeSummaryRow>,
    pub predicates: Vec<WorkflowActionOutcomePredicateRow>,
    pub approval_resolution: Option<WorkflowApprovalResolutionRow>,
    pub session_outcome: Option<WorkflowSessionActionOutcomeRow>,
    pub trace_links: Vec<WorkflowOutcomeTraceLinkRow>,
    pub warnings: Vec<String>,
}

pub fn workflow_action_outcome_safety_warning() -> String {
    "Workflow approval outcome linkage uses existing SessionRunner approval governance. \
     Workflow observes approval/tool/trace outcomes only and does not approve tools, execute tools, append trace, or mutate approval state directly.".into()
}

pub fn workflow_action_outcome_summary(record: &WorkflowActionOutcomeRecord) -> WorkflowActionOutcomeSummaryRow {
    WorkflowActionOutcomeSummaryRow {
        outcome_id: record.outcome_id.0.clone(),
        status: format!("{:?}", record.status).to_lowercase(),
        decision: format!("{:?}", record.decision).to_lowercase(),
    }
}

pub fn workflow_action_outcome_predicate_rows(record: &WorkflowActionOutcomeRecord) -> Vec<WorkflowActionOutcomePredicateRow> {
    record.predicates.iter().map(|p| WorkflowActionOutcomePredicateRow {
        predicate: format!("{:?}", p.predicate), passed: p.passed, reason: p.reason.clone(),
    }).collect()
}

pub fn workflow_approval_resolution_lines(record: &WorkflowActionOutcomeRecord) -> WorkflowApprovalResolutionRow {
    match &record.approval_resolution {
        WorkflowApprovalResolution::Approve { rationale } => WorkflowApprovalResolutionRow { resolution: "approved".into(), rationale: rationale.clone() },
        WorkflowApprovalResolution::Reject { rationale } => WorkflowApprovalResolutionRow { resolution: "rejected".into(), rationale: rationale.clone() },
    }
}

pub fn workflow_session_action_outcome_lines(record: &WorkflowActionOutcomeRecord) -> Option<WorkflowSessionActionOutcomeRow> {
    record.session_outcome.as_ref().map(|o| WorkflowSessionActionOutcomeRow {
        session_id: o.session_id.clone(),
        tool_name: o.tool_name_observed_from_session.clone(),
        tool_status: o.tool_status_observed_from_session.clone(),
        trace_count: o.trace_ids.len(),
    })
}

pub fn workflow_outcome_trace_link_rows(record: &WorkflowActionOutcomeRecord) -> Vec<WorkflowOutcomeTraceLinkRow> {
    record.session_outcome.as_ref().map(|o| o.trace_ids.iter().map(|t| WorkflowOutcomeTraceLinkRow { trace_id: t.clone() }).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_action_route::WorkflowActionRouteId;
    use chrono::Utc;

    fn test_record() -> WorkflowActionOutcomeRecord {
        WorkflowActionOutcomeRecord {
            outcome_id: WorkflowActionOutcomeId("wao_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            route_id: WorkflowActionRouteId("war_t".into()),
            stage_id: "s".into(), action_request_id: "ar".into(),
            session_id: "sess".into(), pending_approval_id: "arid".into(),
            tool_call_id: Some("tc".into()), route_hash: "rh".into(), workflow_run_hash: "wrh".into(),
            status: WorkflowActionOutcomeStatus::ToolCompleted,
            decision: WorkflowActionOutcomeDecision::ToolCompleted { summary: "done".into() },
            predicates: vec![WorkflowActionOutcomePredicateResult {
                predicate: WorkflowActionOutcomePredicate::WorkflowRunExists, passed: true, reason: "ok".into(),
            }],
            approval_resolution: WorkflowApprovalResolution::Approve { rationale: "safe".into() },
            session_outcome: Some(WorkflowSessionActionOutcomeSnapshot {
                session_id: "sess".into(), session_run_id: Some("run".into()),
                trace_ids: vec!["trace_1".into()], approval_request_id_observed: "arid".into(),
                approval_resolution_observed: "approved".into(),
                tool_call_id_observed_from_session: Some("tc".into()),
                tool_name_observed_from_session: Some("local__file_write".into()),
                tool_status_observed_from_session: Some("completed".into()),
                safe_result_summary: Some("ok".into()),
            }),
            created_at: Utc::now(), completed_at: Some(Utc::now()),
        }
    }

    #[test] fn ui_state_loads_latest_outcome() {
        let r = test_record();
        let state = WorkflowActionOutcomeUiState {
            latest_outcome: Some(workflow_action_outcome_summary(&r)),
            predicates: workflow_action_outcome_predicate_rows(&r),
            approval_resolution: Some(workflow_approval_resolution_lines(&r)),
            session_outcome: workflow_session_action_outcome_lines(&r),
            trace_links: workflow_outcome_trace_link_rows(&r),
            warnings: vec![],
        };
        assert!(state.latest_outcome.is_some());
        assert!(state.session_outcome.is_some());
    }
    #[test] fn predicate_rows_show_pass_fail_reason() {
        let rows = workflow_action_outcome_predicate_rows(&test_record());
        assert!(!rows.is_empty()); assert!(rows[0].passed);
    }
    #[test] fn approval_resolution_lines_show_decision() {
        let row = workflow_approval_resolution_lines(&test_record());
        assert_eq!("approved", row.resolution);
        assert_eq!("safe", row.rationale);
    }
    #[test] fn session_outcome_lines_show_tool_status() {
        let row = workflow_session_action_outcome_lines(&test_record()).unwrap();
        assert_eq!("completed", row.tool_status.unwrap());
        assert_eq!("local__file_write", row.tool_name.unwrap());
    }
    #[test] fn trace_link_rows_show_trace_ids() {
        let rows = workflow_outcome_trace_link_rows(&test_record());
        assert_eq!(1, rows.len()); assert_eq!("trace_1", rows[0].trace_id);
    }
    #[test] fn safety_warning_mentions_session_approval_governance() {
        let w = workflow_action_outcome_safety_warning();
        assert!(w.contains("SessionRunner"));
        assert!(w.contains("approval governance"));
        assert!(!w.contains("approves tools directly"));
    }
}
