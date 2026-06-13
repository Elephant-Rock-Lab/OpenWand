//! End-to-end consistency report tests.

use openwand_memory::evidence::EvidenceKind;
use openwand_memory::in_memory::InMemoryMemoryStore;
use openwand_memory::memory_store::MemoryStore;
use openwand_memory::query::MemoryQuery;
use openwand_memory::repo_consistency::{
    classify_conflict_claim, classify_current_claim, classify_superseded_claim,
    detect_missing_in_memory, load_memory_inputs, observe_repo,
    parse_claim, match_claim, RepoClaimPattern,
    FixedClock, RepoConsistencyChecker, RepoConsistencyReport,
    RepoConsistencyFindingKind, ConsistencySeverity,
    RepoReadFs, RepoObserveError,
};
use openwand_memory::supersession::RetrievalMode;
use openwand_memory::types::{CandidateMemory, CandidateKind, EpisodeRole, MemoryEpisode};
use std::path::{Path, PathBuf};

/// Stub filesystem for integration tests.
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

fn workspace_fs() -> StubRepoReadFs {
    let mut fs = StubRepoReadFs::new();
    fs.add_file("/repo/Cargo.toml", r#"
[package]
name = "openwand"
members = ["crates/core", "crates/memory"]
"#);
    fs.add_dir("/repo", vec!["/repo/Cargo.toml", "/repo/crates", "/repo/docs"]);
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
serde = "1"
blake3 = "1"
"#);
    fs.add_dir("/repo/crates/memory", vec!["/repo/crates/memory/Cargo.toml", "/repo/crates/memory/src"]);
    fs.add_dir("/repo/crates/memory/src", vec!["/repo/crates/memory/src/lib.rs"]);
    fs.add_file("/repo/crates/memory/src/lib.rs", "");
    fs.add_dir("/repo/docs", vec!["/repo/docs/LOCK.md"]);
    fs.add_file("/repo/docs/LOCK.md", "");
    fs
}

#[tokio::test]
async fn repo_consistency_supported_project_e2e() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.accept_candidate(make_candidate("crate core exists", "ep1")).await.unwrap();
    store.accept_candidate(make_candidate("workspace contains crate memory", "ep1")).await.unwrap();
    store.accept_candidate(make_candidate("crate core depends on serde", "ep1")).await.unwrap();

    let inputs = load_memory_inputs(&store, "crate core memory serde").await.unwrap();
    let snap = observe_repo(&workspace_fs(), Path::new("/repo")).unwrap();

    let crate_names: Vec<String> = snap.crates.iter().map(|c| c.name.clone()).collect();
    let deps: Vec<(String, String)> = snap.dependencies.iter()
        .map(|d| (d.crate_name.clone(), d.dependency_name.clone()))
        .collect();
    let src_files = snap.crates.iter().flat_map(|c| c.src_files.clone()).collect::<Vec<_>>();

    let mut findings = vec![];
    for hit in &inputs.current_claims {
        findings.push(classify_current_claim(hit, &crate_names, &src_files, &deps));
    }

    let supported = findings.iter().filter(|f| f.kind == RepoConsistencyFindingKind::Supported).count();
    assert!(supported >= 2, "at least core and memory should be supported");
}

#[tokio::test]
async fn repo_consistency_stale_memory_e2e() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.accept_candidate(make_candidate("crate nonexistent exists", "ep1")).await.unwrap();

    let inputs = load_memory_inputs(&store, "crate nonexistent").await.unwrap();
    let snap = observe_repo(&workspace_fs(), Path::new("/repo")).unwrap();

    let crate_names: Vec<String> = snap.crates.iter().map(|c| c.name.clone()).collect();

    let findings: Vec<_> = inputs.current_claims.iter()
        .map(|hit| classify_current_claim(hit, &crate_names, &[], &[]))
        .collect();

    let missing = findings.iter().filter(|f| f.kind == RepoConsistencyFindingKind::MissingInRepo).count();
    assert!(missing >= 1, "nonexistent crate should be missing in repo");
}

#[tokio::test]
async fn repo_consistency_conflict_e2e() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.project_episode(make_episode("ep2", "t2")).await.unwrap();

    let r1 = store.accept_candidate(make_candidate("prefer tabs for indentation", "ep1")).await.unwrap().unwrap();
    let r2 = store.accept_candidate(make_candidate("prefer spaces for indentation", "ep2")).await.unwrap().unwrap();

    // Set conflict group
    {
        let mut records = store.records.lock().unwrap();
        records.get_mut(&r1.record_id).unwrap().conflict_group_id = Some("cg_indent".to_string());
        records.get_mut(&r2.record_id).unwrap().conflict_group_id = Some("cg_indent".to_string());
    }

    let inputs = load_memory_inputs(&store, "indentation prefer").await.unwrap();
    // Both claims should be in conflict search results
    assert!(inputs.current_claims.len() >= 2);
}

#[tokio::test]
async fn repo_consistency_supersession_e2e() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    let old = store.accept_candidate(make_candidate("crate core exists", "ep1")).await.unwrap().unwrap();
    store.supersede_record(&old.record_id, "crate memory exists".to_string()).await.unwrap();

    let inputs = load_memory_inputs(&store, "crate core memory").await.unwrap();

    // Superseded claims should be in history
    let findings: Vec<_> = inputs.superseded_history.iter()
        .map(classify_superseded_claim)
        .collect();

    assert!(findings.iter().any(|f| f.kind == RepoConsistencyFindingKind::SupersededMemoryIgnored));
}

#[tokio::test]
async fn repo_consistency_missing_in_memory_e2e() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    // Only one claim about core
    store.accept_candidate(make_candidate("crate core exists", "ep1")).await.unwrap();

    let inputs = load_memory_inputs(&store, "crate core memory serde").await.unwrap();
    let snap = observe_repo(&workspace_fs(), Path::new("/repo")).unwrap();

    let findings = detect_missing_in_memory(&snap, &inputs.current_claims);
    // Should detect: memory crate, serde dep, blake3 dep, docs
    assert!(findings.len() >= 2, "should detect missing memory claims for repo observations");
}

#[tokio::test]
async fn repo_consistency_unverifiable_claim_e2e() {
    let store = InMemoryMemoryStore::new();
    store.project_episode(make_episode("ep1", "t1")).await.unwrap();
    store.accept_candidate(make_candidate("the project uses microservices", "ep1")).await.unwrap();

    let inputs = load_memory_inputs(&store, "project microservices").await.unwrap();
    let snap = observe_repo(&workspace_fs(), Path::new("/repo")).unwrap();

    let crate_names: Vec<String> = snap.crates.iter().map(|c| c.name.clone()).collect();

    let findings: Vec<_> = inputs.current_claims.iter()
        .map(|hit| classify_current_claim(hit, &crate_names, &[], &[]))
        .collect();

    assert!(findings.iter().any(|f| f.kind == RepoConsistencyFindingKind::Unverifiable));
}

#[test]
fn repo_consistency_no_git_mutation_e2e() {
    // This test documents that the consistency check never mutates git state.
    // RepoReadFs has no write methods — this is enforced at the type level.
    let fs = workspace_fs();
    let snap = observe_repo(&fs, Path::new("/repo")).unwrap();
    // No git operations were performed
    assert!(snap.cargo_package_name.is_some());
}

#[test]
fn repo_consistency_no_file_mutation_e2e() {
    // Two observations must produce identical results — nothing was written
    let fs = workspace_fs();
    let snap1 = observe_repo(&fs, Path::new("/repo")).unwrap();
    let snap2 = observe_repo(&fs, Path::new("/repo")).unwrap();
    assert_eq!(snap1.cargo_package_name, snap2.cargo_package_name);
    assert_eq!(snap1.crates.len(), snap2.crates.len());
    assert_eq!(snap1.dependencies.len(), snap2.dependencies.len());
}

// SQLite/in-memory parity test
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
    async fn sqlite_and_inmemory_report_same_findings() {
        let crate_names = vec!["openwand-core".to_string(), "openwand-memory".to_string()];
        let deps = vec![
            ("core".to_string(), "serde".to_string()),
            ("memory".to_string(), "blake3".to_string()),
        ];
        let src_files = vec![
            "crates/core/src/lib.rs".to_string(),
            "crates/memory/src/lib.rs".to_string(),
        ];

        // In-memory store
        let im_store = InMemoryMemoryStore::new();
        im_store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        im_store.accept_candidate(make_candidate("crate core exists", "ep1")).await.unwrap();

        let im_inputs = load_memory_inputs(&im_store, "crate core").await.unwrap();
        let im_findings: Vec<_> = im_inputs.current_claims.iter()
            .map(|hit| classify_current_claim(hit, &crate_names, &src_files, &deps))
            .collect();

        // SQLite store
        let sq_store = make_sqlite_store();
        sq_store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        sq_store.accept_candidate(make_candidate("crate core exists", "ep1")).await.unwrap();

        let sq_inputs = load_memory_inputs(&sq_store, "crate core").await.unwrap();
        let sq_findings: Vec<_> = sq_inputs.current_claims.iter()
            .map(|hit| classify_current_claim(hit, &crate_names, &src_files, &deps))
            .collect();

        // Compare normalized findings
        assert_eq!(im_findings.len(), sq_findings.len(), "finding count must match");
        for (im, sq) in im_findings.iter().zip(sq_findings.iter()) {
            assert_eq!(im.kind, sq.kind, "finding kind must match");
            assert_eq!(im.claim_text, sq.claim_text, "claim text must match");
        }
    }
}
