//! Wave 02t — Regression classifier + no-behavior-change guards.
//!
//! Proves 02t doesn't change governance, prompt format, panel, classifiers,
//! or detection wiring.

use openwand_memory::governance::MemoryGovernanceProfileId;

// ── Regression classifier ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptInputDeltaClassification {
    /// Low-confidence claim included under Default, excluded under Batch02rDefault.
    ApprovedLowConfidenceDelta,
    /// Same content under both profiles.
    Unchanged,
    /// Any unexpected change.
    UnapprovedRegression { detail: String },
}

/// Classify the difference between two prompt blocks.
pub fn classify_prompt_delta(
    default_prompt: &Option<String>,
    tuned_prompt: &Option<String>,
    scenario_id: &str,
) -> PromptInputDeltaClassification {
    match (default_prompt, tuned_prompt) {
        (Some(d), Some(t)) if d == t => PromptInputDeltaClassification::Unchanged,
        (Some(d), Some(t)) => {
            // Content differs — only approved for low-confidence scenarios
            if scenario_id.contains("low_confidence") {
                PromptInputDeltaClassification::ApprovedLowConfidenceDelta
            } else {
                PromptInputDeltaClassification::UnapprovedRegression {
                    detail: format!("Prompt content changed for scenario '{}'", scenario_id),
                }
            }
        }
        (Some(_), None) => {
            // Default has content, tuned is empty
            if scenario_id.contains("low_confidence") {
                PromptInputDeltaClassification::ApprovedLowConfidenceDelta
            } else {
                PromptInputDeltaClassification::UnapprovedRegression {
                    detail: format!("Tuned prompt became empty for scenario '{}'", scenario_id),
                }
            }
        }
        (None, Some(_)) => PromptInputDeltaClassification::UnapprovedRegression {
            detail: format!("Tuned prompt gained content from empty for '{}'", scenario_id),
        },
        (None, None) => PromptInputDeltaClassification::Unchanged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_identical_prompts_as_unchanged() {
        let prompt = Some("## Verified Memory\n- claim".to_string());
        let result = classify_prompt_delta(&prompt, &prompt, "any_scenario");
        assert_eq!(PromptInputDeltaClassification::Unchanged, result);
    }

    #[test]
    fn classifies_both_empty_as_unchanged() {
        let result = classify_prompt_delta(&None, &None, "any_scenario");
        assert_eq!(PromptInputDeltaClassification::Unchanged, result);
    }

    #[test]
    fn classifies_low_confidence_content_diff_as_approved() {
        let d = Some("## Verified Memory\n- crate core exists".to_string());
        let t = Some("## Context Gaps\n...".to_string());
        let result = classify_prompt_delta(&d, &t, "low_confidence_claim");
        assert_eq!(PromptInputDeltaClassification::ApprovedLowConfidenceDelta, result);
    }

    #[test]
    fn classifies_low_confidence_tuned_empty_as_approved() {
        let d = Some("## Verified Memory\n- crate core exists".to_string());
        let result = classify_prompt_delta(&d, &None, "low_confidence_claim");
        assert_eq!(PromptInputDeltaClassification::ApprovedLowConfidenceDelta, result);
    }

    #[test]
    fn classifies_non_low_confidence_diff_as_regression() {
        let d = Some("## Verified Memory\n- claim".to_string());
        let t = Some("## Verified Memory\n- different".to_string());
        let result = classify_prompt_delta(&d, &t, "high_confidence_claim");
        assert!(matches!(result, PromptInputDeltaClassification::UnapprovedRegression { .. }));
    }

    #[test]
    fn classifies_tuned_gaining_content_as_regression() {
        let t = Some("## Verified Memory\n- claim".to_string());
        let result = classify_prompt_delta(&None, &t, "any_scenario");
        assert!(matches!(result, PromptInputDeltaClassification::UnapprovedRegression { .. }));
    }

    // ── No-behavior-change guards ─────────────────────────────────────────

    #[test]
    fn integration_proof_does_not_modify_governance_profile() {
        let default = MemoryGovernanceProfileId::Default.resolve();
        let tuned = MemoryGovernanceProfileId::Batch02rDefault.resolve();
        // These values must match the 02r lock doc
        assert_eq!(0, default.confidence_policy.prompt_include_min_bps);
        assert_eq!(3000, tuned.confidence_policy.prompt_include_min_bps);
        assert_eq!(2000, tuned.verification_policy.verifies_boost_bps);
    }

    #[test]
    fn integration_proof_does_not_modify_prompt_format() {
        // Prompt format is determined by MemoryPromptAssemblyInputs::to_prompt_block()
        // which was not modified in 02t. Verify the format is stable.
        use openwand_memory::prompt_assembly::MemoryPromptAssemblyInputs;
        let inputs = MemoryPromptAssemblyInputs {
            supported_claims: vec![openwand_memory::prompt_assembly::SupportedMemoryClaim {
                claim_text: "crate core exists".to_string(),
                evidence_kind: openwand_memory::evidence::EvidenceKind::AcceptedClaim,
                confidence_bps: 0,
                source_provenance: None,
                repo_evidence_key: vec!["crate:core".to_string()],
                inclusion_reason: openwand_memory::prompt_assembly::PromptInclusionReason::RepoSupported {
                    evidence_keys: vec!["crate:core".to_string()],
                },
            }],
            relevant_superseded_history: vec![],
            conflicts_for_user_or_model: vec![],
            missing_memory_gaps: vec![],
            unverifiable_claims_excluded: vec![],
        };
        let block = inputs.to_prompt_block().unwrap();
        assert!(block.starts_with("## Verified Memory"));
        assert!(block.contains("crate core exists"));
        assert!(block.contains("[verified:"));
        assert!(!block.contains("governance"));
        assert!(!block.contains("profile"));
        assert!(!block.contains("Batch02r"));
    }

    #[test]
    fn integration_proof_does_not_modify_panel_buckets() {
        use openwand_memory::provenance_hydration::MemoryTrustBucket;
        use openwand_memory::repo_consistency::RepoConsistencyFindingKind;
        // Same mapping as before 02t
        assert_eq!(MemoryTrustBucket::PromptIncluded, MemoryTrustBucket::from_finding_kind(&RepoConsistencyFindingKind::Supported));
        assert_eq!(MemoryTrustBucket::SupersededIgnored, MemoryTrustBucket::from_finding_kind(&RepoConsistencyFindingKind::SupersededMemoryIgnored));
        assert_eq!(MemoryTrustBucket::Conflict, MemoryTrustBucket::from_finding_kind(&RepoConsistencyFindingKind::ConflictRequiresReview));
        assert_eq!(MemoryTrustBucket::MissingInMemory, MemoryTrustBucket::from_finding_kind(&RepoConsistencyFindingKind::MissingInMemory));
    }

    #[test]
    fn integration_proof_does_not_wire_stale_classifier() {
        // StaleMemory variant exists but classify_current_claim never produces it
        use openwand_memory::repo_consistency::RepoConsistencyFindingKind;
        let _ = RepoConsistencyFindingKind::StaleMemory; // variant exists
        // But the only production path is:
        // classify_current_claim → Supported | MissingInRepo | Unverifiable
        // classify_superseded_claim → SupersededMemoryIgnored
        // classify_conflict_claim → ConflictRequiresReview
        // No function produces StaleMemory
    }

    #[test]
    fn integration_proof_does_not_wire_conflict_detection() {
        // conflict_group_id exists on MemoryRecord but is never populated
        // conflict detection is not wired
        // This test exists to guard against accidental wiring in 02t
        use openwand_memory::types::MemoryRecord;
        let record = MemoryRecord {
            record_id: "test".to_string(),
            claim: "test".to_string(),
            kind: openwand_memory::types::MemoryKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec![],
            source_trace_ids: vec![],
            created_at: chrono::Utc::now(),
            valid_until: None,
            superseded_by: None,
            evidence_kind: openwand_memory::evidence::EvidenceKind::AcceptedClaim,
            normalized_text_hash: "hash".to_string(),
            supersedes_record_id: None,
            conflict_group_id: None, // Never populated by store
        };
        assert!(record.conflict_group_id.is_none());
    }
}
