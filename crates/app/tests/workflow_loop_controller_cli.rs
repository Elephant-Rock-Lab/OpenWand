//! CLI tests for workflow loop controller.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}
fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

#[test]
fn cli_workflow_loop_recommend_outputs_controller_id() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-loop", "recommend",
        "--workflow-execution-id", "wfx_t", "--expected-workflow-run-hash", "h",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("recommend");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("wlc_"), "Expected controller ID: {}", stdout);
}

#[test]
fn cli_workflow_loop_show_roundtrips_record() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-loop", "recommend",
        "--workflow-execution-id", "wfx_t", "--expected-workflow-run-hash", "h",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("recommend");
    let rec: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let cid = rec["controller_id"].as_str().unwrap();
    let show = Command::new(openwand_bin()).args(["workflow-loop", "show", cid,
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("show");
    let shown: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&show.stdout)).unwrap();
    assert_eq!(cid, shown["controller_id"].as_str().unwrap_or(""));
}

#[test]
fn cli_workflow_loop_latest_by_workflow_run_returns_latest() {
    let d = temp_dir();
    Command::new(openwand_bin()).args(["workflow-loop", "recommend",
        "--workflow-execution-id", "wfx_t", "--expected-workflow-run-hash", "h",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("recommend");
    let out = Command::new(openwand_bin()).args(["workflow-loop", "latest",
        "--workflow-execution-id", "wfx_t",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wlc_"));
}

#[test]
fn cli_workflow_loop_latest_by_run_revision_returns_latest() {
    let d = temp_dir();
    Command::new(openwand_bin()).args(["workflow-loop", "recommend",
        "--workflow-execution-id", "wfx_t2", "--expected-workflow-run-hash", "h",
        "--latest-run-revision-id", "wrr_t2",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("recommend");
    let out = Command::new(openwand_bin()).args(["workflow-loop", "latest",
        "--run-revision-id", "wrr_t2",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wlc_"));
}

#[test]
fn cli_workflow_loop_requires_expected_workflow_run_hash() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-loop", "recommend",
        "--workflow-execution-id", "wfx_t",
        "--output-dir"]).arg(d.path()).output().expect("recommend");
    assert!(!out.status.success());
}

#[test]
fn cli_workflow_loop_does_not_expose_route_resolve_reconcile_retry_resume() {
    let out = Command::new(openwand_bin()).args(["workflow-loop", "--help"]).output().expect("help");
    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(!stdout.contains("route")); assert!(!stdout.contains("resolve"));
    assert!(!stdout.contains("reconcile")); assert!(!stdout.contains("approve"));
    assert!(!stdout.contains("reject")); assert!(!stdout.contains("retry"));
    assert!(!stdout.contains("resume")); assert!(!stdout.contains("execute-tool"));
    assert!(!stdout.contains("shell")); assert!(!stdout.contains("git"));
}
