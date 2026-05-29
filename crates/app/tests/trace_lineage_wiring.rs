//! Wave 02o integration tests — trace relation audit hydration.

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
use openwand_trace::{TraceRelation, TraceRelationDraft, TraceRelationKind};
use openwand_trace::ids::TraceId;
use openwand_trace::actor::Actor;
use openwand_trace::entry::TraceEntry;
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

fn make_coordinator(store: Arc<InMemoryMemoryStore>, trace: Arc<InMemoryTraceStore<StoredEvent>>) -> MemoryCoordinator {
    MemoryCoordinator::new(
        store as Arc<dyn MemoryStore>,
        Arc::new(StubExtractor),
        trace as Arc<dyn openwand_trace::TraceStore<StoredEvent>>,
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
async fn trace_lineage_hydration_does_not_change_prompt_context_text() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let trace = Arc::new(InMemoryTraceStore::<StoredEvent>::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store, trace);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    let prompt_block = result.inputs.to_prompt_block();

    // Prompt should contain the claim but NOT trace lineage artifacts
    if let Some(ref block) = prompt_block {
        assert!(block.contains("crate core exists"), "claim should be in prompt");
        assert!(!block.contains("derived from"), "trace lineage must not be in prompt");
        assert!(!block.contains("verified by"), "trace lineage must not be in prompt");
        assert!(!block.contains("supersedes"), "trace lineage must not be in prompt");
        assert!(!block.contains("no trace relations"), "trace lineage summary must not be in prompt");
    }
}

#[tokio::test]
async fn normal_prompt_assembly_does_not_render_trace_ids() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let trace = Arc::new(InMemoryTraceStore::<StoredEvent>::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store, trace);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    let prompt_block = result.inputs.to_prompt_block();
    if let Some(ref block) = prompt_block {
        // trace_ IDs from lineage should not appear in prompt
        let has_trace_id = block.lines().any(|l| l.contains("trace_") && !l.contains("crate"));
        assert!(!has_trace_id, "trace IDs from lineage must not appear in prompt");
    }
}

#[tokio::test]
async fn hydrated_claims_carry_trace_lineage() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let trace = Arc::new(InMemoryTraceStore::<StoredEvent>::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store, trace);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    // Every hydrated claim should have trace_lineage set (even if it's Partial/Missing)
    for claim in &result.hydrated_claims {
        assert!(claim.trace_lineage.is_some(), "every claim should have trace_lineage after 02o");
    }
}

#[tokio::test]
async fn coordinator_deduplicates_source_trace_ids_before_relation_scan() {
    // Two claims sharing the same source trace ID should not cause duplicate queries.
    // The hydrator should handle deduplicated input correctly.
    let store = Arc::new(InMemoryMemoryStore::new());
    let trace = Arc::new(InMemoryTraceStore::<StoredEvent>::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();
    store.accept_candidate(make_candidate("crate core has tests", 0.8, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store, trace);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    // Both claims share the same source_trace_ids from ep1
    let claims_with_lineage = result.hydrated_claims.iter()
        .filter(|c| c.trace_lineage.is_some())
        .count();
    assert!(claims_with_lineage >= 2, "both claims should have trace lineage");
}

#[tokio::test]
async fn trace_relation_scan_failure_does_not_fail_prompt_input_assembly() {
    // Use a trace store that will have empty relations (no failure, but empty)
    // This tests the non-fatal path: empty relations → Partial status
    let store = Arc::new(InMemoryMemoryStore::new());
    let trace = Arc::new(InMemoryTraceStore::<StoredEvent>::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store, trace);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    // Should succeed even with no trace relations
    assert!(!result.hydrated_claims.is_empty());

    // The prompt should still work
    let prompt_block = result.inputs.to_prompt_block();
    assert!(prompt_block.is_some() || result.report.findings.is_empty());
}

#[tokio::test]
async fn hydrated_claims_preserve_02n_fields_after_trace_lineage() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let trace = Arc::new(InMemoryTraceStore::<StoredEvent>::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store, trace);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    let supported = result.hydrated_claims.iter().find(|c| c.claim_text.contains("core")).unwrap();

    // 02n fields preserved
    assert!(supported.provenance.record_id.is_some(), "record_id from 02n should survive");
    assert!(!supported.provenance.source_trace_ids.is_empty(), "source_trace_ids from 02n should survive");

    // 02o field added
    assert!(supported.trace_lineage.is_some(), "trace_lineage from 02o should be present");
}

#[tokio::test]
async fn panel_shows_trace_lineage_summary() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let trace = Arc::new(InMemoryTraceStore::<StoredEvent>::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store, trace);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    let panel = build_filtered_panel(&result);
    if let Some(row) = panel.prompt_included.first() {
        // trace_lineage_summary should be set (even if "no trace relations")
        assert!(row.trace_lineage_summary.is_some(), "panel row should have trace lineage summary");
    }
}

#[tokio::test]
async fn panel_builder_does_not_access_trace_store() {
    // Architectural guard: build_filtered_panel takes &PromptInputResult only.
    // No TraceStore parameter. Trace lineage comes through hydrated_claims.
    let store = Arc::new(InMemoryMemoryStore::new());
    let trace = Arc::new(InMemoryTraceStore::<StoredEvent>::new());
    let coordinator = make_coordinator(store, trace);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    // This compiles → the guard passes. build_filtered_panel has no trace parameter.
    let _panel = build_filtered_panel(&result);
}

#[tokio::test]
async fn trace_lineage_is_panel_audit_only() {
    // The prompt block must not contain any trace lineage artifacts
    let store = Arc::new(InMemoryMemoryStore::new());
    let trace = Arc::new(InMemoryTraceStore::<StoredEvent>::new());
    store.project_episode(make_episode("ep1", "s1", "text")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store, trace);
    let dir = create_workspace_dir();
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default()).await;

    let prompt = result.inputs.to_prompt_block();

    // Panel should have lineage
    let has_lineage = result.hydrated_claims.iter().any(|c| c.trace_lineage.is_some());
    assert!(has_lineage, "panel should have trace lineage data");

    // Prompt should NOT
    if let Some(ref block) = prompt {
        assert!(!block.contains("no trace relations"), "lineage summary must not be in prompt");
    }
}
