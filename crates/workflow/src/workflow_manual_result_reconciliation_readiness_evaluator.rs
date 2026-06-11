//! Manual result reconciliation readiness evaluator.
//!
//! Evaluates all readiness predicates for an accepted manual result review.
//! No LLM, no tool calls, no provider invocation, no shell/git, no reconciliation.


use crate::workflow_manual_result::WorkflowManualResult;
use crate::workflow_manual_result_review::WorkflowManualResultReview;
use crate::workflow_manual_result_reconciliation_readiness::*;

/// Context for readiness evaluation.
pub struct WorkflowManualResultReconciliationReadinessContext<'a> {
    pub manual_result: Option<&'a WorkflowManualResult>,
    pub manual_result_review: Option<&'a WorkflowManualResultReview>,
    // Patch 2: latest review for the same manual result
    pub latest_manual_result_review: Option<&'a WorkflowManualResultReview>,
    pub existing_readiness_records: Vec<WorkflowManualResultReconciliationReadinessRecord>,
}

fn pr(predicate: WorkflowManualResultReconciliationReadinessPredicate, passed: bool, reason: &str) -> WorkflowManualResultReconciliationReadinessPredicateResult {
    WorkflowManualResultReconciliationReadinessPredicateResult { predicate, passed, reason: reason.into() }
}

/// Evaluate reconciliation readiness deterministically.
pub fn evaluate_manual_result_reconciliation_readiness(
    request: &WorkflowManualResultReconciliationReadinessRequest,
    context: &WorkflowManualResultReconciliationReadinessContext,
) -> WorkflowManualResultReconciliationReadinessRecord {
    let mut predicates = Vec::new();

    // 1. ManualResultRecordExists
    let mr = context.manual_result;
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::ManualResultRecordExists,
        mr.is_some(), if mr.is_some() { "Found" } else { "Missing" }));

    // 2. ManualResultReviewExists
    let review = context.manual_result_review;
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::ManualResultReviewExists,
        review.is_some(), if review.is_some() { "Found" } else { "Missing" }));

    // 3. ReviewDecisionIsAccepted
    let accepted = review.is_some_and(|r| {
        matches!(r.decision, crate::workflow_manual_result_review::WorkflowManualResultReviewDecision::Accepted)
    });
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::ReviewDecisionIsAccepted,
        accepted, if accepted { "Accepted" } else { "Not accepted" }));

    // 4. ManualResultWasReportedByOperator
    let reported = mr.is_some_and(|m| m.reported_by_operator);
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::ManualResultWasReportedByOperator,
        reported, if reported { "Reported" } else { "Not reported" }));

    // 5. ManualResultNotVerifiedByOpenwand
    let not_verified = mr.is_some_and(|m| !m.verified_by_openwand);
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::ManualResultNotVerifiedByOpenwand,
        not_verified, if not_verified { "Not verified" } else { "Claims verified" }));

    // 6. ReviewAcceptsReportedEvidenceOnly
    let evidence_only = review.is_some_and(|r| r.acceptance_snapshot.accepts_reported_evidence
        && !r.acceptance_snapshot.verifies_external_state);
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::ReviewAcceptsReportedEvidenceOnly,
        evidence_only, if evidence_only { "Reported evidence only" } else { "Claims more than reported evidence" }));

    // Patch 1: 7-12. Hash matching predicates
    let review_hash_ok = review.is_some_and(|r| {
        let actual = blake3::hash(serde_json::to_string(r).unwrap_or_default().as_bytes()).to_hex().to_string();
        actual == request.expected_manual_result_review_hash
    });
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::ManualResultReviewHashMatchesRequest,
        review_hash_ok || review.is_none(), if review_hash_ok { "Match" } else { "Mismatch" }));

    let mr_hash_ok = mr.is_some_and(|m| {
        let actual = blake3::hash(serde_json::to_string(m).unwrap_or_default().as_bytes()).to_hex().to_string();
        actual == request.expected_manual_result_hash
    });
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::ManualResultHashMatchesRequest,
        mr_hash_ok || mr.is_none(), if mr_hash_ok { "Match" } else { "Mismatch" }));

    let cr_hash_ok = mr.is_some_and(|m| m.command_review_hash == request.expected_command_review_hash);
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::CommandReviewHashMatchesRequest,
        cr_hash_ok || mr.is_none(), if cr_hash_ok { "Match" } else { "Mismatch" }));

    let cc_hash_ok = mr.is_some_and(|m| m.command_composer_hash == request.expected_command_composer_hash);
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::CommandComposerHashMatchesRequest,
        cc_hash_ok || mr.is_none(), if cc_hash_ok { "Match" } else { "Mismatch" }));

    let cd_hash_ok = mr.is_some_and(|m| m.command_descriptor_hash == request.expected_command_descriptor_hash);
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::CommandDescriptorHashMatchesRequest,
        cd_hash_ok || mr.is_none(), if cd_hash_ok { "Match" } else { "Mismatch" }));

    let lc_hash_ok = mr.is_some_and(|m| m.loop_controller_hash == request.expected_loop_controller_hash);
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::LoopControllerHashMatchesRequest,
        lc_hash_ok || mr.is_none(), if lc_hash_ok { "Match" } else { "Mismatch" }));

    // Patch 2: 13. Review is latest for this manual result
    let is_latest = if let (Some(r), Some(latest)) = (review, context.latest_manual_result_review) {
        r.review_id == latest.review_id
    } else {
        review.is_some()
    };
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::ManualResultReviewIsLatest,
        is_latest, if is_latest { "Latest" } else { "Superseded" }));

    // 14. NoPriorConflictingReconciliationReadiness
    let no_conflict = !context.existing_readiness_records.iter().any(|er| {
        er.manual_result_review_id == request.manual_result_review_id
            && matches!(er.status, WorkflowManualResultReconciliationReadinessStatus::Ready)
            && er.readiness_id.0 != format!("wmrrr_{}", "")
    });
    predicates.push(pr(WorkflowManualResultReconciliationReadinessPredicate::NoPriorConflictingReconciliationReadiness,
        no_conflict, if no_conflict { "No conflict" } else { "Conflicting readiness exists" }));

    let all_pass = predicates.iter().all(|p| p.passed);

    // Compute hashes for record
    let review_hash = review.map_or(String::new(), |r| {
        blake3::hash(serde_json::to_string(r).unwrap_or_default().as_bytes()).to_hex().to_string()
    });
    let mr_hash = mr.map_or(String::new(), |m| {
        blake3::hash(serde_json::to_string(m).unwrap_or_default().as_bytes()).to_hex().to_string()
    });
    let cr_hash = mr.map_or(String::new(), |m| m.command_review_hash.clone());
    let cc_hash = mr.map_or(String::new(), |m| m.command_composer_hash.clone());
    let cd_hash = mr.map_or(String::new(), |m| m.command_descriptor_hash.clone());
    let lc_hash = mr.map_or(String::new(), |m| m.loop_controller_hash.clone());

    // Compute ID
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"reconciliation_readiness:v1:");
    hasher.update(request.workflow_execution_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.manual_result_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.manual_result_review_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    let readiness_id = WorkflowManualResultReconciliationReadinessId(format!("wmrrr_{}", &hex[..16]));

    // Patch 3: reconciliation preview based on manual result status
    let preview = mr.map(|m| match m.status {
        crate::workflow_manual_result::WorkflowManualResultStatus::ReportedSucceeded =>
            WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess,
        crate::workflow_manual_result::WorkflowManualResultStatus::ReportedFailed =>
            WorkflowManualResultReconciliationPreview::FailStageFromReportedFailure,
        crate::workflow_manual_result::WorkflowManualResultStatus::ReportedPartial =>
            WorkflowManualResultReconciliationPreview::PartialResultRequiresReview,
        crate::workflow_manual_result::WorkflowManualResultStatus::NotPerformed =>
            WorkflowManualResultReconciliationPreview::NotPerformedBlocksReconciliation,
        crate::workflow_manual_result::WorkflowManualResultStatus::Inconclusive =>
            WorkflowManualResultReconciliationPreview::InconclusiveBlocksReconciliation,
    });

    let (status, decision) = if all_pass {
        let preview_ref = preview.as_ref();
        match preview_ref {
            Some(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess) =>
                (WorkflowManualResultReconciliationReadinessStatus::Ready,
                 WorkflowManualResultReconciliationReadinessDecision::Ready { summary: "All predicates pass. Reported success with accepted review.".into() }),
            Some(WorkflowManualResultReconciliationPreview::FailStageFromReportedFailure) =>
                (WorkflowManualResultReconciliationReadinessStatus::Ready,
                 WorkflowManualResultReconciliationReadinessDecision::Ready { summary: "All predicates pass. Reported failure with accepted review.".into() }),
            Some(WorkflowManualResultReconciliationPreview::BlockStageFromReportedFailure) =>
                (WorkflowManualResultReconciliationReadinessStatus::Ready,
                 WorkflowManualResultReconciliationReadinessDecision::Ready { summary: "All predicates pass. Reported failure with accepted review — block stage.".into() }),
            Some(WorkflowManualResultReconciliationPreview::PartialResultRequiresReview) =>
                (WorkflowManualResultReconciliationReadinessStatus::Inconclusive,
                 WorkflowManualResultReconciliationReadinessDecision::Inconclusive { reason_code: "partial_result".into(), summary: "Partial result requires further review before reconciliation.".into() }),
            Some(WorkflowManualResultReconciliationPreview::NotPerformedBlocksReconciliation) =>
                (WorkflowManualResultReconciliationReadinessStatus::Blocked,
                 WorkflowManualResultReconciliationReadinessDecision::Blocked { reason_code: "not_performed".into(), summary: "Result not performed blocks reconciliation.".into() }),
            Some(WorkflowManualResultReconciliationPreview::InconclusiveBlocksReconciliation) =>
                (WorkflowManualResultReconciliationReadinessStatus::Inconclusive,
                 WorkflowManualResultReconciliationReadinessDecision::Inconclusive { reason_code: "inconclusive_result".into(), summary: "Inconclusive result blocks reconciliation.".into() }),
            None =>
                (WorkflowManualResultReconciliationReadinessStatus::Blocked,
                 WorkflowManualResultReconciliationReadinessDecision::Blocked { reason_code: "no_result".into(), summary: "No manual result record.".into() }),
        }
    } else {
        let failed: Vec<&str> = predicates.iter().filter(|p| !p.passed).map(|p| p.reason.as_str()).collect();
        (WorkflowManualResultReconciliationReadinessStatus::Blocked,
         WorkflowManualResultReconciliationReadinessDecision::Blocked { reason_code: "predicate_failed".into(), summary: format!("Failed: {}", failed.join(", ")) })
    };

    WorkflowManualResultReconciliationReadinessRecord {
        readiness_id,
        workflow_execution_id: request.workflow_execution_id.clone(),
        manual_result_id: request.manual_result_id.clone(),
        manual_result_review_id: request.manual_result_review_id.clone(),
        command_review_id: request.command_review_id.clone(),
        command_composer_id: request.command_composer_id.clone(),
        loop_controller_id: request.loop_controller_id.clone(),
        manual_result_review_hash: review_hash,
        manual_result_hash: mr_hash,
        command_review_hash: cr_hash,
        command_composer_hash: cc_hash,
        command_descriptor_hash: cd_hash,
        loop_controller_hash: lc_hash,
        status, decision, predicates, reconciliation_preview: preview,
        verifies_external_state: false, reconciles_now: false,
        mutates_workflow_state: false, creates_run_revision: false,
        appends_trace: false, writes_memory: false,
        routes_action: false, resolves_approval: false,
        creates_execution_grant: false, execution_allowed_now: false,
        evaluator: request.evaluator.clone(),
        evaluated_at: request.evaluated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_manual_result::*;
    use crate::workflow_manual_result_review::*;
    use crate::workflow_run::WorkflowExecutionId;
    use crate::workflow_command_review::WorkflowCommandReviewId;
    use crate::workflow_command_composer::WorkflowCommandComposerId;
    use crate::workflow_loop_controller::WorkflowLoopControllerId;

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

    fn valid_request(mr: &WorkflowManualResult, review: &WorkflowManualResultReview) -> WorkflowManualResultReconciliationReadinessRequest {
        let mr_hash = blake3::hash(serde_json::to_string(mr).unwrap().as_bytes()).to_hex().to_string();
        let review_hash = blake3::hash(serde_json::to_string(review).unwrap().as_bytes()).to_hex().to_string();
        WorkflowManualResultReconciliationReadinessRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            manual_result_review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            expected_manual_result_review_hash: review_hash,
            expected_manual_result_hash: mr_hash,
            expected_command_review_hash: "crh".into(),
            expected_command_composer_hash: "cch".into(),
            expected_command_descriptor_hash: "cdh".into(),
            expected_loop_controller_hash: "lch".into(),
            evaluator: "test".into(), evaluated_at: Utc::now(), idempotency_key: "k1".into(),
        }
    }

    #[test] fn reported_succeeded_with_accepted_review_is_ready() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: Some(&mr), manual_result_review: Some(&review),
            latest_manual_result_review: Some(&review), existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert_eq!(WorkflowManualResultReconciliationReadinessStatus::Ready, rec.status);
        assert!(matches!(rec.reconciliation_preview, Some(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess)));
    }

    #[test] fn reported_failed_with_accepted_review_is_ready() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedFailed);
        let review = test_review();
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: Some(&mr), manual_result_review: Some(&review),
            latest_manual_result_review: Some(&review), existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert_eq!(WorkflowManualResultReconciliationReadinessStatus::Ready, rec.status);
        assert!(matches!(rec.reconciliation_preview, Some(WorkflowManualResultReconciliationPreview::FailStageFromReportedFailure)));
    }

    #[test] fn reported_partial_is_inconclusive() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedPartial);
        let review = test_review();
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: Some(&mr), manual_result_review: Some(&review),
            latest_manual_result_review: Some(&review), existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert_eq!(WorkflowManualResultReconciliationReadinessStatus::Inconclusive, rec.status);
    }

    #[test] fn not_performed_is_blocked() {
        let mr = test_manual_result(WorkflowManualResultStatus::NotPerformed);
        let review = test_review();
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: Some(&mr), manual_result_review: Some(&review),
            latest_manual_result_review: Some(&review), existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert_eq!(WorkflowManualResultReconciliationReadinessStatus::Blocked, rec.status);
    }

    #[test] fn inconclusive_result_is_inconclusive() {
        let mr = test_manual_result(WorkflowManualResultStatus::Inconclusive);
        let review = test_review();
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: Some(&mr), manual_result_review: Some(&review),
            latest_manual_result_review: Some(&review), existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert_eq!(WorkflowManualResultReconciliationReadinessStatus::Inconclusive, rec.status);
    }

    #[test] fn blocks_missing_manual_result() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: None, manual_result_review: Some(&review),
            latest_manual_result_review: Some(&review), existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert_eq!(WorkflowManualResultReconciliationReadinessStatus::Blocked, rec.status);
    }

    #[test] fn blocks_missing_review() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: Some(&mr), manual_result_review: None,
            latest_manual_result_review: None, existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert_eq!(WorkflowManualResultReconciliationReadinessStatus::Blocked, rec.status);
    }

    #[test] fn blocks_rejected_review() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let mut review = test_review();
        review.decision = WorkflowManualResultReviewDecision::Rejected;
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: Some(&mr), manual_result_review: Some(&review),
            latest_manual_result_review: Some(&review), existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert_eq!(WorkflowManualResultReconciliationReadinessStatus::Blocked, rec.status);
    }

    // Patch 2: latest review tests
    #[test] fn blocks_non_latest_manual_result_review() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let mut later_review = test_review();
        later_review.review_id = WorkflowManualResultReviewId("wmrr_later".into());
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: Some(&mr), manual_result_review: Some(&review),
            latest_manual_result_review: Some(&later_review), existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert_eq!(WorkflowManualResultReconciliationReadinessStatus::Blocked, rec.status);
    }

    #[test] fn blocks_later_rejected_review_for_same_manual_result() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let mut later = test_review();
        later.review_id = WorkflowManualResultReviewId("wmrr_rej".into());
        later.decision = WorkflowManualResultReviewDecision::Rejected;
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: Some(&mr), manual_result_review: Some(&review),
            latest_manual_result_review: Some(&later), existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert_eq!(WorkflowManualResultReconciliationReadinessStatus::Blocked, rec.status);
    }

    #[test] fn blocks_later_changes_requested_review_for_same_manual_result() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let mut later = test_review();
        later.review_id = WorkflowManualResultReviewId("wmrr_chg".into());
        later.decision = WorkflowManualResultReviewDecision::ChangesRequested;
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: Some(&mr), manual_result_review: Some(&review),
            latest_manual_result_review: Some(&later), existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert_eq!(WorkflowManualResultReconciliationReadinessStatus::Blocked, rec.status);
    }

    #[test] fn readiness_does_not_reconcile_or_mutate() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: Some(&mr), manual_result_review: Some(&review),
            latest_manual_result_review: Some(&review), existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert!(!rec.verifies_external_state);
        assert!(!rec.reconciles_now);
        assert!(!rec.mutates_workflow_state);
        assert!(!rec.creates_run_revision);
    }

    #[test] fn readiness_id_starts_with_wmrrr() {
        let mr = test_manual_result(WorkflowManualResultStatus::ReportedSucceeded);
        let review = test_review();
        let req = valid_request(&mr, &review);
        let ctx = WorkflowManualResultReconciliationReadinessContext {
            manual_result: Some(&mr), manual_result_review: Some(&review),
            latest_manual_result_review: Some(&review), existing_readiness_records: vec![],
        };
        let rec = evaluate_manual_result_reconciliation_readiness(&req, &ctx);
        assert!(rec.readiness_id.0.starts_with("wmrrr_"));
    }
}
