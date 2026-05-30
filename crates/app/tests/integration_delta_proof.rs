//! Wave 02t — Integration delta proof.
//!
//! Proves that Batch02rDefault produces different prompt output than Default
//! through the real coordinator path with real search_ranked hits.
//!
//! Root cause of 02s gap: InMemoryMemoryStore rejected claims below 0.7,
//! so the low-confidence governance exclusion zone (< 0.3) was unreachable.
//!
//! 02t fix: configurable threshold lets integration tests seed claims at 0.25
//! confidence (2500 bps) — accepted by store, excluded by governance.

use openwand_app::memory_coordinator::{MemoryCoordinator, PromptInputProductionConfig};
use openwand_core::ids::SessionId;
use openwand_memory::governance::{
    MemoryGovernanceProfileId,
};
use openwand_memory::{
    MemoryStore, MemoryEpisode, EpisodeRole, CandidateMemory, CandidateKind,
};
use openwand_memory::supersession::RetrievalMode;
use openwand_store::envelope::StoredEvent;
use openwand_trace::store::TraceStore;

use std::sync::Arc;

// StubExtractor is private in the app crate. Create a local one.
use openwand_memory::extractor::MemoryExtractor;
use async_trait::async_trait;

struct StubExtractor;
#[async_trait]
impl MemoryExtractor for StubExtractor {
    async fn extract(&self, _episodes: &[MemoryEpisode]) -> Vec<CandidateMemory> {
        vec![]
    }
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

/// Seed a claim directly into the store (bypasses evaluation harness scenario path).
async fn seed_claim_directly(
    store: &Arc<dyn MemoryStore>,
    claim: &str,
    confidence: f64,
) {
    let episode = MemoryEpisode {
        episode_id: format!("int_ep_{}", claim.replace(' ', "_")),
        source_trace_id: "int_trace".to_string(),
        session_id: "int_session".to_string(),
        event_kind: "integration".to_string(),
        role: EpisodeRole::User,
        content: claim.to_string(),
        created_at: chrono::Utc::now(),
    };
    store.project_episode(episode).await.unwrap();

    let candidate = CandidateMemory {
        claim: claim.to_string(),
        kind: CandidateKind::Fact,
        confidence,
        source_episode_ids: vec![format!("int_ep_{}", claim.replace(' ', "_"))],
    };
    let result = store.accept_candidate(candidate).await;
    assert!(result.unwrap().is_some(), "Claim '{}' at confidence {} must be accepted", claim, confidence);
}

// ── Commit 2: Integration delta fixture + proof ─────────────────────────────

#[tokio::test]
async fn integration_fixture_produces_search_ranked_hits() {
    // Prove that the fixture claim produces real search_ranked hits
    let store = openwand_memory::in_memory::InMemoryMemoryStore::with_confidence_threshold(0.1);
    let store_arc: Arc<dyn MemoryStore> = Arc::new(store);

    seed_claim_directly(&store_arc, "crate core exists", 0.25).await;

    let records = store_arc.list_active_records().await.unwrap();
    assert_eq!(1, records.len(), "Store must contain exactly one record");

    let query = openwand_memory::query::MemoryQuery::new("crate core exists");
    let ctx = store_arc.search_ranked(query, RetrievalMode::CurrentState).await.unwrap();
    assert!(!ctx.hits.is_empty(), "search_ranked must produce hits for the seeded claim");
    assert_eq!(0.25, ctx.hits[0].confidence_bps as f64 / 10000.0);
}

#[tokio::test]
async fn low_confidence_delta_visible_through_real_coordinator() {
    // THE CENTRAL 02t PROOF TEST.
    // Seeds a 2500 bps claim, runs through real coordinator with both profiles,
    // proves prompt hash/content differs.

    let store = openwand_memory::in_memory::InMemoryMemoryStore::with_confidence_threshold(0.1);
    let store_arc: Arc<dyn MemoryStore> = Arc::new(store);
    let trace_arc: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        openwand_trace::testing::InMemoryTraceStore::new()
    );

    let claim = "crate core exists";
    seed_claim_directly(&store_arc, claim, 0.25).await;

    let coordinator = MemoryCoordinator::new(
        store_arc.clone(),
        Arc::new(StubExtractor),
        trace_arc.clone(),
    );

    let dir = create_workspace_dir();

    // Run with Default (compatibility)
    let config_default = PromptInputProductionConfig {
        governance_profile: Some(MemoryGovernanceProfileId::Default.resolve()),
        ..Default::default()
    };
    let result_default = coordinator
        .produce_prompt_inputs(None, dir.path(), &config_default)
        .await;

    // Run with Batch02rDefault (production)
    let config_tuned = PromptInputProductionConfig {
        governance_profile: Some(MemoryGovernanceProfileId::Batch02rDefault.resolve()),
        ..Default::default()
    };
    let result_tuned = coordinator
        .produce_prompt_inputs(None, dir.path(), &config_tuned)
        .await;

    // Both must have retrieved the claim
    assert!(!result_default.inputs.supported_claims.is_empty() || result_default.inputs.missing_memory_gaps.is_empty(),
        "Default result must have content");
    assert!(result_tuned.repo_observed, "Tuned result must observe repo");

    // The prompt hashes must differ
    let prompt_default = result_default.inputs.to_prompt_block();
    let prompt_tuned = result_tuned.inputs.to_prompt_block();

    assert!(prompt_default.is_some(), "Default prompt must be non-empty");
    // Tuned prompt may be empty (claim excluded) or non-empty with different content

    // The critical assertion: prompt hashes differ
    let hash_default = {
        use std::hash::{Hash, Hasher};
        let text = prompt_default.as_ref().unwrap();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut h);
        format!("{:016x}", h.finish())
    };
    let hash_tuned = match &prompt_tuned {
        Some(text) => {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            text.hash(&mut h);
            format!("{:016x}", h.finish())
        }
        None => String::new(),
    };

    assert_ne!(hash_default, hash_tuned,
        "THE 02t PROOF: Default and Batch02rDefault must produce different prompt hashes for 2500 bps claim");

    // Content-level assertions
    assert!(prompt_default.as_ref().unwrap().contains("crate core"),
        "Default prompt must contain the claim text");
    match &prompt_tuned {
        Some(text) => assert!(!text.contains("crate core"),
            "Batch02rDefault prompt must NOT contain the excluded claim"),
        None => {} // Empty prompt = claim excluded. This is correct.
    }

    // Audit visibility: the claim must still appear in the report
    let has_claim_in_report = result_tuned.report.findings.iter()
        .any(|f| f.claim_text.as_deref() == Some(claim));
    assert!(has_claim_in_report, "Batch02rDefault report must still contain the excluded claim for audit");
}

#[tokio::test]
async fn default_prompt_contains_low_confidence_claim() {
    // Granular: Default includes the 2500 bps claim
    let store = openwand_memory::in_memory::InMemoryMemoryStore::with_confidence_threshold(0.1);
    let store_arc: Arc<dyn MemoryStore> = Arc::new(store);
    let trace_arc: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        openwand_trace::testing::InMemoryTraceStore::new()
    );

    let claim = "crate core exists";
    seed_claim_directly(&store_arc, claim, 0.25).await;

    let coordinator = MemoryCoordinator::new(
        store_arc, Arc::new(StubExtractor), trace_arc,
    );
    let dir = create_workspace_dir();

    let config = PromptInputProductionConfig {
        governance_profile: Some(MemoryGovernanceProfileId::Default.resolve()),
        ..Default::default()
    };
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &config).await;
    let prompt = result.inputs.to_prompt_block();

    assert!(prompt.is_some());
    assert!(prompt.as_ref().unwrap().contains("crate core"));
}

#[tokio::test]
async fn batch_02r_prompt_omits_low_confidence_claim() {
    // Granular: Batch02rDefault excludes the 2500 bps claim from prompt
    let store = openwand_memory::in_memory::InMemoryMemoryStore::with_confidence_threshold(0.1);
    let store_arc: Arc<dyn MemoryStore> = Arc::new(store);
    let trace_arc: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        openwand_trace::testing::InMemoryTraceStore::new()
    );

    let claim = "crate core exists";
    seed_claim_directly(&store_arc, claim, 0.25).await;

    let coordinator = MemoryCoordinator::new(
        store_arc, Arc::new(StubExtractor), trace_arc,
    );
    let dir = create_workspace_dir();

    let config = PromptInputProductionConfig {
        governance_profile: Some(MemoryGovernanceProfileId::Batch02rDefault.resolve()),
        ..Default::default()
    };
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &config).await;
    let prompt = result.inputs.to_prompt_block();

    match &prompt {
        Some(text) => assert!(!text.contains("crate core"),
            "Batch02rDefault must exclude 2500 bps claim from prompt"),
        None => {} // Empty = excluded. Correct.
    }
}

#[tokio::test]
async fn batch_02r_audit_still_shows_excluded_claim() {
    // The 2500 bps claim must still be in the report even when excluded from prompt
    let store = openwand_memory::in_memory::InMemoryMemoryStore::with_confidence_threshold(0.1);
    let store_arc: Arc<dyn MemoryStore> = Arc::new(store);
    let trace_arc: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        openwand_trace::testing::InMemoryTraceStore::new()
    );

    let claim = "crate core exists";
    seed_claim_directly(&store_arc, claim, 0.25).await;

    let coordinator = MemoryCoordinator::new(
        store_arc, Arc::new(StubExtractor), trace_arc,
    );
    let dir = create_workspace_dir();

    let config = PromptInputProductionConfig {
        governance_profile: Some(MemoryGovernanceProfileId::Batch02rDefault.resolve()),
        ..Default::default()
    };
    let result = coordinator.produce_prompt_inputs(None, dir.path(), &config).await;

    // Report must contain the finding
    let findings_with_claim = result.report.findings.iter()
        .filter(|f| f.claim_text.as_deref() == Some(claim))
        .count();
    assert!(findings_with_claim > 0, "Excluded claim must still appear in report findings");
}

// ── Commit 3: High-confidence stability proof ───────────────────────────────

#[tokio::test]
async fn high_confidence_hash_stable_across_profiles() {
    // A 0.9 confidence claim must be included under both profiles
    let store = openwand_memory::in_memory::InMemoryMemoryStore::with_confidence_threshold(0.1);
    let store_arc: Arc<dyn MemoryStore> = Arc::new(store);
    let trace_arc: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        openwand_trace::testing::InMemoryTraceStore::new()
    );

    let claim = "crate core exists";
    seed_claim_directly(&store_arc, claim, 0.9).await;

    let coordinator = MemoryCoordinator::new(
        store_arc, Arc::new(StubExtractor), trace_arc,
    );
    let dir = create_workspace_dir();

    let config_default = PromptInputProductionConfig {
        governance_profile: Some(MemoryGovernanceProfileId::Default.resolve()),
        ..Default::default()
    };
    let r_default = coordinator.produce_prompt_inputs(None, dir.path(), &config_default).await;

    let config_tuned = PromptInputProductionConfig {
        governance_profile: Some(MemoryGovernanceProfileId::Batch02rDefault.resolve()),
        ..Default::default()
    };
    let r_tuned = coordinator.produce_prompt_inputs(None, dir.path(), &config_tuned).await;

    let p_default = r_default.inputs.to_prompt_block();
    let p_tuned = r_tuned.inputs.to_prompt_block();

    assert!(p_default.is_some(), "Default prompt must be non-empty for 0.9 confidence");
    assert!(p_tuned.is_some(), "Batch02rDefault prompt must be non-empty for 0.9 confidence");

    // Hashes must be identical
    assert_eq!(p_default, p_tuned,
        "High confidence (0.9) claim must produce identical prompts under both profiles");
}

#[tokio::test]
async fn high_confidence_included_under_both_profiles() {
    let store = openwand_memory::in_memory::InMemoryMemoryStore::with_confidence_threshold(0.1);
    let store_arc: Arc<dyn MemoryStore> = Arc::new(store);
    let trace_arc: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        openwand_trace::testing::InMemoryTraceStore::new()
    );

    let claim = "crate core exists";
    seed_claim_directly(&store_arc, claim, 0.9).await;

    let coordinator = MemoryCoordinator::new(
        store_arc, Arc::new(StubExtractor), trace_arc,
    );
    let dir = create_workspace_dir();

    for profile_id in [MemoryGovernanceProfileId::Default, MemoryGovernanceProfileId::Batch02rDefault] {
        let config = PromptInputProductionConfig {
            governance_profile: Some(profile_id.resolve()),
            ..Default::default()
        };
        let result = coordinator.produce_prompt_inputs(None, dir.path(), &config).await;
        let prompt = result.inputs.to_prompt_block();
        assert!(prompt.as_ref().map(|p| p.contains("crate core")).unwrap_or(false),
            "{:?} must include 0.9 confidence claim in prompt", profile_id);
    }
}
