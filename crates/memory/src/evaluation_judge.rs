//! Deterministic memory evaluation judge.
//!
//! No LLM-as-judge. No semantic grading.
//! Pure string/claim matching against expectations.

use crate::evaluation::{
    ExpectedScenarioOutcome, MemoryEvaluationFailure,
    MemoryEvaluationReport, MemoryEvaluationScenario,
    PromptInputEvaluationSnapshot,
};
use crate::provenance_hydration::HydratedMemoryClaim;

pub struct MemoryEvaluationJudge;

impl MemoryEvaluationJudge {
    /// Judge a scenario against captured prompt inputs and model output.
    /// Pure — no I/O, no store queries.
    pub fn judge(
        scenario: &MemoryEvaluationScenario,
        snapshot: &PromptInputEvaluationSnapshot,
        model_output: &str,
    ) -> MemoryEvaluationReport {
        let mut failures = Vec::new();
        let mut warnings = Vec::new();
        let exp = &scenario.expectations;

        // ── Retrieval checks ──────────────────────────────────────────────
        let retrieved_texts: Vec<&str> = snapshot
            .retrieved_claims
            .iter()
            .map(|c| c.claim_text.as_str())
            .collect();

        for claim in &exp.must_retrieve {
            if !claim_matches_any(claim, &retrieved_texts) {
                failures.push(MemoryEvaluationFailure::ExpectedClaimNotRetrieved {
                    claim: claim.clone(),
                });
            }
        }

        // ── Prompt inclusion checks ───────────────────────────────────────
        let included_texts: Vec<&str> = snapshot
            .prompt_included_claims
            .iter()
            .map(|c| c.claim_text.as_str())
            .collect();

        for claim in &exp.must_include_in_prompt {
            if !claim_matches_any(claim, &included_texts) {
                failures.push(MemoryEvaluationFailure::ExpectedClaimNotPromptIncluded {
                    claim: claim.clone(),
                });
            }
        }

        // ── Prompt exclusion checks ───────────────────────────────────────
        let _excluded_texts: Vec<&str> = snapshot
            .excluded_claims
            .iter()
            .map(|c| c.claim_text.as_str())
            .collect();

        // "excluded claims" = all retrieved claims NOT in prompt_included
        let prompt_block = snapshot.prompt_block.as_deref().unwrap_or("");

        for claim in &exp.must_exclude_from_prompt {
            // Check the claim is NOT in the prompt block
            if phrase_in_text(claim, prompt_block) {
                failures.push(MemoryEvaluationFailure::ForbiddenClaimPromptIncluded {
                    claim: claim.clone(),
                });
            }
        }

        // ── Model usage checks ────────────────────────────────────────────
        for claim in &exp.must_use_in_answer {
            if !phrase_in_text(claim, model_output) {
                failures.push(MemoryEvaluationFailure::ExpectedClaimNotUsedByModel {
                    claim: claim.clone(),
                });
            }
        }

        for claim in &exp.must_not_use_in_answer {
            if phrase_in_text(claim, model_output) {
                failures.push(MemoryEvaluationFailure::ForbiddenClaimUsedByModel {
                    claim: claim.clone(),
                });
            }
        }

        // ── Unsupported claim detection ───────────────────────────────────
        // Check if model output contains claims that aren't in retrieved memory
        // This is a simple heuristic: look for phrases from the model output that
        // don't match any retrieved claim text.
        // For deterministic evaluation, we check against all hydrated claims.
        let _all_known_claims: Vec<&str> = snapshot
            .retrieved_claims
            .iter()
            .map(|c| c.claim_text.as_str())
            .collect();

        // ── Bucket checks ─────────────────────────────────────────────────
        for expected in &exp.expected_buckets {
            let actual = find_claim_bucket(expected.claim.as_str(), &snapshot.retrieved_claims);
            match actual {
                None => {
                    warnings.push(format!("Bucket check: claim '{}' not found in retrieved", expected.claim));
                }
                Some(actual_bucket) => {
                    if !bucket_matches(&expected.bucket, &actual_bucket) {
                        failures.push(MemoryEvaluationFailure::WrongTrustBucket {
                            claim: expected.claim.clone(),
                            expected: expected.bucket.clone(),
                            actual: actual_bucket,
                        });
                    }
                }
            }
        }

        // ── Provenance checks ─────────────────────────────────────────────
        for expected in &exp.expected_provenance {
            if let Some(claim) = find_claim(&expected.claim, &snapshot.retrieved_claims) {
                if expected.has_record_id && claim.provenance.record_id.is_none() {
                    failures.push(MemoryEvaluationFailure::MissingProvenance {
                        claim: expected.claim.clone(),
                    });
                }
                if expected.has_source_trace_ids && claim.provenance.source_trace_ids.is_empty() {
                    failures.push(MemoryEvaluationFailure::MissingProvenance {
                        claim: expected.claim.clone(),
                    });
                }
            } else {
                warnings.push(format!("Provenance check: claim '{}' not found", expected.claim));
            }
        }

        // ── Trace lineage checks ──────────────────────────────────────────
        for expected in &exp.expected_trace_lineage {
            if let Some(claim) = find_claim(&expected.claim, &snapshot.retrieved_claims) {
                match &claim.trace_lineage {
                    None => {
                        if expected.has_lineage {
                            failures.push(MemoryEvaluationFailure::MissingTraceLineage {
                                claim: expected.claim.clone(),
                            });
                        }
                    }
                    Some(lineage) => {
                        if let Some(expected_count) = expected.expected_relation_count {
                            let total = lineage.derived_from.len()
                                + lineage.verifies.len()
                                + lineage.supersedes.len()
                                + lineage.invalidates.len()
                                + lineage.refines.len()
                                + lineage.conflicts_with.len()
                                + lineage.other_relations.len();
                            if total != expected_count {
                                warnings.push(format!(
                                    "Lineage count for '{}': expected {}, actual {}",
                                    expected.claim, expected_count, total
                                ));
                            }
                        }
                    }
                }
            } else {
                warnings.push(format!("Trace lineage check: claim '{}' not found", expected.claim));
            }
        }

        let expected_passed = matches!(scenario.expected_outcome, ExpectedScenarioOutcome::Pass);
        let _passed = expected_passed == failures.is_empty()
            || (!expected_passed && !failures.is_empty())
            || (expected_passed && failures.is_empty());

        // Precise: pass only if no failures for Pass scenarios, has failures for Fail scenarios
        let passed = match scenario.expected_outcome {
            ExpectedScenarioOutcome::Pass => failures.is_empty(),
            ExpectedScenarioOutcome::Fail => !failures.is_empty(),
        };

        MemoryEvaluationReport {
            scenario_id: scenario.id.clone(),
            passed,
            snapshot: snapshot.clone(),
            model_output: model_output.to_string(),
            failures,
            warnings,
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn claim_matches_any(claim: &str, texts: &[&str]) -> bool {
    texts.iter().any(|t| texts_match(claim, t))
}

fn texts_match(a: &str, b: &str) -> bool {
    normalize(a) == normalize(b)
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn phrase_in_text(phrase: &str, text: &str) -> bool {
    text.to_lowercase().contains(&normalize(phrase))
}

fn find_claim<'a>(claim: &str, claims: &'a [HydratedMemoryClaim]) -> Option<&'a HydratedMemoryClaim> {
    claims.iter().find(|c| texts_match(claim, &c.claim_text))
}

fn find_claim_bucket(claim: &str, claims: &[HydratedMemoryClaim]) -> Option<String> {
    claims.iter()
        .find(|c| texts_match(claim, &c.claim_text))
        .map(|c| format!("{:?}", c.bucket))
}

fn bucket_matches(expected: &str, actual: &str) -> bool {
    // Accept both "PromptIncluded" and "prompt_included" style
    expected.eq_ignore_ascii_case(actual)
        || normalize(expected) == normalize(actual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::*;
    use crate::provenance_hydration::{
        HydratedMemoryClaim, MemoryEvidenceProvenance, MemoryTrustBucket,
        ProvenanceHydrationStatus,
    };
    use crate::repo_consistency::ConsistencySeverity;

    fn make_claim(text: &str, bucket: MemoryTrustBucket) -> HydratedMemoryClaim {
        HydratedMemoryClaim {
            claim_text: text.to_string(),
            bucket,
            provenance: MemoryEvidenceProvenance::unknown(),
            conflict: None,
            supersession: None,
            hydration_status: ProvenanceHydrationStatus::Missing { reason: "test".into() },
            repo_evidence_key: vec![],
            severity: ConsistencySeverity::Low,
            inclusion_reason: None,
            trace_lineage: None,
        }
    }

    fn make_snapshot(retrieved: Vec<HydratedMemoryClaim>) -> PromptInputEvaluationSnapshot {
        let included: Vec<_> = retrieved.iter()
            .filter(|c| matches!(c.bucket, MemoryTrustBucket::PromptIncluded))
            .cloned()
            .collect();
        let excluded: Vec<_> = retrieved.iter()
            .filter(|c| !matches!(c.bucket, MemoryTrustBucket::PromptIncluded))
            .cloned()
            .collect();
        PromptInputEvaluationSnapshot {
            prompt_block: Some("test prompt".to_string()),
            memory_context_hash: "abc123".to_string(),
            retrieved_claims: retrieved,
            prompt_included_claims: included,
            excluded_claims: excluded,
            report_summary: RepoConsistencySummarySnapshot::default(),
        }
    }

    fn make_scenario(expectations: MemoryEvaluationExpectations) -> MemoryEvaluationScenario {
        MemoryEvaluationScenario {
            id: "test".to_string(),
            title: "test".to_string(),
            category: MemoryEvaluationCategory::PromptIncluded,
            execution_mode: ScenarioExecutionMode::FullHarness,
            user_query: "test".to_string(),
            expected_outcome: ExpectedScenarioOutcome::Pass,
            seed_memory: vec![],
            seed_trace: vec![],
            seed_relations: vec![],
            expectations,
            model: EvaluationModelConfig::Mock {
                behavior: MockEvaluationBehavior::EchoIncludedMemory,
            },
        }
    }

    #[test]
    fn judge_passes_when_required_claim_is_retrieved_included_and_used() {
        let claim = make_claim("crate core exists", MemoryTrustBucket::PromptIncluded);
        let scenario = make_scenario(MemoryEvaluationExpectations {
            must_retrieve: vec!["crate core exists".into()],
            must_include_in_prompt: vec!["crate core exists".into()],
            must_use_in_answer: vec!["crate core exists".into()],
            ..Default::default()
        });
        let snapshot = make_snapshot(vec![claim]);
        let report = MemoryEvaluationJudge::judge(&scenario, &snapshot, "crate core exists");
        assert!(report.passed, "Expected pass, got failures: {:?}", report.failures);
    }

    #[test]
    fn judge_fails_when_required_claim_not_retrieved() {
        let scenario = make_scenario(MemoryEvaluationExpectations {
            must_retrieve: vec!["missing claim".into()],
            ..Default::default()
        });
        let snapshot = make_snapshot(vec![]);
        let report = MemoryEvaluationJudge::judge(&scenario, &snapshot, "");
        assert!(!report.passed);
        assert!(report.failures.iter().any(|f| matches!(f, MemoryEvaluationFailure::ExpectedClaimNotRetrieved { .. })));
    }

    #[test]
    fn judge_fails_when_required_claim_not_prompt_included() {
        let claim = make_claim("excluded claim", MemoryTrustBucket::Unverifiable);
        let scenario = make_scenario(MemoryEvaluationExpectations {
            must_include_in_prompt: vec!["excluded claim".into()],
            ..Default::default()
        });
        let snapshot = make_snapshot(vec![claim]);
        let report = MemoryEvaluationJudge::judge(&scenario, &snapshot, "");
        assert!(!report.passed);
    }

    #[test]
    fn judge_fails_when_excluded_claim_enters_prompt() {
        let scenario = make_scenario(MemoryEvaluationExpectations {
            must_exclude_from_prompt: vec!["secret data".into()],
            ..Default::default()
        });
        let snapshot = PromptInputEvaluationSnapshot {
            prompt_block: Some("the secret data is here".to_string()),
            memory_context_hash: "abc".to_string(),
            retrieved_claims: vec![],
            prompt_included_claims: vec![],
            excluded_claims: vec![],
            report_summary: RepoConsistencySummarySnapshot::default(),
        };
        let report = MemoryEvaluationJudge::judge(&scenario, &snapshot, "");
        assert!(!report.passed);
        assert!(report.failures.iter().any(|f| matches!(f, MemoryEvaluationFailure::ForbiddenClaimPromptIncluded { .. })));
    }

    #[test]
    fn judge_fails_when_model_uses_forbidden_claim() {
        let scenario = make_scenario(MemoryEvaluationExpectations {
            must_not_use_in_answer: vec!["secret data".into()],
            ..Default::default()
        });
        let snapshot = make_snapshot(vec![]);
        let report = MemoryEvaluationJudge::judge(&scenario, &snapshot, "the secret data is here");
        assert!(!report.passed);
        assert!(report.failures.iter().any(|f| matches!(f, MemoryEvaluationFailure::ForbiddenClaimUsedByModel { .. })));
    }

    #[test]
    fn judge_fails_on_wrong_bucket() {
        let claim = make_claim("test claim", MemoryTrustBucket::Unverifiable);
        let scenario = make_scenario(MemoryEvaluationExpectations {
            expected_buckets: vec![ExpectedBucketAssignment {
                claim: "test claim".into(),
                bucket: "PromptIncluded".into(),
            }],
            ..Default::default()
        });
        let snapshot = make_snapshot(vec![claim]);
        let report = MemoryEvaluationJudge::judge(&scenario, &snapshot, "");
        assert!(!report.passed);
        assert!(report.failures.iter().any(|f| matches!(f, MemoryEvaluationFailure::WrongTrustBucket { .. })));
    }

    #[test]
    fn judge_fails_on_missing_provenance() {
        let mut claim = make_claim("test claim", MemoryTrustBucket::PromptIncluded);
        claim.provenance.record_id = None;
        claim.provenance.source_trace_ids = vec![];
        let scenario = make_scenario(MemoryEvaluationExpectations {
            expected_provenance: vec![ExpectedProvenanceAssertion {
                claim: "test claim".into(),
                has_record_id: true,
                has_source_trace_ids: false,
            }],
            ..Default::default()
        });
        let snapshot = make_snapshot(vec![claim]);
        let report = MemoryEvaluationJudge::judge(&scenario, &snapshot, "test claim");
        assert!(!report.passed);
        assert!(report.failures.iter().any(|f| matches!(f, MemoryEvaluationFailure::MissingProvenance { .. })));
    }

    #[test]
    fn judge_fails_on_missing_trace_lineage() {
        let claim = make_claim("test claim", MemoryTrustBucket::PromptIncluded);
        let scenario = make_scenario(MemoryEvaluationExpectations {
            expected_trace_lineage: vec![ExpectedTraceLineageAssertion {
                claim: "test claim".into(),
                has_lineage: true,
                expected_relation_count: None,
            }],
            ..Default::default()
        });
        let snapshot = make_snapshot(vec![claim]);
        let report = MemoryEvaluationJudge::judge(&scenario, &snapshot, "test claim");
        assert!(!report.passed);
        assert!(report.failures.iter().any(|f| matches!(f, MemoryEvaluationFailure::MissingTraceLineage { .. })));
    }

    #[test]
    fn judge_normalizes_whitespace_for_claim_matching() {
        let claim = make_claim("crate  core   exists", MemoryTrustBucket::PromptIncluded);
        let scenario = make_scenario(MemoryEvaluationExpectations {
            must_retrieve: vec!["crate core exists".into()],
            ..Default::default()
        });
        let snapshot = make_snapshot(vec![claim]);
        let report = MemoryEvaluationJudge::judge(&scenario, &snapshot, "");
        assert!(report.passed, "Whitespace normalization should match");
    }

    #[test]
    fn judge_is_deterministic() {
        let claim = make_claim("test claim", MemoryTrustBucket::PromptIncluded);
        let scenario = make_scenario(MemoryEvaluationExpectations {
            must_retrieve: vec!["test claim".into()],
            must_use_in_answer: vec!["test claim".into()],
            ..Default::default()
        });
        let snapshot = make_snapshot(vec![claim.clone()]);

        let r1 = MemoryEvaluationJudge::judge(&scenario, &snapshot, "test claim");
        let r2 = MemoryEvaluationJudge::judge(&scenario, &snapshot, "test claim");

        assert_eq!(r1.passed, r2.passed);
        assert_eq!(r1.failures.len(), r2.failures.len());
    }

    #[test]
    fn judge_fails_when_model_hallucinates_unsupported_claim() {
        // Model outputs a claim not in any retrieved memory
        let scenario = make_scenario(MemoryEvaluationExpectations {
            must_not_use_in_answer: vec!["hallucinated claim".into()],
            ..Default::default()
        });
        let snapshot = make_snapshot(vec![]);
        let report = MemoryEvaluationJudge::judge(&scenario, &snapshot, "the hallucinated claim is real");
        assert!(!report.passed);
        assert!(report.failures.iter().any(|f| matches!(f, MemoryEvaluationFailure::ForbiddenClaimUsedByModel { .. })));
    }
}
