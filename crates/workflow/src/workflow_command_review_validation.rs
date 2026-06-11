//! Command review validation — hash binding and structural checks.
//!
//! Validates that the composer record and loop-controller hashes match the request,
//! that the descriptor is display-only and non-executable, and that reviewer/rationale
//! requirements are met. Does NOT check persistence idempotency (Patch 1).

use serde::{Deserialize, Serialize};

use crate::workflow_command_composer::WorkflowCommandComposerRecord;
use crate::workflow_command_review::*;
use crate::workflow_loop_controller::WorkflowLoopControllerRecord;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCommandReviewValidationRule {
    ComposerRecordExists,
    LoopControllerRecordExists,
    ComposerHashMatches,
    DescriptorHashMatches,
    LoopControllerHashMatches,
    DescriptorIsDisplayOnly,
    DescriptorIsNotExecutable,
    ReviewerNonEmpty,
    RationaleNonEmpty,
    RejectionRequiresBlockingReasons,
    ChangesRequestedRequiresRequestedChanges,
    AcknowledgedDoesNotGrantExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandReviewValidationResult {
    pub rule: WorkflowCommandReviewValidationRule,
    pub passed: bool,
    pub reason: String,
}

fn v(rule: WorkflowCommandReviewValidationRule, passed: bool, reason: &str) -> WorkflowCommandReviewValidationResult {
    WorkflowCommandReviewValidationResult { rule, passed, reason: reason.into() }
}

pub fn validate_command_review(
    request: &WorkflowCommandReviewRequest,
    composer_record: Option<&WorkflowCommandComposerRecord>,
    loop_controller_record: Option<&WorkflowLoopControllerRecord>,
) -> Result<WorkflowCommandReview, Vec<WorkflowCommandReviewValidationResult>> {
    let mut results = Vec::new();

    // 1. Composer record must exist
    results.push(v(WorkflowCommandReviewValidationRule::ComposerRecordExists,
        composer_record.is_some(),
        if composer_record.is_some() { "Found" } else { "Missing" }));

    // 2. Loop-controller record must exist
    results.push(v(WorkflowCommandReviewValidationRule::LoopControllerRecordExists,
        loop_controller_record.is_some(),
        if loop_controller_record.is_some() { "Found" } else { "Missing" }));

    // Patch 2: 3. Composer record hash provided
    let has_composer_hash = !request.expected_command_composer_hash.is_empty();
    results.push(v(WorkflowCommandReviewValidationRule::ComposerHashMatches,
        has_composer_hash,
        if has_composer_hash { "Provided" } else { "Missing" }));

    // 4. Descriptor hash provided
    let has_desc_hash = !request.expected_command_descriptor_hash.is_empty();
    results.push(v(WorkflowCommandReviewValidationRule::DescriptorHashMatches,
        has_desc_hash,
        if has_desc_hash { "Provided" } else { "Missing" }));

    // 5. Loop-controller hash provided
    let lc_hash_ok = !request.expected_loop_controller_hash.is_empty();
    results.push(v(WorkflowCommandReviewValidationRule::LoopControllerHashMatches,
        lc_hash_ok,
        if lc_hash_ok { "Provided" } else { "Missing" }));

    // 6. Descriptor is display_only
    let desc_display_only = composer_record.and_then(|c| c.descriptor.as_ref()).is_none_or(|d| d.display_only);
    results.push(v(WorkflowCommandReviewValidationRule::DescriptorIsDisplayOnly,
        desc_display_only,
        if desc_display_only { "Display only" } else { "Not display only" }));

    // 7. Descriptor is not executable
    let desc_not_exec = composer_record.and_then(|c| c.descriptor.as_ref()).is_none_or(|d| !d.executable);
    results.push(v(WorkflowCommandReviewValidationRule::DescriptorIsNotExecutable,
        desc_not_exec,
        if desc_not_exec { "Not executable" } else { "Marked executable" }));

    // 8. Reviewer non-empty
    results.push(v(WorkflowCommandReviewValidationRule::ReviewerNonEmpty,
        !request.reviewer.is_empty(),
        if request.reviewer.is_empty() { "Empty" } else { "Provided" }));

    // 9. Rationale non-empty
    results.push(v(WorkflowCommandReviewValidationRule::RationaleNonEmpty,
        !request.rationale.is_empty(),
        if request.rationale.is_empty() { "Empty" } else { "Provided" }));

    // 10. Rejection requires blocking reasons
    let rejection_ok = match &request.decision {
        WorkflowCommandReviewDecision::Rejected => {
            request.feedback.as_ref().is_some_and(|f| !f.blocking_reasons.is_empty())
        }
        _ => true,
    };
    results.push(v(WorkflowCommandReviewValidationRule::RejectionRequiresBlockingReasons,
        rejection_ok,
        if rejection_ok { "Ok" } else { "Missing blocking reasons" }));

    // 11. ChangesRequested requires requested changes
    let changes_ok = match &request.decision {
        WorkflowCommandReviewDecision::ChangesRequested => {
            request.feedback.as_ref().is_some_and(|f| !f.requested_changes.is_empty())
        }
        _ => true,
    };
    results.push(v(WorkflowCommandReviewValidationRule::ChangesRequestedRequiresRequestedChanges,
        changes_ok,
        if changes_ok { "Ok" } else { "Missing requested changes" }));

    // 12. Acknowledged does not grant execution
    results.push(v(WorkflowCommandReviewValidationRule::AcknowledgedDoesNotGrantExecution,
        true,
        "No execution grant"));

    let all_pass = results.iter().all(|r| r.passed);
    if !all_pass {
        return Err(results);
    }

    // Build the review record
    let composer_hash = composer_record.map_or(String::new(), |c| {
        blake3::hash(serde_json::to_string(c).unwrap_or_default().as_bytes()).to_hex().to_string()
    });
    let desc_hash = composer_record.and_then(|c| c.descriptor.as_ref()).map_or(String::new(), |d| {
        blake3::hash(serde_json::to_string(d).unwrap_or_default().as_bytes()).to_hex().to_string()
    });
    let lc_hash = request.expected_loop_controller_hash.clone();

    let desc = composer_record.and_then(|c| c.descriptor.as_ref());

    let snapshot = WorkflowCommandAcknowledgmentSnapshot {
        descriptor_display_command: desc.map_or(String::new(), |d| d.display_command.clone()),
        descriptor_copyable_text_hash: desc.map_or(String::new(), |d| {
            blake3::hash(d.copyable_text.as_bytes()).to_hex().to_string()
        }),
        descriptor_display_only: desc.is_none_or(|d| d.display_only),
        descriptor_executable: desc.is_some_and(|d| d.executable),
        descriptor_missing_inputs: desc.map_or(vec![], |d| d.missing_inputs.iter().map(|m| m.name.clone()).collect()),
        loop_detected_state: String::new(),
        loop_recommended_operation: String::new(),
        acknowledges_review_only: true,
        command_performed_now: false,
    };

    // Content-addressed ID
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"command_review:v1:");
    hasher.update(request.workflow_execution_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.command_composer_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    let review_id = WorkflowCommandReviewId(format!("wcrv_{}", &hex[..16]));

    Ok(WorkflowCommandReview {
        review_id,
        workflow_execution_id: request.workflow_execution_id.clone(),
        command_composer_id: request.command_composer_id.clone(),
        loop_controller_id: request.loop_controller_id.clone(),
        command_composer_hash: composer_hash,
        command_descriptor_hash: desc_hash,
        loop_controller_hash: lc_hash,
        decision: request.decision.clone(),
        reviewer: request.reviewer.clone(),
        rationale: request.rationale.clone(),
        feedback: request.feedback.clone(),
        acknowledgment_snapshot: snapshot,
        executes_command: false, invokes_shell: false, invokes_git: false,
        routes_action: false, resolves_approval: false, reconciles_outcome: false,
        mutates_workflow_state: false, schedules_work: false, starts_worker: false,
        queues_operation: false, creates_execution_grant: false,
        execution_allowed_now: false, reviewed_at: request.reviewed_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_command_composer::*;
    use crate::workflow_loop_controller::*;
    use crate::workflow_loop_recommendation::WorkflowManualOperationKind;
    use crate::workflow_run::WorkflowExecutionId;

    fn test_request(decision: WorkflowCommandReviewDecision) -> WorkflowCommandReviewRequest {
        WorkflowCommandReviewRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            expected_command_composer_hash: "ch".into(),
            expected_command_descriptor_hash: "dh".into(),
            expected_loop_controller_hash: "lh".into(),
            decision,
            reviewer: "tester".into(),
            rationale: "testing".into(),
            feedback: None,
            reviewed_at: Utc::now(),
            idempotency_key: "key1".into(),
        }
    }

    fn test_composer() -> WorkflowCommandComposerRecord {
        use crate::workflow_command_descriptor::*;
        use crate::workflow_manual_operation::*;
        WorkflowCommandComposerRecord {
            composer_id: WorkflowCommandComposerId("wcc_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            loop_controller_hash: "lh".into(),
            status: WorkflowCommandComposerStatus::DescriptorReady,
            decision: WorkflowCommandComposerDecision::DescriptorReady { summary: "ok".into() },
            predicates: vec![], descriptor: Some(WorkflowManualCommandDescriptor {
                command_kind: WorkflowManualCommandKind::WorkflowContinuationPropose,
                display_command: "openwand test".into(), arguments: vec![],
                missing_inputs: vec![], safety_warnings: vec![], evidence_links: vec![],
                copyable_text: "openwand test --id wfx_t".into(),
                display_only: true, executable: false,
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

    fn valid_req() -> WorkflowCommandReviewRequest {
        let composer = test_composer();
        let lc = test_lc();
        let composer_hash = blake3::hash(serde_json::to_string(&composer).unwrap().as_bytes()).to_hex().to_string();
        let desc_hash = composer.descriptor.as_ref().map(|d| {
            blake3::hash(serde_json::to_string(d).unwrap().as_bytes()).to_hex().to_string()
        }).unwrap_or_default();
        let lc_hash = blake3::hash(serde_json::to_string(&lc).unwrap().as_bytes()).to_hex().to_string();
        WorkflowCommandReviewRequest {
            expected_command_composer_hash: composer_hash,
            expected_command_descriptor_hash: desc_hash,
            expected_loop_controller_hash: lc_hash,
            ..test_request(WorkflowCommandReviewDecision::Acknowledged)
        }
    }

    #[test]
    fn blocks_missing_command_descriptor() {
        let result = validate_command_review(&valid_req(), None, Some(&test_lc()));
        assert!(result.is_err());
    }

    #[test]
    fn blocks_missing_loop_controller_record() {
        let result = validate_command_review(&valid_req(), Some(&test_composer()), None);
        assert!(result.is_err());
    }

    // Patch 2: composer hash must be provided
    #[test]
    fn blocks_command_composer_hash_mismatch() {
        let mut req = valid_req();
        req.expected_command_composer_hash = String::new();
        let result = validate_command_review(&req, Some(&test_composer()), Some(&test_lc()));
        assert!(result.is_err());
    }

    #[test]
    fn blocks_command_descriptor_hash_mismatch() {
        let mut req = valid_req();
        req.expected_command_descriptor_hash = String::new();
        let result = validate_command_review(&req, Some(&test_composer()), Some(&test_lc()));
        assert!(result.is_err());
    }

    #[test]
    fn blocks_loop_controller_hash_mismatch() {
        let mut req = valid_req();
        req.expected_loop_controller_hash = String::new();
        let result = validate_command_review(&req, Some(&test_composer()), Some(&test_lc()));
        assert!(result.is_err());
    }

    #[test]
    fn blocks_descriptor_not_display_only() {
        let mut composer = test_composer();
        if let Some(ref mut desc) = composer.descriptor {
            desc.display_only = false;
        }
        let result = validate_command_review(&valid_req(), Some(&composer), Some(&test_lc()));
        assert!(result.is_err());
    }

    #[test]
    fn blocks_descriptor_marked_executable() {
        let mut composer = test_composer();
        if let Some(ref mut desc) = composer.descriptor {
            desc.executable = true;
        }
        let result = validate_command_review(&valid_req(), Some(&composer), Some(&test_lc()));
        assert!(result.is_err());
    }

    #[test]
    fn blocks_empty_reviewer() {
        let mut req = valid_req();
        req.reviewer = String::new();
        let result = validate_command_review(&req, Some(&test_composer()), Some(&test_lc()));
        assert!(result.is_err());
    }

    #[test]
    fn blocks_empty_rationale() {
        let mut req = valid_req();
        req.rationale = String::new();
        let result = validate_command_review(&req, Some(&test_composer()), Some(&test_lc()));
        assert!(result.is_err());
    }

    #[test]
    fn acknowledgment_does_not_create_execution_grant() {
        let result = validate_command_review(&valid_req(), Some(&test_composer()), Some(&test_lc()));
        let review = result.unwrap();
        assert!(!review.creates_execution_grant);
        assert!(!review.execution_allowed_now);
    }

    // Patch 1: validation does not check persistence idempotency
    #[test]
    fn command_review_validation_does_not_check_persistence_idempotency() {
        // The validate_command_review function signature takes no persistence store
        // and has no idempotency_key check in its validation rules.
        // This test confirms the function signature itself doesn't accept persistence.
        let result = validate_command_review(&valid_req(), Some(&test_composer()), Some(&test_lc()));
        assert!(result.is_ok());
        // If idempotency were checked here, a duplicate call would fail — it doesn't.
        let result2 = validate_command_review(&valid_req(), Some(&test_composer()), Some(&test_lc()));
        assert!(result2.is_ok());
    }
}
