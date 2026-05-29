//! Memory evaluation harness — orchestrates coordinator + mock model + judge.
//!
//! Evaluation-only. Creates isolated stores. Does not share state with runtime.
//! Gated behind test builds only.

use crate::memory_coordinator::{MemoryCoordinator, PromptInputProductionConfig};
use crate::memory_evaluation_model::run_mock_model;
use openwand_memory::evaluation::{
    EvaluationModelConfig, MemoryEvaluationReport, MemoryEvaluationScenario,
    PromptInputEvaluationSnapshot, RepoConsistencySummarySnapshot,
};
use openwand_memory::provenance_hydration::{HydratedMemoryClaim, MemoryTrustBucket};
use openwand_memory::evaluation_judge::MemoryEvaluationJudge;
use openwand_memory::{CandidateKind, CandidateMemory, EpisodeRole, InMemoryMemoryStore, MemoryEpisode, MemoryStore};
use openwand_memory::MemoryExtractor;
use openwand_store::StoredEvent;
use openwand_trace::testing::InMemoryTraceStore;
use openwand_trace::TraceStore;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

/// Stub extractor — evaluation seeds memory directly, not via extraction.
struct StubEvalExtractor;

#[async_trait::async_trait]
impl MemoryExtractor for StubEvalExtractor {
    async fn extract(&self, _episodes: &[MemoryEpisode]) -> Vec<CandidateMemory> {
        vec![]
    }
}

/// Isolated evaluation harness. Creates its own stores.
pub struct MemoryEvaluationHarness {
    memory_store: Arc<InMemoryMemoryStore>,
    trace_store: Arc<InMemoryTraceStore<StoredEvent>>,
}

impl MemoryEvaluationHarness {
    pub fn new() -> Self {
        Self {
            memory_store: Arc::new(InMemoryMemoryStore::new()),
            trace_store: Arc::new(InMemoryTraceStore::new()),
        }
    }

    /// Seed memory + trace fixtures, run produce_prompt_inputs(), judge results.
    pub async fn run_scenario(
        &self,
        scenario: &MemoryEvaluationScenario,
        working_dir: &Path,
    ) -> MemoryEvaluationReport {
        // 1. Seed memory
        for seed in &scenario.seed_memory {
            let episode = MemoryEpisode {
                episode_id: format!("eval_ep_{}", seed.claim.replace(' ', "_")),
                source_trace_id: format!("eval_trace_{}", seed.claim.replace(' ', "_")),
                session_id: "eval_session".to_string(),
                event_kind: "evaluation.seed".to_string(),
                role: EpisodeRole::User,
                content: seed.claim.clone(),
                created_at: chrono::Utc::now(),
            };
            let _ = self.memory_store.project_episode(episode).await;
            let candidate = CandidateMemory {
                claim: seed.claim.clone(),
                kind: CandidateKind::Fact,
                confidence: seed.confidence,
                source_episode_ids: vec![format!("eval_ep_{}", seed.claim.replace(' ', "_"))],
            };
            let _ = self.memory_store.accept_candidate(candidate).await;
        }

        // 2. Create coordinator with isolated stores
        let coordinator = MemoryCoordinator::new(
            self.memory_store.clone() as Arc<dyn MemoryStore>,
            Arc::new(StubEvalExtractor),
            self.trace_store.clone() as Arc<dyn TraceStore<StoredEvent>>,
        );

        // 3. Run produce_prompt_inputs (same path as runtime)
        let result = coordinator
            .produce_prompt_inputs(None, working_dir, &PromptInputProductionConfig::default())
            .await;

        // 4. Build snapshot
        let prompt_block = result.inputs.to_prompt_block();
        let memory_context_hash = match &prompt_block {
            Some(text) => {
                let mut hasher = DefaultHasher::new();
                text.hash(&mut hasher);
                format!("{:016x}", hasher.finish())
            }
            None => String::new(),
        };

        let prompt_included: Vec<HydratedMemoryClaim> = result
            .hydrated_claims
            .iter()
            .filter(|c| matches!(c.bucket, MemoryTrustBucket::PromptIncluded))
            .cloned()
            .collect();

        let excluded: Vec<HydratedMemoryClaim> = result
            .hydrated_claims
            .iter()
            .filter(|c| !matches!(c.bucket, MemoryTrustBucket::PromptIncluded))
            .cloned()
            .collect();

        let snapshot = PromptInputEvaluationSnapshot {
            prompt_block: prompt_block.clone(),
            memory_context_hash,
            retrieved_claims: result.hydrated_claims.clone(),
            prompt_included_claims: prompt_included.clone(),
            excluded_claims: excluded.clone(),
            report_summary: RepoConsistencySummarySnapshot::from_report(&result.report),
        };

        // 5. Run model
        let model_output = match &scenario.model {
            EvaluationModelConfig::Mock { behavior } => run_mock_model(
                behavior,
                &prompt_block,
                &prompt_included,
                &excluded,
            ),
            EvaluationModelConfig::Real { manual_only: true, .. } => {
                "[real LLM evaluation not implemented]".to_string()
            }
            EvaluationModelConfig::Real { .. } => {
                "[real LLM evaluation not implemented]".to_string()
            }
        };

        // 6. Judge
        MemoryEvaluationJudge::judge(scenario, &snapshot, &model_output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_memory::evaluation::{
        ExpectedScenarioOutcome, MemoryEvaluationExpectations, EvaluationModelConfig,
        MockEvaluationBehavior, MemoryRecordSeed,
    };

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

    fn make_simple_scenario() -> MemoryEvaluationScenario {
        MemoryEvaluationScenario {
            id: "test_simple".into(),
            title: "Simple test".into(),
            user_query: "test".into(),
            expected_outcome: ExpectedScenarioOutcome::Pass,
            seed_memory: vec![MemoryRecordSeed {
                claim: "crate core exists".into(),
                kind: "Fact".into(),
                confidence: 0.95,
                evidence_kind: "AcceptedClaim".into(),
            }],
            seed_trace: vec![],
            seed_relations: vec![],
            expectations: MemoryEvaluationExpectations {
                must_retrieve: vec!["crate core exists".into()],
                ..Default::default()
            },
            model: EvaluationModelConfig::Mock {
                behavior: MockEvaluationBehavior::EchoIncludedMemory,
            },
        }
    }

    #[tokio::test]
    async fn harness_uses_existing_prompt_input_path() {
        let harness = MemoryEvaluationHarness::new();
        let dir = create_workspace_dir();
        let scenario = make_simple_scenario();
        let report = harness.run_scenario(&scenario, dir.path()).await;
        // The harness must produce a report (even if some expectations fail)
        assert_eq!("test_simple", report.scenario_id);
    }

    #[tokio::test]
    async fn harness_captures_hydrated_claims() {
        let harness = MemoryEvaluationHarness::new();
        let dir = create_workspace_dir();
        let scenario = make_simple_scenario();
        let report = harness.run_scenario(&scenario, dir.path()).await;
        // Should have retrieved at least one claim
        assert!(!report.snapshot.retrieved_claims.is_empty());
    }

    #[tokio::test]
    async fn harness_captures_prompt_block() {
        let harness = MemoryEvaluationHarness::new();
        let dir = create_workspace_dir();
        let scenario = make_simple_scenario();
        let report = harness.run_scenario(&scenario, dir.path()).await;
        // Should have a prompt block if there are findings
        if report.snapshot.report_summary.supported > 0 {
            assert!(report.snapshot.prompt_block.is_some());
        }
    }

    #[tokio::test]
    async fn harness_does_not_change_prompt_text() {
        let harness = MemoryEvaluationHarness::new();
        let dir = create_workspace_dir();
        let scenario = make_simple_scenario();
        let report = harness.run_scenario(&scenario, dir.path()).await;
        // Prompt block should contain claim but NOT evaluation artifacts
        if let Some(ref block) = report.snapshot.prompt_block {
            assert!(!block.contains("[evaluation]"), "eval artifacts must not be in prompt");
        }
    }

    #[tokio::test]
    async fn harness_memory_context_hash_is_stable() {
        let harness = MemoryEvaluationHarness::new();
        let dir = create_workspace_dir();
        let scenario = make_simple_scenario();

        let r1 = harness.run_scenario(&scenario, dir.path()).await;
        let r2 = harness.run_scenario(&scenario, dir.path()).await;

        // Same seed → same prompt → same hash
        assert_eq!(r1.snapshot.memory_context_hash, r2.snapshot.memory_context_hash);
    }

    #[tokio::test]
    async fn harness_uses_isolated_stores() {
        let h1 = MemoryEvaluationHarness::new();
        let h2 = MemoryEvaluationHarness::new();
        let dir = create_workspace_dir();
        let scenario = make_simple_scenario();

        let r1 = h1.run_scenario(&scenario, dir.path()).await;
        let r2 = h2.run_scenario(&scenario, dir.path()).await;

        // Both produce the same result with the same seed
        assert_eq!(r1.snapshot.memory_context_hash, r2.snapshot.memory_context_hash);
    }
}
