//! Memory evaluation coverage validator and suite report.
//!
//! Ensures the deterministic fixture suite covers all declared categories
//! and renders stable markdown suite indices.

use std::collections::BTreeMap;

use crate::evaluation::{MemoryEvaluationCategory, MemoryEvaluationReport, MemoryEvaluationScenario};

/// Coverage report for the evaluation fixture suite.
#[derive(Debug, Clone)]
pub struct MemoryEvaluationCoverageReport {
    pub total_fixtures: usize,
    pub categories: BTreeMap<MemoryEvaluationCategory, usize>,
    pub missing_categories: Vec<MemoryEvaluationCategory>,
}

/// Validates that the fixture suite covers all evaluation categories.
pub struct MemoryEvaluationCoverageValidator;

impl MemoryEvaluationCoverageValidator {
    /// Validate scenario coverage across all categories.
    pub fn validate(scenarios: &[MemoryEvaluationScenario]) -> MemoryEvaluationCoverageReport {
        let mut categories: BTreeMap<MemoryEvaluationCategory, usize> = BTreeMap::new();
        for scenario in scenarios {
            *categories.entry(scenario.category).or_insert(0) += 1;
        }
        let missing_categories: Vec<MemoryEvaluationCategory> =
            MemoryEvaluationCategory::all()
                .iter()
                .filter(|cat| !categories.contains_key(cat))
                .copied()
                .collect();
        MemoryEvaluationCoverageReport {
            total_fixtures: scenarios.len(),
            categories,
            missing_categories,
        }
    }
}

// ── Suite report ───────────────────────────────────────────────────────────

/// Suite-level report aggregating multiple scenario results.
#[derive(Debug, Clone)]
pub struct MemoryEvaluationSuiteReport {
    pub total: usize,
    pub passed_as_expected: usize,
    pub failed_unexpectedly: usize,
    pub categories: BTreeMap<MemoryEvaluationCategory, usize>,
    pub scenario_reports: Vec<MemoryEvaluationReport>,
}

impl MemoryEvaluationSuiteReport {
    /// Build a suite report from individual scenario reports.
    pub fn from_reports(
        reports: Vec<MemoryEvaluationReport>,
        scenarios: &[MemoryEvaluationScenario],
    ) -> Self {
        let total = reports.len();
        let passed_as_expected = reports.iter().filter(|r| r.passed).count();
        let failed_unexpectedly = total - passed_as_expected;

        let mut categories: BTreeMap<MemoryEvaluationCategory, usize> = BTreeMap::new();
        for scenario in scenarios {
            *categories.entry(scenario.category).or_insert(0) += 1;
        }

        Self {
            total,
            passed_as_expected,
            failed_unexpectedly,
            categories,
            scenario_reports: reports,
        }
    }

    /// Render the suite report as a stable markdown index.
    pub fn to_markdown_index(&self) -> String {
        let mut lines = Vec::new();

        lines.push("# Memory Evaluation Suite".to_string());
        lines.push(String::new());
        lines.push(format!("Total scenarios: {}", self.total));
        lines.push(format!("Passed as expected: {}", self.passed_as_expected));
        lines.push(format!("Failed unexpectedly: {}", self.failed_unexpectedly));
        lines.push(String::new());

        lines.push("## Coverage by Category".to_string());
        lines.push("| Category | Count |".to_string());
        lines.push("|---|---:|".to_string());
        for (cat, count) in &self.categories {
            lines.push(format!("| {:?} | {} |", cat, count));
        }
        lines.push(String::new());

        if !self.scenario_reports.is_empty() {
            lines.push("## Scenario Results".to_string());
            for report in &self.scenario_reports {
                let status = if report.passed { "PASS" } else { "FAIL" };
                lines.push(format!("- {} [{}]", report.scenario_id, status));
            }
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::*;
    use crate::provenance_hydration::MemoryTrustBucket;
    use crate::repo_consistency::ConsistencySeverity;

    fn make_scenario(category: MemoryEvaluationCategory) -> MemoryEvaluationScenario {
        MemoryEvaluationScenario {
            id: format!("test_{:?}", category),
            title: "test".into(),
            category,
            execution_mode: ScenarioExecutionMode::FullHarness,
            user_query: "test".into(),
            expected_outcome: ExpectedScenarioOutcome::Pass,
            seed_memory: vec![],
            seed_trace: vec![],
            seed_relations: vec![],
            expectations: MemoryEvaluationExpectations::default(),
            model: EvaluationModelConfig::Mock {
                behavior: MockEvaluationBehavior::EchoIncludedMemory,
            },
        }
    }

    fn make_report(id: &str, passed: bool) -> MemoryEvaluationReport {
        MemoryEvaluationReport {
            scenario_id: id.to_string(),
            passed,
            snapshot: PromptInputEvaluationSnapshot {
                prompt_block: None,
                memory_context_hash: String::new(),
                retrieved_claims: vec![],
                prompt_included_claims: vec![],
                excluded_claims: vec![],
                report_summary: RepoConsistencySummarySnapshot::default(),
            },
            model_output: String::new(),
            failures: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn coverage_report_counts_categories() {
        let scenarios = vec![
            make_scenario(MemoryEvaluationCategory::PromptIncluded),
            make_scenario(MemoryEvaluationCategory::PromptIncluded),
            make_scenario(MemoryEvaluationCategory::Superseded),
        ];
        let report = MemoryEvaluationCoverageValidator::validate(&scenarios);
        assert_eq!(3, report.total_fixtures);
        assert_eq!(2, *report.categories.get(&MemoryEvaluationCategory::PromptIncluded).unwrap());
        assert_eq!(1, *report.categories.get(&MemoryEvaluationCategory::Superseded).unwrap());
    }

    #[test]
    fn coverage_report_detects_missing_categories() {
        let scenarios = vec![make_scenario(MemoryEvaluationCategory::PromptIncluded)];
        let report = MemoryEvaluationCoverageValidator::validate(&scenarios);
        assert_eq!(9, report.missing_categories.len());
        assert!(report.missing_categories.contains(&MemoryEvaluationCategory::Stale));
    }

    #[test]
    fn coverage_report_is_deterministic() {
        let scenarios = vec![
            make_scenario(MemoryEvaluationCategory::PromptIncluded),
            make_scenario(MemoryEvaluationCategory::Conflict),
        ];
        let r1 = MemoryEvaluationCoverageValidator::validate(&scenarios);
        let r2 = MemoryEvaluationCoverageValidator::validate(&scenarios);
        assert_eq!(r1.total_fixtures, r2.total_fixtures);
        assert_eq!(r1.missing_categories, r2.missing_categories);
    }

    #[test]
    fn coverage_validator_accepts_full_taxonomy() {
        let scenarios: Vec<_> = MemoryEvaluationCategory::all()
            .iter()
            .map(|cat| make_scenario(*cat))
            .collect();
        let report = MemoryEvaluationCoverageValidator::validate(&scenarios);
        assert_eq!(10, report.total_fixtures);
        assert!(report.missing_categories.is_empty(), "All categories must be covered");
    }

    #[test]
    fn suite_report_counts_scenarios() {
        let scenarios = vec![make_scenario(MemoryEvaluationCategory::PromptIncluded)];
        let reports = vec![make_report("t1", true)];
        let suite = MemoryEvaluationSuiteReport::from_reports(reports, &scenarios);
        assert_eq!(1, suite.total);
        assert_eq!(1, suite.passed_as_expected);
        assert_eq!(0, suite.failed_unexpectedly);
    }

    #[test]
    fn suite_report_marks_unexpected_failures() {
        let scenarios = vec![
            make_scenario(MemoryEvaluationCategory::PromptIncluded),
            make_scenario(MemoryEvaluationCategory::Stale),
        ];
        let reports = vec![make_report("t1", true), make_report("t2", false)];
        let suite = MemoryEvaluationSuiteReport::from_reports(reports, &scenarios);
        assert_eq!(2, suite.total);
        assert_eq!(1, suite.passed_as_expected);
        assert_eq!(1, suite.failed_unexpectedly);
    }

    #[test]
    fn suite_report_counts_categories_from_scenarios() {
        let scenarios = vec![
            make_scenario(MemoryEvaluationCategory::PromptIncluded),
            make_scenario(MemoryEvaluationCategory::Conflict),
            make_scenario(MemoryEvaluationCategory::Conflict),
        ];
        let reports = vec![make_report("t1", true), make_report("t2", true), make_report("t3", true)];
        let suite = MemoryEvaluationSuiteReport::from_reports(reports, &scenarios);
        assert_eq!(2, *suite.categories.get(&MemoryEvaluationCategory::Conflict).unwrap());
        assert_eq!(1, *suite.categories.get(&MemoryEvaluationCategory::PromptIncluded).unwrap());
    }

    #[test]
    fn suite_report_markdown_is_deterministic() {
        let scenarios = vec![make_scenario(MemoryEvaluationCategory::PromptIncluded)];
        let reports = vec![make_report("t1", true)];
        let suite = MemoryEvaluationSuiteReport::from_reports(reports, &scenarios);
        let md1 = suite.to_markdown_index();
        let md2 = suite.to_markdown_index();
        assert_eq!(md1, md2);
        assert!(md1.contains("Total scenarios: 1"));
        assert!(md1.contains("Coverage by Category"));
    }
}
