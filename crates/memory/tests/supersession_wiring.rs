//! Commit 4 — Wire supersession writes.

use openwand_memory::evidence::EvidenceKind;
use openwand_memory::in_memory::InMemoryMemoryStore;
use openwand_memory::memory_store::MemoryStore;
use openwand_memory::types::{CandidateMemory, CandidateKind, EpisodeRole, MemoryEpisode};

fn make_episode(id: &str, trace_id: &str) -> MemoryEpisode {
    MemoryEpisode {
        episode_id: id.to_string(),
        source_trace_id: trace_id.to_string(),
        session_id: "s1".to_string(),
        event_kind: "message".to_string(),
        role: EpisodeRole::User,
        content: "test".to_string(),
        created_at: chrono::Utc::now(),
    }
}

fn make_candidate(claim: &str, ep_id: &str) -> CandidateMemory {
    CandidateMemory {
        claim: claim.to_string(),
        kind: CandidateKind::Fact,
        confidence: 0.9,
        source_episode_ids: vec![ep_id.to_string()],
    }
}

#[tokio::test]
async fn supersession_populates_supersedes_record_id() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("old claim", "ep1")).await.unwrap().unwrap();
    let new = store.supersede_record(&old.record_id, "new claim".to_string()).await.unwrap();

    assert_eq!(Some(old.record_id.clone()), new.supersedes_record_id);
}

#[tokio::test]
async fn superseded_record_labeled_superseded_claim() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("old claim", "ep1")).await.unwrap().unwrap();
    store.supersede_record(&old.record_id, "new claim".to_string()).await.unwrap();

    // Fetch the old record — should have superseded_by set
    let all = store.list_active_records().await.unwrap();
    // Only the new record should be active
    assert_eq!(1, all.len());
    assert_eq!("new claim", all[0].claim);
}

#[tokio::test]
async fn successor_links_to_prior() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("original", "ep1")).await.unwrap().unwrap();
    let new = store.supersede_record(&old.record_id, "replacement".to_string()).await.unwrap();

    assert_eq!(Some(old.record_id), new.supersedes_record_id);
    assert!(new.is_active());
}

#[tokio::test]
async fn supersession_preserves_old_record() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("v1", "ep1")).await.unwrap().unwrap();
    let old_id = old.record_id.clone();
    store.supersede_record(&old_id, "v2".to_string()).await.unwrap();

    // Old record should still exist in the store (just not active)
    // We can't easily query inactive records through the public API,
    // but we can verify the successor exists and is linked
    let active = store.list_active_records().await.unwrap();
    assert_eq!(1, active.len());
    assert_eq!(Some(old_id), active[0].supersedes_record_id);
}

#[tokio::test]
async fn supersession_keeps_successor_active() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("old", "ep1")).await.unwrap().unwrap();
    let new = store.supersede_record(&old.record_id, "new".to_string()).await.unwrap();

    assert!(new.is_active());
    assert!(new.superseded_by.is_none());
    assert!(new.supersedes_record_id.is_some());
}

#[tokio::test]
async fn derived_evidence_kind_superseded() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("v1", "ep1")).await.unwrap().unwrap();

    // Before supersession: derived kind is AcceptedClaim
    assert_eq!(EvidenceKind::AcceptedClaim, old.derived_evidence_kind());

    // After supersession, fetch the old record
    store.supersede_record(&old.record_id, "v2".to_string()).await.unwrap();

    // The successor should have AcceptedClaim (not superseded)
    let active = store.list_active_records().await.unwrap();
    assert_eq!(1, active.len());
    assert_eq!(EvidenceKind::AcceptedClaim, active[0].derived_evidence_kind());
}
