//! Central memory governance profile — deterministic ranking and inclusion policy.
//!
//! All governance knobs in one place. No scattered constants across ranking,
//! prompt assembly, repo consistency, and evaluation.
//!
//! Default matches pre-02r behavior exactly. batch_02r_default() introduces
//! measured behavioral changes.

use serde::{Deserialize, Serialize};

use crate::ranking::RankingWeights;

// ── Profile ID and registry ─────────────────────────────────────────────────

/// Stable identifier for a named governance profile.
/// Production code references profiles by ID — no hand-assembled weights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryGovernanceProfileId {
    /// Pre-02r: no governance filtering, all claims eligible regardless of confidence.
    Default,
    /// 02r tuned: confidence minimum (3000 bps), verification boost, stale exclusion.
    Batch02rDefault,
}

impl MemoryGovernanceProfileId {
    /// Resolve this ID to its concrete profile.
    /// Deterministic: same ID always produces same profile.
    pub fn resolve(&self) -> MemoryGovernanceProfile {
        match self {
            Self::Default => MemoryGovernanceProfile::default(),
            Self::Batch02rDefault => MemoryGovernanceProfile::batch_02r_default(),
        }
    }

    /// Parse from string. Returns None for unknown IDs.
    /// Used in config parsing. Unknown profile fails closed.
    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "Default" | "default" => Some(Self::Default),
            "Batch02rDefault" | "batch_02r_default" | "batch-02r-default" => Some(Self::Batch02rDefault),
            _ => None,
        }
    }

    /// All known profile IDs.
    pub fn all() -> &'static [Self] {
        &[Self::Default, Self::Batch02rDefault]
    }
}

impl std::fmt::Display for MemoryGovernanceProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => write!(f, "Default"),
            Self::Batch02rDefault => write!(f, "Batch02rDefault"),
        }
    }
}

// ── Governance profile ──────────────────────────────────────────────────────

/// Central governance profile for memory ranking, confidence, and inclusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGovernanceProfile {
    pub ranking_weights: RankingWeights,
    pub confidence_policy: ConfidencePolicy,
    pub verification_policy: VerificationPolicy,
    pub stale_policy: StalePolicy,
    pub supersession_policy: SupersessionPolicy,
    pub conflict_policy: ConflictPolicy,
}

impl Default for MemoryGovernanceProfile {
    /// Matches pre-02r behavior exactly: no confidence minimum, no verification
    /// boost, no stale/conFLICT governance filtering.
    fn default() -> Self {
        Self {
            ranking_weights: RankingWeights::default(),
            confidence_policy: ConfidencePolicy::default(),
            verification_policy: VerificationPolicy::default(),
            stale_policy: StalePolicy::default(),
            supersession_policy: SupersessionPolicy::default(),
            conflict_policy: ConflictPolicy::default(),
        }
    }
}

impl MemoryGovernanceProfile {
    /// Tuned profile for 02r batch. Changes from Default:
    /// - Confidence minimum for prompt inclusion
    /// - Verification boost for Verifies relations
    /// - Stale claims excluded from prompt
    /// - Conflict resolution governance
    pub fn batch_02r_default() -> Self {
        Self {
            ranking_weights: RankingWeights {
                relevance: 3000,
                provenance: 2000,
                scope: 1500,
                recency: 1000,
                confidence: 1000,
                evidence: 500,
                verification: 1000,
            },
            confidence_policy: ConfidencePolicy {
                high_min_bps: 8500,
                medium_min_bps: 5000,
                low_min_bps: 2000,
                // Below this, claim needs verification to be prompt-eligible
                prompt_include_min_bps: 3000,
                require_verification_below_bps: 5000,
            },
            verification_policy: VerificationPolicy {
                verifies_boost_bps: 2000,
                derived_from_boost_bps: 500,
                refines_boost_bps: 800,
            },
            stale_policy: StalePolicy {
                exclude_from_prompt: true,
                report_stale: true,
            },
            supersession_policy: SupersessionPolicy {
                exclude_superseded_from_prompt: true,
                keep_superseded_panel_visible: true,
            },
            conflict_policy: ConflictPolicy {
                exclude_unresolved_from_prompt: true,
                verified_winner_can_be_included: true,
            },
        }
    }
}

// ── Confidence policy ──────────────────────────────────────────────────────

/// Confidence band classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceBand {
    High,
    Medium,
    Low,
    Untrusted,
}

/// Policy for confidence-based governance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidencePolicy {
    /// Minimum confidence for High band (in basis points).
    pub high_min_bps: u16,
    /// Minimum confidence for Medium band.
    pub medium_min_bps: u16,
    /// Minimum confidence for Low band.
    pub low_min_bps: u16,
    /// Minimum confidence for prompt inclusion. Below this → audit-only.
    pub prompt_include_min_bps: u16,
    /// Below this confidence, claim needs verification to be prompt-eligible.
    pub require_verification_below_bps: u16,
}

impl Default for ConfidencePolicy {
    fn default() -> Self {
        // Pre-02r: no confidence filtering, all claims eligible
        Self {
            high_min_bps: 8500,
            medium_min_bps: 5000,
            low_min_bps: 2000,
            prompt_include_min_bps: 0, // no minimum
            require_verification_below_bps: 0, // no verification requirement
        }
    }
}

impl ConfidencePolicy {
    /// Classify a confidence value (in basis points) into a band.
    /// Deterministic: same input always produces same band.
    pub fn classify_band(&self, confidence_bps: u16) -> ConfidenceBand {
        if confidence_bps >= self.high_min_bps {
            ConfidenceBand::High
        } else if confidence_bps >= self.medium_min_bps {
            ConfidenceBand::Medium
        } else if confidence_bps >= self.low_min_bps {
            ConfidenceBand::Low
        } else {
            ConfidenceBand::Untrusted
        }
    }

    /// Whether a claim at this confidence level is prompt-eligible
    /// (without verification).
    pub fn is_prompt_eligible_by_confidence(&self, confidence_bps: u16) -> bool {
        confidence_bps >= self.prompt_include_min_bps
    }
}

// ── Verification policy ────────────────────────────────────────────────────

/// Policy for verification-based ranking and eligibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct VerificationPolicy {
    /// Ranking boost for Verifies relation (basis points added to verification_bps).
    pub verifies_boost_bps: u16,
    /// Smaller boost for DerivedFrom relation.
    pub derived_from_boost_bps: u16,
    /// Boost for Refines relation.
    pub refines_boost_bps: u16,
}


// ── Stale policy ───────────────────────────────────────────────────────────

/// Policy for stale (repo-reality mismatch) claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalePolicy {
    /// Whether to exclude stale claims from normal prompt context.
    pub exclude_from_prompt: bool,
    /// Whether to report stale claims in audit/panel.
    pub report_stale: bool,
}

impl Default for StalePolicy {
    fn default() -> Self {
        // Pre-02r: stale claims currently go into supported_claims (no special treatment)
        Self {
            exclude_from_prompt: false,
            report_stale: true,
        }
    }
}

// ── Supersession policy ────────────────────────────────────────────────────

/// Policy for superseded claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupersessionPolicy {
    /// Whether to exclude superseded claims from prompt.
    pub exclude_superseded_from_prompt: bool,
    /// Whether to keep superseded claims visible in panel/audit.
    pub keep_superseded_panel_visible: bool,
}

impl Default for SupersessionPolicy {
    fn default() -> Self {
        // Pre-02r: superseded already excluded from prompt (hardcoded in assembler)
        Self {
            exclude_superseded_from_prompt: true,
            keep_superseded_panel_visible: true,
        }
    }
}

// ── Conflict policy ────────────────────────────────────────────────────────

/// Policy for unresolved conflicting claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictPolicy {
    /// Whether to exclude unresolved conflicts from prompt.
    pub exclude_unresolved_from_prompt: bool,
    /// Whether a verified claim can be included when in conflict with an unverified one.
    pub verified_winner_can_be_included: bool,
}

impl Default for ConflictPolicy {
    fn default() -> Self {
        // Pre-02r: conflict already excluded from prompt (hardcoded in assembler)
        Self {
            exclude_unresolved_from_prompt: true,
            verified_winner_can_be_included: false,
        }
    }
}

// ── Prompt eligibility ─────────────────────────────────────────────────────

/// Whether a finding is eligible for normal prompt inclusion.
/// Separate from trust bucket — bucket is "what is this claim?",
/// eligibility is "should the model see it now?"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptEligibility {
    Include,
    ExcludeAuditOnly { reason: String },
}

// ── Governed finding ───────────────────────────────────────────────────────

/// A repo consistency finding with governance overlay.
/// Preserves original finding + bucket, adds eligibility and governance reasons.
#[derive(Debug, Clone)]
pub struct GovernedMemoryFinding {
    pub finding: crate::repo_consistency::RepoConsistencyFinding,
    pub bucket: crate::provenance_hydration::MemoryTrustBucket,
    pub prompt_eligibility: PromptEligibility,
    pub governance_reasons: Vec<String>,
}

/// Governed view over a RepoConsistencyReport.
/// Does not mutate the original report — it's a separate selection layer.
#[derive(Debug, Clone)]
pub struct GovernanceFilteredReport {
    /// Original report, preserved as classification truth.
    pub original_report: crate::repo_consistency::RepoConsistencyReport,
    /// All findings with governance overlay.
    pub governed_findings: Vec<GovernedMemoryFinding>,
    /// Findings eligible for prompt inclusion.
    pub included_claims: Vec<GovernedMemoryFinding>,
    /// Findings excluded from prompt (audit/panel only).
    pub audit_only_claims: Vec<GovernedMemoryFinding>,
}

impl GovernanceFilteredReport {
    /// Apply governance profile to a report and ranked hits.
    /// Pure function — no I/O, no store queries.
    pub fn from_report(
        report: &crate::repo_consistency::RepoConsistencyReport,
        profile: &MemoryGovernanceProfile,
        hits: &[crate::retrieval::RankedMemoryHit],
    ) -> Self {
        use crate::provenance_hydration::MemoryTrustBucket;
        use crate::repo_consistency::RepoConsistencyFindingKind;

        let mut governed = Vec::new();
        let mut included = Vec::new();
        let mut audit_only = Vec::new();

        for finding in &report.findings {
            let bucket = MemoryTrustBucket::from_finding_kind(&finding.kind);
            let mut reasons = Vec::new();

            let eligibility = match finding.kind {
                RepoConsistencyFindingKind::Supported => {
                    let confidence = find_confidence_for_claim(&finding.claim_text, hits);
                    if !profile.confidence_policy.is_prompt_eligible_by_confidence(confidence) {
                        let band = profile.confidence_policy.classify_band(confidence);
                        reasons.push(format!(
                            "Excluded: confidence {} bps ({:?}) below prompt_include_min_bps {}",
                            confidence, band, profile.confidence_policy.prompt_include_min_bps
                        ));
                        PromptEligibility::ExcludeAuditOnly {
                            reason: "low_confidence".to_string(),
                        }
                    } else {
                        PromptEligibility::Include
                    }
                }
                RepoConsistencyFindingKind::StaleMemory => {
                    if profile.stale_policy.exclude_from_prompt {
                        reasons.push("Excluded: stale claim per policy".to_string());
                        PromptEligibility::ExcludeAuditOnly {
                            reason: "stale_policy".to_string(),
                        }
                    } else {
                        PromptEligibility::Include
                    }
                }
                RepoConsistencyFindingKind::MissingInRepo => {
                    // MissingInRepo: include as caution (current behavior)
                    PromptEligibility::Include
                }
                RepoConsistencyFindingKind::SupersededMemoryIgnored => {
                    if profile.supersession_policy.exclude_superseded_from_prompt {
                        reasons.push("Excluded: superseded claim per policy".to_string());
                        PromptEligibility::ExcludeAuditOnly {
                            reason: "superseded".to_string(),
                        }
                    } else {
                        PromptEligibility::Include
                    }
                }
                RepoConsistencyFindingKind::ConflictRequiresReview => {
                    if profile.conflict_policy.exclude_unresolved_from_prompt {
                        reasons.push("Excluded: unresolved conflict per policy".to_string());
                        PromptEligibility::ExcludeAuditOnly {
                            reason: "conflict_policy".to_string(),
                        }
                    } else {
                        PromptEligibility::Include
                    }
                }
                RepoConsistencyFindingKind::Unverifiable => {
                    reasons.push("Excluded: unverifiable claim".to_string());
                    PromptEligibility::ExcludeAuditOnly {
                        reason: "unverifiable".to_string(),
                    }
                }
                RepoConsistencyFindingKind::MissingInMemory => {
                    // MissingInMemory: include as gap (not a claim)
                    PromptEligibility::Include
                }
            };

            let gf = GovernedMemoryFinding {
                finding: finding.clone(),
                bucket,
                prompt_eligibility: eligibility.clone(),
                governance_reasons: reasons,
            };

            match &eligibility {
                PromptEligibility::Include => included.push(gf.clone()),
                PromptEligibility::ExcludeAuditOnly { .. } => audit_only.push(gf.clone()),
            }
            governed.push(gf);
        }

        Self {
            original_report: report.clone(),
            governed_findings: governed,
            included_claims: included,
            audit_only_claims: audit_only,
        }
    }
}

/// Find confidence_bps for a claim by matching against ranked hits.
fn find_confidence_for_claim(claim_text: &Option<String>, hits: &[crate::retrieval::RankedMemoryHit]) -> u16 {
    let text = match claim_text {
        Some(t) => t,
        None => return 10000, // no claim text = assume eligible
    };
    hits.iter()
        .find(|h| h.text == *text)
        .map(|h| h.confidence_bps)
        .unwrap_or(10000) // no matching hit = assume eligible
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_governance_profile_is_stable() {
        let p1 = MemoryGovernanceProfile::default();
        let p2 = MemoryGovernanceProfile::default();
        assert_eq!(p1.confidence_policy.prompt_include_min_bps, p2.confidence_policy.prompt_include_min_bps);
        assert_eq!(p1.verification_policy.verifies_boost_bps, 0);
    }

    #[test]
    fn batch_02r_default_differs_from_default() {
        let def = MemoryGovernanceProfile::default();
        let tuned = MemoryGovernanceProfile::batch_02r_default();
        assert_ne!(
            def.confidence_policy.prompt_include_min_bps,
            tuned.confidence_policy.prompt_include_min_bps
        );
        assert_ne!(
            def.verification_policy.verifies_boost_bps,
            tuned.verification_policy.verifies_boost_bps
        );
        assert!(tuned.stale_policy.exclude_from_prompt);
    }

    #[test]
    fn confidence_band_classification_is_deterministic() {
        let policy = ConfidencePolicy::default();
        assert_eq!(ConfidenceBand::High, policy.classify_band(9000));
        assert_eq!(ConfidenceBand::High, policy.classify_band(8500));
        assert_eq!(ConfidenceBand::Medium, policy.classify_band(7000));
        assert_eq!(ConfidenceBand::Low, policy.classify_band(3000));
        assert_eq!(ConfidenceBand::Untrusted, policy.classify_band(1000));
    }

    #[test]
    fn governance_profile_serializes_stably() {
        let profile = MemoryGovernanceProfile::default();
        let json = serde_json::to_string(&profile).unwrap();
        let restored: MemoryGovernanceProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(
            profile.confidence_policy.prompt_include_min_bps,
            restored.confidence_policy.prompt_include_min_bps
        );
    }

    #[test]
    fn ranking_weights_include_verification() {
        let tuned = MemoryGovernanceProfile::batch_02r_default();
        assert_eq!(10000, tuned.ranking_weights.sum());
    }

    #[test]
    fn default_profile_no_confidence_filtering() {
        let profile = MemoryGovernanceProfile::default();
        assert_eq!(0, profile.confidence_policy.prompt_include_min_bps);
        assert!(profile.confidence_policy.is_prompt_eligible_by_confidence(0));
        assert!(profile.confidence_policy.is_prompt_eligible_by_confidence(100));
    }

    #[test]
    fn tuned_profile_filters_low_confidence() {
        let profile = MemoryGovernanceProfile::batch_02r_default();
        assert!(!profile.confidence_policy.is_prompt_eligible_by_confidence(1000));
        assert!(!profile.confidence_policy.is_prompt_eligible_by_confidence(2000));
        assert!(profile.confidence_policy.is_prompt_eligible_by_confidence(5000));
    }

    #[test]
    fn prompt_eligibility_roundtrips() {
        let inc = PromptEligibility::Include;
        let exc = PromptEligibility::ExcludeAuditOnly { reason: "test".into() };
        let json_inc = serde_json::to_string(&inc).unwrap();
        let json_exc = serde_json::to_string(&exc).unwrap();
        let restored_inc: PromptEligibility = serde_json::from_str(&json_inc).unwrap();
        let restored_exc: PromptEligibility = serde_json::from_str(&json_exc).unwrap();
        assert_eq!(inc, restored_inc);
        assert_eq!(exc, restored_exc);
    }

    // ── Profile ID registry tests ─────────────────────────────────────────

    #[test]
    fn registry_resolves_default_to_default_profile() {
        let id = MemoryGovernanceProfileId::Default;
        let profile = id.resolve();
        assert_eq!(0, profile.confidence_policy.prompt_include_min_bps);
        assert_eq!(0, profile.verification_policy.verifies_boost_bps);
    }

    #[test]
    fn registry_resolves_batch_02r_to_tuned_profile() {
        let id = MemoryGovernanceProfileId::Batch02rDefault;
        let profile = id.resolve();
        assert_eq!(3000, profile.confidence_policy.prompt_include_min_bps);
        assert_eq!(2000, profile.verification_policy.verifies_boost_bps);
        assert!(profile.stale_policy.exclude_from_prompt);
    }

    #[test]
    fn registry_rejects_unknown_id() {
        assert!(MemoryGovernanceProfileId::from_str_lossy("nonexistent").is_none());
        assert!(MemoryGovernanceProfileId::from_str_lossy("batch_03").is_none());
        assert!(MemoryGovernanceProfileId::from_str_lossy("").is_none());
    }

    #[test]
    fn registry_accepts_known_ids() {
        assert_eq!(Some(MemoryGovernanceProfileId::Default), MemoryGovernanceProfileId::from_str_lossy("Default"));
        assert_eq!(Some(MemoryGovernanceProfileId::Default), MemoryGovernanceProfileId::from_str_lossy("default"));
        assert_eq!(Some(MemoryGovernanceProfileId::Batch02rDefault), MemoryGovernanceProfileId::from_str_lossy("Batch02rDefault"));
        assert_eq!(Some(MemoryGovernanceProfileId::Batch02rDefault), MemoryGovernanceProfileId::from_str_lossy("batch_02r_default"));
        assert_eq!(Some(MemoryGovernanceProfileId::Batch02rDefault), MemoryGovernanceProfileId::from_str_lossy("batch-02r-default"));
    }

    #[test]
    fn batch_02r_default_values_match_lock_doc() {
        let profile = MemoryGovernanceProfileId::Batch02rDefault.resolve();
        // Ranking weights
        assert_eq!(3000, profile.ranking_weights.relevance);
        assert_eq!(2000, profile.ranking_weights.provenance);
        assert_eq!(1500, profile.ranking_weights.scope);
        assert_eq!(1000, profile.ranking_weights.recency);
        assert_eq!(1000, profile.ranking_weights.confidence);
        assert_eq!(500, profile.ranking_weights.evidence);
        assert_eq!(1000, profile.ranking_weights.verification);
        assert_eq!(10000, profile.ranking_weights.sum());
        // Confidence policy
        assert_eq!(3000, profile.confidence_policy.prompt_include_min_bps);
        assert_eq!(5000, profile.confidence_policy.require_verification_below_bps);
        // Verification policy
        assert_eq!(2000, profile.verification_policy.verifies_boost_bps);
        assert_eq!(500, profile.verification_policy.derived_from_boost_bps);
        assert_eq!(800, profile.verification_policy.refines_boost_bps);
        // Stale/conflict/supersession
        assert!(profile.stale_policy.exclude_from_prompt);
        assert!(profile.supersession_policy.exclude_superseded_from_prompt);
        assert!(profile.conflict_policy.exclude_unresolved_from_prompt);
    }

    #[test]
    fn profile_id_serializes_stably() {
        let id = MemoryGovernanceProfileId::Batch02rDefault;
        let json = serde_json::to_string(&id).unwrap();
        let restored: MemoryGovernanceProfileId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, restored);
    }

    #[test]
    fn profile_id_display_matches_enum_name() {
        assert_eq!("Default", format!("{}", MemoryGovernanceProfileId::Default));
        assert_eq!("Batch02rDefault", format!("{}", MemoryGovernanceProfileId::Batch02rDefault));
    }
}
