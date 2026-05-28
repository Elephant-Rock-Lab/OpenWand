//! Repo consistency report types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::evidence::EvidenceKind;

/// Clock abstraction for deterministic testing.
pub trait RepoConsistencyClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// System clock (production).
pub struct SystemClock;

impl RepoConsistencyClock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Fixed clock (testing).
pub struct FixedClock {
    pub timestamp: DateTime<Utc>,
}

impl RepoConsistencyClock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

/// The full consistency report comparing memory claims to repo observations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConsistencyReport {
    pub repo_root: PathBuf,
    pub checked_at: DateTime<Utc>,
    pub summary: RepoConsistencySummary,
    pub findings: Vec<RepoConsistencyFinding>,
    pub memory_inputs: RepoMemoryInputSummary,
    pub repo_inputs: RepoObservationSummary,
}

/// Counts per finding kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoConsistencySummary {
    pub supported: usize,
    pub stale: usize,
    pub missing_in_repo: usize,
    pub missing_in_memory: usize,
    pub superseded_ignored: usize,
    pub conflicted: usize,
    pub unverifiable: usize,
}

impl RepoConsistencySummary {
    pub fn total(&self) -> usize {
        self.supported
            + self.stale
            + self.missing_in_repo
            + self.missing_in_memory
            + self.superseded_ignored
            + self.conflicted
            + self.unverifiable
    }

    /// Compute summary from findings — the authoritative count.
    pub fn from_findings(findings: &[RepoConsistencyFinding]) -> Self {
        let mut s = Self {
            supported: 0,
            stale: 0,
            missing_in_repo: 0,
            missing_in_memory: 0,
            superseded_ignored: 0,
            conflicted: 0,
            unverifiable: 0,
        };
        for f in findings {
            match f.kind {
                RepoConsistencyFindingKind::Supported => s.supported += 1,
                RepoConsistencyFindingKind::StaleMemory => s.stale += 1,
                RepoConsistencyFindingKind::MissingInRepo => s.missing_in_repo += 1,
                RepoConsistencyFindingKind::MissingInMemory => s.missing_in_memory += 1,
                RepoConsistencyFindingKind::SupersededMemoryIgnored => s.superseded_ignored += 1,
                RepoConsistencyFindingKind::ConflictRequiresReview => s.conflicted += 1,
                RepoConsistencyFindingKind::Unverifiable => s.unverifiable += 1,
            }
        }
        s
    }
}

/// What kind of consistency finding this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoConsistencyFindingKind {
    /// Memory claim matches repo observation.
    Supported,
    /// Memory claims property V1 but repo shows V2 for the same object.
    StaleMemory,
    /// Memory claims X exists, but repo has no observable evidence.
    MissingInRepo,
    /// Repo observation has no corresponding memory claim.
    MissingInMemory,
    /// Superseded memory record — not treated as current truth.
    SupersededMemoryIgnored,
    /// Conflict-grouped record — needs human review.
    ConflictRequiresReview,
    /// Claim outside v0 grammar — cannot be verified.
    Unverifiable,
}

/// Severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsistencySeverity {
    Low,
    Medium,
    High,
}

/// A single consistency finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConsistencyFinding {
    pub kind: RepoConsistencyFindingKind,
    pub claim_text: Option<String>,
    pub evidence_kind: Option<EvidenceKind>,
    pub repo_evidence_key: Vec<String>,
    pub severity: ConsistencySeverity,
    pub detail: String,
}

/// Summary of memory inputs used for the check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoMemoryInputSummary {
    pub current_claims_count: usize,
    pub superseded_count: usize,
    pub conflict_groups_count: usize,
}

/// Summary of repo observations used for the check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoObservationSummary {
    pub crates_count: usize,
    pub dependencies_count: usize,
    pub docs_count: usize,
}

/// The consistency checker — generic over memory store and filesystem.
pub struct RepoConsistencyChecker<C: RepoConsistencyClock> {
    pub clock: C,
}

impl<C: RepoConsistencyClock> RepoConsistencyChecker<C> {
    pub fn new(clock: C) -> Self {
        Self { clock }
    }

    /// Produce an empty report for a repo with no memory and no observations.
    pub fn empty_report(&self, repo_root: PathBuf) -> RepoConsistencyReport {
        let checked_at = self.clock.now();
        RepoConsistencyReport {
            repo_root,
            checked_at,
            summary: RepoConsistencySummary {
                supported: 0,
                stale: 0,
                missing_in_repo: 0,
                missing_in_memory: 0,
                superseded_ignored: 0,
                conflicted: 0,
                unverifiable: 0,
            },
            findings: vec![],
            memory_inputs: RepoMemoryInputSummary {
                current_claims_count: 0,
                superseded_count: 0,
                conflict_groups_count: 0,
            },
            repo_inputs: RepoObservationSummary {
                crates_count: 0,
                dependencies_count: 0,
                docs_count: 0,
            },
        }
    }
}

/// Normalize a repo-relative path for stable comparison.
/// Strips leading `./`, collapses `\\` to `/`, trims whitespace.
pub fn normalize_repo_path(path: &std::path::Path) -> String {
    let s = path.to_string_lossy();
    let s = s.trim_start_matches("./");
    s.replace('\\', "/").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_consistency_report_serializes() {
        let report = RepoConsistencyReport {
            repo_root: PathBuf::from("/test/repo"),
            checked_at: DateTime::UNIX_EPOCH,
            summary: RepoConsistencySummary {
                supported: 1,
                stale: 0,
                missing_in_repo: 0,
                missing_in_memory: 0,
                superseded_ignored: 0,
                conflicted: 0,
                unverifiable: 0,
            },
            findings: vec![RepoConsistencyFinding {
                kind: RepoConsistencyFindingKind::Supported,
                claim_text: Some("test claim".to_string()),
                evidence_kind: Some(EvidenceKind::AcceptedClaim),
                repo_evidence_key: vec!["crate:core".to_string()],
                severity: ConsistencySeverity::Low,
                detail: "test detail".to_string(),
            }],
            memory_inputs: RepoMemoryInputSummary {
                current_claims_count: 1,
                superseded_count: 0,
                conflict_groups_count: 0,
            },
            repo_inputs: RepoObservationSummary {
                crates_count: 1,
                dependencies_count: 0,
                docs_count: 0,
            },
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("Supported"));
        assert!(json.contains("test claim"));
    }

    #[test]
    fn repo_consistency_finding_kinds_are_stable() {
        let kinds = vec![
            RepoConsistencyFindingKind::Supported,
            RepoConsistencyFindingKind::StaleMemory,
            RepoConsistencyFindingKind::MissingInRepo,
            RepoConsistencyFindingKind::MissingInMemory,
            RepoConsistencyFindingKind::SupersededMemoryIgnored,
            RepoConsistencyFindingKind::ConflictRequiresReview,
            RepoConsistencyFindingKind::Unverifiable,
        ];
        assert_eq!(7, kinds.len(), "finding kinds must be stable");

        // Verify serialization round-trip
        for kind in &kinds {
            let json = serde_json::to_string(kind).unwrap();
            let parsed: RepoConsistencyFindingKind = serde_json::from_str(&json).unwrap();
            assert_eq!(*kind, parsed);
        }
    }

    #[test]
    fn repo_consistency_summary_counts_match_findings() {
        let findings = vec![
            RepoConsistencyFinding {
                kind: RepoConsistencyFindingKind::Supported,
                claim_text: None,
                evidence_kind: None,
                repo_evidence_key: vec![],
                severity: ConsistencySeverity::Low,
                detail: String::new(),
            },
            RepoConsistencyFinding {
                kind: RepoConsistencyFindingKind::Supported,
                claim_text: None,
                evidence_kind: None,
                repo_evidence_key: vec![],
                severity: ConsistencySeverity::Low,
                detail: String::new(),
            },
            RepoConsistencyFinding {
                kind: RepoConsistencyFindingKind::MissingInRepo,
                claim_text: None,
                evidence_kind: None,
                repo_evidence_key: vec![],
                severity: ConsistencySeverity::Medium,
                detail: String::new(),
            },
            RepoConsistencyFinding {
                kind: RepoConsistencyFindingKind::Unverifiable,
                claim_text: None,
                evidence_kind: None,
                repo_evidence_key: vec![],
                severity: ConsistencySeverity::Low,
                detail: String::new(),
            },
        ];
        let summary = RepoConsistencySummary::from_findings(&findings);
        assert_eq!(2, summary.supported);
        assert_eq!(1, summary.missing_in_repo);
        assert_eq!(1, summary.unverifiable);
        assert_eq!(4, summary.total(), "total must equal findings count");
    }

    #[test]
    fn empty_repo_check_returns_empty_report() {
        let clock = FixedClock { timestamp: DateTime::UNIX_EPOCH };
        let checker = RepoConsistencyChecker::new(clock);
        let report = checker.empty_report(PathBuf::from("/empty/repo"));
        assert_eq!(0, report.summary.total());
        assert_eq!(0, report.findings.len());
        assert_eq!(DateTime::UNIX_EPOCH, report.checked_at);
    }

    #[test]
    fn normalize_repo_path_strips_dot_slash() {
        assert_eq!("crates/memory/src/lib.rs", normalize_repo_path(std::path::Path::new("./crates/memory/src/lib.rs")));
    }

    #[test]
    fn normalize_repo_path_converts_backslashes() {
        assert_eq!("crates/memory/src/lib.rs", normalize_repo_path(std::path::Path::new("crates\\memory\\src\\lib.rs")));
    }

    #[test]
    fn normalize_repo_path_idempotent() {
        let p = "crates/memory/src/lib.rs";
        assert_eq!(p, normalize_repo_path(std::path::Path::new(p)));
    }
}
