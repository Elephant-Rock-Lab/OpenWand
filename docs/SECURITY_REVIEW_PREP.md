# OpenWand Security Review Preparation

**Date:** 2026-06-14 (Wave 94A)
**Status:** Preparation document - NOT a formal security review
**Scope:** Threat model refresh, authority-boundary checklist, caveat ledger, review-ready evidence after v0.4 operation and v0.5 verification work

---

## Important Scope Distinction

This document is **security review preparation**. It organizes evidence and boundaries for a future reviewer. It is not:

- **Security review execution** - no penetration testing, fuzzing, or adversarial probing was performed
- **Security assurance claim** - no production-readiness, full-immutability, or formal-certification claim is made

Only the first of these three activities is happening here.

---

## Section 1: Threat Model (refreshed for v0.5)

### Adversary Model

OpenWand assumes the following adversary classes:

| Adversary | Capability | Motivation | In scope? |
|-----------|-----------|------------|-----------|
| Local concurrent process | Runs on the same machine; can create/replace files, symlinks, and directories at high speed | Cause OpenWand to write outside its workspace via TOCTOU race | Yes - sandbox hardening (69A, 72B-73C, 78C) |
| Prompt injection | Malicious content in files, tool output, or retrieved memory manipulates the LLM | Cause unauthorized tool execution or memory mutation | Yes - deterministic gates + approval (trust architecture) |
| Malicious provider | Hosted LLM endpoint returns crafted responses designed to trigger harmful actions | Same as prompt injection but from the model itself | Yes - approval gate does not trust model output |
| Physical database access | Direct read/write to SQLite trace/memory DB files | Tamper with evidence chain or memory store | Yes - trace verifier (92A-92B) validates chain continuity |

### Attack Surfaces and Mitigation Status

| Surface | Threat | Mitigation | Status |
|---------|-------|------------|--------|
| Filesystem sandbox | Path traversal, symlink escape, TOCTOU race | `resolve_workspace_path()` centralization (69A); `write_file_no_follow()` with `O_NOFOLLOW`/`FILE_FLAG_NO_REPARSE_POINT` (72B); `openat`-based handle-relative traversal on Unix (73B); `NtCreateFile`-based handle-relative traversal on Windows (78C) | **Closed on Unix.** Windows: directory traversal closed; final-component relies on 72B no-follow write path (safe-failure mode). |
| Approval gate | Unauthorized tool execution | Policy engine evaluates every tool call; approval required for non-auto-allowed tools; workspace binding prevents cross-workspace approval reuse (69B) | **Closed.** |
| Trace store | Evidence tampering, chain breakage | Append-only API; hash chain (BLAKE3 on SQLite, deterministic placeholder on in-memory); trace verifier validates ordering, chain continuity, duplicates, well-formedness (92A) | **Chain continuity validated by verifier.** No full hash recomputation. Physical DB file is not impossible to tamper with - that is why the verifier exists. |
| Desktop DTO path | UI bypassing authority gates | Desktop UI -> DTO -> UiSessionService -> existing governance gate -> result. UI may not import backend crates, construct RunConfig, or call gates directly (88A-88C). | **Closed by architecture.** Guard tests verify no backend imports. |
| CLI commands | False claims of verification | `trace-verify` and `operation-replay` exit with distinct codes, print honest notes about what Pass means, and do not claim full verification (92B, 93B) | **Closed.** |
| Operation correspondence | Desktop operations not backed by trace evidence | `OperationReplayVerifier` matches desktop operations (workflow initiation, approval resolution, evidence export) against trace events by explicit IDs (93A-93B) | **Partial.** Workflow initiation is Inconclusive (workflow modules do not emit trace). Evidence export is Unsupported (export does not emit trace). |

### What v0.4 Added

v0.4.0 transitioned the desktop from observation to operation. Three new authority paths:

1. **Workflow run initiation** (88A): Desktop UI -> DTO -> UiSessionService::request_workflow_run -> existing workflow execution gate
2. **Approval resolution** (88B): Desktop UI -> DTO -> UiSessionService::submit_approval_resolution -> existing policy approval path
3. **Evidence export** (88C): Desktop UI -> DTO -> UiSessionService::request_evidence_export -> existing export path with separate store_root (evidence source) and export_root (allowed destination)

All three delegate to existing authority gates. The desktop does not gain direct execution, approval, or filesystem authority.

### What v0.5 Added

v0.5.0 introduced a new **READ authority** - the runtime verifier:

1. **Trace verifier** (92A-92B): Reads trace entries from store, validates global ordering, per-stream ordering, cross-ordering consistency, hash chain continuity (prev_hash to entry_hash linkage per stream), entry well-formedness, and duplicate detection. Exposed via `openwand trace-verify`.

2. **Operation replay verifier** (93A-93B): Reads desktop operation descriptors and trace entries, validates correspondence by explicit IDs (workflow_execution_id, approval_request_id, tool_call_id). Exposed via `openwand operation-replay`.

Neither verifier mutates, repairs, appends, executes, approves, or exports. They are READ-ONLY.

---

## Section 2: Authority-Boundary Checklist

Each surface has explicit MAY and MAY NOT constraints, enforced by architecture and guard tests.

### Desktop UI

```
MAY:     Request operations through DTOs (88A-88C)
         Display governance state, trace entries, memory entries
         Trigger refresh of inspector state (89A)
MAY NOT: Import backend crates (session, memory, tools, policy, store, trace)
         Execute tools directly
         Approve without policy evaluation
         Append trace entries
         Write memory entries
         Bypass sandbox resolution
         Create workflow records without delegation
         Construct RunConfig
```

### DTO Layer (UiRunRequest, UiApprovalResolution, UiEvidenceExportRequest)

```
MAY:     Carry request parameters from UI to service boundary
         Carry operation identifiers (workflow_execution_id, approval_request_id, tool_call_id)
MAY NOT: Construct RunConfig (must be constructed only in service boundary)
         Bypass validation (DTOs are validated before service processing)
         Call governance gates directly
```

### UiSessionService

```
MAY:     Delegate to existing governance gates (workflow execution, approval, export)
         Construct RunConfig from validated DTO fields
         Return results to UI
MAY NOT: Bypass policy engine for any tool execution
         Skip sandbox resolution for file operations
         Approve without policy evaluation
```

### Policy Engine (BuiltinPolicyEngine)

```
MAY:     Evaluate tool calls against policy rules
         Allow auto-allowed tools
         Require approval for non-auto-allowed tools
         Deny policy violations
MAY NOT: Execute side effects (evaluation is pure)
         Self-approve (approval comes from user or delegate)
```

### Tool Executor

```
MAY:     Execute approved tools within workspace sandbox
         Write files through resolve_workspace_path()
         Read files within sandbox
MAY NOT: Self-approve (must receive approval from policy engine)
         Write outside workspace sandbox
         Follow symlinks at final path component (O_NOFOLLOW / FILE_FLAG_NO_REPARSE_POINT)
```

### Trace Store

```
MAY:     Append trace entries with hash chain linkage
         Provide paginated scan for verification
MAY NOT: Supported OpenWand trace APIs must not mutate or delete committed trace entries.
         (The physical SQLite file is technically mutable by direct DB access - this is
          why the verifier exists. The API authority boundary is append-only.)
```

### Trace Verifier (TraceVerifier)

```
MAY:     Read trace entries from store
         Validate global ordering, per-stream ordering, cross-ordering consistency
         Validate hash chain continuity (prev_hash to entry_hash linkage per stream)
         Detect duplicates and malformed entries
         Report Pass / Fail / Inconclusive / Unsupported
MAY NOT: Mutate trace entries
         Repair broken chains
         Recompute entry hashes (backend-specific; verifier validates linkage, not correctness)
         Append new entries
         Execute tools or workflows
         Approve operations
```

### Operation Replay Verifier (OperationReplayVerifier)

```
MAY:     Read desktop operation descriptors
         Read trace entries
         Validate correspondence by explicit IDs
         Report Pass / Fail / Inconclusive / Unsupported with findings
MAY NOT: Execute tools or workflows
         Approve operations
         Export evidence
         Mutate any state
         Instantiate runners, tools, exporters, gates, or policies
```

### CLI Commands (trace-verify, operation-replay)

```
MAY:     Load trace entries from SQLite store (paginated)
         Run verifier / replay verifier
         Print structured report
         Exit with distinct exit codes
MAY NOT: Claim full cryptographic verification
         Mutate trace entries or state
         Execute, approve, or export
```

**Exit code scheme (both commands):**
- 0 = Pass
- 1 = Operational error (file not found, DB not accessible, malformed input)
- 2 = Fail (integrity violation or correspondence failure)
- 3 = Inconclusive (some evidence exists but insufficient)
- 4 = Unsupported (operation type has no trace representation)

---

## Section 3: Security Review Checklist

A structured checklist for an external reviewer:

### 3.1 Filesystem Sandbox

- [ ] Verify all local tools resolve paths through `resolve_workspace_path()`
- [ ] Verify path traversal (`../../../../etc/passwd`) is rejected at validation
- [ ] Verify symlink escape is blocked at final component (`O_NOFOLLOW` / `FILE_FLAG_NO_REPARSE_POINT`)
- [ ] Verify Unix intermediate-directory TOCTOU is closed via `openat` + `dirfd`
- [ ] Verify Windows intermediate-directory TOCTOU is closed via `NtCreateFile` handle-relative traversal
- [ ] Note: Windows final file component still relies on the 72B no-follow final-write path; directory traversal uses hardened NtCreateFile handle-relative traversal. Residual is safe-failure/limited final-component race posture, not a known arbitrary write vulnerability.

### 3.2 Approval Gate Integrity

- [ ] Verify every non-auto-allowed tool call goes through policy evaluation
- [ ] Verify approval resumption is bound to original canonical workspace (69B)
- [ ] Verify approval_request_id is mandatory and non-empty (88B)
- [ ] Verify tool-name binding is display-only context (88B)

### 3.3 Trace Chain Integrity

- [ ] Verify trace entries are append-only through the API
- [ ] Verify hash chain linkage exists (prev_hash to entry_hash per stream)
- [ ] Verify trace verifier detects: tampered hash, deleted entry, swapped entries, multi-stream tamper
- [ ] Note: Verifier validates chain continuity, not hash correctness. No full hash recomputation is performed.

### 3.4 Operation Correspondence

- [ ] Verify workflow initiation reports Inconclusive (workflow modules declare appends_trace: false)
- [ ] Verify approval resolution reports Pass when ARID matches trace event, Fail when contradicted
- [ ] Verify evidence export reports Unsupported (export does not emit trace events)
- [ ] Verify ARID is preferred over tool_call_id (93A); fallback reported as Warning finding

### 3.5 Authority Boundary Enforcement

- [ ] Verify desktop UI does not import backend crates
- [ ] Verify DTO layer does not construct RunConfig
- [ ] Verify verifier does not mutate, repair, append, or execute
- [ ] Verify operation replay verifier does not instantiate runners/tools/exporters
- [ ] Verify CLI commands exit non-zero on operational errors

### 3.6 Dependency Audit

- [ ] Rerun `cargo audit` and compare with recorded results
- [ ] Last recorded dependency audit: 0 vulnerabilities, 15 warnings at Wave 82A. Audit should be refreshed before any production-readiness claim.

### 3.7 No-Overclaim Verification

- [ ] Scan documentation for unsupported affirmative claims
- [ ] Verify caveat language is present where needed
- [ ] Verify "not production-ready", "not formal security review", "not full immutability" are stated

---

## Section 4: Caveat Ledger

This section honestly documents what 94A does NOT do and what OpenWand does NOT claim.

| # | Caveat | Detail |
|---|--------|--------|
| CV-1 | Not a formal security review | This document is preparation for review. No penetration testing, fuzzing, or adversarial probing was performed. Self-audit only. |
| CV-2 | No full hash recomputation | Trace verifier validates chain continuity (prev_hash to entry_hash linkage). It does not recompute BLAKE3 hashes. Backend-specific hash recomputation is deferred. |
| CV-3 | No full immutability proof | The physical SQLite file is technically mutable by direct DB access. The API authority boundary is append-only. The verifier exists to detect tampering after the fact, not prevent it at the physical layer. |
| CV-4 | Linux GUI runtime deferred | Desktop compilation validated on Linux (85A). GUI runtime not validated - no display server available in test environment. |
| CV-5 | macOS not validated | No macOS environment available. Compilation and runtime are untested. |
| CV-6 | Provider validation limited | Provider validation remains limited to the documented matrix: 5 models across 2 provider families, including LM Studio and Z.AI. Direct OpenAI/Anthropic/Ollama coverage remains deferred unless separately validated. |
| CV-7 | TD-93B-1: module name debt | `crates/app/src/operation_audit.rs` contains the operation_replay module code due to filesystem virtualization during Wave 93B. Functional behavior is correct; maintainability seam tracked for rename. |
| CV-8 | Windows final-component TOCTOU residual | Windows final file component still relies on the 72B no-follow final-write path; directory traversal uses hardened NtCreateFile handle-relative traversal. Residual is safe-failure/limited final-component race posture, not a known arbitrary write vulnerability. |
| CV-9 | Transitive dependency warnings | Last recorded dependency audit: 0 vulnerabilities, 15 warnings at Wave 82A. All transitive via Dioxus desktop stack (12) or Loro CRDT (1) or CSS selector path (2). Audit should be refreshed before any production-readiness claim. |
| CV-10 | Workflow trace gap | Workflow modules declare `appends_trace: false`. No dedicated workflow trace events are emitted. Operation replay reports Inconclusive for workflow initiation. This is a known architectural decision, not a defect. |
| CV-11 | Evidence export trace gap | Evidence export does not emit trace events. Operation replay reports Unsupported for evidence export. The export operation integrity is verified through checksum scope rules (88C), not through trace correspondence. |

---

## Section 5: Review-Ready Assets

### Release Lineage

| Version | Tag | Date | Theme |
|---------|-----|------|-------|
| v0.1.0-alpha | `v0.1.0-alpha` | 2026-06-12 | First public alpha |
| v0.1.0-beta | `v0.1.0-beta` | 2026-06-12 | Public beta |
| v0.2.0 | `v0.2.0` | 2026-06-12 | Live workflow surfaces |
| v0.3.0 | `v0.3.0` | 2026-06-13 | Live workflow wiring |
| v0.4.0 | `v0.4.0` | 2026-06-13 | From observation to operation |
| v0.5.0 | (in progress) | - | Runtime integrity hardening |

### Test Baseline

| Metric | Value |
|--------|-------|
| Total tests | 4,068 |
| Failures | 0 |
| Platform | Windows (full), Linux (compile + tests) |
| Desktop feature build | PASS (0 errors, 0 warnings) |
| Production crate clippy | 0 warnings (11 crates) |
| App crate clippy warnings | ~50 (accepted cosmetic, all in test code) |

### CLI Commands Available for Verification

| Command | Purpose | Read-only? |
|---------|---------|------------|
| `openwand trace-verify <session-id>` | Validate trace chain integrity, ordering, hash continuity | Yes - reads store, prints report, exits with code |
| `openwand operation-replay --session <id> --operations <ops.json>` | Validate desktop operation correspondence against trace | Yes - reads store + JSON, prints report, exits with code |
| `openwand eval` | Run evaluation scenarios | Yes (test harness) |
| `openwand run` | Launch desktop UI | Interactive |

### Wave History Summary (v0.5 arc)

| Wave | Description | Tag |
|------|-------------|-----|
| 91A | Post-v0.4 roadmap reset | `wave-91a-lock` |
| 92A | Trace verifier core: append-only + hash chain | `wave-92a-lock` |
| 92B | Trace verifier tamper detection + CLI command | `wave-92b-lock` |
| 93A | Operation replay: desktop operations to trace events | `wave-93a-lock` |
| 93B | Operation replay CLI command | `wave-93b-lock` |
| 94A | Security review preparation | (this wave) |

### Known Deferred Risks Registry

See `docs/DEFERRED_RISKS.md` for the full deferred-risk ledger with statuses. Key items:

- DEFERRED-001: App crate clippy warnings (accepted cosmetic)
- DEFERRED-002: Cargo audit transitive warnings (upstream-blocked)
- DEFERRED-004: Trace immutability - chain continuity validated; hash recomputation and physical-layer immutability still deferred
- DEFERRED-008: Sandbox TOCTOU - closed on Unix and Windows directory traversal; Windows final-component residual (safe-failure mode)

---

## Authority Posture Summary

```
v0.4.0 established: Desktop UI may REQUEST operations through existing authority gates.
v0.5.0 extends:     Runtime verifier may READ and VALIDATE trace/store/workflow records.
                    The verifier does not mutate, execute, approve, or dispatch.
                    The verifier is a new READ authority, not a new WRITE authority.
```

```
Prepare for review. Do not claim review.
```

---

*This document was prepared by the same agent that wrote the code. An independent review would provide stronger assurance.*
