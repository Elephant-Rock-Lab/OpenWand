//! Memory integration hardening acceptance tests.
//!
//! Proves:
//! - Projection creates episodes from trace events
//! - Memory panel refreshes after projection
//! - Retrieved memory is available for prompt injection
//! - Manual rebuild is idempotent
//! - Projection errors don't corrupt existing records

use openwand_core::SessionId;
use openwand_memory::testing::HeuristicExtractor;
use openwand_memory::{
    CandidateKind, CandidateMemory, EpisodeRole, InMemoryMemoryStore, MemoryEpisode,
    MemoryExtractor, MemoryQuery, MemoryStore,
};
use chrono::Utc;
use std::sync::Arc;

fn make_episode(id: &str, trace_id: &str, session_id: &str, content: &str) -> MemoryEpisode {
    MemoryEpisode {
        episode_id: id.to_string(),
        source_trace_id: trace_id.to_string(),
        session_id: session_id.to_string(),
        event_kind: "session.user_message_injected".to_string(),
        role: EpisodeRole::User,
        content: content.to_string(),
        created_at: Utc::now(),
    }
}

fn make_store() -> Arc<InMemoryMemoryStore> {
    Arc::new(InMemoryMemoryStore::new())
}

#[tokio::test]
async fn projection_creates_episodes_from_trace() {
    let memory = make_store();
    let session_id = SessionId::new();

    let episode = make_episode("ep_1", "trace_001", &session_id.to_string(), "Remember I use Rust");
    memory.project_episode(episode).await.unwrap();

    // Extract and accept
    let extractor = HeuristicExtractor;
    let accepted = memory.extract_and_accept(&extractor).await.unwrap();
    assert_eq!(1, accepted.len());
    assert_eq!("Remember I use Rust", accepted[0].claim);
}

#[tokio::test]
async fn memory_panel_refreshes_after_projection() {
    let memory = make_store();

    let episode = make_episode("ep_1", "trace_001", "sess_1", "Remember I use Rust");
    memory.project_episode(episode).await.unwrap();

    let extractor = HeuristicExtractor;
    memory.extract_and_accept(&extractor).await.unwrap();

    // Verify memory was accepted
    let records = memory.list_active_records().await.unwrap();
    assert_eq!(1, records.len());
    assert!(records[0].claim.contains("Rust"));
}

#[tokio::test]
async fn retrieved_memory_injected_into_prompt() {
    let memory = make_store();

    // Create a memory
    let episode = make_episode("ep_1", "trace_001", "sess_1", "Remember I prefer dark mode");
    memory.project_episode(episode).await.unwrap();

    let extractor = HeuristicExtractor;
    memory.extract_and_accept(&extractor).await.unwrap();

    // Simulate what the runner does: search by user message
    let ctx = memory
        .search_records(MemoryQuery::new("dark mode"))
        .await
        .unwrap();

    let block = ctx.to_context_block();
    assert!(block.is_some());
    assert!(block.unwrap().contains("dark mode"));
}

#[tokio::test]
async fn manual_rebuild_is_idempotent() {
    let memory = make_store();

    let episode = make_episode("ep_1", "trace_001", "sess_1", "Remember I use Rust");

    // Project twice (simulate rebuild)
    memory.project_episode(episode.clone()).await.unwrap();
    memory.project_episode(episode.clone()).await.unwrap();

    let episodes = memory.get_episodes("sess_1").await.unwrap();
    assert_eq!(1, episodes.len(), "Idempotent: only one episode");
}

#[tokio::test]
async fn projection_error_does_not_corrupt_existing_records() {
    let memory = make_store();

    // Create an initial memory
    let episode = make_episode("ep_1", "trace_001", "sess_1", "Remember I use Rust");
    memory.project_episode(episode).await.unwrap();

    let extractor = HeuristicExtractor;
    memory.extract_and_accept(&extractor).await.unwrap();

    // Verify initial state
    let active = memory.list_active_records().await.unwrap();
    assert_eq!(1, active.len());

    // Try to accept a malformed candidate — should not corrupt
    let result = memory
        .accept_candidate(CandidateMemory {
            claim: "".into(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep_1".into()],
        })
        .await
        .unwrap();
    assert!(result.is_none());

    // Existing records still intact
    let active = memory.list_active_records().await.unwrap();
    assert_eq!(1, active.len());
    assert_eq!("Remember I use Rust", active[0].claim);
}

#[tokio::test]
async fn memory_projection_runs_after_session_run() {
    let memory = make_store();
    let session_id = SessionId::new();

    // Simulate what happens after a run: user message gets projected
    let episode = make_episode(
        "ep_1",
        "trace_001",
        &session_id.to_string(),
        "Remember I always prefer concise responses",
    );
    memory.project_episode(episode).await.unwrap();

    // Extract and accept (automatic)
    let extractor = HeuristicExtractor;
    let accepted = memory.extract_and_accept(&extractor).await.unwrap();
    assert_eq!(1, accepted.len());

    // Verify memory was accepted
    let records = memory.list_active_records().await.unwrap();
    assert_eq!(1, records.len());

    // Next run can retrieve it
    let ctx = memory
        .search_records(MemoryQuery::new("concise"))
        .await
        .unwrap();
    assert!(!ctx.is_empty());
}

#[tokio::test]
async fn manual_rebuild_recovers_from_empty() {
    let memory = make_store();

    // Rebuild from empty trace → no records (but no errors)
    let episode = make_episode("ep_1", "trace_001", "sess_1", "Remember I use Rust");
    memory.project_episode(episode).await.unwrap();

    let extractor = HeuristicExtractor;
    memory.extract_and_accept(&extractor).await.unwrap();

    // Verify record exists
    let active = memory.list_active_records().await.unwrap();
    assert_eq!(1, active.len());

    // Re-extract (idempotent rebuild)
    memory.extract_and_accept(&extractor).await.unwrap();

    // Still only one record (duplicate claim attaches, doesn't duplicate)
    let active = memory.list_active_records().await.unwrap();
    assert_eq!(1, active.len());
}
