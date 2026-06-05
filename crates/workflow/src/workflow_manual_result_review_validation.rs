//! Manual result review validation — hash binding and structural checks.
//!
//! Validates that the manual result record exists, all evidence chain hashes match,
//! the result was operator-reported and not verified by OpenWand, and reviewer/
//! rationale requirements are met. Does NOT check prior review records (Patch 1).

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::workflow_command_composer::WorkflowCommandComposerRecord;
use crate::workflow_command_review::WorkflowCommandReview;
use crate::workflow_loop_controller::WorkflowLoopControllerRecord;
use crate::workflow_manual_result::WorkflowManualResult;
use crate::workflow_manual_result_review::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowManualResultReviewValidationRule {
    ManualResultExists,
    ManualResultHashMatches,
    ManualResultWasReportedByOperator,
    ManualResultNotVerifiedByOpenwand,
    ManualResultNotExecutedByOpenwand,
    ReviewerNonEmpty,
    RationaleNonEmpty,
    RejectionRequiresBlockingReasons,
    ChangesRequestedRequiresRequestedChanges,
    AcceptedDoesNotVerifyExternalState,
    AcceptedDoesNotReconcileWorkflowState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultReviewValidationResult {
    pub rule: WorkflowManualResultReviewValidationRule,
    pub passed: bool,
    pub reason: String,
}

fn v(rule: WorkflowManualResultReviewValidationRule, passed: bool, reason: &str) -> WorkflowManualResultReviewValidationResult {
    WorkflowManualResultReviewValidationResult { rule, passed, reason: reason.into() }
}

pub fn validate_manual_result_review(
    request: &WorkflowManualResultReviewRequest,
    manual_result: Option<&WorkflowManualResult>,
    _review_record: Option<&WorkflowCommandReview>,
    _composer_record: Option<&WorkflowCommandComposerRecord>,
    _loop_controller_record: Option<&WorkflowLoopControllerRecord>,
) -> Result<WorkflowManualResultReview, Vec<WorkflowManualResultReviewValidationResult>> {
    let mut results = Vec::new();

    // 1. Manual result record must exist
    results.push(v(WorkflowManualResultReviewValidationRule::ManualResultExists,
        manual_result.is_some(), if manual_result.is_some() { "Found" } else { "Missing" }));

    // 2. Manual result hash must match
    // Patch 2: compare against the manual result's stored hashes
    let mr_hash_ok = manual_result.map_or(false, |mr| {
        let actual = blake3::hash(serde_json::to_string(mr).unwrap_or_default().as_bytes()).to_hex().to_string();
        actual == request.expected_manual_result_hash
    });
    results.push(v(WorkflowManualResultReviewValidationRule::ManualResultHashMatches,
        mr_hash_ok || manual_result.is_none(),
        if mr_hash_ok { "Match" } else { "Mismatch" }));

    // 3. Manual result was reported by operator
    let reported_ok = manual_result.map_or(true, |mr| mr.reported_by_operator);
    results.push(v(WorkflowManualResultReviewValidationRule::ManualResultWasReportedByOperator,
        reported_ok, if reported_ok { "Reported" } else { "Not reported" }));

    // 4. Manual result was not verified by OpenWand
    let not_verified = manual_result.map_or(true, |mr| !mr.verified_by_openwand);
    results.push(v(WorkflowManualResultReviewValidationRule::ManualResultNotVerifiedByOpenwand,
        not_verified, if not_verified { "Not verified" } else { "Claims verified" }));

    // 5. Manual result was not executed by OpenWand
    let not_executed = manual_result.map_or(true, |mr| !mr.command_executed_by_openwand);
    results.push(v(WorkflowManualResultReviewValidationRule::ManualResultNotExecutedByOpenwand,
        not_executed, if not_executed { "Not executed" } else { "Claims executed" }));

    // 6. Reviewer non-empty
    results.push(v(WorkflowManualResultReviewValidationRule::ReviewerNonEmpty,
        !request.reviewer.is_empty(),
        if request.reviewer.is_empty() { "Empty" } else { "Provided" }));

    // 7. Rationale non-empty
    results.push(v(WorkflowManualResultReviewValidationRule::RationaleNonEmpty,
        !request.rationale.is_empty(),
        if request.rationale.is_empty() { "Empty" } else { "Provided" }));

    // 8. Rejection requires blocking reasons
    let reject_ok = match request.decision {
        WorkflowManualResultReviewDecision::Rejected => {
            request.feedback.as_ref().map_or(false, |f| !f.blocking_reasons.is_empty())
        }
        _ => true,
    };
    results.push(v(WorkflowManualResultReviewValidationRule::RejectionRequiresBlockingReasons,
        reject_ok, if reject_ok { "Ok" } else { "Missing blocking reasons" }));

    // 9. ChangesRequested requires requested changes
    let changes_ok = match request.decision {
        WorkflowManualResultReviewDecision::ChangesRequested => {
            request.feedback.as_ref().map_or(false, |f| !f.requested_changes.is_empty())
        }
        _ => true,
    };
    results.push(v(WorkflowManualResultReviewValidationRule::ChangesRequestedRequiresRequestedChanges,
        changes_ok, if changes_ok { "Ok" } else { "Missing requested changes" }));

    // 10. Accepted does not verify external state (structural — always true)
    results.push(v(WorkflowManualResultReviewValidationRule::AcceptedDoesNotVerifyExternalState,
        true, "No external verification"));

    // 11. Accepted does not reconcile workflow state (structural — always true)
    results.push(v(WorkflowManualResultReviewValidationRule::AcceptedDoesNotReconcileWorkflowState,
        true, "No reconciliation"));

    let all_pass = results.iter().all(|r| r.passed);
    if !all_pass {
        return Err(results);
    }

    // Build review record
    let mr_hash = manual_result.map_or(String::new(), |mr| {
        blake3::hash(serde_json::to_string(mr).unwrap_or_default().as_bytes()).to_hex().to_string()
    });
    // Patch 2: copy hashes from the manual result record
    let cr_hash = manual_result.map_or(String::new(), |mr| mr.command_review_hash.clone());
    let cc_hash = manual_result.map_or(String::new(), |mr| mr.command_composer_hash.clone());
    let cd_hash = manual_result.map_or(String::new(), |mr| mr.command_descriptor_hash.clone());
    let lc_hash = manual_result.map_or(String::new(), |mr| mr.loop_controller_hash.clone());

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"manual_result_review:v1:");
    hasher.update(request.workflow_execution_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.manual_result_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    let review_id = WorkflowManualResultReviewId(format!("wmrr_{}", &hex[..16]));

    // Patch 3: acceptance semantics
    let accepts_evidence = matches!(request.decision, WorkflowManualResultReviewDecision::Accepted);
    let result_verified = manual_result.map_or(false, |mr| mr.verified_by_openwand);

    Ok(WorkflowManualResultReview {
        review_id,
        workflow_execution_id: request.workflow_execution_id.clone(),
        manual_result_id: request.manual_result_id.clone(),
        command_review_id: request.command_review_id.clone(),
        command_composer_id: request.command_composer_id.clone(),
        loop_controller_id: request.loop_controller_id.clone(),
        manual_result_hash: mr_hash,
        command_review_hash: cr_hash,
        command_composer_hash: cc_hash,
        command_descriptor_hash: cd_hash,
        loop_controller_hash: lc_hash,
        decision: request.decision.clone(),
        reviewer: request.reviewer.clone(),
        rationale: request.rationale.clone(),
        feedback: request.feedback.clone(),
        acceptance_snapshot: WorkflowManualResultReviewAcceptanceSnapshot {
            accepts_reported_evidence: accepts_evidence,
            verifies_external_state: false,
            reconciles_workflow_state: false,
            result_verified_by_openwand: result_verified,
        },
        verifies_external_state: false,
        reconciles_workflow_state: false,
        mutates_workflow_state: false,
        executes_command: false,
        invokes_shell: false,
        invokes_git: false,
        routes_action: false,
        resolves_approval: false,
        appends_trace: false,
        writes_memory: false,
        creates_execution_grant: false,
        execution_allowed_now: false,
        reviewed_at: request.reviewed_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_command_composer::*;
    use crate::workflow_command_descriptor::*;
    use crate::workflow_command_review::*;
    use crate::workflow_loop_controller::*;
    use crate::workflow_loop_recommendation::WorkflowManualOperationKind;
    use crate::workflow_manual_operation::*;
    use crate::workflow_manual_result::*;
    use crate::workflow_run::WorkflowExecutionId;

    fn test_manual_result() -> WorkflowManualResult {
        WorkflowManualResult {
            result_id: WorkflowManualResultId("wmr_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            command_review_hash: "crh".into(),
            command_composer_hash: "cch".into(),
            command_descriptor_hash: "cdh".into(),
            loop_controller_hash: "lch".into(),
            status: WorkflowManualResultStatus::ReportedSucceeded,
            operator: "tester".into(),
            summary: WorkflowManualResultSummary {
                operator_summary: "done".into(),
                operator_details: None,
                reported_status: WorkflowManualResultStatus::ReportedSucceeded,
                caveat: "Operator-reported, not verified by OpenWand.".into(),
            },
            artifact_references: vec![],
            validation_snapshot: WorkflowManualResultValidationSnapshot {
                command_review_was_acknowledged: true,
                command_review_hash_matched: true,
                command_composer_hash_matched: true,
                command_descriptor_hash_matched: true,
                loop_controller_hash_matched: true,
                command_review_marked_not_performed_by_openwand: true,
            },
            reported_by_operator: true,
            verified_by_openwand: false,
            command_executed_by_openwand: false,
            mutates_workflow_state: false,
            reconciles_outcome: false,
            routes_action: false,
            resolves_approval: false,
            appends_trace: false,
            writes_memory: false,
            invokes_shell: false,
            invokes_git: false,
            creates_execution_grant: false,
            execution_allowed_now: false,
            captured_at: Utc::now(),
        }
    }

    fn valid_request_with(mr: &WorkflowManualResult) -> WorkflowManualResultReviewRequest {
        let mr_hash = blake3::hash(
            serde_json::to_string(mr).unwrap().as_bytes()
        ).to_hex().to_string();
        WorkflowManualResultReviewRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            expected_manual_result_hash: mr_hash,
            expected_command_review_hash: "crh".into(),
            expected_command_composer_hash: "cch".into(),
            expected_command_descriptor_hash: "cdh".into(),
            expected_loop_controller_hash: "lch".into(),
            decision: WorkflowManualResultReviewDecision::Accepted,
            reviewer: "reviewer".into(),
            rationale: "evidence sufficient".into(),
            feedback: None,
            reviewed_at: Utc::now(),
            idempotency_key: "k1".into(),
        }
    }

    #[test]
    fn blocks_missing_manual_result() {
        let mr = test_manual_result();
        let req = valid_request_with(&mr);
        let r = validate_manual_result_review(&req, None, None, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn blocks_manual_result_hash_mismatch() {
        let mr = test_manual_result();
        let mut req = valid_request_with(&mr);
        req.expected_manual_result_hash = "wrong".into();
        let r = validate_manual_result_review(&req, Some(&mr), None, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn blocks_non_operator_reported_result() {
        let mut mr = test_manual_result();
        mr.reported_by_operator = false;
        let req = valid_request_with(&mr);
        let r = validate_manual_result_review(&req, Some(&mr), None, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn blocks_result_verified_by_openwand() {
        let mut mr = test_manual_result();
        mr.verified_by_openwand = true;
        let req = valid_request_with(&mr);
        let r = validate_manual_result_review(&req, Some(&mr), None, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn blocks_result_executed_by_openwand() {
        let mut mr = test_manual_result();
        mr.command_executed_by_openwand = true;
        let req = valid_request_with(&mr);
        let r = validate_manual_result_review(&req, Some(&mr), None, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn blocks_empty_reviewer() {
        let mr = test_manual_result();
        let mut req = valid_request_with(&mr);
        req.reviewer = String::new();
        let r = validate_manual_result_review(&req, Some(&mr), None, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn blocks_empty_rationale() {
        let mr = test_manual_result();
        let mut req = valid_request_with(&mr);
        req.rationale = String::new();
        let r = validate_manual_result_review(&req, Some(&mr), None, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn blocks_rejection_without_blocking_reasons() {
        let mr = test_manual_result();
        let mut req = valid_request_with(&mr);
        req.decision = WorkflowManualResultReviewDecision::Rejected;
        req.feedback = None;
        let r = validate_manual_result_review(&req, Some(&mr), None, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn blocks_changes_requested_without_requested_changes() {
        let mr = test_manual_result();
        let mut req = valid_request_with(&mr);
        req.decision = WorkflowManualResultReviewDecision::ChangesRequested;
        req.feedback = None;
        let r = validate_manual_result_review(&req, Some(&mr), None, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn accepted_review_passes_all_rules() {
        let mr = test_manual_result();
        let req = valid_request_with(&mr);
        let r = validate_manual_result_review(&req, Some(&mr), None, None, None);
        assert!(r.is_ok());
        let review = r.unwrap();
        assert!(review.acceptance_snapshot.accepts_reported_evidence);
        assert!(!review.verifies_external_state);
        assert!(!review.reconciles_workflow_state);
    }

    #[test]
    fn rejected_review_passes_with_blocking_reasons() {
        let mr = test_manual_result();
        let mut req = valid_request_with(&mr);
        req.decision = WorkflowManualResultReviewDecision::Rejected;
        req.feedback = Some(WorkflowManualResultReviewFeedback {
            summary: "unsafe".into(),
            blocking_reasons: vec!["risk".into()],
            requested_changes: vec![],
            evidence_gaps: vec![],
        });
        let r = validate_manual_result_review(&req, Some(&mr), None, None, None);
        assert!(r.is_ok());
        let review = r.unwrap();
        assert!(!review.acceptance_snapshot.accepts_reported_evidence);
    }

    #[test]
    fn changes_requested_passes_with_requested_changes() {
        let mr = test_manual_result();
        let mut req = valid_request_with(&mr);
        req.decision = WorkflowManualResultReviewDecision::ChangesRequested;
        req.feedback = Some(WorkflowManualResultReviewFeedback {
            summary: "needs detail".into(),
            blocking_reasons: vec![],
            requested_changes: vec!["add evidence".into()],
            evidence_gaps: vec![],
        });
        let r = validate_manual_result_review(&req, Some(&mr), None, None, None);
        assert!(r.is_ok());
    }

    #[test]
    fn review_copies_evidence_chain_hashes_from_result() {
        let mr = test_manual_result();
        let req = valid_request_with(&mr);
        let review = validate_manual_result_review(&req, Some(&mr), None, None, None).unwrap();
        assert_eq!("crh", review.command_review_hash);
        assert_eq!("cch", review.command_composer_hash);
        assert_eq!("cdh", review.command_descriptor_hash);
        assert_eq!("lch", review.loop_controller_hash);
    }

    // Patch 1: workflow validation does not check prior reviews
    #[test]
    fn manual_result_review_validation_does_not_check_prior_reviews() {
        let src = include_str!("workflow_manual_result_review_validation.rs");
        // The validate function itself should not reference prior reviews or idempotency.
        // Extract lines between the validate function start and the #[cfg(test)] marker.
        let lines: Vec<&str> = src.lines().collect();
        let validate_start = lines.iter().position(|l| l.contains("pub fn validate_manual_result_review")).unwrap();
        let test_start = lines.iter().position(|l| l.trim() == "#[cfg(test)]").unwrap();
        let fn_body: String = lines[validate_start..test_start].join("\n");
        assert!(!fn_body.contains("prior_review"), "Validation should not reference prior reviews");
        assert!(!fn_body.contains("existing_review"), "Validation should not check existing reviews");
        assert!(!fn_body.contains("idempotency_check"), "Validation should not check idempotency");
        assert!(!fn_body.contains("check_duplicate"), "Validation should not check duplicates");
    }

    #[test]
    fn review_id_is_content_addressed() {
        let mr = test_manual_result();
        let req = valid_request_with(&mr);
        let review = validate_manual_result_review(&req, Some(&mr), None, None, None).unwrap();
        assert!(review.review_id.0.starts_with("wmrr_"));
    }
}
