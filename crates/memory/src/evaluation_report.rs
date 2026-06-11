//! Memory evaluation report rendering.

use crate::evaluation::MemoryEvaluationReport;

impl MemoryEvaluationReport {
    /// Render the report as stable markdown.
    pub fn to_markdown(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("# Memory Evaluation Report: {}", self.scenario_id));
        lines.push(String::new());
        lines.push(format!("**Status: {}**", if self.passed { "PASS" } else { "FAIL" }));
        lines.push(String::new());

        // Prompt inputs
        lines.push("## Prompt Inputs".to_string());
        lines.push(format!("- Retrieved: {}", self.snapshot.retrieved_claims.len()));
        lines.push(format!("- Included: {}", self.snapshot.prompt_included_claims.len()));
        lines.push(format!("- Excluded: {}", self.snapshot.excluded_claims.len()));
        lines.push(format!("- Context hash: `{}`", self.snapshot.memory_context_hash));
        lines.push(String::new());

        // Report summary
        let s = &self.snapshot.report_summary;
        lines.push("## Report Summary".to_string());
        lines.push(format!(
            "- Total: {} | Supported: {} | Stale: {} | Unverifiable: {} | Superseded: {} | Conflict: {}",
            s.total_findings, s.supported, s.stale, s.unverifiable, s.superseded, s.conflict,
        ));
        lines.push(String::new());

        // Failures
        if !self.failures.is_empty() {
            lines.push("## Failures".to_string());
            for (i, f) in self.failures.iter().enumerate() {
                lines.push(format!("{}. {}", i + 1, f.message()));
            }
            lines.push(String::new());
        }

        // Warnings
        if !self.warnings.is_empty() {
            lines.push("## Warnings".to_string());
            for w in &self.warnings {
                lines.push(format!("- {}", w));
            }
            lines.push(String::new());
        }

        // Hydrated claims
        if !self.snapshot.retrieved_claims.is_empty() {
            lines.push("## Hydrated Claims".to_string());
            for (i, c) in self.snapshot.retrieved_claims.iter().enumerate() {
                lines.push(format!(
                    "{}. **{}** → `{:?}`",
                    i + 1,
                    c.claim_text,
                    c.bucket,
                ));
                let prov_line = c.provenance.evidence_line();
                if !prov_line.is_empty() {
                    lines.push(format!("   - Provenance: {}", prov_line));
                }
                if let Some(ref lineage) = c.trace_lineage {
                    lines.push(format!("   - Trace lineage: {}", lineage.compact_summary()));
                }
            }
        }

        lines.join("\n")
    }
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

    fn make_report(passed: bool, failures: Vec<MemoryEvaluationFailure>) -> MemoryEvaluationReport {
        MemoryEvaluationReport {
            scenario_id: "test_report".into(),
            passed,
            snapshot: PromptInputEvaluationSnapshot {
                prompt_block: Some("test prompt".into()),
                memory_context_hash: "abc123def456".into(),
                retrieved_claims: vec![HydratedMemoryClaim {
                    claim_text: "crate core exists".into(),
                    bucket: MemoryTrustBucket::PromptIncluded,
                    provenance: MemoryEvidenceProvenance {
                        provenance_kind: crate::provenance::ProvenanceKind::UserStated,
                        record_id: Some("rec_1".into()),
                        source_trace_ids: vec!["trace_001".into()],
                        source_episode_ids: vec![],
                        confidence: Some(0.95),
                        created_at: None,
                        evidence_kind: None,
                        retrieval_reason: None,
                        rank_score_summary: None,
                    },
                    conflict: None,
                    supersession: None,
                    hydration_status: ProvenanceHydrationStatus::Complete,
                    repo_evidence_key: vec![],
                    severity: ConsistencySeverity::Low,
                    inclusion_reason: None,
                    trace_lineage: None,
                }],
                prompt_included_claims: vec![],
                excluded_claims: vec![],
                report_summary: RepoConsistencySummarySnapshot {
                    total_findings: 1,
                    supported: 1,
                    ..Default::default()
                },
            },
            model_output: "test output".into(),
            failures,
            warnings: vec![],
        }
    }

    #[test]
    fn report_markdown_includes_status() {
        let md = make_report(true, vec![]).to_markdown();
        assert!(md.contains("PASS"));
        let md_fail = make_report(false, vec![MemoryEvaluationFailure::MissingProvenance {
            claim: "test".into(),
        }]).to_markdown();
        assert!(md_fail.contains("FAIL"));
    }

    #[test]
    fn report_markdown_includes_claim_counts() {
        let md = make_report(true, vec![]).to_markdown();
        assert!(md.contains("Retrieved: 1"));
    }

    #[test]
    fn report_markdown_includes_failures() {
        let report = make_report(false, vec![
            MemoryEvaluationFailure::ExpectedClaimNotRetrieved { claim: "test claim".into() },
        ]);
        let md = report.to_markdown();
        assert!(md.contains("## Failures"));
        assert!(md.contains("test claim"));
    }

    #[test]
    fn report_markdown_includes_hydrated_claim_provenance() {
        let md = make_report(true, vec![]).to_markdown();
        assert!(md.contains("Provenance:"));
        assert!(md.contains("rec_1"));
    }

    #[test]
    fn report_markdown_includes_trace_lineage_summary() {
        let mut report = make_report(true, vec![]);
        report.snapshot.retrieved_claims[0].trace_lineage = Some(
            crate::trace_relation_hydration::ClaimTraceLineage {
                source_trace_ids: vec!["trace_001".into()],
                derived_from: vec![],
                verifies: vec![],
                supersedes: vec![],
                invalidates: vec![],
                refines: vec![],
                conflicts_with: vec![],
                other_relations: vec![],
                hydration_status: ProvenanceHydrationStatus::Partial {
                    missing: vec!["no relations".into()],
                },
            },
        );
        let md = report.to_markdown();
        assert!(md.contains("Trace lineage:"));
    }

    #[test]
    fn report_markdown_is_deterministic() {
        let report = make_report(true, vec![]);
        let md1 = report.to_markdown();
        let md2 = report.to_markdown();
        assert_eq!(md1, md2);
    }
}
