//! CLI tests for workflow readiness commands.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// Create the full chain: plan → review → proposal → proposal review
fn create_full_chain(dir: &std::path::Path) -> (String, String, String, String) {
    // Create task plan
    let create_output = Command::new(openwand_bin())
        .args(["task-plan", "create", "--intent", "Readiness CLI test", "--output-dir"])
        .arg(dir)
        .arg("--json")
        .output().expect("Failed to create plan");
    let plan: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&create_output.stdout)).unwrap();
    let plan_id = plan["plan_id"].as_str().unwrap().to_string();
    let plan_hash = plan["plan_hash"].as_str().unwrap().to_string();

    // Approve task plan
    let _ = Command::new(openwand_bin())
        .args(["task-plan", "review", "approve", "--plan-id", &plan_id, "--reviewer", "test", "--rationale", "OK", "--output-dir"])
        .arg(dir).arg("--json")
        .output().expect("Failed to approve plan");

    // Create workflow proposal
    let proposal_output = Command::new(openwand_bin())
        .args(["workflow-proposal", "create", "--task-plan-id", &plan_id, "--output-dir"])
        .arg(dir).arg("--json")
        .output().expect("Failed to create proposal");
    let proposal: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&proposal_output.stdout)).unwrap();
    let proposal_id = proposal["proposal_id"].as_str().unwrap().to_string();
    let proposal_hash = proposal["proposal_hash"].as_str().unwrap().to_string();
    let source_plan_hash = proposal["source_task_plan_hash"].as_str().unwrap().to_string();

    // Approve workflow proposal
    let review_output = Command::new(openwand_bin())
        .args(["workflow-proposal", "review", "approve", "--proposal-id", &proposal_id, "--reviewer", "test", "--rationale", "Good", "--output-dir"])
        .arg(dir).arg("--json")
        .output().expect("Failed to approve proposal");
    let review: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&review_output.stdout)).unwrap();
    let review_id = review["review_id"].as_str().unwrap().to_string();

    (proposal_id, review_id, proposal_hash, source_plan_hash)
}

#[test]
fn cli_workflow_readiness_evaluate_outputs_readiness_id() {
    let dir = temp_dir();
    let (proposal_id, review_id, proposal_hash, source_plan_hash) = create_full_chain(dir.path());

    let output = Command::new(openwand_bin())
        .args(["workflow-readiness", "evaluate",
            "--proposal-id", &proposal_id,
            "--review-id", &review_id,
            "--expected-proposal-hash", &proposal_hash,
            "--expected-source-task-plan-hash", &source_plan_hash,
            "--output-dir"])
        .arg(dir.path()).arg("--json")
        .output().expect("Failed to evaluate readiness");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wfrd_"), "Expected readiness ID: {}", stdout);
}

#[test]
fn cli_workflow_readiness_show_roundtrips_record() {
    let dir = temp_dir();
    let (proposal_id, review_id, proposal_hash, source_plan_hash) = create_full_chain(dir.path());

    let eval_output = Command::new(openwand_bin())
        .args(["workflow-readiness", "evaluate",
            "--proposal-id", &proposal_id,
            "--review-id", &review_id,
            "--expected-proposal-hash", &proposal_hash,
            "--expected-source-task-plan-hash", &source_plan_hash,
            "--output-dir"])
        .arg(dir.path()).arg("--json")
        .output().expect("Failed to evaluate");
    let eval: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&eval_output.stdout)).unwrap();
    let readiness_id = eval["readiness_id"].as_str().unwrap();

    let show_output = Command::new(openwand_bin())
        .args(["workflow-readiness", "show", readiness_id, "--output-dir"])
        .arg(dir.path()).arg("--json")
        .output().expect("Failed to show");
    let show: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&show_output.stdout)).unwrap();
    assert_eq!(readiness_id, show["readiness_id"].as_str().unwrap());
}

#[test]
fn cli_workflow_readiness_latest_by_proposal_returns_latest() {
    let dir = temp_dir();
    let (proposal_id, review_id, proposal_hash, source_plan_hash) = create_full_chain(dir.path());

    let _ = Command::new(openwand_bin())
        .args(["workflow-readiness", "evaluate",
            "--proposal-id", &proposal_id,
            "--review-id", &review_id,
            "--expected-proposal-hash", &proposal_hash,
            "--expected-source-task-plan-hash", &source_plan_hash,
            "--output-dir"])
        .arg(dir.path()).arg("--json")
        .output().expect("Failed to evaluate");

    let output = Command::new(openwand_bin())
        .args(["workflow-readiness", "latest", "--proposal-id", &proposal_id, "--output-dir"])
        .arg(dir.path()).arg("--json")
        .output().expect("Failed to get latest");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wfrd_"), "Expected readiness in latest: {}", stdout);
}

#[test]
fn cli_workflow_readiness_latest_by_review_returns_latest() {
    let dir = temp_dir();
    let (proposal_id, review_id, proposal_hash, source_plan_hash) = create_full_chain(dir.path());

    let _ = Command::new(openwand_bin())
        .args(["workflow-readiness", "evaluate",
            "--proposal-id", &proposal_id,
            "--review-id", &review_id,
            "--expected-proposal-hash", &proposal_hash,
            "--expected-source-task-plan-hash", &source_plan_hash,
            "--output-dir"])
        .arg(dir.path()).arg("--json")
        .output().expect("Failed to evaluate");

    let output = Command::new(openwand_bin())
        .args(["workflow-readiness", "latest", "--review-id", &review_id, "--output-dir"])
        .arg(dir.path()).arg("--json")
        .output().expect("Failed to get latest");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wfrd_"), "Expected readiness: {}", stdout);
}

#[test]
fn cli_workflow_readiness_latest_by_task_plan_returns_latest() {
    let dir = temp_dir();
    let (proposal_id, review_id, proposal_hash, source_plan_hash) = create_full_chain(dir.path());

    let eval_output = Command::new(openwand_bin())
        .args(["workflow-readiness", "evaluate",
            "--proposal-id", &proposal_id,
            "--review-id", &review_id,
            "--expected-proposal-hash", &proposal_hash,
            "--expected-source-task-plan-hash", &source_plan_hash,
            "--output-dir"])
        .arg(dir.path()).arg("--json")
        .output().expect("Failed to evaluate");
    let eval: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&eval_output.stdout)).unwrap();
    let task_plan_id = eval["source_task_plan_id"].as_str().unwrap();

    let output = Command::new(openwand_bin())
        .args(["workflow-readiness", "latest", "--task-plan-id", task_plan_id, "--output-dir"])
        .arg(dir.path()).arg("--json")
        .output().expect("Failed to get latest");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wfrd_"), "Expected readiness: {}", stdout);
}

#[test]
fn cli_workflow_readiness_does_not_expose_execute_run_start_schedule() {
    let output = Command::new(openwand_bin())
        .args(["workflow-readiness", "--help"])
        .output().expect("Failed to run openwand");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lower = stdout.to_lowercase();
    assert!(!lower.contains("execute"), "must not expose execute: {}", stdout);
    assert!(!lower.contains("start"), "must not expose start: {}", stdout);
    assert!(!lower.contains("schedule"), "must not expose schedule: {}", stdout);
}

#[test]
fn cli_workflow_readiness_command_namespace_matches_workflow_proposal() {
    // Both are top-level commands (workflow-proposal, workflow-readiness)
    let help = Command::new(openwand_bin())
        .args(["--help"])
        .output().expect("Failed to run openwand");
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(stdout.contains("workflow-proposal"), "workflow-proposal should be top-level");
    assert!(stdout.contains("workflow-readiness"), "workflow-readiness should be top-level");
    // Neither should be under 'eval'
    let wr_help = Command::new(openwand_bin())
        .args(["workflow-readiness", "--help"])
        .output().expect("Failed to run openwand");
    let wr_stdout = String::from_utf8_lossy(&wr_help.stdout);
    assert!(wr_stdout.contains("evaluate"), "should have evaluate subcommand");
    assert!(wr_stdout.contains("show"), "should have show subcommand");
    assert!(wr_stdout.contains("latest"), "should have latest subcommand");
}

#[test]
fn cli_workflow_readiness_requires_expected_hashes() {
    let dir = temp_dir();
    let (proposal_id, review_id, _, _) = create_full_chain(dir.path());
    // Missing expected hashes should fail
    let output = Command::new(openwand_bin())
        .args(["workflow-readiness", "evaluate",
            "--proposal-id", &proposal_id,
            "--review-id", &review_id,
            "--output-dir"])
        .arg(dir.path())
        .output().expect("Failed to run openwand");
    assert!(!output.status.success(), "Should fail without required hashes");
}
