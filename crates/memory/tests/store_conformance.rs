//! Commit 7 — Backend conformance tests.
//!
//! Run the same semantic scenarios against InMemoryStore and SqliteStore.

use openwand_memory::evidence::EvidenceKind;
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

async fn setup_store(store: &dyn MemoryStore) {
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();
    store.accept_candidate(make_candidate("Rust programming language", "ep1")).await.unwrap();
    store.accept_candidate(make_candidate("Python programming language", "ep2")).await.unwrap();
}

// ── InMemoryStore tests ──

#[tokio::test]
async fn inmemory_evidence_kind_order() {
    let store = openwand_memory::in_memory::InMemoryMemoryStore::new();
    setup_store(&store).await;
    let ctx = store.search_ranked(MemoryQuery::new("programming"), RetrievalMode::Default).await.unwrap();
    assert_eq!(2, ctx.hits.len());
    assert_eq!(EvidenceKind::AcceptedClaim, ctx.hits[0].evidence_kind);
}

#[tokio::test]
async fn inmemory_dedup_order() {
    let store = openwand_memory::in_memory::InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();
    store.accept_candidate(make_candidate("same claim", "ep1")).await.unwrap();
    store.accept_candidate(make_candidate("same claim", "ep2")).await.unwrap();
    let records = store.list_active_records().await.unwrap();
    assert_eq!(1, records.len(), "inmemory: dedup should produce one record");
}

#[tokio::test]
async fn inmemory_supersession_order() {
    let store = openwand_memory::in_memory::InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("Rust language", "ep1")).await.unwrap().unwrap();
    store.supersede_record(&old.record_id, "Python language".to_string()).await.unwrap();
    let ctx = store.search_ranked(MemoryQuery::new("language"), RetrievalMode::Default).await.unwrap();
    assert_eq!(2, ctx.hits.len());
    assert_eq!("Python language", ctx.hits[0].text, "successor should rank first");
}

#[tokio::test]
async fn inmemory_current_state_order() {
    let store = openwand_memory::in_memory::InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("Rust language", "ep1")).await.unwrap().unwrap();
    store.supersede_record(&old.record_id, "Python language".to_string()).await.unwrap();
    let ctx = store.search_ranked(MemoryQuery::new("language"), RetrievalMode::CurrentState).await.unwrap();
    assert_eq!(1, ctx.hits.len(), "inmemory: current state excludes superseded");
    assert_eq!("Python language", ctx.hits[0].text);
}

#[tokio::test]
async fn inmemory_change_history_order() {
    let store = openwand_memory::in_memory::InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("Rust language", "ep1")).await.unwrap().unwrap();
    store.supersede_record(&old.record_id, "Python language".to_string()).await.unwrap();
    let ctx = store.search_ranked(MemoryQuery::new("language"), RetrievalMode::ChangeHistory).await.unwrap();
    assert_eq!(2, ctx.hits.len(), "inmemory: change history includes both");
}

// ── SQLiteStore tests (feature-gated) ──

#[cfg(feature = "sqlite-testing")]
mod sqlite_conformance {
    use super::*;
    use openwand_memory::sqlite_store::SqliteMemoryStore;
    use tempfile::TempDir;

    fn make_store() -> SqliteMemoryStore {
        let dir = TempDir::new().unwrap();
        SqliteMemoryStore::open(&dir.path().join("test.db")).unwrap()
    }

    #[tokio::test]
    async fn sqlite_evidence_kind_order() {
        let store = make_store();
        setup_store(&store).await;
        let ctx = store.search_ranked(MemoryQuery::new("programming"), RetrievalMode::Default).await.unwrap();
        assert_eq!(2, ctx.hits.len());
        assert_eq!(EvidenceKind::AcceptedClaim, ctx.hits[0].evidence_kind);
    }

    #[tokio::test]
    async fn sqlite_dedup_order() {
        let store = make_store();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        store.project_episode(make_episode("ep2", "t2")).await.unwrap();
        store.accept_candidate(make_candidate("same claim", "ep1")).await.unwrap();
        store.accept_candidate(make_candidate("same claim", "ep2")).await.unwrap();
        let records = store.list_active_records().await.unwrap();
        assert_eq!(1, records.len(), "sqlite: dedup should produce one record");
    }

    #[tokio::test]
    async fn sqlite_supersession_order() {
        let store = make_store();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        let old = store.accept_candidate(make_candidate("Rust language", "ep1")).await.unwrap().unwrap();
        store.supersede_record(&old.record_id, "Python language".to_string()).await.unwrap();
        let ctx = store.search_ranked(MemoryQuery::new("language"), RetrievalMode::Default).await.unwrap();
        assert_eq!(2, ctx.hits.len());
        assert_eq!("Python language", ctx.hits[0].text, "successor should rank first");
    }

    #[tokio::test]
    async fn sqlite_current_state_order() {
        let store = make_store();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        let old = store.accept_candidate(make_candidate("Rust language", "ep1")).await.unwrap().unwrap();
        store.supersede_record(&old.record_id, "Python language".to_string()).await.unwrap();
        let ctx = store.search_ranked(MemoryQuery::new("language"), RetrievalMode::CurrentState).await.unwrap();
        assert_eq!(1, ctx.hits.len(), "sqlite: current state excludes superseded");
        assert_eq!("Python language", ctx.hits[0].text);
    }

    #[tokio::test]
    async fn sqlite_change_history_order() {
        let store = make_store();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        let old = store.accept_candidate(make_candidate("Rust language", "ep1")).await.unwrap().unwrap();
        store.supersede_record(&old.record_id, "Python language".to_string()).await.unwrap();
        let ctx = store.search_ranked(MemoryQuery::new("language"), RetrievalMode::ChangeHistory).await.unwrap();
        assert_eq!(2, ctx.hits.len(), "sqlite: change history includes both");
    }

    #[tokio::test]
    async fn sqlite_conflict_order() {
        let store = make_store();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        store.project_episode(make_episode("ep2", "t2")).await.unwrap();
        store.accept_candidate(make_candidate("Rust language", "ep1")).await.unwrap();
        store.accept_candidate(make_candidate("Python language", "ep2")).await.unwrap();
        let ctx = store.search_ranked(MemoryQuery::new("language"), RetrievalMode::ConflictSearch).await.unwrap();
        assert_eq!(2, ctx.hits.len());
    }
}
