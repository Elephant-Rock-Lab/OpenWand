//! Session registry store trait.
//!
//! Defines the contract for session metadata persistence.
//! Implementations store navigation metadata; authority lives in trace_entry.

use crate::registry::{NewSessionRecord, SessionListFilter, SessionRecord, SessionRegistryUpdate, SessionSummary};
use crate::error::StoreError;

/// Store trait for session registry operations.
///
/// Implementations must be Send + Sync and safe for concurrent access.
/// The registry is a cache — data loss here is recoverable from trace.
pub trait SessionRegistryStore: Send + Sync {
    /// Create a new session record.
    fn create_session(&self, record: NewSessionRecord) -> Result<SessionRecord, StoreError>;

    /// Get a session by ID. Returns None if not found.
    fn get_session(&self, session_id: &str) -> Result<Option<SessionRecord>, StoreError>;

    /// List sessions ordered by updated_at descending.
    fn list_sessions(&self, filter: SessionListFilter) -> Result<Vec<SessionSummary>, StoreError>;

    /// Update session metadata (partial update — None fields are skipped).
    fn update_session(&self, update: SessionRegistryUpdate) -> Result<(), StoreError>;

    /// Archive a session (sets status to 'archived').
    fn archive_session(&self, session_id: &str) -> Result<(), StoreError>;
}
