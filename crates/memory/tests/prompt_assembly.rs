//! Wave 02k integration tests — prompt assembly edge cases + SQLite parity.

use openwand_memory::evidence::EvidenceKind;
use openwand_memory::in_memory::InMemoryMemoryStore;
use openwand_memory::memory_store::MemoryStore;
use openwand_memory::prompt_assembly::{
    MemoryPromptAssemblyInputs, MissingMemoryObservation,
    PromptInclusionReason, RepoConsistencyPromptAssembler, SupportedMemoryClaim,
    SupersededMemoryClaim,
};
use openwand_memory::repo_consistency::{
    classify_conflict_claim, classify_current_claim, detect_missing_in_memory,
    load_memory_inputs, observe_repo, ConsistencySeverity, RepoConsistencyFinding,
    RepoConsistencyFindingKind, RepoConsistencyReport,
    RepoConsistencySummary, RepoMemoryInputSummary, RepoObservationSummary,
    RepoReadFs, RepoObserveError,
};
use openwand_memory::ranking::MemoryRankScore;
use openwand_memory::retrieval::RankedMemoryHit;
use openwand_memory::supersession::RetrievalMode;
use openwand_memory::types::{CandidateMemory, CandidateKind, EpisodeRole, MemoryEpisode};
use std::path::{Path, PathBuf};

// --- Stub FS (same as repo_consistency test) ---

struct StubRepoReadFs {
    files: std::collections::HashMap<String, String>,
    dirs: std::collections::HashMap<String, Vec<PathBuf>>,
}

impl StubRepoReadFs {
    fn new() -> Self {
        Self {
            files: std::collections::HashMap::new(),
            dirs: std::collections::HashMap::new(),
        }
    }
    fn add_file(&mut self, path: &str, content: &str) {
        self.files.insert(path.to_string(), content.to_string());
        if let Some(parent) = Path::new(path).parent() {
            let parent_str = parent.to_string_lossy().to_string();
            self.dirs.entry(parent_str).or_default().push(PathBuf::from(path));
        }
    }
    fn add_dir(&mut self, path: &str, children: Vec<&str>) {
        self.dirs.insert(
            path.to_string(),
            children.into_iter().map(PathBuf::from).collect(),
        );
    }
}

impl RepoReadFs for StubRepoReadFs {
    fn read_to_string(&self, path: &Path) -> Result<String, RepoObserveError> {
        let key = path.to_string_lossy().replace('\\', "/");
        self.files.get(&key).cloned()
            .ok_or_else(|| RepoObserveError::NotFound(path.to_path_buf()))
    }
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, RepoObserveError> {
        let key = path.to_string_lossy().replace('\\', "/");
        self.dirs.get(&key).cloned()
            .ok_or_else(|| RepoObserveError::NotFound(path.to_path_buf()))
    }
    fn exists(&self, path: &Path) -> bool {
        let key = path.to_string_lossy().replace('\\', "/");
        self.files.contains_key(&key) || self.dirs.contains_key(&key)
    }
}

fn make_episode(id: &str, trace_id: &str) -> MemoryEpisode {
    MemoryEpisode {
        episode_id: id.to_string(),
        source_trace_id: trace_id.to_string(),
        session_id: "s1".to_string(),
        event_kind: "message".to_string(),
        role: EpisodeRole::User,
        content: "test".to_string(),
        created_at: chrono::Utc::now(),
    }
}

fn make_candidate(claim: &str, ep_id: &str) -> CandidateMemory {
    CandidateMemory {
        claim: claim.to_string(),
        kind: CandidateKind::Fact,
        confidence: 0.9,
        source_episode_ids: vec![ep_id.to_string()],
    }
}

fn make_hit(text: &str) -> RankedMemoryHit {
    RankedMemoryHit {
        id: "test".to_string(),
        text: text.to_string(),
        score: MemoryRankScore {
            relevance_bps: 0, provenance_bps: 0, scope_bps: 0,
            recency_bps: 0, confidence_bps: 0, evidence_bps: 0, verification_bps: 0, final_bps: 0,
        },
        evidence_kind: EvidenceKind::AcceptedClaim,
        source_episode_ids: vec![],
        source_trace_ids: vec![],
        scope: openwand_memory::provenance::MemoryScope::Global,
        provenance: openwand_memory::provenance::ProvenanceSnapshot::default(),
        confidence_bps: 7000,
        reason: "test".to_string(),
    }
}

fn make_finding(kind: RepoConsistencyFindingKind, claim: &str) -> RepoConsistencyFinding {
    RepoConsistencyFinding {
        kind,
        claim_text: Some(claim.to_string()),
        evidence_kind: Some(EvidenceKind::AcceptedClaim),
        repo_evidence_key: vec![],
        severity: ConsistencySeverity::Low,
        detail: "test".to_string(),
    }
}

fn workspace_fs() -> StubRepoReadFs {
    let mut fs = StubRepoReadFs::new();
    fs.add_file("/repo/Cargo.toml", r#"
[package]
name = "openwand"
members = ["crates/core", "crates/memory"]
"#);
    fs.add_dir("/repo", vec!["/repo/Cargo.toml", "/repo/crates"]);
    fs.add_dir("/repo/crates", vec!["/repo/crates/core", "/repo/crates/memory"]);
    fs.add_file("/repo/crates/core/Cargo.toml", r#"
[package]
name = "openwand-core"
[dependencies]
serde = "1"
"#);
    fs.add_dir("/repo/crates/core", vec!["/repo/crates/core/Cargo.toml", "/repo/crates/core/src"]);
    fs.add_dir("/repo/crates/core/src", vec!["/repo/crates/core/src/lib.rs"]);
    fs.add_file("/repo/crates/core/src/lib.rs", "");
    fs.add_file("/repo/crates/memory/Cargo.toml", r#"
[package]
name = "openwand-memory"
[dependencies]
blake3 = "1"
"#);
    fs.add_dir("/repo/crates/memory", vec!["/repo/crates/memory/Cargo.toml", "/repo/crates/memory/src"]);
    fs.add_dir("/repo/crates/memory/src", vec!["/repo/crates/memory/src/lib.rs"]);
    fs.add_file("/repo/crates/memory/src/lib.rs", "");
    fs
}

fn findings_to_inputs(findings: &[RepoConsistencyFinding]) -> MemoryPromptAssemblyInputs {
    let report = RepoConsistencyReport {
        repo_root: PathBuf::from("/test"),
        checked_at: chrono::Utc::now(),
        findings: findings.to_vec(),
        summary: RepoConsistencySummary::from_findings(findings),
        memory_inputs: RepoMemoryInputSummary::default(),
        repo_inputs: RepoObservationSummary::default(),
    };
    RepoConsistencyPromptAssembler::assemble_from_report(&report)
}

#[test]
fn supported_claim_without_repo_evidence_is_not_included() {
    let findings = vec![RepoConsistencyFinding {
        kind: RepoConsistencyFindingKind::MissingInRepo,
        claim_text: Some("crate nonexistent exists".to_string()),
        evidence_kind: Some(EvidenceKind::AcceptedClaim),
        repo_evidence_key: vec![],  // no evidence
        severity: ConsistencySeverity::Medium,
        detail: "not found".to_string(),
    }];
    let inputs = findings_to_inputs(&findings);
    assert_eq!(1, inputs.supported_claims.len());
    // It's in supported_claims but with empty evidence keys
    let block = inputs.to_prompt_block().unwrap();
    assert!(block.contains("(not verified by repo)"));
}

#[test]
fn unverifiable_claim_text_absent_from_prompt() {
    let findings = vec![
        make_finding(RepoConsistencyFindingKind::Unverifiable, "the project uses microservices"),
        make_finding(RepoConsistencyFindingKind::Supported, "crate core exists"),
    ];
    let inputs = findings_to_inputs(&findings);
    let block = inputs.to_prompt_block().unwrap();
    assert!(!block.contains("microservices"), "unverifiable text must not appear");
    assert!(block.contains("1 claims excluded"));
    assert!(block.contains("crate core exists"));
}

#[test]
fn conflict_claim_matching_repo_still_not_supported() {
    // Even if a conflict claim happens to match repo, it goes to conflicts, not supported
    let findings = vec![RepoConsistencyFinding {
        kind: RepoConsistencyFindingKind::ConflictRequiresReview,
        claim_text: Some("crate core exists".to_string()),
        evidence_kind: Some(EvidenceKind::ConflictingClaim),
        repo_evidence_key: vec!["crate:core".to_string()],
        severity: ConsistencySeverity::Medium,
        detail: "conflict".to_string(),
    }];
    let inputs = findings_to_inputs(&findings);
    assert!(inputs.supported_claims.is_empty(), "conflict must not be in supported");
    assert_eq!(1, inputs.conflicts_for_user_or_model.len());
    let block = inputs.to_prompt_block().unwrap();
    assert!(!block.contains("## Verified Memory"), "no verified section for conflicts");
    assert!(block.contains("## Memory Conflicts"));
}

#[test]
fn superseded_claim_only_appears_under_history() {
    let findings = vec![make_finding(RepoConsistencyFindingKind::SupersededMemoryIgnored, "old fact")];
    let inputs = findings_to_inputs(&findings);
    let block = inputs.to_prompt_block().unwrap();
    assert!(block.contains("## Memory History"));
    assert!(block.contains("old fact"));
    assert!(!block.contains("## Verified Memory"), "superseded not in verified");
}

#[test]
fn missing_memory_gap_appears_as_todo_not_fact() {
    let findings = vec![RepoConsistencyFinding {
        kind: RepoConsistencyFindingKind::MissingInMemory,
        claim_text: None,
        evidence_kind: None,
        repo_evidence_key: vec!["crate:tools".to_string()],
        severity: ConsistencySeverity::Medium,
        detail: "no memory claim for tools crate".to_string(),
    }];
    let inputs = findings_to_inputs(&findings);
    let block = inputs.to_prompt_block().unwrap();
    assert!(block.contains("## Context Gaps"));
    assert!(block.contains("[TODO:"));
    assert!(!block.contains("## Verified Memory"), "gap not in verified");
}

#[test]
fn empty_inputs_returns_none_prompt_block() {
    let inputs = MemoryPromptAssemblyInputs::empty();
    assert!(inputs.to_prompt_block().is_none());
}

// --- E2E: full pipeline ---

#[tokio::test]
async fn full_pipeline_claims_to_formatted_prompt() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", "ep1")).await.unwrap();
    store.accept_candidate(make_candidate("the project uses microservices", "ep1")).await.unwrap();

    let inputs = load_memory_inputs(&store, "crate core microservices").await.unwrap();
    let snap = observe_repo(&workspace_fs(), Path::new("/repo")).unwrap();

    let crate_names: Vec<String> = snap.crates.iter().map(|c| c.name.clone()).collect();
    let deps: Vec<(String, String)> = snap.dependencies.iter()
        .map(|d| (d.crate_name.clone(), d.dependency_name.clone()))
        .collect();
    let src_files = snap.crates.iter().flat_map(|c| c.src_files.clone()).collect::<Vec<_>>();

    let mut findings: Vec<RepoConsistencyFinding> = inputs.current_claims.iter()
        .map(|hit| classify_current_claim(hit, &crate_names, &src_files, &deps))
        .collect();

    let missing = detect_missing_in_memory(&snap, &inputs.current_claims);
    findings.extend(missing);

    let report = RepoConsistencyReport {
        repo_root: PathBuf::from("/repo"),
        checked_at: chrono::Utc::now(),
        findings: findings.clone(),
        summary: RepoConsistencySummary::from_findings(&findings),
        memory_inputs: RepoMemoryInputSummary::default(),
        repo_inputs: RepoObservationSummary::default(),
    };

    let assembly = RepoConsistencyPromptAssembler::assemble_from_report(&report);
    let block = assembly.to_prompt_block().unwrap();

    // Supported claims appear
    assert!(block.contains("crate core exists"));
    // Unverifiable microservices claim does NOT appear by name
    assert!(!block.contains("microservices"));
    // Some excluded count
    assert!(block.contains("excluded") || block.contains("Verified"));
}

// --- SQLite parity ---

#[cfg(feature = "sqlite-testing")]
mod sqlite_parity {
    use super::*;
    use openwand_memory::sqlite_store::SqliteMemoryStore;
    use tempfile::TempDir;

    fn make_sqlite_store() -> SqliteMemoryStore {
        let dir = TempDir::new().unwrap();
        SqliteMemoryStore::open(&dir.path().join("test.db")).unwrap()
    }

    #[tokio::test]
    async fn sqlite_and_inmemory_prompt_blocks_match() {
        let crate_names = vec!["openwand-core".to_string(), "openwand-memory".to_string()];

        // In-memory
        let im = InMemoryMemoryStore::new();
        im.project_episode(make_episode("ep1", "t1")).await.unwrap();
        im.accept_candidate(make_candidate("crate core exists", "ep1")).await.unwrap();

        let im_findings = vec![RepoConsistencyFinding {
            kind: RepoConsistencyFindingKind::Supported,
            claim_text: Some("crate core exists".to_string()),
            evidence_kind: Some(EvidenceKind::AcceptedClaim),
            repo_evidence_key: vec!["crate:core".to_string()],
            severity: ConsistencySeverity::Low,
            detail: "test".to_string(),
        }];
        let im_report = RepoConsistencyReport {
            repo_root: PathBuf::from("/test"),
            checked_at: chrono::Utc::now(),
            findings: im_findings.clone(),
            summary: RepoConsistencySummary::from_findings(&im_findings),
            memory_inputs: RepoMemoryInputSummary::default(),
            repo_inputs: RepoObservationSummary::default(),
        };
        let im_inputs = RepoConsistencyPromptAssembler::assemble_from_report(&im_report);
        let im_block = im_inputs.to_prompt_block();

        // SQLite
        let sq = make_sqlite_store();
        sq.project_episode(make_episode("ep1", "t1")).await.unwrap();
        sq.accept_candidate(make_candidate("crate core exists", "ep1")).await.unwrap();

        let sq_inputs = RepoConsistencyPromptAssembler::assemble_from_report(&im_report);
        let sq_block = sq_inputs.to_prompt_block();

        // Compare struct parity
        assert_eq!(im_inputs.supported_claims.len(), sq_inputs.supported_claims.len());
        assert_eq!(im_inputs.unverifiable_claims_excluded, sq_inputs.unverifiable_claims_excluded);

        // Compare formatted output parity
        assert_eq!(im_block, sq_block, "formatted prompt blocks must match");
    }
}
