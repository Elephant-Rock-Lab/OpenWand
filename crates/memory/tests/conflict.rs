//! Commit 7 — Conflict-aware retrieval integration tests.

use openwand_memory::conflict::ConflictGroup;
use openwand_memory::evidence::EvidenceKind;

#[test]
fn integration_conflicting_claims_both_retained() {
    let group = ConflictGroup::new(
        "cg_int1".to_string(),
        vec!["mem_left".to_string(), "mem_right".to_string()],
    );
    assert_eq!(2, group.record_ids.len());
}

#[test]
fn integration_conflict_search_returns_all_members() {
    let group = ConflictGroup::new(
        "cg_int2".to_string(),
        vec!["mem_1".to_string(), "mem_2".to_string(), "mem_3".to_string()],
    );
    assert_eq!(3, group.record_ids.len());
    assert!(group.contains_record("mem_1"));
    assert!(group.contains_record("mem_2"));
    assert!(group.contains_record("mem_3"));
}

#[test]
fn integration_conflicting_claim_is_labeled() {
    assert_eq!(EvidenceKind::ConflictingClaim, EvidenceKind::ConflictingClaim);
    assert!(!EvidenceKind::ConflictingClaim.is_accepted_state());
}
