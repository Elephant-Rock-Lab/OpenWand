//! Guard tests for Linux GUI smoke test results (Wave 109A).
//!
//! Proves:
//! 1. Smoke test document exists.
//! 2. Result is honestly classified as Partial.
//! 3. Document does not claim full Linux support.

#[cfg(test)]
mod linux_gui_smoke_guards {
    use std::path::PathBuf;

    fn doc_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("docs")
            .join("LINUX_GUI_SMOKE_TEST.md")
    }

    /// Guard: smoke test document exists.
    #[test]
    fn smoke_test_doc_exists() {
        let path = doc_path();
        assert!(path.exists(), "LINUX_GUI_SMOKE_TEST.md must exist");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.is_empty(), "document must not be empty");
    }

    /// Guard: result classified as Partial.
    #[test]
    fn result_is_partial() {
        let content = std::fs::read_to_string(doc_path()).unwrap();
        assert!(content.contains("Partial"),
            "smoke test must be classified as Partial");
        assert!(content.contains("Classification: Partial") || content.contains("**Partial**"),
            "smoke test must state Partial classification");
    }

    /// Guard: does not claim full Linux support.
    #[test]
    fn does_not_claim_full_linux_support() {
        let content = std::fs::read_to_string(doc_path()).unwrap();
        assert!(!content.to_lowercase().contains("fully supported on linux"),
            "must not claim fully supported on Linux");
        assert!(!content.to_lowercase().contains("production ready"),
            "must not claim production ready");
    }

    /// Guard: distinguishes compile from runtime validation.
    #[test]
    fn distinguishes_compile_from_runtime() {
        let content = std::fs::read_to_string(doc_path()).unwrap();
        assert!(content.contains("compile") && content.to_lowercase().contains("runtime"),
            "must distinguish compile validation from runtime validation");
    }

    /// Guard: honestly documents what was NOT proven.
    #[test]
    fn documents_what_was_not_proven() {
        let content = std::fs::read_to_string(doc_path()).unwrap();
        assert!(content.contains("NOT Prove") || content.contains("does not prove"),
            "must document what was not proven");
        assert!(content.contains("Visual rendering") || content.contains("rendering"),
            "must mention visual rendering was not verified");
    }
}
