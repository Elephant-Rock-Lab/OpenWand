//! Workflow run validation.

use crate::workflow_run::*;

/// Validate a workflow run record.
pub fn validate_workflow_run(record: &WorkflowRunRecord) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if record.predicates.is_empty() {
        errors.push("workflow run must have at least one predicate result".into());
    }

    match record.status {
        WorkflowRunStatus::Suspended | WorkflowRunStatus::Completed => {
            match &record.decision {
                WorkflowExecutionDecision::RunCreated => {
                    if record.stages.is_empty() {
                        errors.push("RunCreated decision requires at least one stage".into());
                    }
                }
                _ => {
                    errors.push(format!("{:?} status requires RunCreated decision", record.status));
                }
            }
        }
        WorkflowRunStatus::Blocked => {
            match &record.decision {
                WorkflowExecutionDecision::Blocked { reason_code, summary } => {
                    if reason_code.trim().is_empty() {
                        errors.push("Blocked decision requires reason_code".into());
                    }
                    if summary.trim().is_empty() {
                        errors.push("Blocked decision requires summary".into());
                    }
                }
                _ => {
                    errors.push("Blocked status requires Blocked decision".into());
                }
            }
        }
        WorkflowRunStatus::Failed => {
            match &record.decision {
                WorkflowExecutionDecision::Failed { reason_code, summary } => {
                    if reason_code.trim().is_empty() {
                        errors.push("Failed decision requires reason_code".into());
                    }
                    if summary.trim().is_empty() {
                        errors.push("Failed decision requires summary".into());
                    }
                }
                _ => {
                    errors.push("Failed status requires Failed decision".into());
                }
            }
        }
        WorkflowRunStatus::Running => {}
        WorkflowRunStatus::AlreadyExecuted => {}
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Compute content-addressed execution ID.
pub fn workflow_execution_id_for(
    readiness_id: &str,
    proposal_id: &str,
    idempotency_key: &str,
    stage_count: usize,
) -> WorkflowExecutionId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(readiness_id.as_bytes());
    hasher.update(proposal_id.as_bytes());
    hasher.update(idempotency_key.as_bytes());
    hasher.update(stage_count.to_le_bytes().as_slice());
    let hash = hasher.finalize();
    WorkflowExecutionId(format!("wfx_{}", hash.to_hex()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::TaskPlanId;
    use crate::workflow_proposal::WorkflowProposalId;
    use crate::workflow_proposal_review::WorkflowProposalReviewId;
    use crate::workflow_readiness::WorkflowReadinessId;
    use chrono::Utc;

    fn base_record() -> WorkflowRunRecord {
        WorkflowRunRecord {
            execution_id: WorkflowExecutionId("wfx_test".into()),
            readiness_id: WorkflowReadinessId("wfrd_test".into()),
            proposal_id: WorkflowProposalId("wfp_test".into()),
            proposal_review_id: WorkflowProposalReviewId("wfr_test".into()),
            source_task_plan_id: TaskPlanId("tpl_test".into()),
            status: WorkflowRunStatus::Suspended,
            decision: WorkflowExecutionDecision::RunCreated,
            predicates: vec![WorkflowExecutionPredicateResult {
                predicate: WorkflowExecutionPredicate::ReadinessRecordExists,
                passed: true,
                reason: "ok".into(),
            }],
            run_snapshot: WorkflowRunSnapshot {
                readiness_id: "wfrd_test".into(),
                proposal_id: "wfp_test".into(),
                proposal_hash: "phash".into(),
                source_task_plan_hash: "sphash".into(),
                readiness_status_at_execution: "ready".into(),
                proposal_review_decision_at_execution: "approved".into(),
            },
            stages: vec![WorkflowStageRun {
                stage_id: "stage_1".into(),
                title: "Observe".into(),
                kind: crate::workflow_proposal::WorkflowStageKind::Observe,
                status: WorkflowStageRunStatus::Completed,
                order: 0,
                depends_on: vec![],
                started_at: Some(Utc::now()),
                completed_at: Some(Utc::now()),
                summary: "Marked complete as non-tool deterministic stage".into(),
            }],
            lifecycle_events: vec![],
            action_requests: vec![],
            abort_snapshot: WorkflowAbortSnapshot {
                abort_notes_available: true,
                rollback_notes_available: true,
                recovery_notes: vec![],
            },
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    #[test]
    fn validate_run_created_accepts_valid() {
        assert!(validate_workflow_run(&base_record()).is_ok());
    }

    #[test]
    fn validate_run_created_requires_stages() {
        let mut record = base_record();
        record.stages = vec![];
        let errors = validate_workflow_run(&record).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("stage")));
    }

    #[test]
    fn validate_blocked_requires_reason() {
        let mut record = base_record();
        record.status = WorkflowRunStatus::Blocked;
        record.decision = WorkflowExecutionDecision::Blocked {
            reason_code: "test".into(),
            summary: "test blocked".into(),
        };
        assert!(validate_workflow_run(&record).is_ok());
    }

    #[test]
    fn validate_rejects_empty_predicates() {
        let mut record = base_record();
        record.predicates = vec![];
        let errors = validate_workflow_run(&record).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("predicate")));
    }

    #[test]
    fn execution_id_is_deterministic() {
        let id1 = workflow_execution_id_for("wfrd_a", "wfp_b", "key1", 3);
        let id2 = workflow_execution_id_for("wfrd_a", "wfp_b", "key1", 3);
        let id3 = workflow_execution_id_for("wfrd_a", "wfp_b", "key2", 3);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert!(id1.0.starts_with("wfx_"));
    }
}
