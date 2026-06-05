//! CLI integration tests for workflow-manual-result-reconciliation-readiness.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}
fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

fn evaluate_args(d: &tempfile::TempDir, suffix: &str) -> Vec<String> {
    let out = d.path().to_string_lossy().to_string();
    vec!["workflow-manual-result-reconciliation-readiness".into(), "evaluate".into(),
        "--workflow-execution-id".into(), format!("wfx_{}", suffix),
        "--manual-result-id".into(), format!("wmr_{}", suffix),
        "--manual-result-review-id".into(), format!("wmrr_{}", suffix),
        "--command-review-id".into(), format!("wcrv_{}", suffix),
        "--command-composer-id".into(), format!("wcc_{}", suffix),
        "--loop-controller-id".into(), format!("wlc_{}", suffix),
        "--expected-manual-result-review-hash".into(), "rrh".into(),
        "--expected-manual-result-hash".into(), "mrh".into(),
        "--expected-command-review-hash".into(), "crh".into(),
        "--expected-command-composer-hash".into(), "cch".into(),
        "--expected-command-descriptor-hash".into(), "cdh".into(),
        "--expected-loop-controller-hash".into(), "lch".into(),
        "--evaluator".into(), "test".into(),
        "--output-dir".into(), out,
    ]
}

#[test]
fn cli_evaluate_outputs_readiness_id() {
    let d = temp_dir();
    let out = Command::new(openwand_bin())
        .args(&evaluate_args(&d, "eval"))
        .output().unwrap();
    assert!(out.status.success(), "evaluate failed: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Readiness recorded"), "expected 'Readiness recorded', got: {}", stdout);
    assert!(stdout.contains("wmrrr_"), "expected wmrrr_ id, got: {}", stdout);
}

#[test]
fn cli_evaluate_requires_expected_hashes() {
    let args: Vec<String> = vec![
        "workflow-manual-result-reconciliation-readiness".into(), "evaluate".into(),
        "--workflow-execution-id".into(), "wfx_miss".into(),
        "--manual-result-id".into(), "wmr_miss".into(),
        "--evaluator".into(), "test".into(),
    ];
    let result = Command::new(openwand_bin()).args(&args).output().unwrap();
    assert!(!result.status.success(), "should fail without expected hashes");
}

#[test]
fn cli_show_returns_readiness() {
    let d = temp_dir();
    let mut args = evaluate_args(&d, "show");
    args.push("--json".into());
    let create = Command::new(openwand_bin()).args(&args).output().unwrap();
    assert!(create.status.success(), "create failed: {}", String::from_utf8_lossy(&create.stderr));
    let stdout = String::from_utf8_lossy(&create.stdout);
    let stdout_str = stdout.to_string();
    // Extract readiness_id from JSON (wmrrr_... pattern)
    let start = stdout_str.find("wmrrr_").unwrap();
    let end = stdout_str[start..].find('"').unwrap() + start;
    let readiness_id = &stdout_str[start..end];

    let out = d.path().to_string_lossy().to_string();
    let show = Command::new(openwand_bin())
        .args(["workflow-manual-result-reconciliation-readiness", "show", readiness_id, "--output-dir", &out])
        .output().unwrap();
    assert!(show.status.success(), "show failed: {}", String::from_utf8_lossy(&show.stderr));
    assert!(String::from_utf8_lossy(&show.stdout).contains(readiness_id));
}

#[test]
fn cli_latest_returns_latest() {
    let d = temp_dir();
    let create = Command::new(openwand_bin())
        .args(&evaluate_args(&d, "lat"))
        .output().unwrap();
    assert!(create.status.success());

    let out = d.path().to_string_lossy().to_string();
    let latest = Command::new(openwand_bin())
        .args(["workflow-manual-result-reconciliation-readiness", "latest",
            "--manual-result-id", "wmr_lat", "--output-dir", &out])
        .output().unwrap();
    assert!(latest.status.success());
    assert!(String::from_utf8_lossy(&latest.stdout).contains("wmrrr_"));
}
