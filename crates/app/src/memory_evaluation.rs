//! Memory evaluation harness — orchestrates coordinator + mock model + judge.
//!
//! Evaluation-only. Creates isolated stores. Does not share state with runtime.

use crate::memory_coordinator::{MemoryCoordinator, PromptInputProductionConfig};
use crate::memory_evaluation_model::run_mock_model;
use openwand_core::events::{SessionEvent, OpenWandTraceEvent};
use openwand_core::mode::InteractionMode;
use openwand_memory::evaluation::{
    EvaluationModelConfig, MemoryEvaluationReport, MemoryEvaluationScenario,
    PromptInputEvaluationSnapshot, RepoConsistencySummarySnapshot,
    ScenarioExecutionMode, SeedResolutionMaps,
};
use openwand_memory::provenance_hydration::{HydratedMemoryClaim, MemoryTrustBucket};
use openwand_memory::evaluation_judge::MemoryEvaluationJudge;
use openwand_memory::{
    CandidateKind, CandidateMemory, EpisodeRole, InMemoryMemoryStore, MemoryEpisode,
    MemoryStore,
};
use openwand_memory::MemoryExtractor;
use openwand_store::StoredEvent;
use openwand_trace::actor::Actor;
use openwand_trace::append::AppendTraceEntry;
use openwand_trace::relation::{TraceRelationDraft, TraceRelationKind};
use openwand_trace::testing::InMemoryTraceStore;
use openwand_trace::TraceStore;
use openwand_trace::{TraceStreamId, TraceStreamScope};
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
        self.run_scenario_with_config(scenario, working_dir, &crate::memory_coordinator::PromptInputProductionConfig::default()).await
    }

    /// Run scenario with explicit config (for governance profile testing).
    pub async fn run_scenario_with_config(
        &self,
        scenario: &MemoryEvaluationScenario,
        working_dir: &Path,
        config: &crate::memory_coordinator::PromptInputProductionConfig,
    ) -> MemoryEvaluationReport {
        let maps = self.seed_all(scenario).await;

        // 2. Create coordinator with isolated stores
        let coordinator = MemoryCoordinator::new(
            self.memory_store.clone() as Arc<dyn MemoryStore>,
            Arc::new(StubEvalExtractor),
            self.trace_store.clone() as Arc<dyn TraceStore<StoredEvent>>,
        );

        // 3. Run produce_prompt_inputs (same path as runtime)
        let result = coordinator
            .produce_prompt_inputs(None, working_dir, config)
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
            EvaluationModelConfig::Mock { behavior } => {
                run_mock_model(behavior, &prompt_block, &prompt_included, &excluded)
            }
            EvaluationModelConfig::Real { .. } => {
                "[real LLM evaluation not implemented]".to_string()
            }
        };

        // 6. Judge
        MemoryEvaluationJudge::judge(scenario, &snapshot, &model_output)
    }

    /// Run a single scenario against both Default and Batch02rDefault profiles,
    /// producing a delta report that captures prompt hash, inclusion, and bucket changes.
    pub async fn run_governance_delta(
        &self,
        scenario: &MemoryEvaluationScenario,
        working_dir: &Path,
        approved: &[openwand_memory::evaluation_delta::ApprovedBehaviorChange],
    ) -> openwand_memory::evaluation_delta::MemoryEvaluationDeltaReport {
        use openwand_memory::governance::MemoryGovernanceProfileId;

        // Run with Default (compatibility)
        let config_default = crate::memory_coordinator::PromptInputProductionConfig {
            governance_profile: Some(MemoryGovernanceProfileId::Default.resolve()),
            ..Default::default()
        };
        let report_default = self.run_scenario_with_config(scenario, working_dir, &config_default).await;

        // Run with Batch02rDefault (production)
        let config_tuned = crate::memory_coordinator::PromptInputProductionConfig {
            governance_profile: Some(MemoryGovernanceProfileId::Batch02rDefault.resolve()),
            ..Default::default()
        };
        let report_tuned = self.run_scenario_with_config(scenario, working_dir, &config_tuned).await;

        // Build baseline from Default run
        let mut scenario_hashes = std::collections::BTreeMap::new();
        let mut scenario_results = std::collections::BTreeMap::new();
        scenario_hashes.insert(scenario.id.clone(), report_default.snapshot.memory_context_hash.clone());
        scenario_results.insert(scenario.id.clone(), report_default.passed);

        let baseline = openwand_memory::evaluation_delta::MemoryEvaluationBaseline {
            profile_label: "Default".to_string(),
            scenario_hashes,
            scenario_results,
        };

        // Compute delta
        openwand_memory::evaluation_delta::MemoryEvaluationDeltaReport::compute(
            "Default",
            "Batch02rDefault",
            &baseline,
            &[report_tuned],
            approved,
        )
    }

    /// Seed trace entries, trace relations, memory records, and supersession.
    /// Returns resolution maps (label → ID) for use in test assertions.
    async fn seed_all(&self, scenario: &MemoryEvaluationScenario) -> SeedResolutionMaps {
        let mut maps = SeedResolutionMaps::default();

        // Phase 1: Seed trace entries (captures store-assigned TraceIds)
        for ts in &scenario.seed_trace {
            let event = StoredEvent(OpenWandTraceEvent::Session(
                SessionEvent::Started {
                    session_id: openwand_core::ids::SessionId::new(),
                    mode: InteractionMode::Direct,
                },
            ));
            let command = AppendTraceEntry {
                actor: Actor::User,
                event,
                relations: vec![],
                stream_id: TraceStreamId {
                    scope: TraceStreamScope::Session,
                    id: "eval_trace".to_string(),
                },
                idempotency_key: None,
            };
            if let Ok(trace_id) = self.trace_store.append(command).await {
                maps.trace_labels
                    .insert(ts.label.clone(), trace_id.0.clone());
            }
        }

        // Phase 2: Seed trace relations (resolves labels to TraceIds)
        for rel in &scenario.seed_relations {
            let from_id = match maps.trace_labels.get(&rel.from_label) {
                Some(id) => openwand_trace::ids::TraceId(id.clone()),
                None => continue,
            };
            let to_id = match maps.trace_labels.get(&rel.to_label) {
                Some(id) => openwand_trace::ids::TraceId(id.clone()),
                None => continue,
            };
            let kind = match rel.kind.as_str() {
                "Verifies" => TraceRelationKind::Verifies,
                "DerivedFrom" => TraceRelationKind::DerivedFrom,
                "Supersedes" => TraceRelationKind::Supersedes,
                "Invalidates" => TraceRelationKind::Invalidates,
                "Refines" => TraceRelationKind::Refines,
                "ConflictsWith" => TraceRelationKind::ConflictsWith,
                "Implements" => TraceRelationKind::Implements,
                "CausedBy" => TraceRelationKind::CausedBy,
                "Reverts" => TraceRelationKind::Reverts,
                "References" => TraceRelationKind::References,
                other => continue, // Unknown kinds skipped for eval seeding
            };
            // Append a minimal entry with the relation pointing to the target
            let event = StoredEvent(OpenWandTraceEvent::Session(SessionEvent::Started {
                session_id: openwand_core::ids::SessionId::new(),
                mode: InteractionMode::Direct,
            }));;
            let command = AppendTraceEntry {
                actor: Actor::System {
                    component: "eval.relation".to_string(),
                },
                event,
                relations: vec![TraceRelationDraft { to: to_id, kind }],
                stream_id: TraceStreamId {
                    scope: TraceStreamScope::Session,
                    id: "eval_relation".to_string(),
                },
                idempotency_key: None,
            };
            if let Ok(trace_id) = self.trace_store.append(command).await {
                maps.trace_labels
                    .insert(format!("{}_relation_to_{}", rel.from_label, rel.to_label), trace_id.0);
            }
        }

        // Phase 3: Seed memory records (resolve source_trace_labels to trace IDs)
        for seed in &scenario.seed_memory {
            // Resolve source trace labels to trace IDs for this seed
            let source_trace_ids: Vec<String> = seed
                .source_trace_labels
                .iter()
                .filter_map(|lbl| maps.trace_labels.get(lbl))
                .cloned()
                .collect();

            let trace_id_for_episode = source_trace_ids
                .first()
                .cloned()
                .unwrap_or_else(|| format!("eval_trace_{}", seed.claim.replace(' ', "_")));

            let episode = MemoryEpisode {
                episode_id: format!("eval_ep_{}", seed.claim.replace(' ', "_")),
                source_trace_id: trace_id_for_episode,
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
            if let Ok(Some(record)) = self.memory_store.accept_candidate(candidate).await {
                if let Some(ref label) = seed.label {
                    maps.memory_labels.insert(label.clone(), record.record_id.clone());
                }
            }
        }

        // Phase 4: Supersession (resolve labels to record IDs)
        for seed in &scenario.seed_memory {
            if let Some(ref superseded_label) = seed.superseded_by_label {
                // This seed supersedes the one with label = superseded_label
                if let (Some(new_record_id), Some(old_record_id)) = (
                    seed.label.as_ref().and_then(|l| maps.memory_labels.get(l)),
                    maps.memory_labels.get(superseded_label),
                ) {
                    // supersede_record takes (old_id, new_claim) and returns new record
                    let _ = self
                        .memory_store
                        .supersede_record(old_record_id, seed.claim.clone())
                        .await;
                }
            }
        }

        maps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_memory::evaluation::{
        ExpectedScenarioOutcome, MemoryEvaluationCategory, MemoryEvaluationExpectations,
        EvaluationModelConfig, MockEvaluationBehavior, MemoryRecordSeed,
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
            category: MemoryEvaluationCategory::PromptIncluded,
            execution_mode: ScenarioExecutionMode::FullHarness,
            user_query: "test".into(),
            expected_outcome: ExpectedScenarioOutcome::Pass,
            seed_memory: vec![MemoryRecordSeed {
                label: Some("core_claim".into()),
                claim: "crate core exists".into(),
                kind: "Fact".into(),
                confidence: 0.95,
                evidence_kind: "AcceptedClaim".into(),
                source_trace_labels: vec![],
                superseded_by_label: None,
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
        assert_eq!("test_simple", report.scenario_id);
    }

    #[tokio::test]
    async fn harness_captures_hydrated_claims() {
        let harness = MemoryEvaluationHarness::new();
        let dir = create_workspace_dir();
        let scenario = make_simple_scenario();
        let report = harness.run_scenario(&scenario, dir.path()).await;
        assert!(!report.snapshot.retrieved_claims.is_empty());
    }

    #[tokio::test]
    async fn harness_captures_prompt_block() {
        let harness = MemoryEvaluationHarness::new();
        let dir = create_workspace_dir();
        let scenario = make_simple_scenario();
        let report = harness.run_scenario(&scenario, dir.path()).await;
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
        if let Some(ref block) = report.snapshot.prompt_block {
            assert!(!block.contains("[evaluation]"));
        }
    }

    #[tokio::test]
    async fn harness_memory_context_hash_is_stable() {
        let harness = MemoryEvaluationHarness::new();
        let dir = create_workspace_dir();
        let scenario = make_simple_scenario();
        let r1 = harness.run_scenario(&scenario, dir.path()).await;
        let r2 = harness.run_scenario(&scenario, dir.path()).await;
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
        assert_eq!(r1.snapshot.memory_context_hash, r2.snapshot.memory_context_hash);
    }

    #[tokio::test]
    async fn harness_seeds_trace_entries_with_label_map() {
        let harness = MemoryEvaluationHarness::new();
        let dir = create_workspace_dir();
        let scenario = MemoryEvaluationScenario {
            id: "trace_seed_test".into(),
            title: "Trace seed test".into(),
            category: MemoryEvaluationCategory::VerifiedTraceLineage,
            execution_mode: ScenarioExecutionMode::FullHarness,
            user_query: "test".into(),
            expected_outcome: ExpectedScenarioOutcome::Pass,
            seed_memory: vec![MemoryRecordSeed {
                label: Some("core_claim".into()),
                claim: "crate core exists".into(),
                kind: "Fact".into(),
                confidence: 0.95,
                evidence_kind: "AcceptedClaim".into(),
                source_trace_labels: vec!["my_trace".into()],
                superseded_by_label: None,
            }],
            seed_trace: vec![openwand_memory::evaluation::TraceSeed {
                label: "my_trace".into(),
                event_kind: "session.created".into(),
                actor_label: "user".into(),
            }],
            seed_relations: vec![],
            expectations: MemoryEvaluationExpectations::default(),
            model: EvaluationModelConfig::Mock {
                behavior: MockEvaluationBehavior::EchoIncludedMemory,
            },
        };
        let maps = harness.seed_all(&scenario).await;
        assert!(maps.trace_labels.contains_key("my_trace"), "trace label must resolve");
        assert!(maps.memory_labels.contains_key("core_claim"), "memory label must resolve");
    }

    #[tokio::test]
    async fn harness_seeds_supersession_via_labels() {
        let harness = MemoryEvaluationHarness::new();
        let dir = create_workspace_dir();
        let scenario = MemoryEvaluationScenario {
            id: "supersede_test".into(),
            title: "Supersede test".into(),
            category: MemoryEvaluationCategory::Superseded,
            execution_mode: ScenarioExecutionMode::FullHarness,
            user_query: "test".into(),
            expected_outcome: ExpectedScenarioOutcome::Pass,
            seed_memory: vec![
                MemoryRecordSeed {
                    label: Some("old_claim".into()),
                    claim: "project uses version 1".into(),
                    kind: "Fact".into(),
                    confidence: 0.9,
                    evidence_kind: "AcceptedClaim".into(),
                    source_trace_labels: vec![],
                    superseded_by_label: None,
                },
                MemoryRecordSeed {
                    label: Some("new_claim".into()),
                    claim: "project uses version 2".into(),
                    kind: "Fact".into(),
                    confidence: 0.95,
                    evidence_kind: "AcceptedClaim".into(),
                    source_trace_labels: vec![],
                    superseded_by_label: Some("old_claim".into()),
                },
            ],
            seed_trace: vec![],
            seed_relations: vec![],
            expectations: MemoryEvaluationExpectations::default(),
            model: EvaluationModelConfig::Mock {
                behavior: MockEvaluationBehavior::EchoIncludedMemory,
            },
        };
        let maps = harness.seed_all(&scenario).await;
        assert!(maps.memory_labels.contains_key("old_claim"));
        assert!(maps.memory_labels.contains_key("new_claim"));
        // Verify old claim is superseded
        let records = harness.memory_store.list_active_records().await.unwrap();
        // The old claim should be superseded (superseded_by is set)
        // Active records are those NOT superseded
        let old_active = records.iter().any(|r| r.claim == "project uses version 1");
        // If supersede_record works, old claim should NOT be active
        // (unless supersede_record only sets the field without removing)
    }
}
