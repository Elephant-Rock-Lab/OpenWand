//! Manual result validation — hash binding and acknowledgment checks.
//!
//! Validates that the command review was acknowledged, all hashes match,
//! and reported-status requirements are met. Does NOT verify external state.

use serde::{Deserialize, Serialize};

use crate::workflow_command_composer::WorkflowCommandComposerRecord;
use crate::workflow_command_review::{WorkflowCommandReview, WorkflowCommandReviewDecision};
use crate::workflow_loop_controller::WorkflowLoopControllerRecord;
use crate::workflow_manual_result::*;
#[cfg(test)]
use chrono::Utc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowManualResultValidationRule {
    CommandReviewExists,
    CommandComposerExists,
    LoopControllerExists,
    CommandReviewHashMatches,
    CommandComposerHashMatches,
    DescriptorHashMatches,
    LoopControllerHashMatches,
    CommandReviewIsAcknowledged,
    CommandReviewNotPerformedByOpenwand,
    OperatorNonEmpty,
    SummaryNonEmpty,
    ReportedFailedRequiresDetailsOrArtifact,
    ReportedPartialRequiresDetails,
    ArtifactsAreReferencesOnly,
    AcknowledgedDoesNotGrantExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultValidationResult {
    pub rule: WorkflowManualResultValidationRule,
    pub passed: bool,
    pub reason: String,
}

fn v(rule: WorkflowManualResultValidationRule, passed: bool, reason: &str) -> WorkflowManualResultValidationResult {
    WorkflowManualResultValidationResult { rule, passed, reason: reason.into() }
}

pub fn validate_manual_result(
    request: &WorkflowManualResultRequest,
    review_record: Option<&WorkflowCommandReview>,
    composer_record: Option<&WorkflowCommandComposerRecord>,
    loop_controller_record: Option<&WorkflowLoopControllerRecord>,
) -> Result<WorkflowManualResult, Vec<WorkflowManualResultValidationResult>> {
    let mut results = Vec::new();

    // 1. Command review must exist
    results.push(v(WorkflowManualResultValidationRule::CommandReviewExists,
        review_record.is_some(), if review_record.is_some() { "Found" } else { "Missing" }));

    // 2. Command composer record must exist
    results.push(v(WorkflowManualResultValidationRule::CommandComposerExists,
        composer_record.is_some(), if composer_record.is_some() { "Found" } else { "Missing" }));

    // 3. Loop-controller record must exist
    results.push(v(WorkflowManualResultValidationRule::LoopControllerExists,
        loop_controller_record.is_some(), if loop_controller_record.is_some() { "Found" } else { "Missing" }));

    // Patch 2: 4-7. Hashes must match expected hashes
    let review_hash_ok = review_record.is_some_and(|r| {
        let actual = blake3::hash(serde_json::to_string(r).unwrap_or_default().as_bytes()).to_hex().to_string();
        actual == request.expected_command_review_hash
    });
    results.push(v(WorkflowManualResultValidationRule::CommandReviewHashMatches,
        review_hash_ok || review_record.is_none(),
        if review_hash_ok { "Match" } else { "Mismatch" }));

    let composer_hash_ok = composer_record.is_some_and(|c| {
        let actual = blake3::hash(serde_json::to_string(c).unwrap_or_default().as_bytes()).to_hex().to_string();
        actual == request.expected_command_composer_hash
    });
    results.push(v(WorkflowManualResultValidationRule::CommandComposerHashMatches,
        composer_hash_ok || composer_record.is_none(),
        if composer_hash_ok { "Match" } else { "Mismatch" }));

    let desc_hash_ok = composer_record.and_then(|c| c.descriptor.as_ref()).is_some_and(|d| {
        let actual = blake3::hash(serde_json::to_string(d).unwrap_or_default().as_bytes()).to_hex().to_string();
        actual == request.expected_command_descriptor_hash
    });
    results.push(v(WorkflowManualResultValidationRule::DescriptorHashMatches,
        desc_hash_ok || composer_record.is_none(),
        if desc_hash_ok { "Match" } else { "Mismatch" }));

    let lc_hash_ok = loop_controller_record.is_some_and(|l| {
        let actual = blake3::hash(serde_json::to_string(l).unwrap_or_default().as_bytes()).to_hex().to_string();
        actual == request.expected_loop_controller_hash
    });
    results.push(v(WorkflowManualResultValidationRule::LoopControllerHashMatches,
        lc_hash_ok || loop_controller_record.is_none(),
        if lc_hash_ok { "Match" } else { "Mismatch" }));

    // 8. Command review decision must be Acknowledged
    let review_acknowledged = review_record.is_some_and(|r| {
        matches!(r.decision, WorkflowCommandReviewDecision::Acknowledged)
    });
    results.push(v(WorkflowManualResultValidationRule::CommandReviewIsAcknowledged,
        review_acknowledged || review_record.is_none(),
        if review_acknowledged { "Acknowledged" } else { "Not acknowledged" }));

    // 9. Command review snapshot must indicate command_performed_now == false
    let not_performed = review_record.is_none_or(|r| {
        !r.acknowledgment_snapshot.command_performed_now
    });
    results.push(v(WorkflowManualResultValidationRule::CommandReviewNotPerformedByOpenwand,
        not_performed,
        if not_performed { "Not performed" } else { "Claims performed" }));

    // 10. Operator non-empty
    results.push(v(WorkflowManualResultValidationRule::OperatorNonEmpty,
        !request.operator.is_empty(),
        if request.operator.is_empty() { "Empty" } else { "Provided" }));

    // 11. Summary non-empty
    results.push(v(WorkflowManualResultValidationRule::SummaryNonEmpty,
        !request.summary.is_empty(),
        if request.summary.is_empty() { "Empty" } else { "Provided" }));

    // 12. ReportedFailed requires details or artifact reference
    let failed_ok = match request.status {
        WorkflowManualResultStatus::ReportedFailed => {
            request.details.as_ref().is_some_and(|d| !d.is_empty())
                || !request.artifact_references.is_empty()
        }
        _ => true,
    };
    results.push(v(WorkflowManualResultValidationRule::ReportedFailedRequiresDetailsOrArtifact,
        failed_ok, if failed_ok { "Ok" } else { "Missing details/artifact" }));

    // 13. ReportedPartial requires details
    let partial_ok = match request.status {
        WorkflowManualResultStatus::ReportedPartial => {
            request.details.as_ref().is_some_and(|d| !d.is_empty())
        }
        _ => true,
    };
    results.push(v(WorkflowManualResultValidationRule::ReportedPartialRequiresDetails,
        partial_ok, if partial_ok { "Ok" } else { "Missing details" }));

    // 14. Artifact references are references only (structural — always true)
    results.push(v(WorkflowManualResultValidationRule::ArtifactsAreReferencesOnly,
        true, "References only"));

    // 15. Acknowledged does not grant execution (structural — always true)
    results.push(v(WorkflowManualResultValidationRule::AcknowledgedDoesNotGrantExecution,
        true, "No execution grant"));

    let all_pass = results.iter().all(|r| r.passed);
    if !all_pass {
        return Err(results);
    }

    // Build result
    let review_hash = review_record.map_or(String::new(), |r| {
        blake3::hash(serde_json::to_string(r).unwrap_or_default().as_bytes()).to_hex().to_string()
    });
    let composer_hash = composer_record.map_or(String::new(), |c| {
        blake3::hash(serde_json::to_string(c).unwrap_or_default().as_bytes()).to_hex().to_string()
    });
    let desc_hash = composer_record.and_then(|c| c.descriptor.as_ref()).map_or(String::new(), |d| {
        blake3::hash(serde_json::to_string(d).unwrap_or_default().as_bytes()).to_hex().to_string()
    });
    let lc_hash = loop_controller_record.map_or(String::new(), |l| {
        blake3::hash(serde_json::to_string(l).unwrap_or_default().as_bytes()).to_hex().to_string()
    });

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"manual_result:v1:");
    hasher.update(request.workflow_execution_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.command_review_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    let result_id = WorkflowManualResultId(format!("wmr_{}", &hex[..16]));

    Ok(WorkflowManualResult {
        result_id,
        workflow_execution_id: request.workflow_execution_id.clone(),
        command_review_id: request.command_review_id.clone(),
        command_composer_id: request.command_composer_id.clone(),
        loop_controller_id: request.loop_controller_id.clone(),
        command_review_hash: review_hash,
        command_composer_hash: composer_hash,
        command_descriptor_hash: desc_hash,
        loop_controller_hash: lc_hash,
        status: request.status.clone(),
        operator: request.operator.clone(),
        summary: WorkflowManualResultSummary {
            operator_summary: request.summary.clone(),
            operator_details: request.details.clone(),
            reported_status: request.status.clone(),
            caveat: "Operator-reported, not verified by OpenWand.".into(),
        },
        artifact_references: request.artifact_references.clone(),
        validation_snapshot: WorkflowManualResultValidationSnapshot {
            command_review_was_acknowledged: review_acknowledged,
            command_review_hash_matched: review_hash_ok,
            command_composer_hash_matched: composer_hash_ok,
            command_descriptor_hash_matched: desc_hash_ok,
            loop_controller_hash_matched: lc_hash_ok,
            command_review_marked_not_performed_by_openwand: not_performed,
        },
        reported_by_operator: true,
        verified_by_openwand: false, command_executed_by_openwand: false,
        mutates_workflow_state: false, reconciles_outcome: false,
        routes_action: false, resolves_approval: false,
        appends_trace: false, writes_memory: false,
        invokes_shell: false, invokes_git: false,
        creates_execution_grant: false, execution_allowed_now: false,
        captured_at: request.captured_at,
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
    use crate::workflow_run::WorkflowExecutionId;

    fn test_review() -> WorkflowCommandReview {
        WorkflowCommandReview {
            review_id: WorkflowCommandReviewId("wcrv_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            command_composer_hash: "ch".into(), command_descriptor_hash: "dh".into(),
            loop_controller_hash: "lh".into(),
            decision: WorkflowCommandReviewDecision::Acknowledged,
            reviewer: "tester".into(), rationale: "ok".into(), feedback: None,
            acknowledgment_snapshot: WorkflowCommandAcknowledgmentSnapshot {
                descriptor_display_command: "test".into(),
                descriptor_copyable_text_hash: "cth".into(),
                descriptor_display_only: true, descriptor_executable: false,
                descriptor_missing_inputs: vec![], loop_detected_state: "idle".into(),
                loop_recommended_operation: "none".into(),
                acknowledges_review_only: true, command_performed_now: false,
            },
            executes_command: false, invokes_shell: false, invokes_git: false,
            routes_action: false, resolves_approval: false, reconciles_outcome: false,
            mutates_workflow_state: false, schedules_work: false, starts_worker: false,
            queues_operation: false, creates_execution_grant: false,
            execution_allowed_now: false, reviewed_at: Utc::now(),
        }
    }

    fn test_composer() -> WorkflowCommandComposerRecord {
        WorkflowCommandComposerRecord {
            composer_id: WorkflowCommandComposerId("wcc_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            loop_controller_hash: "lh".into(),
            status: WorkflowCommandComposerStatus::DescriptorReady,
            decision: WorkflowCommandComposerDecision::DescriptorReady { summary: "ok".into() },
            predicates: vec![], descriptor: Some(WorkflowManualCommandDescriptor {
                command_kind: WorkflowManualCommandKind::WorkflowContinuationPropose,
                display_command: "test".into(), arguments: vec![],
                missing_inputs: vec![], safety_warnings: vec![], evidence_links: vec![],
                copyable_text: "test".into(), display_only: true, executable: false,
            }),
            missing_inputs: vec![], evidence_links: vec![],
            executes_command: false, invokes_shell: false, invokes_git: false,
            routes_action: false, resolves_approval: false, reconciles_outcome: false,
            mutates_workflow_state: false, schedules_work: false, starts_worker: false,
            queues_operation: false, created_at: Utc::now(),
        }
    }

    fn test_lc() -> WorkflowLoopControllerRecord {
        WorkflowLoopControllerRecord {
            controller_id: WorkflowLoopControllerId("wlc_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            latest_run_revision_id: None,
            status: WorkflowLoopControllerStatus::RecommendationReady,
            decision: WorkflowLoopControllerDecision::Recommend {
                operation: WorkflowManualOperationKind::NoAction, summary: "test".into(),
            },
            loop_state: None, recommendation: None,
            predicates: vec![], evidence_links: vec![],
            creates_route: false, resolves_approval: false, reconciles_outcome: false,
            executes_tool: false, mutates_workflow_state: false,
            schedules_work: false, starts_worker: false, queues_operation: false,
            retries_operation: false, resumes_workflow: false, created_at: Utc::now(),
        }
    }

    fn valid_request_with(review: &WorkflowCommandReview, composer: &WorkflowCommandComposerRecord, lc: &WorkflowLoopControllerRecord) -> WorkflowManualResultRequest {
        WorkflowManualResultRequest {
            expected_command_review_hash: blake3::hash(
                serde_json::to_string(review).unwrap().as_bytes()
            ).to_hex().to_string(),
            expected_command_composer_hash: blake3::hash(
                serde_json::to_string(composer).unwrap().as_bytes()
            ).to_hex().to_string(),
            expected_command_descriptor_hash: composer.descriptor.as_ref().map(|d| {
                blake3::hash(serde_json::to_string(d).unwrap().as_bytes()).to_hex().to_string()
            }).unwrap_or_default(),
            expected_loop_controller_hash: blake3::hash(
                serde_json::to_string(lc).unwrap().as_bytes()
            ).to_hex().to_string(),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            status: WorkflowManualResultStatus::ReportedSucceeded,
            operator: "tester".into(), summary: "done".into(),
            details: None, artifact_references: vec![],
            captured_at: Utc::now(), idempotency_key: "k1".into(),
        }
    }

    // Macro to set up shared records per test
    #[allow(unused_macros)]
macro_rules! setup {
        () => {
            let review = test_review();
            let composer = test_composer();
            let lc = test_lc();
            let req = valid_request_with(&review, &composer, &lc);
        };
    }

    #[test] fn blocks_missing_command_review() {
        let review = test_review(); let composer = test_composer(); let lc = test_lc();
        let req = valid_request_with(&review, &composer, &lc);
        let r = validate_manual_result(&req, None, Some(&composer), Some(&lc));
        assert!(r.is_err());
    }
    #[test] fn blocks_missing_command_composer() {
        let review = test_review(); let composer = test_composer(); let lc = test_lc();
        let req = valid_request_with(&review, &composer, &lc);
        let r = validate_manual_result(&req, Some(&review), None, Some(&lc));
        assert!(r.is_err());
    }
    #[test] fn blocks_missing_loop_controller() {
        let review = test_review(); let composer = test_composer(); let lc = test_lc();
        let req = valid_request_with(&review, &composer, &lc);
        let r = validate_manual_result(&req, Some(&review), Some(&composer), None);
        assert!(r.is_err());
    }
    #[test] fn blocks_command_review_hash_mismatch() {
        let review = test_review(); let composer = test_composer(); let lc = test_lc();
        let mut req = valid_request_with(&review, &composer, &lc);
        req.expected_command_review_hash = "wrong".into();
        let r = validate_manual_result(&req, Some(&review), Some(&composer), Some(&lc));
        assert!(r.is_err());
    }
    #[test] fn blocks_command_composer_hash_mismatch() {
        let review = test_review(); let composer = test_composer(); let lc = test_lc();
        let mut req = valid_request_with(&review, &composer, &lc);
        req.expected_command_composer_hash = "wrong".into();
        let r = validate_manual_result(&req, Some(&review), Some(&composer), Some(&lc));
        assert!(r.is_err());
    }
    #[test] fn blocks_command_descriptor_hash_mismatch() {
        let review = test_review(); let composer = test_composer(); let lc = test_lc();
        let mut req = valid_request_with(&review, &composer, &lc);
        req.expected_command_descriptor_hash = "wrong".into();
        let r = validate_manual_result(&req, Some(&review), Some(&composer), Some(&lc));
        assert!(r.is_err());
    }
    #[test] fn blocks_loop_controller_hash_mismatch() {
        let review = test_review(); let composer = test_composer(); let lc = test_lc();
        let mut req = valid_request_with(&review, &composer, &lc);
        req.expected_loop_controller_hash = "wrong".into();
        let r = validate_manual_result(&req, Some(&review), Some(&composer), Some(&lc));
        assert!(r.is_err());
    }
    #[test] fn blocks_unacknowledged_command_review() {
        let mut review = test_review();
        review.decision = WorkflowCommandReviewDecision::Rejected;
        let composer = test_composer(); let lc = test_lc();
        let req = valid_request_with(&review, &composer, &lc);
        let r = validate_manual_result(&req, Some(&review), Some(&composer), Some(&lc));
        assert!(r.is_err());
    }
    #[test] fn blocks_command_review_that_claims_command_performed() {
        let mut review = test_review();
        review.acknowledgment_snapshot.command_performed_now = true;
        let composer = test_composer(); let lc = test_lc();
        let req = valid_request_with(&review, &composer, &lc);
        let r = validate_manual_result(&req, Some(&review), Some(&composer), Some(&lc));
        assert!(r.is_err());
    }
    #[test] fn reported_failed_requires_details_or_artifact() {
        let review = test_review(); let composer = test_composer(); let lc = test_lc();
        let mut req = valid_request_with(&review, &composer, &lc);
        req.status = WorkflowManualResultStatus::ReportedFailed;
        req.details = None;
        req.artifact_references = vec![];
        let r = validate_manual_result(&req, Some(&review), Some(&composer), Some(&lc));
        assert!(r.is_err());
    }
    #[test] fn reported_partial_requires_details() {
        let review = test_review(); let composer = test_composer(); let lc = test_lc();
        let mut req = valid_request_with(&review, &composer, &lc);
        req.status = WorkflowManualResultStatus::ReportedPartial;
        req.details = None;
        let r = validate_manual_result(&req, Some(&review), Some(&composer), Some(&lc));
        assert!(r.is_err());
    }
    #[test] fn manual_result_capture_does_not_verify_external_state() {
        let review = test_review(); let composer = test_composer(); let lc = test_lc();
        let req = valid_request_with(&review, &composer, &lc);
        let result = validate_manual_result(&req, Some(&review), Some(&composer), Some(&lc));
        let r = result.unwrap();
        assert!(!r.verified_by_openwand);
        assert!(!r.command_executed_by_openwand);
    }
}
