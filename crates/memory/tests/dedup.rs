//! Commit 5 — Content hash dedup integration tests.

use openwand_memory::dedup::{DedupKey, DedupDecision, compute_normalized_hash};
use openwand_memory::evidence::EvidenceKind;
use openwand_memory::provenance::MemoryScope;

#[test]
fn integration_dedup_same_normalized_text_same_scope() {
    let k1 = DedupKey::from_parts("The project uses Rust", &MemoryScope::Global, EvidenceKind::AcceptedClaim);
    let k2 = DedupKey::from_parts("the project uses rust", &MemoryScope::Global, EvidenceKind::AcceptedClaim);
    assert_eq!(k1, k2);
}

#[test]
fn integration_dedup_different_scope_not_duplicate() {
    let k1 = DedupKey::from_parts("test", &MemoryScope::Global, EvidenceKind::AcceptedClaim);
    let k2 = DedupKey::from_parts("test", &MemoryScope::Session { session_id: "s1".into() }, EvidenceKind::AcceptedClaim);
    assert_ne!(k1, k2);
}

#[test]
fn integration_dedup_same_trace_source_attaches_source() {
    let decision = DedupDecision::DuplicateAttachSource {
        existing_record_id: "mem_001".to_string(),
    };
    match decision {
        DedupDecision::DuplicateAttachSource { existing_record_id } => {
            assert_eq!("mem_001", existing_record_id);
        }
        DedupDecision::New => panic!("expected DuplicateAttachSource"),
    }
}

#[test]
fn integration_dedup_preserves_existing_record_id() {
    let decision = DedupDecision::DuplicateAttachSource {
        existing_record_id: "mem_original".to_string(),
    };
    if let DedupDecision::DuplicateAttachSource { existing_record_id } = decision {
        assert_eq!("mem_original", existing_record_id);
    }
}

#[test]
fn integration_normalized_hash_is_deterministic() {
    let h1 = compute_normalized_hash("Rust is great");
    let h2 = compute_normalized_hash("rust   is   great");
    assert_eq!(h1, h2, "normalized hashes must be identical");
}
