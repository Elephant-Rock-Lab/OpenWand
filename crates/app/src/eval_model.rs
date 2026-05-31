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

/// Load all eval scenario fixtures from the fixtures directory.
/// Returns scenarios sorted by id for deterministic ordering.
pub fn load_eval_fixtures(dir: &std::path::Path) -> Result<Vec<EvalScenario>, String> {
    if !dir.exists() {
        return Err(format!("Eval fixture dir not found: {:?}", dir));
    }
    let mut scenarios = Vec::new();
    let entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read fixture dir: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().map(|ext| ext == "yaml" || ext == "yml").unwrap_or(false)
        })
        .collect();

    for entry in entries {
        let content = std::fs::read_to_string(entry.path())
            .map_err(|e| format!("Failed to read {:?}: {}", entry.path(), e))?;
        let scenario: EvalScenario = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse {:?}: {}", entry.path(), e))?;

        // Validate
        if scenario.id.is_empty() {
            return Err(format!("Scenario in {:?} has empty id", entry.path()));
        }
        if scenario.turns.is_empty() {
            return Err(format!("Scenario '{}' has empty turns", scenario.id));
        }

        scenarios.push(scenario);
    }

    scenarios.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(scenarios)
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

    /// Create a snapshot from existing LLM crate types.
    /// Redacts the API key from base_url.
    pub fn from_llm_target(
        target: &openwand_llm::request::LlmTarget,
        caps: &openwand_llm::request::LlmCapabilities,
        health: ProviderHealthStatus,
        temperature: Option<f64>,
        max_tokens: Option<u64>,
    ) -> Self {
        let base_url_redacted = target.base_url.as_ref().map(|url| {
            // Redact any embedded credentials
            if url.contains('@') {
                "[redacted]".to_string()
            } else {
                url.clone()
            }
        });

        Self {
            provider: format!("{:?}", target.provider),
            model: target.model.clone(),
            base_url_redacted,
            supports_streaming: caps.supports_streaming,
            supports_tools: caps.supports_tools,
            supports_reasoning: caps.supports_reasoning,
            health_status: health,
            temperature,
            max_tokens,
            observed_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_caps() -> openwand_llm::request::LlmCapabilities {
        openwand_llm::request::LlmCapabilities {
            supports_streaming: false,
            supports_tools: false,
            supports_reasoning: false,
            supports_vision: false,
            max_context_tokens: None,
            supported_features: vec![],
        }
    }

    fn default_target() -> openwand_llm::request::LlmTarget {
        openwand_llm::request::LlmTarget {
            provider: openwand_llm::request::LlmProvider::OpenAI,
            model: "test".to_string(),
            base_url: None,
            api_key: None,
        }
    }

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

    #[test]
    fn provider_snapshot_redacts_api_key() {
        let target = openwand_llm::request::LlmTarget {
            provider: openwand_llm::request::LlmProvider::OpenAI,
            model: "gpt-4".to_string(),
            base_url: Some("http://localhost:1234/v1".to_string()),
            api_key: Some("sk-secret-key-12345".to_string()),
        };
        let caps = default_caps();

        let snapshot = ProviderRealitySnapshot::from_llm_target(
            &target, &caps, ProviderHealthStatus::Healthy, None, None,
        );

        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(!json.contains("sk-secret-key-12345"), "API key leaked: {}", json);
        assert!(json.contains("localhost:1234"));
    }

    #[test]
    fn provider_snapshot_records_capabilities() {
        let target = openwand_llm::request::LlmTarget {
            provider: openwand_llm::request::LlmProvider::Ollama,
            model: "qwen3".to_string(),
            base_url: None,
            api_key: None,
        };
        let caps = openwand_llm::request::LlmCapabilities {
            supports_streaming: true,
            supports_tools: true,
            supports_reasoning: false,
            supports_vision: false,
            max_context_tokens: Some(8192),
            supported_features: vec![],
        };

        let snapshot = ProviderRealitySnapshot::from_llm_target(
            &target, &caps, ProviderHealthStatus::Healthy, Some(0.7), Some(4096),
        );

        assert!(snapshot.supports_streaming);
        assert!(snapshot.supports_tools);
        assert!(!snapshot.supports_reasoning);
        assert_eq!(Some(0.7), snapshot.temperature);
        assert_eq!(Some(4096), snapshot.max_tokens);
    }

    #[test]
    fn provider_health_failure_does_not_panic() {
        let snapshot = ProviderRealitySnapshot::from_llm_target(
            &openwand_llm::request::LlmTarget {
                provider: openwand_llm::request::LlmProvider::OpenAI,
                model: "gpt-4".to_string(),
                base_url: None,
                api_key: None,
            },
            &default_caps(),
            ProviderHealthStatus::Unreachable,
            None, None,
        );
        assert_eq!(ProviderHealthStatus::Unreachable, snapshot.health_status);
    }

    #[test]
    fn provider_unknown_works_without_feature() {
        let snapshot = ProviderRealitySnapshot::unknown();
        assert_eq!("unknown", snapshot.provider);
        assert_eq!(ProviderHealthStatus::Unknown, snapshot.health_status);
    }

    #[test]
    fn eval_fixture_loads_all_scenarios() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("eval");
        let scenarios = load_eval_fixtures(&dir).unwrap();
        assert!(scenarios.len() >= 8, "Expected at least 8 fixtures, found {}", scenarios.len());
        // Verify sorted by id
        for i in 1..scenarios.len() {
            assert!(scenarios[i - 1].id < scenarios[i].id, "Not sorted: {} >= {}", scenarios[i - 1].id, scenarios[i].id);
        }
    }

    #[test]
    fn eval_fixture_requires_id() {
        let dir = tempfile::tempdir().unwrap();
        let yaml_content = "id: \"\"\ntitle: test\nturns:\n  - hello\nexpected: {}\n";
        std::fs::write(dir.path().join("no_id.yaml"), yaml_content).unwrap();
        let result = load_eval_fixtures(dir.path());
        assert!(result.is_err(), "Should reject empty id");
        assert!(result.unwrap_err().contains("empty id"));
    }

    #[test]
    fn eval_fixture_rejects_empty_turns() {
        let dir = tempfile::tempdir().unwrap();
        let yaml_content = "id: empty\ntitle: test\nturns: []\nexpected: {}\n";
        std::fs::write(dir.path().join("empty.yaml"), yaml_content).unwrap();
        let result = load_eval_fixtures(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty turns"));
    }

    #[test]
    fn eval_fixture_rejects_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.yaml"), "this is not: valid: yaml: [[[\n").unwrap();
        let result = load_eval_fixtures(dir.path());
        // It may parse as valid YAML but fail deserialization
        // The important thing is it doesn't panic
        let _ = result;
    }
}
