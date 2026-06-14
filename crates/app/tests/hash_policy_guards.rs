//! Guard tests for hash verification policy architecture (Wave 98A).

#[cfg(test)]
mod hash_policy_guards {
    /// The verifier module must define a HashVerificationPolicy trait.
    #[test]
    fn hash_verification_policy_trait_exists() {
        let src = include_str!("../../../crates/trace/src/verifier.rs");
        assert!(
            src.contains("pub trait HashVerificationPolicy"),
            "HashVerificationPolicy trait must be defined in verifier.rs"
        );
    }

    /// Blake3HashPolicy must be a public struct.
    #[test]
    fn blake3_hash_policy_exists() {
        let src = include_str!("../../../crates/trace/src/verifier.rs");
        assert!(
            src.contains("pub struct Blake3HashPolicy"),
            "Blake3HashPolicy struct must be defined"
        );
        assert!(
            src.contains("pub fn compute_hash"),
            "Blake3HashPolicy must have a public compute_hash function"
        );
    }

    /// verify_with_hash_policy method must exist on TraceVerifier.
    #[test]
    fn verify_with_hash_policy_method_exists() {
        let src = include_str!("../../../crates/trace/src/verifier.rs");
        assert!(
            src.contains("verify_with_hash_policy"),
            "TraceVerifier must have verify_with_hash_policy method"
        );
    }

    /// HashCorrectnessValid must be a verification check variant.
    #[test]
    fn hash_correctness_check_exists() {
        let src = include_str!("../../../crates/trace/src/verifier.rs");
        assert!(
            src.contains("HashCorrectnessValid"),
            "VerificationCheck must have HashCorrectnessValid variant"
        );
    }

    /// The store must implement HashVerificationPolicy for StoredEvent.
    #[test]
    fn store_implements_hash_policy_for_stored_event() {
        let src = include_str!("../../../crates/store/src/envelope.rs");
        assert!(
            src.contains("HashVerificationPolicy<StoredEvent>"),
            "Store must implement HashVerificationPolicy for StoredEvent"
        );
        assert!(
            src.contains("serde_json::to_string(&event.0)"),
            "Store hash policy must serialize inner event (.0)"
        );
    }

    /// The trait must be read-only — no mutation methods.
    #[test]
    fn hash_policy_is_read_only() {
        let src = include_str!("../../../crates/trace/src/verifier.rs");
        // The trait section is between "pub trait HashVerificationPolicy" and "pub struct Blake3HashPolicy"
        let trait_start = src.find("pub trait HashVerificationPolicy").unwrap();
        let trait_end = src.find("pub struct Blake3HashPolicy").unwrap();
        let trait_section = &src[trait_start..trait_end];
        assert!(
            !trait_section.contains("fn append") && !trait_section.contains("fn mutate")
                && !trait_section.contains("fn repair") && !trait_section.contains("fn execute")
                && !trait_section.contains("fn write"),
            "HashVerificationPolicy trait must not have mutation methods"
        );
    }

    /// The documentation must state the external anchor limitation.
    #[test]
    fn hash_policy_documents_external_anchor_limitation() {
        let src = include_str!("../../../crates/trace/src/verifier.rs");
        assert!(
            src.contains("external trust anchor") || src.contains("external anchor"),
            "Hash policy documentation must mention external trust anchor limitation"
        );
    }
}
