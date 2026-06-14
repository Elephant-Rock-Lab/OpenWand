//! Integration tests for security scan results presence (Wave 105A).
//!
//! Proves:
//! 1. SECURITY_SCAN_RESULTS.md exists and is non-empty.
//! 2. Results document does not overclaim.
//! 3. Results document mentions upstream-blocked findings honestly.

#[cfg(test)]
mod security_scan_guards {
    use std::path::PathBuf;

    fn scan_doc_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("docs")
            .join("SECURITY_SCAN_RESULTS.md")
    }

    /// Guard: security scan results document exists.
    #[test]
    fn security_scan_results_exist() {
        let path = scan_doc_path();
        assert!(path.exists(), "SECURITY_SCAN_RESULTS.md must exist at docs/SECURITY_SCAN_RESULTS.md");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.is_empty(), "SECURITY_SCAN_RESULTS.md must not be empty");
    }

    /// Guard: scan results do not overclaim security.
    #[test]
    fn scan_results_do_not_overclaim() {
        let path = scan_doc_path();
        let content = std::fs::read_to_string(&path).unwrap();

        // Must NOT claim production readiness or formal review
        assert!(!content.to_lowercase().contains("production-ready"),
            "scan results must not claim production-ready");
        assert!(!content.to_lowercase().contains("formally verified"),
            "scan results must not claim formal verification");
        assert!(!content.to_lowercase().contains("fully secure"),
            "scan results must not claim fully secure");
        assert!(!content.to_lowercase().contains("zero-day"),
            "scan results must not claim zero-day coverage");
    }

    /// Guard: scan results honestly mention limitations.
    #[test]
    fn scan_results_mention_limitations() {
        let path = scan_doc_path();
        let content = std::fs::read_to_string(&path).unwrap();

        // Must mention upstream-blocked findings
        assert!(content.contains("upstream-blocked"),
            "scan results must classify upstream-blocked findings");
        // Must mention what it does NOT cover
        assert!(content.to_lowercase().contains("does not cover"),
            "scan results must state what they do not cover");
        // Must mention not a formal review
        assert!(content.contains("not a formal security review"),
            "scan results must state they are not a formal security review");
    }

    /// Guard: scan results record specific counts.
    #[test]
    fn scan_results_record_counts() {
        let path = scan_doc_path();
        let content = std::fs::read_to_string(&path).unwrap();

        // Must record vulnerability count
        assert!(content.contains("0 vulnerabilities") || content.contains("0 CVE"),
            "scan results must record vulnerability count");
        // Must record clippy posture
        assert!(content.contains("0 warnings"),
            "scan results must record clippy posture");
        // Must record dependency count
        assert!(content.contains("721"),
            "scan results must record dependency count");
    }
}
