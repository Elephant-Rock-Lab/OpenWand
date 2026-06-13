//! CLI tests for workflow command review.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}
fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

fn acknowledge_args(d: &tempfile::TempDir) -> Vec<String> {
    vec!["workflow-command-review".into(), "acknowledge".into(),
        "--command-composer-id".into(), "wcc_t".into(),
        "--loop-controller-id".into(), "wlc_t".into(),
        "--workflow-execution-id".into(), "wfx_t".into(),
        "--expected-command-composer-hash".into(), "ch".into(),
        "--expected-command-descriptor-hash".into(), "dh".into(),
        "--expected-loop-controller-hash".into(), "lh".into(),
        "--reviewer".into(), "tester".into(),
        "--rationale".into(), "looks good".into(),
        "--output-dir".into(), d.path().to_string_lossy().into()]
}

#[test]
fn cli_command_review_acknowledge_outputs_review_id() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(acknowledge_args(&d))
        .arg("--json").output().expect("ack");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("wcrv_"), "Expected review ID: {}", stdout);
}

// Patch 4: CLI output says "review recorded" not "executed"
#[test]
fn cli_acknowledge_output_says_review_recorded_not_executed() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args(acknowledge_args(&d)).output().expect("ack");
    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(stdout.contains("review recorded"), "Expected 'review recorded': {}", stdout);
    assert!(stdout.contains("not executed"), "Expected 'not executed': {}", stdout);
}

#[test]
fn cli_command_review_reject_requires_feedback() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args([
        "workflow-command-review", "reject",
        "--command-composer-id", "wcc_t",
        "--loop-controller-id", "wlc_t",
        "--workflow-execution-id", "wfx_t",
        "--expected-command-composer-hash", "ch",
        "--expected-command-descriptor-hash", "dh",
        "--expected-loop-controller-hash", "lh",
        "--reviewer", "tester",
        "--rationale", "bad",
        "--output-dir"]).arg(d.path()).output().expect("reject");
    // Missing --feedback should fail (clap requires it)
    assert!(!out.status.success());
}

#[test]
fn cli_command_review_request_changes_requires_feedback() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args([
        "workflow-command-review", "request-changes",
        "--command-composer-id", "wcc_t",
        "--loop-controller-id", "wlc_t",
        "--workflow-execution-id", "wfx_t",
        "--expected-command-composer-hash", "ch",
        "--expected-command-descriptor-hash", "dh",
        "--expected-loop-controller-hash", "lh",
        "--reviewer", "tester",
        "--rationale", "needs work",
        "--output-dir"]).arg(d.path()).output().expect("req");
    assert!(!out.status.success());
}

#[test]
fn cli_command_review_show_roundtrips_record() {
    let d = temp_dir();
    let ack = Command::new(openwand_bin()).args(acknowledge_args(&d))
        .arg("--json").output().expect("ack");
    let rec: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&ack.stdout)).unwrap();
    let rid = rec["review_id"].as_str().unwrap();
    let show = Command::new(openwand_bin()).args(["workflow-command-review", "show", rid,
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("show");
    let shown: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&show.stdout)).unwrap();
    assert_eq!(rid, shown["review_id"].as_str().unwrap_or(""));
}

#[test]
fn cli_command_review_latest_by_command_composer_returns_latest() {
    let d = temp_dir();
    let _ = Command::new(openwand_bin()).args(acknowledge_args(&d))
        .arg("--json").output().expect("ack");
    let out = Command::new(openwand_bin()).args(["workflow-command-review", "latest",
        "--command-composer-id", "wcc_t",
        "--output-dir"]).arg(d.path()).arg("--json").output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wcrv_"));
}

#[test]
fn cli_command_review_requires_expected_hashes() {
    let d = temp_dir();
    let out = Command::new(openwand_bin()).args([
        "workflow-command-review", "acknowledge",
        "--command-composer-id", "wcc_t",
        "--loop-controller-id", "wlc_t",
        "--workflow-execution-id", "wfx_t",
        "--reviewer", "tester",
        "--rationale", "ok",
        "--output-dir"]).arg(d.path()).output().expect("no hashes");
    assert!(!out.status.success());
}

#[test]
fn cli_command_review_does_not_expose_execute_route_resolve_reconcile_retry_resume() {
    let out = Command::new(openwand_bin()).args(["workflow-command-review", "--help"]).output().expect("help");
    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(!stdout.contains("execute")); assert!(!stdout.contains("route-now"));
    assert!(!stdout.contains("resolve-now")); assert!(!stdout.contains("reconcile-now"));
    assert!(!stdout.contains("retry")); assert!(!stdout.contains("resume"));
    assert!(!stdout.contains("shell")); assert!(!stdout.contains("git"));
}
