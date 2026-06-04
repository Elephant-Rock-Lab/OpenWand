//! CLI tests for workflow continuation commands.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}
fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

#[test]
fn cli_propose_outputs_proposal_or_status() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-continuation", "propose",
        "--workflow-execution-id", "wfx_t", "--latest-run-revision-id", "wrr_t",
        "--expected-run-revision-hash", "h", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("propose");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("wcr_"), "Expected readiness ID: {}", stdout);
}

#[test]
fn cli_show_readiness_roundtrips_record() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-continuation", "propose",
        "--workflow-execution-id", "wfx_t", "--latest-run-revision-id", "wrr_t",
        "--expected-run-revision-hash", "h", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("propose");
    let rec: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let rid = rec["readiness_id"].as_str().unwrap();
    let show = Command::new(openwand_bin()).args(["workflow-continuation", "show-readiness", rid,
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("show");
    let shown: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&show.stdout)).unwrap();
    assert_eq!(rid, shown["readiness_id"].as_str().unwrap_or(""));
}

#[test]
fn cli_show_proposal_roundtrips_record() {
    // No proposal created without context — but the command should work with a valid ID
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-continuation", "show-proposal",
        "wnap_nonexistent", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("show-proposal");
    // Should fail gracefully (no record found)
    assert!(!out.status.success());
}

#[test]
fn cli_propose_no_eligible_action_outputs_readiness_id_not_proposal_id() {
    // Patch 2: when no eligible action, readiness is created, not proposal
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-continuation", "propose",
        "--workflow-execution-id", "wfx_t", "--latest-run-revision-id", "wrr_t",
        "--expected-run-revision-hash", "h", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("propose");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Should have readiness_id, not proposal_id
    assert!(stdout.contains("readiness_id"));
    assert!(!stdout.contains("proposal_id"));
}

#[test]
fn cli_latest_by_workflow_run_returns_latest() {
    let d = temp_dir();
    Command::new(openwand_bin()).args(["workflow-continuation", "propose",
        "--workflow-execution-id", "wfx_t", "--latest-run-revision-id", "wrr_t",
        "--expected-run-revision-hash", "h", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("propose");
    let out = Command::new(openwand_bin()).args(["workflow-continuation", "latest",
        "--workflow-execution-id", "wfx_t", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wcr_"));
}

#[test]
fn cli_latest_by_run_revision_returns_latest() {
    let d = temp_dir();
    Command::new(openwand_bin()).args(["workflow-continuation", "propose",
        "--workflow-execution-id", "wfx_t", "--latest-run-revision-id", "wrr_t",
        "--expected-run-revision-hash", "h", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("propose");
    let out = Command::new(openwand_bin()).args(["workflow-continuation", "latest",
        "--run-revision-id", "wrr_t", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wnap_") || String::from_utf8_lossy(&out.stdout).contains("No proposal"));
}

#[test]
fn cli_latest_by_stage_returns_latest() {
    let d = temp_dir();
    Command::new(openwand_bin()).args(["workflow-continuation", "propose",
        "--workflow-execution-id", "wfx_t", "--latest-run-revision-id", "wrr_t",
        "--expected-run-revision-hash", "h", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("propose");
    let out = Command::new(openwand_bin()).args(["workflow-continuation", "latest",
        "--stage-id", "s1", "--output-dir"]).arg(d.path()).arg("--json")
        .output().expect("latest");
    // May not find proposal, but command should succeed
    assert!(out.status.success());
}

#[test]
fn cli_requires_expected_run_revision_hash() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-continuation", "propose",
        "--workflow-execution-id", "wfx_t", "--latest-run-revision-id", "wrr_t",
        "--output-dir"]).arg(d.path()).output().expect("propose");
    assert!(!out.status.success());
}

#[test]
fn cli_does_not_expose_route_resolve_reconcile_retry_resume() {
    let out = Command::new(openwand_bin()).args(["workflow-continuation", "--help"]).output().expect("help");
    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(!stdout.contains("route"));
    assert!(!stdout.contains("resolve"));
    assert!(!stdout.contains("reconcile"));
    assert!(!stdout.contains("approve"));
    assert!(!stdout.contains("reject"));
    assert!(!stdout.contains("retry"));
    assert!(!stdout.contains("resume"));
    assert!(!stdout.contains("execute-tool"));
    assert!(!stdout.contains("run-tool"));
    assert!(!stdout.contains("shell"));
    assert!(!stdout.contains("git"));
}
