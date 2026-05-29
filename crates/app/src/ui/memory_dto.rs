//! Memory UI DTOs — filtered by repo-consistency.
//!
//! Replaces the flat UiMemoryPanel with a governed, bucket-based view.
//! Each row shows a memory claim classified by repo consistency (02j)
//! and tagged with prompt inclusion provenance (02k).

use serde::{Deserialize, Serialize};

/// Summary counts for the panel header.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiMemoryPanelSummary {
    pub prompt_included: usize,
    pub stale: usize,
    pub missing_in_repo: usize,
    pub missing_in_memory: usize,
    pub conflicts: usize,
    pub unverifiable: usize,
    pub superseded_ignored: usize,
}

impl UiMemoryPanelSummary {
    pub fn total(&self) -> usize {
        self.prompt_included
            + self.stale
            + self.missing_in_repo
            + self.missing_in_memory
            + self.conflicts
            + self.unverifiable
            + self.superseded_ignored
    }
}

/// A single memory claim row in the panel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiMemoryPanelRow {
    pub claim: String,
    pub finding_kind: String,
    pub evidence_kind: String,
    pub repo_evidence_key: Vec<String>,
    pub inclusion_reason: Option<String>,
    pub severity: String,
}

/// A conflict group requiring review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiMemoryPanelConflict {
    pub group_id: String,
    pub claims: Vec<UiMemoryPanelRow>,
    pub detail: String,
}

/// The full panel view — bucket-based, repo-filtered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiFilteredMemoryPanel {
    pub working_directory: String,
    pub generated_at: i64,
    pub summary: UiMemoryPanelSummary,
    pub prompt_included: Vec<UiMemoryPanelRow>,
    pub stale: Vec<UiMemoryPanelRow>,
    pub missing_in_repo: Vec<UiMemoryPanelRow>,
    pub missing_in_memory: Vec<UiMemoryPanelRow>,
    pub conflicts: Vec<UiMemoryPanelConflict>,
    pub unverifiable: Vec<UiMemoryPanelRow>,
    pub superseded_ignored: Vec<UiMemoryPanelRow>,
}

impl UiFilteredMemoryPanel {
    pub fn empty() -> Self {
        Self {
            working_directory: String::new(),
            generated_at: 0,
            summary: UiMemoryPanelSummary {
                prompt_included: 0,
                stale: 0,
                missing_in_repo: 0,
                missing_in_memory: 0,
                conflicts: 0,
                unverifiable: 0,
                superseded_ignored: 0,
            },
            prompt_included: vec![],
            stale: vec![],
            missing_in_repo: vec![],
            missing_in_memory: vec![],
            conflicts: vec![],
            unverifiable: vec![],
            superseded_ignored: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.summary.total() == 0
    }
}
