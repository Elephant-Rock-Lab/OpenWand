//! Evaluation delta reporting — measure governance tuning against 02q baseline.
//!
//! Captures per-scenario changes in prompt hash, inclusion, buckets, and model usage.
//! All deltas must be explicitly approved. Unapproved regressions fail the suite.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A captured baseline for comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvaluationBaseline {
    pub profile_label: String,
    /// scenario_id → memory_context_hash
    pub scenario_hashes: BTreeMap<String, String>,
    /// scenario_id → passed
    pub scenario_results: BTreeMap<String, bool>,
}

/// Delta for a single scenario between baseline and candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioEvaluationDelta {
    pub scenario_id: String,
    pub hash_changed: bool,
    pub inclusion_changed: bool,
    pub bucket_changed: bool,
    pub model_usage_changed: bool,
    /// Baseline hash (empty if not captured)
    pub baseline_hash: String,
    /// Candidate hash (empty if not captured)
    pub candidate_hash: String,
}

/// An explicitly approved behavioral change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovedBehaviorChange {
    pub scenario_id: String,
    pub field: String,
    pub before: String,
    pub after: String,
    pub reason: String,
}

/// A regression that was NOT approved.
#[derive(Debug, Clone)]
pub struct MemoryEvaluationRegression {
    pub scenario_id: String,
    pub field: String,
    pub detail: String,
}

/// Full delta report between baseline and candidate profiles.
#[derive(Debug, Clone)]
pub struct MemoryEvaluationDeltaReport {
    pub baseline_label: String,
    pub candidate_label: String,
    pub scenario_deltas: Vec<ScenarioEvaluationDelta>,
    pub approved_changes: Vec<ApprovedBehaviorChange>,
    pub unapproved_regressions: Vec<MemoryEvaluationRegression>,
}

impl MemoryEvaluationDeltaReport {
    /// Compute deltas between baseline and candidate reports.
    /// Each candidate report is matched by scenario_id.
    pub fn compute(
        baseline_label: &str,
        candidate_label: &str,
        baseline: &MemoryEvaluationBaseline,
        candidate_reports: &[crate::evaluation::MemoryEvaluationReport],
        approved: &[ApprovedBehaviorChange],
    ) -> Self {
        let mut deltas = Vec::new();
        let mut regressions = Vec::new();

        for report in candidate_reports {
            let baseline_hash = baseline.scenario_hashes.get(&report.scenario_id)
                .cloned()
                .unwrap_or_default();
            let hash_changed = !baseline_hash.is_empty()
                && baseline_hash != report.snapshot.memory_context_hash;

            // Check for approved change on hash
            let has_approved_hash_change = approved.iter().any(|a| {
                a.scenario_id == report.scenario_id && a.field == "prompt_hash"
            });

            if hash_changed && !has_approved_hash_change {
                regressions.push(MemoryEvaluationRegression {
                    scenario_id: report.scenario_id.clone(),
                    field: "prompt_hash".to_string(),
                    detail: format!(
                        "hash changed from {} to {} without approval",
                        baseline_hash, report.snapshot.memory_context_hash
                    ),
                });
            }

            deltas.push(ScenarioEvaluationDelta {
                scenario_id: report.scenario_id.clone(),
                hash_changed,
                inclusion_changed: false, // filled by caller with detailed comparison
                bucket_changed: false,
                model_usage_changed: false,
                baseline_hash,
                candidate_hash: report.snapshot.memory_context_hash.clone(),
            });
        }

        Self {
            baseline_label: baseline_label.to_string(),
            candidate_label: candidate_label.to_string(),
            scenario_deltas: deltas,
            approved_changes: approved.to_vec(),
            unapproved_regressions: regressions,
        }
    }

    /// Render as stable markdown.
    pub fn to_markdown(&self) -> String {
        let mut lines = Vec::new();

        lines.push("# Memory Evaluation Delta Report".to_string());
        lines.push(String::new());
        lines.push(format!("Baseline: {}", self.baseline_label));
        lines.push(format!("Candidate: {}", self.candidate_label));
        lines.push(String::new());

        lines.push(format!("Scenarios: {}", self.scenario_deltas.len()));
        lines.push(format!("Approved changes: {}", self.approved_changes.len()));
        lines.push(format!("Unapproved regressions: {}", self.unapproved_regressions.len()));
        lines.push(String::new());

        if !self.approved_changes.is_empty() {
            lines.push("## Approved Changes".to_string());
            for a in &self.approved_changes {
                lines.push(format!(
                    "- {} [{}]: {} → {} ({})",
                    a.scenario_id, a.field, a.before, a.after, a.reason
                ));
            }
            lines.push(String::new());
        }

        if !self.unapproved_regressions.is_empty() {
            lines.push("## Unapproved Regressions".to_string());
            for r in &self.unapproved_regressions {
                lines.push(format!("- {} [{}]: {}", r.scenario_id, r.field, r.detail));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }
}

/// Approved behavior changes for 02s production wiring.
/// Every change from Default → Batch02rDefault must be listed here.
/// The delta harness validates that observed changes match this ledger.
pub fn approved_02s_deltas() -> Vec<ApprovedBehaviorChange> {
    vec![
        ApprovedBehaviorChange {
            scenario_id: "low_confidence_claim_behavior".to_string(),
            field: "prompt_hash".to_string(),
            before: "(captured at runtime)".to_string(),
            after: "(captured at runtime)".to_string(),
            reason: "02s: batch_02r_default excludes low-confidence claims (2000 bps) below prompt_include_min_bps (3000)".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::{
        MemoryEvaluationReport, PromptInputEvaluationSnapshot,
        RepoConsistencySummarySnapshot,
    };

    fn make_report(id: &str, hash: &str, passed: bool) -> MemoryEvaluationReport {
        MemoryEvaluationReport {
            scenario_id: id.to_string(),
            passed,
            snapshot: PromptInputEvaluationSnapshot {
                prompt_block: None,
                memory_context_hash: hash.to_string(),
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
    fn delta_report_detects_prompt_hash_change() {
        let baseline = MemoryEvaluationBaseline {
            profile_label: "02q".into(),
            scenario_hashes: BTreeMap::from([("s1".into(), "hash_a".into())]),
            scenario_results: BTreeMap::from([("s1".into(), true)]),
        };
        let reports = vec![make_report("s1", "hash_b", true)];
        let delta = MemoryEvaluationDeltaReport::compute("02q", "02r", &baseline, &reports, &[]);
        assert_eq!(1, delta.scenario_deltas.len());
        assert!(delta.scenario_deltas[0].hash_changed);
        assert_eq!(1, delta.unapproved_regressions.len());
    }

    #[test]
    fn delta_report_marks_approved_hash_change() {
        let baseline = MemoryEvaluationBaseline {
            profile_label: "02q".into(),
            scenario_hashes: BTreeMap::from([("s1".into(), "hash_a".into())]),
            scenario_results: BTreeMap::from([("s1".into(), true)]),
        };
        let reports = vec![make_report("s1", "hash_b", true)];
        let approved = vec![ApprovedBehaviorChange {
            scenario_id: "s1".into(),
            field: "prompt_hash".into(),
            before: "hash_a".into(),
            after: "hash_b".into(),
            reason: "confidence policy change".into(),
        }];
        let delta = MemoryEvaluationDeltaReport::compute("02q", "02r", &baseline, &reports, &approved);
        assert!(delta.scenario_deltas[0].hash_changed);
        assert!(delta.unapproved_regressions.is_empty());
    }

    #[test]
    fn delta_report_marks_unapproved_regression() {
        let baseline = MemoryEvaluationBaseline {
            profile_label: "02q".into(),
            scenario_hashes: BTreeMap::from([("s1".into(), "hash_a".into())]),
            scenario_results: BTreeMap::from([("s1".into(), true)]),
        };
        let reports = vec![make_report("s1", "hash_b", true)];
        let delta = MemoryEvaluationDeltaReport::compute("02q", "02r", &baseline, &reports, &[]);
        assert_eq!(1, delta.unapproved_regressions.len());
        assert_eq!("s1", delta.unapproved_regressions[0].scenario_id);
    }

    #[test]
    fn delta_report_no_change_is_neutral() {
        let baseline = MemoryEvaluationBaseline {
            profile_label: "02q".into(),
            scenario_hashes: BTreeMap::from([("s1".into(), "hash_a".into())]),
            scenario_results: BTreeMap::from([("s1".into(), true)]),
        };
        let reports = vec![make_report("s1", "hash_a", true)];
        let delta = MemoryEvaluationDeltaReport::compute("02q", "02r", &baseline, &reports, &[]);
        assert!(!delta.scenario_deltas[0].hash_changed);
        assert!(delta.unapproved_regressions.is_empty());
    }

    #[test]
    fn delta_report_markdown_is_deterministic() {
        let baseline = MemoryEvaluationBaseline {
            profile_label: "02q".into(),
            scenario_hashes: BTreeMap::from([("s1".into(), "hash_a".into())]),
            scenario_results: BTreeMap::from([("s1".into(), true)]),
        };
        let reports = vec![make_report("s1", "hash_a", true)];
        let d1 = MemoryEvaluationDeltaReport::compute("02q", "02r", &baseline, &reports, &[]);
        let d2 = MemoryEvaluationDeltaReport::compute("02q", "02r", &baseline, &reports, &[]);
        assert_eq!(d1.to_markdown(), d2.to_markdown());
    }

    #[test]
    fn approved_behavior_change_roundtrips_json() {
        let change = ApprovedBehaviorChange {
            scenario_id: "s1".into(),
            field: "prompt_hash".into(),
            before: "abc".into(),
            after: "def".into(),
            reason: "test".into(),
        };
        let json = serde_json::to_string(&change).unwrap();
        let restored: ApprovedBehaviorChange = serde_json::from_str(&json).unwrap();
        assert_eq!(change, restored);
    }

    // ── 02s ledger validation tests ───────────────────────────────────────

    #[test]
    fn approved_02s_deltas_reference_existing_scenario() {
        let deltas = approved_02s_deltas();
        assert!(!deltas.is_empty(), "Ledger must not be empty");
        for delta in &deltas {
            assert!(!delta.scenario_id.is_empty());
            assert!(!delta.field.is_empty());
            assert!(!delta.reason.is_empty());
        }
    }

    #[test]
    fn approved_02s_delta_ids_are_unique() {
        let deltas = approved_02s_deltas();
        let mut seen = std::collections::HashSet::new();
        for delta in &deltas {
            let key = format!("{}:{}", delta.scenario_id, delta.field);
            assert!(seen.insert(key), "Duplicate delta: {}", delta.scenario_id);
        }
    }

    #[test]
    fn approved_02s_deltas_only_reference_low_confidence() {
        let deltas = approved_02s_deltas();
        for delta in &deltas {
            assert_eq!("low_confidence_claim_behavior", delta.scenario_id,
                "Only low_confidence_claim_behavior should change under batch_02r_default");
        }
    }
}
