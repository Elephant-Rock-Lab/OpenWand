//! Integration tests for operation-replay CLI command (Wave 93B).

#[cfg(test)]
mod authority_guards {
    #[test]
    fn cli_operation_replay_is_real() {
        let src = include_str!("../src/main.rs");
        let section = src.split("async fn cmd_operation_replay").nth(1).unwrap_or("").split("async fn cmd_audit_check").next().unwrap_or("");
        assert!(!section.contains("not yet implemented"), "must not be stub");
        assert!(section.contains("OperationReplayVerifier::verify"), "must call verifier");
    }

    #[test]
    fn cli_operation_replay_is_read_only() {
        let src = include_str!("../src/main.rs");
        let section = src.split("async fn cmd_operation_replay").nth(1).unwrap_or("").split("async fn cmd_audit_check").next().unwrap_or("");
        assert!(!section.contains("resolve_approval"), "must not resolve");
        assert!(!section.contains("export_audit_packet"), "must not export");
    }
}

#[cfg(test)]
mod cli_behavior_tests {
    use std::process::Command;
    fn bin() -> String { env!("CARGO_BIN_EXE_openwand").to_string() }

    #[test] fn missing_file_exits_1() {
        let o = Command::new(bin()).args(["operation-replay", "--session", "t", "--operations", "/nonexistent"]).output().unwrap();
        assert_eq!(o.status.code().unwrap_or(-1), 1);
    }
    #[test] fn malformed_json_exits_1() {
        let t = std::env::temp_dir().join("93b_bad.json"); std::fs::write(&t, "{ bad }").unwrap();
        let o = Command::new(bin()).args(["operation-replay", "--session", "t", "--operations", t.to_str().unwrap()]).output().unwrap();
        assert_eq!(o.status.code().unwrap_or(-1), 1); let _ = std::fs::remove_file(&t);
    }
    #[test] fn empty_ops_exits_1() {
        let t = std::env::temp_dir().join("93b_empty.json"); std::fs::write(&t, r#"{"operations": []}"#).unwrap();
        let o = Command::new(bin()).args(["operation-replay", "--session", "t", "--operations", t.to_str().unwrap()]).output().unwrap();
        assert_eq!(o.status.code().unwrap_or(-1), 1); let _ = std::fs::remove_file(&t);
    }
    #[test] fn valid_input_exits_0_to_4() {
        let t = std::env::temp_dir().join("93b_valid.json");
        std::fs::write(&t, r#"{"operations": [{"type": "workflow_initiation", "workflow_execution_id": "w"}]}"#).unwrap();
        let o = Command::new(bin()).args(["operation-replay", "--session", "t", "--operations", t.to_str().unwrap()]).output().unwrap();
        let c = o.status.code().unwrap_or(-1); assert!(c >= 0 && c <= 4, "got {c}"); let _ = std::fs::remove_file(&t);
    }
    #[test] fn prints_report_or_error() {
        let t = std::env::temp_dir().join("93b_rep.json");
        std::fs::write(&t, r#"{"operations": [{"type": "approval_resolution", "approval_request_id": "a", "tool_call_id": "t"}]}"#).unwrap();
        let o = Command::new(bin()).args(["operation-replay", "--session", "t", "--operations", t.to_str().unwrap()]).output().unwrap();
        let s = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
        assert!(s.contains("Operation Replay Report") || s.contains("error:")); let _ = std::fs::remove_file(&t);
    }
    #[test] fn no_full_verification_claim() {
        let t = std::env::temp_dir().join("93b_claim.json");
        std::fs::write(&t, r#"{"operations": [{"type": "evidence_export", "workflow_execution_id": "w"}]}"#).unwrap();
        let o = Command::new(bin()).args(["operation-replay", "--session", "t", "--operations", t.to_str().unwrap()]).output().unwrap();
        let s = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
        assert!(!s.contains("full verification")); let _ = std::fs::remove_file(&t);
    }

    #[test]
    fn verifier_does_not_emit_trace() {
        let src = include_str!("../src/operation_audit.rs");
        let impl_only = src.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!impl_only.contains(".append("), "verifier must not append trace");
        assert!(!impl_only.contains("AppendTraceEntry"), "verifier must not construct trace entries");
    }
}
