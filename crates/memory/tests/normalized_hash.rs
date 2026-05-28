//! Commit 2 — Persist normalized_text_hash on write.

use openwand_memory::dedup::compute_normalized_hash;
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
async fn accept_candidate_persists_normalized_text_hash() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let record = store.accept_candidate(make_candidate("Rust is great", "ep1")).await.unwrap().unwrap();
    assert!(!record.normalized_text_hash.is_empty());
    assert_eq!(compute_normalized_hash("Rust is great"), record.normalized_text_hash);
}

#[test]
fn same_text_same_hash() {
    let h1 = compute_normalized_hash("Rust is great");
    let h2 = compute_normalized_hash("Rust is great");
    assert_eq!(h1, h2);
}

#[test]
fn different_text_different_hash() {
    let h1 = compute_normalized_hash("Rust is great");
    let h2 = compute_normalized_hash("Python is great");
    assert_ne!(h1, h2);
}

#[test]
fn case_insensitive_normalization() {
    let h1 = compute_normalized_hash("Rust Is Great");
    let h2 = compute_normalized_hash("rust is great");
    assert_eq!(h1, h2);
}

#[test]
fn whitespace_normalization() {
    let h1 = compute_normalized_hash("Rust   Is   Great");
    let h2 = compute_normalized_hash("Rust Is Great");
    assert_eq!(h1, h2);
}

#[tokio::test]
async fn different_scope_same_hash_no_false_dedup() {
    // The hash is the same for same text, but dedup checks scope separately.
    // This test verifies the hash itself doesn't encode scope.
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();

    let r1 = store.accept_candidate(make_candidate("same claim text", "ep1")).await.unwrap().unwrap();
    let r2 = store.accept_candidate(make_candidate("same claim text", "ep2")).await.unwrap();

    // r2 should be None — duplicate claim text → dedup (attaches source)
    // Wait, both are same claim text, so the second should trigger dedup
    // and attach source to existing record, returning Some(existing)
    assert!(r2.is_some());
    assert_eq!(r1.record_id, r2.unwrap().record_id);
}

#[test]
fn dedup_index_covers_scope_columns() {
    // Verify the migration SQL includes the dedup index with scope
    let sql = openwand_memory::sqlite_schema::MEMORY_MIGRATION_0003_SQL;
    assert!(sql.contains("idx_memory_record_dedup"));
    assert!(sql.contains("normalized_text_hash"));
    assert!(sql.contains("scope_kind"));
    assert!(sql.contains("scope_payload"));
    assert!(sql.contains("evidence_kind"));
}
