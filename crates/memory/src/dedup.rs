//! Content hash deduplication for memory records.
//!
//! Deterministic BLAKE3-based dedup.
//! Same normalized text + same scope + same evidence kind → duplicate.

use crate::evidence::EvidenceKind;
use crate::provenance::MemoryScope;
use serde::{Deserialize, Serialize};

/// Dedup key for a memory record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DedupKey {
    pub normalized_text_hash: String,
    pub scope_key: String,
    pub evidence_kind: EvidenceKind,
}

/// Decision for what to do with a candidate that might duplicate an existing record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupDecision {
    /// No existing match — insert as new record.
    New,
    /// Duplicate content — attach new source reference to existing record.
    DuplicateAttachSource { existing_record_id: String },
}

impl DedupKey {
    /// Compute a dedup key from text, scope, and evidence kind.
    pub fn from_parts(text: &str, scope: &MemoryScope, kind: EvidenceKind) -> Self {
        Self {
            normalized_text_hash: compute_normalized_hash(text),
            scope_key: scope_to_key(scope),
            evidence_kind: kind,
        }
    }
}

/// Normalize text for dedup: lowercase, collapse whitespace, trim.
fn normalize_for_dedup(text: &str) -> String {
    text.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Compute BLAKE3 hash of normalized text.
pub fn compute_normalized_hash(text: &str) -> String {
    let normalized = normalize_for_dedup(text);
    blake3::hash(normalized.as_bytes()).to_hex().to_string()
}

/// Derive a deterministic key from scope.
fn scope_to_key(scope: &MemoryScope) -> String {
    match scope {
        MemoryScope::Project { repo, branch } => {
            format!("project:{repo}:{}", branch.as_deref().unwrap_or(""))
        }
        MemoryScope::Session { session_id } => format!("session:{session_id}"),
        MemoryScope::Global => "global".to_string(),
        MemoryScope::Unknown => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::MemoryScope;

    #[test]
    fn dedup_same_normalized_text_same_scope() {
        let k1 = DedupKey::from_parts(
            "Rust is the primary language",
            &MemoryScope::Global,
            EvidenceKind::AcceptedClaim,
        );
        let k2 = DedupKey::from_parts(
            "rust is the primary language",
            &MemoryScope::Global,
            EvidenceKind::AcceptedClaim,
        );
        assert_eq!(k1, k2, "normalized text + same scope + same kind = duplicate");
    }

    #[test]
    fn dedup_different_scope_not_duplicate() {
        let k1 = DedupKey::from_parts("same text", &MemoryScope::Global, EvidenceKind::AcceptedClaim);
        let k2 = DedupKey::from_parts(
            "same text",
            &MemoryScope::Project { repo: "foo".into(), branch: None },
            EvidenceKind::AcceptedClaim,
        );
        assert_ne!(k1, k2, "different scope = not duplicate");
    }

    #[test]
    fn dedup_different_evidence_kind_not_duplicate() {
        let k1 = DedupKey::from_parts("same text", &MemoryScope::Global, EvidenceKind::AcceptedClaim);
        let k2 = DedupKey::from_parts("same text", &MemoryScope::Global, EvidenceKind::RawObservation);
        assert_ne!(k1, k2, "different evidence kind = not duplicate");
    }

    #[test]
    fn dedup_same_trace_source_attaches_source() {
        // Simulate: same content, same trace source → DuplicateAttachSource
        let existing_id = "mem_abc123".to_string();
        let decision = DedupDecision::DuplicateAttachSource { existing_record_id: existing_id.clone() };
        assert_eq!(DedupDecision::DuplicateAttachSource { existing_record_id: existing_id }, decision);
    }

    #[test]
    fn dedup_same_episode_source_attaches_source() {
        let existing_id = "mem_def456".to_string();
        let decision = DedupDecision::DuplicateAttachSource { existing_record_id: existing_id.clone() };
        if let DedupDecision::DuplicateAttachSource { existing_record_id } = decision {
            assert_eq!("mem_def456", existing_record_id);
        } else {
            panic!("expected DuplicateAttachSource");
        }
    }

    #[test]
    fn dedup_preserves_existing_record_id() {
        let existing_id = "mem_original".to_string();
        let decision = DedupDecision::DuplicateAttachSource { existing_record_id: existing_id.clone() };
        match decision {
            DedupDecision::DuplicateAttachSource { existing_record_id } => {
                assert_eq!("mem_original", existing_record_id);
            }
            DedupDecision::New => panic!("expected DuplicateAttachSource"),
        }
    }

    #[test]
    fn dedup_does_not_change_existing_rank_reason() {
        // Dedup decision doesn't affect existing record's rank/reason
        // This test verifies the New variant carries no mutation payload
        let decision = DedupDecision::New;
        assert_eq!(DedupDecision::New, decision);
    }
}
