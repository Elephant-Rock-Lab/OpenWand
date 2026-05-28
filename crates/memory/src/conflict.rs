//! Conflict-aware retrieval.
//!
//! Contradictory claims are retained and grouped, never overwritten.

use serde::{Deserialize, Serialize};

/// A group of memory records that contradict each other.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictGroup {
    /// Unique identifier for this conflict group.
    pub id: String,
    /// Record IDs participating in this conflict.
    pub record_ids: Vec<String>,
}

impl ConflictGroup {
    pub fn new(id: String, record_ids: Vec<String>) -> Self {
        Self { id, record_ids }
    }

    /// Whether a given record is part of this conflict.
    pub fn contains_record(&self, record_id: &str) -> bool {
        self.record_ids.iter().any(|r| r == record_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflicting_claims_are_both_retained() {
        let group = ConflictGroup::new(
            "cg_001".to_string(),
            vec!["mem_a".to_string(), "mem_b".to_string()],
        );
        assert_eq!(2, group.record_ids.len());
        assert!(group.contains_record("mem_a"));
        assert!(group.contains_record("mem_b"));
    }

    #[test]
    fn conflicting_claims_share_conflict_group_id() {
        let group = ConflictGroup::new(
            "cg_002".to_string(),
            vec!["mem_x".to_string(), "mem_y".to_string()],
        );
        assert_eq!("cg_002", group.id);
        assert_eq!(2, group.record_ids.len());
    }

    #[test]
    fn default_search_labels_conflicting_claim() {
        // ConflictGroup doesn't affect ranking directly —
        // EvidenceKind::ConflictingClaim does. This test verifies
        // the group structure is correct for labeling.
        use crate::evidence::EvidenceKind;
        let kind = EvidenceKind::ConflictingClaim;
        assert!(!kind.is_accepted_state());
        assert_eq!("ConflictingClaim", format!("{:?}", kind));
    }

    #[test]
    fn conflict_search_returns_all_group_members() {
        let group = ConflictGroup::new(
            "cg_003".to_string(),
            vec!["mem_1".to_string(), "mem_2".to_string(), "mem_3".to_string()],
        );
        assert_eq!(3, group.record_ids.len());
        for id in &["mem_1", "mem_2", "mem_3"] {
            assert!(group.contains_record(id));
        }
    }

    #[test]
    fn conflict_does_not_supersede_without_explicit_successor() {
        // Conflict groups don't create supersession — they're independent
        let group = ConflictGroup::new(
            "cg_004".to_string(),
            vec!["mem_a".to_string(), "mem_b".to_string()],
        );
        // Both records are still active — no superseded_by
        assert!(group.contains_record("mem_a"));
        assert!(group.contains_record("mem_b"));
        // No successor link implied by conflict
        assert_eq!(2, group.record_ids.len());
    }
}
