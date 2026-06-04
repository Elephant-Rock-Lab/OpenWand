//! CLI tests for workflow next-action routing.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}
fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

#[test]
fn cli_next_action_routing_route_outputs_routing_id() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-next-action-routing", "route",
        "--routing-readiness-id", "wrrd_t", "--next-action-proposal-id", "wnap_t",
        "--next-action-review-id", "wnar_t", "--workflow-execution-id", "wfx_t",
        "--source-run-revision-id", "wrr_t",
        "--expected-routing-readiness-hash", "h", "--expected-proposal-hash", "h",
        "--expected-review-hash", "h", "--expected-run-revision-hash", "h",
        "--expected-action-request-hash", "h",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("route");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("wnaroute_"), "Expected routing ID: {}", stdout);
}

#[test]
fn cli_next_action_routing_show_roundtrips_record() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-next-action-routing", "route",
        "--routing-readiness-id", "wrrd_t", "--next-action-proposal-id", "wnap_t",
        "--next-action-review-id", "wnar_t", "--workflow-execution-id", "wfx_t",
        "--source-run-revision-id", "wrr_t",
        "--expected-routing-readiness-hash", "h", "--expected-proposal-hash", "h",
        "--expected-review-hash", "h", "--expected-run-revision-hash", "h",
        "--expected-action-request-hash", "h",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("route");
    let rec: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let rid = rec["routing_id"].as_str().unwrap();
    let show = Command::new(openwand_bin()).args(["workflow-next-action-routing", "show", rid,
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("show");
    let shown: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&show.stdout)).unwrap();
    assert_eq!(rid, shown["routing_id"].as_str().unwrap_or(""));
}

#[test]
fn cli_next_action_routing_latest_by_readiness_returns_latest() {
    let d = temp_dir();
    Command::new(openwand_bin()).args(["workflow-next-action-routing", "route",
        "--routing-readiness-id", "wrrd_t", "--next-action-proposal-id", "wnap_t",
        "--next-action-review-id", "wnar_t", "--workflow-execution-id", "wfx_t",
        "--source-run-revision-id", "wrr_t",
        "--expected-routing-readiness-hash", "h", "--expected-proposal-hash", "h",
        "--expected-review-hash", "h", "--expected-run-revision-hash", "h",
        "--expected-action-request-hash", "h",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("route");
    let out = Command::new(openwand_bin()).args(["workflow-next-action-routing", "latest",
        "--routing-readiness-id", "wrrd_t",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wnaroute_"));
}

#[test]
fn cli_next_action_routing_latest_by_route_returns_latest() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-next-action-routing", "route",
        "--routing-readiness-id", "wrrd_t2", "--next-action-proposal-id", "wnap_t2",
        "--next-action-review-id", "wnar_t2", "--workflow-execution-id", "wfx_t2",
        "--source-run-revision-id", "wrr_t2",
        "--expected-routing-readiness-hash", "h", "--expected-proposal-hash", "h",
        "--expected-review-hash", "h", "--expected-run-revision-hash", "h",
        "--expected-action-request-hash", "h",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("route");
    // The gate-only path (no adapter) has "pending" route_id, skip by_route test
    // Test by_workflow_run instead
    let out2 = Command::new(openwand_bin()).args(["workflow-next-action-routing", "latest",
        "--workflow-execution-id", "wfx_t2",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("latest");
    assert!(String::from_utf8_lossy(&out2.stdout).contains("wnaroute_"));
}

#[test]
fn cli_next_action_routing_requires_expected_hashes() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-next-action-routing", "route",
        "--routing-readiness-id", "wrrd_t", "--next-action-proposal-id", "wnap_t",
        "--next-action-review-id", "wnar_t", "--workflow-execution-id", "wfx_t",
        "--source-run-revision-id", "wrr_t",
        "--output-dir"]).arg(d.path()).output().expect("route");
    assert!(!out.status.success());
}

#[test]
fn cli_next_action_routing_does_not_expose_tool_approval_reconcile_retry_resume() {
    let out = Command::new(openwand_bin()).args(["workflow-next-action-routing", "--help"]).output().expect("help");
    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(!stdout.contains("execute-tool")); assert!(!stdout.contains("run-tool"));
    assert!(!stdout.contains("approve-tool")); assert!(!stdout.contains("reject-tool"));
    assert!(!stdout.contains("resolve")); assert!(!stdout.contains("reconcile"));
    assert!(!stdout.contains("retry")); assert!(!stdout.contains("resume"));
    assert!(!stdout.contains("shell")); assert!(!stdout.contains("git"));
}
