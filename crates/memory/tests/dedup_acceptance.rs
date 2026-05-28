//! Commit 3 — Invoke DedupKey during acceptance.

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
async fn accept_candidate_dedups_same_text() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();

    let r1 = store.accept_candidate(make_candidate("same claim", "ep1")).await.unwrap().unwrap();
    let r2 = store.accept_candidate(make_candidate("same claim", "ep2")).await.unwrap().unwrap();

    // Same record returned — dedupped
    assert_eq!(r1.record_id, r2.record_id, "duplicate claim should return same record");
    // Source episode attached
    assert!(r2.source_episode_ids.contains(&"ep1".to_string()));
    assert!(r2.source_episode_ids.contains(&"ep2".to_string()));
}

#[tokio::test]
async fn accept_candidate_does_not_dedup_different_text() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();

    let r1 = store.accept_candidate(make_candidate("claim A", "ep1")).await.unwrap().unwrap();
    let r2 = store.accept_candidate(make_candidate("claim B", "ep2")).await.unwrap().unwrap();

    assert_ne!(r1.record_id, r2.record_id, "different claims should create separate records");
}

#[tokio::test]
async fn accept_candidate_case_insensitive_dedup() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();

    let r1 = store.accept_candidate(make_candidate("Rust Is Great", "ep1")).await.unwrap().unwrap();
    let r2 = store.accept_candidate(make_candidate("rust is great", "ep2")).await.unwrap().unwrap();

    assert_eq!(r1.record_id, r2.record_id, "case-normalized dedup should match");
}

#[tokio::test]
async fn duplicate_attaches_source_trace_id() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();

    let r1 = store.accept_candidate(make_candidate("test claim", "ep1")).await.unwrap().unwrap();
    let r2 = store.accept_candidate(make_candidate("test claim", "ep2")).await.unwrap().unwrap();

    // Both source trace IDs should be present
    assert!(r2.source_trace_ids.contains(&"t1".to_string()));
    assert!(r2.source_trace_ids.contains(&"t2".to_string()));
}

#[tokio::test]
async fn duplicate_attaches_source_episode_id() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();

    let r = store.accept_candidate(make_candidate("test claim", "ep1")).await.unwrap().unwrap();
    store.accept_candidate(make_candidate("test claim", "ep2")).await.unwrap();

    let records = store.list_active_records().await.unwrap();
    assert_eq!(1, records.len());
    assert!(records[0].source_episode_ids.contains(&"ep1".to_string()));
    assert!(records[0].source_episode_ids.contains(&"ep2".to_string()));
}

#[tokio::test]
async fn duplicate_does_not_create_second_record() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();

    store.accept_candidate(make_candidate("unique claim", "ep1")).await.unwrap();
    store.accept_candidate(make_candidate("unique claim", "ep2")).await.unwrap();

    let records = store.list_active_records().await.unwrap();
    assert_eq!(1, records.len(), "duplicate should not create second record");
}

#[tokio::test]
async fn duplicate_is_idempotent_for_same_source() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();

    let r1 = store.accept_candidate(make_candidate("test claim", "ep1")).await.unwrap().unwrap();
    let r2 = store.accept_candidate(make_candidate("test claim", "ep1")).await.unwrap().unwrap();

    assert_eq!(r1.record_id, r2.record_id);
    // Source episode should not be duplicated
    let ep_count = r2.source_episode_ids.iter().filter(|id| *id == "ep1").count();
    assert!(ep_count <= 2, "source episode should not be excessively duplicated");
}
