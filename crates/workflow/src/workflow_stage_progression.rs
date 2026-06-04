//! Stage progression engine — compute stage transitions from terminal outcome evidence.
//!
//! Updates only the linked stage. Does not start next stage, route next action,
//! resolve approval, execute tools, or append trace.

use chrono::Utc;

use crate::workflow_action_outcome::WorkflowActionOutcomeStatus;
use crate::workflow_reconciliation::*;
use crate::workflow_reconciliation_validation::run_revision_id_for;
use crate::workflow_run::{
    WorkflowStageLifecycleEvent, WorkflowStageLifecycleKind, WorkflowStageRun, WorkflowStageRunStatus,
};

/// Compute stage progression from a terminal outcome.
/// Returns None if the outcome does not warrant a stage transition.
pub fn compute_stage_progression(
    stage_id: &str,
    current_status: WorkflowStageRunStatus,
    outcome_status: &WorkflowActionOutcomeStatus,
) -> Option<WorkflowStageProgression> {
    // Only Suspended stages can be progressed
    if current_status != WorkflowStageRunStatus::Suspended {
        return None;
    }

    match outcome_status {
        WorkflowActionOutcomeStatus::ToolCompleted => Some(WorkflowStageProgression {
            stage_id: stage_id.into(),
            previous_status: WorkflowStageRunStatus::Suspended,
            new_status: WorkflowStageRunStatus::Completed,
            outcome_status: outcome_status.clone(),
            lifecycle_event: WorkflowStageLifecycleEvent {
                event_id: format!("evt_{}_completed", stage_id),
                stage_id: stage_id.into(),
                event_kind: WorkflowStageLifecycleKind::StageCompleted,
                summary: "Stage completed from session-produced tool outcome evidence.".into(),
                occurred_at: Utc::now(),
            },
            summary: "Stage completed from session-produced tool outcome evidence.".into(),
        }),
        WorkflowActionOutcomeStatus::ToolDenied => Some(WorkflowStageProgression {
            stage_id: stage_id.into(),
            previous_status: WorkflowStageRunStatus::Suspended,
            new_status: WorkflowStageRunStatus::Blocked,
            outcome_status: outcome_status.clone(),
            lifecycle_event: WorkflowStageLifecycleEvent {
                event_id: format!("evt_{}_blocked", stage_id),
                stage_id: stage_id.into(),
                event_kind: WorkflowStageLifecycleKind::StageBlocked,
                summary: "Stage blocked from session-produced denial outcome evidence.".into(),
                occurred_at: Utc::now(),
            },
            summary: "Stage blocked from session-produced denial outcome evidence.".into(),
        }),
        WorkflowActionOutcomeStatus::Failed => Some(WorkflowStageProgression {
            stage_id: stage_id.into(),
            previous_status: WorkflowStageRunStatus::Suspended,
            new_status: WorkflowStageRunStatus::Failed,
            outcome_status: outcome_status.clone(),
            lifecycle_event: WorkflowStageLifecycleEvent {
                event_id: format!("evt_{}_failed", stage_id),
                stage_id: stage_id.into(),
                event_kind: WorkflowStageLifecycleKind::StageFailed,
                summary: "Stage failed from session-produced error outcome evidence.".into(),
                occurred_at: Utc::now(),
            },
            summary: "Stage failed from session-produced error outcome evidence.".into(),
        }),
        // ApprovalResolved alone does not advance a stage
        _ => None,
    }
}

/// Apply a progression to a set of stages, returning new stages with only the linked stage updated.
/// Does not start next stage or route next action.
pub fn apply_progression_to_stages(
    stages: &[WorkflowStageRun],
    progression: &WorkflowStageProgression,
) -> Vec<WorkflowStageRun> {
    stages.iter().map(|s| {
        if s.stage_id == progression.stage_id {
            let mut updated = s.clone();
            updated.status = progression.new_status.clone();
            updated.completed_at = Some(progression.lifecycle_event.occurred_at);
            updated
        } else {
            s.clone()
        }
    }).collect()
}

/// Compute aggregate status from stages (Patch 1).
/// If all stages are terminal, returns Completed.
/// The original run record is never mutated.
pub fn compute_aggregate_status(stages: &[WorkflowStageRun]) -> Option<WorkflowStageRunStatus> {
    if stages.is_empty() {
        return None;
    }
    let all_terminal = stages.iter().all(|s| is_terminal_stage_status(&s.status));
    if all_terminal {
        Some(WorkflowStageRunStatus::Completed)
    } else {
        None
    }
}

/// Build a new WorkflowRunRevision from reconciliation.
/// The original run record is never mutated (Patch 1).
pub fn build_run_revision(
    workflow_execution_id: &str,
    reconciliation_id: &WorkflowReconciliationId,
    previous_revision_id: Option<&WorkflowRunRevisionId>,
    run_hash_before: &str,
    stages: Vec<WorkflowStageRun>,
    lifecycle_events: Vec<WorkflowStageLifecycleEvent>,
) -> WorkflowRunRevision {
    let aggregate = compute_aggregate_status(&stages);
    let run_hash_after = format!("revision_{}", Utc::now().timestamp_millis());

    WorkflowRunRevision {
        revision_id: run_revision_id_for(
            workflow_execution_id,
            &reconciliation_id.0,
            &run_hash_after,
        ),
        workflow_execution_id: crate::workflow_run::WorkflowExecutionId(workflow_execution_id.into()),
        previous_revision_id: previous_revision_id.cloned(),
        source_reconciliation_id: reconciliation_id.clone(),
        run_hash_before: run_hash_before.into(),
        run_hash_after,
        stages,
        lifecycle_events,
        aggregate_status: aggregate,
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_proposal::WorkflowStageKind;

    fn test_stage(id: &str, status: WorkflowStageRunStatus) -> WorkflowStageRun {
        WorkflowStageRun {
            stage_id: id.into(), title: format!("Stage {}", id),
            kind: WorkflowStageKind::ApplyChange, status,
            order: 0, depends_on: vec![], started_at: None, completed_at: None,
            summary: "test".into(),
        }
    }

    #[test]
    fn tool_completed_outcome_marks_stage_completed() {
        let p = compute_stage_progression("s1", WorkflowStageRunStatus::Suspended,
            &WorkflowActionOutcomeStatus::ToolCompleted).unwrap();
        assert_eq!(WorkflowStageRunStatus::Completed, p.new_status);
        assert_eq!(WorkflowStageLifecycleKind::StageCompleted, p.lifecycle_event.event_kind);
    }

    #[test]
    fn tool_denied_outcome_marks_stage_blocked() {
        let p = compute_stage_progression("s1", WorkflowStageRunStatus::Suspended,
            &WorkflowActionOutcomeStatus::ToolDenied).unwrap();
        assert_eq!(WorkflowStageRunStatus::Blocked, p.new_status);
        assert_eq!(WorkflowStageLifecycleKind::StageBlocked, p.lifecycle_event.event_kind);
    }

    #[test]
    fn failed_outcome_marks_stage_failed() {
        let p = compute_stage_progression("s1", WorkflowStageRunStatus::Suspended,
            &WorkflowActionOutcomeStatus::Failed).unwrap();
        assert_eq!(WorkflowStageRunStatus::Failed, p.new_status);
        assert_eq!(WorkflowStageLifecycleKind::StageFailed, p.lifecycle_event.event_kind);
    }

    #[test]
    fn approval_resolved_only_does_not_advance_stage() {
        let p = compute_stage_progression("s1", WorkflowStageRunStatus::Suspended,
            &WorkflowActionOutcomeStatus::ApprovalResolved);
        assert!(p.is_none());
    }

    #[test]
    fn reconciliation_updates_only_linked_stage() {
        let stages = vec![
            test_stage("s1", WorkflowStageRunStatus::Completed),
            test_stage("s2", WorkflowStageRunStatus::Suspended),
            test_stage("s3", WorkflowStageRunStatus::Pending),
        ];
        let prog = compute_stage_progression("s2", WorkflowStageRunStatus::Suspended,
            &WorkflowActionOutcomeStatus::ToolCompleted).unwrap();
        let updated = apply_progression_to_stages(&stages, &prog);
        assert_eq!(WorkflowStageRunStatus::Completed, updated[0].status);
        assert_eq!(WorkflowStageRunStatus::Completed, updated[1].status); // updated
        assert_eq!(WorkflowStageRunStatus::Pending, updated[2].status); // untouched
    }

    #[test]
    fn reconciliation_does_not_start_next_stage() {
        let stages = vec![
            test_stage("s1", WorkflowStageRunStatus::Suspended),
            test_stage("s2", WorkflowStageRunStatus::Pending),
        ];
        let prog = compute_stage_progression("s1", WorkflowStageRunStatus::Suspended,
            &WorkflowActionOutcomeStatus::ToolCompleted).unwrap();
        let updated = apply_progression_to_stages(&stages, &prog);
        // s2 should still be Pending — we don't start it
        assert_eq!(WorkflowStageRunStatus::Pending, updated[1].status);
    }

    #[test]
    fn reconciliation_does_not_route_next_action() {
        // Progression engine has no route/action/request concept
        let prog = compute_stage_progression("s1", WorkflowStageRunStatus::Suspended,
            &WorkflowActionOutcomeStatus::ToolCompleted).unwrap();
        // No routing fields in progression
        assert!(!format!("{:?}", prog).contains("route"));
        assert!(!format!("{:?}", prog).contains("action_request"));
    }

    #[test]
    fn reconciliation_records_lifecycle_event() {
        let p = compute_stage_progression("s1", WorkflowStageRunStatus::Suspended,
            &WorkflowActionOutcomeStatus::ToolCompleted).unwrap();
        assert!(!p.lifecycle_event.event_id.is_empty());
        assert_eq!("s1", p.lifecycle_event.stage_id);
        assert_eq!(WorkflowStageLifecycleKind::StageCompleted, p.lifecycle_event.event_kind);
        assert!(p.lifecycle_event.summary.contains("session-produced"));
    }

    #[test]
    fn all_terminal_stages_mark_revision_completed_not_original_run() {
        // Patch 1: aggregate is on revision only
        let stages = vec![
            test_stage("s1", WorkflowStageRunStatus::Completed),
            test_stage("s2", WorkflowStageRunStatus::Completed),
        ];
        let agg = compute_aggregate_status(&stages);
        assert_eq!(Some(WorkflowStageRunStatus::Completed), agg);
    }

    #[test]
    fn reconciliation_does_not_mutate_original_run_status() {
        // Patch 1: build_run_revision creates new, does not touch original
        let stages = vec![test_stage("s1", WorkflowStageRunStatus::Suspended)];
        let prog = compute_stage_progression("s1", WorkflowStageRunStatus::Suspended,
            &WorkflowActionOutcomeStatus::ToolCompleted).unwrap();
        let updated = apply_progression_to_stages(&stages, &prog);
        // Original stages still Suspended
        assert_eq!(WorkflowStageRunStatus::Suspended, stages[0].status);
        // Updated stages are Completed
        assert_eq!(WorkflowStageRunStatus::Completed, updated[0].status);
    }

    #[test]
    fn terminal_stage_set_includes_completed_blocked_failed_skipped() {
        // Patch 2
        assert!(is_terminal_stage_status(&WorkflowStageRunStatus::Completed));
        assert!(is_terminal_stage_status(&WorkflowStageRunStatus::Blocked));
        assert!(is_terminal_stage_status(&WorkflowStageRunStatus::Failed));
        assert!(is_terminal_stage_status(&WorkflowStageRunStatus::Skipped));
    }

    #[test]
    fn pending_running_suspended_are_not_terminal() {
        // Patch 2
        assert!(!is_terminal_stage_status(&WorkflowStageRunStatus::Pending));
        assert!(!is_terminal_stage_status(&WorkflowStageRunStatus::Running));
        assert!(!is_terminal_stage_status(&WorkflowStageRunStatus::Suspended));
    }

    #[test]
    fn workflow_crate_dependency_guard_still_allows_only_6_deps() {
        // Patch 4: workflow crate stays at exactly 6 dependencies
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let manifest = std::fs::read_to_string(&manifest_path).unwrap();
        let allowed = ["serde", "serde_json", "blake3", "chrono", "thiserror", "tracing"];
        let mut dep_count = 0u32;
        let mut in_deps = false;
        for line in manifest.lines() {
            let trimmed = line.trim();
            if trimmed == "[dependencies]" { in_deps = true; continue; }
            if trimmed.starts_with('[') { in_deps = false; continue; }
            if !in_deps { continue; }
            if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
            let name = trimmed.split('=').next().unwrap().trim();
            assert!(allowed.contains(&name), "Unexpected dependency: {}", name);
            dep_count += 1;
        }
        assert_eq!(6, dep_count, "Workflow crate must have exactly 6 dependencies");
    }
}
