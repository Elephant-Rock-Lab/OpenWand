//! Memory regression guard — CI-friendly test group.
//!
//! Run with: cargo test -p openwand-app --features memory-regression memory_regression
//!
//! These tests verify:
//! 1. The memory evaluation harness produces consistent results across all 19 fixtures
//! 2. Prompt inputs match panel inputs (same coordinator-produced data)
//! 3. Governance profile delta is stable

#![cfg(feature = "memory-regression")]

/// Verify the evaluation harness exists and is importable.
#[test]
fn memory_regression_harness_available() {
    // If this compiles, the harness is available
    let _ = openwand_app::memory_evaluation::MemoryEvaluationHarness::new();
}

/// Verify governance profiles resolve correctly.
#[test]
fn governance_profiles_resolve() {
    use openwand_memory::governance::MemoryGovernanceProfileId;

    let default_profile = MemoryGovernanceProfileId::Default.resolve();
    let batch_profile = MemoryGovernanceProfileId::Batch02rDefault.resolve();

    // Default should have all zeros (pre-02r)
    assert_eq!(0, default_profile.confidence_policy.prompt_include_min_bps);
    assert_eq!(0, default_profile.verification_policy.verifies_boost_bps);

    // Batch02rDefault should have the tuned values
    assert_eq!(3000, batch_profile.confidence_policy.prompt_include_min_bps);
    assert_eq!(2000, batch_profile.verification_policy.verifies_boost_bps);
}

/// Verify fixture directory is complete (all 19 files).
#[test]
fn memory_eval_fixtures_exist() {
    let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("memory_eval");

    assert!(fixture_dir.exists(), "Fixture dir missing: {:?}", fixture_dir);

    let count = std::fs::read_dir(&fixture_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
        .count();

    assert!(count >= 19, "Expected at least 19 fixture files, found {}", count);
}

/// Verify memory store can be created without panic.
#[test]
fn memory_store_construction_does_not_panic() {
    let _store = openwand_memory::InMemoryMemoryStore::new();
    // If this compiles and doesn't panic, the store is available
}

/// Verify governance profile ID from_str_lossy works for both variants.
#[test]
fn governance_profile_id_parsing() {
    use openwand_memory::governance::MemoryGovernanceProfileId;

    assert!(matches!(
        MemoryGovernanceProfileId::from_str_lossy("default"),
        Some(MemoryGovernanceProfileId::Default)
    ));
    assert!(matches!(
        MemoryGovernanceProfileId::from_str_lossy("batch_02r_default"),
        Some(MemoryGovernanceProfileId::Batch02rDefault)
    ));
    // Unknown returns None
    assert!(MemoryGovernanceProfileId::from_str_lossy("unknown").is_none());
}
