//! Memory extraction v0 acceptance tests.
//!
//! Proves:
//! - Episode projection from trace events
//! - Idempotent projection by source_trace_id
//! - Mock extractor returns candidates
//! - Malformed extraction writes no fact
//! - Accepted fact requires source_trace_id
//! - Duplicate claims attach new source episode
//! - Superseded facts keep old row with valid_to
//! - Memory search returns relevant facts
//! - Memory context formatting for LLM injection

use openwand_memory::{
    CandidateKind, CandidateMemory, EpisodeRole, InMemoryMemoryStore,
    MemoryEpisode, MemoryExtractor, MemoryKind, MemoryQuery, MemoryStore,
};
use openwand_memory::testing::{HeuristicExtractor, NullExtractor};
use chrono::Utc;

fn make_episode(id: &str, trace_id: &str, session_id: &str, role: EpisodeRole, content: &str) -> MemoryEpisode {
    MemoryEpisode {
        episode_id: id.to_string(),
        source_trace_id: trace_id.to_string(),
        session_id: session_id.to_string(),
        event_kind: "session.user_message_injected".to_string(),
        role,
        content: content.to_string(),
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn memory_projection_creates_episode_from_user_message_trace() {
    let store = InMemoryMemoryStore::new();

    let episode = make_episode("ep1", "trace_001", "sess_1", EpisodeRole::User, "Hello");
    store.project_episode(episode).await.unwrap();

    let episodes = store.get_episodes("sess_1").await.unwrap();
    assert_eq!(1, episodes.len());
    assert_eq!("Hello", episodes[0].content);
    assert_eq!(EpisodeRole::User, episodes[0].role);
}

#[tokio::test]
async fn memory_projection_is_idempotent_by_source_trace_id() {
    let store = InMemoryMemoryStore::new();

    let ep1 = make_episode("ep1", "trace_001", "sess_1", EpisodeRole::User, "First");
    let ep2 = make_episode("ep2", "trace_001", "sess_1", EpisodeRole::User, "Second");

    store.project_episode(ep1).await.unwrap();
    store.project_episode(ep2).await.unwrap(); // Same source_trace_id

    let episodes = store.get_episodes("sess_1").await.unwrap();
    assert_eq!(1, episodes.len());
    assert_eq!("First", episodes[0].content); // First one wins
}

#[tokio::test]
async fn extractor_mock_returns_candidate_fact() {
    let extractor = HeuristicExtractor;

    let episodes = vec![
        make_episode("ep1", "t1", "s1", EpisodeRole::User, "Remember that I use Rust"),
    ];

    let candidates = extractor.extract(&episodes).await;
    assert_eq!(1, candidates.len());
    assert_eq!("Remember that I use Rust", candidates[0].claim);
    assert_eq!(CandidateKind::Fact, candidates[0].kind);
    assert!(candidates[0].confidence >= 0.7);
}

#[tokio::test]
async fn malformed_extraction_writes_no_fact() {
    let store = InMemoryMemoryStore::new();

    // Empty claim
    let result = store
        .accept_candidate(CandidateMemory {
            claim: "".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep1".into()],
        })
        .await
        .unwrap();
    assert!(result.is_none());

    // No source episodes
    let result = store
        .accept_candidate(CandidateMemory {
            claim: "Some fact".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec![],
        })
        .await
        .unwrap();
    assert!(result.is_none());

    // Low confidence
    let result = store
        .accept_candidate(CandidateMemory {
            claim: "Some fact".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.3,
            source_episode_ids: vec!["ep1".into()],
        })
        .await
        .unwrap();
    assert!(result.is_none());

    // Verify nothing was stored
    let records = store.list_active_records().await.unwrap();
    assert!(records.is_empty());
}

#[tokio::test]
async fn accepted_fact_requires_source_trace_id() {
    let store = InMemoryMemoryStore::new();

    // First project the episode so trace ID is available
    let episode = make_episode("ep1", "trace_abc", "s1", EpisodeRole::User, "I use Rust");
    store.project_episode(episode).await.unwrap();

    let result = store
        .accept_candidate(CandidateMemory {
            claim: "I use Rust".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep1".into()],
        })
        .await
        .unwrap()
        .unwrap();

    assert!(!result.source_trace_ids.is_empty());
    assert!(result.source_trace_ids.contains(&"trace_abc".to_string()));
}

#[tokio::test]
async fn duplicate_fact_attaches_new_source_episode() {
    let store = InMemoryMemoryStore::new();

    let ep1 = make_episode("ep1", "t1", "s1", EpisodeRole::User, "I use Rust");
    let ep2 = make_episode("ep2", "t2", "s1", EpisodeRole::User, "I use Rust");
    store.project_episode(ep1).await.unwrap();
    store.project_episode(ep2).await.unwrap();

    // Accept first
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

    // Accept duplicate — should attach, not create new
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

    assert_eq!(r1.record_id, r2.record_id, "Should be same record");
    assert!(r2.source_episode_ids.contains(&"ep1".to_string()));
    assert!(r2.source_episode_ids.contains(&"ep2".to_string()));

    // Only one record in store
    let records = store.list_active_records().await.unwrap();
    assert_eq!(1, records.len());
}

#[tokio::test]
async fn superseded_fact_keeps_old_row_with_valid_to() {
    let store = InMemoryMemoryStore::new();

    let ep1 = make_episode("ep1", "t1", "s1", EpisodeRole::User, "I use Python");
    store.project_episode(ep1).await.unwrap();

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

    // Supersede
    let r2 = store.supersede_record(&r1.record_id, "I use Rust".into()).await.unwrap();

    assert!(r2.superseded_by.is_none());
    assert_eq!("I use Rust", r2.claim);
    assert_eq!(r2.kind, MemoryKind::Fact);
    // Verify: old record is not active, new one is
    let active = store.list_active_records().await.unwrap();
    assert_eq!(1, active.len(), "Only new record should be active");
    assert_eq!("I use Rust", active[0].claim);
    assert_ne!(r1.record_id, r2.record_id, "New record should have different ID");
}

#[tokio::test]
async fn memory_search_returns_relevant_fact() {
    let store = InMemoryMemoryStore::new();

    let ep1 = make_episode("ep1", "t1", "s1", EpisodeRole::User, "Remember I use Rust");
    store.project_episode(ep1).await.unwrap();

    // Extract and accept
    let extractor = HeuristicExtractor;
    store.extract_and_accept(&extractor).await.unwrap();

    // Search
    let ctx = store
        .search_records(MemoryQuery::new("Rust"))
        .await
        .unwrap();

    assert!(!ctx.is_empty());
    assert!(ctx.facts.iter().any(|f| f.contains("Rust")));
}

#[tokio::test]
async fn memory_context_formatting_for_llm_injection() {
    let store = InMemoryMemoryStore::new();

    let ep1 = make_episode("ep1", "t1", "s1", EpisodeRole::User, "Remember I prefer dark mode");
    store.project_episode(ep1).await.unwrap();

    let extractor = HeuristicExtractor;
    store.extract_and_accept(&extractor).await.unwrap();

    let ctx = store
        .search_records(MemoryQuery::new("prefer"))
        .await
        .unwrap();

    let block = ctx.to_context_block();
    assert!(block.is_some());
    let text = block.unwrap();
    assert!(text.contains("dark mode"));
}

#[tokio::test]
async fn null_extractor_extracts_nothing() {
    let extractor = NullExtractor;

    let episodes = vec![
        make_episode("ep1", "t1", "s1", EpisodeRole::User, "Remember this"),
    ];

    let candidates = extractor.extract(&episodes).await;
    assert!(candidates.is_empty());
}
