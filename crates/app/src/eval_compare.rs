//! Evaluation comparison engine.
//!
//! Compare evaluation reports across runs, models, providers.
//! Detect regressions and improvements using configurable thresholds.

use crate::eval_model::*;
use crate::eval_reports::EvalReportStore;
use std::path::PathBuf;

/// How to select the baseline for comparison.
#[derive(Debug, Clone)]
pub enum EvalBaselineSelection {
    /// No comparison — just produce the current report.
    None,
    /// Use the latest stored report for the same scenario.
    Latest,
    /// Use a specific report file.
    Path(PathBuf),
}

/// Delta between two scores.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScoreDelta {
    pub current_total: u32,
    pub baseline_total: Option<u32>,
    pub delta: Option<i32>,
    pub current_pass_rate: f64,
    pub baseline_pass_rate: Option<f64>,
}

/// Delta for a single dimension.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DimensionDelta {
    pub dimension: String,
    pub current_score: u32,
    pub baseline_score: Option<u32>,
    pub delta: Option<i32>,
}

/// A detected regression.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvalRegression {
    pub dimension: String,
    pub description: String,
    pub severity: RegressionSeverity,
}

/// A detected improvement.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvalImprovement {
    pub dimension: String,
    pub description: String,
}

/// Regression severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RegressionSeverity {
    /// Required dimension went from pass to fail.
    Hard,
    /// Score dropped beyond threshold.
    Soft,
    /// Evidence missing (anti-vacuous-pass).
    Evidence,
}

/// Change in provider metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderDelta {
    pub provider_changed: bool,
    pub model_changed: bool,
    pub current_provider: String,
    pub baseline_provider: Option<String>,
    pub current_model: String,
    pub baseline_model: Option<String>,
}

/// Complete comparison between current and baseline reports.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvalComparisonReport {
    pub scenario_id: String,
    pub current_report_path: Option<PathBuf>,
    pub baseline_report_path: Option<PathBuf>,
    pub score_delta: ScoreDelta,
    pub dimension_deltas: Vec<DimensionDelta>,
    pub provider_delta: ProviderDelta,
    pub regressions: Vec<EvalRegression>,
    pub improvements: Vec<EvalImprovement>,
}

/// Thresholds for regression detection.
#[derive(Debug, Clone)]
pub struct RegressionThresholds {
    /// Maximum allowed score drop (total points).
    pub max_score_drop: i32,
    /// Maximum allowed pass rate drop (0.0–1.0).
    pub max_pass_rate_drop: f64,
    /// Dimensions that must not regress.
    pub required_dimensions: Vec<String>,
}

impl Default for RegressionThresholds {
    fn default() -> Self {
        Self {
            max_score_drop: 5,
            max_pass_rate_drop: 0.05,
            required_dimensions: vec!["rebuild".to_string(), "policy".to_string()],
        }
    }
}

/// Resolve the baseline selection to a concrete report.
pub fn resolve_baseline(
    selection: &EvalBaselineSelection,
    store: &EvalReportStore,
    scenario_id: &str,
) -> Result<Option<EvalRunReport>, String> {
    match selection {
        EvalBaselineSelection::None => Ok(None),
        EvalBaselineSelection::Latest => {
            match store.latest_for_scenario(scenario_id)? {
                Some(stored) => {
                    // Validate scenario match
                    if stored.report.scenario_id != scenario_id {
                        return Err(format!(
                            "Baseline scenario mismatch: expected '{}', got '{}'",
                            scenario_id, stored.report.scenario_id
                        ));
                    }
                    // Validate schema compatibility
                    if stored.report.report_schema_version > EVAL_REPORT_SCHEMA_VERSION {
                        return Err(format!(
                            "Baseline schema {} is newer than supported {}",
                            stored.report.report_schema_version, EVAL_REPORT_SCHEMA_VERSION
                        ));
                    }
                    Ok(Some(stored.report))
                }
                None => Ok(None),
            }
        }
        EvalBaselineSelection::Path(path) => {
            let report = store.load_report(path)?;
            if report.scenario_id != scenario_id {
                return Err(format!(
                    "Baseline scenario mismatch: expected '{}', got '{}'",
                    scenario_id, report.scenario_id
                ));
            }
            Ok(Some(report))
        }
    }
}

/// Compare a current report against a baseline.
pub fn compare_reports(
    current: &EvalRunReport,
    baseline: Option<&EvalRunReport>,
    thresholds: &RegressionThresholds,
) -> EvalComparisonReport {
    let baseline_ref = baseline;

    // Score delta
    let score_delta = ScoreDelta {
        current_total: current.score.total,
        baseline_total: baseline_ref.map(|b| b.score.total),
        delta: baseline_ref.map(|b| current.score.total as i32 - b.score.total as i32),
        current_pass_rate: current.score.pass_rate,
        baseline_pass_rate: baseline_ref.map(|b| b.score.pass_rate),
    };

    // Provider delta
    let provider_delta = ProviderDelta {
        provider_changed: baseline_ref.map_or(false, |b| b.provider.provider != current.provider.provider),
        model_changed: baseline_ref.map_or(false, |b| b.provider.model != current.provider.model),
        current_provider: current.provider.provider.clone(),
        baseline_provider: baseline_ref.map(|b| b.provider.provider.clone()),
        current_model: current.provider.model.clone(),
        baseline_model: baseline_ref.map(|b| b.provider.model.clone()),
    };

    // Dimension deltas
    let mut dimension_deltas = Vec::new();
    for dim in &current.score.dimensions {
        let baseline_score = baseline_ref.and_then(|b| {
            b.score.dimensions.iter()
                .find(|d| d.name == dim.name)
                .map(|d| d.passed)
        });
        dimension_deltas.push(DimensionDelta {
            dimension: dim.name.clone(),
            current_score: dim.passed,
            baseline_score,
            delta: baseline_score.map(|bs| dim.passed as i32 - bs as i32),
        });
    }

    // Detect regressions
    let mut regressions = Vec::new();
    let mut improvements = Vec::new();

    if let Some(b) = baseline_ref {
        // Score drop
        let score_diff = current.score.total as i32 - b.score.total as i32;
        if score_diff < -thresholds.max_score_drop {
            regressions.push(EvalRegression {
                dimension: "overall".to_string(),
                description: format!("Score dropped by {} (threshold: {})", -score_diff, thresholds.max_score_drop),
                severity: RegressionSeverity::Soft,
            });
        }

        // Pass rate drop
        let pass_rate_diff = current.score.pass_rate - b.score.pass_rate;
        if pass_rate_diff < -thresholds.max_pass_rate_drop {
            regressions.push(EvalRegression {
                dimension: "overall".to_string(),
                description: format!("Pass rate dropped by {:.1}% (threshold: {:.1}%)",
                    -pass_rate_diff * 100.0, thresholds.max_pass_rate_drop * 100.0),
                severity: RegressionSeverity::Soft,
            });
        }

        // Required dimension regression
        for req_dim in &thresholds.required_dimensions {
            let current_dim = current.score.dimensions.iter().find(|d| d.name == *req_dim);
            let baseline_dim = b.score.dimensions.iter().find(|d| d.name == *req_dim);

            if let (Some(cd), Some(bd)) = (current_dim, baseline_dim) {
                if cd.passed < bd.passed && bd.passed > 0 {
                    regressions.push(EvalRegression {
                        dimension: req_dim.clone(),
                        description: format!(
                            "Required dimension '{}' regressed: {}/{} → {}/{}",
                            req_dim, cd.passed, cd.total, bd.passed, bd.total
                        ),
                        severity: RegressionSeverity::Hard,
                    });
                }
            }
        }

        // Detect improvements
        if score_diff > 0 {
            improvements.push(EvalImprovement {
                dimension: "overall".to_string(),
                description: format!("Score improved by {} points", score_diff),
            });
        }

        for dd in &dimension_deltas {
            if let Some(delta) = dd.delta {
                if delta > 0 {
                    improvements.push(EvalImprovement {
                        dimension: dd.dimension.clone(),
                        description: format!("{} improved by {} points", dd.dimension, delta),
                    });
                }
            }
        }
    }

    // Anti-vacuous-pass: check evidence presence
    if current.score.max == 0 {
        regressions.push(EvalRegression {
            dimension: "evidence".to_string(),
            description: "No evaluation evidence collected (anti-vacuous-pass)".to_string(),
            severity: RegressionSeverity::Evidence,
        });
    }

    EvalComparisonReport {
        scenario_id: current.scenario_id.clone(),
        current_report_path: None,
        baseline_report_path: None,
        score_delta,
        dimension_deltas,
        provider_delta,
        regressions,
        improvements,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scored_report(scenario: &str, total: u32, max: u32, dims: Vec<(&str, u32, u32)>) -> EvalRunReport {
        let dimensions: Vec<DimensionScore> = dims.into_iter()
            .map(|(n, p, t)| DimensionScore { name: n.to_string(), passed: p, total: t, evidence_refs: vec![] })
            .collect();
        EvalRunReport {
            report_schema_version: EVAL_REPORT_SCHEMA_VERSION,
            scenario_id: scenario.to_string(),
            provider: ProviderRealitySnapshot {
                provider: "test".to_string(),
                model: "test-model".to_string(),
                base_url_redacted: None,
                supports_streaming: false,
                supports_tools: false,
                supports_reasoning: false,
                health_status: ProviderHealthStatus::Healthy,
                temperature: None,
                max_tokens: None,
                observed_at: chrono::Utc::now(),
            },
            prompt: PromptEvalResult::default(),
            memory: MemoryEvalResult {
                included_claims_seen: vec![], excluded_claims_seen: vec![],
                missing_required: vec![], unexpected_included: vec![],
                prompt_panel_equivalent: true,
            },
            tools: ToolEvalResult {
                requested_tools: vec![], executed_tools: vec![],
                blocked_tools: vec![], forbidden_requested: vec![],
            },
            policy: PolicyEvalResult {
                gates_seen: vec![], required_approvals_seen: vec![],
                unexpected_allows: vec![],
            },
            patch: PatchEvalResult {
                planned: false, applied: false, preimage_verified: false,
                postimage_verified: false, rollback_available: false,
                changed_files_match_expected: true,
            },
            explain: ExplainEvalResult {
                memory_matches: true, policy_matches: true,
                tool_matches: true, completion_matches: true,
            },
            rebuild: RebuildEvalResult {
                events_replayed: 0, state_matches: true, divergences: vec![],
            },
            capability_context: CapabilityContextEvalResult::default(),
            score: EvalScore::from_dimensions(dimensions),
            // Override the computed score with explicit values for testing
        }
    }

    #[test]
    fn eval_compare_detects_score_drop() {
        let baseline = make_scored_report("test", 80, 100, vec![
            ("memory", 20, 20), ("policy", 20, 20), ("rebuild", 20, 20), ("tools", 20, 20),
        ]);
        let current = make_scored_report("test", 70, 100, vec![
            ("memory", 15, 20), ("policy", 20, 20), ("rebuild", 20, 20), ("tools", 15, 20),
        ]);
        // Recalculate score properly
        let mut current = current;
        current.score = EvalScore::from_dimensions(vec![
            DimensionScore { name: "memory".into(), passed: 15, total: 20, evidence_refs: vec![] },
            DimensionScore { name: "policy".into(), passed: 20, total: 20, evidence_refs: vec![] },
            DimensionScore { name: "rebuild".into(), passed: 20, total: 20, evidence_refs: vec![] },
            DimensionScore { name: "tools".into(), passed: 15, total: 20, evidence_refs: vec![] },
        ]);

        let thresholds = RegressionThresholds {
            max_score_drop: 5,
            ..Default::default()
        };
        let report = compare_reports(&current, Some(&baseline), &thresholds);
        assert!(!report.regressions.is_empty(), "Should detect score drop of 10");
        assert!(report.regressions.iter().any(|r| r.dimension == "overall"));
    }

    #[test]
    fn eval_compare_detects_improvement() {
        let baseline = make_scored_report("test", 70, 100, vec![]);
        let current = make_scored_report("test", 80, 100, vec![]);
        let mut current = current;
        current.score.total = 80;
        current.score.pass_rate = 0.8;

        let mut baseline = baseline;
        baseline.score.total = 70;
        baseline.score.pass_rate = 0.7;

        let report = compare_reports(&current, Some(&baseline), &Default::default());
        assert!(!report.improvements.is_empty());
    }

    #[test]
    fn eval_compare_handles_no_baseline() {
        let mut current = make_scored_report("test", 80, 100, vec![]);
        current.score = EvalScore::from_dimensions(vec![
            DimensionScore { name: "memory".into(), passed: 10, total: 10, evidence_refs: vec![] },
        ]);
        let report = compare_reports(&current, None, &Default::default());
        assert!(report.regressions.is_empty());
        assert!(report.improvements.is_empty());
        assert!(report.score_delta.baseline_total.is_none());
    }

    #[test]
    fn eval_compare_rejects_scenario_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        let report = make_scored_report("scenario_a", 80, 100, vec![]);
        let path = store.save_report(&report).unwrap();

        let result = resolve_baseline(
            &EvalBaselineSelection::Path(path),
            &store,
            "scenario_b",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mismatch"));
    }

    #[test]
    fn eval_baseline_none_skips_comparison() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());
        let result = resolve_baseline(&EvalBaselineSelection::None, &store, "test").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn eval_baseline_latest_selects_latest() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        let mut report1 = make_scored_report("test", 70, 100, vec![]);
        report1.score.total = 70;
        store.save_report(&report1).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut report2 = make_scored_report("test", 80, 100, vec![]);
        report2.score.total = 80;
        store.save_report(&report2).unwrap();

        let result = resolve_baseline(&EvalBaselineSelection::Latest, &store, "test").unwrap();
        assert!(result.is_some());
        // The latest is the newest saved, which should be report2 (80)
        assert_eq!(80, result.unwrap().score.total);
    }

    #[test]
    fn eval_compare_detects_required_dimension_regression() {
        let baseline = EvalRunReport {
            score: EvalScore::from_dimensions(vec![
                DimensionScore { name: "rebuild".into(), passed: 10, total: 10, evidence_refs: vec![] },
                DimensionScore { name: "policy".into(), passed: 10, total: 10, evidence_refs: vec![] },
            ]),
            ..make_scored_report("test", 0, 0, vec![])
        };
        let current = EvalRunReport {
            score: EvalScore::from_dimensions(vec![
                DimensionScore { name: "rebuild".into(), passed: 5, total: 10, evidence_refs: vec![] },
                DimensionScore { name: "policy".into(), passed: 10, total: 10, evidence_refs: vec![] },
            ]),
            ..make_scored_report("test", 0, 0, vec![])
        };

        let thresholds = RegressionThresholds {
            required_dimensions: vec!["rebuild".to_string()],
            ..Default::default()
        };
        let report = compare_reports(&current, Some(&baseline), &thresholds);
        assert!(report.regressions.iter().any(|r|
            r.dimension == "rebuild" && r.severity == RegressionSeverity::Hard
        ));
    }
}
