//! Memory UI DTOs.
//!
//! Types for rendering memory records in the UI panel.
//! Inspection only — no editing.

use serde::{Deserialize, Serialize};

/// A memory record for UI display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiMemoryRecord {
    pub record_id: String,
    pub claim: String,
    pub kind: String,         // "fact" | "decision" | "preference"
    pub confidence: f64,
    pub status: String,       // "active" | "superseded" | "rejected"
    pub source_count: usize,
    pub source_trace_ids: Vec<String>,
    pub created_at: i64,
    pub superseded_by: Option<String>,
}

/// Summary of all memory state for the UI panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiMemoryPanel {
    pub total_records: usize,
    pub active_count: usize,
    pub records: Vec<UiMemoryRecord>,
}
