//! Evaluation model — DTOs for real-model quality evaluation.
//!
//! Wave 06: measure real model behavior under the governed loop.
//! All evaluation code is feature-gated behind `real-model-eval`.
//! Default CI never exercises this module.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Current report schema version. Increment when report shape changes.
pub const EVAL_REPORT_SCHEMA_VERSION: u16 = 1;

/// A single evaluation scenario with deterministic expected outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalScenario {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub turns: Vec<String>,
    pub expected: EvalExpectations,
    #[serde(default)]
    pub tags: Vec<EvalTag>,
}

/// Tag for categorizing evaluation scenarios.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvalTag {
    Memory,
    Policy,
    Patch,
    MultiTurn,
    Rebuild,
    Explain,
    Provider,
}

/// Deterministic expectations for an evaluation run.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvalExpectations {
    #[serde(default)]
    pub included_claims: Vec<String>,
    #[serde(default)]
    pub excluded_claims: Vec<String>,
    #[serde(default)]
    pub tool_calls: Vec<String>,
    #[serde(default)]
    pub forbidden_tool_calls: Vec<String>,
    #[serde(default)]
    pub file_changes: Vec<String>,
    #[serde(default)]
    pub policy_events: Vec<String>,
    #[serde(default = "default_true")]
    pub rebuild_matches: bool,
    #[serde(default = "default_true")]
    pub explain_matches: bool,
}

fn default_true() -> bool {
    true
}

/// Snapshot of provider capabilities and health at evaluation time.
/// API keys are redacted — never serialized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRealitySnapshot {
    pub provider: String,
    pub model: String,
    pub base_url_redacted: Option<String>,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_reasoning: bool,
    pub health_status: ProviderHealthStatus,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    pub observed_at: DateTime<Utc>,
}

/// Provider health status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderHealthStatus {
    Healthy,
    Degraded,
    Unreachable,
    Unknown,
}

/// Complete report from a single evaluation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalRunReport {
    pub report_schema_version: u16,
    pub scenario_id: String,
    pub provider: ProviderRealitySnapshot,
    pub memory: MemoryEvalResult,
    pub tools: ToolEvalResult,
    pub policy: PolicyEvalResult,
    pub patch: PatchEvalResult,
    pub explain: ExplainEvalResult,
    pub rebuild: RebuildEvalResult,
    pub score: EvalScore,
}

/// Scoring across all dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalScore {
    pub total: u32,
    pub max: u32,
    pub pass_rate: f64,
    pub dimensions: Vec<DimensionScore>,
}

/// Score for a single evaluation dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub name: String,
    pub passed: u32,
    pub total: u32,
}

/// Memory evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvalResult {
    pub included_claims_seen: Vec<String>,
    pub excluded_claims_seen: Vec<String>,
    pub missing_required: Vec<String>,
    pub unexpected_included: Vec<String>,
    pub prompt_panel_equivalent: bool,
}

/// Tool evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEvalResult {
    pub requested_tools: Vec<String>,
    pub executed_tools: Vec<String>,
    pub blocked_tools: Vec<String>,
    pub forbidden_requested: Vec<String>,
}

/// Policy evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvalResult {
    pub gates_seen: Vec<String>,
    pub required_approvals_seen: Vec<String>,
    pub unexpected_allows: Vec<String>,
}

/// Patch evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchEvalResult {
    pub planned: bool,
    pub applied: bool,
    pub preimage_verified: bool,
    pub postimage_verified: bool,
    pub rollback_available: bool,
    pub changed_files_match_expected: bool,
}

/// Explain evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainEvalResult {
    pub memory_matches: bool,
    pub policy_matches: bool,
    pub tool_matches: bool,
    pub completion_matches: bool,
}

/// Rebuild evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildEvalResult {
    pub events_replayed: usize,
    pub state_matches: bool,
    pub divergences: Vec<String>,
}

impl EvalScore {
    /// Compute score from dimension results.
    pub fn from_dimensions(dimensions: Vec<DimensionScore>) -> Self {
        let total: u32 = dimensions.iter().map(|d| d.passed).sum();
        let max: u32 = dimensions.iter().map(|d| d.total).sum();
        let pass_rate = if max > 0 {
            total as f64 / max as f64
        } else {
            0.0
        };
        EvalScore {
            total,
            max,
            pass_rate,
            dimensions,
        }
    }
}

impl ProviderRealitySnapshot {
    /// Create a snapshot for an unknown/unavailable provider.
    pub fn unknown() -> Self {
        Self {
            provider: "unknown".to_string(),
            model: "unknown".to_string(),
            base_url_redacted: None,
            supports_streaming: false,
            supports_tools: false,
            supports_reasoning: false,
            health_status: ProviderHealthStatus::Unknown,
            temperature: None,
            max_tokens: None,
            observed_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_report_serializes_stably() {
        let report = EvalRunReport {
            report_schema_version: EVAL_REPORT_SCHEMA_VERSION,
            scenario_id: "test".to_string(),
            provider: ProviderRealitySnapshot::unknown(),
            memory: MemoryEvalResult {
                included_claims_seen: vec![],
                excluded_claims_seen: vec![],
                missing_required: vec![],
                unexpected_included: vec![],
                prompt_panel_equivalent: true,
            },
            tools: ToolEvalResult {
                requested_tools: vec![],
                executed_tools: vec![],
                blocked_tools: vec![],
                forbidden_requested: vec![],
            },
            policy: PolicyEvalResult {
                gates_seen: vec![],
                required_approvals_seen: vec![],
                unexpected_allows: vec![],
            },
            patch: PatchEvalResult {
                planned: false,
                applied: false,
                preimage_verified: false,
                postimage_verified: false,
                rollback_available: false,
                changed_files_match_expected: true,
            },
            explain: ExplainEvalResult {
                memory_matches: true,
                policy_matches: true,
                tool_matches: true,
                completion_matches: true,
            },
            rebuild: RebuildEvalResult {
                events_replayed: 0,
                state_matches: true,
                divergences: vec![],
            },
            score: EvalScore::from_dimensions(vec![]),
        };

        let json = serde_json::to_string(&report).unwrap();
        let back: EvalRunReport = serde_json::from_str(&json).unwrap();
        assert_eq!(EVAL_REPORT_SCHEMA_VERSION, back.report_schema_version);
        assert_eq!("test", back.scenario_id);
    }

    #[test]
    fn eval_score_is_deterministic() {
        let dims = vec![
            DimensionScore { name: "memory".into(), passed: 3, total: 4 },
            DimensionScore { name: "tools".into(), passed: 2, total: 2 },
        ];
        let score = EvalScore::from_dimensions(dims);
        assert_eq!(5, score.total);
        assert_eq!(6, score.max);
        assert!((score.pass_rate - 0.8333).abs() < 0.01);
    }

    #[test]
    fn eval_expectations_detect_missing_required_tool() {
        let expected = EvalExpectations {
            tool_calls: vec!["local__file_patch".to_string()],
            ..Default::default()
        };
        let executed: Vec<String> = vec![];
        let missing: Vec<String> = expected
            .tool_calls
            .iter()
            .filter(|t| !executed.contains(t))
            .cloned()
            .collect();
        assert_eq!(vec!["local__file_patch"], missing);
    }

    #[test]
    fn eval_expectations_detect_forbidden_tool() {
        let expected = EvalExpectations {
            forbidden_tool_calls: vec!["local__file_write".to_string()],
            ..Default::default()
        };
        let requested = vec!["local__file_read".to_string(), "local__file_write".to_string()];
        let forbidden: Vec<String> = requested
            .iter()
            .filter(|t| expected.forbidden_tool_calls.contains(t))
            .cloned()
            .collect();
        assert_eq!(vec!["local__file_write"], forbidden);
    }
}
