//! External checkpoint anchor model — read-only verification.
//!
//! Wave 104A: Defines the checkpoint anchor DTOs, root-hash computation,
//! and verification semantics for externally persisted trace checkpoints.
//!
//! **What this module provides:**
//! - `CheckpointAnchor` DTO (serde-serializable)
//! - `AnchorVerificationResult` + `AnchorFreshness` enums
//! - `AnchorVerificationReport` struct
//! - `compute_root_hash()` pure function
//! - `verify_anchor()` pure/read-only function
//!
//! **What this module does NOT provide:**
//! - Anchor file writing (deferred to 104B: `CheckpointWriter`)
//! - CLI commands (deferred to 104B)
//! - Path containment checks for anchor roots (deferred to 104B)
//! - Checkpoint sequence registry (deferred to 104B)
//!
//! **Authority:** The verifier READS anchors and COMPARES hashes.
//! It does not create, repair, delete, or modify anchors.
//! It does not mutate trace entries.
//!
//! **Dependency on hash correctness:** The checkpoint root hash is a rollup
//! over stored `entry_hash` values. It does NOT recompute event payload hashes.
//! In practice, verification should be run as:
//!   1. `trace-verify` with hash correctness (98B)
//!   2. Anchor verification over entry_hash rollup
//!
//! **Known limitation:** If an attacker modifies entry payloads and recomputes
//! both `entry_hash` values AND the external anchor, the anchor model cannot
//! detect it. Full immutability requires remote attestation or cryptographic
//! signatures, which are out of scope.

use crate::entry::TraceEntry;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── Anchor DTO ─────────────────────────────────────────────

/// Trace state captured at checkpoint time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointTraceState {
    /// Number of trace entries at checkpoint time.
    pub entry_count: u64,
    /// Last global_sequence value at checkpoint time.
    pub last_global_sequence: u64,
    /// BLAKE3 root hash over all entry_hash values (sorted by global_sequence).
    pub root_hash: String,
}

/// A checkpoint anchor — externally persisted evidence of trace state.
///
/// Serialized as JSON and stored outside the trace store root.
/// The anchor provides tamper evidence: if the trace store is modified
/// after the checkpoint, the root hash will not match on verification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointAnchor {
    /// Anchor format version. Currently 1.
    pub version: u16,

    /// Monotonic checkpoint sequence number.
    pub checkpoint_sequence: u64,

    /// Workspace or session identifier.
    pub workspace_id: String,

    /// ISO-8601 timestamp when the checkpoint was created.
    pub created_at: String,

    /// Trace state at checkpoint time.
    pub trace: CheckpointTraceState,

    /// Hash algorithm used for root hash computation.
    pub hash_algorithm: String,

    /// Source identifier for the anchor writer.
    pub anchor_source: String,
}

/// Supported anchor format versions.
pub const ANCHOR_FORMAT_VERSION: u16 = 1;

// ── Verification Result Types ──────────────────────────────

/// Integrity verification result for an anchor.
///
/// This is SEPARATE from freshness: an anchor can be `Pass` (integrity holds
/// for the checkpointed prefix) even if the trace has grown since the
/// checkpoint. Freshness is tracked by `AnchorFreshness`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorVerificationResult {
    /// Anchor exists and root hash matches for the checkpointed prefix.
    Pass,
    /// Anchor exists but root hash or entry count mismatch for the
    /// checkpointed prefix. The trace was modified after the checkpoint.
    Fail,
    /// No anchor was provided or the anchor file does not exist.
    MissingAnchor,
    /// Anchor format version is not recognized.
    Unsupported,
}

/// Freshness state of an anchor relative to the current trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorFreshness {
    /// Trace has not grown since the checkpoint.
    /// `current_entries == anchor.trace.entry_count`.
    Current,

    /// Trace has additional entries appended after the checkpoint.
    /// The checkpointed prefix is still valid; the new entries are
    /// simply outside anchor coverage.
    Stale {
        /// Number of entries appended after the checkpoint.
        additional_entries: u64,
    },
}

/// Full anchor verification report.
#[derive(Debug, Clone)]
pub struct AnchorVerificationReport {
    /// Integrity result for the checkpointed prefix.
    pub result: AnchorVerificationResult,
    /// Freshness relative to current trace.
    pub freshness: AnchorFreshness,
    /// Recomputed root hash (if verification was performed).
    pub recomputed_root_hash: Option<String>,
    /// Human-readable detail about the verification.
    pub detail: String,
}

// ── Root Hash Computation ──────────────────────────────────

/// Compute a root hash over trace entries.
///
/// The root hash is a BLAKE3 rollup:
/// 1. Sort entries by `global_sequence` ascending.
/// 2. Feed each entry's `entry_hash` bytes into a running BLAKE3 hasher.
/// 3. The final digest is the root hash.
///
/// This detects insertion, deletion, or modification of entries after
/// checkpoint unless the attacker can also rewrite the external anchor
/// to match the modified store.
///
/// **Dependency:** The root hash is computed over stored `entry_hash` values.
/// It does NOT recompute event payload hashes. For full integrity, run
/// `trace-verify` with hash correctness (98B) BEFORE anchor verification.
///
/// **Known limitation:** If an attacker modifies payloads and recomputes
/// both `entry_hash` values AND the external anchor, the local anchor model
/// cannot detect it.
pub fn compute_root_hash<E>(entries: &[TraceEntry<E>]) -> String {
    let mut sorted: Vec<&TraceEntry<E>> = entries.iter().collect();
    sorted.sort_by_key(|e| e.global_sequence);

    let mut hasher = blake3::Hasher::new();
    for entry in &sorted {
        hasher.update(entry.entry_hash.0.as_bytes());
    }

    let hash = hasher.finalize();
    format!("blake3:{}", hash.to_hex())
}

/// Compute a root hash over entries up to a given global_sequence (inclusive).
///
/// This is used for prefix verification: the anchor covers entries with
/// `global_sequence <= last_global_sequence`. Entries appended after the
/// checkpoint are excluded from the root hash computation.
pub fn compute_root_hash_prefix<E>(
    entries: &[TraceEntry<E>],
    last_global_sequence: u64,
) -> (String, u64) {
    let mut filtered: Vec<&TraceEntry<E>> = entries
        .iter()
        .filter(|e| e.global_sequence <= last_global_sequence)
        .collect();
    filtered.sort_by_key(|e| e.global_sequence);

    let count = filtered.len() as u64;

    let mut hasher = blake3::Hasher::new();
    for entry in &filtered {
        hasher.update(entry.entry_hash.0.as_bytes());
    }

    let hash = hasher.finalize();
    (format!("blake3:{}", hash.to_hex()), count)
}

// ── Anchor Verification ────────────────────────────────────

/// Verify an anchor against the current trace entries.
///
/// This is a PURE, READ-ONLY function. It does not read files, write files,
/// or mutate state. The caller provides both the entries and the anchor.
///
/// Verification logic:
/// 1. Check anchor format version → `Unsupported` if not recognized.
/// 2. Compute root hash over entries with `global_sequence <= anchor.trace.last_global_sequence`.
/// 3. Compare entry count and root hash.
/// 4. Determine freshness: `Current` if no additional entries, `Stale` otherwise.
///
/// **Authority:** Read-only. No mutation of entries or anchors.
pub fn verify_anchor<E>(
    entries: &[TraceEntry<E>],
    anchor: Option<&CheckpointAnchor>,
) -> AnchorVerificationReport {
    // No anchor provided
    let anchor = match anchor {
        None => {
            return AnchorVerificationReport {
                result: AnchorVerificationResult::MissingAnchor,
                freshness: AnchorFreshness::Current,
                recomputed_root_hash: None,
                detail: "No anchor provided. Anchor verification is Inconclusive (not a trace failure).".into(),
            };
        }
        Some(a) => a,
    };

    // Version check
    if anchor.version != ANCHOR_FORMAT_VERSION {
        return AnchorVerificationReport {
            result: AnchorVerificationResult::Unsupported,
            freshness: AnchorFreshness::Current,
            recomputed_root_hash: None,
            detail: format!(
                "Anchor version {} is not supported (expected {}).",
                anchor.version, ANCHOR_FORMAT_VERSION
            ),
        };
    }

    // Validate root_hash format
    if !anchor.trace.root_hash.starts_with("blake3:") {
        return AnchorVerificationReport {
            result: AnchorVerificationResult::Unsupported,
            freshness: AnchorFreshness::Current,
            recomputed_root_hash: None,
            detail: format!(
                "Anchor root_hash does not have expected 'blake3:' prefix: {}",
                &anchor.trace.root_hash.get(..20).unwrap_or(&anchor.trace.root_hash)
            ),
        };
    }

    // Compute root hash over the checkpointed prefix
    let (recomputed_hash, prefix_count) =
        compute_root_hash_prefix(entries, anchor.trace.last_global_sequence);

    // Check for entries removed (current has fewer entries than checkpoint)
    let current_count = entries.len() as u64;
    if current_count < anchor.trace.entry_count {
        return AnchorVerificationReport {
            result: AnchorVerificationResult::Fail,
            freshness: AnchorFreshness::Current,
            recomputed_root_hash: Some(recomputed_hash),
            detail: format!(
                "Trace has {} entries but anchor recorded {}. Entries were removed after checkpoint.",
                current_count, anchor.trace.entry_count
            ),
        };
    }

    // Check prefix entry count matches anchor
    if prefix_count != anchor.trace.entry_count {
        return AnchorVerificationReport {
            result: AnchorVerificationResult::Fail,
            freshness: AnchorFreshness::Current,
            recomputed_root_hash: Some(recomputed_hash),
            detail: format!(
                "Prefix (global_sequence <= {}) has {} entries but anchor recorded {}. Entries were removed or reordered within the checkpoint range.",
                anchor.trace.last_global_sequence, prefix_count, anchor.trace.entry_count
            ),
        };
    }

    // Compare root hashes
    if recomputed_hash != anchor.trace.root_hash {
        return AnchorVerificationReport {
            result: AnchorVerificationResult::Fail,
            freshness: AnchorFreshness::Current,
            recomputed_root_hash: Some(recomputed_hash),
            detail: "Root hash mismatch: trace was modified after checkpoint.".into(),
        };
    }

    // Root hash matches — determine freshness
    let additional = current_count.saturating_sub(anchor.trace.entry_count);
    if additional == 0 {
        AnchorVerificationReport {
            result: AnchorVerificationResult::Pass,
            freshness: AnchorFreshness::Current,
            recomputed_root_hash: Some(recomputed_hash),
            detail: "Anchor root hash matches. Trace is at checkpoint state.".into(),
        }
    } else {
        AnchorVerificationReport {
            result: AnchorVerificationResult::Pass,
            freshness: AnchorFreshness::Stale {
                additional_entries: additional,
            },
            recomputed_root_hash: Some(recomputed_hash),
            detail: format!(
                "Anchor root hash matches for checkpointed prefix. {} additional entries appended after checkpoint (outside anchor coverage).",
                additional
            ),
        }
    }
}

// ── Anchor File I/O (104B) ─────────────────────────────

/// Errors that can occur during anchor file operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorError {
    /// The anchor root is the same as the store root after canonicalization.
    AnchorRootEqualsStoreRoot,
    /// The anchor root is inside the store root after canonicalization.
    AnchorRootInsideStoreRoot,
    /// The store root is inside the anchor root after canonicalization.
    StoreRootInsideAnchorRoot,
    /// A path could not be canonicalized (may not exist).
    CanonicalizationFailed(String),
    /// A checkpoint file with this sequence already exists.
    CheckpointSequenceCollision(u64),
    /// File I/O error.
    IoError(String),
    /// Anchor file parsing error.
    ParseError(String),
}

impl std::fmt::Display for AnchorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnchorError::AnchorRootEqualsStoreRoot => {
                write!(f, "anchor_root must not equal store_root after canonicalization")
            }
            AnchorError::AnchorRootInsideStoreRoot => {
                write!(f, "anchor_root must not be inside store_root after canonicalization")
            }
            AnchorError::StoreRootInsideAnchorRoot => {
                write!(f, "store_root must not be inside anchor_root after canonicalization")
            }
            AnchorError::CanonicalizationFailed(p) => {
                write!(f, "failed to canonicalize path: {p}")
            }
            AnchorError::CheckpointSequenceCollision(seq) => {
                write!(f, "checkpoint sequence {seq} already exists")
            }
            AnchorError::IoError(msg) => write!(f, "I/O error: {msg}"),
            AnchorError::ParseError(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for AnchorError {}

/// Validate that the anchor root is properly separated from the store root.
///
/// Checks (after canonicalization):
/// 1. `anchor_root != store_root`
/// 2. `anchor_root` is not inside `store_root`
/// 3. `store_root` is not inside `anchor_root` (stronger separation)
///
/// Both paths must exist for canonicalization to succeed. The caller should
/// create the anchor root directory before calling this function.
pub fn validate_anchor_root(
    anchor_root: &Path,
    store_root: &Path,
) -> Result<PathBuf, AnchorError> {
    let canon_anchor = anchor_root
        .canonicalize()
        .map_err(|e| AnchorError::CanonicalizationFailed(format!("{}: {}", anchor_root.display(), e)))?;
    let canon_store = store_root
        .canonicalize()
        .map_err(|e| AnchorError::CanonicalizationFailed(format!("{}: {}", store_root.display(), e)))?;

    // Reject equality
    if canon_anchor == canon_store {
        return Err(AnchorError::AnchorRootEqualsStoreRoot);
    }

    // Reject anchor_root inside store_root
    if canon_anchor.starts_with(&canon_store) {
        return Err(AnchorError::AnchorRootInsideStoreRoot);
    }

    // Reject store_root inside anchor_root (stronger separation)
    if canon_store.starts_with(&canon_anchor) {
        return Err(AnchorError::StoreRootInsideAnchorRoot);
    }

    Ok(canon_anchor)
}

/// Controlled checkpoint writer — creates anchor files outside the trace store.
///
/// **Authority:** Creates anchor files ONLY. Does not mutate trace entries,
/// does not execute tools, does not approve actions, does not modify policy.
///
/// The writer computes the root hash from trace entries, constructs the
/// anchor DTO, and writes it to a JSON file in the anchor root directory.
pub struct CheckpointWriter;

impl CheckpointWriter {
    /// Write a checkpoint anchor file.
    ///
    /// The anchor is written to `{anchor_root}/openwand-checkpoint-{sequence}.json`.
    /// If a file with this sequence already exists, returns
    /// `CheckpointSequenceCollision`.
    ///
    /// The `anchor_root` must be canonicalized and separated from `store_root`
    /// (call `validate_anchor_root` first).
    ///
    /// Returns the path to the written anchor file.
    pub fn write_checkpoint<E>(
        entries: &[TraceEntry<E>],
        anchor_root: &Path,
        store_root: &Path,
        workspace_id: &str,
        checkpoint_sequence: u64,
    ) -> Result<PathBuf, AnchorError> {
        // Validate path separation
        let canon_anchor = validate_anchor_root(anchor_root, store_root)?;

        // Compute root hash over all entries
        let root_hash = compute_root_hash(entries);
        let entry_count = entries.len() as u64;
        let last_global_sequence = entries
            .iter()
            .map(|e| e.global_sequence)
            .max()
            .unwrap_or(0);

        // Construct anchor
        let anchor = CheckpointAnchor {
            version: ANCHOR_FORMAT_VERSION,
            checkpoint_sequence,
            workspace_id: workspace_id.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            trace: CheckpointTraceState {
                entry_count,
                last_global_sequence,
                root_hash,
            },
            hash_algorithm: "blake3".to_string(),
            anchor_source: "openwand-checkpoint-writer".to_string(),
        };

        // Build filename
        let filename = format!("openwand-checkpoint-{}.json", checkpoint_sequence);
        let anchor_path = canon_anchor.join(&filename);

        // Collision check
        if anchor_path.exists() {
            return Err(AnchorError::CheckpointSequenceCollision(checkpoint_sequence));
        }

        // Serialize and write
        let json = serde_json::to_string_pretty(&anchor)
            .map_err(|e| AnchorError::IoError(format!("failed to serialize anchor: {e}")))?;

        std::fs::write(&anchor_path, &json)
            .map_err(|e| AnchorError::IoError(format!("failed to write anchor file: {e}")))?;

        Ok(anchor_path)
    }

    /// Compute the next available checkpoint sequence by scanning existing
    /// anchor files in the anchor root.
    ///
    /// Looks for files matching `openwand-checkpoint-*.json` and returns
    /// `max(existing) + 1`, or 1 if none exist.
    pub fn next_sequence(anchor_root: &Path) -> Result<u64, AnchorError> {
        let canon = anchor_root
            .canonicalize()
            .map_err(|e| AnchorError::CanonicalizationFailed(format!("{}: {}", anchor_root.display(), e)))?;

        let mut max_seq: u64 = 0;

        let entries = std::fs::read_dir(&canon)
            .map_err(|e| AnchorError::IoError(format!("failed to read anchor root: {e}")))?;

        for entry in entries {
            let entry = entry.map_err(|e| AnchorError::IoError(format!("dir entry error: {e}")))?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if let Some(seq_str) = name_str
                .strip_prefix("openwand-checkpoint-")
                .and_then(|s| s.strip_suffix(".json"))
                .and_then(|s| s.parse::<u64>().ok())
            {
                max_seq = max_seq.max(seq_str);
            }
        }

        Ok(max_seq + 1)
    }
}

/// Read an anchor file from disk.
///
/// This is a READ-ONLY operation used by the verifier CLI to load
/// externally persisted anchors.
pub fn read_anchor_file(path: &Path) -> Result<CheckpointAnchor, AnchorError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| AnchorError::IoError(format!("failed to read anchor file {}: {}", path.display(), e)))?;

    serde_json::from_str(&contents)
        .map_err(|e| AnchorError::ParseError(format!("failed to parse anchor JSON: {e}")))
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::Actor;
    use crate::entry::TraceEntry;
    use crate::ids::TraceId;
    use crate::stream::{EntryHash, TraceStreamId, TraceStreamScope};

    /// Minimal test event type.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    struct TestEvent(String);

    fn make_entry(
        global_seq: u64,
        entry_hash: &str,
    ) -> TraceEntry<TestEvent> {
        TraceEntry {
            id: TraceId::new(),
            stream_id: TraceStreamId {
                scope: TraceStreamScope::Session,
                id: "s1".into(),
            },
            stream_sequence: global_seq,
            global_sequence: global_seq,
            occurred_at: chrono::Utc::now(),
            actor: Actor::User,
            event: TestEvent("test".into()),
            event_kind: "test.event".into(),
            event_schema_version: 1,
            trace_schema_version: 1,
            prev_hash: if global_seq > 1 {
                Some(EntryHash(format!("prev_{}", global_seq - 1)))
            } else {
                None
            },
            entry_hash: EntryHash(entry_hash.into()),
        }
    }

    fn make_anchor(entries: &[TraceEntry<TestEvent>]) -> CheckpointAnchor {
        let root = compute_root_hash(entries);
        let last_gseq = entries.iter().map(|e| e.global_sequence).max().unwrap_or(0);
        CheckpointAnchor {
            version: ANCHOR_FORMAT_VERSION,
            checkpoint_sequence: 1,
            workspace_id: "test-ws".into(),
            created_at: "2026-06-14T12:00:00Z".into(),
            trace: CheckpointTraceState {
                entry_count: entries.len() as u64,
                last_global_sequence: last_gseq,
                root_hash: root,
            },
            hash_algorithm: "blake3".into(),
            anchor_source: "test".into(),
        }
    }

    // ── compute_root_hash tests ──

    #[test]
    fn root_hash_is_deterministic() {
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
            make_entry(3, "hash_c"),
        ];
        let h1 = compute_root_hash(&entries);
        let h2 = compute_root_hash(&entries);
        assert_eq!(h1, h2);
    }

    #[test]
    fn root_hash_changes_on_entry_modification() {
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];
        let original = compute_root_hash(&entries);

        let mut tampered = entries.clone();
        tampered[1].entry_hash = EntryHash("hash_TAMPERED".into());
        let modified = compute_root_hash(&tampered);

        assert_ne!(original, modified, "root hash must change when entry_hash is modified");
    }

    #[test]
    fn root_hash_changes_on_insertion() {
        let entries = vec![make_entry(1, "hash_a")];
        let original = compute_root_hash(&entries);

        let extended = vec![make_entry(1, "hash_a"), make_entry(2, "hash_b")];
        let grown = compute_root_hash(&extended);

        assert_ne!(original, grown, "root hash must change on insertion");
    }

    #[test]
    fn root_hash_changes_on_deletion() {
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];
        let original = compute_root_hash(&entries);

        let reduced = vec![make_entry(1, "hash_a")];
        let shrunk = compute_root_hash(&reduced);

        assert_ne!(original, shrunk, "root hash must change on deletion");
    }

    #[test]
    fn root_hash_independent_of_input_order() {
        let e1 = make_entry(1, "hash_a");
        let e2 = make_entry(2, "hash_b");
        let e3 = make_entry(3, "hash_c");

        let ordered = compute_root_hash(&[e1.clone(), e2.clone(), e3.clone()]);
        let shuffled = compute_root_hash(&[e3, e1, e2]);

        assert_eq!(ordered, shuffled, "root hash must sort internally by global_sequence");
    }

    #[test]
    fn root_hash_empty_entries() {
        let entries: Vec<TraceEntry<TestEvent>> = vec![];
        let hash = compute_root_hash(&entries);
        assert!(hash.starts_with("blake3:"), "empty root hash should still have blake3 prefix");
    }

    // ── compute_root_hash_prefix tests ──

    #[test]
    fn prefix_hash_only_covers_entries_up_to_checkpoint() {
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
            make_entry(3, "hash_c"),  // appended after checkpoint
        ];

        // Prefix covers only entries 1 and 2
        let (prefix_hash, count) = compute_root_hash_prefix(&entries, 2);
        assert_eq!(count, 2);

        // Should match root over just entries 1 and 2
        let just_prefix = compute_root_hash(&[entries[0].clone(), entries[1].clone()]);
        assert_eq!(prefix_hash, just_prefix);
    }

    // ── verify_anchor tests ──

    #[test]
    fn verify_anchor_pass_current() {
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];
        let anchor = make_anchor(&entries);

        let report = verify_anchor(&entries, Some(&anchor));
        assert_eq!(report.result, AnchorVerificationResult::Pass);
        assert_eq!(report.freshness, AnchorFreshness::Current);
    }

    #[test]
    fn verify_anchor_pass_stale_after_append() {
        // Entries at checkpoint time
        let checkpoint_entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];
        let anchor = make_anchor(&checkpoint_entries);

        // Trace grows: entries appended after checkpoint
        let mut grown = checkpoint_entries.clone();
        grown.push(make_entry(3, "hash_c"));
        grown.push(make_entry(4, "hash_d"));

        let report = verify_anchor(&grown, Some(&anchor));
        assert_eq!(report.result, AnchorVerificationResult::Pass,
            "anchor should remain valid after append-only growth");
        assert_eq!(report.freshness, AnchorFreshness::Stale { additional_entries: 2 });
    }

    #[test]
    fn verify_anchor_fail_on_modification_within_prefix() {
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];
        let anchor = make_anchor(&entries);

        // Tamper: modify an entry within the checkpoint prefix
        let mut tampered = entries.clone();
        tampered[1].entry_hash = EntryHash("hash_EVIL".into());

        let report = verify_anchor(&tampered, Some(&anchor));
        assert_eq!(report.result, AnchorVerificationResult::Fail,
            "modification within checkpoint prefix must Fail");
    }

    #[test]
    fn verify_anchor_fail_on_deletion_within_prefix() {
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
            make_entry(3, "hash_c"),
        ];
        let anchor = make_anchor(&entries);

        // Delete an entry within the checkpoint range
        let reduced = vec![entries[0].clone(), entries[2].clone()];

        let report = verify_anchor(&reduced, Some(&anchor));
        assert_eq!(report.result, AnchorVerificationResult::Fail,
            "deletion within checkpoint prefix must Fail");
    }

    #[test]
    fn verify_anchor_fail_on_insertion_within_prefix() {
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];
        let anchor = make_anchor(&entries);

        // Insert a new entry within the checkpoint range
        let mut modified = entries.clone();
        modified.push(make_entry(2, "hash_inserted")); // same global_seq

        let report = verify_anchor(&modified, Some(&anchor));
        // This either fails on count mismatch or root hash mismatch
        assert_eq!(report.result, AnchorVerificationResult::Fail);
    }

    #[test]
    fn verify_anchor_missing_anchor() {
        let entries = vec![make_entry(1, "hash_a")];
        let report = verify_anchor(&entries, None);
        assert_eq!(report.result, AnchorVerificationResult::MissingAnchor);
    }

    #[test]
    fn verify_anchor_unsupported_version() {
        let entries = vec![make_entry(1, "hash_a")];
        let mut anchor = make_anchor(&entries);
        anchor.version = 99;

        let report = verify_anchor(&entries, Some(&anchor));
        assert_eq!(report.result, AnchorVerificationResult::Unsupported);
    }

    #[test]
    fn verify_anchor_unsupported_root_hash_prefix() {
        let entries = vec![make_entry(1, "hash_a")];
        let mut anchor = make_anchor(&entries);
        anchor.trace.root_hash = "sha256:abc123".into(); // wrong prefix

        let report = verify_anchor(&entries, Some(&anchor));
        assert_eq!(report.result, AnchorVerificationResult::Unsupported);
    }

    #[test]
    fn verify_anchor_tamper_after_checkpoint_not_covered_but_anchor_still_valid() {
        // Entries at checkpoint time
        let checkpoint_entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];
        let anchor = make_anchor(&checkpoint_entries);

        // After checkpoint, add then tamper an entry
        let mut grown = checkpoint_entries.clone();
        grown.push(make_entry(3, "hash_c"));
        // Tamper the post-checkpoint entry
        grown[2].entry_hash = EntryHash("hash_EVIL".into());

        let report = verify_anchor(&grown, Some(&anchor));
        // Anchor covers only entries 1-2. Entry 3 tamper is outside coverage.
        // So the anchor verification should still Pass (stale).
        assert_eq!(report.result, AnchorVerificationResult::Pass,
            "tamper after checkpoint is outside anchor coverage");
        assert_eq!(report.freshness, AnchorFreshness::Stale { additional_entries: 1 });
    }

    #[test]
    fn verify_anchor_fully_consistent_tamper_not_detected() {
        // KNOWN LIMITATION: An attacker who modifies entries AND recomputes
        // both entry_hash values AND the external anchor produces a trace
        // that passes anchor verification.
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];

        // Attacker modifies entry 2 and recomputes everything
        let mut tampered = entries.clone();
        tampered[1].entry_hash = EntryHash("hash_recomputed_consistently".into());

        // Attacker creates a new anchor matching the tampered trace
        let fake_anchor = make_anchor(&tampered);

        let report = verify_anchor(&tampered, Some(&fake_anchor));
        assert_eq!(report.result, AnchorVerificationResult::Pass,
            "fully consistent tamper passes — documented limitation (requires external trust anchor)");
    }

    #[test]
    fn verify_anchor_current_count_less_than_anchor_fails() {
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
            make_entry(3, "hash_c"),
        ];
        let anchor = make_anchor(&entries);

        // Remove an entry (trace shrank)
        let reduced = vec![entries[0].clone(), entries[1].clone()];

        let report = verify_anchor(&reduced, Some(&anchor));
        assert_eq!(report.result, AnchorVerificationResult::Fail);
    }

    #[test]
    fn verify_anchor_report_contains_recomputed_hash() {
        let entries = vec![make_entry(1, "hash_a"), make_entry(2, "hash_b")];
        let anchor = make_anchor(&entries);

        let report = verify_anchor(&entries, Some(&anchor));
        assert!(report.recomputed_root_hash.is_some(),
            "Pass report should include recomputed root hash");
        assert_eq!(
            report.recomputed_root_hash.as_ref().unwrap(),
            &anchor.trace.root_hash
        );
    }

    // ── Authority guard test ──

    #[test]
    fn verify_anchor_is_read_only() {
        // Prove that verify_anchor does not mutate the entries
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];
        let hash_before: Vec<String> = entries.iter().map(|e| e.entry_hash.0.clone()).collect();
        let gseq_before: Vec<u64> = entries.iter().map(|e| e.global_sequence).collect();
        let kind_before: Vec<String> = entries.iter().map(|e| e.event_kind.clone()).collect();
        let anchor = make_anchor(&entries);

        let _ = verify_anchor(&entries, Some(&anchor));

        let hash_after: Vec<String> = entries.iter().map(|e| e.entry_hash.0.clone()).collect();
        let gseq_after: Vec<u64> = entries.iter().map(|e| e.global_sequence).collect();
        let kind_after: Vec<String> = entries.iter().map(|e| e.event_kind.clone()).collect();

        assert_eq!(hash_before, hash_after, "verify_anchor must not mutate entry hashes");
        assert_eq!(gseq_before, gseq_after, "verify_anchor must not mutate global_sequence");
        assert_eq!(kind_before, kind_after, "verify_anchor must not mutate event_kind");
    }

    // ── Source-level authority guard ──

    #[test]
    fn anchor_module_does_not_mutate_trace() {
        let src = include_str!("anchor.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!impl_only.contains("append_trace"),
            "anchor module must not append to trace");
        assert!(!impl_only.contains("delete_entry") && !impl_only.contains("remove_entry"),
            "anchor module must not delete trace entries");
    }

    #[test]
    fn anchor_module_does_not_import_backend() {
        let src = include_str!("anchor.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!impl_only.contains("openwand_store"),
            "anchor module must not import openwand-store (backend coupling)");
        assert!(!impl_only.contains("openwand_core"),
            "anchor module must not import openwand-core (domain coupling)");
        assert!(!impl_only.contains("openwand_session"),
            "anchor module must not import openwand-session");
    }

    // ── Wave 104B: Writer and path containment tests ──

    use std::path::PathBuf;

    fn make_temp_dirs() -> (tempfile::TempDir, tempfile::TempDir) {
        (
            tempfile::TempDir::new().unwrap(),
            tempfile::TempDir::new().unwrap(),
        )
    }

    #[test]
    fn validate_anchor_root_rejects_equality() {
        let (anchor, _store) = make_temp_dirs();
        let err = validate_anchor_root(anchor.path(), anchor.path()).unwrap_err();
        assert_eq!(err, AnchorError::AnchorRootEqualsStoreRoot);
    }

    #[test]
    fn validate_anchor_root_rejects_anchor_inside_store() {
        let store = tempfile::TempDir::new().unwrap();
        let anchor_inside = store.path().join("subdir");
        std::fs::create_dir_all(&anchor_inside).unwrap();
        let err = validate_anchor_root(&anchor_inside, store.path()).unwrap_err();
        assert_eq!(err, AnchorError::AnchorRootInsideStoreRoot);
    }

    #[test]
    fn validate_anchor_root_rejects_store_inside_anchor() {
        let anchor = tempfile::TempDir::new().unwrap();
        let store_inside = anchor.path().join("store");
        std::fs::create_dir_all(&store_inside).unwrap();
        let err = validate_anchor_root(anchor.path(), &store_inside).unwrap_err();
        assert_eq!(err, AnchorError::StoreRootInsideAnchorRoot);
    }

    #[test]
    fn validate_anchor_root_accepts_separate_paths() {
        let (anchor, store) = make_temp_dirs();
        let result = validate_anchor_root(anchor.path(), store.path());
        assert!(result.is_ok(), "separate canonicalized paths should be accepted");
    }

    #[test]
    fn validate_anchor_root_rejects_nonexistent_path() {
        let anchor = tempfile::TempDir::new().unwrap();
        let store = tempfile::TempDir::new().unwrap();
        let nonexistent = anchor.path().join("does-not-exist");
        let err = validate_anchor_root(&nonexistent, store.path()).unwrap_err();
        assert!(matches!(err, AnchorError::CanonicalizationFailed(_)));
    }

    #[test]
    fn writer_creates_anchor_file() {
        let (anchor, store) = make_temp_dirs();
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];

        let path = CheckpointWriter::write_checkpoint(
            &entries,
            anchor.path(),
            store.path(),
            "test-ws",
            1,
        ).unwrap();

        assert!(path.exists(), "anchor file should exist");
        assert!(path.file_name().unwrap().to_str().unwrap().contains("checkpoint-1"));
    }

    #[test]
    fn writer_anchor_file_is_valid_json() {
        let (anchor, store) = make_temp_dirs();
        let entries = vec![make_entry(1, "hash_a")];

        let path = CheckpointWriter::write_checkpoint(
            &entries, anchor.path(), store.path(), "ws1", 1,
        ).unwrap();

        let loaded = read_anchor_file(&path).unwrap();
        assert_eq!(loaded.version, ANCHOR_FORMAT_VERSION);
        assert_eq!(loaded.workspace_id, "ws1");
        assert_eq!(loaded.trace.entry_count, 1);
        assert!(loaded.trace.root_hash.starts_with("blake3:"));
    }

    #[test]
    fn writer_written_anchor_roundtrips_through_verify() {
        let (anchor, store) = make_temp_dirs();
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];

        let path = CheckpointWriter::write_checkpoint(
            &entries, anchor.path(), store.path(), "ws1", 1,
        ).unwrap();

        let loaded_anchor = read_anchor_file(&path).unwrap();

        let report = verify_anchor(&entries, Some(&loaded_anchor));
        assert_eq!(report.result, AnchorVerificationResult::Pass);
        assert_eq!(report.freshness, AnchorFreshness::Current);
    }

    #[test]
    fn writer_rejects_collision() {
        let (anchor, store) = make_temp_dirs();
        let entries = vec![make_entry(1, "hash_a")];

        CheckpointWriter::write_checkpoint(
            &entries, anchor.path(), store.path(), "ws1", 1,
        ).unwrap();

        let err = CheckpointWriter::write_checkpoint(
            &entries, anchor.path(), store.path(), "ws1", 1,
        ).unwrap_err();
        assert_eq!(err, AnchorError::CheckpointSequenceCollision(1));
    }

    #[test]
    fn writer_rejects_anchor_inside_store() {
        let store = tempfile::TempDir::new().unwrap();
        let anchor_inside = store.path().join("anchors");
        std::fs::create_dir_all(&anchor_inside).unwrap();
        let entries = vec![make_entry(1, "hash_a")];

        let err = CheckpointWriter::write_checkpoint(
            &entries, &anchor_inside, store.path(), "ws1", 1,
        ).unwrap_err();
        assert_eq!(err, AnchorError::AnchorRootInsideStoreRoot);
    }

    #[test]
    fn next_sequence_empty_returns_1() {
        let anchor = tempfile::TempDir::new().unwrap();
        let seq = CheckpointWriter::next_sequence(anchor.path()).unwrap();
        assert_eq!(seq, 1);
    }

    #[test]
    fn next_sequence_after_existing_returns_max_plus_1() {
        let (anchor, store) = make_temp_dirs();
        let entries = vec![make_entry(1, "hash_a")];

        CheckpointWriter::write_checkpoint(
            &entries, anchor.path(), store.path(), "ws1", 1,
        ).unwrap();
        CheckpointWriter::write_checkpoint(
            &entries, anchor.path(), store.path(), "ws1", 3,
        ).unwrap();

        let seq = CheckpointWriter::next_sequence(anchor.path()).unwrap();
        assert_eq!(seq, 4, "next sequence should be max(1,3)+1 = 4");
    }

    #[test]
    fn next_sequence_ignores_non_anchor_files() {
        let anchor = tempfile::TempDir::new().unwrap();
        std::fs::write(anchor.path().join("random.json"), "{}").unwrap();
        std::fs::write(anchor.path().join("openwand-checkpoint-5.json"), "{}").unwrap();
        std::fs::write(anchor.path().join("openwand-checkpoint-not-a-number.json"), "{}").unwrap();

        let seq = CheckpointWriter::next_sequence(anchor.path()).unwrap();
        assert_eq!(seq, 6, "should find 5 and return 6");
    }

    #[test]
    fn read_anchor_file_rejects_nonexistent() {
        let path = PathBuf::from("/nonexistent/anchor.json");
        let err = read_anchor_file(&path).unwrap_err();
        assert!(matches!(err, AnchorError::IoError(_)));
    }

    #[test]
    fn read_anchor_file_rejects_malformed_json() {
        let anchor = tempfile::TempDir::new().unwrap();
        let path = anchor.path().join("bad.json");
        std::fs::write(&path, "not valid json {{{").unwrap();
        let err = read_anchor_file(&path).unwrap_err();
        assert!(matches!(err, AnchorError::ParseError(_)));
    }

    #[test]
    fn writer_full_workflow_write_then_verify_after_growth() {
        let (anchor, store) = make_temp_dirs();
        let checkpoint_entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];

        let path = CheckpointWriter::write_checkpoint(
            &checkpoint_entries, anchor.path(), store.path(), "ws1", 1,
        ).unwrap();
        let anchor_obj = read_anchor_file(&path).unwrap();

        let mut grown = checkpoint_entries.clone();
        grown.push(make_entry(3, "hash_c"));

        let report = verify_anchor(&grown, Some(&anchor_obj));
        assert_eq!(report.result, AnchorVerificationResult::Pass);
        assert_eq!(report.freshness, AnchorFreshness::Stale { additional_entries: 1 });
    }

    #[test]
    fn writer_full_workflow_detects_tamper() {
        let (anchor, store) = make_temp_dirs();
        let entries = vec![
            make_entry(1, "hash_a"),
            make_entry(2, "hash_b"),
        ];

        let path = CheckpointWriter::write_checkpoint(
            &entries, anchor.path(), store.path(), "ws1", 1,
        ).unwrap();
        let anchor_obj = read_anchor_file(&path).unwrap();

        let mut tampered = entries.clone();
        tampered[1].entry_hash = crate::stream::EntryHash("hash_EVIL".into());

        let report = verify_anchor(&tampered, Some(&anchor_obj));
        assert_eq!(report.result, AnchorVerificationResult::Fail);
    }
}
