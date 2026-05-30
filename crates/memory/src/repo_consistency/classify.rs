//! Consistency classification with supersession and conflict awareness.
//!
//! Trust-critical: superseded records are not current truth,
//! conflict-grouped records are flagged for review.

use crate::evidence::EvidenceKind;
use crate::retrieval::RankedMemoryHit;

use super::claim_match::{match_claim, parse_claim, RepoClaimPattern};
use super::report::{
    ConsistencySeverity, RepoConsistencyFinding, RepoConsistencyFindingKind,
};

/// Classify a current memory claim against repo observations.
pub fn classify_current_claim(
    hit: &RankedMemoryHit,
    crate_names: &[String],
    src_files: &[String],
    dependencies: &[(String, String)],
) -> RepoConsistencyFinding {
    let pattern = parse_claim(&hit.text);

    match &pattern {
        RepoClaimPattern::Unsupported => RepoConsistencyFinding {
            kind: RepoConsistencyFindingKind::Unverifiable,
            claim_text: Some(hit.text.clone()),
            evidence_kind: Some(hit.evidence_kind),
            repo_evidence_key: vec![],
            severity: ConsistencySeverity::Low,
            detail: "Claim outside v0 grammar".to_string(),
        },
        _ => {
            if match_claim(&pattern, crate_names, src_files, dependencies) {
                RepoConsistencyFinding {
                    kind: RepoConsistencyFindingKind::Supported,
                    claim_text: Some(hit.text.clone()),
                    evidence_kind: Some(hit.evidence_kind),
                    repo_evidence_key: pattern_to_evidence_key(&pattern),
                    severity: ConsistencySeverity::Low,
                    detail: "Memory claim matches repo observation".to_string(),
                }
            } else {
                RepoConsistencyFinding {
                    kind: RepoConsistencyFindingKind::MissingInRepo,
                    claim_text: Some(hit.text.clone()),
                    evidence_kind: Some(hit.evidence_kind),
                    repo_evidence_key: pattern_to_evidence_key(&pattern),
                    severity: ConsistencySeverity::Medium,
                    detail: "Memory claim not found in repo".to_string(),
                }
            }
        }
    }
}

/// Classify a superseded claim — always ignored as current truth.
pub fn classify_superseded_claim(hit: &RankedMemoryHit) -> RepoConsistencyFinding {
    RepoConsistencyFinding {
        kind: RepoConsistencyFindingKind::SupersededMemoryIgnored,
        claim_text: Some(hit.text.clone()),
        evidence_kind: Some(EvidenceKind::SupersededClaim),
        repo_evidence_key: vec![],
        severity: ConsistencySeverity::Low,
        detail: "Superseded record not treated as current truth".to_string(),
    }
}

/// Classify a conflict-grouped claim — always flagged for review.
pub fn classify_conflict_claim(hit: &RankedMemoryHit) -> RepoConsistencyFinding {
    RepoConsistencyFinding {
        kind: RepoConsistencyFindingKind::ConflictRequiresReview,
        claim_text: Some(hit.text.clone()),
        evidence_kind: Some(EvidenceKind::ConflictingClaim),
        repo_evidence_key: vec![],
        severity: ConsistencySeverity::Medium,
        detail: "Conflicting claims require human review".to_string(),
    }
}

fn pattern_to_evidence_key(pattern: &RepoClaimPattern) -> Vec<String> {
    match pattern {
        RepoClaimPattern::CrateExists { crate_name } => {
            vec![format!("crate:{}", crate_name)]
        }
        RepoClaimPattern::WorkspaceContainsCrate { crate_name } => {
            vec![format!("workspace_crate:{}", crate_name)]
        }
        RepoClaimPattern::FileExists { path } => {
            vec![format!("file:{}", path)]
        }
        RepoClaimPattern::ModuleExists { module_path } => {
            vec![format!("module:{}", module_path)]
        }
        RepoClaimPattern::CrateDependsOn {
            crate_name,
            dependency,
        } => {
            vec![format!("dep:{}:{}", crate_name, dependency)]
        }
        RepoClaimPattern::Unsupported => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ranking::MemoryRankScore;

    fn make_hit(text: &str, kind: EvidenceKind) -> RankedMemoryHit {
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
            evidence_kind: kind,
            source_episode_ids: vec![],
            source_trace_ids: vec![],
            scope: crate::provenance::MemoryScope::Global,
            provenance: crate::provenance::ProvenanceSnapshot::default(),
            confidence_bps: 7000,
            reason: "test".to_string(),
        }
    }

    #[test]
    fn superseded_claim_not_reported_as_current_failure() {
        let hit = make_hit("crate old exists", EvidenceKind::SupersededClaim);
        let finding = classify_superseded_claim(&hit);
        assert_eq!(
            RepoConsistencyFindingKind::SupersededMemoryIgnored,
            finding.kind
        );
    }

    #[test]
    fn superseded_claim_reported_as_ignored_history() {
        let hit = make_hit("crate old exists", EvidenceKind::SupersededClaim);
        let finding = classify_superseded_claim(&hit);
        assert_eq!(ConsistencySeverity::Low, finding.severity);
        assert!(finding.detail.contains("Superseded"));
    }

    #[test]
    fn conflict_group_claims_reported_for_review() {
        let hit = make_hit("crate core exists", EvidenceKind::ConflictingClaim);
        let finding = classify_conflict_claim(&hit);
        assert_eq!(
            RepoConsistencyFindingKind::ConflictRequiresReview,
            finding.kind
        );
    }

    #[test]
    fn conflicted_claim_not_promoted_as_supported() {
        let hit = make_hit("crate core exists", EvidenceKind::ConflictingClaim);
        let finding = classify_conflict_claim(&hit);
        // Even if the claim matches repo, conflict takes priority
        assert_ne!(RepoConsistencyFindingKind::Supported, finding.kind);
    }

    #[test]
    fn conflict_group_increments_conflicted_summary_count() {
        let findings = vec![
            classify_conflict_claim(&make_hit("claim A", EvidenceKind::ConflictingClaim)),
            classify_conflict_claim(&make_hit("claim B", EvidenceKind::ConflictingClaim)),
        ];
        let summary = super::super::report::RepoConsistencySummary::from_findings(&findings);
        assert_eq!(2, summary.conflicted);
        assert_eq!(2, summary.total());
    }
}
