//! Workflow external attestation model — DTOs and builder.
//!
//! Wave 46: Attach structured third-party/external attestations to evidence records.
//!
//! Boundary:
//!   External attestation is reported evidence.
//!   It is not verification.
//!   It is not trust promotion.
//!   It is not reconciliation.
//!   It does not certify external truth.
//!
//! Patch 1: ExternalAttestationReference list (metadata-only).
//! Patch 2: ExternalAttestationTarget enum with typed IDs.
//! Patch 3: ExternalReportedSignature (unverified semantics).
//! Patch 4: No trust scoring, no confidence, no promotion fields.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_run::WorkflowExecutionId;

/// Content-addressed attestation ID. Format: `watt_<blake3_hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowExternalAttestationId(pub String);

impl WorkflowExternalAttestationId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What kind of external attestation is this?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalAttestationKind {
    ThirdPartySignoff,
    CiPipelineResult,
    CodeReviewApproval,
    AuditLogEntry,
    ExternalSignature,
    ManualSignoff,
    Other,
}

/// Patch 2: What evidence record type does this attestation attach to?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalAttestationTargetKind {
    WorkflowRun,
    WorkflowRunRevision,
    ManualResult,
    ManualResultReview,
    ManualResultReconciliationReadiness,
    ManualResultReconciliationGate,
    EvidenceChainInspection,
    AuditPacket,
    Reconciliation,
    ActionOutcome,
    Other,
}

/// Patch 2: Attachment target — which record is being attested.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAttestationTarget {
    pub target_kind: ExternalAttestationTargetKind,
    pub target_id: String,
    pub workflow_execution_id: WorkflowExecutionId,
    pub expected_target_hash: Option<String>,
}

/// Who or what produced this attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAttestationSource {
    pub name: String,
    pub role: String,
    pub system_identifier: Option<String>,
}

/// Patch 1: A single reference within an attestation — metadata-only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAttestationReference {
    pub reference_id: String,
    pub label: String,
    pub kind: ExternalAttestationReferenceKind,
    /// Metadata-only reference string. Not read, fetched, or verified by OpenWand.
    pub reference: String,
    /// Stored verbatim. OpenWand never recomputes from file/URL contents.
    pub operator_supplied_hash: Option<String>,
    pub description: Option<String>,
}

/// Reference kind mirrors artifact semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalAttestationReferenceKind {
    ExternalUrl,
    FilePathReference,
    LogExcerpt,
    PlainTextNote,
    Screenshot,
    SignatureBlock,
    Other,
}

/// Patch 3: Reported signature — unverified semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalReportedSignature {
    pub signature_text_or_reference: String,
    pub claimed_signer: Option<String>,
    pub claimed_algorithm: Option<String>,
    pub verification_status: ExternalSignatureVerificationStatus,
}

/// Patch 3: Signature verification status — only one variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalSignatureVerificationStatus {
    NotVerifiedByOpenWand,
}

/// The core external attestation DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExternalAttestation {
    pub attestation_id: WorkflowExternalAttestationId,
    pub target: ExternalAttestationTarget,
    pub kind: ExternalAttestationKind,
    pub source: ExternalAttestationSource,
    pub claim: String,
    /// Patch 1: multiple references, all metadata-only.
    pub references: Vec<ExternalAttestationReference>,
    /// Patch 3: optional reported signature — unverified.
    pub reported_signature: Option<ExternalReportedSignature>,
    pub attested_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub idempotency_key: String,
    // Patch 4: authority/verification flags — all hardcoded false
    pub reported_by_operator: bool,
    pub verified_by_openwand: bool,
    pub promotes_trust: bool,
    pub certifies_external_truth: bool,
    pub mutates_workflow_state: bool,
    pub reconciles_outcome: bool,
    pub creates_execution_grant: bool,
    pub execution_allowed_now: bool,
}

/// Request to attach an external attestation.
#[derive(Debug, Clone)]
pub struct ExternalAttestationRequest {
    pub workflow_execution_id: WorkflowExecutionId,
    pub target_kind: ExternalAttestationTargetKind,
    pub target_id: String,
    pub expected_target_hash: Option<String>,
    pub kind: ExternalAttestationKind,
    pub source_name: String,
    pub source_role: String,
    pub source_system_identifier: Option<String>,
    pub claim: String,
    pub references: Vec<ExternalAttestationReference>,
    pub reported_signature: Option<ExternalReportedSignature>,
    pub attested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

/// Build a content-addressed attestation ID.
pub fn compute_attestation_id(
    workflow_execution_id: &str,
    target_id: &str,
    claim: &str,
    source_name: &str,
    idempotency_key: &str,
) -> WorkflowExternalAttestationId {
    let input = format!("{}{}{}{}{}", workflow_execution_id, target_id, claim, source_name, idempotency_key);
    let hash = blake3::hash(input.as_bytes());
    WorkflowExternalAttestationId(format!("watt_{}", &hash.to_hex()[..16]))
}

/// Build an attestation from a validated request.
pub fn build_external_attestation(request: ExternalAttestationRequest) -> WorkflowExternalAttestation {
    let attestation_id = compute_attestation_id(
        request.workflow_execution_id.0.as_str(),
        request.target_id.as_str(),
        request.claim.as_str(),
        request.source_name.as_str(),
        request.idempotency_key.as_str(),
    );
    WorkflowExternalAttestation {
        attestation_id,
        target: ExternalAttestationTarget {
            target_kind: request.target_kind,
            target_id: request.target_id,
            workflow_execution_id: request.workflow_execution_id,
            expected_target_hash: request.expected_target_hash,
        },
        kind: request.kind,
        source: ExternalAttestationSource {
            name: request.source_name,
            role: request.source_role,
            system_identifier: request.source_system_identifier,
        },
        claim: request.claim,
        references: request.references,
        reported_signature: request.reported_signature,
        attested_at: request.attested_at,
        recorded_at: Utc::now(),
        idempotency_key: request.idempotency_key,
        // Patch 4: all false
        reported_by_operator: true,
        verified_by_openwand: false,
        promotes_trust: false,
        certifies_external_truth: false,
        mutates_workflow_state: false,
        reconciles_outcome: false,
        creates_execution_grant: false,
        execution_allowed_now: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_request() -> ExternalAttestationRequest {
        ExternalAttestationRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            target_kind: ExternalAttestationTargetKind::ManualResult,
            target_id: "wmr_t".into(),
            expected_target_hash: None,
            kind: ExternalAttestationKind::ThirdPartySignoff,
            source_name: "Alice".into(),
            source_role: "reviewer".into(),
            source_system_identifier: None,
            claim: "Code reviewed and approved".into(),
            references: vec![],
            reported_signature: None,
            attested_at: Utc::now(),
            idempotency_key: "key1".into(),
        }
    }

    // Patch 1 tests
    #[test]
    fn attestation_references_are_metadata_only() {
        let ref_ = ExternalAttestationReference {
            reference_id: "r1".into(),
            label: "CI log".into(),
            kind: ExternalAttestationReferenceKind::ExternalUrl,
            reference: "https://ci.example.com/build/123".into(),
            operator_supplied_hash: Some("deadbeef".into()),
            description: Some("CI build log".into()),
        };
        // Reference is a string, not file contents
        assert!(ref_.reference.starts_with("http"));
        assert!(ref_.operator_supplied_hash.is_some());
    }

    #[test]
    fn attestation_does_not_read_reference_files() {
        let src = include_str!("workflow_external_attestation.rs");
        // No file I/O functions
        let pub_fns: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn")).collect();
        assert!(!pub_fns.iter().any(|l| l.contains("read_file") || l.contains("std::fs::read")));
    }

    #[test]
    fn attestation_does_not_fetch_reference_urls() {
        let src = include_str!("workflow_external_attestation.rs");
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("reqwest") || l.contains("http")));
    }

    #[test]
    fn operator_supplied_attestation_hash_is_stored_verbatim() {
        let mut req = test_request();
        req.references.push(ExternalAttestationReference {
            reference_id: "r1".into(),
            label: "log".into(),
            kind: ExternalAttestationReferenceKind::LogExcerpt,
            reference: "/var/log/build.log".into(),
            operator_supplied_hash: Some("sha256:abc123".into()),
            description: None,
        });
        let att = build_external_attestation(req);
        assert_eq!(Some("sha256:abc123".to_string()), att.references[0].operator_supplied_hash);
    }

    #[test]
    fn attestation_serialized_json_contains_no_file_bytes() {
        let att = build_external_attestation(test_request());
        let json = serde_json::to_string(&att).unwrap();
        assert!(!json.contains("file_bytes"));
        assert!(!json.contains("base64"));
    }

    // Patch 2 tests
    #[test]
    fn attestation_target_kind_serializes_snake_case() {
        let kind = ExternalAttestationTargetKind::ManualResult;
        let json = serde_json::to_string(&kind).unwrap();
        assert!(json.contains("manual_result"));
    }

    #[test]
    fn attestation_target_has_workflow_execution_id() {
        let att = build_external_attestation(test_request());
        assert_eq!("wfx_t", att.target.workflow_execution_id.0);
    }

    // Patch 3 tests
    #[test]
    fn reported_signature_is_not_verified_by_openwand() {
        let sig = ExternalReportedSignature {
            signature_text_or_reference: "signed-by:alice".into(),
            claimed_signer: Some("alice".into()),
            claimed_algorithm: Some("ed25519".into()),
            verification_status: ExternalSignatureVerificationStatus::NotVerifiedByOpenWand,
        };
        assert_eq!(ExternalSignatureVerificationStatus::NotVerifiedByOpenWand, sig.verification_status);
    }

    #[test]
    fn reported_signature_does_not_promote_trust() {
        let mut req = test_request();
        req.reported_signature = Some(ExternalReportedSignature {
            signature_text_or_reference: "sig".into(),
            claimed_signer: None,
            claimed_algorithm: None,
            verification_status: ExternalSignatureVerificationStatus::NotVerifiedByOpenWand,
        });
        let att = build_external_attestation(req);
        assert!(att.reported_signature.is_some());
        assert!(!att.promotes_trust);
    }

    #[test]
    fn attestation_with_reported_signature_still_has_verified_false() {
        let mut req = test_request();
        req.reported_signature = Some(ExternalReportedSignature {
            signature_text_or_reference: "sig".into(),
            claimed_signer: None,
            claimed_algorithm: None,
            verification_status: ExternalSignatureVerificationStatus::NotVerifiedByOpenWand,
        });
        let att = build_external_attestation(req);
        assert!(!att.verified_by_openwand);
    }

    // Patch 4 tests
    #[test]
    fn external_attestation_has_no_trust_promotion_fields() {
        let att = build_external_attestation(test_request());
        let json = serde_json::to_string(&att).unwrap().to_lowercase();
        assert!(!json.contains("trust_score"));
        assert!(!json.contains("confidence"));
        assert!(!json.contains("authority_level"));
    }

    #[test]
    fn external_attestation_verified_by_openwand_is_false() {
        let att = build_external_attestation(test_request());
        assert!(!att.verified_by_openwand);
    }

    #[test]
    fn external_attestation_does_not_certify_external_truth() {
        let att = build_external_attestation(test_request());
        assert!(!att.certifies_external_truth);
    }

    #[test]
    fn external_attestation_does_not_create_execution_grant() {
        let att = build_external_attestation(test_request());
        assert!(!att.creates_execution_grant);
        assert!(!att.execution_allowed_now);
    }

    #[test]
    fn serialized_attestation_contains_no_trust_score_or_confidence() {
        let att = build_external_attestation(test_request());
        let json = serde_json::to_string(&att).unwrap().to_lowercase();
        assert!(!json.contains("trust_score"));
        assert!(!json.contains("confidence"));
        assert!(!json.contains("trusted"));
        assert!(!json.contains("promoted"));
        assert!(!json.contains("certified"));
    }

    // Basic roundtrip
    #[test]
    fn attestation_roundtrips_json() {
        let att = build_external_attestation(test_request());
        let json = serde_json::to_string(&att).unwrap();
        let back: WorkflowExternalAttestation = serde_json::from_str(&json).unwrap();
        assert_eq!(att.attestation_id, back.attestation_id);
    }

    #[test]
    fn attestation_id_has_watt_prefix() {
        let id = compute_attestation_id("wfx_1", "wmr_1", "claim", "alice", "key");
        assert!(id.0.starts_with("watt_"));
    }

    #[test]
    fn attestation_id_is_deterministic() {
        let a = compute_attestation_id("wfx_1", "wmr_1", "claim", "alice", "key");
        let b = compute_attestation_id("wfx_1", "wmr_1", "claim", "alice", "key");
        assert_eq!(a, b);
    }

    #[test]
    fn attestation_id_changes_with_different_inputs() {
        let a = compute_attestation_id("wfx_1", "wmr_1", "claim", "alice", "key");
        let b = compute_attestation_id("wfx_1", "wmr_1", "claim", "bob", "key");
        assert_ne!(a, b);
    }

    #[test]
    fn multiple_references_allowed() {
        let mut req = test_request();
        req.references.push(ExternalAttestationReference {
            reference_id: "r1".into(), label: "url1".into(),
            kind: ExternalAttestationReferenceKind::ExternalUrl,
            reference: "https://a.com".into(),
            operator_supplied_hash: None, description: None,
        });
        req.references.push(ExternalAttestationReference {
            reference_id: "r2".into(), label: "url2".into(),
            kind: ExternalAttestationReferenceKind::ExternalUrl,
            reference: "https://b.com".into(),
            operator_supplied_hash: None, description: None,
        });
        let att = build_external_attestation(req);
        assert_eq!(2, att.references.len());
    }
}
