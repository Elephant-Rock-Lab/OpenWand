//! Session registry — navigation metadata for UI session lists.
//!
//! Invariant: session_registry is a cache/index for app navigation.
//! trace_entry remains authority. Loro remains projection.
//! Registry may be stale; it is always rebuildable from trace.

use serde::{Deserialize, Serialize};

/// A new session to be created in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSessionRecord {
    pub session_id: String,
    pub title: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub working_directory: Option<String>,
    pub interaction_mode: String,
}

/// A full session record from the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub title: Option<String>,
    pub status: String,

    pub created_at: i64,
    pub updated_at: i64,
    pub last_opened_at: Option<i64>,

    pub provider: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub working_directory: Option<String>,

    pub interaction_mode: String,
    pub current_phase: Option<String>,
    pub current_step: i64,

    pub last_message_preview: Option<String>,
    pub last_trace_id: Option<String>,
    pub last_global_sequence: Option<i64>,

    pub snapshot_key: Option<String>,
    pub projection_stale: bool,

    pub metadata_json: Option<String>,
}

/// Summary for session list rendering (lighter weight).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub title: Option<String>,
    pub status: String,
    pub model: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_message_preview: Option<String>,
    pub current_phase: Option<String>,
}

/// Filter for listing sessions.
#[derive(Debug, Clone, Default)]
pub struct SessionListFilter {
    pub include_archived: bool,
    pub limit: Option<u32>,
}

/// Update to apply to a session registry record.
#[derive(Debug, Clone, Default)]
pub struct SessionRegistryUpdate {
    pub session_id: String,
    pub title: Option<String>,
    pub status: Option<String>,
    pub current_phase: Option<String>,
    pub current_step: Option<i64>,
    pub last_message_preview: Option<String>,
    pub last_trace_id: Option<String>,
    pub last_global_sequence: Option<i64>,
    pub snapshot_key: Option<String>,
    pub projection_stale: Option<bool>,
    pub metadata_json: Option<String>,
}
