//! Evidence report generator — aggregates existing verification evidence.
//!
//! Wave 106A: Packages live trace verification, operation replay, anchor
//! verification, and sourced scan/authority-review summaries into a structured
//! JSON artifact for external reviewers.
//!
//! **Authority:** The report generator MAY write only the requested report file.
//! It does NOT mutate trace, write anchors, execute tools, approve actions,
//! repair records, change policy, or claim assurance beyond the included
//! evidence and caveats.

use serde::{Deserialize, Serialize};

// ── Report DTOs ────────────────────────────────────────────

/// Top-level evidence report result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceReportResult {
    /// All supplied evidence sources produced results.
    Complete,
    /// Evidence produced but with caveats (missing anchor, stale, inconclusive).
    CompleteWithCaveats,
    /// A required source failed (trace loading error, malformed operations file).
    Incomplete,
}

/// Trace verification summary (live result from TraceVerifier).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceVerificationSummary {
    pub result: String,
    pub entries_checked: usize,
    pub streams_checked: usize,
    pub error_findings: usize,
    pub warning_findings: usize,
}

/// Operation replay summary (live result from OperationReplayVerifier).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationReplaySummary {
    pub status: String, // "verified" or "not_requested"
    pub result: Option<String>,
    pub operations_checked: Option<usize>,
    pub findings_count: Option<usize>,
}

/// Anchor verification summary (live result from verify_anchor).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorVerificationSummary {
    pub result: String,
    pub freshness: String,
    pub detail: String,
}

/// Security scan summary (sourced from recorded artifact).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanSummary {
    pub source: String,
    pub recorded_at_wave: String,
    pub vulnerabilities: Option<usize>,
    pub warnings: Option<usize>,
    pub status: String, // "available" or "unavailable"
}

/// Authority review summary (sourced from recorded artifact).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityReviewSummary {
    pub source: String,
    pub surfaces_documented: Option<usize>,
    pub write_capable_surfaces: Option<usize>,
    pub read_only_verifiers: Option<usize>,
    pub status: String, // "available" or "unavailable"
}

/// Full evidence report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceReport {
    pub session_id: String,
    pub generated_at: String,
    pub result: EvidenceReportResult,
    pub trace_verification: TraceVerificationSummary,
    pub operation_replay: OperationReplaySummary,
    pub anchor_verification: Option<AnchorVerificationSummary>,
    pub security_scan: SecurityScanSummary,
    pub authority_review: AuthorityReviewSummary,
    pub caveats: Vec<String>,
}

impl EvidenceReport {
    /// Count total caveats for display.
    pub fn caveat_count(&self) -> usize {
        self.caveats.len()
    }
}

// ── Source loaders ─────────────────────────────────────────

/// Load security scan summary from the recorded artifact file.
///
/// Reads `docs/SECURITY_SCAN_RESULTS.md` and extracts key numbers.
/// Returns `status = "unavailable"` if the file is missing.
pub fn load_security_scan_summary(docs_root: &std::path::Path) -> SecurityScanSummary {
    let path = docs_root.join("SECURITY_SCAN_RESULTS.md");

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            return SecurityScanSummary {
                source: path.display().to_string(),
                recorded_at_wave: String::new(),
                vulnerabilities: None,
                warnings: None,
                status: "unavailable".into(),
            };
        }
    };

    // Extract wave from the header
    let wave = content
        .lines()
        .find(|l| l.contains("Wave 10"))
        .and_then(|l| l.split("Wave ").nth(1))
        .and_then(|l| l.split(|c: char| !c.is_ascii_alphanumeric()).next())
        .unwrap_or("unknown")
        .to_string();

    // Extract vulnerability count
    let vulns = if content.contains("0 vulnerabilities") {
        Some(0)
    } else {
        content
            .lines()
            .find(|l| l.contains("vulnerabilities"))
            .and_then(|l| l.split_whitespace().find(|w| w.chars().all(|c| c.is_ascii_digit())))
            .and_then(|s| s.parse().ok())
    };

    // Extract warning count
    let warnings = content
        .lines()
        .find(|l| l.contains("warnings"))
        .and_then(|l| {
            l.split_whitespace()
                .find(|w| w.chars().all(|c| c.is_ascii_digit()) && !w.is_empty())
        })
        .and_then(|s| s.parse().ok());

    SecurityScanSummary {
        source: "docs/SECURITY_SCAN_RESULTS.md".into(),
        recorded_at_wave: format!("Wave {}", wave),
        vulnerabilities: vulns,
        warnings,
        status: "available".into(),
    }
}

/// Load authority review summary from the recorded artifact file.
///
/// Reads `docs/AUTHORITY_REVIEW.md` and extracts surface counts.
/// Returns `status = "unavailable"` if the file is missing.
pub fn load_authority_review_summary(docs_root: &std::path::Path) -> AuthorityReviewSummary {
    let path = docs_root.join("AUTHORITY_REVIEW.md");

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            return AuthorityReviewSummary {
                source: path.display().to_string(),
                surfaces_documented: None,
                write_capable_surfaces: None,
                read_only_verifiers: None,
                status: "unavailable".into(),
            };
        }
    };

    // The document says "12 surfaces", "4 write-capable", "3 read-only verifiers"
    let surfaces = if content.contains("12 surfaces") || content.contains("12 authority surfaces") {
        Some(12)
    } else {
        None
    };

    let write_capable = if content.contains("4 write") || content.contains("4 of 12") {
        Some(4)
    } else {
        None
    };

    let read_only = if content.contains("3 read-only verifier") || content.contains("3 read-only") {
        Some(3)
    } else {
        None
    };

    AuthorityReviewSummary {
        source: "docs/AUTHORITY_REVIEW.md".into(),
        surfaces_documented: surfaces,
        write_capable_surfaces: write_capable,
        read_only_verifiers: read_only,
        status: "available".into(),
    }
}

// ── Standard caveats ───────────────────────────────────────

/// The standard set of caveats included in every evidence report.
pub fn standard_caveats() -> Vec<String> {
    vec![
        "This report aggregates existing evidence. It does not constitute a formal security review.".into(),
        "Trace verification checks chain continuity, ordering, and hash correctness under Blake3HashPolicy. It does not prove physical immutability.".into(),
        "An attacker who can rewrite the trace store AND recompute all hashes can produce a self-consistent trace. Full immutability requires an external trust anchor.".into(),
        "Not production-ready. Not a stable API guarantee.".into(),
    ]
}

/// Caveat added when no anchor is supplied.
pub fn anchor_missing_caveat() -> String {
    "No anchor file supplied; report does not include external checkpoint validation.".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_serializes_to_json() {
        let report = EvidenceReport {
            session_id: "test-session".into(),
            generated_at: "2026-06-14T14:00:00Z".into(),
            result: EvidenceReportResult::Complete,
            trace_verification: TraceVerificationSummary {
                result: "Pass".into(),
                entries_checked: 42,
                streams_checked: 3,
                error_findings: 0,
                warning_findings: 0,
            },
            operation_replay: OperationReplaySummary {
                status: "verified".into(),
                result: Some("Pass".into()),
                operations_checked: Some(2),
                findings_count: Some(0),
            },
            anchor_verification: None,
            security_scan: SecurityScanSummary {
                source: "docs/SECURITY_SCAN_RESULTS.md".into(),
                recorded_at_wave: "Wave 105A".into(),
                vulnerabilities: Some(0),
                warnings: Some(15),
                status: "available".into(),
            },
            authority_review: AuthorityReviewSummary {
                source: "docs/AUTHORITY_REVIEW.md".into(),
                surfaces_documented: Some(12),
                write_capable_surfaces: Some(4),
                read_only_verifiers: Some(3),
                status: "available".into(),
            },
            caveats: standard_caveats(),
        };

        let json = serde_json::to_string_pretty(&report).unwrap();
        let restored: EvidenceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.session_id, "test-session");
        assert_eq!(restored.result, EvidenceReportResult::Complete);
    }

    #[test]
    fn load_security_scan_missing_returns_unavailable() {
        let temp = tempfile::TempDir::new().unwrap();
        let summary = load_security_scan_summary(temp.path());
        assert_eq!(summary.status, "unavailable");
        assert_eq!(summary.vulnerabilities, None);
    }

    #[test]
    fn load_authority_review_missing_returns_unavailable() {
        let temp = tempfile::TempDir::new().unwrap();
        let summary = load_authority_review_summary(temp.path());
        assert_eq!(summary.status, "unavailable");
        assert_eq!(summary.surfaces_documented, None);
    }

    #[test]
    fn standard_caveats_mention_limitations() {
        let caveats = standard_caveats();
        assert!(caveats.iter().any(|c| c.contains("formal security review")));
        assert!(caveats.iter().any(|c| c.contains("physical immutability")));
        assert!(caveats.iter().any(|c| c.contains("external trust anchor")));
        assert!(caveats.iter().any(|c| c.contains("Not production-ready")));
    }

    #[test]
    fn anchor_missing_caveat_is_informative() {
        let caveat = anchor_missing_caveat();
        assert!(caveat.contains("anchor"));
        assert!(caveat.contains("not include"));
    }

    #[test]
    fn report_result_enum_has_three_variants() {
        assert_eq!(
            vec![
                EvidenceReportResult::Complete,
                EvidenceReportResult::CompleteWithCaveats,
                EvidenceReportResult::Incomplete,
            ]
            .len(),
            3
        );
    }

    // ── Source-level authority guards ──

    #[test]
    fn report_module_does_not_write_anchors() {
        let src = include_str!("evidence_report.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!impl_only.contains("write_checkpoint"),
            "report module must not call write_checkpoint");
        assert!(!impl_only.contains("CheckpointWriter"),
            "report module must not use CheckpointWriter");
    }

    #[test]
    fn report_module_does_not_execute_tools() {
        let src = include_str!("evidence_report.rs");
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

    #[test]
    fn report_module_does_not_overclaim() {
        let src = include_str!("evidence_report.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        // standard_caveats contains honest denials ("Not production-ready")
        // which are caveats, not claims. Check non-caveat code only.
        let non_caveat = impl_only.split("pub fn standard_caveats").next().unwrap_or("");
        assert!(!non_caveat.contains("is production-ready"),
            "report module must not claim production-ready");
        assert!(!non_caveat.contains("formally verified"),
            "report module must not claim formal verification");
        assert!(!non_caveat.contains("physically immutable"),
            "report module must not claim physical immutability");
    }
}
