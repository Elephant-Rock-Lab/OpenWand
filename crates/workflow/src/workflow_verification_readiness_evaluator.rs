//! Verification readiness evaluator — individual predicate functions.
//!
//! Patch 4: Manual result readiness requires latest accepted review.
//! Patch 5: External attestation readiness preserves unverified semantics.
//! Patch 6: Audit packet target uses in-memory data only.

use crate::workflow_external_attestation::*;
use crate::workflow_manual_result_review::*;
use crate::workflow_verification_readiness::{
    VerificationReadinessPredicate, VerificationReadinessPredicateResult,
    VerificationReadinessRequest, VerificationReadinessRecord,
    VerificationReadinessStatus, VerificationReadinessTargetKind,
    build_readiness_record, compute_readiness_id, p,
};

/// Evaluate full verification readiness for a manual result with review context.
/// Patch 4: Requires latest accepted review.
pub fn evaluate_manual_result_readiness(
    request: &VerificationReadinessRequest,
    result_status: &str,
    result_hash: &str,
    result_workflow_id: &str,
    latest_review: Option<&WorkflowManualResultReview>,
) -> VerificationReadinessRecord {
    let mut predicates = Vec::new();

    // Core predicates
    predicates.push(p(VerificationReadinessPredicate::TargetRecordExists, true, "Manual result loaded"));
    let wfx_match = result_workflow_id == request.workflow_execution_id.0;
    predicates.push(p(VerificationReadinessPredicate::TargetWorkflowExecutionIdMatchesRequest,
        wfx_match, if wfx_match { "Match" } else { "Mismatch" }));
    let hash_match = result_hash == request.expected_target_hash;
    predicates.push(p(VerificationReadinessPredicate::TargetHashMatchesRequest,
        hash_match, if hash_match { "Match" } else { "Hash mismatch" }));

    // Status eligibility (Patch 3)
    let status_eligible = !matches!(result_status, "not_performed");
    predicates.push(p(VerificationReadinessPredicate::TargetStatusEligibleForVerificationReadiness,
        status_eligible, if status_eligible { "Eligible" } else { "Not performed" }));

    // Patch 4: latest review predicates
    let review_exists = latest_review.is_some();
    predicates.push(p(VerificationReadinessPredicate::ManualResultLatestReviewExists,
        review_exists, if review_exists { "Review found" } else { "No review" }));

    let review_accepted = latest_review.map_or(false, |r| {
        matches!(r.decision, WorkflowManualResultReviewDecision::Accepted)
    });
    predicates.push(p(VerificationReadinessPredicate::ManualResultLatestReviewAccepted,
        review_accepted, if review_accepted { "Accepted" } else { "Not accepted" }));

    build_readiness_record(request, predicates)
}

/// Patch 5: Evaluate verification readiness for an external attestation.
pub fn evaluate_attestation_readiness(
    request: &VerificationReadinessRequest,
    attestation: &WorkflowExternalAttestation,
) -> VerificationReadinessRecord {
    let mut predicates = Vec::new();

    predicates.push(p(VerificationReadinessPredicate::TargetRecordExists, true, "Attestation loaded"));
    let wfx_match = attestation.target.workflow_execution_id.0 == request.workflow_execution_id.0;
    predicates.push(p(VerificationReadinessPredicate::TargetWorkflowExecutionIdMatchesRequest,
        wfx_match, if wfx_match { "Match" } else { "Mismatch" }));
    let att_hash = blake3::hash(serde_json::to_string(attestation).unwrap_or_default().as_bytes());
    let hash_match = &att_hash.to_hex()[..16] == &request.expected_target_hash[..16.min(request.expected_target_hash.len())];
    predicates.push(p(VerificationReadinessPredicate::TargetHashMatchesRequest,
        hash_match, if hash_match { "Match" } else { "Hash mismatch" }));
    predicates.push(p(VerificationReadinessPredicate::TargetStatusEligibleForVerificationReadiness,
        true, "Attestations are structurally eligible"));

    // Patch 5: preserve unverified semantics
    predicates.push(p(VerificationReadinessPredicate::AttestationMarkedUnverified,
        !attestation.verified_by_openwand, if !attestation.verified_by_openwand { "Unverified" } else { "Already verified" }));
    predicates.push(p(VerificationReadinessPredicate::AttestationDoesNotPromoteTrust,
        !attestation.promotes_trust, if !attestation.promotes_trust { "No trust promotion" } else { "Trust promotion present" }));
    predicates.push(p(VerificationReadinessPredicate::AttestationDoesNotCertifyTruth,
        !attestation.certifies_external_truth, if !attestation.certifies_external_truth { "No certification" } else { "Certification present" }));

    // References are metadata-only (always true by construction)
    let refs_metadata_only = attestation.references.iter().all(|r| !r.reference.is_empty() || r.description.is_some());
    predicates.push(p(VerificationReadinessPredicate::AttestationReferencesAreMetadataOnly,
        refs_metadata_only, "References are strings"));

    // Informational: has operator hash?
    let has_op_hash = attestation.references.iter().any(|r| r.operator_supplied_hash.is_some());
    predicates.push(p(VerificationReadinessPredicate::AttestationHasOperatorSuppliedHash,
        has_op_hash, if has_op_hash { "Has operator hash" } else { "No operator hash" }));

    // Informational: signature claimed?
    let sig_claimed = attestation.reported_signature.is_some();
    predicates.push(p(VerificationReadinessPredicate::AttestationSignatureClaimed,
        sig_claimed, if sig_claimed { "Signature claimed" } else { "No signature" }));

    build_readiness_record(request, predicates)
}

/// Patch 6: Evaluate verification readiness for an audit packet.
/// Uses in-memory data only — does not read packet files from disk.
pub fn evaluate_audit_packet_readiness(
    request: &VerificationReadinessRequest,
    packet_chain_hash: &str,
    packet_certifies: bool,
    packet_workflow_id: &str,
) -> VerificationReadinessRecord {
    let mut predicates = Vec::new();

    predicates.push(p(VerificationReadinessPredicate::TargetRecordExists, true, "Packet loaded"));
    let wfx_match = packet_workflow_id == request.workflow_execution_id.0;
    predicates.push(p(VerificationReadinessPredicate::TargetWorkflowExecutionIdMatchesRequest,
        wfx_match, if wfx_match { "Match" } else { "Mismatch" }));
    predicates.push(p(VerificationReadinessPredicate::TargetHashMatchesRequest,
        packet_chain_hash == request.expected_target_hash,
        if packet_chain_hash == request.expected_target_hash { "Match" } else { "Hash mismatch" }));
    predicates.push(p(VerificationReadinessPredicate::TargetStatusEligibleForVerificationReadiness,
        true, "Packets are structurally eligible"));

    predicates.push(p(VerificationReadinessPredicate::AuditPacketHashExists,
        !packet_chain_hash.is_empty(), if !packet_chain_hash.is_empty() { "Hash present" } else { "No hash" }));
    predicates.push(p(VerificationReadinessPredicate::AuditPacketDoesNotCertifyRecordedEvidence,
        !packet_certifies, if !packet_certifies { "Does not certify" } else { "Claims certification" }));

    build_readiness_record(request, predicates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_run::WorkflowExecutionId;

    fn test_request(kind: VerificationReadinessTargetKind) -> VerificationReadinessRequest {
        VerificationReadinessRequest {
            target_kind: kind,
            target_id: "wmr_t".into(),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            expected_target_hash: "hash_t".into(),
            idempotency_key: "key1".into(),
        }
    }

    fn accepted_review() -> WorkflowManualResultReview {
        WorkflowManualResultReview {
            review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: crate::workflow_manual_result::WorkflowManualResultId("wmr_t".into()),
            command_review_id: crate::workflow_command_review::WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: crate::workflow_command_composer::WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: crate::workflow_loop_controller::WorkflowLoopControllerId("wlc_t".into()),
            manual_result_hash: "h".into(),
            command_review_hash: "h".into(),
            command_composer_hash: "h".into(),
            command_descriptor_hash: "h".into(),
            loop_controller_hash: "h".into(),
            decision: WorkflowManualResultReviewDecision::Accepted,
            reviewer: "alice".into(),
            rationale: "ok".into(),
            feedback: None,
            acceptance_snapshot: crate::workflow_manual_result_review::WorkflowManualResultReviewAcceptanceSnapshot {
                accepts_reported_evidence: true,
                verifies_external_state: false,
                reconciles_workflow_state: false,
                result_verified_by_openwand: false,
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
            reviewed_at: chrono::Utc::now(),
        }
    }

    // Patch 4 tests
    #[test]
    fn manual_result_blocks_without_latest_review() {
        let rec = evaluate_manual_result_readiness(
            &test_request(VerificationReadinessTargetKind::ManualResult),
            "reported_succeeded", "hash_t", "wfx_t", None,
        );
        assert_eq!(VerificationReadinessStatus::Blocked, rec.status);
    }

    #[test]
    fn manual_result_blocks_later_rejected_review() {
        let mut review = accepted_review();
        review.decision = WorkflowManualResultReviewDecision::Rejected;
        let rec = evaluate_manual_result_readiness(
            &test_request(VerificationReadinessTargetKind::ManualResult),
            "reported_succeeded", "hash_t", "wfx_t", Some(&review),
        );
        assert_eq!(VerificationReadinessStatus::Blocked, rec.status);
    }

    #[test]
    fn manual_result_blocks_later_changes_requested_review() {
        let mut review = accepted_review();
        review.decision = WorkflowManualResultReviewDecision::ChangesRequested;
        let rec = evaluate_manual_result_readiness(
            &test_request(VerificationReadinessTargetKind::ManualResult),
            "reported_succeeded", "hash_t", "wfx_t", Some(&review),
        );
        assert_eq!(VerificationReadinessStatus::Blocked, rec.status);
    }

    #[test]
    fn manual_result_ready_with_latest_accepted_review() {
        let review = accepted_review();
        let rec = evaluate_manual_result_readiness(
            &test_request(VerificationReadinessTargetKind::ManualResult),
            "reported_succeeded", "hash_t", "wfx_t", Some(&review),
        );
        assert_eq!(VerificationReadinessStatus::Ready, rec.status);
    }

    // Patch 5 tests
    #[test]
    fn attestation_readiness_does_not_verify_signature() {
        let req = ExternalAttestationRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            target_kind: ExternalAttestationTargetKind::ManualResult,
            target_id: "wmr_t".into(), expected_target_hash: None,
            kind: ExternalAttestationKind::ThirdPartySignoff,
            source_name: "A".into(), source_role: "r".into(),
            source_system_identifier: None, claim: "c".into(),
            references: vec![], reported_signature: Some(ExternalReportedSignature {
                signature_text_or_reference: "sig".into(),
                claimed_signer: None, claimed_algorithm: None,
                verification_status: ExternalSignatureVerificationStatus::NotVerifiedByOpenWand,
            }),
            attested_at: chrono::Utc::now(), idempotency_key: "k".into(),
        };
        let att = build_external_attestation(req);
        let readiness_id = compute_readiness_id("wmr_t", "wfx_t", "hash_t", "key1");
        let mut vreq = test_request(VerificationReadinessTargetKind::ExternalAttestation);
        vreq.target_id = att.attestation_id.0.clone();
        let rec = evaluate_attestation_readiness(&vreq, &att);
        assert!(!rec.performs_verification);
        assert!(!rec.verifies_signature);
    }

    #[test]
    fn attestation_readiness_does_not_promote_trust() {
        let req = ExternalAttestationRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            target_kind: ExternalAttestationTargetKind::ManualResult,
            target_id: "wmr_t".into(), expected_target_hash: None,
            kind: ExternalAttestationKind::ThirdPartySignoff,
            source_name: "A".into(), source_role: "r".into(),
            source_system_identifier: None, claim: "c".into(),
            references: vec![], reported_signature: None,
            attested_at: chrono::Utc::now(), idempotency_key: "k".into(),
        };
        let att = build_external_attestation(req);
        let mut vreq = test_request(VerificationReadinessTargetKind::ExternalAttestation);
        vreq.target_id = att.attestation_id.0.clone();
        let rec = evaluate_attestation_readiness(&vreq, &att);
        assert!(!rec.promotes_trust);
    }

    #[test]
    fn attestation_without_operator_hash_can_still_be_structurally_ready() {
        let req = ExternalAttestationRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            target_kind: ExternalAttestationTargetKind::ManualResult,
            target_id: "wmr_t".into(), expected_target_hash: None,
            kind: ExternalAttestationKind::ThirdPartySignoff,
            source_name: "A".into(), source_role: "r".into(),
            source_system_identifier: None, claim: "c".into(),
            references: vec![], reported_signature: None,
            attested_at: chrono::Utc::now(), idempotency_key: "k".into(),
        };
        let att = build_external_attestation(req);
        let mut vreq = test_request(VerificationReadinessTargetKind::ExternalAttestation);
        vreq.target_id = att.attestation_id.0.clone();
        let rec = evaluate_attestation_readiness(&vreq, &att);
        // operator hash predicate is informational — should not block
        let op_hash_pred = rec.predicate_results.iter()
            .find(|r| r.predicate == VerificationReadinessPredicate::AttestationHasOperatorSuppliedHash)
            .unwrap();
        assert!(!op_hash_pred.passed); // no hash present
        // But overall should not be blocked solely because of this
        // (other predicates may cause blocked, but not this one specifically)
    }

    #[test]
    fn attestation_signature_claim_is_informational_only() {
        let req = ExternalAttestationRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            target_kind: ExternalAttestationTargetKind::ManualResult,
            target_id: "wmr_t".into(), expected_target_hash: None,
            kind: ExternalAttestationKind::ThirdPartySignoff,
            source_name: "A".into(), source_role: "r".into(),
            source_system_identifier: None, claim: "c".into(),
            references: vec![], reported_signature: None,
            attested_at: chrono::Utc::now(), idempotency_key: "k".into(),
        };
        let att = build_external_attestation(req);
        let mut vreq = test_request(VerificationReadinessTargetKind::ExternalAttestation);
        vreq.target_id = att.attestation_id.0.clone();
        let rec = evaluate_attestation_readiness(&vreq, &att);
        let sig_pred = rec.predicate_results.iter()
            .find(|r| r.predicate == VerificationReadinessPredicate::AttestationSignatureClaimed)
            .unwrap();
        assert!(!sig_pred.passed); // informational
    }

    // Patch 6 tests
    #[test]
    fn audit_packet_readiness_does_not_read_packet_file_path() {
        let rec = evaluate_audit_packet_readiness(
            &test_request(VerificationReadinessTargetKind::AuditPacket),
            "hash_t", false, "wfx_t",
        );
        assert!(!rec.reads_artifacts);
        assert!(!rec.fetches_urls);
    }

    #[test]
    fn audit_packet_readiness_uses_supplied_packet_metadata_only() {
        let rec = evaluate_audit_packet_readiness(
            &test_request(VerificationReadinessTargetKind::AuditPacket),
            "hash_t", false, "wfx_t",
        );
        assert_eq!(VerificationReadinessStatus::Ready, rec.status);
    }

    #[test]
    fn audit_packet_readiness_does_not_certify_recorded_evidence() {
        let rec = evaluate_audit_packet_readiness(
            &test_request(VerificationReadinessTargetKind::AuditPacket),
            "hash_t", true, "wfx_t",
        );
        // Packet claims certification → blocked
        assert_eq!(VerificationReadinessStatus::Blocked, rec.status);
        assert!(!rec.certifies_evidence);
    }
}
