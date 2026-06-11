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
use openwand_memory::provenance_hydration::{HydratedMemoryClaim, MemoryProvenanceHydrator};
use openwand_memory::trace_relation_hydration::{
    TraceEventAuditMetadata, TraceRelationAuditHydrator, TraceRelationAuditRow,
};
use openwand_memory::repo_consistency::{
    classify_current_claim, detect_missing_in_memory, observe_repo,
    RepoConsistencyFinding, RepoConsistencyReport,
    RepoMemoryInputSummary, RepoObservationSummary, StdRepoReadFs,
};
use openwand_memory::retrieval::RankedMemoryHit;
use openwand_memory::supersession::RetrievalMode;
use openwand_store::StoredEvent;
use openwand_trace::{RelationQuery, TraceQuery, TraceStore};
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
    /// Governance profile. If None, pre-02r behavior is preserved.
    pub governance_profile: Option<openwand_memory::governance::MemoryGovernanceProfile>,
}

impl Default for PromptInputProductionConfig {
    fn default() -> Self {
        Self {
            max_records_checked: 100,
            max_hits_per_record: 5,
            governance_profile: Some(openwand_memory::governance::MemoryGovernanceProfileId::Batch02rDefault.resolve()),
        }
    }
}

/// Result of prompt input production.
#[derive(Debug, Clone)]
pub struct PromptInputResult {
    pub inputs: MemoryPromptAssemblyInputs,
    pub report: RepoConsistencyReport,
    /// Hydrated claims with full provenance from MemoryRecord + RankedMemoryHit.
    /// Panel/audit consumers use this instead of raw findings.
    pub hydrated_claims: Vec<HydratedMemoryClaim>,
    pub claims_checked: usize,
    pub repo_observed: bool,
    pub source_session_id: Option<SessionId>,
    pub source_working_directory: std::path::PathBuf,
    pub errors: Vec<String>,
    /// Which governance profile was used for prompt assembly.
    pub governance_profile_id: Option<openwand_memory::governance::MemoryGovernanceProfileId>,
}

/// Assemble prompt inputs from a governed report.
/// Only includes findings with PromptEligibility::Include.
fn assemble_from_governed(
    governed: &openwand_memory::governance::GovernanceFilteredReport,
) -> MemoryPromptAssemblyInputs {
    use openwand_memory::governance::PromptEligibility;
    use openwand_memory::prompt_assembly::*;
    use openwand_memory::repo_consistency::RepoConsistencyFindingKind;

    let mut supported = Vec::new();
    let mut superseded = Vec::new();
    let mut conflicts = Vec::new();
    let mut missing = Vec::new();
    let mut unverifiable = Vec::new();

    for gf in &governed.governed_findings {
        match gf.prompt_eligibility {
            PromptEligibility::Include => {
                match gf.finding.kind {
                    RepoConsistencyFindingKind::Supported
                    | RepoConsistencyFindingKind::StaleMemory
                    | RepoConsistencyFindingKind::MissingInRepo => {
                        if let Some(ref claim_text) = gf.finding.claim_text {
                            supported.push(SupportedMemoryClaim {
                                claim_text: claim_text.clone(),
                                evidence_kind: gf.finding.evidence_kind
                                    .unwrap_or(openwand_memory::evidence::EvidenceKind::AcceptedClaim),
                                confidence_bps: 0,
                                source_provenance: None,
                                repo_evidence_key: gf.finding.repo_evidence_key.clone(),
                                inclusion_reason: PromptInclusionReason::RepoSupported {
                                    evidence_keys: gf.finding.repo_evidence_key.clone(),
                                },
                            });
                        }
                    }
                    RepoConsistencyFindingKind::SupersededMemoryIgnored => {
                        if let Some(ref claim_text) = gf.finding.claim_text {
                            superseded.push(SupersededMemoryClaim {
                                claim_text: claim_text.clone(),
                                source_provenance: None,
                                inclusion_reason: PromptInclusionReason::SupersededHistory,
                            });
                        }
                    }
                    RepoConsistencyFindingKind::ConflictRequiresReview => {
                        if let Some(ref claim_text) = gf.finding.claim_text {
                            conflicts.push(MemoryConflictGroup {
                                claims: vec![ConflictPromptClaim {
                                    claim_text: claim_text.clone(),
                                    source_provenance: None,
                                }],
                                group_id: String::new(),
                                inclusion_reason: PromptInclusionReason::ConflictReview,
                            });
                        }
                    }
                    RepoConsistencyFindingKind::MissingInMemory => {
                        missing.push(MissingMemoryObservation {
                            repo_evidence_key: gf.finding.repo_evidence_key
                                .first()
                                .cloned()
                                .unwrap_or_default(),
                            detail: gf.finding.detail.clone(),
                            severity: gf.finding.severity,
                            inclusion_reason: PromptInclusionReason::MissingMemoryGap,
                        });
                    }
                    RepoConsistencyFindingKind::Unverifiable => {
                        unverifiable.push(UnverifiableMemoryClaim {
                            claim_text: gf.finding.claim_text.clone().unwrap_or_default(),
                            evidence_kind: gf.finding.evidence_kind,
                        });
                    }
                }
            }
            PromptEligibility::ExcludeAuditOnly { .. } => {
                if matches!(gf.finding.kind, RepoConsistencyFindingKind::Unverifiable) {
                    unverifiable.push(UnverifiableMemoryClaim {
                        claim_text: gf.finding.claim_text.clone().unwrap_or_default(),
                        evidence_kind: gf.finding.evidence_kind,
                    });
                }
            }
        }
    }

    MemoryPromptAssemblyInputs {
        supported_claims: supported,
        relevant_superseded_history: superseded,
        conflicts_for_user_or_model: conflicts,
        missing_memory_gaps: missing,
        unverifiable_claims_excluded: unverifiable,
    }
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
        let empty_report = || RepoConsistencyReport {
            repo_root: working_directory.to_path_buf(),
            checked_at: chrono::Utc::now(),
            summary: openwand_memory::repo_consistency::RepoConsistencySummary::from_findings(&[]),
            findings: vec![],
            memory_inputs: RepoMemoryInputSummary::default(),
            repo_inputs: RepoObservationSummary::default(),
        };
        let governance_profile_id = config.governance_profile.as_ref()
            .map(|_| openwand_memory::governance::MemoryGovernanceProfileId::Batch02rDefault);

        let make_empty = |errors: Vec<String>| PromptInputResult {
            inputs: MemoryPromptAssemblyInputs::empty(),
            report: empty_report(),
            hydrated_claims: vec![],
            claims_checked: 0,
            repo_observed: false,
            source_session_id: session_id.clone(),
            source_working_directory: working_directory.to_path_buf(),
            errors,
            governance_profile_id,
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
                report: empty_report(),
                hydrated_claims: vec![],
                claims_checked: claims_to_check,
                repo_observed: false,
                source_session_id: session_id,
                source_working_directory: working_directory.to_path_buf(),
                errors: search_errors,
                governance_profile_id,
            };
        }

        // Step 5: Observe repo
        let snapshot = match observe_repo(&StdRepoReadFs, working_directory) {
            Ok(s) => s,
            Err(e) => {
                return PromptInputResult {
                    inputs: MemoryPromptAssemblyInputs::empty(),
                    report: empty_report(),
                    hydrated_claims: vec![],
                    claims_checked: claims_to_check,
                    repo_observed: false,
                    source_session_id: session_id,
                    source_working_directory: working_directory.to_path_buf(),
                    errors: vec![format!("observe_repo: {e}")],
                    governance_profile_id,
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

        // Step 9: Assemble (governed if profile provided)
        let inputs = if let Some(ref profile) = config.governance_profile {
            let governed = openwand_memory::governance::GovernanceFilteredReport::from_report(
                &report, profile, &all_hits,
            );
            // Build assembly inputs from governed findings only
            assemble_from_governed(&governed)
        } else {
            RepoConsistencyPromptAssembler::assemble_from_report(&report)
        };

        // Step 10: Hydrate provenance from records + hits
        let hydrated_claims = MemoryProvenanceHydrator::hydrate_findings(
            &report.findings,
            &all_hits,
            &records,
        );

        // Step 11: Hydrate trace relation lineage
        let hydrated_claims = Self::hydrate_trace_lineage(hydrated_claims, &self.trace).await;

        PromptInputResult {
            claims_checked: claims_to_check,
            repo_observed: true,
            inputs,
            report,
            hydrated_claims,
            source_session_id: session_id,
            source_working_directory: working_directory.to_path_buf(),
            errors: search_errors,
            governance_profile_id,
        }
    }

    /// Hydrate trace relation lineage for all claims.
    /// Non-fatal: failures produce Partial status and continue.
    async fn hydrate_trace_lineage(
        claims: Vec<HydratedMemoryClaim>,
        trace: &Arc<dyn TraceStore<StoredEvent>>,
    ) -> Vec<HydratedMemoryClaim> {
        // 1. Collect and deduplicate all source trace IDs
        let mut all_trace_ids = std::collections::HashSet::new();
        for claim in &claims {
            for id in &claim.provenance.source_trace_ids {
                all_trace_ids.insert(id.clone());
            }
        }

        if all_trace_ids.is_empty() {
            // No source traces — nothing to hydrate
            return claims;
        }

        // 2. Query relations for each trace ID (bidirectional)
        let mut relation_rows = Vec::new();
        let mut trace_errors = Vec::new();

        for trace_id in &all_trace_ids {
            let id = openwand_trace::ids::TraceId(trace_id.clone());

            // Forward: trace_id is the source (from)
            match trace.scan_relations(RelationQuery {
                from: Some(id.clone()),
                ..Default::default()
            }).await {
                Ok(rels) => {
                    for r in rels {
                        relation_rows.push(TraceRelationAuditRow {
                            from_trace_id: r.from.0.clone(),
                            to_trace_id: r.to.0.clone(),
                            kind: format!("{:?}", r.kind),
                            created_at: r.created_at,
                        });
                    }
                }
                Err(e) => {
                    trace_errors.push(format!("scan_relations(from {}): {}", trace_id, e));
                }
            }

            // Reverse: trace_id is the target (to)
            match trace.scan_relations(RelationQuery {
                to: Some(id.clone()),
                ..Default::default()
            }).await {
                Ok(rels) => {
                    for r in rels {
                        relation_rows.push(TraceRelationAuditRow {
                            from_trace_id: r.from.0.clone(),
                            to_trace_id: r.to.0.clone(),
                            kind: format!("{:?}", r.kind),
                            created_at: r.created_at,
                        });
                    }
                }
                Err(e) => {
                    trace_errors.push(format!("scan_relations(to {}): {}", trace_id, e));
                }
            }
        }

        // 3. Collect all unique related trace IDs for metadata lookup
        let mut related_ids = std::collections::HashSet::new();
        for row in &relation_rows {
            related_ids.insert(row.from_trace_id.clone());
            related_ids.insert(row.to_trace_id.clone());
        }
        // Also include source IDs
        for id in &all_trace_ids {
            related_ids.insert(id.clone());
        }

        // 4. Query event metadata for all involved trace IDs
        let mut event_metadata = Vec::new();
        for trace_id in &related_ids {
            let id = openwand_trace::ids::TraceId(trace_id.clone());
            match trace.get(id).await {
                Ok(Some(entry)) => {
                    let actor_label = match &entry.actor {
                        openwand_trace::actor::Actor::User => "User".to_string(),
                        openwand_trace::actor::Actor::Llm { model, .. } => format!("LLM ({})", model),
                        openwand_trace::actor::Actor::System { component } => format!("System ({})", component),
                        openwand_trace::actor::Actor::MemoryPipeline => "MemoryPipeline".to_string(),
                        openwand_trace::actor::Actor::WorkflowEngine => "WorkflowEngine".to_string(),
                        openwand_trace::actor::Actor::PolicyEngine => "PolicyEngine".to_string(),
                    };
                    event_metadata.push(TraceEventAuditMetadata {
                        trace_id: entry.id.0.clone(),
                        event_kind: entry.event_kind.clone(),
                        occurred_at: entry.occurred_at,
                        actor_label,
                    });
                }
                Ok(None) => {} // Trace not found — skip
                Err(e) => {
                    trace_errors.push(format!("get({}): {}", trace_id, e));
                }
            }
        }

        // 5. Pure hydration
        let claims_trace_ids: Vec<Vec<String>> = claims
            .iter()
            .map(|c| c.provenance.source_trace_ids.clone())
            .collect();
        let lineages = TraceRelationAuditHydrator::hydrate_claims(
            &claims_trace_ids,
            &relation_rows,
            &event_metadata,
        );

        // 6. Attach lineage to claims
        claims
            .into_iter()
            .zip(lineages)
            .map(|(mut claim, lineage)| {
                claim.trace_lineage = Some(lineage);
                claim
            })
            .collect()
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
