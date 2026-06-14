//! Documentation-presence guards for security review preparation (Wave 94A).
//!
//! These tests verify the security review prep document exists, contains
//! required sections, does not make unsupported affirmative claims, and
//! documents the read-only verifier authority boundary.

#[cfg(test)]
mod security_review_guards {
    use std::path::PathBuf;

    fn doc_path() -> PathBuf {
        // Walk up from CARGO_MANIFEST_DIR to find docs/SECURITY_REVIEW_PREP.md
        let manifest = env!("CARGO_MANIFEST_DIR");
        let p = PathBuf::from(manifest)
            .parent() // crates/
            .unwrap()
            .parent() // workspace root
            .unwrap()
            .join("docs")
            .join("SECURITY_REVIEW_PREP.md");
        assert!(p.exists(), "SECURITY_REVIEW_PREP.md not found at {:?}", p);
        p
    }

    fn doc_content() -> String {
        std::fs::read_to_string(doc_path())
            .expect("failed to read SECURITY_REVIEW_PREP.md")
    }

    #[test]
    fn security_review_doc_exists() {
        let content = doc_content();

        // Required section headers
        let required = [
            "Section 1: Threat Model",
            "Section 2: Authority-Boundary Checklist",
            "Section 3: Security Review Checklist",
            "Section 4: Caveat Ledger",
            "Section 5: Review-Ready Assets",
        ];

        for section in &required {
            assert!(
                content.contains(section),
                "missing required section: {}",
                section
            );
        }

        // Must mention the scope distinction
        assert!(
            content.contains("security review preparation"),
            "must state this is preparation, not review"
        );
    }

    #[test]
    fn security_review_no_overclaim() {
        let content = doc_content();

        // Ban affirmative unsupported claims - these are phrases that would
        // constitute a security or production-readiness assertion.
        // We check for these as affirmative statements, not as caveats.
        let banned_affirmative = [
            "is production-ready",
            "production ready",
            "completed formal security review",
            "formally security reviewed",
            "fully secure",
            "proves full immutability",
            "immutable trace guarantee",
            "cryptographic immutability verified",
            "formally verified",
            "security certified",
        ];

        for phrase in &banned_affirmative {
            let lower = content.to_lowercase();
            assert!(
                !lower.contains(phrase),
                "document contains banned affirmative overclaim phrase: \"{}\"",
                phrase
            );
        }

        // Must contain caveat language (negative statements are allowed)
        assert!(
            content.contains("not production-ready") || content.contains("Not production"),
            "must state not production-ready"
        );
        assert!(
            content.contains("not a formal security review")
                || content.contains("Not a formal security review")
                || content.contains("NOT a formal security review"),
            "must state this is not a formal security review"
        );
        assert!(
            content.contains("not full immutability")
                || content.contains("No full immutability"),
            "must acknowledge no full immutability proof"
        );
    }

    #[test]
    fn security_review_mentions_read_only_verifiers() {
        let content = doc_content();

        // Trace verifier read-only boundary must be documented
        assert!(
            content.contains("may not") && content.contains("Mutate trace entries"),
            "must document trace verifier may not mutate"
        );
        assert!(
            content.contains("Recompute entry hashes") == false
                || content.contains("may not") || content.contains("MAY NOT"),
            "must state verifier may not recompute hashes"
        );

        // Operation replay verifier read-only boundary must be documented
        assert!(
            content.contains("Execute tools or workflows")
                || content.contains("Execute tools or workflows"),
            "must document operation replay may not execute"
        );
        assert!(
            content.contains("Instantiate runners") == false
                || content.contains("may not") || content.contains("MAY NOT"),
            "must state operation replay may not instantiate components"
        );

        // Central authority boundary statement
        assert!(
            content.contains("READ authority"),
            "must state the verifier is a READ authority"
        );
        assert!(
            content.contains("not a new WRITE authority"),
            "must state the verifier is not a new WRITE authority"
        );
    }
}
