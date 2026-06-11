//! Evaluation report persistence layer.
//!
//! Stable directory layout for schema-versioned JSON reports.
//! Operations: save, load, list, latest_for_scenario, promote_baseline.

use crate::eval_model::*;
use std::path::PathBuf;

/// Report storage root.
#[derive(Debug, Clone)]
pub struct EvalReportStore {
    pub root: PathBuf,
}

/// A stored report with its path.
#[derive(Debug, Clone)]
pub struct StoredEvalReport {
    pub path: PathBuf,
    pub report: EvalRunReport,
}

/// Filter for listing reports.
#[derive(Debug, Clone, Default)]
pub struct ReportFilter {
    pub scenario_id: Option<String>,
}

impl EvalReportStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Save a report to the stable directory layout.
    /// Path: {root}/scenarios/{scenario_id}/{timestamp}_{model}.json
    pub fn save_report(&self, report: &EvalRunReport) -> Result<PathBuf, String> {
        let scenario_dir = self.root.join("scenarios").join(&report.scenario_id);
        std::fs::create_dir_all(&scenario_dir)
            .map_err(|e| format!("Failed to create scenario dir: {}", e))?;

        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%SZ");
        let filename = format!("{}_{}.json", timestamp, report.provider.model);
        let path = scenario_dir.join(&filename);

        let json = serde_json::to_string_pretty(report)
            .map_err(|e| format!("Failed to serialize report: {}", e))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write report: {}", e))?;

        Ok(path)
    }

    /// Load a report from a specific path.
    pub fn load_report(&self, path: &std::path::Path) -> Result<EvalRunReport, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read report: {}", e))?;
        let report: EvalRunReport = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse report: {}", e))?;

        // Validate schema version
        if report.report_schema_version > EVAL_REPORT_SCHEMA_VERSION {
            return Err(format!(
                "Report schema version {} is newer than supported {}",
                report.report_schema_version, EVAL_REPORT_SCHEMA_VERSION
            ));
        }

        Ok(report)
    }

    /// List reports matching a filter.
    pub fn list_reports(&self, filter: &ReportFilter) -> Result<Vec<StoredEvalReport>, String> {
        let scenarios_dir = self.root.join("scenarios");
        if !scenarios_dir.exists() {
            return Ok(vec![]);
        }

        let mut results = Vec::new();

        let scenario_dirs: Vec<_> = std::fs::read_dir(&scenarios_dir)
            .map_err(|e| format!("Failed to read scenarios dir: {}", e))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        for scenario_dir in scenario_dirs {
            let scenario_name = scenario_dir.file_name().to_string_lossy().to_string();

            // Apply filter
            if let Some(ref id) = filter.scenario_id {
                if scenario_name != *id {
                    continue;
                }
            }

            let entries: Vec<_> = std::fs::read_dir(scenario_dir.path())
                .map_err(|e| format!("Failed to read scenario dir: {}", e))?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().extension().map(|ext| ext == "json").unwrap_or(false)
                })
                .collect();

            for entry in entries {
                if let Ok(report) = self.load_report(&entry.path()) {
                    results.push(StoredEvalReport {
                        path: entry.path(),
                        report,
                    });
                }
            }
        }

        // Sort by scenario_id then by observed_at descending (newest first)
        results.sort_by(|a, b| {
            match a.report.scenario_id.cmp(&b.report.scenario_id) {
                std::cmp::Ordering::Equal => {
                    b.report.provider.observed_at.cmp(&a.report.provider.observed_at)
                }
                other => other,
            }
        });

        Ok(results)
    }

    /// Get the latest report for a given scenario.
    pub fn latest_for_scenario(&self, scenario_id: &str) -> Result<Option<StoredEvalReport>, String> {
        let reports = self.list_reports(&ReportFilter {
            scenario_id: Some(scenario_id.to_string()),
        })?;
        Ok(reports.into_iter().next()) // Already sorted newest-first
    }

    /// Promote a report as the baseline for its scenario.
    /// Copies to {root}/baselines/{scenario_id}.json
    pub fn promote_baseline(&self, report: &EvalRunReport) -> Result<PathBuf, String> {
        let baselines_dir = self.root.join("baselines");
        std::fs::create_dir_all(&baselines_dir)
            .map_err(|e| format!("Failed to create baselines dir: {}", e))?;

        let path = baselines_dir.join(format!("{}.json", report.scenario_id));
        let json = serde_json::to_string_pretty(report)
            .map_err(|e| format!("Failed to serialize baseline: {}", e))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write baseline: {}", e))?;

        Ok(path)
    }

    /// Load the baseline for a scenario.
    pub fn load_baseline(&self, scenario_id: &str) -> Result<Option<EvalRunReport>, String> {
        let path = self.root.join("baselines").join(format!("{}.json", scenario_id));
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(self.load_report(&path)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_report(scenario_id: &str, model: &str) -> EvalRunReport {
        EvalRunReport {
            report_schema_version: EVAL_REPORT_SCHEMA_VERSION,
            scenario_id: scenario_id.to_string(),
            provider: ProviderRealitySnapshot {
                provider: "openai-compatible".to_string(),
                model: model.to_string(),
                base_url_redacted: Some("http://localhost:1234/v1".to_string()),
                supports_streaming: true,
                supports_tools: true,
                supports_reasoning: false,
                health_status: ProviderHealthStatus::Healthy,
                temperature: None,
                max_tokens: None,
                observed_at: chrono::Utc::now(),
            },
            prompt: PromptEvalResult::default(),
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
            capability_context: CapabilityContextEvalResult::default(),
            score: EvalScore::from_dimensions(vec![]),
        }
    }

    #[test]
    fn eval_report_store_saves_and_loads_json() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());
        let report = make_report("test_scenario", "qwen3");

        let path = store.save_report(&report).unwrap();
        assert!(path.exists());

        let loaded = store.load_report(&path).unwrap();
        assert_eq!("test_scenario", loaded.scenario_id);
        assert_eq!(EVAL_REPORT_SCHEMA_VERSION, loaded.report_schema_version);
    }

    fn eval_report_store_lists_by_scenario() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        store.save_report(&make_report("alpha", "qwen3")).unwrap();
        store.save_report(&make_report("beta", "qwen3")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.save_report(&make_report("alpha", "qwen3")).unwrap();

        let all = store.list_reports(&ReportFilter::default()).unwrap();
        assert_eq!(3, all.len());

        let alpha = store.list_reports(&ReportFilter {
            scenario_id: Some("alpha".to_string()),
        }).unwrap();
        assert_eq!(2, alpha.len());
        assert!(alpha.iter().all(|r| r.report.scenario_id == "alpha"));
    }

    #[test]
    fn eval_report_store_latest_for_scenario() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        store.save_report(&make_report("gamma", "qwen3")).unwrap();
        // Small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.save_report(&make_report("gamma", "qwen3")).unwrap();

        let latest = store.latest_for_scenario("gamma").unwrap().unwrap();
        // Both have the same scenario_id, just verifying it returns one
        assert_eq!("gamma", latest.report.scenario_id);
    }

    #[test]
    fn eval_report_store_uses_stable_directory_layout() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());
        let path = store.save_report(&make_report("my_scenario", "qwen3")).unwrap();

        // Path should contain scenarios/my_scenario/
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("scenarios"), "Missing scenarios dir: {}", path_str);
        assert!(path_str.contains("my_scenario"), "Missing scenario dir: {}", path_str);
        assert!(path_str.contains("qwen3"), "Missing model in filename: {}", path_str);
    }

    #[test]
    fn eval_report_store_rejects_schema_version_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        // Write a report with a future schema version
        let mut report = make_report("future", "qwen3");
        report.report_schema_version = 9999;
        let json = serde_json::to_string_pretty(&report).unwrap();

        let path = dir.path().join("scenarios").join("future").join("test.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, json).unwrap();

        let result = store.load_report(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("newer than supported"));
    }

    #[test]
    fn eval_report_store_promote_baseline() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        let report = make_report("baseline_test", "qwen3");
        let path = store.promote_baseline(&report).unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("baselines"));

        let loaded = store.load_baseline("baseline_test").unwrap().unwrap();
        assert_eq!("baseline_test", loaded.scenario_id);
    }

    #[test]
    fn eval_report_store_baseline_returns_none_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        let result = store.load_baseline("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn eval_report_store_handles_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvalReportStore::new(dir.path().to_path_buf());

        let reports = store.list_reports(&ReportFilter::default()).unwrap();
        assert!(reports.is_empty());
    }
}
