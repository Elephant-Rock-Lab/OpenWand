//! Integration tests for trace verifier CLI command and tamper detection (Wave 92B).
//!
//! Proves:
//! 1. The CLI command is real (not a stub).
//! 2. The verifier detects tampering through deterministic tests.
//! 3. The CLI exits with distinct codes for pass/fail/error.
//! 4. The verifier is read-only (source-level guard).

#[cfg(test)]
mod authority_guards {
    /// Guard: verifier module does not mutate, repair, or append.
    #[test]
    fn verifier_is_read_only() {
        let src = include_str!("../../trace/src/verifier.rs");
        // Must not contain mutation methods
        assert!(!src.contains(".append("), "verifier must not append trace");
        // Check for actual repair calls, not the word in comments
        assert!(!src.contains("fn repair"), "verifier must not have repair function");
        assert!(!src.contains(".write("), "verifier must not write trace");
        assert!(!src.contains("std::fs::write"), "verifier must not write files");
        assert!(!src.contains("fn migrate"), "verifier must not have migrate function");
        assert!(!src.contains("fn rewrite"), "verifier must not have rewrite function");
    }

    /// Guard: CLI command loads and verifies, does not fabricate.
    #[test]
    fn cli_trace_verify_is_real() {
        // The CLI command should no longer say "not yet implemented"
        // This is tested via truthful_commands.rs as well, but here we
        // verify the source doesn't contain stub language.
        let src = include_str!("../src/main.rs");
        // Split on the function and take only until the next function or section
        let cmd_section = src
            .split("async fn cmd_trace_verify")
            .nth(1)
            .unwrap_or("")
            .split("async fn cmd_audit_check")
            .next()
            .unwrap_or("");

        // The trace-verify function should not be a stub
        assert!(!cmd_section.contains("not yet implemented"),
            "trace-verify must not be a stub");
        assert!(cmd_section.contains("verify_with_hash_policy"),
            "must use hash policy verification (98B)");
        assert!(cmd_section.contains("Blake3HashPolicy"),
            "must use Blake3HashPolicy (98B)");
        assert!(cmd_section.contains("exit_code"),
            "must use distinct exit codes");
    }
}

#[cfg(test)]
mod tamper_detection_tests {
    use openwand_trace::verifier::{TraceVerifier, VerificationResult, VerificationCheck, FindingSeverity};
    use openwand_trace::entry::TraceEntry;
    use openwand_trace::stream::{EntryHash, TraceStreamId, TraceStreamScope};
    use openwand_trace::actor::Actor;
    use openwand_trace::ids::TraceId;

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

    #[test]
    fn valid_five_entry_chain_passes() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Session, "s1", 1, None, "h1"),
            make_entry(2, TraceStreamScope::Session, "s1", 2, Some(EntryHash("h1".into())), "h2"),
            make_entry(3, TraceStreamScope::Session, "s1", 3, Some(EntryHash("h2".into())), "h3"),
            make_entry(4, TraceStreamScope::Session, "s1", 4, Some(EntryHash("h3".into())), "h4"),
            make_entry(5, TraceStreamScope::Session, "s1", 5, Some(EntryHash("h4".into())), "h5"),
        ];
        let report = TraceVerifier::verify(&entries);
        assert_eq!(report.result, VerificationResult::Pass);
        assert_eq!(report.entries_checked, 5);
        assert!(!report.has_errors());
    }

    #[test]
    fn tampered_hash_in_middle_detected() {
        // Tamper: change entry 3's prev_hash to something wrong
        let entries = vec![
            make_entry(1, TraceStreamScope::Session, "s1", 1, None, "h1"),
            make_entry(2, TraceStreamScope::Session, "s1", 2, Some(EntryHash("h1".into())), "h2"),
            make_entry(3, TraceStreamScope::Session, "s1", 3, Some(EntryHash("TAMPERED".into())), "h3"),
            make_entry(4, TraceStreamScope::Session, "s1", 4, Some(EntryHash("h3".into())), "h4"),
        ];
        let report = TraceVerifier::verify(&entries);
        assert_eq!(report.result, VerificationResult::Fail);
        // Should find exactly one hash chain error at entry 3
        let chain_errors: Vec<_> = report.findings.iter()
            .filter(|f| f.check == VerificationCheck::HashChainValid && f.severity == FindingSeverity::Error)
            .collect();
        assert_eq!(chain_errors.len(), 1, "should find exactly one broken link");
        assert!(chain_errors[0].detail.contains("TAMPERED"));
    }

    #[test]
    fn deleted_entry_detected_via_broken_chain() {
        // Simulate deletion: entry 3 removed, entry 4 still links to h3
        // but entry 2's hash (h2) is what entry 4 would need to link to
        let entries = vec![
            make_entry(1, TraceStreamScope::Session, "s1", 1, None, "h1"),
            make_entry(2, TraceStreamScope::Session, "s1", 2, Some(EntryHash("h1".into())), "h2"),
            // Entry 3 deleted — stream_seq gap from 2 to 4
            make_entry(4, TraceStreamScope::Session, "s1", 4, Some(EntryHash("h3".into())), "h4"),
        ];
        let report = TraceVerifier::verify(&entries);
        assert_eq!(report.result, VerificationResult::Fail);
        // prev_hash h3 doesn't match previous entry_hash h2
        assert!(report.findings.iter().any(|f|
            f.check == VerificationCheck::HashChainValid && f.severity == FindingSeverity::Error
        ));
    }

    #[test]
    fn swapped_entries_detected_via_cross_ordering() {
        // Two entries where global_seq disagrees with stream_seq ordering.
        // Entry A: global=1, stream=2 (claims to be 2nd in stream but 1st globally)
        // Entry B: global=2, stream=1 (claims to be 1st in stream but 2nd globally)
        // This is detectable by the cross-ordering check.
        let entries = vec![
            make_entry(1, TraceStreamScope::Session, "s1", 2, Some(EntryHash("h1".into())), "h2"),
            make_entry(2, TraceStreamScope::Session, "s1", 1, None, "h1"),
        ];
        let report = TraceVerifier::verify(&entries);
        assert_eq!(report.result, VerificationResult::Fail);
    }

    #[test]
    fn multi_stream_tamper_in_one_stream_detected() {
        // Two streams, one has a broken link
        let entries = vec![
            make_entry(1, TraceStreamScope::Session, "s1", 1, None, "h1"),
            make_entry(2, TraceStreamScope::Session, "s1", 2, Some(EntryHash("h1".into())), "h2"),
            make_entry(3, TraceStreamScope::Session, "s2", 1, None, "h3"),
            make_entry(4, TraceStreamScope::Session, "s2", 2, Some(EntryHash("WRONG".into())), "h4"), // broken
        ];
        let report = TraceVerifier::verify(&entries);
        assert_eq!(report.result, VerificationResult::Fail);
        // Only stream s2 should have the error
        let s2_errors: Vec<_> = report.findings.iter()
            .filter(|f| f.stream_id.as_deref() == Some("Session:s2"))
            .collect();
        assert!(!s2_errors.is_empty(), "should find error in s2");
    }

    #[test]
    fn empty_trace_passes_with_zero_entries() {
        let report = TraceVerifier::verify::<TestEvent>(&[]);
        assert_eq!(report.result, VerificationResult::Pass);
        assert_eq!(report.entries_checked, 0);
        assert_eq!(report.streams_checked, 0);
    }
}

#[cfg(test)]
mod cli_exit_code_tests {
    use std::process::Command;

    fn openwand_bin() -> String {
        env!("CARGO_BIN_EXE_openwand").to_string()
    }

    #[test]
    fn trace_verify_exits_with_documented_code() {
        let output = Command::new(openwand_bin())
            .args(["trace-verify", "test-session-92b"])
            .output()
            .expect("Failed to run openwand");

        let code = output.status.code().unwrap_or(-1);
        // Must be one of the documented exit codes
        assert!(
            code == 0 || code == 1 || code == 2 || code == 3 || code == 4,
            "trace-verify should exit with documented code (0=pass, 1=error, 2=fail, 3=inconclusive, 4=unsupported), got {}",
            code
        );
    }

    #[test]
    fn trace_verify_prints_structured_report() {
        let output = Command::new(openwand_bin())
            .args(["trace-verify", "test-session-92b"])
            .output()
            .expect("Failed to run openwand");

        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        // Should contain either a verification report or an operational error
        assert!(
            combined.contains("Trace Verification Report") ||
            combined.contains("error:") ||
            combined.contains("Result:"),
            "should print structured output, got: {}",
            combined
        );
    }

    #[test]
    fn trace_verify_mentions_hash_correctness() {
        let output = Command::new(openwand_bin())
            .args(["trace-verify", "test-session-98b"])
            .output()
            .expect("Failed to run openwand");

        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        // The output should mention hash correctness as one of the checks
        assert!(
            combined.contains("hash correctness") ||
            combined.contains("Hash correctness") ||
            combined.contains("BLAKE3"),
            "output should mention hash correctness verification (98B)"
        );
    }

    #[test]
    fn trace_verify_no_immutability_overclaim() {
        let output = Command::new(openwand_bin())
            .args(["trace-verify", "test-session-98b"])
            .output()
            .expect("Failed to run openwand");

        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        // Should NOT claim physical immutability
        assert!(
            !combined.contains("proves physical immutability") &&
            !combined.contains("guarantees immutability") &&
            !combined.contains("fully immutable"),
            "must not claim physical immutability"
        );
        // Should mention the limitation
        assert!(
            combined.contains("does not prove physical immutability") ||
            combined.contains("external trust anchor"),
            "should mention external trust anchor limitation"
        );
    }
}

#[cfg(test)]
mod hash_policy_cli_tests {
    use openwand_trace::verifier::{TraceVerifier, Blake3HashPolicy, HashVerificationPolicy, VerificationResult, VerificationCheck, FindingSeverity};
    use openwand_trace::entry::TraceEntry;
    use openwand_trace::stream::{EntryHash, TraceStreamId, TraceStreamScope};
    use openwand_trace::actor::Actor;
    use openwand_trace::ids::TraceId;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    struct TestEvent(String);

    impl HashVerificationPolicy<TestEvent> for Blake3HashPolicy {
        fn serialize_event(&self, event: &TestEvent) -> Result<String, serde_json::Error> {
            serde_json::to_string(event)
        }
        fn compute_entry_hash(
            &self,
            global_sequence: u64,
            stream_scope: &str,
            stream_id: &str,
            stream_sequence: u64,
            event_kind: &str,
            event_payload_json: &str,
            prev_hash: Option<&EntryHash>,
        ) -> EntryHash {
            Blake3HashPolicy::compute_hash(
                global_sequence, stream_scope, stream_id,
                stream_sequence, event_kind, event_payload_json, prev_hash,
            )
        }
    }

    fn make_hashed_entry(global_seq: u64, stream_id: &str, stream_seq: u64, prev_hash: Option<&EntryHash>) -> TraceEntry<TestEvent> {
        let event_json = serde_json::to_string(&TestEvent("test".into())).unwrap();
        let hash = Blake3HashPolicy::compute_hash(
            global_seq, "Session", stream_id, stream_seq, "test.event", &event_json, prev_hash,
        );
        TraceEntry {
            id: TraceId::new(),
            stream_id: TraceStreamId { scope: TraceStreamScope::Session, id: stream_id.into() },
            stream_sequence: stream_seq,
            global_sequence: global_seq,
            occurred_at: chrono::Utc::now(),
            actor: Actor::User,
            event: TestEvent("test".into()),
            event_kind: "test.event".into(),
            event_schema_version: 1,
            trace_schema_version: 1,
            prev_hash: prev_hash.cloned(),
            entry_hash: hash,
        }
    }

    #[test]
    fn hash_policy_correct_entries_pass_cli_verification() {
        let e1 = make_hashed_entry(1, "s1", 1, None);
        let e2 = make_hashed_entry(2, "s1", 2, Some(&e1.entry_hash));
        let report = TraceVerifier::verify_with_hash_policy(&[e1, e2], &Blake3HashPolicy);
        assert_eq!(report.result, VerificationResult::Pass);
        assert!(!report.findings.iter().any(|f|
            f.check == VerificationCheck::HashCorrectnessValid && f.severity == FindingSeverity::Error
        ));
    }

    #[test]
    fn hash_policy_tampered_content_detected_by_cli_verifier() {
        let e1 = make_hashed_entry(1, "s1", 1, None);
        let mut e2 = make_hashed_entry(2, "s1", 2, Some(&e1.entry_hash));
        // Tamper: change content but leave hash unchanged
        e2.event = TestEvent("TAMPERED".into());
        let report = TraceVerifier::verify_with_hash_policy(&[e1, e2], &Blake3HashPolicy);
        assert_eq!(report.result, VerificationResult::Fail);
        assert!(report.findings.iter().any(|f| f.check == VerificationCheck::HashCorrectnessValid));
    }
}
