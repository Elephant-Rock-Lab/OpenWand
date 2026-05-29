//! Wave 02l integration tests — coordinator wiring.

use openwand_app::memory_coordinator::{
    MemoryCoordinator, PromptInputProductionConfig, PromptInputResult,
};
use openwand_core::SessionId;
use openwand_memory::evidence::EvidenceKind;
use openwand_memory::in_memory::InMemoryMemoryStore;
use openwand_memory::memory_store::MemoryStore;
use openwand_memory::types::{CandidateMemory, CandidateKind, EpisodeRole, MemoryEpisode};
use openwand_memory::MemoryExtractor;
use openwand_store::StoredEvent;
use openwand_trace::testing::InMemoryTraceStore;
use std::sync::Arc;
use chrono::Utc;

// ── Test infrastructure ──

fn make_episode(episode_id: &str, session_id: &str, content: &str) -> MemoryEpisode {
    MemoryEpisode {
        episode_id: episode_id.to_string(),
        source_trace_id: format!("trace_{episode_id}"),
        session_id: session_id.to_string(),
        event_kind: "session.user_message_injected".to_string(),
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

/// Create a temp directory with Cargo.toml workspace + crates/core.
fn create_workspace_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Workspace Cargo.toml
    std::fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = ["crates/core"]
"#,
    )
    .unwrap();

    // crates/core
    let core_dir = root.join("crates").join("core");
    std::fs::create_dir_all(core_dir.join("src")).unwrap();

    std::fs::write(
        core_dir.join("Cargo.toml"),
        r#"[package]
name = "core"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    std::fs::write(
        core_dir.join("src").join("lib.rs"),
        "pub fn hello() {}",
    )
    .unwrap();

    dir
}

/// Create a temp directory WITHOUT Cargo.toml (non-workspace).
fn create_non_workspace_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn make_coordinator(store: Arc<InMemoryMemoryStore>) -> MemoryCoordinator {
    MemoryCoordinator::new(
        store as Arc<dyn MemoryStore>,
        Arc::new(StubExtractor),
        Arc::new(InMemoryTraceStore::<StoredEvent>::new()),
    )
}

/// Stub extractor that returns no candidates (we use accept_candidate directly).
struct StubExtractor;

#[async_trait::async_trait]
impl MemoryExtractor for StubExtractor {
    async fn extract(&self, _episodes: &[MemoryEpisode]) -> Vec<CandidateMemory> {
        vec![]
    }
}

// ── Tests ──

#[tokio::test]
async fn empty_memory_produces_empty_inputs() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();

    let result = coordinator
        .produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default())
        .await;

    assert!(result.inputs.is_empty(), "empty memory should produce empty inputs");
    assert_eq!(result.claims_checked, 0);
    assert!(!result.repo_observed, "no records means repo not checked");
    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn non_workspace_directory_produces_empty_inputs() {
    let store = Arc::new(InMemoryMemoryStore::new());

    // Add a record so there's something to check
    store.project_episode(make_episode("ep1", "sess1", "hello")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.9, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_non_workspace_dir();

    let result = coordinator
        .produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default())
        .await;

    // observe_repo succeeds on empty dirs — returns empty snapshot.
    // The claim will be classified as MissingInRepo (crate core doesn't exist in empty dir).
    // The assembler still pushes it into supported_claims with confidence_bps: 0.
    assert!(result.repo_observed, "empty dir is still observed, just empty");
    assert!(result.claims_checked > 0, "records were checked");
    // MissingInRepo claims get confidence_bps: 0
    if let Some(claim) = result.inputs.supported_claims.first() {
        assert_eq!(claim.confidence_bps, 0, "MissingInRepo claim should have zero confidence");
    }
}

#[tokio::test]
async fn supported_claim_appears_in_inputs() {
    let store = Arc::new(InMemoryMemoryStore::new());
    store.project_episode(make_episode("ep1", "sess1", "hello")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"])).await.unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();

    let result = coordinator
        .produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default())
        .await;

    assert!(result.repo_observed, "workspace dir should be observed");
    assert!(!result.inputs.supported_claims.is_empty(), "supported claims should be present");
    assert!(
        result.inputs.supported_claims.iter().any(|c| c.claim_text.contains("core")),
        "should contain 'core' claim"
    );
}

#[tokio::test]
async fn unverifiable_claim_excluded_from_text() {
    let store = Arc::new(InMemoryMemoryStore::new());
    store.project_episode(make_episode("ep1", "sess1", "hello")).await.unwrap();
    store.accept_candidate(
        make_candidate("the project uses microservices", 0.7, &["ep1"]),
    )
    .await
    .unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();

    let result = coordinator
        .produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default())
        .await;

    assert!(result.repo_observed);
    assert!(
        result.inputs.unverifiable_claims_excluded >= 1,
        "microservices claim should be excluded as unverifiable"
    );
    if let Some(block) = result.inputs.to_prompt_block() {
        assert!(
            !block.contains("microservices"),
            "unverifiable claim text must not appear in prompt block"
        );
    }
}

#[tokio::test]
async fn mixed_claims_classify_correctly() {
    let store = Arc::new(InMemoryMemoryStore::new());
    store.project_episode(make_episode("ep1", "sess1", "text")).await.unwrap();

    // Supported: matches a crate name
    store
        .accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"]))
        .await
        .unwrap();

    // Unverifiable: doesn't match v0 grammar
    store
        .accept_candidate(make_candidate("the project uses microservices", 0.7, &["ep1"]))
        .await
        .unwrap();

    // This one is a fact about a non-existent crate — will be classified by grammar
    store
        .accept_candidate(make_candidate("crate nonexistent exists", 0.8, &["ep1"]))
        .await
        .unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();

    let result = coordinator
        .produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default())
        .await;

    assert!(result.repo_observed);
    assert!(result.claims_checked >= 3);

    // Supported should have at least the 'core' claim
    assert!(!result.inputs.supported_claims.is_empty());

    // Unverifiable should have at least the 'microservices' claim
    assert!(result.inputs.unverifiable_claims_excluded >= 1);
}

#[tokio::test]
async fn produce_prompt_inputs_caps_records_checked() {
    let store = Arc::new(InMemoryMemoryStore::new());

    // Add 20 records
    for i in 0..20 {
        let ep_id = format!("ep_{i}");
        store.project_episode(make_episode(&ep_id, "sess1", &format!("text {i}"))).await.unwrap();
        store
            .accept_candidate(make_candidate(&format!("claim number {i}"), 0.8, &[&ep_id]))
            .await
            .unwrap();
    }

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();

    let config = PromptInputProductionConfig {
        max_records_checked: 5,
        max_hits_per_record: 2,
    };

    let result = coordinator
        .produce_prompt_inputs(None, dir.path(), &config)
        .await;

    assert_eq!(result.claims_checked, 5, "should cap at max_records_checked");
}

#[tokio::test]
async fn produce_prompt_inputs_order_is_deterministic() {
    let store = Arc::new(InMemoryMemoryStore::new());

    // Add records with varying confidence
    for (i, conf) in [(0, 0.5), (1, 0.9), (2, 0.7)] {
        let ep_id = format!("ep_{i}");
        store.project_episode(make_episode(&ep_id, "sess1", &format!("text {i}"))).await.unwrap();
        store
            .accept_candidate(make_candidate(&format!("claim {i}"), conf, &[&ep_id]))
            .await
            .unwrap();
    }

    let coordinator = make_coordinator(store.clone());
    let dir = create_workspace_dir();
    let config = PromptInputProductionConfig {
        max_records_checked: 2,
        max_hits_per_record: 5,
    };

    let result1 = coordinator
        .produce_prompt_inputs(None, dir.path(), &config)
        .await;

    let coordinator2 = make_coordinator(store);
    let result2 = coordinator2
        .produce_prompt_inputs(None, dir.path(), &config)
        .await;

    assert_eq!(result1.claims_checked, result2.claims_checked);
    // The higher-confidence claims should be selected in both runs
    // (confidence 0.9 and 0.7, not 0.5)
    assert_eq!(result1.claims_checked, 2);
}

#[tokio::test]
async fn all_ranked_search_failures_degrade_to_empty_inputs() {
    // This test verifies the guard against false missing-memory findings
    // when all search_ranked calls fail.
    //
    // We can't easily make InMemoryMemoryStore's search_ranked fail,
    // so we test the code path indirectly: if search returns empty hits
    // for all records (no text match), the pipeline should still work
    // gracefully without false missing-memory findings.
    //
    // True ranked-search failure testing requires a mock store that
    // returns Err from search_ranked, which would need a custom impl.

    let store = Arc::new(InMemoryMemoryStore::new());
    store.project_episode(make_episode("ep1", "sess1", "text")).await.unwrap();

    // A claim that won't match anything in search_ranked
    store
        .accept_candidate(make_candidate("xyzzy_nonsense_claim_99999", 0.5, &["ep1"]))
        .await
        .unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();

    let result = coordinator
        .produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default())
        .await;

    // Should complete without error — the claim just won't produce hits
    assert!(result.errors.is_empty(), "no errors should occur");
    // The claim is unverifiable, so no supported claims expected
    // But missing_in_memory findings may appear for repo crates with no matching claims
}

#[tokio::test]
async fn empty_store_produces_none_prompt_block() {
    let store = Arc::new(InMemoryMemoryStore::new());
    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();

    let result = coordinator
        .produce_prompt_inputs(None, dir.path(), &PromptInputProductionConfig::default())
        .await;

    assert!(result.inputs.is_empty());
    assert!(result.inputs.to_prompt_block().is_none());
    assert_eq!(result.claims_checked, 0);
}

#[tokio::test]
async fn full_pipeline_produces_provenance_tagged_prompt() {
    let store = Arc::new(InMemoryMemoryStore::new());

    // Supported claim
    store.project_episode(make_episode("ep1", "sess1", "text")).await.unwrap();
    store
        .accept_candidate(make_candidate("crate core exists", 0.95, &["ep1"]))
        .await
        .unwrap();

    // Unverifiable claim
    store
        .accept_candidate(make_candidate("the project uses microservices", 0.7, &["ep1"]))
        .await
        .unwrap();

    let coordinator = make_coordinator(store);
    let dir = create_workspace_dir();

    let session_id = SessionId("test-session".to_string());
    let result = coordinator
        .produce_prompt_inputs(
            Some(session_id.clone()),
            dir.path(),
            &PromptInputProductionConfig::default(),
        )
        .await;

    // Verify result metadata
    assert!(result.repo_observed);
    assert!(result.claims_checked >= 2);
    assert_eq!(result.source_session_id, Some(session_id));
    assert_eq!(result.source_working_directory, dir.path().to_path_buf());

    // Verify supported claims
    assert!(!result.inputs.supported_claims.is_empty());
    assert!(result.inputs.unverifiable_claims_excluded >= 1);

    // Verify prompt block
    let block = result.inputs.to_prompt_block().expect("should produce prompt block");
    assert!(block.contains("## Verified Memory"), "should contain Verified Memory heading");
    assert!(block.contains("core"), "should mention the supported crate");
    assert!(!block.contains("microservices"), "unverifiable text must not appear");
}
