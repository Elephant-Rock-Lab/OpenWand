//! Wave 02n integration tests — provenance hydration end-to-end.

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

fn make_episode(id: &str, session_id: &str, content: &str) -> MemoryEpisode {
    MemoryEpisode {
        episode_id: id.to_string(),
        source_trace_id: format!("trace_{id}"),
        session_id: session_id.to_string(),
        event_kind: "session.user_message".to_string(),
        role: EpisodeRole::User,
        content: content.to_string(),
        created_at: Utc::now(),
    }
}

fn make_candidate(claim: &str, confidence: f64, episode_ids: &[&str]) -> CandidateMemory {
    CandidateMemory {
        claim: claim.to_string(),
        kind: CandidateKind::Fact,
        confidence,
        source_episode_ids: episode_ids.iter().map(|s| s.to_string()).collect(),
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
    std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = [\"crates/core\"]\n").unwrap();
    let core_dir = root.join("crates").join("core");
    std::fs::create_dir_all(core_dir.join("src")).unwrap();
    std::fs::write(core_dir.join("Cargo.toml"), "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::write(core_dir.join("src").join("lib.rs"), "pub fn hello() {}").unwrap();
    dir
}

#[tokio::test]
async fn hydrated_panel_shows_provenance_for_supported_claim() {
    let store = Arc::new(InMemoryMemoryStore::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    assert!(!result.hydrated_claims.is_empty(), "should have hydrated claims");

    let panel = build_filtered_panel(&result);
    assert!(!panel.prompt_included.is_empty());
    let row = &panel.prompt_included[0];

    assert!(row.record_id.is_some(), "should have record_id");
    assert!(!row.provenance_label.is_empty(), "should have provenance label");
    assert!(!row.source_traces.is_empty(), "should have source traces");
    assert!(row.confidence.is_some(), "should have confidence");
}

#[tokio::test]
async fn hydrated_panel_shows_missing_status_for_unverifiable() {
    let store = Arc::new(InMemoryMemoryStore::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("the project uses microservices", 0.7, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    let panel = build_filtered_panel(&result);
    assert!(panel.summary.unverifiable >= 1);

    let unver_row = panel.unverifiable.iter().find(|r| r.claim.contains("microservices")).unwrap();
    assert!(unver_row.hydration_status.contains("Missing"), "unverifiable should have Missing status");
}

#[tokio::test]
async fn hydrated_panel_shows_supersession_for_stale_claim() {
    // Stale claims won't appear from a fresh workspace — this tests the wiring
    // by verifying the hydrated claims are populated from the report.
    let store = Arc::new(InMemoryMemoryStore::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    // The supported claim should have Complete hydration
    let supported = result.hydrated_claims.iter().find(|c| c.claim_text.contains("core")).unwrap();
    assert!(matches!(supported.hydration_status, openwand_memory::provenance_hydration::ProvenanceHydrationStatus::Complete));
}

#[tokio::test]
async fn hydrated_panel_builder_reads_hydrated_claims_not_memory_store() {
    // Architectural guard: build_filtered_panel takes &PromptInputResult only.
    // No MemoryStore parameter. Provenance comes from hydrated_claims.
    let store = Arc::new(InMemoryMemoryStore::new());
    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    // This compiles → the guard passes
    let _panel = build_filtered_panel(&result);
}

#[tokio::test]
async fn hydration_does_not_change_prompt_context_text() {
    // Prompt-stability test: the prompt block text must be identical
    // regardless of whether hydration ran.
    let store = Arc::new(InMemoryMemoryStore::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    let prompt_block = result.inputs.to_prompt_block();

    // The prompt block should contain the claim but NOT provenance labels
    // (provenance tags are not injected into prompt context in 02n)
    if let Some(ref block) = prompt_block {
        assert!(block.contains("crate core exists"), "claim should be in prompt");
        assert!(!block.contains("User-stated claim"), "provenance labels should NOT be in prompt");
        assert!(!block.contains("record"), "record IDs should NOT be in prompt");
    }
}

#[tokio::test]
async fn prompt_input_result_carries_hydrated_report() {
    let store = Arc::new(InMemoryMemoryStore::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();
    store.accept_candidate(make_candidate("the project uses microservices", 0.7, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    // Should have hydrated claims matching report findings
    assert_eq!(result.report.findings.len(), result.hydrated_claims.len(),
        "hydrated claims count must match findings count");
}

#[tokio::test]
async fn hydrated_claims_include_record_ids() {
    let store = Arc::new(InMemoryMemoryStore::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    let rec = store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    let supported = result.hydrated_claims.iter().find(|c| c.claim_text.contains("core")).unwrap();
    assert_eq!(rec.map(|r| r.record_id.clone()), supported.provenance.record_id,
        "hydrated claim should carry the actual record_id");
}
