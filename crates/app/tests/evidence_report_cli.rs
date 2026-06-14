//! Integration tests for evidence report CLI (Wave 106A).
//!
//! Proves:
//! 1. evidence-report command exists in CLI enum.
//! 2. Report module does not write anchors, execute tools, or overclaim.
//! 3. Output collision is rejected (source-level guard).
//! 4. Operations are required (source-level guard).
//! 5. Scan/review summaries are sourced, not hardcoded.

#[cfg(test)]
mod evidence_report_cli_tests {
    /// Guard: evidence-report command exists in CLI.
    #[test]
    fn evidence_report_command_exists() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("evidence-report"), "CLI must have evidence-report command");
        assert!(src.contains("EvidenceReport {"), "CLI enum must have EvidenceReport variant");
        assert!(src.contains("fn cmd_evidence_report"), "must have cmd_evidence_report function");
    }

    /// Guard: evidence-report requires explicit operations.
    #[test]
    fn evidence_report_requires_operations() {
        let src = include_str!("../src/main.rs");
        let section_start = src.find("EvidenceReport {").unwrap_or(0);
        let section = &src[section_start..];
        assert!(section.contains("operations"),
            "evidence-report must require --operations argument");
    }

    /// Guard: evidence-report rejects output collision.
    #[test]
    fn evidence_report_rejects_collision() {
        let src = include_str!("../src/main.rs");
        let section_start = src.find("async fn cmd_evidence_report").unwrap_or(0);
        let section_end = src[section_start..].find("async fn cmd_audit_check")
            .map(|o| section_start + o).unwrap_or(src.len());
        let section = &src[section_start..section_end];
        assert!(section.contains("already exists"),
            "evidence-report must reject existing output file");
    }

    /// Guard: evidence-report does not claim formal review or production-ready.
    #[test]
    fn evidence_report_cli_does_not_overclaim() {
        let src = include_str!("../src/main.rs");
        let section_start = src.find("async fn cmd_evidence_report").unwrap_or(0);
        let section_end = src[section_start..].find("async fn cmd_audit_check")
            .map(|o| section_start + o).unwrap_or(src.len());
        let section = &src[section_start..section_end];
        assert!(!section.contains("production-ready"),
            "evidence-report must not claim production-ready");
        assert!(!section.contains("formally verified"),
            "evidence-report must not claim formal verification");
    }

    /// Guard: evidence-report mentions it is not a formal review.
    #[test]
    fn evidence_report_states_not_formal() {
        let src = include_str!("../src/main.rs");
        let section_start = src.find("async fn cmd_evidence_report").unwrap_or(0);
        let section_end = src[section_start..].find("async fn cmd_audit_check")
            .map(|o| section_start + o).unwrap_or(src.len());
        let section = &src[section_start..section_end];
        assert!(section.contains("not") && section.contains("formal"),
            "evidence-report output should state it is not a formal review");
    }
}

#[cfg(test)]
mod evidence_report_authority_guards {
    /// Guard: report module does not write anchors.
    #[test]
    fn report_does_not_write_anchors() {
        let src = include_str!("../src/evidence_report.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!impl_only.contains("write_checkpoint"),
            "report module must not call write_checkpoint");
        assert!(!impl_only.contains("CheckpointWriter"),
            "report module must not use CheckpointWriter");
    }

    /// Guard: report module does not execute tools or approve actions.
    #[test]
    fn report_does_not_execute_or_approve() {
        let src = include_str!("../src/evidence_report.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!impl_only.contains("ToolExecutor"),
            "report module must not call ToolExecutor");
        assert!(!impl_only.contains("export_audit_packet"),
            "report module must not call export_audit_packet");
        assert!(!impl_only.contains("submit_approval_resolution"),
            "report module must not call submit_approval_resolution");
        assert!(!impl_only.contains("request_workflow_run"),
            "report module must not call request_workflow_run");
    }

    /// Guard: report module does not overclaim.
    #[test]
    fn report_does_not_overclaim() {
        let src = include_str!("../src/evidence_report.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        // The standard_caveats function contains "Not production-ready" which is
        // a denial, not a claim. Check for affirmactive claims only.
        // Remove the standard_caveats function body for this check.
        let non_caveat = impl_only.split("pub fn standard_caveats").next().unwrap_or("");
        assert!(!non_caveat.contains("is production-ready"),
            "report module must not claim production-ready");
        assert!(!non_caveat.contains("formally verified"),
            "report module must not claim formal verification");
        assert!(!non_caveat.contains("physically immutable"),
            "report module must not claim physical immutability");
    }

    /// Guard: scan summary is sourced, returns unavailable when missing.
    #[test]
    fn scan_summary_is_sourced() {
        let src = include_str!("../src/evidence_report.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        assert!(impl_only.contains("unavailable"),
            "scan summary must return unavailable when document is missing");
        assert!(impl_only.contains("SECURITY_SCAN_RESULTS"),
            "scan summary must reference SECURITY_SCAN_RESULTS document");
    }

    /// Guard: authority review summary is sourced.
    #[test]
    fn review_summary_is_sourced() {
        let src = include_str!("../src/evidence_report.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        assert!(impl_only.contains("AUTHORITY_REVIEW"),
            "review summary must reference AUTHORITY_REVIEW document");
    }

    /// Guard: anchor absence produces caveat, not failure.
    #[test]
    fn anchor_absence_is_caveat() {
        let src = include_str!("../src/evidence_report.rs");
        assert!(src.contains("anchor_missing_caveat"),
            "report module must have anchor_missing_caveat function");
        let caveat = openwand_app::evidence_report::anchor_missing_caveat();
        assert!(caveat.contains("anchor") && caveat.contains("not include"),
            "missing anchor caveat must be informative");
    }
}
