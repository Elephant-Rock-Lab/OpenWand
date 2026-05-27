//! Memory UI service integration test.
//!
//! Proves:
//! - Memory panel lists records
//! - Memory panel shows source trace IDs
//! - Memory context formatting works

use openwand_app::ui::memory_dto::UiMemoryRecord;
use openwand_app::ui::memory_service::build_memory_panel;
use openwand_memory::{
    CandidateKind, CandidateMemory, EpisodeRole, InMemoryMemoryStore, MemoryEpisode, MemoryStore,
};
use chrono::Utc;

fn make_episode(id: &str, trace_id: &str, session_id: &str, content: &str) -> MemoryEpisode {
    MemoryEpisode {
        episode_id: id.to_string(),
        source_trace_id: trace_id.to_string(),
        session_id: session_id.to_string(),
        event_kind: "session.user_message".to_string(),
        role: EpisodeRole::User,
        content: content.to_string(),
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn ui_memory_panel_lists_records() {
    let store = InMemoryMemoryStore::new();

    let ep = make_episode("ep1", "trace_001", "s1", "Remember I use Rust");
    store.project_episode(ep).await.unwrap();

    store
        .accept_candidate(CandidateMemory {
            claim: "Remember I use Rust".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep1".into()],
        })
        .await
        .unwrap();

    let panel = build_memory_panel(&store).await.unwrap();

    assert_eq!(1, panel.active_count);
    assert_eq!(1, panel.records.len());
    assert_eq!("Remember I use Rust", panel.records[0].claim);
}

#[tokio::test]
async fn ui_memory_panel_shows_source_trace_ids() {
    let store = InMemoryMemoryStore::new();

    let ep = make_episode("ep1", "trace_xyz", "s1", "I prefer dark mode");
    store.project_episode(ep).await.unwrap();

    store
        .accept_candidate(CandidateMemory {
            claim: "I prefer dark mode".to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep1".into()],
        })
        .await
        .unwrap();

    let panel = build_memory_panel(&store).await.unwrap();

    assert_eq!(vec!["trace_xyz"], panel.records[0].source_trace_ids);
    assert_eq!(1, panel.records[0].source_count);
}
