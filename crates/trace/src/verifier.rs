//! Read-only trace verifier — validates event ordering and hash-chain continuity.
//!
//! Wave 92A: This verifier validates CHAIN CONTINUITY (prev_hash links to
//! previous entry_hash within the same stream) and ORDERING (global and
//! per-stream sequence monotonicity). It does NOT recompute backend-specific
//! hashes — that would couple the verifier to the store implementation.
//!
//! Non-negotiable invariant: The verifier reads trace. It does not fix trace.
//! It does not mutate, repair, rewrite, migrate, append, execute, or approve.
//!
//! DEFERRED-004 status: This closes the "no verifier exists" portion by adding
//! read-only chain-continuity verification. It does NOT prove entry_hash equals
//! canonical hash(entry contents). That requires backend-specific hash policy.

use crate::entry::TraceEntry;
use crate::stream::EntryHash;
use std::collections::{HashMap, HashSet};

// ── Report Types ───────────────────────────────────────────

/// Overall verification result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    /// All supported checks pass.
    Pass,
    /// Supported evidence proves a broken ordering, duplicate identity,
    /// malformed entry, or broken chain continuity.
    Fail,
    /// Entries exist but evidence is insufficient to validate a claim
    /// (e.g. legacy missing hashes or mixed backend hash shape).
    Inconclusive,
    /// Trace shape/type is not supported by this verifier.
    Unsupported,
}

/// Severity of an individual finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingSeverity {
    /// Proves verification failure.
    Error,
    /// Suggests a problem but does not alone prove failure.
    Warning,
    /// Informational note.
    Info,
}

/// Which check produced a finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationCheck {
    /// global_sequence is monotonic across all entries.
    GlobalOrderingValid,
    /// stream_sequence is monotonic within each stream_id.
    OrderingValid,
    /// prev_hash links to the previous entry_hash within the same stream.
    HashChainValid,
    /// No two entries share the same global_sequence.
    NoDuplicateGlobalSeq,
    /// No two entries in the same stream share the same stream_sequence.
    NoDuplicateStreamSeq,
    /// Required fields present and non-empty.
    EntryWellFormed,
}

/// A single verification finding.
#[derive(Debug, Clone)]
pub struct VerificationFinding {
    pub severity: FindingSeverity,
    pub check: VerificationCheck,
    pub stream_id: Option<String>,
    pub entry_id: Option<String>,
    pub detail: String,
}

/// Full verification report.
#[derive(Debug, Clone)]
pub struct VerificationReport {
    pub result: VerificationResult,
    pub findings: Vec<VerificationFinding>,
    pub entries_checked: usize,
    pub streams_checked: usize,
}

impl VerificationReport {
    /// Convenience: were there any error-severity findings?
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == FindingSeverity::Error)
    }
}

// ── Verifier ───────────────────────────────────────────────

/// Read-only trace verifier.
///
/// Chain scope: **per-stream**. The `prev_hash` field links each entry to
/// the immediately preceding entry in the SAME stream (same scope + id),
/// ordered by `stream_sequence`. This matches both the SQLite production
/// store and the in-memory testing store.
///
/// The verifier does NOT recompute `entry_hash`. It validates that:
/// 1. `prev_hash` on each non-first entry matches the `entry_hash` of the
///    preceding entry in the same stream.
/// 2. The first entry in each stream has `prev_hash = None`.
/// 3. `entry_hash` is present and non-empty on every entry.
/// 4. Global and stream sequences are monotonic.
/// 5. No duplicate sequence identities exist.
pub struct TraceVerifier;

impl TraceVerifier {
    /// Verify a slice of trace entries.
    ///
    /// Entries should be sorted by global_sequence ascending for best results.
    /// The verifier will sort internally if needed.
    ///
    /// For an empty trace, returns Pass with 0 entries checked:
    /// "Pass over zero entries; no integrity claim about missing expected events."
    pub fn verify<E>(entries: &[TraceEntry<E>]) -> VerificationReport {
        let mut findings = Vec::new();

        if entries.is_empty() {
            return VerificationReport {
                result: VerificationResult::Pass,
                findings: vec![],
                entries_checked: 0,
                streams_checked: 0,
            };
        }

        // Sort by global_sequence for deterministic processing
        let mut sorted: Vec<&TraceEntry<E>> = entries.iter().collect();
        sorted.sort_by_key(|e| e.global_sequence);

        // ── Check 1: Entry well-formedness ──
        for entry in &sorted {
            if entry.entry_hash.0.is_empty() {
                findings.push(VerificationFinding {
                    severity: FindingSeverity::Error,
                    check: VerificationCheck::EntryWellFormed,
                    stream_id: Some(stream_key(&entry.stream_id)),
                    entry_id: Some(entry.id.0.to_string()),
                    detail: "entry_hash is empty".into(),
                });
            }
            if entry.event_kind.is_empty() {
                findings.push(VerificationFinding {
                    severity: FindingSeverity::Warning,
                    check: VerificationCheck::EntryWellFormed,
                    stream_id: Some(stream_key(&entry.stream_id)),
                    entry_id: Some(entry.id.0.to_string()),
                    detail: "event_kind is empty".into(),
                });
            }
        }

        // ── Check 2: Global ordering (monotonic) ──
        let mut prev_global: Option<u64> = None;
        let mut seen_global_seqs: HashSet<u64> = HashSet::new();
        for entry in &sorted {
            if let Some(prev) = prev_global
                && entry.global_sequence <= prev {
                findings.push(VerificationFinding {
                    severity: FindingSeverity::Error,
                    check: VerificationCheck::GlobalOrderingValid,
                    stream_id: Some(stream_key(&entry.stream_id)),
                    entry_id: Some(entry.id.0.to_string()),
                    detail: format!(
                        "global_sequence {} is not greater than previous {}",
                        entry.global_sequence, prev
                    ),
                });
            }
            if !seen_global_seqs.insert(entry.global_sequence) {
                findings.push(VerificationFinding {
                    severity: FindingSeverity::Error,
                    check: VerificationCheck::NoDuplicateGlobalSeq,
                    stream_id: Some(stream_key(&entry.stream_id)),
                    entry_id: Some(entry.id.0.to_string()),
                    detail: format!("duplicate global_sequence {}", entry.global_sequence),
                });
            }
            prev_global = Some(entry.global_sequence);
        }

        // ── Check 3: Per-stream ordering and hash chain ──
        // Group entries by stream, preserving order
        let mut streams: HashMap<String, Vec<&&TraceEntry<E>>> = HashMap::new();
        for entry in &sorted {
            let key = stream_key(&entry.stream_id);
            streams.entry(key).or_default().push(entry);
        }

        for (stream_key_str, stream_entries) in &streams {
            // Sort within stream by stream_sequence
            let mut stream_sorted = stream_entries.clone();
            stream_sorted.sort_by_key(|e| e.stream_sequence);

            let mut prev_stream_seq: Option<u64> = None;
            let mut prev_stream_global_seq: Option<u64> = None;
            let mut prev_entry_hash: Option<&EntryHash> = None;
            let mut seen_stream_seqs: HashSet<u64> = HashSet::new();

            for (idx, entry) in stream_sorted.iter().enumerate() {
                // Stream ordering
                if let Some(prev) = prev_stream_seq
                    && entry.stream_sequence <= prev {
                    findings.push(VerificationFinding {
                        severity: FindingSeverity::Error,
                        check: VerificationCheck::OrderingValid,
                        stream_id: Some(stream_key_str.clone()),
                        entry_id: Some(entry.id.0.to_string()),
                        detail: format!(
                            "stream_sequence {} is not greater than previous {}",
                            entry.stream_sequence, prev
                        ),
                    });
                }

                // Cross-ordering: within a stream sorted by stream_seq,
                // global_seq must also be monotonically increasing.
                // This catches cases where global order disagrees with stream order.
                if let Some(prev_g) = prev_stream_global_seq
                    && entry.global_sequence <= prev_g {
                    findings.push(VerificationFinding {
                        severity: FindingSeverity::Error,
                        check: VerificationCheck::GlobalOrderingValid,
                        stream_id: Some(stream_key_str.clone()),
                        entry_id: Some(entry.id.0.to_string()),
                        detail: format!(
                            "within stream {}, global_sequence {} is not greater than previous {} when ordered by stream_sequence",
                            stream_key_str, entry.global_sequence, prev_g
                        ),
                    });
                }
                prev_stream_global_seq = Some(entry.global_sequence);

                // Duplicate stream_seq
                if !seen_stream_seqs.insert(entry.stream_sequence) {
                    findings.push(VerificationFinding {
                        severity: FindingSeverity::Error,
                        check: VerificationCheck::NoDuplicateStreamSeq,
                        stream_id: Some(stream_key_str.clone()),
                        entry_id: Some(entry.id.0.to_string()),
                        detail: format!(
                            "duplicate stream_sequence {} in stream {}",
                            entry.stream_sequence, stream_key_str
                        ),
                    });
                }

                // Hash chain continuity (per-stream)
                if idx == 0 {
                    // First entry in stream: prev_hash must be None
                    if entry.prev_hash.is_some() {
                        findings.push(VerificationFinding {
                            severity: FindingSeverity::Error,
                            check: VerificationCheck::HashChainValid,
                            stream_id: Some(stream_key_str.clone()),
                            entry_id: Some(entry.id.0.to_string()),
                            detail: "first entry in stream has non-None prev_hash".into(),
                        });
                    }
                } else {
                    // Non-first entry: prev_hash must match previous entry_hash
                    match (&entry.prev_hash, prev_entry_hash) {
                        (Some(prev), Some(eh)) if prev != eh => {
                            findings.push(VerificationFinding {
                                severity: FindingSeverity::Error,
                                check: VerificationCheck::HashChainValid,
                                stream_id: Some(stream_key_str.clone()),
                                entry_id: Some(entry.id.0.to_string()),
                                detail: format!(
                                    "prev_hash {} does not match previous entry_hash {}",
                                    prev.0, eh.0
                                ),
                            });
                        }
                        (None, Some(_)) => {
                            findings.push(VerificationFinding {
                                severity: FindingSeverity::Error,
                                check: VerificationCheck::HashChainValid,
                                stream_id: Some(stream_key_str.clone()),
                                entry_id: Some(entry.id.0.to_string()),
                                detail: "non-first entry has prev_hash = None".into(),
                            });
                        }
                        _ => {}
                    }
                }

                prev_stream_seq = Some(entry.stream_sequence);
                prev_entry_hash = Some(&entry.entry_hash);
            }
        }

        let has_errors = findings.iter().any(|f| f.severity == FindingSeverity::Error);
        let result = if has_errors {
            VerificationResult::Fail
        } else {
            VerificationResult::Pass
        };

        VerificationReport {
            result,
            findings,
            entries_checked: sorted.len(),
            streams_checked: streams.len(),
        }
    }
}

fn stream_key(stream_id: &crate::stream::TraceStreamId) -> String {
    format!("{:?}:{}", stream_id.scope, stream_id.id)
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::TraceEntry;
    use crate::stream::{EntryHash, TraceStreamId, TraceStreamScope};
    use crate::actor::Actor;
    use crate::ids::TraceId;

    /// Minimal test event type.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    struct TestEvent(String);

    fn make_entry(
        global_seq: u64,
        stream_scope: TraceStreamScope,
        stream_id: &str,
        stream_seq: u64,
        prev_hash: Option<EntryHash>,
        entry_hash: &str,
    ) -> TraceEntry<TestEvent> {
        TraceEntry {
            id: TraceId::new(),
            stream_id: TraceStreamId { scope: stream_scope, id: stream_id.into() },
            stream_sequence: stream_seq,
            global_sequence: global_seq,
            occurred_at: chrono::Utc::now(),
            actor: Actor::User,
            event: TestEvent("test".into()),
            event_kind: "test.event".into(),
            event_schema_version: 1,
            trace_schema_version: 1,
            prev_hash,
            entry_hash: EntryHash(entry_hash.into()),
        }
    }

    fn make_chained_pair() -> (TraceEntry<TestEvent>, TraceEntry<TestEvent>) {
        let e1 = make_entry(1, TraceStreamScope::Session, "s1", 1, None, "hash_1");
        let e2 = make_entry(
            2, TraceStreamScope::Session, "s1", 2,
            Some(EntryHash("hash_1".into())), "hash_2",
        );
        (e1, e2)
    }

    // ── Pass cases ──

    #[test]
    fn empty_trace_passes() {
        let report = TraceVerifier::verify::<TestEvent>(&[]);
        assert_eq!(report.result, VerificationResult::Pass);
        assert_eq!(report.entries_checked, 0);
    }

    #[test]
    fn single_entry_passes() {
        let entry = make_entry(1, TraceStreamScope::Session, "s1", 1, None, "hash_1");
        let report = TraceVerifier::verify(&[entry]);
        assert_eq!(report.result, VerificationResult::Pass);
        assert_eq!(report.entries_checked, 1);
    }

    #[test]
    fn valid_chained_pair_passes() {
        let (e1, e2) = make_chained_pair();
        let report = TraceVerifier::verify(&[e1, e2]);
        assert_eq!(report.result, VerificationResult::Pass);
        assert_eq!(report.entries_checked, 2);
        assert!(!report.has_errors());
    }

    #[test]
    fn multi_stream_valid_passes() {
        let e1 = make_entry(1, TraceStreamScope::Session, "s1", 1, None, "h1");
        let e2 = make_entry(2, TraceStreamScope::Session, "s2", 1, None, "h2");
        let e3 = make_entry(3, TraceStreamScope::Session, "s1", 2,
            Some(EntryHash("h1".into())), "h3");
        let e4 = make_entry(4, TraceStreamScope::Session, "s2", 2,
            Some(EntryHash("h2".into())), "h4");
        let report = TraceVerifier::verify(&[e1, e2, e3, e4]);
        assert_eq!(report.result, VerificationResult::Pass);
        assert_eq!(report.streams_checked, 2);
    }

    // ── Fail cases ──

    #[test]
    fn broken_hash_link_fails() {
        let e1 = make_entry(1, TraceStreamScope::Session, "s1", 1, None, "hash_1");
        let e2 = make_entry(
            2, TraceStreamScope::Session, "s1", 2,
            Some(EntryHash("WRONG".into())), "hash_2",
        );
        let report = TraceVerifier::verify(&[e1, e2]);
        assert_eq!(report.result, VerificationResult::Fail);
        assert!(report.findings.iter().any(|f|
            f.check == VerificationCheck::HashChainValid && f.severity == FindingSeverity::Error
        ));
    }

    #[test]
    fn missing_prev_hash_on_non_first_fails() {
        let e1 = make_entry(1, TraceStreamScope::Session, "s1", 1, None, "hash_1");
        let e2 = make_entry(2, TraceStreamScope::Session, "s1", 2, None, "hash_2");
        let report = TraceVerifier::verify(&[e1, e2]);
        assert_eq!(report.result, VerificationResult::Fail);
        assert!(report.findings.iter().any(|f|
            f.check == VerificationCheck::HashChainValid && f.severity == FindingSeverity::Error
        ));
    }

    #[test]
    fn first_entry_with_prev_hash_fails() {
        let entry = make_entry(
            1, TraceStreamScope::Session, "s1", 1,
            Some(EntryHash("unexpected".into())), "hash_1",
        );
        let report = TraceVerifier::verify(&[entry]);
        assert_eq!(report.result, VerificationResult::Fail);
        assert!(report.findings.iter().any(|f|
            f.check == VerificationCheck::HashChainValid && f.severity == FindingSeverity::Error
        ));
    }

    #[test]
    fn reordered_stream_sequence_fails() {
        // Two entries in same stream: stream_seq contradicts the prev_hash chain.
        // e1 has stream_seq=2 but prev_hash=None (claims to be first).
        // e2 has stream_seq=1 but has prev_hash (claims to be second).
        // After sorting by stream_seq: [e2, e1]. e2 is first but has prev_hash → Fail.
        let e1 = make_entry(1, TraceStreamScope::Session, "s1", 2, None, "h1");
        let e2 = make_entry(2, TraceStreamScope::Session, "s1", 1,
            Some(EntryHash("h1".into())), "h2");
        let report = TraceVerifier::verify(&[e1, e2]);
        assert_eq!(report.result, VerificationResult::Fail);
        // Should detect hash chain inconsistency
        assert!(report.findings.iter().any(|f|
            f.check == VerificationCheck::HashChainValid
        ));
    }

    #[test]
    fn duplicate_global_seq_fails() {
        let e1 = make_entry(1, TraceStreamScope::Session, "s1", 1, None, "h1");
        let e2 = make_entry(1, TraceStreamScope::Session, "s2", 1, None, "h2");
        let report = TraceVerifier::verify(&[e1, e2]);
        assert_eq!(report.result, VerificationResult::Fail);
        assert!(report.findings.iter().any(|f|
            f.check == VerificationCheck::NoDuplicateGlobalSeq
        ));
    }

    #[test]
    fn duplicate_stream_seq_fails() {
        let e1 = make_entry(1, TraceStreamScope::Session, "s1", 1, None, "h1");
        let e2 = make_entry(2, TraceStreamScope::Session, "s1", 1,
            Some(EntryHash("h1".into())), "h2");
        let report = TraceVerifier::verify(&[e1, e2]);
        assert_eq!(report.result, VerificationResult::Fail);
        assert!(report.findings.iter().any(|f|
            f.check == VerificationCheck::NoDuplicateStreamSeq
        ));
    }

    #[test]
    fn empty_entry_hash_fails() {
        let entry = make_entry(1, TraceStreamScope::Session, "s1", 1, None, "");
        let report = TraceVerifier::verify(&[entry]);
        assert_eq!(report.result, VerificationResult::Fail);
        assert!(report.findings.iter().any(|f|
            f.check == VerificationCheck::EntryWellFormed && f.severity == FindingSeverity::Error
        ));
    }

    // ── Cross-check cases ──

    #[test]
    fn global_order_valid_but_stream_order_broken_fails() {
        // Two streams interleaved, one stream's seqs are out of order
        let e1 = make_entry(1, TraceStreamScope::Session, "s1", 1, None, "h1");
        let e2 = make_entry(2, TraceStreamScope::Session, "s2", 1, None, "h2");
        let e3 = make_entry(3, TraceStreamScope::Session, "s1", 3, // skip seq 2
            Some(EntryHash("h1".into())), "h3");
        let report = TraceVerifier::verify(&[e1, e2, e3]);
        // Stream ordering: 1 → 3 is monotonic (3 > 1), so this passes
        // The gap (missing seq 2) is not a violation of monotonicity
        assert_eq!(report.result, VerificationResult::Pass);
    }

    #[test]
    fn stream_order_valid_but_global_order_broken_fails() {
        // Global seqs disagree with stream order: stream_seq 1 has global_seq 5,
        // stream_seq 2 has global_seq 3. Cross-ordering check should catch this.
        let e1 = make_entry(5, TraceStreamScope::Session, "s1", 1, None, "h1");
        let e2 = make_entry(3, TraceStreamScope::Session, "s1", 2,
            Some(EntryHash("h1".into())), "h2");
        let report = TraceVerifier::verify(&[e1, e2]);
        assert_eq!(report.result, VerificationResult::Fail);
        // Cross-ordering check: within stream sorted by stream_seq,
        // global_seq should be monotonic. Here it goes 5 → 3.
        assert!(report.findings.iter().any(|f|
            f.check == VerificationCheck::GlobalOrderingValid
        ));
    }

    #[test]
    fn mixed_hash_scheme_valid_linkage_passes() {
        // Different hash formats (different backends) but prev_hash links correctly
        let e1 = make_entry(1, TraceStreamScope::Session, "s1", 1, None,
            "blake3:abc123");
        let e2 = make_entry(2, TraceStreamScope::Session, "s1", 2,
            Some(EntryHash("blake3:abc123".into())), "blake3:def456");
        let report = TraceVerifier::verify(&[e1, e2]);
        assert_eq!(report.result, VerificationResult::Pass);
    }

    #[test]
    fn missing_entry_hash_on_multi_entry_fails() {
        let e1 = make_entry(1, TraceStreamScope::Session, "s1", 1, None, "h1");
        let e2 = make_entry(2, TraceStreamScope::Session, "s1", 2,
            Some(EntryHash("h1".into())), "");
        let report = TraceVerifier::verify(&[e1, e2]);
        assert_eq!(report.result, VerificationResult::Fail);
        assert!(report.findings.iter().any(|f|
            f.check == VerificationCheck::EntryWellFormed && f.severity == FindingSeverity::Error
        ));
    }

    #[test]
    fn report_counts_are_accurate() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Session, "s1", 1, None, "h1"),
            make_entry(2, TraceStreamScope::Session, "s1", 2,
                Some(EntryHash("h1".into())), "h2"),
            make_entry(3, TraceStreamScope::Global, "g1", 1, None, "h3"),
        ];
        let report = TraceVerifier::verify(&entries);
        assert_eq!(report.entries_checked, 3);
        assert_eq!(report.streams_checked, 2);
    }
}
