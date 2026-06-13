//! UI workflow reconciliation state — read-only display helpers.

use openwand_workflow::workflow_reconciliation::*;

#[derive(Debug, Clone)]
pub struct WorkflowReconciliationSummaryRow { pub reconciliation_id: String, pub status: String, pub decision: String }
#[derive(Debug, Clone)]
pub struct WorkflowReconciliationPredicateRow { pub predicate: String, pub passed: bool, pub reason: String }
#[derive(Debug, Clone)]
pub struct WorkflowStageProgressionRow { pub stage_id: String, pub previous_status: String, pub new_status: String, pub summary: String }
#[derive(Debug, Clone)]
pub struct WorkflowRunRevisionRow { pub revision_id: String, pub workflow_execution_id: String, pub stage_count: usize, pub aggregate_status: Option<String> }
#[derive(Debug, Clone)]
pub struct WorkflowLifecycleEventRow { pub event_id: String, pub stage_id: String, pub event_kind: String, pub summary: String }

#[derive(Debug, Clone)]
pub struct WorkflowReconciliationUiState {
    pub latest_reconciliation: Option<WorkflowReconciliationSummaryRow>,
    pub latest_run_revision: Option<WorkflowRunRevisionRow>,
    pub predicates: Vec<WorkflowReconciliationPredicateRow>,
    pub progression: Option<WorkflowStageProgressionRow>,
    pub lifecycle_event: Option<WorkflowLifecycleEventRow>,
    pub warnings: Vec<String>,
}

pub fn workflow_reconciliation_safety_warning() -> String {
    "Workflow reconciliation updates run evidence from persisted session-produced \
     outcomes. It does not route actions, resolve approvals, execute tools, append \
     trace, or mutate session state.".into()
}

pub fn workflow_reconciliation_summary(record: &WorkflowReconciliationRecord) -> WorkflowReconciliationSummaryRow {
    WorkflowReconciliationSummaryRow {
        reconciliation_id: record.reconciliation_id.0.clone(),
        status: format!("{:?}", record.status).to_lowercase(),
        decision: format!("{:?}", record.decision).to_lowercase(),
    }
}

pub fn workflow_reconciliation_predicate_rows(record: &WorkflowReconciliationRecord) -> Vec<WorkflowReconciliationPredicateRow> {
    record.predicates.iter().map(|p| WorkflowReconciliationPredicateRow {
        predicate: format!("{:?}", p.predicate), passed: p.passed, reason: p.reason.clone(),
    }).collect()
}

pub fn workflow_stage_progression_lines(progression: &WorkflowStageProgression) -> WorkflowStageProgressionRow {
    WorkflowStageProgressionRow {
        stage_id: progression.stage_id.clone(),
        previous_status: format!("{:?}", progression.previous_status).to_lowercase(),
        new_status: format!("{:?}", progression.new_status).to_lowercase(),
        summary: progression.summary.clone(),
    }
}

pub fn workflow_run_revision_lines(revision: &WorkflowRunRevision) -> WorkflowRunRevisionRow {
    WorkflowRunRevisionRow {
        revision_id: revision.revision_id.0.clone(),
        workflow_execution_id: revision.workflow_execution_id.0.clone(),
        stage_count: revision.stages.len(),
        aggregate_status: revision.aggregate_status.as_ref().map(|s| format!("{:?}", s).to_lowercase()),
    }
}

pub fn workflow_lifecycle_event_lines(progression: &WorkflowStageProgression) -> WorkflowLifecycleEventRow {
    WorkflowLifecycleEventRow {
        event_id: progression.lifecycle_event.event_id.clone(),
        stage_id: progression.lifecycle_event.stage_id.clone(),
        event_kind: format!("{:?}", progression.lifecycle_event.event_kind).to_lowercase(),
        summary: progression.lifecycle_event.summary.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_action_outcome::WorkflowActionOutcomeStatus;
    use openwand_workflow::workflow_run::{WorkflowExecutionId, WorkflowStageLifecycleEvent, WorkflowStageLifecycleKind, WorkflowStageRun, WorkflowStageRunStatus};
    use openwand_workflow::workflow_action_route::WorkflowActionRouteId;
    use openwand_workflow::workflow_action_outcome::WorkflowActionOutcomeId;
    use openwand_workflow::workflow_proposal::WorkflowStageKind;
    use chrono::Utc;

    fn test_progression() -> WorkflowStageProgression {
        WorkflowStageProgression {
            stage_id: "s1".into(),
            previous_status: WorkflowStageRunStatus::Suspended,
            new_status: WorkflowStageRunStatus::Completed,
            outcome_status: WorkflowActionOutcomeStatus::ToolCompleted,
            lifecycle_event: WorkflowStageLifecycleEvent {
                event_id: "evt_1".into(), stage_id: "s1".into(),
                event_kind: WorkflowStageLifecycleKind::StageCompleted,
                summary: "Stage completed from session-produced tool outcome evidence.".into(),
                occurred_at: Utc::now(),
            },
            summary: "Stage completed from session-produced tool outcome evidence.".into(),
        }
    }

    fn test_record() -> WorkflowReconciliationRecord {
        WorkflowReconciliationRecord {
            reconciliation_id: WorkflowReconciliationId("wrc_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            route_id: WorkflowActionRouteId("war_t".into()),
            outcome_id: WorkflowActionOutcomeId("wao_t".into()),
            stage_id: "s1".into(), action_request_id: "ar_1".into(),
            status: WorkflowReconciliationStatus::Reconciled,
            decision: WorkflowReconciliationDecision::Reconciled { summary: "ok".into() },
            predicates: vec![WorkflowReconciliationPredicateResult {
                predicate: WorkflowReconciliationPredicate::WorkflowRunExists, passed: true, reason: "ok".into(),
            }],
            progression: Some(test_progression()),
            new_run_revision_id: Some(WorkflowRunRevisionId("wrr_t".into())),
            created_at: Utc::now(),
        }
    }

    #[test] fn ui_state_loads_latest_reconciliation() {
        let r = test_record();
        let state = WorkflowReconciliationUiState {
            latest_reconciliation: Some(workflow_reconciliation_summary(&r)),
            latest_run_revision: None,
            predicates: workflow_reconciliation_predicate_rows(&r),
            progression: r.progression.as_ref().map(workflow_stage_progression_lines),
            lifecycle_event: r.progression.as_ref().map(workflow_lifecycle_event_lines),
            warnings: vec![],
        };
        assert!(state.latest_reconciliation.is_some());
        assert_eq!("reconciled", state.latest_reconciliation.unwrap().status);
    }
    #[test] fn predicate_rows_show_pass_fail_reason() {
        let rows = workflow_reconciliation_predicate_rows(&test_record());
        assert!(!rows.is_empty()); assert!(rows[0].passed);
    }
    #[test] fn stage_progression_lines_show_old_new_status() {
        let row = workflow_stage_progression_lines(&test_progression());
        assert_eq!("suspended", row.previous_status);
        assert_eq!("completed", row.new_status);
    }
    #[test] fn run_revision_lines_show_revision_hash() {
        let revision = WorkflowRunRevision {
            revision_id: WorkflowRunRevisionId("wrr_abc".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            previous_revision_id: None,
            source_reconciliation_id: WorkflowReconciliationId("wrc_1".into()),
            run_hash_before: "h1".into(), run_hash_after: "h2".into(),
            stages: vec![WorkflowStageRun {
                stage_id: "s1".into(), title: "Stage 1".into(), kind: WorkflowStageKind::ApplyChange,
                status: WorkflowStageRunStatus::Completed, order: 0, depends_on: vec![],
                started_at: None, completed_at: None, summary: "done".into(),
            }],
            lifecycle_events: vec![], aggregate_status: Some(WorkflowStageRunStatus::Completed),
            created_at: Utc::now(),
        };
        let row = workflow_run_revision_lines(&revision);
        assert_eq!("wrr_abc", row.revision_id);
        assert_eq!(1, row.stage_count);
        assert_eq!(Some("completed".into()), row.aggregate_status);
    }
    #[test] fn safety_warning_mentions_no_execution_or_routing() {
        let w = workflow_reconciliation_safety_warning();
        // Warning is: "It does not route actions, resolve approvals, execute tools, append trace, or mutate session state."
        assert!(w.contains("route actions") && w.contains("does not"));
        assert!(w.contains("resolve approvals"));
        assert!(w.contains("execute tools"));
        assert!(w.contains("append trace"));
        assert!(w.contains("mutate session state"));
    }
}
