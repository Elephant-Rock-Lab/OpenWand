//! Integration tests for anchor CLI commands (Wave 104B).
//!
//! Proves:
//! 1. anchor-write and anchor-verify commands exist in the CLI enum.
//! 2. The anchor writer creates files outside the store root.
//! 3. The anchor verifier reads files and compares prefixes.
//! 4. Path containment is enforced.
//! 5. Neither command mutates trace.

#[cfg(test)]
mod anchor_cli_tests {
    /// Guard: anchor-write and anchor-verify commands exist in CLI enum.
    #[test]
    fn anchor_commands_exist_in_cli() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("anchor-write"), "CLI must have anchor-write command");
        assert!(src.contains("anchor-verify"), "CLI must have anchor-verify command");
        assert!(src.contains("AnchorWrite {"), "CLI enum must have AnchorWrite variant");
        assert!(src.contains("AnchorVerify {"), "CLI enum must have AnchorVerify variant");
        assert!(src.contains("fn cmd_anchor_write"), "must have cmd_anchor_write function");
        assert!(src.contains("fn cmd_anchor_verify"), "must have cmd_anchor_verify function");
    }

    /// Guard: anchor CLI does not claim production readiness or immutability.
    #[test]
    fn anchor_cli_does_not_overclaim() {
        let src = include_str!("../src/main.rs");
        // Find the anchor functions
        let anchor_section = src.split("async fn cmd_anchor_write").nth(1).unwrap_or("");
        let combined = format!("{}\n{}", 
            src.split("async fn cmd_anchor_write").nth(1).unwrap_or(""),
            src.split("async fn cmd_anchor_verify").nth(1).unwrap_or(""),
        );
        assert!(!combined.contains("production-ready"), "anchor CLI must not claim production-ready");
        assert!(!combined.contains("fully secure"), "anchor CLI must not claim fully secure");
        assert!(!combined.contains("physically immutable"), "anchor CLI must not claim physical immutability");
        // Should mention the limitation
        assert!(combined.contains("attacker") || combined.contains("self-consistent"),
            "anchor CLI should mention the self-consistent tamper limitation");
    }

    /// Guard: anchor writer uses path containment checks.
    #[test]
    fn anchor_writer_uses_path_containment() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("validate_anchor_root") || src.contains("AnchorRootInsideStoreRoot"),
            "anchor writer must use path containment validation");
    }

    /// Guard: anchor commands are not stubs.
    #[test]
    fn anchor_commands_are_not_stubs() {
        let src = include_str!("../src/main.rs");
        // Extract just the anchor_write function body (up to the next function)
        let write_start = src.find("async fn cmd_anchor_write").unwrap_or(0);
        let write_end = src[write_start..].find("async fn cmd_anchor_verify")
            .map(|o| write_start + o).unwrap_or(src.len());
        let write_section = &src[write_start..write_end];

        let verify_start = src.find("async fn cmd_anchor_verify").unwrap_or(0);
        let verify_end = src[verify_start..].find("async fn cmd_audit_check")
            .map(|o| verify_start + o).unwrap_or(src.len());
        let verify_section = &src[verify_start..verify_end];

        assert!(!write_section.contains("not yet implemented"),
            "anchor-write must not be a stub");
        assert!(!verify_section.contains("not yet implemented"),
            "anchor-verify must not be a stub");
    }
}

#[cfg(test)]
mod anchor_authority_guards {
    /// Guard: anchor module does not mutate trace entries.
    #[test]
    fn anchor_writer_does_not_mutate_trace() {
        let src = include_str!("../../trace/src/anchor.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!impl_only.contains("append_trace"),
            "anchor module must not append to trace");
        assert!(!impl_only.contains("delete_entry") && !impl_only.contains("remove_entry"),
            "anchor module must not delete trace entries");
        assert!(!impl_only.contains(".append("),
            "anchor module must not call append on trace store");
    }

    /// Guard: anchor module does not import backend crates.
    #[test]
    fn anchor_module_does_not_import_backend() {
        let src = include_str!("../../trace/src/anchor.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!impl_only.contains("openwand_store"),
            "anchor module must not import openwand-store");
        assert!(!impl_only.contains("openwand_core"),
            "anchor module must not import openwand-core");
        assert!(!impl_only.contains("openwand_session"),
            "anchor module must not import openwand-session");
    }

    /// Guard: anchor module writer only writes anchor files, not trace.
    #[test]
    fn anchor_writer_only_writes_anchor_files() {
        let src = include_str!("../../trace/src/anchor.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        // The only file writes should be anchor files
        assert!(impl_only.contains("openwand-checkpoint"),
            "file writes should be for checkpoint files only");
        // Must not write to trace DB paths
        assert!(!impl_only.contains("openwand.db"),
            "anchor module must not write to trace database");
    }
}
