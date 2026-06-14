//! Integration tests for structured authority review (Wave 105B).
//!
//! Proves:
//! 1. AUTHORITY_REVIEW.md exists and covers all 12 surfaces.
//! 2. Review does not overclaim.
//! 3. Review documents write-authority surfaces honestly.
//! 4. Review documents read-only verifiers.

#[cfg(test)]
mod authority_review_guards {
    use std::path::PathBuf;

    fn review_doc_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("docs")
            .join("AUTHORITY_REVIEW.md")
    }

    /// Guard: authority review document exists.
    #[test]
    fn authority_review_exists() {
        let path = review_doc_path();
        assert!(path.exists(), "AUTHORITY_REVIEW.md must exist");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.len() > 5000, "AUTHORITY_REVIEW.md must be substantial (>5KB)");
    }

    /// Guard: review covers all 12 authority surfaces.
    #[test]
    fn review_covers_all_surfaces() {
        let content = std::fs::read_to_string(review_doc_path()).unwrap();
        for surface in &[
            "S1", "S2", "S3", "S4", "S5", "S6",
            "S7", "S8", "S9", "S10", "S11", "S12",
        ] {
            assert!(content.contains(surface),
                "review must cover surface {}", surface);
        }
        // Verify key surface names
        assert!(content.contains("Desktop UI"), "must cover desktop UI");
        assert!(content.contains("UiSessionService"), "must cover service");
        assert!(content.contains("Policy"), "must cover policy gate");
        assert!(content.contains("Tool Executor"), "must cover tool executor");
        assert!(content.contains("Trace Store"), "must cover trace store");
        assert!(content.contains("Trace Verifier"), "must cover trace verifier");
        assert!(content.contains("Operation Replay"), "must cover operation replay");
        assert!(content.contains("Anchor Writer"), "must cover anchor writer");
        assert!(content.contains("Anchor Verifier"), "must cover anchor verifier");
    }

    /// Guard: review does not overclaim.
    #[test]
    fn review_does_not_overclaim() {
        let content = std::fs::read_to_string(review_doc_path()).unwrap();
        assert!(!content.to_lowercase().contains("production-ready"),
            "authority review must not claim production-ready");
        assert!(!content.to_lowercase().contains("formally verified"),
            "authority review must not claim formal verification");
        assert!(!content.to_lowercase().contains("zero vulnerabilities"),
            "authority review must not claim zero vulnerabilities");
        assert!(!content.to_lowercase().contains("fully secure"),
            "authority review must not claim fully secure");
    }

    /// Guard: review documents read-only verifiers honestly.
    #[test]
    fn review_documents_read_only_verifiers() {
        let content = std::fs::read_to_string(review_doc_path()).unwrap();
        assert!(content.contains("Read-Only Verifiers"),
            "review must have a read-only verifiers summary");
        assert!(content.contains("None") || content.contains("none"),
            "review must state verifiers write nothing");
    }

    /// Guard: review documents write-authority surfaces.
    #[test]
    fn review_documents_write_authority() {
        let content = std::fs::read_to_string(review_doc_path()).unwrap();
        assert!(content.contains("Write-Authority Surfaces") || content.contains("write-authority"),
            "review must enumerate write-authority surfaces");
        // Must mention the specific gates that control each write surface
        assert!(content.contains("Policy gate") || content.contains("policy gate"),
            "review must document policy gate as enforcement");
        assert!(content.contains("sandbox"),
            "review must document sandbox containment");
        assert!(content.contains("path containment"),
            "review must document anchor path containment");
    }

    /// Guard: review honestly documents residual risks.
    #[test]
    fn review_documents_residual_risks() {
        let content = std::fs::read_to_string(review_doc_path()).unwrap();
        assert!(content.contains("Residual Risk") || content.contains("residual risk"),
            "review must document residual risks");
        assert!(content.contains("Physical") || content.contains("physical"),
            "review must mention physical trace store mutability");
        assert!(content.contains("TOCTOU"),
            "review must mention TOCTOU residual");
    }

    /// Guard: review is not a formal security review.
    #[test]
    fn review_states_it_is_not_formal() {
        let content = std::fs::read_to_string(review_doc_path()).unwrap();
        assert!(content.contains("not a formal security review") || content.contains("Not a formal security review"),
            "review must state it is not a formal security review");
    }
}
