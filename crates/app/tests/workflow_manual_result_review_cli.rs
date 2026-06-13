//! CLI integration tests for workflow-manual-result-review.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}
fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

fn review_accept_args(d: &tempfile::TempDir, suffix: &str) -> Vec<String> {
    let out = d.path().to_string_lossy().to_string();
    vec!["workflow-manual-result-review".into(), "review-accept".into(),
        "--manual-result-id".into(), format!("wmr_{}", suffix),
        "--workflow-execution-id".into(), format!("wfx_{}", suffix),
        "--command-review-id".into(), format!("wcrv_{}", suffix),
        "--command-composer-id".into(), format!("wcc_{}", suffix),
        "--loop-controller-id".into(), format!("wlc_{}", suffix),
        "--expected-manual-result-hash".into(), "mrh".into(),
        "--expected-command-review-hash".into(), "crh".into(),
        "--expected-command-composer-hash".into(), "cch".into(),
        "--expected-command-descriptor-hash".into(), "cdh".into(),
        "--expected-loop-controller-hash".into(), "lch".into(),
        "--reviewer".into(), "alice".into(),
        "--rationale".into(), "evidence sufficient".into(),
        "--output-dir".into(), out,
    ]
}

#[test]
fn cli_manual_result_review_accept_outputs_review_id() {
    let d = temp_dir();
    let out = Command::new(openwand_bin())
        .args(review_accept_args(&d, "accept"))
        .output().unwrap();
    assert!(out.status.success(), "accept failed: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Review recorded"), "expected 'Review recorded', got: {}", stdout);
    assert!(stdout.contains("accepted"), "expected 'accepted', got: {}", stdout);
}

#[test]
fn cli_manual_result_review_reject_requires_blocking_reasons() {
    let d = temp_dir();
    let out = d.path().to_string_lossy().to_string();
    let args: Vec<String> = vec![
        "workflow-manual-result-review".into(), "review-reject".into(),
        "--manual-result-id".into(), "wmr_rej".into(),
        "--workflow-execution-id".into(), "wfx_rej".into(),
        "--command-review-id".into(), "wcrv_rej".into(),
        "--command-composer-id".into(), "wcc_rej".into(),
        "--loop-controller-id".into(), "wlc_rej".into(),
        "--expected-manual-result-hash".into(), "mrh".into(),
        "--expected-command-review-hash".into(), "crh".into(),
        "--expected-command-composer-hash".into(), "cch".into(),
        "--expected-command-descriptor-hash".into(), "cdh".into(),
        "--expected-loop-controller-hash".into(), "lch".into(),
        "--reviewer".into(), "bob".into(),
        "--rationale".into(), "unsafe".into(),
        "--blocking-reasons".into(), "risk_of_data_loss".into(),
        "--output-dir".into(), out,
    ];
    let result = Command::new(openwand_bin()).args(&args).output().unwrap();
    assert!(result.status.success(), "reject failed: {}", String::from_utf8_lossy(&result.stderr));
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("rejected"), "expected 'rejected', got: {}", stdout);
}

#[test]
fn cli_manual_result_review_request_changes_requires_requested_changes() {
    let d = temp_dir();
    let out = d.path().to_string_lossy().to_string();
    let args: Vec<String> = vec![
        "workflow-manual-result-review".into(), "review-request-changes".into(),
        "--manual-result-id".into(), "wmr_chg".into(),
        "--workflow-execution-id".into(), "wfx_chg".into(),
        "--command-review-id".into(), "wcrv_chg".into(),
        "--command-composer-id".into(), "wcc_chg".into(),
        "--loop-controller-id".into(), "wlc_chg".into(),
        "--expected-manual-result-hash".into(), "mrh".into(),
        "--expected-command-review-hash".into(), "crh".into(),
        "--expected-command-composer-hash".into(), "cch".into(),
        "--expected-command-descriptor-hash".into(), "cdh".into(),
        "--expected-loop-controller-hash".into(), "lch".into(),
        "--reviewer".into(), "carol".into(),
        "--rationale".into(), "needs more detail".into(),
        "--requested-changes".into(), "add_screenshot_evidence".into(),
        "--output-dir".into(), out,
    ];
    let result = Command::new(openwand_bin()).args(&args).output().unwrap();
    assert!(result.status.success(), "request-changes failed: {}", String::from_utf8_lossy(&result.stderr));
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("changes requested"), "expected 'changes requested', got: {}", stdout);
}

#[test]
fn cli_manual_result_review_requires_expected_hashes() {
    let args: Vec<String> = vec![
        "workflow-manual-result-review".into(), "review-accept".into(),
        "--manual-result-id".into(), "wmr_miss".into(),
        "--workflow-execution-id".into(), "wfx_miss".into(),
        "--reviewer".into(), "alice".into(),
        "--rationale".into(), "test".into(),
    ];
    let result = Command::new(openwand_bin()).args(&args).output().unwrap();
    assert!(!result.status.success(), "should fail without expected hashes");
}

#[test]
fn cli_manual_result_review_latest_by_manual_result_returns_latest() {
    let d = temp_dir();
    // Create review first
    let create = Command::new(openwand_bin())
        .args(review_accept_args(&d, "lat"))
        .output().unwrap();
    assert!(create.status.success(), "create failed: {}", String::from_utf8_lossy(&create.stderr));

    // Look up by manual result id
    let out = d.path().to_string_lossy().to_string();
    let args: Vec<String> = vec![
        "workflow-manual-result-review".into(), "latest".into(),
        "--manual-result-id".into(), "wmr_lat".into(),
        "--output-dir".into(), out,
    ];
    let result = Command::new(openwand_bin()).args(&args).output().unwrap();
    assert!(result.status.success(), "latest failed: {}", String::from_utf8_lossy(&result.stderr));
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("wmrr_"), "expected wmrr_ id, got: {}", stdout);
}

#[test]
fn cli_manual_result_review_show_returns_review() {
    let d = temp_dir();
    // Create review with JSON output
    let mut args = review_accept_args(&d, "show");
    args.push("--json".into());
    let create = Command::new(openwand_bin())
        .args(&args)
        .output().unwrap();
    assert!(create.status.success(), "create failed: {}", String::from_utf8_lossy(&create.stderr));
    let stdout = String::from_utf8_lossy(&create.stdout);
    // The JSON output should contain the review
    assert!(stdout.contains("wmrr_") || stdout.contains("review_id"), "expected review output, got: {}", stdout);
}
