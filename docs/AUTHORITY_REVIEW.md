# Structured Authority Review — Wave 105B

**Review date:** 2026-06-14
**Scope:** OpenWand v0.7.0 (post-105A)
**Reviewer:** Automated structured review of source code, tests, and guards

---

## Purpose

This document enumerates each authority surface in OpenWand, documenting what
each component MAY do, what it MUST NOT do, how enforcement is achieved, what
tests/guards exist, and what residual risks remain.

**This is not a formal security review.** It is a structured authority-mapping
exercise to ensure every write-capable surface is documented and bounded.

---

## Surface Inventory

| # | Surface | Write Authority | Location |
|---|---------|----------------|----------|
| S1 | Desktop UI (render) | None | crates/app/src/ui/*_components.rs |
| S2 | Desktop UI (request DTOs) | None (request only) | crates/app/src/ui/*_request.rs, run_dto.rs |
| S3 | UiSessionService | Delegated (see below) | crates/app/src/ui/service.rs |
| S4 | Policy/Trust Gate | Decision only (no writes) | crates/policy/src/engine.rs |
| S5 | Tool Executor + Sandbox | File writes via sandbox only | crates/tools/src/ |
| S6 | Session Runner | Governed execution | crates/session/src/runner.rs |
| S7 | Trace Store | Append-only | crates/trace/src/store.rs, crates/store/src/ |
| S8 | Trace Verifier | None (read-only) | crates/trace/src/verifier.rs |
| S9 | Operation Replay Verifier | None (read-only) | crates/app/src/operation_replay.rs |
| S10 | Anchor Writer | Anchor files only | crates/trace/src/anchor.rs |
| S11 | Anchor Verifier | None (read-only) | crates/trace/src/anchor.rs |
| S12 | CLI Commands | Delegated (see below) | crates/app/src/main.rs |

---

## S1: Desktop UI Render Components

**Files:** `crates/app/src/ui/*_components.rs`, `crates/app/src/ui/design_tokens.rs`

### May-do authority
- Render read-only displays of workflow state, session data, provider info
- Apply design system tokens (UiTone, UiSize, UiDensity)
- Build style strings for Dioxus rsx! rendering

### Must-not-do boundary
- May NOT import backend crates (openwand-store, openwand-session, openwand-workflow)
- May NOT mutate state
- May NOT execute tools
- May NOT verify or certify
- May NOT write memory or append trace
- May NOT create workflow records

### Enforcement
- Module separation: pure render functions in `*_components.rs` with no `use openwand_store` imports
- Guard tests: 32 source-level checks across test files verify no backend imports in component files

### Tests/Guards
- `crates/app/tests/` — multiple test files contain `no_backend` / `does_not_import` guard assertions
- Design tokens tested separately in `design_tokens.rs`

### Residual risks
- Desktop bootstrap (`desktop_bootstrap.rs`) imports store types — this is the shell initialization layer, not a render component. Acceptable: it constructs services for the shell, render components receive data through service boundary.

---

## S2: Desktop UI Request DTOs

**Files:** `crates/app/src/ui/workflow_run_request.rs`, `evidence_export_request.rs`, `approval_resolution_request.rs`, `run_dto.rs`

### May-do authority
- Carry request parameters from desktop UI to service boundary
- Validate field presence and format (DTO-level validation)
- Convert to service-callable form

### Must-not-do boundary
- May NOT construct RunConfig (must be service-boundary only)
- May NOT execute operations
- May NOT bypass approval gates
- May NOT contain "resolve whatever is pending" implicit logic

### Enforcement
- DTO structs are plain data — no methods that call backend services
- 88A testability refactor: `request_workflow_run()` (instance) separated from `evaluate_workflow_run_request()` (static)
- 88B explicit ARID rule: `approval_request_id` must be mandatory non-empty
- 88C separate roots rule: `store_root` must differ from `export_root`

### Tests/Guards
- `crates/app/tests/workflow_run_initiation.rs`
- `crates/app/tests/approval_resolution_desktop.rs`
- `crates/app/tests/evidence_export_desktop.rs`
- Source-level guards for mandatory fields

### Residual risks
- None known. DTO layer is pure data with validation.

---

## S3: UiSessionService

**File:** `crates/app/src/ui/service.rs`

### May-do authority
- Delegate workflow run creation through governed execution gate
- Delegate approval resolution through SessionRunner
- Delegate evidence export through `export_audit_packet()`
- Emit trace events AFTER successful governed operations (workflow initiation, evidence export)
- Compute SHA-256 checksum on artifact path returned by export
- Parse exported JSON for record count

### Must-not-do boundary
- May NOT bypass policy gate for workflow runs
- May NOT resolve approvals without explicit ARID
- May NOT read arbitrary files (only the artifact path returned by export)
- May NOT construct RunConfig in UI or DTO layer
- May NOT emit trace events before operation succeeds
- May NOT append trace for evidence export checksum (only artifact.generated)

### Enforcement
- Service methods delegate to `SessionRunner`, `WorkflowExecutionGate`, `export_audit_packet()`
- 88A: Static `evaluate_workflow_run_request()` is testable without a service instance
- 88C: Checksum computed ONLY on artifact path returned by export operation
- 99A/99B: Trace events emitted only in instance methods after success, not in static methods

### Tests/Guards
- `crates/app/tests/workflow_run_initiation.rs` — verifies governed path
- `crates/app/tests/workflow_live_wiring.rs` — end-to-end wiring
- `crates/app/tests/evidence_export_desktop.rs` — export path validation
- `crates/app/src/operation_replay.rs` guard: no `export_audit_packet`, `request_workflow_run`, or `submit_approval_resolution` calls in verifier

### Residual risks
- Service is the primary write-delegation surface. All writes are governed through existing gates. No known bypass.

---

## S4: Policy/Trust Gate

**Files:** `crates/policy/src/engine.rs`, `decision.rs`, `builtin.rs`

### May-do authority
- Evaluate tool requests against deterministic rules
- Produce GateDecision (Allow, Deny, Confirm)
- Produce PolicyEvaluation with findings

### Must-not-do boundary
- May NOT execute tools
- May NOT write files
- May NOT mutate session state
- May NOT modify rules at runtime (deterministic evaluation only)

### Enforcement
- Policy engine is pure evaluation: input → decision
- `fail_closed()` method ensures errors produce Deny, not Allow
- ToolMatcher patterns are static

### Tests/Guards
- `crates/policy/tests/write_gate.rs` — 8+ tests for fail-closed behavior
- Guard: `allows_execution()` only returns true for explicit Allow decision
- Guard: `fail_closed()` produces Deny on any error

### Residual risks
- Policy is only as strong as its rule set. Misconfigured rules could allow unintended operations. This is an operational risk, not a code-level authority violation.

---

## S5: Tool Executor + Sandbox

**Files:** `crates/tools/src/lib.rs`, `local.rs`, `sandbox.rs`, `sandbox_ntapi.rs`, `file_patch.rs`

### May-do authority
- Execute tools ONLY after policy gate approval
- Write files within workspace sandbox (via `resolve_workspace_path()`)
- Read files within workspace sandbox
- Apply file patches within sandbox
- Use composite tools (sequenced tool execution)

### Must-not-do boundary
- May NOT write outside workspace sandbox
- May NOT bypass `resolve_workspace_path()` for any local file operation
- May NOT execute without policy gate approval
- May NOT access `~/.openwand/` data directory directly (sandbox resolves workspace, not data dir)
- May NOT follow symlinks that escape the workspace (TOCTOU hardening)

### Enforcement
- 69A: ALL local tools resolve paths through centralized `resolve_workspace_path()`
- 69A Patch 7: Path containment errors do NOT leak external canonical paths
- 72B-73C: Unix fully closed via openat; Windows NtCreateFile dirs + write_file_no_follow
- 69B: Approval resumption bound to original canonical workspace; fail-closed on mismatch
- `unsafe` usage: `libc::dup(fd)` in Unix openat wrapper — intentional, reviewed syscall

### Tests/Guards
- 1,314+ sandbox containment tests across test files
- `crates/session/tests/governed_shell_exec.rs`
- `crates/session/tests/approval_production_path.rs`
- `crates/session/tests/approval_real_file_effect.rs`
- `crates/policy/tests/write_gate.rs`
- Source guard: `resolve_workspace_path` called before every file write

### Residual risks
- Windows final-component TOCTOU: residual is safe-failure/limited final-component race posture, not a known arbitrary write vulnerability. Documented in SECURITY_REVIEW_PREP.md (CV-8).

---

## S6: Session Runner

**File:** `crates/session/src/runner.rs`

### May-do authority
- Run agent turns (inference + tool calls)
- Resolve approvals with explicit ARID
- Maintain Loro CRDT session state
- Emit trace events for inference, tool calls, approvals
- Recover from interrupted approval flows

### Must-not-do boundary
- May NOT bypass policy gate for tool execution
- May NOT resolve approvals without explicit ARID and decision
- May NOT append trace events for operations that failed
- May NOT approve its own actions

### Enforcement
- Approval resolution requires `ApprovalResolution::for_approval(arid, resolution)` with explicit ARID
- Tool calls go through policy gate before execution
- 69D: Commands that don't perform real work must exit non-zero
- 69E: Never "mock" or "unknown" actor in production trace path

### Tests/Guards
- `crates/session/tests/approval_post_effect.rs`
- `crates/session/tests/approval_production_path.rs`
- `crates/session/tests/approval_recovery_scanner.rs`
- `crates/session/tests/select_approval_target.rs`
- `crates/session/tests/governed_shell_exec.rs`

### Residual risks
- Approval recovery scanner handles interrupted sessions. If session state is corrupted, recovery may miss pending approvals. This is an operational resilience risk, not an authority violation.

---

## S7: Trace Store

**Files:** `crates/trace/src/store.rs`, `crates/store/src/backends/sqlite/store.rs`, `writer.rs`

### May-do authority
- Append trace entries (assign IDs, sequences, hashes)
- Scan/query entries (read-only)
- Initialize storage (create tables, indexes)
- Rebuild projections from checkpoint

### Must-not-do boundary
- Trace store API must NOT mutate or delete committed trace entries
- May NOT modify existing entry_hash or prev_hash values
- May NOT reorder entries
- (Physical SQLite file is technically mutable — hence verifier exists)

### Enforcement
- 92A: Hash chain metadata exists (`prev_hash` + `entry_hash`)
- 98A: Canonical BLAKE3 hash in `Blake3HashPolicy::compute_hash()` in trace crate
- Store's `hash.rs` delegates to canonical function
- Physical immutability NOT enforced at storage layer — verifier + anchor detect after the fact

### Tests/Guards
- `crates/trace/src/verifier.rs` — 8+ tests for chain validation
- `crates/trace/src/anchor.rs` — 39 tests for anchor verification
- `crates/store/src/backends/sqlite/hash.rs` — hash computation delegation

### Residual risks
- Physical SQLite file is technically mutable (documented caveat). An attacker with filesystem access can modify the database. Verifier + external anchor detect this, but cannot prevent it. Full prevention requires append-only storage or external trust anchor.
- External anchor can also be rewritten by an attacker who controls both locations. Documented limitation.

---

## S8: Trace Verifier

**File:** `crates/trace/src/verifier.rs`

### May-do authority
- READ trace entries
- VALIDATE chain continuity (prev_hash → entry_hash links)
- VALIDATE ordering (global + per-stream monotonicity)
- VALIDATE hash correctness (BLAKE3 recomputation under HashVerificationPolicy)
- Report findings (Pass/Fail/Inconclusive/Unsupported)

### Must-not-do boundary
- May NOT mutate, repair, rewrite, migrate trace entries
- May NOT append new entries
- May NOT delete entries
- May NOT modify hash values
- May NOT execute tools or workflows
- May NOT approve actions
- May NOT write files

### Enforcement
- Source-level guards: verifier.rs checked for absence of `.append(`, `fn repair`, `.write(`, `std::fs::write`, `fn migrate`, `fn rewrite`
- Trait design: `HashVerificationPolicy<E>` is read-only (compute_hash, serialize_event)
- Function signature: `verify()` takes `&[TraceEntry<E>]` (immutable reference)

### Tests/Guards
- `crates/app/tests/trace_verify_cli.rs::authority_guards::verifier_is_read_only`
- 8+ verification tests covering pass/fail/tamper cases
- Hash policy tests for correct/tampered/consistent-tamper behavior

### Residual risks
- None known. Verifier is pure read-only computation.

---

## S9: Operation Replay Verifier

**File:** `crates/app/src/operation_replay.rs`

### May-do authority
- READ trace entries
- READ desktop operation descriptors
- VALIDATE correspondence between operations and trace evidence
- Report findings (Pass/Fail/Inconclusive/Unsupported)

### Must-not-do boundary
- May NOT execute, append, mutate, or repair anything
- May NOT call `export_audit_packet`, `request_workflow_run`, `submit_approval_resolution`
- May NOT instantiate runners, tools, exporters, gates, or policies
- May NOT write files
- "Replay" = correspondence checking, NOT execution replay

### Enforcement
- Source-level guard: implementation-only code checked for absence of execution keywords
- Guard test: `no_exec_calls()`, `is_read_only()` split on `#[cfg(test)]` to exclude test assertions
- Function signature: `verify()` takes immutable slices

### Tests/Guards
- `crates/app/src/operation_replay.rs::tests::no_exec_calls`
- `crates/app/src/operation_replay.rs::tests::is_read_only`
- `crates/app/tests/operation_replay_cli.rs` — CLI surface tests

### Residual risks
- None known. Verifier is pure read-only correspondence checking.

---

## S10: Anchor Writer (CheckpointWriter)

**File:** `crates/trace/src/anchor.rs`

### May-do authority
- CREATE anchor files at `{anchor_root}/openwand-checkpoint-{seq}.json`
- Compute root hash over trace entry_hash values
- Serialize and write anchor JSON
- Compute next sequence from existing anchor files

### Must-not-do boundary
- May NOT write to the trace store
- May NOT write inside the store_root (path containment enforced)
- May NOT mutate trace entries
- May NOT overwrite existing checkpoints (collision rejection)
- May NOT append to trace, execute tools, approve actions, or modify policy
- May NOT write anchor files for nonexistent trace entries
- anchor_root must be canonicalized and separate from store_root in both directions

### Enforcement
- `validate_anchor_root()`: canonical path containment checks (equality, inside-store, store-inside)
- `CheckpointWriter::write_checkpoint()`: collision check before write
- Source-level guard: no `append_trace`, `delete_entry`, `remove_entry` in implementation
- Source-level guard: no `openwand_store`, `openwand_core`, `openwand_session` imports
- File naming convention: only writes `openwand-checkpoint-*.json` files

### Tests/Guards
- 5 path containment tests (equality, inside, store-inside, separate, nonexistent)
- 5 writer tests (create, valid JSON, roundtrip, collision, anchor-inside-store rejection)
- 3 sequence tests (empty, after-existing, ignores-non-anchor)
- 2 full-workflow tests (write-then-verify-growth, detect-tamper)
- Source-level guards: no trace mutation, no backend imports, writer-only-writes-anchor-files

### Residual risks
- Writer creates files on the local filesystem. An attacker with filesystem access can delete or modify anchor files. The verifier detects mismatch between trace and anchor, but cannot prevent anchor deletion. Mitigated by storing anchors in a location the attacker does not control (out of OpenWand's scope to enforce).

---

## S11: Anchor Verifier (verify_anchor)

**File:** `crates/trace/src/anchor.rs`

### May-do authority
- READ trace entries (immutable)
- READ anchor objects (provided by caller)
- COMPARE root hashes over checkpointed prefix
- REPORT result (Pass/Fail/MissingAnchor/Unsupported) and freshness (Current/Stale)

### Must-not-do boundary
- May NOT create, repair, delete, or modify anchor files
- May NOT mutate trace entries
- May NOT write files of any kind
- May NOT execute, approve, or delegate

### Enforcement
- `verify_anchor()` takes `&[TraceEntry<E>]` and `Option<&CheckpointAnchor>` (both immutable)
- `read_anchor_file()` is a separate function used only by CLI, not by verify_anchor
- Source-level guard: verify_anchor is read-only (entries not mutated after call)

### Tests/Guards
- Read-only guard test: entries compared before/after verify_anchor call
- MissingAnchor is Inconclusive, not Fail (legacy compatible)
- Pass/Stale for append growth (anchor remains valid)
- Fail on modification within checkpoint prefix

### Residual risks
- None known. Anchor verifier is pure read-only computation.

---

## S12: CLI Commands

**File:** `crates/app/src/main.rs`

### May-do authority by command

| Command | Authority |
|---------|-----------|
| `run` | Full agent execution (governed) |
| `explain` | Stub (exits non-zero — 69D) |
| `trace-verify` | Read trace + verify (read-only) |
| `operation-replay` | Read trace + verify (read-only) |
| `anchor-write` | Create anchor file outside store root |
| `anchor-verify` | Read trace + read anchor + verify (read-only) |
| `audit-check` | Stub (exits non-zero — 69D) |
| `session-rebuild` | Stub (exits non-zero — 69D) |
| `task-plan` | Static evaluation (no execution) |
| `workflow-*` | Static evaluation (no execution) |

### Must-not-do boundary
- Stubs must exit non-zero (69D: commands that don't perform real work)
- Read-only commands must not write trace or execute tools
- anchor-write must not write inside store_root
- No command may bypass policy gate

### Enforcement
- 69D: Stubs exit with code 1
- 71A: Capabilities claimed as CLI commands must be reachable through binary
- anchor-write and anchor-verify have separate authority: write vs read

### Tests/Guards
- `crates/app/tests/truthful_commands.rs` — stubs exit non-zero
- `crates/app/tests/cli_command_surface.rs` — CLI reachability
- `crates/app/tests/trace_verify_cli.rs` — verifier CLI
- `crates/app/tests/operation_replay_cli.rs` — replay CLI
- `crates/app/tests/anchor_cli.rs` — anchor CLI

### Residual risks
- `run` command has full agent execution authority. This is by design — it runs the governed agent loop. All tool calls go through policy gate.

---

## Summary: Write-Authority Surfaces

Only 4 of 12 surfaces have any write authority:

| Surface | What it writes | Gated by |
|---------|---------------|----------|
| UiSessionService (S3) | Trace events (after success) | Governed execution gate |
| Tool Executor (S5) | Files in workspace sandbox | Policy gate + sandbox containment |
| Session Runner (S6) | Trace events, session state | Policy gate + approval gates |
| Anchor Writer (S10) | Anchor files outside store | Path containment + collision check |

All other surfaces (S1, S2, S4, S7-read, S8, S9, S11, S12-readonly) are read-only or decision-only.

---

## Summary: Read-Only Verifiers

| Verifier | Input | Output | Writes |
|----------|-------|--------|--------|
| TraceVerifier (S8) | `&[TraceEntry<E>]` | VerificationReport | None |
| OperationReplayVerifier (S9) | `&[DesktopOperation]`, `&[TraceEntry]` | OperationReplayReport | None |
| verify_anchor (S11) | `&[TraceEntry<E>]`, `Option<&CheckpointAnchor>` | AnchorVerificationReport | None |

All verifiers take immutable references and return reports. None have side effects.

---

## Cross-Cutting Enforcement

| Mechanism | Scope | Since |
|-----------|-------|-------|
| `resolve_workspace_path()` | All local file operations | 69A |
| Canonical path containment | Anchor roots | 104B |
| Policy gate evaluation | All tool execution | 00-22 |
| Approval gates with explicit ARID | All approval resolution | 88B |
| Trace hash chain + hash policy | All trace entries | 92A/98A |
| Source-level guard tests | All verifier modules | 93A+ |
| Fail-closed policy | All errors produce Deny | 00-22 |

---

## Residual Risk Summary

| Risk | Severity | Mitigation | Status |
|------|----------|------------|--------|
| Physical trace store mutable | Medium | Verifier + anchor detect after fact | Documented |
| External anchor also mutable | Medium | Store anchor in attacker-inaccessible location | Out of scope |
| Windows final-component TOCTOU | Low | Safe-failure mode; not arbitrary write | Documented (CV-8) |
| Upstream dependency warnings | Low | Monitor upstream; 0 CVEs | Accepted |
| GTK3 unmaintained warnings | Low | Desktop-only; resolves with Dioxus GTK4 migration | Upstream-blocked |
| Policy misconfiguration | Medium | Operational responsibility | Out of code scope |
| Approval recovery under corruption | Low | Recovery scanner is best-effort | Operational |

---

## What This Review Is NOT

- Not a formal security review by a qualified auditor
- Not penetration testing
- Not cryptographic review of BLAKE3 or SQLite implementations
- Not supply chain integrity verification
- Not production-readiness certification

This is a structured authority mapping to ensure every write-capable surface in
OpenWand is documented, bounded, tested, and honest about its residual risks.
