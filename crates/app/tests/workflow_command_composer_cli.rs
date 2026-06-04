//! CLI tests for workflow command composer.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}
fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

#[test]
fn cli_workflow_command_compose_outputs_descriptor_id() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-command", "compose",
        "--workflow-execution-id", "wfx_t", "--loop-controller-id", "wlc_t",
        "--expected-loop-controller-hash", "h",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("compose");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("wcc_"), "Expected composer ID: {}", stdout);
}

#[test]
fn cli_workflow_command_show_roundtrips_record() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-command", "compose",
        "--workflow-execution-id", "wfx_t", "--loop-controller-id", "wlc_t",
        "--expected-loop-controller-hash", "h",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("compose");
    let rec: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let cid = rec["composer_id"].as_str().unwrap();
    let show = Command::new(openwand_bin()).args(["workflow-command", "show", cid,
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("show");
    let shown: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&show.stdout)).unwrap();
    assert_eq!(cid, shown["composer_id"].as_str().unwrap_or(""));
}

#[test]
fn cli_workflow_command_latest_by_workflow_run_returns_latest() {
    let d = temp_dir();
    Command::new(openwand_bin()).args(["workflow-command", "compose",
        "--workflow-execution-id", "wfx_t", "--loop-controller-id", "wlc_t",
        "--expected-loop-controller-hash", "h",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("compose");
    let out = Command::new(openwand_bin()).args(["workflow-command", "latest",
        "--workflow-execution-id", "wfx_t",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wcc_"));
}

#[test]
fn cli_workflow_command_latest_by_loop_controller_returns_latest() {
    let d = temp_dir();
    Command::new(openwand_bin()).args(["workflow-command", "compose",
        "--workflow-execution-id", "wfx_t2", "--loop-controller-id", "wlc_t2",
        "--expected-loop-controller-hash", "h",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("compose");
    let out = Command::new(openwand_bin()).args(["workflow-command", "latest",
        "--loop-controller-id", "wlc_t2",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wcc_"));
}

#[test]
fn cli_workflow_command_requires_expected_loop_controller_hash() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(["workflow-command", "compose",
        "--workflow-execution-id", "wfx_t", "--loop-controller-id", "wlc_t",
        "--output-dir"]).arg(d.path()).output().expect("compose");
    assert!(!out.status.success());
}

#[test]
fn cli_workflow_command_does_not_expose_execute_route_resolve_reconcile_retry_resume() {
    let out = Command::new(openwand_bin()).args(["workflow-command", "--help"]).output().expect("help");
    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(!stdout.contains("execute")); assert!(!stdout.contains("route-now"));
    assert!(!stdout.contains("resolve-now")); assert!(!stdout.contains("reconcile-now"));
    assert!(!stdout.contains("approve-now")); assert!(!stdout.contains("retry"));
    assert!(!stdout.contains("resume")); assert!(!stdout.contains("shell"));
    assert!(!stdout.contains("git")); assert!(!stdout.contains("process"));
}
