# Deferred Audit Risks

Wave 69F closes or explicitly downgrades remaining audit findings.

## Resolved in this wave

### Clippy —all-features — -D warnings posture (non-app crates)

All 11 non-app crates pass `cargo clippy --all-features -- -D warnings` cleanly:
- openwand-core, openwand-session, openwand-tools, openwand-trace, openwand-store
- openwand-memory, openwand-llm, openwand-policy, openwand-skills, openwand-goals, openwand-workflow

Fixes: unused imports, unused variables, complex types (#[allow]), large enum variants,
missing Default impls, format-in-format, collapsible-if, manual-strip, sort_by_key.

### Import hygiene

Fixed latent import gaps exposed by clippy --fix:
- eval_capability_context.rs: added InferenceEvent, CapabilityPreviewMode, CapabilityPromptOrderPosition, etc.
- memory_evaluation.rs: added ScenarioExecutionMode
- task_plan_state.rs: added TaskPlanReviewDecision
- workflow_audit_packet_review/distribution/external_attestation/verification_readiness: added WorkflowExecutionId, AuditPacketReviewId
- workflow_reconciliation_state: added WorkflowStageRunStatus
- skills context: added SkillContextKind

## Deferred risks

### DEFERRED-001: openwand-app clippy -D warnings (57 style warnings)

- **Owner:** Release hardening
- **Scope:** `cargo clippy -p openwand-app --all-features --all-targets -- -D warnings` produces 57 warnings
- **Category:** All style/complexity lints in test modules (too_many_arguments, sort_by_key, dead_code, ptr_arg)
- **Rationale:** `-D warnings` turns warnings into hard errors that cannot be suppressed with `#[allow]`. All 57 are in `#[cfg(test)]` test helper functions and test-only structs. Zero affect production code quality.
- **Impact:** None on release correctness. Cosmetic only.
- **Resolution path:** Either (a) add `#![allow(...)]` at crate level for test-specific lints, or (b) refactor test helpers into a separate test-support crate with its own allow posture.

### DEFERRED-002: cargo audit dependency warnings

- **Owner:** Release hardening
- **Scope:** Dependency vulnerability/status audit
- **Category:** Not yet run
- **Rationale:** `cargo audit` is not part of the current toolchain. Needs a separate pass.
- **Impact:** Unknown until run.
- **Resolution path:** Run `cargo audit`, categorize findings, update dependencies or document accepted risks.

### DEFERRED-003: unsafe-env-test claim correction

- **Owner:** Verification hardening
- **Scope:** Test fixtures or docs that claim unsafe environment handling
- **Category:** Documentation/claim accuracy
- **Rationale:** Not yet assessed in this wave. Low priority — does not affect runtime behavior.
- **Impact:** None on release correctness.

### DEFERRED-004: trace immutability claim correction

- **Owner:** Verification hardening
- **Scope:** Any doc/comment claiming trace immutability before verifier exists
- **Category:** Documentation/claim accuracy
- **Rationale:** Trace entries are append-only in the current implementation but no verifier enforces this. Claims of immutability should be softened to "append-only" until a verifier exists.
- **Impact:** None on release correctness. Claim accuracy only.

### DEFERRED-005: MutationHelper live-event correctness

- **Owner:** Runtime hardening
- **Scope:** Verify MutationHelper produces correct live events during concurrent mutations
- **Category:** Runtime correctness
- **Rationale:** Not yet assessed. Would need concurrent mutation tests.
- **Impact:** Low — MutationHelper is used in single-writer mode guarded by run_lock.

### DEFERRED-006: STATE.md and documentation update

- **Owner:** Release documentation
- **Scope:** STATE.md, KNOWN_GAPS.md, UI_DESIGN_SYSTEM.md out of date
- **Category:** Documentation freshness
- **Rationale:** Multiple waves (52A–69F) have modified the codebase. Documentation reflects pre-wave state.
- **Impact:** Documentation accuracy only.

### DEFERRED-007: Local branch publication

- **Owner:** Release process
- **Scope:** Local master branch ahead of origin
- **Category:** Publication state
- **Rationale:** 18+ waves committed locally, not pushed.
- **Impact:** None until release publication is initiated.
