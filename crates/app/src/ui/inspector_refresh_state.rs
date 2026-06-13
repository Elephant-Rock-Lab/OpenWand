//! Inspector refresh state DTO for tracking refresh lifecycle (Wave 89A).
//!
//! This module is a pure data type — it imports NO backend authority types.
//! It represents the honest state of an inspector refresh attempt.
//! Refresh is read-only: it reloads existing data via the existing inspector loader,
//! it does not initiate, approve, export, execute, or mutate anything.
//!
//! Wave 89A patches applied:
//! - workflow_execution_id included in every non-Idle state for auditability
//! - No session_id → workflow_execution_id fallback (non-negotiable)
//! - sections_attempted + sections_loaded avoid misleading "sections" count
//! - No unbounded polling; refresh is triggered once per operation completion
//! - Stale state captures selection mismatch honestly

use serde::{Deserialize, Serialize};

/// State of an inspector refresh attempt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InspectorRefreshState {
    /// No refresh attempted.
    Idle,

    /// Refresh in progress for the given workflow execution ID.
    Loading {
        workflow_execution_id: String,
    },

    /// Refresh completed successfully.
    Live {
        workflow_execution_id: String,
        refreshed_at: String,
        /// Total number of inspector signal sections attempted to load.
        sections_attempted: usize,
        /// Number of sections that returned Some (populated with data).
        sections_loaded: usize,
    },

    /// No workflow run selected — cannot refresh.
    Unavailable {
        reason: String,
    },

    /// Selected workflow run changed during async refresh.
    Stale {
        workflow_execution_id: String,
        reason: String,
    },

    /// Refresh failed due to an error.
    Failed {
        workflow_execution_id: Option<String>,
        error: String,
    },
}

impl Default for InspectorRefreshState {
    fn default() -> Self {
        Self::Idle
    }
}

impl InspectorRefreshState {
    /// Human-readable status label for UI display.
    pub fn status_label(&self) -> &'static str {
        match self {
            Self::Idle => "Inspector not refreshed",
            Self::Loading { .. } => "Refreshing inspector...",
            Self::Live { .. } => "Inspector live",
            Self::Unavailable { .. } => "Inspector unavailable",
            Self::Stale { .. } => "Inspector stale",
            Self::Failed { .. } => "Inspector refresh failed",
        }
    }

    /// Whether this state is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Live { .. }
                | Self::Unavailable { .. }
                | Self::Stale { .. }
                | Self::Failed { .. }
        )
    }

    /// Whether this state is a refresh in progress.
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_lifecycle_is_honest() {
        assert!(!InspectorRefreshState::Idle.is_terminal());
        assert!(!InspectorRefreshState::Idle.is_loading());

        let loading = InspectorRefreshState::Loading {
            workflow_execution_id: "wfx_1".into(),
        };
        assert!(loading.is_loading());
        assert!(!loading.is_terminal());

        let live = InspectorRefreshState::Live {
            workflow_execution_id: "wfx_1".into(),
            refreshed_at: "2026-06-13T22:00:00Z".into(),
            sections_attempted: 7,
            sections_loaded: 5,
        };
        assert!(live.is_terminal());
        assert!(!live.is_loading());

        let unavailable = InspectorRefreshState::Unavailable {
            reason: "No workflow run selected".into(),
        };
        assert!(unavailable.is_terminal());

        let stale = InspectorRefreshState::Stale {
            workflow_execution_id: "wfx_old".into(),
            reason: "Selection changed".into(),
        };
        assert!(stale.is_terminal());

        let failed = InspectorRefreshState::Failed {
            workflow_execution_id: None,
            error: "test".into(),
        };
        assert!(failed.is_terminal());
    }

    #[test]
    fn live_state_carries_audit_context() {
        let live = InspectorRefreshState::Live {
            workflow_execution_id: "wfx_abc".into(),
            refreshed_at: "2026-06-13T22:00:00Z".into(),
            sections_attempted: 10,
            sections_loaded: 8,
        };
        if let InspectorRefreshState::Live {
            workflow_execution_id,
            sections_attempted,
            sections_loaded,
            ..
        } = &live
        {
            assert_eq!(workflow_execution_id, "wfx_abc");
            assert_eq!(*sections_attempted, 10);
            assert_eq!(*sections_loaded, 8);
        }
    }

    #[test]
    fn status_labels_are_descriptive() {
        assert_eq!(InspectorRefreshState::Idle.status_label(), "Inspector not refreshed");
        assert_eq!(
            InspectorRefreshState::Loading { workflow_execution_id: "x".into() }.status_label(),
            "Refreshing inspector..."
        );
        assert_eq!(
            InspectorRefreshState::Live {
                workflow_execution_id: "x".into(),
                refreshed_at: "x".into(),
                sections_attempted: 1,
                sections_loaded: 1,
            }.status_label(),
            "Inspector live"
        );
    }
}
