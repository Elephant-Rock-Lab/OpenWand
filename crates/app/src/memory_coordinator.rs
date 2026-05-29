//! Memory projection coordinator.
//!
//! Subscribes to run completions and automatically runs memory projection.
//! Lives in the app crate because it bridges trace, memory, and session.
//!
//! Memory failures are non-fatal to the run loop.

use openwand_core::SessionId;
use openwand_memory::{
    EpisodeRole, MemoryEpisode, MemoryExtractor, MemoryStore, MemoryQuery,
};
use openwand_memory::prompt_assembly::{MemoryPromptAssemblyInputs, RepoConsistencyPromptAssembler};
use openwand_memory::repo_consistency::{
    classify_current_claim, detect_missing_in_memory, observe_repo,
    RepoConsistencyFinding, RepoConsistencyReport,
    RepoMemoryInputSummary, RepoObservationSummary, StdRepoReadFs,
};
use openwand_memory::retrieval::RankedMemoryHit;
use openwand_memory::supersession::RetrievalMode;
use openwand_store::StoredEvent;
use openwand_trace::{TraceQuery, TraceStore};
use std::path::Path;
use std::sync::Arc;

/// Result of a projection run.
#[derive(Debug, Clone)]
pub struct ProjectionResult {
    pub episodes_projected: usize,
    pub candidates_extracted: usize,
    pub records_accepted: usize,
    pub errors: Vec<String>,
}

impl ProjectionResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Configuration for prompt input production.
/// Caps the work done per turn completion to avoid unbounded O(N) queries.
#[derive(Debug, Clone)]
pub struct PromptInputProductionConfig {
    /// Maximum number of active records to check against the repo.
    pub max_records_checked: usize,
    /// Maximum number of ranked hits to retrieve per record.
    pub max_hits_per_record: usize,
}

impl Default for PromptInputProductionConfig {
    fn default() -> Self {
        Self {
            max_records_checked: 100,
            max_hits_per_record: 5,
        }
    }
}

/// Result of prompt input production.
#[derive(Debug, Clone)]
pub struct PromptInputResult {
    pub inputs: MemoryPromptAssemblyInputs,
    pub claims_checked: usize,
    pub repo_observed: bool,
    pub source_session_id: Option<SessionId>,
    pub source_working_directory: std::path::PathBuf,
    pub errors: Vec<String>,
}

/// Coordinates automatic memory projection after session runs.
pub struct MemoryCoordinator {
    memory_store: Arc<dyn MemoryStore>,
    extractor: Arc<dyn MemoryExtractor>,
    trace: Arc<dyn TraceStore<StoredEvent>>,
}

impl MemoryCoordinator {
    pub fn new(
        memory_store: Arc<dyn MemoryStore>,
        extractor: Arc<dyn MemoryExtractor>,
        trace: Arc<dyn TraceStore<StoredEvent>>,
    ) -> Self {
        Self {
            memory_store,
            extractor,
            trace,
        }
    }

    /// Run projection for a session after a run completes.
    /// Errors are captured in the result, not propagated.
    pub async fn project_after_run(
        &self,
        session_id: &SessionId,
    ) -> ProjectionResult {
        let mut result = ProjectionResult {
            episodes_projected: 0,
            candidates_extracted: 0,
            records_accepted: 0,
            errors: Vec::new(),
        };

        // Scan trace entries for this session
        let query = TraceQuery {
            stream_id: Some(openwand_trace::TraceStreamId {
                scope: openwand_trace::TraceStreamScope::Session,
                id: session_id.to_string(),
            }),
            limit: Some(1000),
            ..Default::default()
        };

        let scan_result = match self.trace.scan(query).await {
            Ok(r) => r,
            Err(e) => {
                result.errors.push(format!("scan trace: {e}"));
                return result;
            }
        };

        // Project each relevant trace entry as an episode
        for entry in &scan_result.entries {
            let episode = match Self::trace_entry_to_episode(entry, session_id) {
                Some(ep) => ep,
                None => continue,
            };

            match self.memory_store.project_episode(episode).await {
                Ok(()) => result.episodes_projected += 1,
                Err(e) => result.errors.push(format!("project episode: {e}")),
            }
        }

        // Extract candidates from all episodes for this session
        let episodes = match self
            .memory_store
            .get_episodes(&session_id.to_string())
            .await
        {
            Ok(eps) => eps,
            Err(e) => {
                result.errors.push(format!("get episodes: {e}"));
                return result;
            }
        };

        let candidates = self.extractor.extract(&episodes).await;
        result.candidates_extracted = candidates.len();

        for candidate in candidates {
            match self.memory_store.accept_candidate(candidate).await {
                Ok(Some(_)) => result.records_accepted += 1,
                Ok(None) => {}
                Err(e) => result.errors.push(format!("accept candidate: {e}")),
            }
        }

        result
    }

    /// Manual rebuild: re-project everything. Idempotent by source_trace_id.
    pub async fn rebuild_from_trace(
        &self,
        session_id: &SessionId,
    ) -> ProjectionResult {
        self.project_after_run(session_id).await
    }

    /// Produce 02k prompt inputs from current memory, checked against repo.
    ///
    /// Steps:
    /// 1. Call `list_active_records()` on the memory store
    /// 2. Sort deterministically: active first → higher confidence → newer created_at → stable record_id
    /// 3. Cap at `config.max_records_checked`
    /// 4. For each selected record, call `search_ranked(claim_text, CurrentState)`
    ///    capped at `config.max_hits_per_record`
    /// 5. Call `observe_repo(&StdRepoReadFs, working_directory)` to snapshot the repo
    /// 6. For each hit, call `classify_current_claim()` against observed crates/files/deps
    /// 7. Call `detect_missing_in_memory()` to find repo items with no memory claim
    /// 8. Collect findings into a `RepoConsistencyReport`
    /// 9. Call `RepoConsistencyPromptAssembler::assemble_from_report(&report)`
    /// 10. Return the `MemoryPromptAssemblyInputs`
    ///
    /// If any step fails (no Cargo.toml, empty memory, store error):
    /// returns `MemoryPromptAssemblyInputs::empty()` with `repo_observed: false`.
    /// Never propagates errors to the caller — prompt input production is non-fatal.
    pub async fn produce_prompt_inputs(
        &self,
        session_id: Option<SessionId>,
        working_directory: &Path,
        config: &PromptInputProductionConfig,
    ) -> PromptInputResult {
        let make_empty = |errors: Vec<String>| PromptInputResult {
            inputs: MemoryPromptAssemblyInputs::empty(),
            claims_checked: 0,
            repo_observed: false,
            source_session_id: session_id.clone(),
            source_working_directory: working_directory.to_path_buf(),
            errors,
        };

        // Step 1: Get all active records
        let mut records = match self.memory_store.list_active_records().await {
            Ok(r) => r,
            Err(e) => return make_empty(vec![e.to_string()]),
        };
        if records.is_empty() {
            return make_empty(vec![]);
        }

        // Step 2: Deterministic sort
        // active (not superseded) first → higher confidence → newer created_at → stable record_id
        records.sort_by(|a, b| {
            let a_active = a.superseded_by.is_none();
            let b_active = b.superseded_by.is_none();
            b_active.cmp(&a_active)
                .then_with(|| {
                    b.confidence
                        .partial_cmp(&a.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| b.created_at.cmp(&a.created_at))
                .then_with(|| a.record_id.cmp(&b.record_id))
        });

        // Step 3: Cap
        records.truncate(config.max_records_checked);
        let claims_to_check = records.len();

        // Step 4: search_ranked per record, capped hits
        let mut all_hits: Vec<RankedMemoryHit> = Vec::new();
        let mut search_errors: Vec<String> = Vec::new();
        for record in &records {
            let query = MemoryQuery::new(&record.claim);
            match self
                .memory_store
                .search_ranked(query, RetrievalMode::CurrentState)
                .await
            {
                Ok(ctx) => {
                    all_hits
                        .extend(ctx.hits.into_iter().take(config.max_hits_per_record));
                }
                Err(e) => {
                    search_errors
                        .push(format!("search_ranked for '{}': {}", record.claim, e));
                }
            }
        }

        // Guard: if every ranked search failed, do not produce false missing-memory findings
        if all_hits.is_empty() && !records.is_empty() && !search_errors.is_empty() {
            return PromptInputResult {
                inputs: MemoryPromptAssemblyInputs::empty(),
                claims_checked: claims_to_check,
                repo_observed: false,
                source_session_id: session_id,
                source_working_directory: working_directory.to_path_buf(),
                errors: search_errors,
            };
        }

        // Step 5: Observe repo
        let snapshot = match observe_repo(&StdRepoReadFs, working_directory) {
            Ok(s) => s,
            Err(e) => {
                return PromptInputResult {
                    inputs: MemoryPromptAssemblyInputs::empty(),
                    claims_checked: claims_to_check,
                    repo_observed: false,
                    source_session_id: session_id,
                    source_working_directory: working_directory.to_path_buf(),
                    errors: vec![format!("observe_repo: {e}")],
                };
            }
        };

        // Step 6: Classify each hit
        let crate_names: Vec<String> = snapshot.crates.iter().map(|c| c.name.clone()).collect();
        let deps: Vec<(String, String)> = snapshot
            .dependencies
            .iter()
            .map(|d| (d.crate_name.clone(), d.dependency_name.clone()))
            .collect();
        let src_files: Vec<String> = snapshot
            .crates
            .iter()
            .flat_map(|c| c.src_files.clone())
            .collect();

        let mut findings: Vec<RepoConsistencyFinding> = Vec::new();
        for hit in &all_hits {
            findings.push(classify_current_claim(hit, &crate_names, &src_files, &deps));
        }

        // Step 7: Missing-in-memory
        let missing = detect_missing_in_memory(&snapshot, &all_hits);
        findings.extend(missing);

        // Step 8: Build report
        let report = RepoConsistencyReport {
            repo_root: working_directory.to_path_buf(),
            checked_at: chrono::Utc::now(),
            summary: openwand_memory::repo_consistency::RepoConsistencySummary::from_findings(
                &findings,
            ),
            findings,
            memory_inputs: RepoMemoryInputSummary::default(),
            repo_inputs: RepoObservationSummary::default(),
        };

        // Step 9: Assemble
        let inputs = RepoConsistencyPromptAssembler::assemble_from_report(&report);

        PromptInputResult {
            claims_checked: claims_to_check,
            repo_observed: true,
            inputs,
            source_session_id: session_id,
            source_working_directory: working_directory.to_path_buf(),
            errors: search_errors,
        }
    }

    /// Convert a trace entry to a memory episode, if relevant.
    fn trace_entry_to_episode(
        entry: &openwand_trace::TraceEntry<StoredEvent>,
        session_id: &SessionId,
    ) -> Option<MemoryEpisode> {
        use openwand_core::events::OpenWandTraceEvent;
        use chrono::Utc;

        let event: &OpenWandTraceEvent = &entry.event;

        match event {
            OpenWandTraceEvent::Session(
                openwand_core::events::SessionEvent::UserMessageInjected { text },
            ) => Some(MemoryEpisode {
                episode_id: format!("ep_{}", entry.id),
                source_trace_id: entry.id.0.clone(),
                session_id: session_id.to_string(),
                event_kind: "session.user_message_injected".into(),
                role: EpisodeRole::User,
                content: text.clone(),
                created_at: Utc::now(),
            }),

            OpenWandTraceEvent::Session(
                openwand_core::events::SessionEvent::AssistantMessageGenerated { text, .. },
            ) => Some(MemoryEpisode {
                episode_id: format!("ep_{}", entry.id),
                source_trace_id: entry.id.0.clone(),
                session_id: session_id.to_string(),
                event_kind: "session.assistant_message_generated".into(),
                role: EpisodeRole::Assistant,
                content: text.clone(),
                created_at: Utc::now(),
            }),

            _ => None,
        }
    }
}
