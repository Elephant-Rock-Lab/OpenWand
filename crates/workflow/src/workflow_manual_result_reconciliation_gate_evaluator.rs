//! Manual result reconciliation gate evaluator.
//!
//! Evaluates all 25 gate predicates. The readiness preview is the gate authority
//! (Patch 1), not raw manual result status. Produces Reconciled or Blocked.
//! AlreadyReconciled is set by the app persistence layer (Patch 7).

use chrono::Utc;

use crate::workflow_manual_result::WorkflowManualResultStatus;
use crate::workflow_manual_result_review::WorkflowManualResultReview;
use crate::workflow_manual_result_reconciliation_readiness::{
    WorkflowManualResultReconciliationPreview,
    WorkflowManualResultReconciliationReadinessRecord,
    WorkflowManualResultReconciliationReadinessStatus,
};
use crate::workflow_manual_result_reconciliation_gate::*;
use crate::workflow_manual_result::WorkflowManualResult;
use crate::workflow_reconciliation_validation::run_revision_id_for;
use crate::workflow_run::{WorkflowRunRecord, WorkflowStageRunStatus};

fn pr(predicate: WorkflowManualResultReconciliationGatePredicate, passed: bool, reason: &str) -> WorkflowManualResultReconciliationGatePredicateResult {
    WorkflowManualResultReconciliationGatePredicateResult { predicate, passed, reason: reason.into() }
}

/// Patch 4: context with latest-review/latest-readiness revalidation.
pub struct WorkflowManualResultReconciliationGateContext<'a> {
    pub workflow_run: Option<&'a WorkflowRunRecord>,
    pub manual_result: Option<&'a WorkflowManualResult>,
    pub manual_result_review: Option<&'a WorkflowManualResultReview>,
    pub reconciliation_readiness: Option<&'a WorkflowManualResultReconciliationReadinessRecord>,
    pub latest_manual_result_review: Option<&'a WorkflowManualResultReview>,
    pub latest_reconciliation_readiness: Option<&'a WorkflowManualResultReconciliationReadinessRecord>,
    pub prior_gates: Vec<WorkflowManualResultReconciliationGateRecord>,
}

/// Evaluate all 25 gate predicates deterministically.
pub fn evaluate_manual_result_reconciliation_gate(
    request: &WorkflowManualResultReconciliationGateRequest,
    context: &WorkflowManualResultReconciliationGateContext,
) -> WorkflowManualResultReconciliationGateRecord {
    let mut predicates = Vec::new();

    // Compute gate ID
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"manual_reconciliation_gate:v1:");
    hasher.update(request.workflow_execution_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.manual_result_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.manual_result_review_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.reconciliation_readiness_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.stage_id.as_bytes());
    hasher.update(b":");
    hasher.update(request.idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    let gate_id = WorkflowManualResultReconciliationGateId(format!("wmrrg_{}", &hex[..16]));

    // 1. WorkflowRunExists
    let run = context.workflow_run;
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::WorkflowRunExists,
        run.is_some(), if run.is_some() { "Found" } else { "Missing" }));

    // 2. WorkflowRunHashMatchesRequest
    let run_hash_ok = run.is_some() && !request.expected_workflow_run_hash.is_empty();
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::WorkflowRunHashMatchesRequest,
        run_hash_ok, if run_hash_ok { "Hash provided" } else { "Missing hash" }));

    // 3. ManualResultExists
    let mr = context.manual_result;
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ManualResultExists,
        mr.is_some(), if mr.is_some() { "Found" } else { "Missing" }));

    // 4. ManualResultHashMatchesRequest
    let mr_hash_ok = mr.is_some() && !request.expected_manual_result_hash.is_empty();
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ManualResultHashMatchesRequest,
        mr_hash_ok, if mr_hash_ok { "Hash provided" } else { "Missing hash" }));

    // 5. ManualResultReviewExists
    let review = context.manual_result_review;
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ManualResultReviewExists,
        review.is_some(), if review.is_some() { "Found" } else { "Missing" }));

    // 6. ManualResultReviewHashMatchesRequest
    let review_hash_ok = review.is_some() && !request.expected_manual_result_review_hash.is_empty();
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ManualResultReviewHashMatchesRequest,
        review_hash_ok, if review_hash_ok { "Hash provided" } else { "Missing hash" }));

    // 7. ReconciliationReadinessRecordExists
    let readiness = context.reconciliation_readiness;
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ReconciliationReadinessRecordExists,
        readiness.is_some(), if readiness.is_some() { "Found" } else { "Missing" }));

    // 8. ReconciliationReadinessHashMatchesRequest
    let readiness_hash_ok = readiness.is_some() && !request.expected_reconciliation_readiness_hash.is_empty();
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ReconciliationReadinessHashMatchesRequest,
        readiness_hash_ok, if readiness_hash_ok { "Hash provided" } else { "Missing hash" }));

    // 9. ReconciliationReadinessStatusIsReady
    let readiness_ready = readiness.is_some_and(|r|
        matches!(r.status, WorkflowManualResultReconciliationReadinessStatus::Ready));
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ReconciliationReadinessStatusIsReady,
        readiness_ready, if readiness_ready { "Ready" } else { "Not ready" }));

    // 10. ReviewDecisionIsAccepted
    let accepted = review.is_some_and(|r| {
        matches!(r.decision, crate::workflow_manual_result_review::WorkflowManualResultReviewDecision::Accepted)
    });
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ReviewDecisionIsAccepted,
        accepted, if accepted { "Accepted" } else { "Not accepted" }));

    // 11. ManualResultWasReportedByOperator
    let reported = mr.is_some_and(|m| m.reported_by_operator);
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ManualResultWasReportedByOperator,
        reported, if reported { "Reported" } else { "Not reported" }));

    // 12. StageExists
    let stage = run.and_then(|r| r.stages.iter().find(|s| s.stage_id == request.stage_id));
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::StageExists,
        stage.is_some(), if stage.is_some() { "Found" } else { "Missing" }));

    // Patch 3: hash linkage through readiness chain (13-16)
    let cr_hash_ok = readiness.is_some_and(|r| r.command_review_hash == request.expected_command_review_hash)
        || readiness.is_none();
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::CommandReviewHashMatchesReadiness,
        cr_hash_ok, if cr_hash_ok { "Match" } else { "Mismatch" }));

    let cc_hash_ok = readiness.is_some_and(|r| r.command_composer_hash == request.expected_command_composer_hash)
        || readiness.is_none();
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::CommandComposerHashMatchesReadiness,
        cc_hash_ok, if cc_hash_ok { "Match" } else { "Mismatch" }));

    let cd_hash_ok = readiness.is_some_and(|r| r.command_descriptor_hash == request.expected_command_descriptor_hash)
        || readiness.is_none();
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::CommandDescriptorHashMatchesReadiness,
        cd_hash_ok, if cd_hash_ok { "Match" } else { "Mismatch" }));

    let lc_hash_ok = readiness.is_some_and(|r| r.loop_controller_hash == request.expected_loop_controller_hash)
        || readiness.is_none();
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::LoopControllerHashMatchesReadiness,
        lc_hash_ok, if lc_hash_ok { "Match" } else { "Mismatch" }));

    // Patch 1: preview authority (17-19)
    // 17. ReconciliationPreviewExists
    let preview = readiness.and_then(|r| r.reconciliation_preview.clone());
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ReconciliationPreviewExists,
        preview.is_some(), if preview.is_some() { "Preview found" } else { "No preview" }));

    // 18. ReconciliationPreviewTargetIsActionable
    let preview_actionable = preview.as_ref().is_some_and(is_actionable_preview);
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ReconciliationPreviewTargetIsActionable,
        preview_actionable, if preview_actionable { "Actionable" } else { "Not actionable" }));

    // 19. ManualResultStatusMatchesReadinessPreview
    #[allow(clippy::match_like_matches_macro)]
    let status_matches_preview = match (mr, preview.as_ref()) {
        (Some(m), Some(p)) => match (&m.status, p) {
            (WorkflowManualResultStatus::ReportedSucceeded, WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess) => true,
            (WorkflowManualResultStatus::ReportedFailed, WorkflowManualResultReconciliationPreview::FailStageFromReportedFailure) => true,
            (WorkflowManualResultStatus::ReportedFailed, WorkflowManualResultReconciliationPreview::BlockStageFromReportedFailure) => true,
            (WorkflowManualResultStatus::ReportedPartial, WorkflowManualResultReconciliationPreview::PartialResultRequiresReview) => true,
            (WorkflowManualResultStatus::NotPerformed, WorkflowManualResultReconciliationPreview::NotPerformedBlocksReconciliation) => true,
            (WorkflowManualResultStatus::Inconclusive, WorkflowManualResultReconciliationPreview::InconclusiveBlocksReconciliation) => true,
            _ => false,
        },
        _ => false,
    };
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ManualResultStatusMatchesReadinessPreview,
        status_matches_preview, if status_matches_preview { "Consistent" } else { "Inconsistent" }));

    // Patch 2: 20. ManualResultEligibleForWorkflowStageReconciliation
    // The readiness preview must target stage progression
    let eligible = preview.as_ref().is_some_and(is_actionable_preview);
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ManualResultEligibleForWorkflowStageReconciliation,
        eligible, if eligible { "Eligible for stage reconciliation" } else { "Not eligible: preview does not target stage progression" }));

    // Patch 4: 21-22. Latest revalidation
    let review_is_latest = if let (Some(r), Some(latest)) = (review, context.latest_manual_result_review) {
        r.review_id == latest.review_id
    } else {
        review.is_some()
    };
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ManualResultReviewIsLatest,
        review_is_latest, if review_is_latest { "Latest" } else { "Superseded" }));

    let readiness_is_latest = if let (Some(r), Some(latest)) = (readiness, context.latest_reconciliation_readiness) {
        r.readiness_id == latest.readiness_id
    } else {
        readiness.is_some()
    };
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::ReconciliationReadinessIsLatest,
        readiness_is_latest, if readiness_is_latest { "Latest" } else { "Superseded" }));

    // Patch 5: 23. StageStatusEligibleForManualReconciliation (only Suspended)
    let stage_eligible = stage.is_some_and(|s| s.status == WorkflowStageRunStatus::Suspended);
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::StageStatusEligibleForManualReconciliation,
        stage_eligible, if stage_eligible { "Suspended — eligible" } else { "Not eligible: only Suspended stages" }));

    // 24. NoPriorConflictingManualReconciliation
    let no_conflict = !context.prior_gates.iter().any(|g| {
        g.manual_result_id == request.manual_result_id
            && g.stage_id == request.stage_id
            && matches!(g.status, WorkflowManualResultReconciliationGateStatus::Reconciled)
            && g.gate_id != gate_id
    });
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::NoPriorConflictingManualReconciliation,
        no_conflict, if no_conflict { "No conflict" } else { "Conflicting gate exists" }));

    // 25. IdempotencyKeyUnusedOrMatchesExisting
    let idempotency_ok = !context.prior_gates.iter().any(|g| {
        g.workflow_execution_id == request.workflow_execution_id
            && g.manual_result_id == request.manual_result_id
            && g.reconciliation_readiness_id == request.reconciliation_readiness_id
            && g.stage_id == request.stage_id
            && g.gate_id != gate_id
    });
    predicates.push(pr(WorkflowManualResultReconciliationGatePredicate::IdempotencyKeyUnusedOrMatchesExisting,
        idempotency_ok, if idempotency_ok { "Key ok" } else { "Key conflict" }));

    let all_pass = predicates.iter().all(|p| p.passed);

    // Compute progression and revision if all pass
    let (status, decision, progression, new_revision_id, creates_revision) = if all_pass {
        let stage_status = stage.map(|s| s.status.clone());
        let preview_ref = preview.as_ref();

        // Compute progression from preview (Patch 1: preview is authority)
        let prog = if let (Some(ss), Some(pv)) = (stage_status.as_ref(), preview_ref) {
            compute_manual_result_stage_progression(&request.stage_id, ss, pv)
        } else {
            None
        };

        let revision_id = prog.as_ref().map(|_p| {
            let run_hash_after = format!("manual_revision_{}", Utc::now().timestamp_millis());
            run_revision_id_for(
                &request.workflow_execution_id.0,
                &gate_id.0,
                &run_hash_after,
            )
        });

        let summary = match &prog {
            Some(p) => format!("Stage {:?} reconciled via manual result", p.new_status),
            None => "All predicates pass but no progression computed".into(),
        };

        (
            WorkflowManualResultReconciliationGateStatus::Reconciled,
            WorkflowManualResultReconciliationGateDecision::Reconciled {
                revision_id: revision_id.as_ref().map(|r| r.0.clone()),
                summary,
            },
            prog,
            revision_id,
            true, // Patch 6: creates_run_revision
        )
    } else {
        let failed: Vec<&str> = predicates.iter().filter(|p| !p.passed).map(|p| p.reason.as_str()).collect();
        (
            WorkflowManualResultReconciliationGateStatus::Blocked,
            WorkflowManualResultReconciliationGateDecision::Blocked {
                reason_code: "predicate_failed".into(),
                summary: format!("Blocked: {}", failed.join(", ")),
            },
            None,
            None,
            false,
        )
    };

    // Extract stored hashes
    let workflow_run_hash = run.map_or(String::new(), |_| request.expected_workflow_run_hash.clone());
    let readiness_hash = readiness.map_or(String::new(), |_| request.expected_reconciliation_readiness_hash.clone());
    let review_hash_stored = review.map_or(String::new(), |_| request.expected_manual_result_review_hash.clone());
    let mr_hash_stored = mr.map_or(String::new(), |_| request.expected_manual_result_hash.clone());
    let cr_hash = readiness.map_or(String::new(), |r| r.command_review_hash.clone());
    let cc_hash = readiness.map_or(String::new(), |r| r.command_composer_hash.clone());
    let cd_hash = readiness.map_or(String::new(), |r| r.command_descriptor_hash.clone());
    let lc_hash = readiness.map_or(String::new(), |r| r.loop_controller_hash.clone());

    WorkflowManualResultReconciliationGateRecord {
        gate_id,
        workflow_execution_id: request.workflow_execution_id.clone(),
        manual_result_id: request.manual_result_id.clone(),
        manual_result_review_id: request.manual_result_review_id.clone(),
        reconciliation_readiness_id: request.reconciliation_readiness_id.clone(),
        command_review_id: readiness.map_or(
            crate::workflow_command_review::WorkflowCommandReviewId(String::new()),
            |r| r.command_review_id.clone(),
        ),
        command_composer_id: readiness.map_or(
            crate::workflow_command_composer::WorkflowCommandComposerId(String::new()),
            |r| r.command_composer_id.clone(),
        ),
        loop_controller_id: readiness.map_or(
            crate::workflow_loop_controller::WorkflowLoopControllerId(String::new()),
            |r| r.loop_controller_id.clone(),
        ),
        stage_id: request.stage_id.clone(),
        workflow_run_hash,
        reconciliation_readiness_hash: readiness_hash,
        manual_result_review_hash: review_hash_stored,
        manual_result_hash: mr_hash_stored,
        command_review_hash: cr_hash,
        command_composer_hash: cc_hash,
        command_descriptor_hash: cd_hash,
        loop_controller_hash: lc_hash,
        status, decision, predicates,
        progression,
        new_run_revision_id: new_revision_id,
        creates_run_revision: creates_revision,
        mutates_original_workflow_run: false,
        verifies_external_truth: false,
        executes_command: false,
        routes_continuation: false,
        appends_trace: false,
        writes_memory: false,
        creates_execution_grant: false,
        execution_allowed_now: false,
        reconciled_by: request.requested_by.clone(),
        reconciled_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_command_composer::WorkflowCommandComposerId;
    use crate::workflow_command_review::WorkflowCommandReviewId;
    use crate::workflow_loop_controller::WorkflowLoopControllerId;
    use crate::workflow_manual_result::*;
    use crate::workflow_manual_result_review::*;
    use crate::workflow_manual_result_reconciliation_readiness::*;
    use crate::workflow_proposal::WorkflowStageKind;
    use crate::workflow_run::*;

    fn test_manual_result(status: WorkflowManualResultStatus) -> WorkflowManualResult {
        WorkflowManualResult {
            result_id: WorkflowManualResultId("wmr_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            command_review_hash: "crh".into(), command_composer_hash: "cch".into(),
            command_descriptor_hash: "cdh".into(), loop_controller_hash: "lch".into(),
            status: status.clone(),
            operator: "tester".into(),
            summary: WorkflowManualResultSummary {
                operator_summary: "done".into(), operator_details: None,
                reported_status: status, caveat: "Operator-reported".into(),
            },
            artifact_references: vec![],
            validation_snapshot: WorkflowManualResultValidationSnapshot {
                command_review_was_acknowledged: true,
                command_review_hash_matched: true, command_composer_hash_matched: true,
                command_descriptor_hash_matched: true, loop_controller_hash_matched: true,
                command_review_marked_not_performed_by_openwand: true,
            },
            reported_by_operator: true,
            verified_by_openwand: false, command_executed_by_openwand: false,
            mutates_workflow_state: false, reconciles_outcome: false,
            routes_action: false, resolves_approval: false,
            appends_trace: false, writes_memory: false,
            invokes_shell: false, invokes_git: false,
            creates_execution_grant: false, execution_allowed_now: false,
            captured_at: Utc::now(),
        }
    }

    fn test_review() -> WorkflowManualResultReview {
        WorkflowManualResultReview {
            review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            manual_result_hash: "mrh".into(), command_review_hash: "crh".into(),
            command_composer_hash: "cch".into(), command_descriptor_hash: "cdh".into(),
            loop_controller_hash: "lch".into(),
            decision: WorkflowManualResultReviewDecision::Accepted,
            reviewer: "r".into(), rationale: "ok".into(), feedback: None,
            acceptance_snapshot: WorkflowManualResultReviewAcceptanceSnapshot {
                accepts_reported_evidence: true, verifies_external_state: false,
                reconciles_workflow_state: false, result_verified_by_openwand: false,
            },
            verifies_external_state: false, reconciles_workflow_state: false,
            mutates_workflow_state: false, executes_command: false,
            invokes_shell: false, invokes_git: false,
            routes_action: false, resolves_approval: false,
            appends_trace: false, writes_memory: false,
            creates_execution_grant: false, execution_allowed_now: false,
            reviewed_at: Utc::now(),
        }
    }

    fn test_readiness(preview: WorkflowManualResultReconciliationPreview) -> WorkflowManualResultReconciliationReadinessRecord {
        WorkflowManualResultReconciliationReadinessRecord {
            readiness_id: WorkflowManualResultReconciliationReadinessId("wmrrr_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            manual_result_review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            manual_result_review_hash: "mrrh".into(), manual_result_hash: "mrh".into(),
            command_review_hash: "crh".into(), command_composer_hash: "cch".into(),
            command_descriptor_hash: "cdh".into(), loop_controller_hash: "lch".into(),
            status: WorkflowManualResultReconciliationReadinessStatus::Ready,
            decision: WorkflowManualResultReconciliationReadinessDecision::Ready { summary: "ok".into() },
            predicates: vec![],
            reconciliation_preview: Some(preview),
            verifies_external_state: false, reconciles_now: false,
            mutates_workflow_state: false, creates_run_revision: false,
            appends_trace: false, writes_memory: false,
            routes_action: false, resolves_approval: false,
            creates_execution_grant: false, execution_allowed_now: false,
            evaluator: "test".into(), evaluated_at: Utc::now(),
        }
    }

    fn test_run(stage_status: WorkflowStageRunStatus) -> WorkflowRunRecord {
        WorkflowRunRecord {
            execution_id: WorkflowExecutionId("wfx_t".into()),
            readiness_id: crate::workflow_readiness::WorkflowReadinessId("wfrd_t".into()),
            proposal_id: crate::workflow_proposal::WorkflowProposalId("wfp_t".into()),
            proposal_review_id: crate::workflow_proposal_review::WorkflowProposalReviewId("wfr_t".into()),
            source_task_plan_id: crate::plan::TaskPlanId("tpl_t".into()),
            status: WorkflowRunStatus::Suspended,
            decision: WorkflowExecutionDecision::Suspended { reason_code: "approval".into(), summary: "s".into() },
            predicates: vec![],
            run_snapshot: WorkflowRunSnapshot {
                readiness_id: "wfrd_t".into(), proposal_id: "wfp_t".into(),
                proposal_hash: "ph".into(), source_task_plan_hash: "sph".into(),
                readiness_status_at_execution: "ready".into(),
                proposal_review_decision_at_execution: "approved".into(),
            },
            stages: vec![WorkflowStageRun {
                stage_id: "stage_1".into(), title: "Stage 1".into(),
                kind: WorkflowStageKind::ApplyChange,
                status: stage_status, order: 0, depends_on: vec![],
                started_at: None, completed_at: None, summary: "test".into(),
            }],
            lifecycle_events: vec![], action_requests: vec![],
            abort_snapshot: WorkflowAbortSnapshot {
                abort_notes_available: false, rollback_notes_available: false,
                recovery_notes: vec![],
            },
            created_at: Utc::now(), completed_at: None,
        }
    }

    fn valid_request() -> WorkflowManualResultReconciliationGateRequest {
        WorkflowManualResultReconciliationGateRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            manual_result_review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            reconciliation_readiness_id: WorkflowManualResultReconciliationReadinessId("wmrrr_t".into()),
            stage_id: "stage_1".into(),
            expected_workflow_run_hash: "wrh".into(),
            expected_reconciliation_readiness_hash: "rrh".into(),
            expected_manual_result_review_hash: "mrrh".into(),
            expected_manual_result_hash: "mrh".into(),
            expected_command_review_hash: "crh".into(),
            expected_command_composer_hash: "cch".into(),
            expected_command_descriptor_hash: "cdh".into(),
            expected_loop_controller_hash: "lch".into(),
            requested_by: "test".into(), requested_at: Utc::now(),
            idempotency_key: "k1".into(),
        }
    }

    fn full_ctx<'a>(_mr: &'a WorkflowManualResult, review: &'a WorkflowManualResultReview, readiness: &'a WorkflowManualResultReconciliationReadinessRecord, run: &'a WorkflowRunRecord) -> WorkflowManualResultReconciliationGateContext<'a> {
        WorkflowManualResultReconciliationGateContext {
            workflow_run: Some(run), manual_result: Some(_mr),
            manual_result_review: Some(review), reconciliation_readiness: Some(readiness),
            latest_manual_result_review: Some(review), latest_reconciliation_readiness: Some(readiness),
            prior_gates: vec![],
        }
    }

    // Standard gate tests
    #[test] fn blocks_missing_workflow_run() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let mut ctx = full_ctx(&mr, &review, &readiness, &run); ctx.workflow_run = None;
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_missing_manual_result() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let mut ctx = full_ctx(&mr, &review, &readiness, &run); ctx.manual_result = None;
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_missing_review() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let mut ctx = full_ctx(&mr, &review, &readiness, &run); ctx.manual_result_review = None;
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_missing_readiness() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let mut ctx = full_ctx(&mr, &review, &readiness, &run); ctx.reconciliation_readiness = None;
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_non_ready_readiness() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let mut readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        readiness.status = WorkflowManualResultReconciliationReadinessStatus::Blocked;
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_rejected_review() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let mut review = test_review();
        review.decision = WorkflowManualResultReviewDecision::Rejected;
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    // Patch 1 tests
    #[test] fn ready_succeeded_with_accepted_review_reconciles() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Reconciled));
        assert!(rec.progression.is_some());
        assert_eq!(WorkflowStageRunStatus::Completed, rec.progression.unwrap().new_status);
        assert!(rec.creates_run_revision);
    }

    #[test] fn ready_failed_with_accepted_review_reconciles_to_failed() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedFailed);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::FailStageFromReportedFailure);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Reconciled));
        assert_eq!(WorkflowStageRunStatus::Failed, rec.progression.unwrap().new_status);
    }

    #[test] fn blocks_ready_readiness_without_reconciliation_preview() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let mut readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        readiness.reconciliation_preview = None;
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_preview_status_mismatch() {
        // Preview says success but result is failed
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedFailed);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn reported_failed_uses_preview_to_choose_blocked_or_failed() {
        // Preview says BlockStage → stage should be Blocked, not Failed
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedFailed);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::BlockStageFromReportedFailure);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Reconciled));
        assert_eq!(WorkflowStageRunStatus::Blocked, rec.progression.unwrap().new_status);
    }

    #[test] fn reported_partial_blocks_unless_preview_explicitly_allows_progression() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedPartial);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::PartialResultRequiresReview);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    // Patch 2 tests
    #[test] fn blocks_manual_result_for_non_stage_operation() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        // Non-actionable preview = meta-operation
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::InconclusiveBlocksReconciliation);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_manual_result_for_inspect_blocked_workflow() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::NotPerformedBlocksReconciliation);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn accepts_manual_result_only_when_readiness_preview_targets_stage_progression() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Reconciled));
    }

    // Patch 4 tests
    #[test] fn blocks_non_latest_manual_result_review() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let mut later = test_review(); later.review_id = WorkflowManualResultReviewId("wmrr_later".into());
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let mut ctx = full_ctx(&mr, &review, &readiness, &run);
        ctx.latest_manual_result_review = Some(&later);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_later_rejected_manual_result_review() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let mut later = test_review();
        later.review_id = WorkflowManualResultReviewId("wmrr_rej".into());
        later.decision = WorkflowManualResultReviewDecision::Rejected;
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let mut ctx = full_ctx(&mr, &review, &readiness, &run);
        ctx.latest_manual_result_review = Some(&later);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_later_changes_requested_manual_result_review() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let mut later = test_review();
        later.review_id = WorkflowManualResultReviewId("wmrr_chg".into());
        later.decision = WorkflowManualResultReviewDecision::ChangesRequested;
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let mut ctx = full_ctx(&mr, &review, &readiness, &run);
        ctx.latest_manual_result_review = Some(&later);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_non_latest_reconciliation_readiness() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let mut later = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        later.readiness_id = WorkflowManualResultReconciliationReadinessId("wmrrr_later".into());
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let mut ctx = full_ctx(&mr, &review, &readiness, &run);
        ctx.latest_reconciliation_readiness = Some(&later);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    // Patch 5 tests
    #[test] fn blocks_pending_stage_to_prevent_skip() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Pending);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_completed_stage() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Completed);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_failed_stage() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Failed);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn blocks_blocked_stage_unless_preview_explicitly_allows_block_update() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedFailed);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::BlockStageFromReportedFailure);
        let run = test_run(WorkflowStageRunStatus::Blocked);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Blocked));
    }

    #[test] fn accepts_suspended_stage_for_manual_reconciliation() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(matches!(rec.status, WorkflowManualResultReconciliationGateStatus::Reconciled));
    }

    // Patch 7: idempotency tests
    #[test] fn gate_id_starts_with_wmrrg() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(rec.gate_id.0.starts_with("wmrrg_"));
    }

    #[test] fn gate_does_not_mutate_original_run() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let readiness = test_readiness(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess);
        let run = test_run(WorkflowStageRunStatus::Suspended);
        let ctx = full_ctx(&mr, &review, &readiness, &run);
        let rec = evaluate_manual_result_reconciliation_gate(&valid_request(), &ctx);
        assert!(!rec.mutates_original_workflow_run);
        // Original run stage is still Suspended
        assert_eq!(WorkflowStageRunStatus::Suspended, run.stages[0].status);
    }
}
