//! Memory UI service integration test.
//!
//! Proves:
//! - Filtered panel renders coordinator output
//! - Panel shows correct trust buckets

use openwand_app::memory_coordinator::{
    MemoryCoordinator, PromptInputProductionConfig,
};
use openwand_app::ui::memory_service::build_filtered_panel;
use openwand_memory::{
    CandidateKind, CandidateMemory, EpisodeRole, InMemoryMemoryStore, MemoryEpisode, MemoryStore,
};
use openwand_memory::MemoryExtractor;
use openwand_store::StoredEvent;
use openwand_trace::testing::InMemoryTraceStore;
use chrono::Utc;
use std::sync::Arc;

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

struct StubExtractor;

#[async_trait::async_trait]
impl MemoryExtractor for StubExtractor {
    async fn extract(&self, _episodes: &[MemoryEpisode]) -> Vec<CandidateMemory> {
        vec![]
    }
}

fn make_coordinator(store: Arc<InMemoryMemoryStore>) -> MemoryCoordinator {
    MemoryCoordinator::new(
        store as Arc<dyn MemoryStore>,
        Arc::new(StubExtractor),
        Arc::new(InMemoryTraceStore::<StoredEvent>::new()),
    )
}

fn create_workspace_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/core\"]\n",
    ).unwrap();
    let core_dir = root.join("crates").join("core");
    std::fs::create_dir_all(core_dir.join("src")).unwrap();
    std::fs::write(core_dir.join("Cargo.toml"), "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::write(core_dir.join("src").join("lib.rs"), "pub fn hello() {}").unwrap();
    dir
}

#[tokio::test]
async fn ui_memory_panel_shows_prompt_included_claims() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let ep = make_episode("ep1", "trace_001", "s1", "text");
    store.project_episode(ep).await.unwrap();
    store.accept_candidate(CandidateMemory {
        claim: "crate core exists".to_string(),
        kind: CandidateKind::Fact,
        confidence: 0.95,
        source_episode_ids: vec!["ep1".into()],
    }).await.unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    let panel = build_filtered_panel(&result);

    assert!(panel.summary.prompt_included >= 1, "should have at least 1 trusted claim");
    assert!(panel.prompt_included.iter().any(|r| r.claim.contains("core")));
}

#[tokio::test]
async fn ui_memory_panel_empty_when_no_memory() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    let panel = build_filtered_panel(&result);
    assert!(panel.is_empty());
}
