//! Provenance tracking for memory retrieval results.
//!
//! Every retrieval hit carries provenance explaining where it came from,
//! how it was validated, and what scope it applies to.

use serde::{Deserialize, Serialize};

/// How a memory record's claim was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum ProvenanceKind {
    /// Explicitly stated by the user.
    UserStated,
    /// Extracted by LLM from conversation or tool output.
    LlmExtracted,
    /// Derived deterministically from trace events.
    SystemDerived,
    /// Provenance unknown (legacy records, migration).
    #[default]
    Unknown,
}


/// Snapshot of provenance information for a retrieval hit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvenanceSnapshot {
    pub kind: ProvenanceKind,
}

impl Default for ProvenanceSnapshot {
    fn default() -> Self {
        Self {
            kind: ProvenanceKind::Unknown,
        }
    }
}

/// Scope of a memory record's applicability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum MemoryScope {
    /// Applies to the current project/repository.
    Project { repo: String, branch: Option<String> },
    /// Applies to the current session only.
    Session { session_id: String },
    /// Applies globally across all contexts.
    Global,
    /// Scope unknown (legacy records).
    #[default]
    Unknown,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_default_is_unknown() {
        let p = ProvenanceSnapshot::default();
        assert_eq!(ProvenanceKind::Unknown, p.kind);
    }

    #[test]
    fn scope_default_is_unknown() {
        let s = MemoryScope::default();
        assert!(matches!(s, MemoryScope::Unknown));
    }
}
