//! UI workflow execution state — read-only display helpers.

use openwand_workflow::workflow_run::*;

#[derive(Debug, Clone)]
pub struct WorkflowRunSummaryRow { pub execution_id: String, pub status: String, pub stage_count: usize, pub predicates_passed: usize, pub predicates_total: usize }
#[derive(Debug, Clone)]
pub struct WorkflowExecutionPredicateRow { pub predicate: String, pub passed: bool, pub reason: String }
#[derive(Debug, Clone)]
pub struct WorkflowStageRunRow { pub stage_id: String, pub kind: String, pub status: String, pub order: u32, pub summary: String }
#[derive(Debug, Clone)]
pub struct WorkflowLifecycleEventRow { pub event_id: String, pub stage_id: String, pub event_kind: String, pub summary: String }
#[derive(Debug, Clone)]
pub struct WorkflowActionRequestRow { pub action_request_id: String, pub capability: String, pub routing_status: String }
#[derive(Debug, Clone)]
pub struct WorkflowAbortSnapshotRow { pub abort_available: bool, pub rollback_available: bool, pub notes: Vec<String> }

#[derive(Debug, Clone)]
pub struct WorkflowExecutionUiState {
    pub latest_run: Option<WorkflowRunSummaryRow>,
    pub predicates: Vec<WorkflowExecutionPredicateRow>,
    pub stages: Vec<WorkflowStageRunRow>,
    pub lifecycle_events: Vec<WorkflowLifecycleEventRow>,
    pub action_requests: Vec<WorkflowActionRequestRow>,
    pub abort_snapshot: Option<WorkflowAbortSnapshotRow>,
    pub warnings: Vec<String>,
}

pub fn workflow_execution_safety_warning() -> String {
    "Workflow execution creates governed run evidence and stage lifecycle records. Tools still execute only through SessionRunner, PolicyEngine, ToolExecutor, and Trace. A workflow run is not direct tool authority.".into()
}

pub fn workflow_execution_summary(record: &WorkflowRunRecord) -> WorkflowRunSummaryRow {
    let passed = record.predicates.iter().filter(|p| p.passed).count();
    WorkflowRunSummaryRow { execution_id: record.execution_id.0.clone(), status: format!("{:?}", record.status).to_lowercase(),
        stage_count: record.stages.len(), predicates_passed: passed, predicates_total: record.predicates.len() }
}

pub fn workflow_execution_predicate_rows(record: &WorkflowRunRecord) -> Vec<WorkflowExecutionPredicateRow> {
    record.predicates.iter().map(|p| WorkflowExecutionPredicateRow { predicate: format!("{:?}", p.predicate), passed: p.passed, reason: p.reason.clone() }).collect()
}

pub fn workflow_stage_run_rows(record: &WorkflowRunRecord) -> Vec<WorkflowStageRunRow> {
    record.stages.iter().map(|s| WorkflowStageRunRow { stage_id: s.stage_id.clone(), kind: format!("{:?}", s.kind).to_lowercase(),
        status: format!("{:?}", s.status).to_lowercase(), order: s.order, summary: s.summary.clone() }).collect()
}

pub fn workflow_lifecycle_event_rows(record: &WorkflowRunRecord) -> Vec<WorkflowLifecycleEventRow> {
    record.lifecycle_events.iter().map(|e| WorkflowLifecycleEventRow { event_id: e.event_id.clone(), stage_id: e.stage_id.clone(),
        event_kind: format!("{:?}", e.event_kind).to_lowercase(), summary: e.summary.clone() }).collect()
}

pub fn workflow_action_request_rows(record: &WorkflowRunRecord) -> Vec<WorkflowActionRequestRow> {
    record.action_requests.iter().map(|a| WorkflowActionRequestRow { action_request_id: a.action_request_id.clone(),
        capability: a.capability_category.clone(), routing_status: format!("{:?}", a.routing_status).to_lowercase() }).collect()
}

pub fn workflow_abort_snapshot_lines(record: &WorkflowRunRecord) -> WorkflowAbortSnapshotRow {
    WorkflowAbortSnapshotRow { abort_available: record.abort_snapshot.abort_notes_available,
        rollback_available: record.abort_snapshot.rollback_notes_available, notes: record.abort_snapshot.recovery_notes.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_record() -> WorkflowRunRecord {
        WorkflowRunRecord { execution_id: WorkflowExecutionId("wfx_t".into()),
            readiness_id: openwand_workflow::workflow_readiness::WorkflowReadinessId("wfrd_t".into()),
            proposal_id: openwand_workflow::workflow_proposal::WorkflowProposalId("wfp_t".into()),
            proposal_review_id: openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId("wfr_t".into()),
            source_task_plan_id: openwand_workflow::plan::TaskPlanId("tpl_t".into()),
            status: WorkflowRunStatus::Suspended, decision: WorkflowExecutionDecision::RunCreated,
            predicates: vec![WorkflowExecutionPredicateResult { predicate: WorkflowExecutionPredicate::ReadinessRecordExists, passed: true, reason: "ok".into() }],
            run_snapshot: WorkflowRunSnapshot { readiness_id: "r".into(), proposal_id: "p".into(), proposal_hash: "h".into(),
                source_task_plan_hash: "s".into(), readiness_status_at_execution: "ready".into(), proposal_review_decision_at_execution: "approved".into() },
            stages: vec![WorkflowStageRun { stage_id: "s1".into(), title: "Observe".into(), kind: openwand_workflow::workflow_proposal::WorkflowStageKind::Observe,
                status: WorkflowStageRunStatus::Completed, order: 0, depends_on: vec![], started_at: None, completed_at: None,
                summary: "Marked complete as non-tool deterministic stage".into() }],
            lifecycle_events: vec![], action_requests: vec![],
            abort_snapshot: WorkflowAbortSnapshot { abort_notes_available: true, rollback_notes_available: true, recovery_notes: vec!["Use git checkout".into()] },
            created_at: chrono::Utc::now(), completed_at: None }
    }

    #[test] fn workflow_execution_ui_state_loads_latest_run() {
        let r = test_record(); let state = WorkflowExecutionUiState { latest_run: Some(workflow_execution_summary(&r)),
            predicates: workflow_execution_predicate_rows(&r), stages: workflow_stage_run_rows(&r),
            lifecycle_events: workflow_lifecycle_event_rows(&r), action_requests: workflow_action_request_rows(&r),
            abort_snapshot: Some(workflow_abort_snapshot_lines(&r)), warnings: vec![] };
        assert!(state.latest_run.is_some()); assert!(!state.stages.is_empty());
    }
    #[test] fn workflow_execution_predicate_rows_show_pass_fail_reason() {
        let rows = workflow_execution_predicate_rows(&test_record()); assert!(!rows.is_empty()); assert!(rows[0].passed); assert!(!rows[0].reason.is_empty()); }
    #[test] fn workflow_stage_run_rows_show_status_order_kind() {
        let rows = workflow_stage_run_rows(&test_record()); assert!(!rows.is_empty()); assert_eq!("completed", rows[0].status); assert_eq!(0, rows[0].order); }
    #[test] fn workflow_lifecycle_event_rows_show_stage_event_summary() { let rows = workflow_lifecycle_event_rows(&test_record()); /* no events in test record */ assert!(rows.is_empty()); }
    #[test] fn workflow_action_request_rows_show_prepared_not_executed() { let rows = workflow_action_request_rows(&test_record()); for r in &rows { assert!(r.routing_status.contains("prepared")); assert!(!r.routing_status.contains("executed")); } }
    #[test] fn workflow_abort_snapshot_lines_show_recovery_notes() { let snap = workflow_abort_snapshot_lines(&test_record()); assert!(snap.abort_available); assert!(!snap.notes.is_empty()); }
    #[test] fn workflow_execution_safety_warning_mentions_tool_seams() { let w = workflow_execution_safety_warning(); assert!(w.contains("SessionRunner")); assert!(w.contains("ToolExecutor")); assert!(!w.contains("executes tools directly")); }
}
