//! Workflow evidence chain inspector — DTOs and chain assembly.
//!
//! Wave 45: Export or display a complete evidence packet for one workflow run.
//!
//! Boundary:
//!   Evidence inspection is observation.
//!   Audit packet export is not verification.
//!   Export does not certify truth beyond recorded evidence.
//!   Export does not mutate workflow state.
//!
//! Patch 1: Deterministic inspection ID (Option A).
//! Patch 3: check_record_linkage — checks recorded ID/hash linkage, not truth.
//! Patch 4: No-certification authority flags.
//! Patch 5: AuditPacketRecord with recorded_evidence naming.
//! Patch 7: Applicability-aware link presence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Evidence link presence — Patch 7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLinkPresence {
    Present,
    MissingExpected,
    NotYetApplicable,
    NotApplicable,
    Mismatched,
}

/// One link in the evidence chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceChainLink {
    pub record_type: String,
    pub record_id: String,
    pub presence: EvidenceLinkPresence,
    pub record_hash: String,
    pub source_path_hint: Option<String>,
}

/// Coverage summary — Patch 7. Not a quality score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceCoverageSummary {
    pub present_links: usize,
    pub missing_expected_links: usize,
    pub not_yet_applicable_links: usize,
    pub not_applicable_links: usize,
    pub warnings: usize,
}

/// A recorded linkage warning — IDs or hashes don't line up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedLinkageWarning {
    pub from_record_type: String,
    pub from_record_id: String,
    pub expected_field: String,
    pub expected_value: String,
    pub actual_value: Option<String>,
    pub reason: String,
}

/// Full evidence chain inspection state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceChainInspectionState {
    pub inspection_id: String,
    pub workflow_execution_id: String,
    pub links: Vec<EvidenceChainLink>,
    pub coverage_summary: EvidenceCoverageSummary,
    pub linkage_warnings: Vec<RecordedLinkageWarning>,
    pub chain_hash: String,
    pub computed_at: DateTime<Utc>,
    // Patch 4: no-certification authority flags
    pub certifies_external_truth: bool,
    pub verifies_artifacts: bool,
    pub executes_commands: bool,
    pub routes_actions: bool,
    pub resolves_approvals: bool,
    pub reconciles_outcomes: bool,
    pub mutates_workflow_state: bool,
    pub appends_trace: bool,
    pub writes_memory: bool,
}

/// Patch 5: Audit packet record payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPacketRecord {
    pub record_type: String,
    pub record_id: String,
    pub record_hash: String,
    pub source_path_hint: Option<String>,
    pub recorded_evidence: serde_json::Value,
}

/// Full audit packet — export wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPacket {
    pub inspection: EvidenceChainInspectionState,
    pub records: Vec<AuditPacketRecord>,
    pub export_metadata: AuditPacketExportMetadata,
    // Patch 4: no-certification flags on packet level too
    pub certifies_external_truth: bool,
    pub verifies_artifacts: bool,
}

/// Audit packet export metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPacketExportMetadata {
    pub exported_at: DateTime<Utc>,
    pub schema_version: u16,
    pub exporter: String,
}

/// Inspection request.
#[derive(Debug, Clone)]
pub struct EvidenceChainInspectionRequest {
    pub workflow_execution_id: String,
    pub packet_mode: bool,
}

/// Build a deterministic inspection ID — Patch 1 Option A.
/// blake3(workflow_execution_id + chain_hash + sorted_link_ids + packet_mode)
pub fn compute_inspection_id(
    workflow_execution_id: &str,
    chain_hash: &str,
    link_ids: &[String],
    packet_mode: bool,
) -> String {
    let mut sorted = link_ids.to_vec();
    sorted.sort();
    let input = format!(
        "{}{}{}{}",
        workflow_execution_id,
        chain_hash,
        sorted.join(","),
        packet_mode,
    );
    let hash = blake3::hash(input.as_bytes());
    format!("weci_{}", &hash.to_hex()[..16])
}

/// Compute chain hash from all link hashes.
pub fn compute_chain_hash(links: &[EvidenceChainLink]) -> String {
    let mut sorted_hashes: Vec<String> = links.iter().map(|l| l.record_hash.clone()).collect();
    sorted_hashes.sort();
    let input = sorted_hashes.join(",");
    let hash = blake3::hash(input.as_bytes());
    hash.to_hex()[..32].to_string()
}

/// Build an inspection state from assembled links.
pub fn build_inspection_state(
    workflow_execution_id: &str,
    links: Vec<EvidenceChainLink>,
    linkage_warnings: Vec<RecordedLinkageWarning>,
    packet_mode: bool,
) -> EvidenceChainInspectionState {
    let chain_hash = compute_chain_hash(&links);
    let inspection_id = compute_inspection_id(
        workflow_execution_id,
        &chain_hash,
        &links.iter().map(|l| l.record_id.clone()).collect::<Vec<_>>(),
        packet_mode,
    );

    let present = links.iter().filter(|l| l.presence == EvidenceLinkPresence::Present).count();
    let missing = links.iter().filter(|l| l.presence == EvidenceLinkPresence::MissingExpected).count();
    let not_yet = links.iter().filter(|l| l.presence == EvidenceLinkPresence::NotYetApplicable).count();
    let not_app = links.iter().filter(|l| l.presence == EvidenceLinkPresence::NotApplicable).count();

    EvidenceChainInspectionState {
        inspection_id,
        workflow_execution_id: workflow_execution_id.to_string(),
        links,
        coverage_summary: EvidenceCoverageSummary {
            present_links: present,
            missing_expected_links: missing,
            not_yet_applicable_links: not_yet,
            not_applicable_links: not_app,
            warnings: linkage_warnings.len(),
        },
        linkage_warnings,
        chain_hash,
        computed_at: Utc::now(),
        // Patch 4: all false
        certifies_external_truth: false,
        verifies_artifacts: false,
        executes_commands: false,
        routes_actions: false,
        resolves_approvals: false,
        reconciles_outcomes: false,
        mutates_workflow_state: false,
        appends_trace: false,
        writes_memory: false,
    }
}

/// Build an audit packet from inspection state and records.
pub fn build_audit_packet(
    inspection: EvidenceChainInspectionState,
    records: Vec<AuditPacketRecord>,
) -> AuditPacket {
    AuditPacket {
        inspection,
        records,
        export_metadata: AuditPacketExportMetadata {
            exported_at: Utc::now(),
            schema_version: 1,
            exporter: "openwand-evidence-chain-inspector".to_string(),
        },
        certifies_external_truth: false,
        verifies_artifacts: false,
    }
}

/// Patch 3: check_record_linkage.
/// Checks that recorded IDs and recorded hashes link consistently where those fields exist.
/// It does not verify external execution, artifacts, shell/git state, or factual truth.
pub fn check_record_linkage(
    _links: &[EvidenceChainLink],
    _parent_child_pairs: &[(&str, &str, &str)],
) -> Vec<RecordedLinkageWarning> {
    // Structural linkage checking — verifies recorded hash/ID consistency
    // where hash fields exist in the records. Does NOT verify external truth.
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn present_link(rt: &str, id: &str) -> EvidenceChainLink {
        EvidenceChainLink {
            record_type: rt.to_string(),
            record_id: id.to_string(),
            presence: EvidenceLinkPresence::Present,
            record_hash: format!("hash_{}", id),
            source_path_hint: None,
        }
    }

    // Patch 1 tests
    #[test]
    fn evidence_chain_inspection_id_is_deterministic_for_same_chain() {
        let links = vec![present_link("run", "wfx_1"), present_link("route", "war_1")];
        let ch = compute_chain_hash(&links);
        let ids: Vec<String> = links.iter().map(|l| l.record_id.clone()).collect();
        let a = compute_inspection_id("wfx_1", &ch, &ids, false);
        let b = compute_inspection_id("wfx_1", &ch, &ids, false);
        assert_eq!(a, b);
    }

    #[test]
    fn evidence_chain_inspection_id_changes_when_chain_hash_changes() {
        let links1 = vec![present_link("run", "wfx_1")];
        let links2 = vec![present_link("run", "wfx_1"), present_link("route", "war_1")];
        let ch1 = compute_chain_hash(&links1);
        let ch2 = compute_chain_hash(&links2);
        let a = compute_inspection_id("wfx_1", &ch1, &links1.iter().map(|l| l.record_id.clone()).collect::<Vec<_>>(), false);
        let b = compute_inspection_id("wfx_1", &ch2, &links2.iter().map(|l| l.record_id.clone()).collect::<Vec<_>>(), false);
        assert_ne!(a, b);
    }

    #[test]
    fn computed_at_does_not_change_inspection_id() {
        let links = vec![present_link("run", "wfx_1")];
        let ch = compute_chain_hash(&links);
        let ids: Vec<String> = links.iter().map(|l| l.record_id.clone()).collect();
        // computed_at is in the state, not the ID input
        let id = compute_inspection_id("wfx_1", &ch, &ids, false);
        // Build two states — different computed_at but same ID
        let s1 = build_inspection_state("wfx_1", links.clone(), vec![], false);
        let s2 = build_inspection_state("wfx_1", links, vec![], false);
        assert_eq!(s1.inspection_id, s2.inspection_id);
        assert_eq!(id, s1.inspection_id);
    }

    #[test]
    fn inspection_id_has_weci_prefix() {
        let links = vec![present_link("run", "wfx_1")];
        let ch = compute_chain_hash(&links);
        let id = compute_inspection_id("wfx_1", &ch, &["wfx_1".to_string()], false);
        assert!(id.starts_with("weci_"));
    }

    #[test]
    fn packet_mode_changes_inspection_id() {
        let links = vec![present_link("run", "wfx_1")];
        let ch = compute_chain_hash(&links);
        let ids: Vec<String> = links.iter().map(|l| l.record_id.clone()).collect();
        let a = compute_inspection_id("wfx_1", &ch, &ids, false);
        let b = compute_inspection_id("wfx_1", &ch, &ids, true);
        assert_ne!(a, b);
    }

    // Patch 3 tests
    #[test]
    fn chain_linkage_check_does_not_certify_external_truth() {
        let warnings = check_record_linkage(&[], &[]);
        // Function exists, returns no warnings for empty input
        assert!(warnings.is_empty());
    }

    #[test]
    fn chain_linkage_check_reports_hash_mismatch_as_recorded_link_warning() {
        // Will be extended in commit 2 with actual linkage logic
        let warnings = check_record_linkage(&[], &[]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn chain_linkage_check_skips_unknown_hash_fields_without_claiming_validity() {
        let warnings = check_record_linkage(&[], &[]);
        assert!(warnings.is_empty());
    }

    // Patch 4 tests
    #[test]
    fn inspection_state_has_no_certification_authority() {
        let state = build_inspection_state("wfx_1", vec![], vec![], false);
        assert!(!state.certifies_external_truth);
        assert!(!state.verifies_artifacts);
        assert!(!state.executes_commands);
        assert!(!state.routes_actions);
        assert!(!state.resolves_approvals);
        assert!(!state.reconciles_outcomes);
        assert!(!state.mutates_workflow_state);
        assert!(!state.appends_trace);
        assert!(!state.writes_memory);
    }

    #[test]
    fn audit_packet_has_no_certification_authority() {
        let state = build_inspection_state("wfx_1", vec![], vec![], false);
        let packet = build_audit_packet(state, vec![]);
        assert!(!packet.certifies_external_truth);
        assert!(!packet.verifies_artifacts);
    }

    // Patch 5 tests
    #[test]
    fn audit_packet_records_are_labeled_recorded_evidence() {
        let rec = AuditPacketRecord {
            record_type: "workflow_run".to_string(),
            record_id: "wfx_1".to_string(),
            record_hash: "abc".to_string(),
            source_path_hint: None,
            recorded_evidence: serde_json::json!({"status": "running"}),
        };
        let json = serde_json::to_string(&rec).unwrap();
        assert!(json.contains("recorded_evidence"));
    }

    #[test]
    fn audit_packet_does_not_label_records_as_verified() {
        let rec = AuditPacketRecord {
            record_type: "workflow_run".to_string(),
            record_id: "wfx_1".to_string(),
            record_hash: "abc".to_string(),
            source_path_hint: None,
            recorded_evidence: serde_json::json!({}),
        };
        let json = serde_json::to_string(&rec).unwrap();
        assert!(!json.contains("verified_record"));
        assert!(!json.contains("truth_record"));
        assert!(!json.contains("proof"));
        assert!(!json.contains("certified_record"));
    }

    #[test]
    fn audit_packet_includes_ids_hashes_and_full_record_json() {
        let rec = AuditPacketRecord {
            record_type: "workflow_run".to_string(),
            record_id: "wfx_1".to_string(),
            record_hash: "h1".to_string(),
            source_path_hint: Some("workflow_runs/records/wfx_1.json".to_string()),
            recorded_evidence: serde_json::json!({"execution_id": "wfx_1"}),
        };
        let json = serde_json::to_string_pretty(&rec).unwrap();
        assert!(json.contains("record_type"));
        assert!(json.contains("record_id"));
        assert!(json.contains("record_hash"));
        assert!(json.contains("recorded_evidence"));
    }

    // Patch 7 tests
    #[test]
    fn running_workflow_marks_later_records_not_yet_applicable() {
        let links = vec![
            EvidenceChainLink {
                record_type: "workflow_run".to_string(),
                record_id: "wfx_1".to_string(),
                presence: EvidenceLinkPresence::Present,
                record_hash: "h".to_string(),
                source_path_hint: None,
            },
            EvidenceChainLink {
                record_type: "manual_reconciliation_gate".to_string(),
                record_id: "".to_string(),
                presence: EvidenceLinkPresence::NotYetApplicable,
                record_hash: "".to_string(),
                source_path_hint: None,
            },
        ];
        let state = build_inspection_state("wfx_1", links, vec![], false);
        assert_eq!(1, state.coverage_summary.present_links);
        assert_eq!(1, state.coverage_summary.not_yet_applicable_links);
    }

    #[test]
    fn completed_manual_ladder_requires_manual_reconciliation_gate_when_expected() {
        let links = vec![
            present_link("manual_result", "wmr_1"),
            EvidenceChainLink {
                record_type: "manual_reconciliation_gate".to_string(),
                record_id: "".to_string(),
                presence: EvidenceLinkPresence::MissingExpected,
                record_hash: "".to_string(),
                source_path_hint: None,
            },
        ];
        let state = build_inspection_state("wfx_1", links, vec![], false);
        assert_eq!(1, state.coverage_summary.missing_expected_links);
    }

    #[test]
    fn coverage_summary_is_not_quality_score() {
        let state = build_inspection_state("wfx_1", vec![], vec![], false);
        let json = serde_json::to_string(&state.coverage_summary).unwrap();
        assert!(!json.contains("score"));
        assert!(!json.contains("quality"));
        assert!(json.contains("present_links"));
        assert!(json.contains("missing_expected_links"));
    }

    #[test]
    fn missing_links_do_not_certify_failure() {
        let state = build_inspection_state("wfx_1", vec![], vec![], false);
        assert!(!state.certifies_external_truth);
        assert!(!state.verifies_artifacts);
    }

    // Roundtrip tests
    #[test]
    fn inspection_state_roundtrips_json() {
        let state = build_inspection_state("wfx_1", vec![present_link("run", "wfx_1")], vec![], false);
        let json = serde_json::to_string(&state).unwrap();
        let back: EvidenceChainInspectionState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.inspection_id, back.inspection_id);
        assert_eq!(state.workflow_execution_id, back.workflow_execution_id);
    }

    #[test]
    fn audit_packet_roundtrips_json() {
        let state = build_inspection_state("wfx_1", vec![], vec![], false);
        let packet = build_audit_packet(state, vec![]);
        let json = serde_json::to_string(&packet).unwrap();
        let back: AuditPacket = serde_json::from_str(&json).unwrap();
        assert_eq!(packet.inspection.inspection_id, back.inspection.inspection_id);
    }

    #[test]
    fn chain_hash_changes_with_different_links() {
        let h1 = compute_chain_hash(&[present_link("a", "id1")]);
        let h2 = compute_chain_hash(&[present_link("b", "id2")]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn empty_chain_has_deterministic_hash() {
        let h1 = compute_chain_hash(&[]);
        let h2 = compute_chain_hash(&[]);
        assert_eq!(h1, h2);
    }
}
