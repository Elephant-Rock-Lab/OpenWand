//! Workflow readiness validation.
//!
//! Ready requires all predicates pass.
//! Blocked/Inconclusive require a reason.

use crate::workflow_readiness::*;

/// Validate a workflow readiness record.
pub fn validate_workflow_readiness(record: &WorkflowReadinessRecord) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if record.predicates.is_empty() {
        errors.push("readiness record must have at least one predicate result".into());
    }

    match record.status {
        WorkflowReadinessStatus::Ready => {
            // All predicates must pass
            let failed: Vec<_> = record
                .predicates
                .iter()
                .filter(|p| !p.passed)
                .collect();
            if !failed.is_empty() {
                let failed_names: Vec<String> = failed
                    .iter()
                    .map(|p| format!("{:?}", p.predicate))
                    .collect();
                errors.push(format!(
                    "Ready status requires all predicates pass, but these failed: {}",
                    failed_names.join(", ")
                ));
            }
        }
        WorkflowReadinessStatus::Blocked => {
            match &record.decision {
                WorkflowReadinessDecision::Blocked { reason_code, summary } => {
                    if reason_code.trim().is_empty() {
                        errors.push("Blocked decision requires a reason_code".into());
                    }
                    if summary.trim().is_empty() {
                        errors.push("Blocked decision requires a summary".into());
                    }
                }
                _ => {
                    errors.push("Blocked status requires Blocked decision".into());
                }
            }
        }
        WorkflowReadinessStatus::Inconclusive => {
            match &record.decision {
                WorkflowReadinessDecision::Inconclusive { reason_code, summary } => {
                    if reason_code.trim().is_empty() {
                        errors.push("Inconclusive decision requires a reason_code".into());
                    }
                    if summary.trim().is_empty() {
                        errors.push("Inconclusive decision requires a summary".into());
                    }
                }
                _ => {
                    errors.push("Inconclusive status requires Inconclusive decision".into());
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Compute content-addressed readiness ID.
pub fn workflow_readiness_id_for(
    proposal_id: &str,
    review_id: &str,
    idempotency_key: &str,
    predicate_count: usize,
) -> WorkflowReadinessId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(proposal_id.as_bytes());
    hasher.update(review_id.as_bytes());
    hasher.update(idempotency_key.as_bytes());
    hasher.update(predicate_count.to_le_bytes().as_slice());
    let hash = hasher.finalize();
    WorkflowReadinessId(format!("wfrd_{}", hash.to_hex()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::TaskPlanId;
    use crate::plan_review::TaskPlanReviewId;
    use crate::workflow_proposal::WorkflowProposalId;
    use crate::workflow_proposal_review::WorkflowProposalReviewId;
    use chrono::Utc;

    fn passing_predicates() -> Vec<WorkflowReadinessPredicateResult> {
        vec![WorkflowReadinessPredicateResult {
            predicate: WorkflowReadinessPredicate::ProposalExists,
            passed: true,
            reason: "ok".into(),
        }]
    }

    fn test_ready_record() -> WorkflowReadinessRecord {
        WorkflowReadinessRecord {
            readiness_id: WorkflowReadinessId("wfrd_test".into()),
            proposal_id: WorkflowProposalId("wfp_test".into()),
            review_id: WorkflowProposalReviewId("wfr_test".into()),
            source_task_plan_id: TaskPlanId("tpl_test".into()),
            source_task_plan_review_id: TaskPlanReviewId("tpr_test".into()),
            proposal_hash: "phash".into(),
            source_task_plan_hash: "sphash".into(),
            status: WorkflowReadinessStatus::Ready,
            decision: WorkflowReadinessDecision::Ready,
            predicates: passing_predicates(),
            tool_intents: vec![],
            approval_markers: vec![],
            environment: WorkflowEnvironmentSnapshot {
                workspace_observed: true,
                provider_config_available: true,
                session_runtime_available: true,
                tool_manifest_available: true,
                policy_context_available: true,
                notes: vec![],
            },
            rollback_abort: WorkflowRollbackAbortSnapshot {
                abort_notes_present: true,
                rollback_notes_present: true,
                unresolved_recovery_gaps: vec![],
            },
            created_at: Utc::now(),
        }
    }

    #[test]
    fn validate_ready_accepts_all_predicates_passed() {
        assert!(validate_workflow_readiness(&test_ready_record()).is_ok());
    }

    #[test]
    fn validate_ready_rejects_failed_predicate() {
        let mut record = test_ready_record();
        record.predicates.push(WorkflowReadinessPredicateResult {
            predicate: WorkflowReadinessPredicate::ProposalHashMatchesReview,
            passed: false,
            reason: "hash mismatch".into(),
        });
        let errors = validate_workflow_readiness(&record).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("failed")));
    }

    #[test]
    fn validate_blocked_requires_reason() {
        let mut record = test_ready_record();
        record.status = WorkflowReadinessStatus::Blocked;
        record.decision = WorkflowReadinessDecision::Blocked {
            reason_code: "hash_mismatch".into(),
            summary: "Proposal hash does not match review".into(),
        };
        assert!(validate_workflow_readiness(&record).is_ok());
    }

    #[test]
    fn validate_blocked_rejects_empty_reason() {
        let mut record = test_ready_record();
        record.status = WorkflowReadinessStatus::Blocked;
        record.decision = WorkflowReadinessDecision::Blocked {
            reason_code: "  ".into(),
            summary: "Has summary".into(),
        };
        let errors = validate_workflow_readiness(&record).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("reason_code")));
    }

    #[test]
    fn validate_inconclusive_requires_reason() {
        let mut record = test_ready_record();
        record.status = WorkflowReadinessStatus::Inconclusive;
        record.decision = WorkflowReadinessDecision::Inconclusive {
            reason_code: "provider_missing".into(),
            summary: "Provider not configured".into(),
        };
        // Predicates can be mixed for inconclusive
        record.predicates.push(WorkflowReadinessPredicateResult {
            predicate: WorkflowReadinessPredicate::ProviderConfigurationAvailable,
            passed: false,
            reason: "Provider not available".into(),
        });
        assert!(validate_workflow_readiness(&record).is_ok());
    }

    #[test]
    fn validate_rejects_empty_predicates() {
        let mut record = test_ready_record();
        record.predicates = vec![];
        let errors = validate_workflow_readiness(&record).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("predicate")));
    }

    #[test]
    fn readiness_id_is_deterministic() {
        let id1 = workflow_readiness_id_for("wfp_a", "wfr_b", "key1", 5);
        let id2 = workflow_readiness_id_for("wfp_a", "wfr_b", "key1", 5);
        let id3 = workflow_readiness_id_for("wfp_a", "wfr_b", "key2", 5);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert!(id1.0.starts_with("wfrd_"));
    }
}
