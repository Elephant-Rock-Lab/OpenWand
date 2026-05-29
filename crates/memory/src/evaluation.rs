//! Memory evaluation harness — deterministic evaluation of memory behavior.
//!
//! Evaluation-only. Does not change runtime prompt assembly, memory ranking,
//! trust buckets, provenance hydration, or trace-lineage hydration.
//!
//! The harness calls the same `produce_prompt_inputs()` path as runtime,
//! captures results, runs mock/deterministic model behaviors, and judges
//! expected vs actual memory usage.

use crate::provenance_hydration::HydratedMemoryClaim;
use crate::repo_consistency::RepoConsistencyReport;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ── Category ───────────────────────────────────────────────────────────────

/// Taxonomy of memory evaluation scenario categories.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MemoryEvaluationCategory {
    PromptIncluded,
    Stale,
    Superseded,
    Conflict,
    Unverifiable,
    MissingInRepo,
    MissingInMemory,
    VerifiedTraceLineage,
    LowConfidence,
    UnsupportedOutput,
}

impl MemoryEvaluationCategory {
    /// All known categories, in canonical order.
    pub fn all() -> &'static [MemoryEvaluationCategory] {
        &[
            MemoryEvaluationCategory::PromptIncluded,
            MemoryEvaluationCategory::Stale,
            MemoryEvaluationCategory::Superseded,
            MemoryEvaluationCategory::Conflict,
            MemoryEvaluationCategory::Unverifiable,
            MemoryEvaluationCategory::MissingInRepo,
            MemoryEvaluationCategory::MissingInMemory,
            MemoryEvaluationCategory::VerifiedTraceLineage,
            MemoryEvaluationCategory::LowConfidence,
            MemoryEvaluationCategory::UnsupportedOutput,
        ]
    }
}

// ── Scenario execution mode ────────────────────────────────────────────────

/// How a scenario is executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioExecutionMode {
    /// Seed memory + trace, run coordinator, judge results.
    FullHarness,
    /// Construct hydrated claims directly, judge only (no coordinator).
    JudgeOnly,
}

impl Default for ScenarioExecutionMode {
    fn default() -> Self {
        ScenarioExecutionMode::FullHarness
    }
}

// ── Scenario definition ────────────────────────────────────────────────────

/// A deterministic memory evaluation scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvaluationScenario {
    pub id: String,
    pub title: String,
    /// Required — every fixture must declare its category explicitly.
    pub category: MemoryEvaluationCategory,
    /// How this scenario is executed. Defaults to FullHarness.
    #[serde(default)]
    pub execution_mode: ScenarioExecutionMode,
    pub user_query: String,
    pub expected_outcome: ExpectedScenarioOutcome,
    #[serde(default)]
    pub seed_memory: Vec<MemoryRecordSeed>,
    #[serde(default)]
    pub seed_trace: Vec<TraceSeed>,
    #[serde(default)]
    pub seed_relations: Vec<TraceRelationSeed>,
    pub expectations: MemoryEvaluationExpectations,
    pub model: EvaluationModelConfig,
}

/// Whether this scenario is expected to pass or fail evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpectedScenarioOutcome {
    Pass,
    Fail,
}

/// A memory record to seed into the store before evaluation.
/// Uses stable labels for cross-referencing (patches 5+6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecordSeed {
    /// Stable label for cross-referencing within the scenario.
    /// Used by superseded_by_label and source_trace_labels.
    #[serde(default)]
    pub label: Option<String>,
    pub claim: String,
    pub kind: String,
    pub confidence: f64,
    pub evidence_kind: String,
    /// Labels of trace seeds that should be linked as source traces
    /// for this memory record. Resolved via the trace-label map.
    #[serde(default)]
    pub source_trace_labels: Vec<String>,
    /// Label of another memory seed that this record supersedes.
    /// Harness resolves labels to record IDs after all seeds are inserted.
    #[serde(default)]
    pub superseded_by_label: Option<String>,
}

/// A trace entry to seed before evaluation. Uses labels, not store-assigned IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSeed {
    /// Stable label for cross-referencing (memory seeds, relations).
    pub label: String,
    pub event_kind: String,
    pub actor_label: String,
}

/// A trace relation to seed before evaluation. Uses labels resolved to TraceIds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceRelationSeed {
    pub from_label: String,
    pub to_label: String,
    pub kind: String,
}

// ── Expectations ───────────────────────────────────────────────────────────

/// What the evaluator expects from memory behavior.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryEvaluationExpectations {
    #[serde(default)]
    pub must_retrieve: Vec<String>,
    #[serde(default)]
    pub must_include_in_prompt: Vec<String>,
    #[serde(default)]
    pub must_exclude_from_prompt: Vec<String>,
    #[serde(default)]
    pub must_use_in_answer: Vec<String>,
    #[serde(default)]
    pub must_not_use_in_answer: Vec<String>,
    #[serde(default)]
    pub expected_buckets: Vec<ExpectedBucketAssignment>,
    #[serde(default)]
    pub expected_provenance: Vec<ExpectedProvenanceAssertion>,
    #[serde(default)]
    pub expected_trace_lineage: Vec<ExpectedTraceLineageAssertion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedBucketAssignment {
    pub claim: String,
    pub bucket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedProvenanceAssertion {
    pub claim: String,
    pub has_record_id: bool,
    pub has_source_trace_ids: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedTraceLineageAssertion {
    pub claim: String,
    pub has_lineage: bool,
    #[serde(default)]
    pub expected_relation_count: Option<usize>,
}

// ── Report ─────────────────────────────────────────────────────────────────

/// The result of evaluating a single scenario.
#[derive(Debug, Clone)]
pub struct MemoryEvaluationReport {
    pub scenario_id: String,
    pub passed: bool,
    pub snapshot: PromptInputEvaluationSnapshot,
    pub model_output: String,
    pub failures: Vec<MemoryEvaluationFailure>,
    pub warnings: Vec<String>,
}

/// Snapshot of prompt inputs captured during evaluation.
#[derive(Debug, Clone)]
pub struct PromptInputEvaluationSnapshot {
    pub prompt_block: Option<String>,
    /// Hash of the prompt block for stability checks.
    pub memory_context_hash: String,
    pub retrieved_claims: Vec<HydratedMemoryClaim>,
    pub prompt_included_claims: Vec<HydratedMemoryClaim>,
    pub excluded_claims: Vec<HydratedMemoryClaim>,
    /// Summary of repo consistency report findings (not a copy of RepoConsistencyReport).
    pub report_summary: RepoConsistencySummarySnapshot,
}

/// Snapshot of repo consistency report findings — evaluation-only type.
/// Named distinctly to avoid confusion with the canonical report type.
#[derive(Debug, Clone, Default)]
pub struct RepoConsistencySummarySnapshot {
    pub total_findings: usize,
    pub supported: usize,
    pub stale: usize,
    pub missing_in_repo: usize,
    pub missing_in_memory: usize,
    pub unverifiable: usize,
    pub superseded: usize,
    pub conflict: usize,
}

impl RepoConsistencySummarySnapshot {
    pub fn from_report(report: &RepoConsistencyReport) -> Self {
        use crate::repo_consistency::RepoConsistencyFindingKind;
        let mut snap = Self::default();
        snap.total_findings = report.findings.len();
        for f in &report.findings {
            match f.kind {
                RepoConsistencyFindingKind::Supported => snap.supported += 1,
                RepoConsistencyFindingKind::StaleMemory => snap.stale += 1,
                RepoConsistencyFindingKind::MissingInRepo => snap.missing_in_repo += 1,
                RepoConsistencyFindingKind::MissingInMemory => snap.missing_in_memory += 1,
                RepoConsistencyFindingKind::Unverifiable => snap.unverifiable += 1,
                RepoConsistencyFindingKind::SupersededMemoryIgnored => snap.superseded += 1,
                RepoConsistencyFindingKind::ConflictRequiresReview => snap.conflict += 1,
            }
        }
        snap
    }
}

/// A specific evaluation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryEvaluationFailure {
    ExpectedClaimNotRetrieved { claim: String },
    ExpectedClaimNotPromptIncluded { claim: String },
    ForbiddenClaimPromptIncluded { claim: String },
    ExpectedClaimNotUsedByModel { claim: String },
    ForbiddenClaimUsedByModel { claim: String },
    /// Model produced a claim not backed by any retrieved or included memory.
    UnsupportedClaimUsedByModel { claim: String },
    WrongTrustBucket { claim: String, expected: String, actual: String },
    MissingProvenance { claim: String },
    MissingTraceLineage { claim: String },
}

impl MemoryEvaluationFailure {
    pub fn message(&self) -> String {
        match self {
            Self::ExpectedClaimNotRetrieved { claim } => format!("Expected claim not retrieved: {}", claim),
            Self::ExpectedClaimNotPromptIncluded { claim } => format!("Expected claim not in prompt: {}", claim),
            Self::ForbiddenClaimPromptIncluded { claim } => format!("Forbidden claim in prompt: {}", claim),
            Self::ExpectedClaimNotUsedByModel { claim } => format!("Expected claim not used by model: {}", claim),
            Self::ForbiddenClaimUsedByModel { claim } => format!("Forbidden claim used by model: {}", claim),
            Self::UnsupportedClaimUsedByModel { claim } => format!("Unsupported claim used by model: {}", claim),
            Self::WrongTrustBucket { claim, expected, actual } => {
                format!("Wrong bucket for '{}': expected {}, actual {}", claim, expected, actual)
            }
            Self::MissingProvenance { claim } => format!("Missing provenance for: {}", claim),
            Self::MissingTraceLineage { claim } => format!("Missing trace lineage for: {}", claim),
        }
    }
}

// ── Model config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvaluationModelConfig {
    Mock { behavior: MockEvaluationBehavior },
    Real { provider: String, model: String, manual_only: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MockEvaluationBehavior {
    EchoIncludedMemory,
    IgnoreIncludedMemory,
    UseExcludedMemory,
    HallucinateUnsupportedClaim,
    CorrectAnswer { text: String },
}

// ── Coverage ───────────────────────────────────────────────────────────────

/// Label-to-ID resolution maps produced by the harness during seeding.
#[derive(Debug, Clone, Default)]
pub struct SeedResolutionMaps {
    /// label → TraceId (from trace seeding)
    pub trace_labels: BTreeMap<String, String>,
    /// label → record_id (from memory seeding)
    pub memory_labels: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_report_passes_with_no_failures() {
        let report = MemoryEvaluationReport {
            scenario_id: "test_pass".to_string(),
            passed: true,
            snapshot: PromptInputEvaluationSnapshot {
                prompt_block: Some("test prompt".to_string()),
                memory_context_hash: "abc123".to_string(),
                retrieved_claims: vec![],
                prompt_included_claims: vec![],
                excluded_claims: vec![],
                report_summary: RepoConsistencySummarySnapshot::default(),
            },
            model_output: "test output".to_string(),
            failures: vec![],
            warnings: vec![],
        };
        assert!(report.passed);
        assert!(report.failures.is_empty());
    }

    #[test]
    fn evaluation_report_fails_with_failures() {
        let report = MemoryEvaluationReport {
            scenario_id: "test_fail".to_string(),
            passed: false,
            snapshot: PromptInputEvaluationSnapshot {
                prompt_block: None,
                memory_context_hash: String::new(),
                retrieved_claims: vec![],
                prompt_included_claims: vec![],
                excluded_claims: vec![],
                report_summary: RepoConsistencySummarySnapshot::default(),
            },
            model_output: String::new(),
            failures: vec![MemoryEvaluationFailure::ExpectedClaimNotRetrieved {
                claim: "test claim".to_string(),
            }],
            warnings: vec![],
        };
        assert!(!report.passed);
        assert_eq!(1, report.failures.len());
    }

    #[test]
    fn failure_messages_are_stable() {
        let failures = vec![
            MemoryEvaluationFailure::ExpectedClaimNotRetrieved { claim: "a".into() },
            MemoryEvaluationFailure::ExpectedClaimNotPromptIncluded { claim: "b".into() },
            MemoryEvaluationFailure::ForbiddenClaimPromptIncluded { claim: "c".into() },
            MemoryEvaluationFailure::ExpectedClaimNotUsedByModel { claim: "d".into() },
            MemoryEvaluationFailure::ForbiddenClaimUsedByModel { claim: "e".into() },
            MemoryEvaluationFailure::UnsupportedClaimUsedByModel { claim: "f".into() },
            MemoryEvaluationFailure::WrongTrustBucket { claim: "g".into(), expected: "X".into(), actual: "Y".into() },
            MemoryEvaluationFailure::MissingProvenance { claim: "h".into() },
            MemoryEvaluationFailure::MissingTraceLineage { claim: "i".into() },
        ];
        for f in &failures {
            let msg = f.message();
            assert!(!msg.is_empty(), "failure message must not be empty");
        }
        assert!(failures[0].message().contains("not retrieved"));
        assert!(failures[5].message().contains("Unsupported claim"));
    }

    #[test]
    fn scenario_category_is_required() {
        let scenario = MemoryEvaluationScenario {
            id: "test_scenario_001".to_string(),
            title: "Test".to_string(),
            category: MemoryEvaluationCategory::PromptIncluded,
            execution_mode: ScenarioExecutionMode::FullHarness,
            user_query: "test query".to_string(),
            expected_outcome: ExpectedScenarioOutcome::Pass,
            seed_memory: vec![],
            seed_trace: vec![],
            seed_relations: vec![],
            expectations: MemoryEvaluationExpectations::default(),
            model: EvaluationModelConfig::Mock {
                behavior: MockEvaluationBehavior::EchoIncludedMemory,
            },
        };
        assert_eq!(MemoryEvaluationCategory::PromptIncluded, scenario.category);
    }

    #[test]
    fn scenario_category_serializes_stably() {
        let json = serde_json::to_string(&MemoryEvaluationCategory::PromptIncluded).unwrap();
        assert_eq!("\"prompt_included\"", json);

        let json = serde_json::to_string(&MemoryEvaluationCategory::VerifiedTraceLineage).unwrap();
        assert_eq!("\"verified_trace_lineage\"", json);

        // Roundtrip
        let cat: MemoryEvaluationCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(MemoryEvaluationCategory::VerifiedTraceLineage, cat);
    }

    #[test]
    fn all_categories_are_ten() {
        assert_eq!(10, MemoryEvaluationCategory::all().len());
    }

    #[test]
    fn expectations_default_to_empty() {
        let exp = MemoryEvaluationExpectations::default();
        assert!(exp.must_retrieve.is_empty());
        assert!(exp.must_include_in_prompt.is_empty());
        assert!(exp.must_exclude_from_prompt.is_empty());
        assert!(exp.must_use_in_answer.is_empty());
        assert!(exp.must_not_use_in_answer.is_empty());
        assert!(exp.expected_buckets.is_empty());
        assert!(exp.expected_provenance.is_empty());
        assert!(exp.expected_trace_lineage.is_empty());
    }

    #[test]
    fn seed_has_label_fields() {
        let seed = MemoryRecordSeed {
            label: Some("claim_a".into()),
            claim: "crate core exists".into(),
            kind: "Fact".into(),
            confidence: 0.95,
            evidence_kind: "AcceptedClaim".into(),
            source_trace_labels: vec!["trace_1".into()],
            superseded_by_label: None,
        };
        assert_eq!(Some("claim_a"), seed.label.as_deref());
        assert_eq!(vec!["trace_1"], seed.source_trace_labels);
    }

    #[test]
    fn trace_seed_uses_label_not_id() {
        let seed = TraceSeed {
            label: "my_trace".into(),
            event_kind: "session.started".into(),
            actor_label: "user".into(),
        };
        assert_eq!("my_trace", seed.label);
        assert!(!seed.label.is_empty());
    }

    #[test]
    fn relation_seed_uses_labels() {
        let seed = TraceRelationSeed {
            from_label: "trace_a".into(),
            to_label: "trace_b".into(),
            kind: "Verifies".into(),
        };
        assert_eq!("trace_a", seed.from_label);
        assert_eq!("trace_b", seed.to_label);
    }

    #[test]
    fn execution_mode_defaults_to_full_harness() {
        let mode: ScenarioExecutionMode = Default::default();
        assert_eq!(ScenarioExecutionMode::FullHarness, mode);
    }

    #[test]
    fn old_fixtures_without_category_default_field_rejected() {
        // Verify that category is required (no serde default)
        let json = r#"{
            "id": "test",
            "title": "test",
            "user_query": "test",
            "expected_outcome": "Pass",
            "seed_memory": [],
            "seed_trace": [],
            "seed_relations": [],
            "expectations": {},
            "model": {"Mock": {"behavior": "EchoIncludedMemory"}}
        }"#;
        let result: Result<MemoryEvaluationScenario, _> = serde_json::from_str(json);
        assert!(result.is_err(), "category field is required, must not deserialize without it");
    }
}
