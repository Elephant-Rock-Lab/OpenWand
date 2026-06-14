//! BLAKE3 entry hash computation for trace entries.
//!
//! Wave 98A: The canonical BLAKE3 hash function now lives in `openwand-trace`
//! as `Blake3HashPolicy::compute_hash()`. This module re-exports it for
//! backward compatibility with existing store code.

pub use openwand_trace::verifier::Blake3HashPolicy;

/// Compute BLAKE3 hash of entry content for the hash chain.
/// Delegates to the canonical implementation in `openwand-trace`.
///
/// Input: the stable fields that define an entry's identity.
pub fn compute_entry_hash(
    global_sequence: u64,
    stream_scope: &str,
    stream_id: &str,
    stream_sequence: u64,
    event_kind: &str,
    event_payload_json: &str,
    prev_hash: Option<&openwand_trace::EntryHash>,
) -> openwand_trace::EntryHash {
    Blake3HashPolicy::compute_hash(
        global_sequence,
        stream_scope,
        stream_id,
        stream_sequence,
        event_kind,
        event_payload_json,
        prev_hash,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        let h1 = compute_entry_hash(
            1,
            "Session",
            "s-main",
            1,
            "session.started",
            r#"{"session_id":"abc"}"#,
            None,
        );
        let h2 = compute_entry_hash(
            1,
            "Session",
            "s-main",
            1,
            "session.started",
            r#"{"session_id":"abc"}"#,
            None,
        );
        assert_eq!(h1, h2, "Same inputs must produce same hash");
    }

    #[test]
    fn hash_changes_with_sequence() {
        let h1 = compute_entry_hash(1, "Session", "s-main", 1, "session.started", "{}", None);
        let h2 = compute_entry_hash(2, "Session", "s-main", 2, "session.started", "{}", None);
        assert_ne!(h1, h2, "Different sequences must produce different hashes");
    }

    #[test]
    fn hash_changes_with_prev_hash() {
        let h1 = compute_entry_hash(2, "Session", "s-main", 2, "tool.called", "{}", None);
        let h2 = compute_entry_hash(
            2,
            "Session",
            "s-main",
            2,
            "tool.called",
            "{}",
            Some(&openwand_trace::EntryHash("prev_hash_value".into())),
        );
        assert_ne!(h1, h2, "prev_hash must participate in the hash");
    }

    #[test]
    fn hash_is_64_hex_chars() {
        let h = compute_entry_hash(1, "G", "s", 1, "e", "{}", None);
        assert_eq!(64, h.0.len(), "BLAKE3 hex output is 64 chars");
    }
}
