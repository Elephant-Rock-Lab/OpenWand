# PRE-BLUEPRINT ENGINEERING HEALTH CHECKLIST

**File:** `/docs/aiv/PRECHECK.md`  
**Version:** 1.0  
**Audience:** Lead Programmer  
**Authority:** Advisory only. This is a self-assessment tool — it does not create new phases or gating steps in the AIV Framework. The Lead decides whether to use it, and what actions to take based on the findings.  
**When to use:** Before drafting a new Batch Blueprint, especially if the Batch touches existing source files, crosses architectural boundaries, or follows a period of rapid development.  
**Output:** Completed checklist may directly inform Hard Boundaries, Task scoping, dependency declarations, test plans, and entries in `STATE.md`. It is not submitted to the Reviewer.

───────────────────────────────────────────────────────────
HOW TO USE
───────────────────────────────────────────────────────────
1. Read `PROJECT.md` to ground yourself in the project’s intent, quality attributes, and known friction points.
2. Read `STATE.md` for the current module map, architectural decisions, gotchas, and carry-forward obligations.
3. Work through each section below. For every item marked NO, record a note in the Notes & Decisions section at the end.
4. Translate significant findings into concrete Blueprint elements:
   - New or tightened Hard Boundaries
   - Additional Task-level acceptance criteria
   - Explicit refactoring Tasks
   - New entries in `STATE.md` (Gotchas, Architectural Decisions)
5. If the checklist reveals risks that cannot be resolved within a single Batch, consider opening a dedicated “Engineering Health” Batch to address them before feature work continues.

───────────────────────────────────────────────────────────
CHECKLIST
───────────────────────────────────────────────────────────

### 1. ARCHITECTURAL ALIGNMENT
Does this Batch respect and reinforce the existing architecture, rather than weakening it?

  [ ] The Batch’s scope remains within the established crate/module boundaries.
  [ ] The Batch does not introduce a new pattern or convention that conflicts with the current codebase style (e.g., a different error-handling strategy, a new dependency injection method without migration).
  [ ] Any new module or crate has a clearly defined responsibility that doesn’t overlap with an existing one.
  [ ] Cross-cutting concerns (logging, authentication, error reporting, configuration) are handled consistently with the rest of the system.
  [ ] If the Batch changes a public API, the change is backward-compatible or the migration path is documented.

### 2. COUPLING & COHESION
Does this Batch create implicit coupling, or blur the boundaries between distinct concerns?

  [ ] No two existing modules that are currently independent will become coupled through this Batch’s changes (e.g., shared mutable state, new import cycles).
  [ ] New modules are cohesive: they have one reason to change and a single, clear responsibility.
  [ ] The Batch does not introduce “shotgun surgery” — where a single logical change would require touching files across many unrelated modules.
  [ ] All new dependencies are justified. If a new library or utility is introduced, its total cost (compile time, audit surface, learning curve) is accounted for.

### 3. PERFORMANCE & SCALE
Does this Batch introduce operations that could degrade under realistic workloads?

  [ ] No new unbounded loops, recursive calls, or large in-memory collections without explicit bounds.
  [ ] No N+1 query patterns are introduced (where fetching a list triggers a separate data fetch per item).
  [ ] Blocking I/O is not added to async contexts or hot synchronous paths without explicit documentation.
  [ ] If the Batch touches existing hot paths (identified by profiling or prior analysis), the performance impact has been estimated.
  [ ] Batch, bulk, or background operations have reasonable timeouts and resource limits.

### 4. SECURITY & AUTHORITY
Does this Batch alter trust boundaries or introduce new attack surfaces?

  [ ] No new privilege escalation path is created (e.g., a user-controlled input enabling admin actions).
  [ ] New inputs are validated before use, and the validation rules are explicit.
  [ ] Sensitive data (PII, credentials, tokens) is not logged, stored in plain text, or exposed in error messages.
  [ ] Any change to authentication or authorization logic has a corresponding Hard Boundary in the Blueprint.
  [ ] External integrations (APIs, file formats, user-provided scripts) are treated as untrusted input.

### 5. TECHNICAL DEBT & KNOWN GOTCHAS
Does this Batch compound existing problems, or actively reduce them?

  [ ] The Batch does not rely on code already marked as brittle, stale, or scheduled for replacement in `STATE.md`.
  [ ] The Batch does not extend or duplicate a workaround that was intended to be temporary.
  [ ] If the Batch touches a Known Friction Point from `PROJECT.md`, the risk is explicitly acknowledged and mitigated.
  [ ] The Batch leaves the codebase strictly better than it found it — at minimum, it does not add new suppressions, `TODO` markers without tracking IDs, or copy-pasted code blocks.

### 6. MISSING ABSTRACTIONS
Does this Batch repeat a pattern that should be unified?

  [ ] The Batch does not duplicate logic that already exists in two or more places without first extracting a shared abstraction.
  [ ] Any new data transformation or conversion could be reused by future Batches and is placed appropriately.
  [ ] If the Batch introduces a new concept (e.g., a new event type, a new configuration section), it is modelled in a way that won’t require breaking changes when extended.
  [ ] Common boilerplate (error wrapping, response formatting, retry logic) is encapsulated, not scattered across Tasks.

### 7. TESTABILITY
Will the code produced by this Batch be easy to test meaningfully?

  [ ] Core business logic can be tested without requiring a running database, network, or external service.
  [ ] Side-effectful code (I/O, system calls, randomness) is separated from pure logic so that it can be mocked or stubbed.
  [ ] Error paths are triggerable in tests — you can simulate a network failure, invalid input, or timeout.
  [ ] Acceptance Criteria for every Task include at least one failure-mode testable condition.

### 8. CARRY-FORWARD OBLIGATIONS FROM STATE.md
Does this Batch address or acknowledge existing promises made by previous Batches?

  [ ] Open deferred tests from `STATE.md` that fall within this Batch’s scope are scheduled for execution or explicitly deferred again with a reason.
  [ ] Known Gaps that this Batch could resolve are addressed, or a note explains why they remain open.
  [ ] Architectural Decisions marked Active are respected; if the Batch intentionally overrides one, the Lead documents it as a new decision.
  [ ] The Test Baseline from `STATE.md` is consistent with the scope — if this Batch will change test count, the expected delta is accurate.

### 9. USER IMPACT & VALUE ALIGNMENT
Does the work align with the project’s declared priorities and user needs?

  [ ] The Batch Goal directly supports at least one Core Value Proposition (VP) from `PROJECT.md`.
  [ ] The changes do not degrade a Critical Workflow from `PROJECT.md` — if they temporarily do, the rollout plan is documented.
  [ ] The quality attributes prioritised in `PROJECT.md` are reflected in the acceptance criteria (e.g., if reliability is priority 5, there are tests covering recovery and data integrity).
  [ ] The user-visible error messages, if any, are helpful and actionable, not developer jargon.

### 10. OPERATIONAL READINESS
Will this Batch be safe to deploy and operate?

  [ ] New configuration options have sensible defaults and are documented.
  [ ] Any new persistent state (database schema changes, file formats) is backward-compatible, or a migration plan exists.
  [ ] New logs and metrics follow existing conventions and are useful for debugging.
  [ ] If this Batch introduces a feature flag or toggled behaviour, the default-off state is safe.
  [ ] The CHANGELOG and any user-facing documentation will be updated in the Batch.

───────────────────────────────────────────────────────────
NOTES & DECISIONS
───────────────────────────────────────────────────────────
Record every item that raised a concern, and the decision made.
These notes feed into the Blueprint’s Hard Boundaries, Task definitions,
and entries in `STATE.md`.

  Item Ref:   [Section].[Item] — e.g., 2.3 (new dependency)
  Concern:    [Brief description of the risk]
  Decision:   [How it will be handled in the Blueprint or deferred to a future Batch]
  Action:     [New Hard Boundary / New Task / New STATE.md entry / Deferred with tracking ID]

  Item Ref:
  Concern:
  Decision:
  Action:

  [Repeat for each concern found]

───────────────────────────────────────────────────────────
INTEGRATION WITH THE AIV FRAMEWORK
───────────────────────────────────────────────────────────
- The Lead may use this checklist without altering the formal Batch cycle.
- Significant architectural risks discovered here SHOULD become Hard Boundaries in the Blueprint to make them visible to the Reviewer and Assistant.
- New gotchas, architectural decisions, and verified module paths discovered during this assessment SHOULD be recorded in `STATE.md` at the next Batch Close (or immediately, if they affect the current Blueprint).
- If the checklist reveals pervasive foundation problems, consider a dedicated “Foundation Audit” Batch before proceeding with feature work.

───────────────────────────────────────────────────────────
SIGN-OFF (OPTIONAL)
───────────────────────────────────────────────────────────
The Lead may record that the precheck was performed for their own audit trail.

  Precheck performed by: [Lead Name]
  Date:                  [YYYY-MM-DD]
  Batch ID (if known):   [BATCH-NN or TBD]
  Notes:

═══════════════════════════════════════════════════════════
