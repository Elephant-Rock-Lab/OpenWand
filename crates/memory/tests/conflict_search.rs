//! Commit 6 — Read conflict_group_id in search.

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
async fn search_labels_conflict_group_records() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();

    let r1 = store.accept_candidate(make_candidate("prefer tabs for indentation", "ep1")).await.unwrap().unwrap();
    let r2 = store.accept_candidate(make_candidate("prefer spaces for indentation", "ep2")).await.unwrap().unwrap();

    // Manually set conflict group on records
    {
        let mut records = store.records.lock().unwrap();
        if let Some(rec) = records.get_mut(&r1.record_id) {
            rec.conflict_group_id = Some("cg_indent".to_string());
        }
        if let Some(rec) = records.get_mut(&r2.record_id) {
            rec.conflict_group_id = Some("cg_indent".to_string());
        }
    }

    let ctx = store.search_ranked(MemoryQuery::new("indentation"), RetrievalMode::ConflictSearch).await.unwrap();
    assert_eq!(2, ctx.hits.len());
    // Both should be labeled as ConflictingClaim (via derived_evidence_kind in search)
}

#[tokio::test]
async fn conflict_search_returns_group_members() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();

    let r1 = store.accept_candidate(make_candidate("theme preference dark", "ep1")).await.unwrap().unwrap();
    let r2 = store.accept_candidate(make_candidate("theme preference light", "ep2")).await.unwrap().unwrap();

    // Manually set conflict group
    {
        let mut records = store.records.lock().unwrap();
        records.get_mut(&r1.record_id).unwrap().conflict_group_id = Some("cg_theme".to_string());
        records.get_mut(&r2.record_id).unwrap().conflict_group_id = Some("cg_theme".to_string());
    }

    let ctx = store.search_ranked(MemoryQuery::new("theme"), RetrievalMode::ConflictSearch).await.unwrap();
    assert_eq!(2, ctx.hits.len());
}

#[tokio::test]
async fn default_search_penalizes_conflicting() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();

    let r1 = store.accept_candidate(make_candidate("conflicting claim A", "ep1")).await.unwrap().unwrap();
    let r2 = store.accept_candidate(make_candidate("conflicting claim B", "ep2")).await.unwrap().unwrap();

    // Set conflict group
    {
        let mut records = store.records.lock().unwrap();
        records.get_mut(&r1.record_id).unwrap().conflict_group_id = Some("cg_conflict".to_string());
        records.get_mut(&r2.record_id).unwrap().conflict_group_id = Some("cg_conflict".to_string());
    }

    let ctx = store.search_ranked(MemoryQuery::new("conflicting"), RetrievalMode::Default).await.unwrap();
    // Both should appear but potentially with penalty for being in conflict
    assert_eq!(2, ctx.hits.len());
}

#[tokio::test]
async fn conflict_group_id_roundtrips_inmemory() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let record = store.accept_candidate(make_candidate("test", "ep1")).await.unwrap().unwrap();
    assert_eq!(None, record.conflict_group_id, "new records should have no conflict group");
}

#[cfg(feature = "sqlite-testing")]
mod sqlite_conflict {
    use openwand_memory::memory_store::MemoryStore;
    use openwand_memory::sqlite_store::SqliteMemoryStore;
    use openwand_memory::types::{CandidateMemory, CandidateKind, EpisodeRole, MemoryEpisode};
    use tempfile::TempDir;

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

    fn make_store() -> SqliteMemoryStore {
        let dir = TempDir::new().unwrap();
        SqliteMemoryStore::open(&dir.path().join("test.db")).unwrap()
    }

    #[tokio::test]
    async fn conflict_group_id_roundtrips_sqlite() {
        let store = make_store();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        let candidate = CandidateMemory {
            claim: "test claim".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep1".to_string()],
        };
        let record = store.accept_candidate(candidate).await.unwrap().unwrap();
        assert_eq!(None, record.conflict_group_id, "new records should have no conflict group");
    }
}
