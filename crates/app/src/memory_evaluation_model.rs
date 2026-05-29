//! Mock model evaluation runner — deterministic model behaviors for CI.

use openwand_memory::evaluation::MockEvaluationBehavior;
use openwand_memory::provenance_hydration::HydratedMemoryClaim;

/// Run a mock model behavior against prompt inputs.
/// Returns deterministic output for CI evaluation.
pub fn run_mock_model(
    behavior: &MockEvaluationBehavior,
    _prompt_block: &Option<String>,
    included_claims: &[HydratedMemoryClaim],
    excluded_claims: &[HydratedMemoryClaim],
) -> String {
    match behavior {
        MockEvaluationBehavior::EchoIncludedMemory => {
            // Return all included claim texts
            included_claims
                .iter()
                .map(|c| c.claim_text.as_str())
                .collect::<Vec<_>>()
                .join(". ")
        }
        MockEvaluationBehavior::IgnoreIncludedMemory => {
            // Return generic response, ignoring all memory
            "I don't have enough information to answer that question.".to_string()
        }
        MockEvaluationBehavior::UseExcludedMemory => {
            // Return excluded claim texts (negative test)
            excluded_claims
                .iter()
                .map(|c| c.claim_text.as_str())
                .collect::<Vec<_>>()
                .join(". ")
        }
        MockEvaluationBehavior::HallucinateUnsupportedClaim => {
            // Return a fabricated claim not in memory
            "The project uses a microservices architecture with 47 services.".to_string()
        }
        MockEvaluationBehavior::CorrectAnswer { text } => {
            text.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_memory::provenance_hydration::{
        HydratedMemoryClaim, MemoryEvidenceProvenance, MemoryTrustBucket,
        ProvenanceHydrationStatus,
    };
    use openwand_memory::repo_consistency::ConsistencySeverity;

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

    #[test]
    fn mock_model_echoes_included_memory() {
        let c1 = make_claim("claim A", MemoryTrustBucket::PromptIncluded);
        let c2 = make_claim("claim B", MemoryTrustBucket::PromptIncluded);
        let output = run_mock_model(
            &MockEvaluationBehavior::EchoIncludedMemory,
            &None,
            &[c1, c2],
            &[],
        );
        assert!(output.contains("claim A"));
        assert!(output.contains("claim B"));
    }

    #[test]
    fn mock_model_can_ignore_required_memory() {
        let c1 = make_claim("important fact", MemoryTrustBucket::PromptIncluded);
        let output = run_mock_model(
            &MockEvaluationBehavior::IgnoreIncludedMemory,
            &None,
            &[c1],
            &[],
        );
        assert!(!output.contains("important fact"));
        assert!(output.contains("don't have enough"));
    }

    #[test]
    fn mock_model_can_use_excluded_memory_for_negative_test() {
        let excluded = make_claim("unverifiable rumor", MemoryTrustBucket::Unverifiable);
        let output = run_mock_model(
            &MockEvaluationBehavior::UseExcludedMemory,
            &None,
            &[],
            &[excluded],
        );
        assert!(output.contains("unverifiable rumor"));
    }

    #[test]
    fn mock_model_can_hallucinate_unsupported_claim() {
        let output = run_mock_model(
            &MockEvaluationBehavior::HallucinateUnsupportedClaim,
            &None,
            &[],
            &[],
        );
        assert!(output.contains("microservices"));
        assert!(output.contains("47 services"));
    }

    #[test]
    fn mock_model_output_is_deterministic() {
        let c = make_claim("test", MemoryTrustBucket::PromptIncluded);
        let o1 = run_mock_model(&MockEvaluationBehavior::EchoIncludedMemory, &None, &[c.clone()], &[]);
        let o2 = run_mock_model(&MockEvaluationBehavior::EchoIncludedMemory, &None, &[c], &[]);
        assert_eq!(o1, o2);
    }
}
