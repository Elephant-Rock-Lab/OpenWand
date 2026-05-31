//! Longitudinal summary reports.
//!
//! Aggregate stored evaluation reports into trend summaries.
//! Score over time, scenario failure frequency, provider changes.

use crate::eval_compare::*;
use crate::eval_model::*;
use crate::eval_reports::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A longitudinal summary across all stored reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSummaryReport {
    pub generated_at: DateTime<Utc>,
    pub report_schema_version: u16,
    pub total_reports: usize,
    pub scenario_summaries: Vec<ScenarioTrendSummary>,
    pub provider_summaries: Vec<ProviderTrendSummary>,
    pub latest_regressions: Vec<EvalRegression>,
    pub latest_improvements: Vec<EvalImprovement>,
}

/// Trend summary for a single scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioTrendSummary {
    pub scenario_id: String,
    pub run_count: usize,
    pub latest_score: u32,
    pub latest_max: u32,
    pub best_score: u32,
    pub worst_score: u32,
    pub trend: ScoreTrend,
    pub latest_provider: String,
    pub latest_model: String,
    pub first_run_at: Option<DateTime<Utc>>,
    pub latest_run_at: Option<DateTime<Utc>>,
}

/// Trend direction for scores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreTrend {
    Improving,
    Stable,
    Declining,
    InsufficientData,
}

/// Trend summary for a provider/model combination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTrendSummary {
    pub provider: String,
    pub model: String,
    pub run_count: usize,
    pub avg_score: f64,
    pub avg_pass_rate: f64,
    pub scenario_coverage: usize,
}

/// Generate a summary report from all stored reports.
pub fn generate_summary(store: &EvalReportStore) -> Result<EvalSummaryReport, String> {
    let reports = store.list_reports(&ReportFilter::default())?;

    if reports.is_empty() {
        return Ok(EvalSummaryReport {
            generated_at: Utc::now(),
            report_schema_version: EVAL_REPORT_SCHEMA_VERSION,
            total_reports: 0,
            scenario_summaries: vec![],
            provider_summaries: vec![],
            latest_regressions: vec![],
            latest_improvements: vec![],
        });
    }

    // Group by scenario
    let mut by_scenario: BTreeMap<String, Vec<&StoredEvalReport>> = BTreeMap::new();
    for r in &reports {
        by_scenario
            .entry(r.report.scenario_id.clone())
            .or_default()
            .push(r);
    }

    // Generate scenario summaries
    let mut scenario_summaries = Vec::new();
    let mut all_regressions = Vec::new();
    let mut all_improvements = Vec::new();

    for (id, scenario_reports) in &by_scenario {
        let scores: Vec<u32> = scenario_reports.iter().map(|r| r.report.score.total).collect();
        let latest = scenario_reports.first().unwrap(); // sorted newest-first
        let earliest = scenario_reports.last().unwrap();

        // Determine trend by comparing latest 3 runs
        let trend = if scores.len() < 2 {
            ScoreTrend::InsufficientData
        } else {
            let recent: Vec<u32> = scores.iter().take(3).cloned().collect();
            let older: Vec<u32> = scores.iter().skip(3).take(3).cloned().collect();
            if older.is_empty() {
                ScoreTrend::InsufficientData
            } else {
                let recent_avg: f64 = recent.iter().sum::<u32>() as f64 / recent.len() as f64;
                let older_avg: f64 = older.iter().sum::<u32>() as f64 / older.len() as f64;
                if recent_avg > older_avg + 1.0 {
                    ScoreTrend::Improving
                } else if recent_avg < older_avg - 1.0 {
                    ScoreTrend::Declining
                } else {
                    ScoreTrend::Stable
                }
            }
        };

        scenario_summaries.push(ScenarioTrendSummary {
            scenario_id: id.clone(),
            run_count: scenario_reports.len(),
            latest_score: latest.report.score.total,
            latest_max: latest.report.score.max,
            best_score: scores.iter().max().copied().unwrap_or(0),
            worst_score: scores.iter().min().copied().unwrap_or(0),
            trend,
            latest_provider: latest.report.provider.provider.clone(),
            latest_model: latest.report.provider.model.clone(),
            first_run_at: Some(earliest.report.provider.observed_at),
            latest_run_at: Some(latest.report.provider.observed_at),
        });

        // Compare latest two runs for regression detection
        if scenario_reports.len() >= 2 {
            let current = &scenario_reports[0].report;
            let baseline = &scenario_reports[1].report;
            let comparison = compare_reports(
                current,
                Some(baseline),
                &RegressionThresholds::default(),
            );
            all_regressions.extend(comparison.regressions);
            all_improvements.extend(comparison.improvements);
        }
    }

    // Group by provider/model
    let mut by_provider: BTreeMap<(String, String), Vec<&StoredEvalReport>> = BTreeMap::new();
    for r in &reports {
        let key = (r.report.provider.provider.clone(), r.report.provider.model.clone());
        by_provider.entry(key).or_default().push(r);
    }

    let provider_summaries: Vec<ProviderTrendSummary> = by_provider
        .iter()
        .map(|((provider, model), provider_reports)| {
            let scores: Vec<u32> = provider_reports.iter().map(|r| r.report.score.total).collect();
            let pass_rates: Vec<f64> = provider_reports.iter().map(|r| r.report.score.pass_rate).collect();
            let scenarios: std::collections::HashSet<&str> = provider_reports
                .iter()
                .map(|r| r.report.scenario_id.as_str())
                .collect();

            ProviderTrendSummary {
                provider: provider.clone(),
                model: model.clone(),
                run_count: provider_reports.len(),
                avg_score: scores.iter().sum::<u32>() as f64 / scores.len() as f64,
                avg_pass_rate: pass_rates.iter().sum::<f64>() / pass_rates.len() as f64,
                scenario_coverage: scenarios.len(),
            }
        })
        .collect();

    Ok(EvalSummaryReport {
        generated_at: Utc::now(),
        report_schema_version: EVAL_REPORT_SCHEMA_VERSION,
        total_reports: reports.len(),
        scenario_summaries,
        provider_summaries,
        latest_regressions: all_regressions,
        latest_improvements: all_improvements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_report(scenario_id: &str, score: u32) -> EvalRunReport {
        EvalRunReport {
            report_schema_version: EVAL_REPORT_SCHEMA_VERSION,
            scenario_id: scenario_id.to_string(),
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
            score: EvalScore::from_dimensions(vec![
                DimensionScore { name: "memory".into(), passed: score, total: 100, evidence_refs: vec![] },
            ]),
        }
    }

    #[test]
    fn eval_summary_groups_by_scenario() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        store.save_report(&make_report("alpha", 80)).unwrap();
        store.save_report(&make_report("beta", 70)).unwrap();

        let summary = generate_summary(&store).unwrap();
        assert_eq!(2, summary.scenario_summaries.len());
        assert_eq!(2, summary.total_reports);
    }

    #[test]
    fn eval_summary_groups_by_provider() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        let mut r1 = make_report("a", 80);
        r1.provider.provider = "ollama".to_string();
        r1.provider.model = "llama3".to_string();
        store.save_report(&r1).unwrap();

        let mut r2 = make_report("b", 70);
        r2.provider.provider = "ollama".to_string();
        r2.provider.model = "llama3".to_string();
        store.save_report(&r2).unwrap();

        let summary = generate_summary(&store).unwrap();
        assert_eq!(1, summary.provider_summaries.len());
        assert_eq!("ollama", summary.provider_summaries[0].provider);
        assert_eq!(2, summary.provider_summaries[0].scenario_coverage);
    }

    #[test]
    fn eval_summary_detects_latest_regressions() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        // Save baseline first (higher score), then current (lower)
        let mut baseline = make_report("test", 80);
        baseline.provider.model = "model_v1".to_string();
        store.save_report(&baseline).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut current = make_report("test", 60);
        current.provider.model = "model_v2".to_string();
        store.save_report(&current).unwrap();

        let summary = generate_summary(&store).unwrap();
        // The latest is the newest saved (current = 60), baseline is the older (80)
        assert!(!summary.latest_regressions.is_empty(), "Should detect regression");
    }

    #[test]
    fn eval_summary_serializes_stably() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());
        store.save_report(&make_report("test", 80)).unwrap();

        let summary = generate_summary(&store).unwrap();
        let json = serde_json::to_string_pretty(&summary).unwrap();
        let back: EvalSummaryReport = serde_json::from_str(&json).unwrap();
        assert_eq!(1, back.total_reports);
        assert_eq!(EVAL_REPORT_SCHEMA_VERSION, back.report_schema_version);
    }

    #[test]
    fn eval_summary_handles_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        let summary = generate_summary(&store).unwrap();
        assert_eq!(0, summary.total_reports);
        assert!(summary.scenario_summaries.is_empty());
        assert!(summary.provider_summaries.is_empty());
    }

    #[test]
    fn eval_summary_computes_trend() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        // Save 6 reports with improving scores (lowest first = oldest)
        for i in 0..6u32 {
            let mut r = make_report("trend_test", 50 + i * 5);
            r.provider.model = format!("v{}", i);
            store.save_report(&r).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let summary = generate_summary(&store).unwrap();
        let trend_scenario = summary.scenario_summaries.iter()
            .find(|s| s.scenario_id == "trend_test")
            .unwrap();
        // Saved oldest-first: 50,55,60,65,70,75
        // After newest-first sort: 75,70,65,60,55,50
        // Recent (first 3): 75,70,65 avg=70
        // Older (next 3): 60,55,50 avg=55
        // 70 > 55+1 → Improving
        assert_eq!(ScoreTrend::Improving, trend_scenario.trend);
    }
}
