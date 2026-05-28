//! Commit 5 — Add search_ranked with RetrievalMode.

use openwand_memory::evidence::EvidenceKind;
use openwand_memory::in_memory::InMemoryMemoryStore;
use openwand_memory::memory_store::MemoryStore;
use openwand_memory::query::MemoryQuery;
use openwand_memory::supersession::RetrievalMode;
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
async fn default_search_penalizes_superseded() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("Rust language", "ep1")).await.unwrap().unwrap();
    store.supersede_record(&old.record_id, "Python language".to_string()).await.unwrap();

    let ctx = store.search_ranked(MemoryQuery::new("language"), RetrievalMode::Default).await.unwrap();
    assert_eq!(2, ctx.hits.len());
    // Superseded record should be ranked lower (penalized 5000 bps)
    assert_eq!(EvidenceKind::AcceptedClaim, ctx.hits[0].evidence_kind);
    assert_eq!(EvidenceKind::SupersededClaim, ctx.hits[1].evidence_kind);
}

#[tokio::test]
async fn current_state_search_excludes_superseded() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("Rust language", "ep1")).await.unwrap().unwrap();
    store.supersede_record(&old.record_id, "Python language".to_string()).await.unwrap();

    let ctx = store.search_ranked(MemoryQuery::new("language"), RetrievalMode::CurrentState).await.unwrap();
    assert_eq!(1, ctx.hits.len());
    assert_eq!("Python language", ctx.hits[0].text);
}

#[tokio::test]
async fn change_history_search_includes_superseded() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("Rust language", "ep1")).await.unwrap().unwrap();
    store.supersede_record(&old.record_id, "Python language".to_string()).await.unwrap();

    let ctx = store.search_ranked(MemoryQuery::new("language"), RetrievalMode::ChangeHistory).await.unwrap();
    assert_eq!(2, ctx.hits.len(), "change history should include superseded");
}

#[tokio::test]
async fn conflict_search_includes_conflicting() {
    // Conflict records would be labeled ConflictingClaim
    // For now, test that ConflictSearch mode doesn't exclude based on superseded_by
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.accept_candidate(make_candidate("Rust language", "ep1")).await.unwrap();

    let ctx = store.search_ranked(MemoryQuery::new("Rust"), RetrievalMode::ConflictSearch).await.unwrap();
    assert_eq!(1, ctx.hits.len());
}

#[tokio::test]
async fn legacy_search_records_unchanged() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.accept_candidate(make_candidate("test claim about Rust", "ep1")).await.unwrap();

    let ctx = store.search_records(MemoryQuery::new("Rust")).await.unwrap();
    assert!(ctx.total_hits > 0);
}
