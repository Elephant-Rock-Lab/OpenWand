//! Audit packet distribution — metadata-only record of packet distribution.
//!
//! Wave 49A: Records that an audit packet was reportedly distributed somewhere.
//! Distribution does not prove delivery, receipt, acceptance, or correctness.
//! OpenWand does not send email, upload files, call APIs, verify paths, or
//! fetch URLs.
//!
//! Patch 2: Binds to review_hash + packet_hash + chain_hash + inspection_id.
//! Patch 4: Reported distribution only semantics.
//! Patch 5: Metadata-only destination model.
//! Patch 8: 14 no-authority flags, all false.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_run::WorkflowExecutionId;
use crate::workflow_audit_packet_review::AuditPacketReviewId;

/// Audit packet distribution ID. Content-addressed with `wapd_` prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditPacketDistributionId(pub String);

/// Destination kind — Patch 5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditPacketDestinationKind {
    FileShare,
    Email,
    Archive,
    Other,
}

/// Metadata-only destination — Patch 5.
/// OpenWand does not send, upload, verify, fetch, or confirm delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPacketDistributionDestination {
    pub destination_kind: AuditPacketDestinationKind,
    pub label: String,
    pub reference: String,
    pub operator_supplied_hash: Option<String>,
    pub notes: Vec<String>,
}

/// Audit packet distribution request.
#[derive(Debug, Clone)]
pub struct AuditPacketDistributionRequest {
    pub review_id: AuditPacketReviewId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub expected_review_hash: String,
    pub audit_packet_hash: String,
    pub chain_hash: String,
    pub inspection_id: String,
    pub destination: AuditPacketDistributionDestination,
    pub distribution_notes: Vec<String>,
    pub idempotency_key: String,
}

/// Audit packet distribution record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPacketDistribution {
    pub distribution_id: AuditPacketDistributionId,
    pub review_id: AuditPacketReviewId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub expected_review_hash: String,
    pub audit_packet_hash: String,
    pub chain_hash: String,
    pub inspection_id: String,
    pub destination: AuditPacketDistributionDestination,
    pub distribution_notes: Vec<String>,
    pub idempotency_key: String,
    pub distributed_at: DateTime<Utc>,
    // Patch 4: reported-distribution-only semantics
    pub reported_distribution: bool,
    pub proof_of_delivery: bool,
    pub recipient_acceptance_proven: bool,
    pub destination_verified: bool,
    pub external_system_integrated: bool,
    // Patch 8: no-authority flags — all false
    pub certifies_external_truth: bool,
    pub verifies_packet_contents: bool,
    pub modifies_audit_packet: bool,
    pub mutates_workflow_state: bool,
    pub promotes_trust: bool,
    pub executes_commands: bool,
    pub sends_external_message: bool,
    pub uploads_files: bool,
    pub appends_trace: bool,
    pub writes_memory: bool,
    pub creates_execution_grant: bool,
    pub execution_allowed_now: bool,
}

/// Build a content-addressed distribution ID: wapd_<blake3(request fields)>
pub fn compute_distribution_id(request: &AuditPacketDistributionRequest) -> AuditPacketDistributionId {
    let input = format!(
        "{}{}{}{}{}{}{}{}",
        request.review_id.0,
        request.workflow_execution_id.0,
        request.expected_review_hash,
        request.audit_packet_hash,
        request.chain_hash,
        request.inspection_id,
        request.destination.label,
        request.idempotency_key,
    );
    let hash = blake3::hash(input.as_bytes());
    AuditPacketDistributionId(format!("wapd_{}", &hash.to_hex()[..16]))
}

/// Build a distribution record from a request.
/// Patch 2: Binds to review_hash, packet_hash, chain_hash, inspection_id.
pub fn build_audit_packet_distribution(request: AuditPacketDistributionRequest) -> AuditPacketDistribution {
    let distribution_id = compute_distribution_id(&request);
    AuditPacketDistribution {
        distribution_id,
        review_id: request.review_id,
        workflow_execution_id: request.workflow_execution_id,
        expected_review_hash: request.expected_review_hash,
        audit_packet_hash: request.audit_packet_hash,
        chain_hash: request.chain_hash,
        inspection_id: request.inspection_id,
        destination: request.destination,
        distribution_notes: request.distribution_notes,
        idempotency_key: request.idempotency_key,
        distributed_at: Utc::now(),
        // Patch 4: reported distribution only
        reported_distribution: true,
        proof_of_delivery: false,
        recipient_acceptance_proven: false,
        destination_verified: false,
        external_system_integrated: false,
        // Patch 8: all false
        certifies_external_truth: false,
        verifies_packet_contents: false,
        modifies_audit_packet: false,
        mutates_workflow_state: false,
        promotes_trust: false,
        executes_commands: false,
        sends_external_message: false,
        uploads_files: false,
        appends_trace: false,
        writes_memory: false,
        creates_execution_grant: false,
        execution_allowed_now: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_destination() -> AuditPacketDistributionDestination {
        AuditPacketDistributionDestination {
            destination_kind: AuditPacketDestinationKind::FileShare,
            label: "Shared drive".into(),
            reference: "\\\\server\\audit\\packet.json".into(),
            operator_supplied_hash: Some("op_hash_123".into()),
            notes: vec!["Q1 audit archive".into()],
        }
    }

    fn test_request() -> AuditPacketDistributionRequest {
        AuditPacketDistributionRequest {
            review_id: AuditPacketReviewId("wapr_test".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_test".into()),
            expected_review_hash: "review_hash_789".into(),
            audit_packet_hash: "pkt_hash_123".into(),
            chain_hash: "chain_hash_456".into(),
            inspection_id: "weci_test".into(),
            destination: test_destination(),
            distribution_notes: vec!["Distributed to archive".into()],
            idempotency_key: "dkey1".into(),
        }
    }

    #[test]
    fn distribution_id_has_wapd_prefix() {
        let rec = build_audit_packet_distribution(test_request());
        assert!(rec.distribution_id.0.starts_with("wapd_"));
    }

    #[test]
    fn distribution_id_is_deterministic() {
        let id1 = compute_distribution_id(&test_request());
        let id2 = compute_distribution_id(&test_request());
        assert_eq!(id1, id2);
    }

    #[test]
    fn distribution_id_changes_on_different_inputs() {
        let mut req = test_request();
        let id1 = compute_distribution_id(&req);
        req.idempotency_key = "different".into();
        let id2 = compute_distribution_id(&req);
        assert_ne!(id1, id2);
    }

    #[test]
    fn distribution_roundtrips_json() {
        let rec = build_audit_packet_distribution(test_request());
        let json = serde_json::to_string(&rec).unwrap();
        let back: AuditPacketDistribution = serde_json::from_str(&json).unwrap();
        assert_eq!(rec.distribution_id, back.distribution_id);
    }

    // Patch 2: binds to review hash and packet hash
    #[test]
    fn distribution_binds_to_review_hash() {
        let rec = build_audit_packet_distribution(test_request());
        assert_eq!("review_hash_789", rec.expected_review_hash);
    }

    #[test]
    fn distribution_copies_packet_and_chain_hashes() {
        let rec = build_audit_packet_distribution(test_request());
        assert_eq!("pkt_hash_123", rec.audit_packet_hash);
        assert_eq!("chain_hash_456", rec.chain_hash);
    }

    #[test]
    fn distribution_binds_to_inspection_id() {
        let rec = build_audit_packet_distribution(test_request());
        assert_eq!("weci_test", rec.inspection_id);
    }

    #[test]
    fn distribution_does_not_modify_review_or_packet() {
        let rec = build_audit_packet_distribution(test_request());
        assert!(!rec.modifies_audit_packet);
    }

    // Patch 4: reported distribution only
    #[test]
    fn distribution_records_reported_distribution_only() {
        let rec = build_audit_packet_distribution(test_request());
        assert!(rec.reported_distribution);
    }

    #[test]
    fn distribution_does_not_prove_delivery() {
        let rec = build_audit_packet_distribution(test_request());
        assert!(!rec.proof_of_delivery);
    }

    #[test]
    fn distribution_does_not_prove_recipient_acceptance() {
        let rec = build_audit_packet_distribution(test_request());
        assert!(!rec.recipient_acceptance_proven);
    }

    #[test]
    fn distribution_does_not_verify_destination() {
        let rec = build_audit_packet_distribution(test_request());
        assert!(!rec.destination_verified);
    }

    // Patch 5: metadata-only destination
    #[test]
    fn distribution_destination_is_metadata_only() {
        let dest = test_destination();
        assert_eq!(AuditPacketDestinationKind::FileShare, dest.destination_kind);
        assert_eq!("Shared drive", dest.label);
    }

    #[test]
    fn distribution_operator_supplied_hash_is_stored_verbatim() {
        let rec = build_audit_packet_distribution(test_request());
        assert_eq!(Some("op_hash_123".into()), rec.destination.operator_supplied_hash);
    }

    #[test]
    fn distribution_does_not_send_email() {
        let rec = build_audit_packet_distribution(test_request());
        assert!(!rec.sends_external_message);
    }

    #[test]
    fn distribution_does_not_upload_or_archive_packet() {
        let rec = build_audit_packet_distribution(test_request());
        assert!(!rec.uploads_files);
        assert!(!rec.external_system_integrated);
    }

    // Patch 8: no-authority flags
    #[test]
    fn distribution_has_no_authority_flags() {
        let rec = build_audit_packet_distribution(test_request());
        assert!(!rec.certifies_external_truth);
        assert!(!rec.verifies_packet_contents);
        assert!(!rec.modifies_audit_packet);
        assert!(!rec.mutates_workflow_state);
        assert!(!rec.promotes_trust);
        assert!(!rec.executes_commands);
        assert!(!rec.sends_external_message);
        assert!(!rec.uploads_files);
        assert!(!rec.appends_trace);
        assert!(!rec.writes_memory);
        assert!(!rec.creates_execution_grant);
        assert!(!rec.execution_allowed_now);
    }

    #[test]
    fn distribution_serialized_contains_no_delivery_proof_fields() {
        let rec = build_audit_packet_distribution(test_request());
        let json = serde_json::to_string(&rec).unwrap().to_lowercase();
        assert!(json.contains("\"proof_of_delivery\":false"));
        assert!(json.contains("\"recipient_acceptance_proven\":false"));
        assert!(json.contains("\"destination_verified\":false"));
    }

    #[test]
    fn all_four_destination_kinds_serialize() {
        let kinds = vec![
            AuditPacketDestinationKind::FileShare,
            AuditPacketDestinationKind::Email,
            AuditPacketDestinationKind::Archive,
            AuditPacketDestinationKind::Other,
        ];
        for k in &kinds {
            let json = serde_json::to_string(k).unwrap();
            let back: AuditPacketDestinationKind = serde_json::from_str(&json).unwrap();
            assert_eq!(*k, back);
        }
    }

    #[test]
    fn distribution_preserves_notes() {
        let rec = build_audit_packet_distribution(test_request());
        assert_eq!(1, rec.distribution_notes.len());
    }
}
