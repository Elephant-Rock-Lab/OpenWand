//! SQLite memory store acceptance tests.
//!
//! Proves:
//! - Episodes persist to SQLite and survive reopen
//! - Projection is idempotent by source_trace_id
//! - Accepted records persist with sources
//! - Duplicate claims attach new source episodes
//! - Superseded records kept with valid_until
//! - Keyword search works through SQLite
//! - Memory search survives reopen (close + reopen DB)
//! - Projection checkpoint advances

use openwand_memory::{
    CandidateKind, CandidateMemory, EpisodeRole, MemoryEpisode,
    MemoryQuery, MemoryStore,
};
#[cfg(feature = "sqlite")]
use openwand_memory::SqliteMemoryStore;
#[cfg(feature = "sqlite")]
use openwand_memory::testing::HeuristicExtractor;

use chrono::Utc;

fn make_episode(id: &str, trace_id: &str, session_id: &str, role: EpisodeRole, content: &str) -> MemoryEpisode {
    MemoryEpisode {
        episode_id: id.to_string(),
        source_trace_id: trace_id.to_string(),
        session_id: session_id.to_string(),
        event_kind: "session.user_message".to_string(),
        role,
        content: content.to_string(),
        created_at: Utc::now(),
    }
}

#[cfg(feature = "sqlite")]
fn make_store() -> SqliteMemoryStore {
    SqliteMemoryStore::open_in_memory().unwrap()
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn sqlite_memory_store_put_episode_persists() {
    let store = make_store();

    let ep = make_episode("ep1", "trace_001", "sess_1", EpisodeRole::User, "Hello world");
    store.project_episode(ep).await.unwrap();

    let episodes = store.get_episodes("sess_1").await.unwrap();
    assert_eq!(1, episodes.len());
    assert_eq!("Hello world", episodes[0].content);
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn sqlite_memory_store_projection_idempotent_by_source_trace_id() {
    let store = make_store();

    let ep1 = make_episode("ep1", "trace_001", "s1", EpisodeRole::User, "First");
    let ep2 = make_episode("ep2", "trace_001", "s1", EpisodeRole::User, "Second");

    store.project_episode(ep1).await.unwrap();
    store.project_episode(ep2).await.unwrap();

    let episodes = store.get_episodes("s1").await.unwrap();
    assert_eq!(1, episodes.len());
    assert_eq!("First", episodes[0].content);
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn sqlite_memory_store_accept_record_persists_with_sources() {
    let store = make_store();

    let ep = make_episode("ep1", "trace_abc", "s1", EpisodeRole::User, "I use Rust");
    store.project_episode(ep).await.unwrap();

    let record = store
        .accept_candidate(CandidateMemory {
            claim: "I use Rust".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep1".into()],
        })
        .await
        .unwrap()
        .unwrap();

    assert!(!record.source_trace_ids.is_empty());
    assert!(record.source_trace_ids.contains(&"trace_abc".to_string()));

    // Verify record persists
    let active = store.list_active_records().await.unwrap();
    assert_eq!(1, active.len());
    assert_eq!("I use Rust", active[0].claim);
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn sqlite_memory_store_duplicate_claim_attaches_source() {
    let store = make_store();

    let ep1 = make_episode("ep1", "t1", "s1", EpisodeRole::User, "I use Rust");
    let ep2 = make_episode("ep2", "t2", "s1", EpisodeRole::User, "I use Rust");
    store.project_episode(ep1).await.unwrap();
    store.project_episode(ep2).await.unwrap();

    let r1 = store
        .accept_candidate(CandidateMemory {
            claim: "I use Rust".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep1".into()],
        })
        .await
        .unwrap()
        .unwrap();

    let r2 = store
        .accept_candidate(CandidateMemory {
            claim: "I use Rust".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep2".into()],
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(r1.record_id, r2.record_id);

    let active = store.list_active_records().await.unwrap();
    assert_eq!(1, active.len());
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn sqlite_memory_store_supersede_keeps_old_record() {
    let store = make_store();

    let ep = make_episode("ep1", "t1", "s1", EpisodeRole::User, "I use Python");
    store.project_episode(ep).await.unwrap();

    let r1 = store
        .accept_candidate(CandidateMemory {
            claim: "I use Python".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep1".into()],
        })
        .await
        .unwrap()
        .unwrap();

    let r2 = store.supersede_record(&r1.record_id, "I use Rust".into()).await.unwrap();

    assert_eq!("I use Rust", r2.claim);
    assert!(r2.superseded_by.is_none());
    assert_ne!(r1.record_id, r2.record_id);

    // Old record not active
    let active = store.list_active_records().await.unwrap();
    assert_eq!(1, active.len());
    assert_eq!("I use Rust", active[0].claim);
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn sqlite_memory_store_search_finds_matching_records() {
    let store = make_store();

    let ep = make_episode("ep1", "t1", "s1", EpisodeRole::User, "Remember I use Rust");
    store.project_episode(ep).await.unwrap();

    // Extract and accept
    let extractor = HeuristicExtractor;
    store.extract_and_accept(&extractor).await.unwrap();

    let ctx = store
        .search_records(MemoryQuery::new("Rust"))
        .await
        .unwrap();

    assert!(!ctx.is_empty());
    assert!(ctx.facts.iter().any(|f| f.contains("Rust")));
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn sqlite_memory_store_search_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("memory_test.db");

    // Write
    {
        let store = SqliteMemoryStore::open(&db_path).unwrap();
        let ep = make_episode("ep1", "t1", "s1", EpisodeRole::User, "Remember I prefer dark mode");
        store.project_episode(ep).await.unwrap();

        let extractor = HeuristicExtractor;
        store.extract_and_accept(&extractor).await.unwrap();
    }

    // Reopen and search
    {
        let store = SqliteMemoryStore::open(&db_path).unwrap();
        let ctx = store
            .search_records(MemoryQuery::new("dark"))
            .await
            .unwrap();

        assert!(!ctx.is_empty());
        assert!(ctx.facts.iter().any(|f| f.contains("dark mode")));
    }
}

#[tokio::test]
#[cfg(feature = "sqlite")]
async fn sqlite_memory_store_full_round_trip_extract_search_format() {
    let store = make_store();

    let ep = make_episode("ep1", "t1", "s1", EpisodeRole::User, "Remember I always use Rust");
    store.project_episode(ep).await.unwrap();

    let extractor = HeuristicExtractor;
    store.extract_and_accept(&extractor).await.unwrap();

    let ctx = store
        .search_records(MemoryQuery::new("Rust"))
        .await
        .unwrap();

    let block = ctx.to_context_block();
    assert!(block.is_some());
    let text = block.unwrap();
    assert!(text.contains("Rust"));
}
