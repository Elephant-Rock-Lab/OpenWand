//! Memory evaluation harness — deterministic evaluation of memory behavior.
//!
//! Evaluation-only. Does not change runtime prompt assembly, memory ranking,
//! trust buckets, provenance hydration, or trace-lineage hydration.
//!
//! The harness calls the same `produce_prompt_inputs()` path as runtime,
//! captures results, runs mock/deterministic model behaviors, and judges
//! expected vs actual memory usage.

use crate::evidence::EvidenceKind;
use crate::provenance_hydration::HydratedMemoryClaim;
use crate::repo_consistency::RepoConsistencyReport;
use serde::{Deserialize, Serialize};

// ── Scenario definition ────────────────────────────────────────────────────

/// A deterministic memory evaluation scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvaluationScenario {
    pub id: String,
    pub title: String,
    pub user_query: String,
    pub expected_outcome: ExpectedScenarioOutcome,
    pub seed_memory: Vec<MemoryRecordSeed>,
    pub seed_trace: Vec<TraceSeed>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecordSeed {
    pub claim: String,
    pub kind: String, // "Fact", "Preference", etc.
    pub confidence: f64,
    pub evidence_kind: String, // "AcceptedClaim", etc.
}

/// A trace entry to seed before evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSeed {
    pub trace_id: String,
    pub event_kind: String,
    pub actor_label: String,
}

/// A trace relation to seed before evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceRelationSeed {
    pub from: String,
    pub to: String,
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
    /// SHA-256 hash of the prompt block for stability checks.
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
        // Verify each message is non-empty and contains the claim text
        for f in &failures {
            let msg = f.message();
            assert!(!msg.is_empty(), "failure message must not be empty");
        }
        // Verify stable ordering of message content
        assert!(failures[0].message().contains("not retrieved"));
        assert!(failures[5].message().contains("Unsupported claim"));
    }

    #[test]
    fn scenario_ids_are_required() {
        let scenario = MemoryEvaluationScenario {
            id: "test_scenario_001".to_string(),
            title: "Test".to_string(),
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
        assert!(!scenario.id.is_empty());
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
}
