//! Verification signal index — pre-ranking trace relation lookup.
//!
//! Computes verification counts keyed by source_trace_id from trace store.
//! Consumed by ranking as a signal, NOT from hydrated panel DTOs.
//! This solves the timing problem: ranking happens before trace hydration.

use std::collections::{HashMap, HashSet};

/// Verification signal for a single source trace ID.
#[derive(Debug, Clone)]
pub struct VerificationSignal {
    /// Number of Verifies relations pointing at this trace.
    pub verifies_count: u16,
    /// Number of DerivedFrom relations.
    pub derived_from_count: u16,
    /// Number of Refines relations.
    pub refines_count: u16,
}

impl VerificationSignal {
    pub fn empty() -> Self {
        Self {
            verifies_count: 0,
            derived_from_count: 0,
            refines_count: 0,
        }
    }
}

/// Index mapping source trace IDs to their verification signals.
/// Built before ranking from trace store relations.
#[derive(Debug, Clone, Default)]
pub struct VerificationSignalIndex {
    signals: HashMap<String, VerificationSignal>,
}

impl VerificationSignalIndex {
    /// Build index from a list of (from_trace_id, to_trace_id, kind) tuples.
    /// The "from" trace is the one that gets the signal.
    /// A Verifies relation from A to B means A is verified by B.
    pub fn from_relations(relations: &[(String, String, String)]) -> Self {
        let mut index = Self::default();

        for (from_id, _to_id, kind) in relations {
            let signal = index.signals.entry(from_id.clone()).or_insert_with(VerificationSignal::empty);
            match kind.as_str() {
                "Verifies" | "verifies" => signal.verifies_count += 1,
                "DerivedFrom" | "derived_from" => signal.derived_from_count += 1,
                "Refines" | "refines" => signal.refines_count += 1,
                _ => {
                    // References, Implements, etc. — no verification signal
                    // Remove the empty entry if we just created it
                    if signal.verifies_count == 0 && signal.derived_from_count == 0 && signal.refines_count == 0 {
                        index.signals.remove(from_id);
                    }
                    continue;
                }
            }
        }

        index
    }

    /// Get the verification signal for a trace ID.
    pub fn get(&self, trace_id: &str) -> Option<&VerificationSignal> {
        self.signals.get(trace_id)
    }

    /// Get all trace IDs in the index.
    pub fn trace_ids(&self) -> HashSet<&str> {
        self.signals.keys().map(|s| s.as_str()).collect()
    }

    /// Compute verification boost (in basis points) for a record
    /// given its source trace IDs and the verification policy.
    pub fn compute_boost(
        &self,
        source_trace_ids: &[String],
        policy: &crate::governance::VerificationPolicy,
    ) -> u16 {
        let mut total_boost: u32 = 0;
        for trace_id in source_trace_ids {
            if let Some(signal) = self.get(trace_id) {
                total_boost +=
                    signal.verifies_count as u32 * policy.verifies_boost_bps as u32
                    + signal.derived_from_count as u32 * policy.derived_from_boost_bps as u32
                    + signal.refines_count as u32 * policy.refines_boost_bps as u32;
            }
        }
        // Cap at 10000 (100%)
        total_boost.min(10000) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::VerificationPolicy;

    #[test]
    fn empty_index_returns_none() {
        let idx = VerificationSignalIndex::default();
        assert!(idx.get("nonexistent").is_none());
    }

    #[test]
    fn verifies_relation_adds_count() {
        let idx = VerificationSignalIndex::from_relations(&[
            ("trace_a".into(), "trace_b".into(), "Verifies".into()),
        ]);
        assert_eq!(1, idx.get("trace_a").unwrap().verifies_count);
    }

    #[test]
    fn derived_from_adds_smaller_count() {
        let idx = VerificationSignalIndex::from_relations(&[
            ("trace_a".into(), "trace_b".into(), "DerivedFrom".into()),
        ]);
        assert_eq!(0, idx.get("trace_a").unwrap().verifies_count);
        assert_eq!(1, idx.get("trace_a").unwrap().derived_from_count);
    }

    #[test]
    fn references_gives_no_signal() {
        let idx = VerificationSignalIndex::from_relations(&[
            ("trace_a".into(), "trace_b".into(), "References".into()),
        ]);
        assert!(idx.get("trace_a").is_none());
    }

    #[test]
    fn unknown_gives_no_signal() {
        let idx = VerificationSignalIndex::from_relations(&[
            ("trace_a".into(), "trace_b".into(), "Unknown".into()),
        ]);
        assert!(idx.get("trace_a").is_none());
    }

    #[test]
    fn compute_boost_with_policy() {
        let idx = VerificationSignalIndex::from_relations(&[
            ("trace_a".into(), "trace_b".into(), "Verifies".into()),
            ("trace_a".into(), "trace_c".into(), "Verifies".into()),
        ]);
        let policy = VerificationPolicy {
            verifies_boost_bps: 2000,
            derived_from_boost_bps: 500,
            refines_boost_bps: 800,
        };
        let boost = idx.compute_boost(&["trace_a".to_string()], &policy);
        assert_eq!(4000, boost); // 2 * 2000
    }

    #[test]
    fn compute_boost_caps_at_10000() {
        let relations: Vec<(String, String, String)> = (0..10)
            .map(|i| ("trace_a".into(), format!("trace_{}", i), "Verifies".into()))
            .collect();
        let idx = VerificationSignalIndex::from_relations(&relations);
        let policy = VerificationPolicy {
            verifies_boost_bps: 2000,
            derived_from_boost_bps: 500,
            refines_boost_bps: 800,
        };
        let boost = idx.compute_boost(&["trace_a".to_string()], &policy);
        assert_eq!(10000, boost); // capped
    }

    #[test]
    fn compute_boost_for_record_with_multiple_traces() {
        let idx = VerificationSignalIndex::from_relations(&[
            ("t1".into(), "t2".into(), "Verifies".into()),
            ("t3".into(), "t4".into(), "DerivedFrom".into()),
        ]);
        let policy = VerificationPolicy {
            verifies_boost_bps: 2000,
            derived_from_boost_bps: 500,
            refines_boost_bps: 800,
        };
        let boost = idx.compute_boost(&["t1".to_string(), "t3".to_string()], &policy);
        assert_eq!(2500, boost); // 2000 + 500
    }

    #[test]
    fn compute_boost_no_matching_traces() {
        let idx = VerificationSignalIndex::default();
        let policy = VerificationPolicy {
            verifies_boost_bps: 2000,
            derived_from_boost_bps: 500,
            refines_boost_bps: 800,
        };
        let boost = idx.compute_boost(&["trace_x".to_string()], &policy);
        assert_eq!(0, boost);
    }

    #[test]
    fn verification_boost_is_deterministic() {
        let idx = VerificationSignalIndex::from_relations(&[
            ("t1".into(), "t2".into(), "Verifies".into()),
        ]);
        let policy = VerificationPolicy {
            verifies_boost_bps: 2000,
            derived_from_boost_bps: 500,
            refines_boost_bps: 800,
        };
        let b1 = idx.compute_boost(&["t1".to_string()], &policy);
        let b2 = idx.compute_boost(&["t1".to_string()], &policy);
        assert_eq!(b1, b2);
    }

    #[test]
    fn default_policy_gives_zero_boost() {
        let idx = VerificationSignalIndex::from_relations(&[
            ("t1".into(), "t2".into(), "Verifies".into()),
        ]);
        let policy = VerificationPolicy::default(); // all zeros
        let boost = idx.compute_boost(&["t1".to_string()], &policy);
        assert_eq!(0, boost);
    }
}
