//! Commit 6 — Supersession-aware retrieval integration tests.

use openwand_memory::supersession::{RetrievalMode, supersession_penalty, should_exclude_superseded};

#[test]
fn integration_default_mode_penalizes_superseded_claim() {
    let penalty = supersession_penalty(true, RetrievalMode::Default);
    assert_eq!(5000, penalty);
    assert!(!should_exclude_superseded(true, RetrievalMode::Default));
}

#[test]
fn integration_current_state_excludes_superseded() {
    assert!(should_exclude_superseded(true, RetrievalMode::CurrentState));
    let penalty = supersession_penalty(true, RetrievalMode::CurrentState);
    assert_eq!(10000, penalty, "max penalty in current-state mode");
}

#[test]
fn integration_change_history_includes_chain() {
    assert!(!should_exclude_superseded(true, RetrievalMode::ChangeHistory));
    assert_eq!(0, supersession_penalty(true, RetrievalMode::ChangeHistory));
}

#[test]
fn integration_conflict_search_mild_penalty() {
    assert!(!should_exclude_superseded(true, RetrievalMode::ConflictSearch));
    assert_eq!(2000, supersession_penalty(true, RetrievalMode::ConflictSearch));
}
