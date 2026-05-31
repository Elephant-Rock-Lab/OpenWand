//! Governed auto-commit proposal.
//!
//! This module produces a reviewable commit proposal from readiness/eval evidence.
//! It NEVER executes git commit, staging, push, tag, or workspace mutation.
//!
//! Core invariant: execution_allowed_now is always false.

use blake3::Hasher;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::eval_compare::EvalComparisonReport;
use crate::eval_model::EvalRunReport;
use crate::eval_readiness::{
    AutoCommitReadinessReport, AutoCommitReadinessStatus, ReadinessBlocker,
};

// ── Proposal ID (content-addressed) ────────────────────────────────────────

/// Content-addressed proposal ID derived from readiness + workspace hash.
/// Not ULID-based. Deterministic for same inputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AutoCommitProposalId(pub String);

/// Compute a deterministic proposal ID from readiness and workspace snapshot.
/// Correction #1: BLAKE3 content hash, not ULID seed.
pub fn proposal_id_for(readiness_id: &str, workspace_hash: &str) -> AutoCommitProposalId {
    let mut hasher = Hasher::new();
    hasher.update(format!("{}:{}", readiness_id, workspace_hash).as_bytes());
    let hash = hasher.finalize();
    AutoCommitProposalId(format!("acp_{}", hash.to_hex()))
}

// ── Proposal status ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AutoCommitProposalStatus {
    Draft,
    Eligible,
    Blocked,
    Superseded,
}

// ── File change ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileChangeKind {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalFileChange {
    pub path: String,
    pub kind: FileChangeKind,
    pub evidence_ref: Option<String>,
}

// ── Workspace snapshot digest ──────────────────────────────────────────────

/// Digest of the workspace state at proposal time.
/// Computed externally and passed in — the proposal builder does NOT
/// read the filesystem itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshotDigest {
    pub blake3_hash: String,
    pub file_count: usize,
    pub generated_at: DateTime<Utc>,
    /// Selected file digests for verification (path → hash).
    pub file_digests: Vec<(String, String)>,
}

// ── Summaries ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionSummary {
    pub name: String,
    pub passed: u32,
    pub total: u32,
    pub pass_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalEvidenceSummary {
    pub readiness_status: AutoCommitReadinessStatus,
    pub total_reports: usize,
    pub pass_rate: f64,
    pub dimension_summaries: Vec<DimensionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalRegressionSummary {
    pub regression_count: usize,
    pub regressed_dimensions: Vec<String>,
    pub compared_scenarios: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalGovernanceSummary {
    /// The readiness decision that produced this proposal.
    pub readiness_decision: AutoCommitReadinessStatus,
    /// What confirmation level would be required for FUTURE execution.
    /// Wave 11 never executes. A later wave defines execution.
    pub confirmation_required_for_future_execution: openwand_core::ConfirmationLevel,
    /// Whether execution is allowed RIGHT NOW.
    /// **Hard invariant: always false in Wave 11.**
    pub execution_allowed_now: bool,
    /// Why execution is or isn't allowed.
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCommitProposalWarning {
    pub detail: String,
}

// ── Full proposal ──────────────────────────────────────────────────────────

/// A governed auto-commit proposal. This is a reviewable artifact, NOT an
/// executable action. No field on this struct represents a git operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCommitProposal {
    pub proposal_id: AutoCommitProposalId,
    pub readiness_report_path: Option<String>,
    pub workspace_snapshot_id: String,

    pub status: AutoCommitProposalStatus,

    pub commit_title: String,
    pub commit_body: String,

    pub included_files: Vec<ProposalFileChange>,
    pub excluded_files: Vec<ProposalFileChange>,

    pub evidence_summary: ProposalEvidenceSummary,
    pub regression_summary: ProposalRegressionSummary,
    pub governance_summary: ProposalGovernanceSummary,

    pub blockers: Vec<ReadinessBlocker>,
    pub warnings: Vec<AutoCommitProposalWarning>,

    pub generated_at: DateTime<Utc>,
}

// ── Proposal builder inputs ────────────────────────────────────────────────

/// Inputs for proposal generation. All are already-observed evidence.
/// The builder does NOT execute git status/diff or read the filesystem.
pub struct AutoCommitProposalInputs<'a> {
    pub readiness: &'a AutoCommitReadinessReport,
    pub workspace_digest: &'a WorkspaceSnapshotDigest,
    pub eval_report: &'a EvalRunReport,
    pub comparison: Option<&'a EvalComparisonReport>,
}

/// Readiness identifier derived from the report's generated_at timestamp
/// and scenario coverage. This is NOT a UUID — it's a stable descriptor.
pub fn readiness_id_for_report(report: &AutoCommitReadinessReport) -> String {
    let mut hasher = Hasher::new();
    hasher.update(report.generated_at.to_rfc3339().as_bytes());
    hasher.update(format!("{}", report.evidence_window.reports_used).as_bytes());
    for s in &report.evidence_window.scenario_ids_covered {
        hasher.update(s.as_bytes());
    }
    format!("acr_{}", hasher.finalize().to_hex())
}

// ── Proposal builder ───────────────────────────────────────────────────────

/// Build an auto-commit proposal from readiness + eval evidence.
///
/// This function is purely computational. It takes borrowed evidence
/// and produces a proposal struct. No I/O, no git, no filesystem access.
pub fn build_auto_commit_proposal(inputs: AutoCommitProposalInputs) -> AutoCommitProposal {
    let readiness = inputs.readiness;
    let workspace_digest = inputs.workspace_digest;
    let eval_report = inputs.eval_report;
    let comparison = inputs.comparison;

    let readiness_id = readiness_id_for_report(readiness);
    let proposal_id = proposal_id_for(&readiness_id, &workspace_digest.blake3_hash);

    // Status mapping
    let status = match readiness.status {
        AutoCommitReadinessStatus::Eligible => AutoCommitProposalStatus::Eligible,
        AutoCommitReadinessStatus::Blocked => AutoCommitProposalStatus::Blocked,
        AutoCommitReadinessStatus::InsufficientEvidence => AutoCommitProposalStatus::Blocked,
    };

    // Evidence summary
    let dimension_summaries: Vec<DimensionSummary> = eval_report.score.dimensions.iter()
        .map(|d| DimensionSummary {
            name: d.name.clone(),
            passed: d.passed,
            total: d.total,
            pass_rate: if d.total > 0 {
                d.passed as f64 / d.total as f64
            } else {
                1.0
            },
        })
        .collect();

    let evidence_summary = ProposalEvidenceSummary {
        readiness_status: readiness.status.clone(),
        total_reports: readiness.evidence_window.total_reports_found,
        pass_rate: readiness.score.weighted_pass_rate,
        dimension_summaries,
    };

    // Regression summary
    let regression_summary = match comparison {
        Some(comp) => ProposalRegressionSummary {
            regression_count: comp.regressions.len(),
            regressed_dimensions: comp.regressions.iter().map(|r| r.dimension.clone()).collect(),
            compared_scenarios: 1,
        },
        None => ProposalRegressionSummary {
            regression_count: readiness.score.regression_count,
            regressed_dimensions: vec![],
            compared_scenarios: readiness.evidence_window.scenario_ids_covered.len(),
        },
    };

    // Governance summary — **always disallows execution in Wave 11**
    let governance_summary = ProposalGovernanceSummary {
        readiness_decision: readiness.status.clone(),
        confirmation_required_for_future_execution: openwand_core::ConfirmationLevel::Escalate,
        execution_allowed_now: false, // HARD INVARIANT
        reason: "Wave 11 produces proposal artifact only. Execution is not implemented.".to_string(),
    };

    // Commit message synthesis (deterministic template)
    let commit_title = synthesize_commit_title(eval_report, &status);
    let commit_body = synthesize_commit_body(
        &evidence_summary,
        &regression_summary,
        &governance_summary,
        &readiness.blockers,
        &readiness.warnings.iter().map(|w| w.detail.clone()).collect::<Vec<_>>(),
    );

    // File changes — derived from patch evidence in the eval report
    let included_files = vec![]; // Populated from patch evidence when available
    let excluded_files = vec![];

    // Warnings
    let warnings: Vec<AutoCommitProposalWarning> = readiness.warnings.iter()
        .map(|w| AutoCommitProposalWarning { detail: w.detail.clone() })
        .collect();

    AutoCommitProposal {
        proposal_id,
        readiness_report_path: None,
        workspace_snapshot_id: workspace_digest.blake3_hash.clone(),
        status,
        commit_title,
        commit_body,
        included_files,
        excluded_files,
        evidence_summary,
        regression_summary,
        governance_summary,
        blockers: readiness.blockers.clone(),
        warnings,
        generated_at: Utc::now(),
    }
}

// ── Commit message synthesis (deterministic template) ──────────────────────

/// List of forbidden strings that must NEVER appear in commit body.
/// Optional strengthening: guard against wording drift.
pub const FORBIDDEN_COMPLETION_PHRASES: &[&str] = &[
    "committed",
    "created commit",
    "commit completed",
    "changes were committed",
    "git commit was run",
];

fn synthesize_commit_title(eval_report: &EvalRunReport, status: &AutoCommitProposalStatus) -> String {
    match status {
        AutoCommitProposalStatus::Eligible => {
            format!("eval: {} passed all dimensions", eval_report.scenario_id)
        }
        AutoCommitProposalStatus::Blocked => {
            format!("eval: {} blocked — proposal only", eval_report.scenario_id)
        }
        AutoCommitProposalStatus::Draft => {
            format!("eval: {} draft proposal", eval_report.scenario_id)
        }
        AutoCommitProposalStatus::Superseded => {
            format!("eval: {} superseded proposal", eval_report.scenario_id)
        }
    }
}

fn synthesize_commit_body(
    evidence: &ProposalEvidenceSummary,
    regression: &ProposalRegressionSummary,
    governance: &ProposalGovernanceSummary,
    blockers: &[ReadinessBlocker],
    warnings: &[String],
) -> String {
    let mut body = String::new();

    // Summary
    body.push_str("Summary:\n");
    body.push_str(&format!("- Reports analyzed: {}\n", evidence.total_reports));
    body.push_str(&format!("- Weighted pass rate: {:.2}\n", evidence.pass_rate));
    for dim in &evidence.dimension_summaries {
        body.push_str(&format!("- {}: {}/{} ({:.0}%)\n", dim.name, dim.passed, dim.total, dim.pass_rate * 100.0));
    }

    // Validation
    body.push_str("\nValidation:\n");
    body.push_str(&format!("- Regressions: {}\n", regression.regression_count));
    if !regression.regressed_dimensions.is_empty() {
        body.push_str(&format!("- Regressed dimensions: {}\n", regression.regressed_dimensions.join(", ")));
    }

    // Governance
    body.push_str("\nGovernance:\n");
    body.push_str(&format!("- Readiness: {:?}\n", governance.readiness_decision));
    body.push_str(&format!("- Execution allowed: {}\n", governance.execution_allowed_now));

    if !blockers.is_empty() {
        body.push_str("- Blockers:\n");
        for b in blockers {
            body.push_str(&format!("  - {} ({})\n", b.detail, b.scenario_id.as_deref().unwrap_or("global")));
        }
    } else {
        body.push_str("- Blockers: none\n");
    }

    if !warnings.is_empty() {
        body.push_str("- Warnings:\n");
        for w in warnings {
            body.push_str(&format!("  - {}\n", w));
        }
    }

    // Mandatory footer
    body.push_str("\nProposal:\nThis is a generated commit proposal only. No git commit was executed.\n");

    body
}

// ── Persistence ────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

/// Save a proposal to disk.
/// Correction #2: Supersession happens HERE (in save), not in load.
pub fn save_proposal(
    store_root: &Path,
    proposal: &AutoCommitProposal,
) -> Result<PathBuf, String> {
    let proposals_dir = store_root.join("proposals");
    std::fs::create_dir_all(&proposals_dir)
        .map_err(|e| format!("Failed to create proposals dir: {}", e))?;

    // Supersession: mark existing proposals for same readiness as Superseded
    // if workspace hash differs. (Correction #2: mutation in save, not load.)
    if let Ok(existing) = list_proposals(store_root) {
        for mut existing_proposal in existing {
            if existing_proposal.workspace_snapshot_id != proposal.workspace_snapshot_id
                && existing_proposal.proposal_id != proposal.proposal_id
                && existing_proposal.status != AutoCommitProposalStatus::Superseded
            {
                existing_proposal.status = AutoCommitProposalStatus::Superseded;
                let existing_path = proposals_dir.join(format!("{}.json", existing_proposal.proposal_id.0));
                let json = serde_json::to_string_pretty(&existing_proposal)
                    .map_err(|e| format!("Failed to serialize superseded proposal: {}", e))?;
                std::fs::write(&existing_path, json)
                    .map_err(|e| format!("Failed to write superseded proposal: {}", e))?;
            }
        }
    }

    // Save the new proposal
    let path = proposals_dir.join(format!("{}.json", proposal.proposal_id.0));
    let json = serde_json::to_string_pretty(proposal)
        .map_err(|e| format!("Failed to serialize proposal: {}", e))?;
    std::fs::write(&path, &json)
        .map_err(|e| format!("Failed to write proposal: {}", e))?;

    // Write latest pointer
    let latest_path = proposals_dir.join("latest.json");
    std::fs::write(&latest_path, &json)
        .map_err(|e| format!("Failed to write latest proposal pointer: {}", e))?;

    Ok(path)
}

/// Load a proposal by ID. Read-only. (Correction #2)
pub fn load_proposal(
    store_root: &Path,
    id: &AutoCommitProposalId,
) -> Result<Option<AutoCommitProposal>, String> {
    let path = store_root.join("proposals").join(format!("{}.json", id.0));
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read proposal: {}", e))?;
    let proposal: AutoCommitProposal = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse proposal: {}", e))?;
    Ok(Some(proposal))
}

/// Load the latest proposal. Read-only. (Correction #2)
pub fn load_latest_proposal(
    store_root: &Path,
) -> Result<Option<AutoCommitProposal>, String> {
    let latest_path = store_root.join("proposals").join("latest.json");
    if !latest_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&latest_path)
        .map_err(|e| format!("Failed to read latest proposal: {}", e))?;
    let proposal: AutoCommitProposal = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse latest proposal: {}", e))?;
    Ok(Some(proposal))
}

/// List all proposals, ordered by generated_at descending. Read-only.
pub fn list_proposals(store_root: &Path) -> Result<Vec<AutoCommitProposal>, String> {
    let proposals_dir = store_root.join("proposals");
    if !proposals_dir.exists() {
        return Ok(vec![]);
    }

    let mut proposals = Vec::new();
    let entries = std::fs::read_dir(&proposals_dir)
        .map_err(|e| format!("Failed to read proposals dir: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && path.file_stem().and_then(|s| s.to_str()) != Some("latest")
        {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read proposal {}: {}", path.display(), e))?;
            let proposal: AutoCommitProposal = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse proposal {}: {}", path.display(), e))?;
            proposals.push(proposal);
        }
    }

    proposals.sort_by(|a, b| b.generated_at.cmp(&a.generated_at));
    Ok(proposals)
}

// ── Wording guard ──────────────────────────────────────────────────────────

/// Check if a commit body contains forbidden completion phrases.
/// Returns list of violations found.
pub fn check_forbidden_phrases(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    FORBIDDEN_COMPLETION_PHRASES
        .iter()
        .filter(|phrase| lower.contains(phrase.to_lowercase().as_str()))
        .copied()
        .collect()
}
