//! CLI tests for workflow reconciliation commands.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}
fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

fn base_args() -> Vec<&'static str> {
    vec!["workflow-reconciliation", "reconcile",
        "--workflow-execution-id", "wfx_t",
        "--route-id", "war_t",
        "--outcome-id", "wao_t",
        "--stage-id", "s1",
        "--action-request-id", "ar1",
        "--expected-workflow-run-hash", "h",
        "--expected-route-hash", "rh",
        "--expected-outcome-hash", "oh"]
}

#[test]
fn cli_reconcile_outputs_reconciliation_id() {
    let d = temp_dir();
    let mut args = base_args();
    args.push("--output-dir"); args.push(d.path().to_str().unwrap());
    args.push("--json");
    let out = Command::new(openwand_bin()).args(&args).output().expect("reconcile");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("wrc_"), "Expected reconciliation ID: {}", stdout);
}

#[test]
fn cli_show_roundtrips_record() {
    let d = temp_dir();
    let mut args = base_args();
    args.push("--output-dir"); args.push(d.path().to_str().unwrap());
    args.push("--json");
    let out = Command::new(openwand_bin()).args(&args).output().expect("reconcile");
    let rec: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let rid = rec["reconciliation_id"].as_str().unwrap();
    let show = Command::new(openwand_bin()).args(["workflow-reconciliation", "show", rid,
        "--output-dir", d.path().to_str().unwrap(), "--json"])
        .output().expect("show");
    let shown: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&show.stdout)).unwrap();
    assert_eq!(rid, shown["reconciliation_id"].as_str().unwrap_or(""));
}

#[test]
fn cli_latest_by_workflow_run_returns_latest() {
    let d = temp_dir();
    let mut args = base_args();
    args.push("--output-dir"); args.push(d.path().to_str().unwrap());
    args.push("--json");
    Command::new(openwand_bin()).args(&args).output().expect("reconcile");
    let out = Command::new(openwand_bin()).args(["workflow-reconciliation", "latest",
        "--workflow-execution-id", "wfx_t",
        "--output-dir", d.path().to_str().unwrap(), "--json"])
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wrc_"));
}

#[test]
fn cli_latest_by_route_returns_latest() {
    let d = temp_dir();
    let mut args = base_args();
    args.push("--output-dir"); args.push(d.path().to_str().unwrap());
    args.push("--json");
    Command::new(openwand_bin()).args(&args).output().expect("reconcile");
    let out = Command::new(openwand_bin()).args(["workflow-reconciliation", "latest",
        "--route-id", "war_t",
        "--output-dir", d.path().to_str().unwrap(), "--json"])
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wrc_"));
}

#[test]
fn cli_latest_by_outcome_returns_latest() {
    let d = temp_dir();
    let mut args = base_args();
    args.push("--output-dir"); args.push(d.path().to_str().unwrap());
    args.push("--json");
    Command::new(openwand_bin()).args(&args).output().expect("reconcile");
    let out = Command::new(openwand_bin()).args(["workflow-reconciliation", "latest",
        "--outcome-id", "wao_t",
        "--output-dir", d.path().to_str().unwrap(), "--json"])
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wrc_"));
}

#[test]
fn cli_requires_expected_hashes() {
    let d = temp_dir();
    // Missing --expected-outcome-hash should fail
    let out = Command::new(openwand_bin()).args(["workflow-reconciliation", "reconcile",
        "--workflow-execution-id", "wfx_t", "--route-id", "war_t",
        "--outcome-id", "wao_t", "--stage-id", "s1", "--action-request-id", "ar1",
        "--expected-workflow-run-hash", "h", "--expected-route-hash", "rh",
        "--output-dir", d.path().to_str().unwrap()])
        .output().expect("reconcile");
    assert!(!out.status.success());
}

#[test]
fn cli_does_not_expose_route_resolve_approve_retry_resume() {
    let out = Command::new(openwand_bin()).args(["workflow-reconciliation", "--help"]).output().expect("help");
    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(!stdout.contains("route"));
    assert!(!stdout.contains("resolve"));
    assert!(!stdout.contains("approve"));
    assert!(!stdout.contains("reject"));
    assert!(!stdout.contains("retry"));
    assert!(!stdout.contains("resume"));
    assert!(!stdout.contains("execute-tool"));
    assert!(!stdout.contains("run-tool"));
    assert!(!stdout.contains("shell"));
    assert!(!stdout.contains("git"));
}
