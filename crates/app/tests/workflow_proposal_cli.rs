//! CLI tests for workflow proposal commands.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// Create a task plan and approve it, returning the plan ID.
fn create_approved_plan(dir: &std::path::Path) -> String {
    // Create plan
    let create_output = Command::new(openwand_bin())
        .args(["task-plan", "create", "--intent", "Test proposal CLI", "--output-dir"])
        .arg(dir)
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let create_stdout = String::from_utf8_lossy(&create_output.stdout);
    let plan: serde_json::Value = serde_json::from_str(&create_stdout).unwrap();
    let plan_id = plan["plan_id"].as_str().unwrap().to_string();

    // Approve plan
    let _ = Command::new(openwand_bin())
        .args(["task-plan", "review", "approve", "--plan-id", &plan_id, "--reviewer", "test-user", "--rationale", "OK", "--output-dir"])
        .arg(dir)
        .arg("--json")
        .output()
        .expect("Failed to approve plan");

    plan_id
}

#[test]
fn cli_workflow_proposal_create_outputs_proposal_id() {
    let dir = temp_dir();
    let plan_id = create_approved_plan(dir.path());

    let output = Command::new(openwand_bin())
        .args(["workflow-proposal", "create", "--task-plan-id", &plan_id, "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wfp_"), "Expected proposal ID in output: {}", stdout);
    assert!(stdout.contains("stages"), "Expected stages in JSON: {}", stdout);
}

#[test]
fn cli_workflow_proposal_show_roundtrips_record() {
    let dir = temp_dir();
    let plan_id = create_approved_plan(dir.path());

    let create_output = Command::new(openwand_bin())
        .args(["workflow-proposal", "create", "--task-plan-id", &plan_id, "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let create_stdout = String::from_utf8_lossy(&create_output.stdout);
    let proposal: serde_json::Value = serde_json::from_str(&create_stdout).unwrap();
    let proposal_id = proposal["proposal_id"].as_str().unwrap();

    let show_output = Command::new(openwand_bin())
        .args(["workflow-proposal", "show", proposal_id, "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let show_stdout = String::from_utf8_lossy(&show_output.stdout);
    let shown: serde_json::Value = serde_json::from_str(&show_stdout).unwrap();
    assert_eq!(proposal_id, shown["proposal_id"].as_str().unwrap());
}

#[test]
fn cli_workflow_proposal_latest_by_task_plan_returns_latest() {
    let dir = temp_dir();
    let plan_id = create_approved_plan(dir.path());

    let _ = Command::new(openwand_bin())
        .args(["workflow-proposal", "create", "--task-plan-id", &plan_id, "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");

    let output = Command::new(openwand_bin())
        .args(["workflow-proposal", "latest", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wfp_"), "Expected proposal in latest output: {}", stdout);
}

#[test]
fn cli_workflow_review_approve_outputs_review_id() {
    let dir = temp_dir();
    let plan_id = create_approved_plan(dir.path());

    let create_output = Command::new(openwand_bin())
        .args(["workflow-proposal", "create", "--task-plan-id", &plan_id, "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let create_stdout = String::from_utf8_lossy(&create_output.stdout);
    let proposal: serde_json::Value = serde_json::from_str(&create_stdout).unwrap();
    let proposal_id = proposal["proposal_id"].as_str().unwrap();

    let review_output = Command::new(openwand_bin())
        .args(["workflow-proposal", "review", "approve", "--proposal-id", proposal_id, "--reviewer", "test-user", "--rationale", "Good proposal", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let review_stdout = String::from_utf8_lossy(&review_output.stdout);
    assert!(review_stdout.contains("wfr_"), "Expected review ID: {}", review_stdout);
    assert!(review_stdout.contains("approved"), "Expected approved: {}", review_stdout);
    assert!(review_stdout.contains("false"), "Expected creates_execution_grant=false: {}", review_stdout);
}

#[test]
fn cli_workflow_review_reject_requires_feedback() {
    let dir = temp_dir();
    let plan_id = create_approved_plan(dir.path());

    let create_output = Command::new(openwand_bin())
        .args(["workflow-proposal", "create", "--task-plan-id", &plan_id, "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let create_stdout = String::from_utf8_lossy(&create_output.stdout);
    let proposal: serde_json::Value = serde_json::from_str(&create_stdout).unwrap();
    let proposal_id = proposal["proposal_id"].as_str().unwrap();

    let review_output = Command::new(openwand_bin())
        .args(["workflow-proposal", "review", "reject", "--proposal-id", proposal_id, "--reviewer", "test-user", "--rationale", "Bad", "--feedback", "Missing stages", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let review_stdout = String::from_utf8_lossy(&review_output.stdout);
    assert!(review_stdout.contains("rejected"), "Expected rejected: {}", review_stdout);
}

#[test]
fn cli_workflow_review_request_changes_requires_feedback() {
    let dir = temp_dir();
    let plan_id = create_approved_plan(dir.path());

    let create_output = Command::new(openwand_bin())
        .args(["workflow-proposal", "create", "--task-plan-id", &plan_id, "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let create_stdout = String::from_utf8_lossy(&create_output.stdout);
    let proposal: serde_json::Value = serde_json::from_str(&create_stdout).unwrap();
    let proposal_id = proposal["proposal_id"].as_str().unwrap();

    let review_output = Command::new(openwand_bin())
        .args(["workflow-proposal", "review", "request-changes", "--proposal-id", proposal_id, "--reviewer", "test-user", "--rationale", "Needs work", "--feedback", "Add more stages", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let review_stdout = String::from_utf8_lossy(&review_output.stdout);
    assert!(review_stdout.contains("changes_requested"), "Expected changes_requested: {}", review_stdout);
}

#[test]
fn cli_workflow_proposal_does_not_expose_execute() {
    let output = Command::new(openwand_bin())
        .args(["workflow-proposal", "--help"])
        .output()
        .expect("Failed to run openwand");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("execute"), "workflow-proposal must not expose execute: {}", stdout);
}

#[test]
fn cli_workflow_proposal_does_not_expose_run_start_schedule() {
    let output = Command::new(openwand_bin())
        .args(["workflow-proposal", "--help"])
        .output()
        .expect("Failed to run openwand");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lower = stdout.to_lowercase();
    assert!(!lower.contains("run"), "workflow-proposal must not expose run: {}", stdout);
    assert!(!lower.contains("start"), "workflow-proposal must not expose start: {}", stdout);
    assert!(!lower.contains("schedule"), "workflow-proposal must not expose schedule: {}", stdout);
}
