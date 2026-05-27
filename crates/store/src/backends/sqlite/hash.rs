//! BLAKE3 entry hash computation for trace entries.
//!
//! Production hash chain. The in-memory store used a deterministic placeholder;
//! this module provides the real BLAKE3 implementation.

use openwand_trace::stream::EntryHash;

/// Compute BLAKE3 hash of entry content for the hash chain.
/// Input: the stable fields that define an entry's identity.
pub fn compute_entry_hash(
    global_sequence: u64,
    stream_scope: &str,
    stream_id: &str,
    stream_sequence: u64,
    event_kind: &str,
    event_payload_json: &str,
    prev_hash: Option<&EntryHash>,
) -> EntryHash {
    let mut hasher = blake3::Hasher::new();

    // Stable fields — never change these without a schema version bump
    hasher.update(&global_sequence.to_le_bytes());
    hasher.update(stream_scope.as_bytes());
    hasher.update(stream_id.as_bytes());
    hasher.update(&stream_sequence.to_le_bytes());
    hasher.update(event_kind.as_bytes());
    hasher.update(event_payload_json.as_bytes());

    if let Some(prev) = prev_hash {
        hasher.update(prev.0.as_bytes());
    }

    let hash = hasher.finalize();
    EntryHash(hash.to_hex().to_string())
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
        let h1 = compute_entry_hash(
            1,
            "Session",
            "s-main",
            1,
            "session.started",
            "{}",
            None,
        );
        let h2 = compute_entry_hash(
            2,
            "Session",
            "s-main",
            2,
            "session.started",
            "{}",
            None,
        );
        assert_ne!(h1, h2, "Different sequences must produce different hashes");
    }

    #[test]
    fn hash_changes_with_prev_hash() {
        let h1 = compute_entry_hash(
            2,
            "Session",
            "s-main",
            2,
            "tool.called",
            "{}",
            None,
        );
        let h2 = compute_entry_hash(
            2,
            "Session",
            "s-main",
            2,
            "tool.called",
            "{}",
            Some(&EntryHash("prev_hash_value".into())),
        );
        assert_ne!(h1, h2, "prev_hash must participate in the hash");
    }

    #[test]
    fn hash_is_64_hex_chars() {
        let h = compute_entry_hash(1, "G", "s", 1, "e", "{}", None);
        assert_eq!(64, h.0.len(), "BLAKE3 hex output is 64 chars");
    }
}
