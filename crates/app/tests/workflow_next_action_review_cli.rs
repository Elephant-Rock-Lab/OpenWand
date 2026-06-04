//! CLI tests for workflow next-action review and routing readiness.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}
fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

#[test]
fn cli_next_action_review_approve_outputs_review_id() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-next-action-review", "approve",
        "--proposal-id", "wnap_t", "--reviewer", "alice", "--rationale", "safe",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("approve");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("wnar_"), "Expected review ID: {}", stdout);
}

#[test]
fn cli_next_action_review_reject_requires_feedback() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-next-action-review", "reject",
        "--proposal-id", "wnap_t", "--reviewer", "alice", "--rationale", "no",
        "--feedback", "unsafe", "--output-dir"]).arg(d.path()).arg("--json").output().expect("reject");
    assert!(out.status.success());
}

#[test]
fn cli_next_action_review_request_changes_requires_feedback() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-next-action-review", "request-changes",
        "--proposal-id", "wnap_t", "--reviewer", "alice", "--rationale", "fix",
        "--feedback", "add evidence", "--output-dir"]).arg(d.path()).arg("--json").output().expect("changes");
    assert!(out.status.success());
}

#[test]
fn cli_next_action_review_show_roundtrips_record() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-next-action-review", "approve",
        "--proposal-id", "wnap_t", "--reviewer", "alice", "--rationale", "safe",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("approve");
    let rec: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let rid = rec["review_id"].as_str().unwrap();
    let show = Command::new(openwand_bin()).args(["workflow-next-action-review", "show", rid,
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("show");
    let shown: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&show.stdout)).unwrap();
    assert_eq!(rid, shown["review_id"].as_str().unwrap_or(""));
}

#[test]
fn cli_next_action_review_latest_by_proposal_returns_latest() {
    let d = temp_dir();
    Command::new(openwand_bin()).args(["workflow-next-action-review", "approve",
        "--proposal-id", "wnap_t", "--reviewer", "alice", "--rationale", "safe",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("approve");
    let out = Command::new(openwand_bin()).args(["workflow-next-action-review", "latest",
        "--proposal-id", "wnap_t", "--output-dir"]).arg(d.path()).arg("--json").output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wnar_"));
}

#[test]
fn cli_routing_readiness_evaluate_outputs_readiness_id() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-routing-readiness", "evaluate",
        "--proposal-id", "wnap_t", "--review-id", "wnar_t",
        "--workflow-execution-id", "wfx_t", "--source-run-revision-id", "wrr_t",
        "--expected-proposal-hash", "h", "--expected-run-revision-hash", "h",
        "--expected-review-hash", "h", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("evaluate");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("wrrd_"), "Expected readiness ID: {}", stdout);
}

#[test]
fn cli_routing_readiness_show_roundtrips_record() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-routing-readiness", "evaluate",
        "--proposal-id", "wnap_t", "--review-id", "wnar_t",
        "--workflow-execution-id", "wfx_t", "--source-run-revision-id", "wrr_t",
        "--expected-proposal-hash", "h", "--expected-run-revision-hash", "h",
        "--expected-review-hash", "h", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("evaluate");
    let rec: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let rid = rec["readiness_id"].as_str().unwrap();
    let show = Command::new(openwand_bin()).args(["workflow-routing-readiness", "show", rid,
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("show");
    let shown: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&show.stdout)).unwrap();
    assert_eq!(rid, shown["readiness_id"].as_str().unwrap_or(""));
}

#[test]
fn cli_routing_readiness_latest_by_proposal_returns_latest() {
    let d = temp_dir();
    Command::new(openwand_bin()).args(["workflow-routing-readiness", "evaluate",
        "--proposal-id", "wnap_t", "--review-id", "wnar_t",
        "--workflow-execution-id", "wfx_t", "--source-run-revision-id", "wrr_t",
        "--expected-proposal-hash", "h", "--expected-run-revision-hash", "h",
        "--expected-review-hash", "h", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("evaluate");
    let out = Command::new(openwand_bin()).args(["workflow-routing-readiness", "latest",
        "--proposal-id", "wnap_t", "--output-dir"]).arg(d.path()).arg("--json").output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wrrd_"));
}

#[test]
fn cli_routing_readiness_requires_expected_hashes() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-routing-readiness", "evaluate",
        "--proposal-id", "wnap_t", "--review-id", "wnar_t",
        "--workflow-execution-id", "wfx_t", "--source-run-revision-id", "wrr_t",
        "--output-dir"]).arg(d.path()).output().expect("evaluate");
    assert!(!out.status.success());
}

#[test]
fn cli_does_not_expose_route_resolve_reconcile_retry_resume() {
    let out1 = Command::new(openwand_bin()).args(["workflow-next-action-review", "--help"]).output().expect("help");
    let out2 = Command::new(openwand_bin()).args(["workflow-routing-readiness", "--help"]).output().expect("help");
    for out in [&out1, &out2] {
        let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
        assert!(!stdout.contains("route")); assert!(!stdout.contains("resolve"));
        assert!(!stdout.contains("reconcile")); assert!(!stdout.contains("approve-tool"));
        assert!(!stdout.contains("reject-tool")); assert!(!stdout.contains("retry"));
        assert!(!stdout.contains("resume")); assert!(!stdout.contains("execute-tool"));
        assert!(!stdout.contains("shell")); assert!(!stdout.contains("git"));
    }
}
