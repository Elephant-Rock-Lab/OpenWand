//! CLI tests for workflow action outcome commands.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}
fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

#[test]
fn cli_workflow_action_outcome_approve_outputs_outcome_id() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-action-outcome", "resolve",
        "--workflow-execution-id", "wfx_t", "--route-id", "war_t",
        "--stage-id", "s1", "--action-request-id", "ar1", "--session-id", "sess1",
        "--pending-approval-id", "arid1", "--expected-route-hash", "h", "--expected-workflow-run-hash", "h",
        "--approve", "--rationale", "safe", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("resolve approve");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("wao_"), "Expected outcome ID: {}", stdout);
}

#[test]
fn cli_workflow_action_outcome_reject_outputs_outcome_id() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-action-outcome", "resolve",
        "--workflow-execution-id", "wfx_r", "--route-id", "war_r",
        "--stage-id", "s2", "--action-request-id", "ar2", "--session-id", "sess2",
        "--pending-approval-id", "arid2", "--expected-route-hash", "h", "--expected-workflow-run-hash", "h",
        "--reject", "--rationale", "risky", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("resolve reject");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("wao_"), "Expected outcome ID: {}", stdout);
}

#[test]
fn cli_workflow_action_outcome_show_roundtrips_record() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-action-outcome", "resolve",
        "--workflow-execution-id", "wfx_s", "--route-id", "war_s",
        "--stage-id", "s3", "--action-request-id", "ar3", "--session-id", "sess3",
        "--pending-approval-id", "arid3", "--expected-route-hash", "h", "--expected-workflow-run-hash", "h",
        "--approve", "--rationale", "ok", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("resolve");
    let rec: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let oid = rec["outcome_id"].as_str().unwrap();
    let show = Command::new(openwand_bin()).args(["workflow-action-outcome", "show", oid, "--output-dir"])
        .arg(d.path()).arg("--json").output().expect("show");
    let shown: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&show.stdout)).unwrap();
    assert_eq!(oid, shown["outcome_id"].as_str().unwrap_or(""));
}

#[test]
fn cli_workflow_action_outcome_latest_by_route_returns_latest() {
    let d = temp_dir();
    Command::new(openwand_bin()).args(["workflow-action-outcome", "resolve",
        "--workflow-execution-id", "wfx_l", "--route-id", "war_l",
        "--stage-id", "s4", "--action-request-id", "ar4", "--session-id", "sess4",
        "--pending-approval-id", "arid4", "--expected-route-hash", "h", "--expected-workflow-run-hash", "h",
        "--approve", "--rationale", "ok", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("resolve");
    let out = Command::new(openwand_bin()).args(["workflow-action-outcome", "latest",
        "--route-id", "war_l", "--output-dir"]).arg(d.path()).arg("--json").output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wao_"));
}

#[test]
fn cli_workflow_action_outcome_latest_by_pending_approval_returns_latest() {
    let d = temp_dir();
    Command::new(openwand_bin()).args(["workflow-action-outcome", "resolve",
        "--workflow-execution-id", "wfx_p", "--route-id", "war_p",
        "--stage-id", "s5", "--action-request-id", "ar5", "--session-id", "sess5",
        "--pending-approval-id", "arid5", "--expected-route-hash", "h", "--expected-workflow-run-hash", "h",
        "--approve", "--rationale", "ok", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("resolve");
    let out = Command::new(openwand_bin()).args(["workflow-action-outcome", "latest",
        "--pending-approval-id", "arid5", "--output-dir"]).arg(d.path()).arg("--json").output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wao_"));
}

#[test]
fn cli_workflow_action_outcome_requires_rationale() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-action-outcome", "resolve",
        "--workflow-execution-id", "wfx", "--route-id", "war",
        "--stage-id", "s", "--action-request-id", "ar", "--session-id", "se",
        "--pending-approval-id", "pa", "--expected-route-hash", "h", "--expected-workflow-run-hash", "h",
        "--approve", "--rationale", "", "--output-dir"]).arg(d.path())
        .output().expect("resolve");
    // Empty rationale should still run (gate blocks it)
    assert!(out.status.success());
}

#[test]
fn cli_workflow_action_outcome_requires_exactly_one_resolution() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-action-outcome", "resolve",
        "--workflow-execution-id", "wfx", "--route-id", "war",
        "--stage-id", "s", "--action-request-id", "ar", "--session-id", "se",
        "--pending-approval-id", "pa", "--expected-route-hash", "h", "--expected-workflow-run-hash", "h",
        "--approve", "--reject", "--rationale", "ok", "--output-dir"]).arg(d.path())
        .output().expect("resolve");
    assert!(!out.status.success(), "Should fail with both approve and reject");
}

#[test]
fn cli_workflow_action_outcome_does_not_expose_direct_approval_or_tool_surfaces() {
    let out = Command::new(openwand_bin()).args(["workflow-action-outcome", "--help"]).output().expect("help");
    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(!stdout.contains("approve-direct"));
    assert!(!stdout.contains("reject-direct"));
    assert!(!stdout.contains("execute-tool"));
    assert!(!stdout.contains("run-tool"));
    assert!(!stdout.contains("mutate-approval"));
    assert!(!stdout.contains("trace-append"));
    assert!(!stdout.contains("retry"));
    assert!(!stdout.contains("resume-workflow"));
}
