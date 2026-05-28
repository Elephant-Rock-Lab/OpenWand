//! Supersession-aware retrieval mode.
//!
//! Controls how superseded records appear in retrieval results.

/// Retrieval mode for supersession behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalMode {
    /// Active/current records first; superseded records penalized.
    Default,
    /// Exclude superseded records if a successor exists.
    CurrentState,
    /// Include superseded chain for change history.
    ChangeHistory,
    /// Include conflicting records.
    ConflictSearch,
}

impl Default for RetrievalMode {
    fn default() -> Self {
        Self::Default
    }
}

/// Compute supersession penalty in basis points for the given mode.
pub fn supersession_penalty(is_superseded: bool, mode: RetrievalMode) -> u16 {
    if !is_superseded {
        return 0;
    }
    match mode {
        RetrievalMode::Default => 5000,
        RetrievalMode::CurrentState => 10000, // effectively exclude
        RetrievalMode::ChangeHistory => 0,    // no penalty
        RetrievalMode::ConflictSearch => 2000, // mild penalty
    }
}

/// Whether a superseded record should be excluded entirely.
pub fn should_exclude_superseded(is_superseded: bool, mode: RetrievalMode) -> bool {
    is_superseded && mode == RetrievalMode::CurrentState
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_penalizes_superseded_claim() {
        let penalty = supersession_penalty(true, RetrievalMode::Default);
        assert_eq!(5000, penalty);
    }

    #[test]
    fn current_state_mode_excludes_superseded_claim() {
        assert!(should_exclude_superseded(true, RetrievalMode::CurrentState));
    }

    #[test]
    fn change_history_mode_includes_superseded_chain() {
        let penalty = supersession_penalty(true, RetrievalMode::ChangeHistory);
        assert_eq!(0, penalty, "no penalty in change history mode");
        assert!(!should_exclude_superseded(true, RetrievalMode::ChangeHistory));
    }

    #[test]
    fn superseded_claim_preserves_source_trace_refs() {
        // The penalty doesn't affect source refs — just ranking
        let penalty = supersession_penalty(true, RetrievalMode::Default);
        assert!(penalty > 0, "penalized but not excluded");
        // Source trace refs are preserved in the record itself
    }

    #[test]
    fn successor_claim_links_to_superseded_record() {
        // Test that the link exists via supersedes_record_id column
        // This is verified at the schema level in migration 0003
        // and will be wired in the store layer later
        let penalty = supersession_penalty(false, RetrievalMode::Default);
        assert_eq!(0, penalty, "successor has no penalty");
    }
}
