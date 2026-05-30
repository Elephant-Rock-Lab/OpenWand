//! Missing-in-memory detection.
//!
//! Compare repo observations back against current memory claims.
//! Capped to high-level observations: workspace crates, dependencies, docs artifacts.
//! Does NOT emit one finding per arbitrary source file.

use crate::retrieval::RankedMemoryHit;

use super::observe::{ObservedDependency, RepoObservationSnapshot};
use super::report::{
    ConsistencySeverity, RepoConsistencyFinding, RepoConsistencyFindingKind,
};

/// Detect repo observations that have no corresponding current memory claim.
/// Only checks high-level observations (crates, dependencies, docs).
pub fn detect_missing_in_memory(
    snapshot: &RepoObservationSnapshot,
    current_claims: &[RankedMemoryHit],
) -> Vec<RepoConsistencyFinding> {
    let mut findings = Vec::new();

    let claim_texts: Vec<String> = current_claims.iter().map(|h| h.text.to_lowercase()).collect();

    // Check workspace crates
    for observed_crate in &snapshot.crates {
        let crate_name = observed_crate.name.to_lowercase();
        let has_claim = claim_texts.iter().any(|t| {
            t.contains(&crate_name) && (t.contains("crate") || t.contains("workspace"))
        });

        if !has_claim {
            findings.push(RepoConsistencyFinding {
                kind: RepoConsistencyFindingKind::MissingInMemory,
                claim_text: None,
                evidence_kind: None,
                repo_evidence_key: vec![format!("crate:{}", observed_crate.name)],
                severity: ConsistencySeverity::Medium,
                detail: format!(
                    "Repo has crate '{}' but no current memory claim",
                    observed_crate.name
                ),
            });
        }
    }

    // Check dependencies (only root-level and crate-level, not per-file)
    for dep in &snapshot.dependencies {
        let dep_lower = dep.dependency_name.to_lowercase();
        let has_claim = claim_texts.iter().any(|t| {
            t.contains(&dep_lower) && t.contains("depend")
        });

        if !has_claim {
            findings.push(RepoConsistencyFinding {
                kind: RepoConsistencyFindingKind::MissingInMemory,
                claim_text: None,
                evidence_kind: None,
                repo_evidence_key: vec![format!("dep:{}:{}", dep.crate_name, dep.dependency_name)],
                severity: ConsistencySeverity::Medium,
                detail: format!(
                    "Crate '{}' depends on '{}' but no memory claim",
                    dep.crate_name, dep.dependency_name
                ),
            });
        }
    }

    // Check docs artifacts
    for doc_file in &snapshot.docs_files {
        let has_claim = claim_texts.iter().any(|t| {
            let doc_name = doc_file.to_lowercase();
            t.contains(&doc_name)
                || (doc_file.contains("LOCK") && t.contains("lock"))
                || (doc_file.contains("DESIGN") && t.contains("design"))
        });

        if !has_claim {
            findings.push(RepoConsistencyFinding {
                kind: RepoConsistencyFindingKind::MissingInMemory,
                claim_text: None,
                evidence_kind: None,
                repo_evidence_key: vec![format!("doc:{}", doc_file)],
                severity: ConsistencySeverity::Low,
                detail: format!("Repo has docs artifact '{}' but no memory claim", doc_file),
            });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::EvidenceKind;
    use super::super::observe::ObservedCrate;
    use crate::ranking::MemoryRankScore;

    fn make_hit(text: &str) -> RankedMemoryHit {
        RankedMemoryHit {
            id: "test".to_string(),
            text: text.to_string(),
            score: MemoryRankScore {
                relevance_bps: 0,
                provenance_bps: 0,
                scope_bps: 0,
                recency_bps: 0,
                confidence_bps: 0,
                evidence_bps: 0,
                verification_bps: 0,
                final_bps: 0,
            },
            evidence_kind: EvidenceKind::AcceptedClaim,
            source_episode_ids: vec![],
            source_trace_ids: vec![],
            scope: crate::provenance::MemoryScope::Global,
            provenance: crate::provenance::ProvenanceSnapshot::default(),
            confidence_bps: 7000,
            reason: "test".to_string(),
        }
    }

    fn make_snapshot(crates: Vec<(&str, &str)>, deps: Vec<(&str, &str)>, docs: Vec<&str>) -> RepoObservationSnapshot {
        RepoObservationSnapshot {
            repo_root: std::path::PathBuf::from("/test"),
            cargo_package_name: Some("test".to_string()),
            workspace_members: vec![],
            crates: crates
                .into_iter()
                .map(|(name, path)| ObservedCrate {
                    name: name.to_string(),
                    path: path.to_string(),
                    src_files: vec![],
                    test_files: vec![],
                })
                .collect(),
            docs_files: docs.into_iter().map(|s| s.to_string()).collect(),
            dependencies: deps
                .into_iter()
                .map(|(cn, dn)| ObservedDependency {
                    crate_name: cn.to_string(),
                    dependency_name: dn.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn repo_crate_without_memory_claim_reported_missing_in_memory() {
        let snap = make_snapshot(vec![("openwand-core", "crates/core")], vec![], vec![]);
        let findings = detect_missing_in_memory(&snap, &[]);
        assert_eq!(1, findings.len());
        assert_eq!(RepoConsistencyFindingKind::MissingInMemory, findings[0].kind);
        assert!(findings[0].detail.contains("openwand-core"));
    }

    #[test]
    fn repo_dependency_without_memory_claim_reported_missing_in_memory() {
        let snap = make_snapshot(vec![], vec![("core", "serde")], vec![]);
        let findings = detect_missing_in_memory(&snap, &[]);
        assert!(findings.iter().any(|f| f.detail.contains("serde")));
    }

    #[test]
    fn repo_docs_without_memory_claim_reported_missing_in_memory() {
        let snap = make_snapshot(vec![], vec![], vec!["docs/LOCK.md"]);
        let findings = detect_missing_in_memory(&snap, &[]);
        assert!(findings.iter().any(|f| f.detail.contains("LOCK.md")));
    }

    #[test]
    fn missing_in_memory_severity_is_low_or_medium() {
        let snap = make_snapshot(
            vec![("core", "crates/core")],
            vec![("core", "serde")],
            vec!["docs/LOCK.md"],
        );
        let findings = detect_missing_in_memory(&snap, &[]);
        for f in &findings {
            assert!(
                f.severity == ConsistencySeverity::Low || f.severity == ConsistencySeverity::Medium,
                "MissingInMemory severity must be Low or Medium"
            );
        }
    }

    #[test]
    fn missing_in_memory_does_not_mark_memory_stale() {
        let snap = make_snapshot(vec![("core", "crates/core")], vec![], vec![]);
        let findings = detect_missing_in_memory(&snap, &[make_hit("crate memory exists")]);
        // core is missing from memory, but memory is not stale — it just lacks a claim
        for f in &findings {
            assert_ne!(RepoConsistencyFindingKind::StaleMemory, f.kind);
        }
    }
}
