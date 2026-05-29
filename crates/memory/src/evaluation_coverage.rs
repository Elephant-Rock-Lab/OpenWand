//! Memory evaluation coverage validator.
//!
//! Ensures the deterministic fixture suite covers all declared categories.

use std::collections::BTreeMap;

use crate::evaluation::{MemoryEvaluationCategory, MemoryEvaluationScenario};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::*;

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
}
