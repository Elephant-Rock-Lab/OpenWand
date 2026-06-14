# Deferred Audit Risks

Wave 69F introduced this ledger. Wave 69G closes or reclassifies items.

## Cargo Audit Results (DEFERRED-002 — closed by recording)

**Run date:** 2026-06-11
**Result:** 0 vulnerabilities, 15 warnings

### Unmaintained advisories (13)

| Advisory | Crate | Dependency Path | Direct? | Desktop-only? |
|----------|-------|----------------|---------|---------------|
| RUSTSEC-2024-0413 | atk 0.18.2 | gtk → wry → dioxus-desktop | No | Yes |
| RUSTSEC-2024-0416 | atk-sys 0.18.2 | gtk-sys → wry → dioxus-desktop | No | Yes |
| RUSTSEC-2024-0412 | gdk 0.18.2 | wry → dioxus-desktop | No | Yes |
| RUSTSEC-2024-0418 | gdk-sys 0.18.2 | wry → dioxus-desktop | No | Yes |
| RUSTSEC-2024-0415 | gtk 0.18.2 | wry → dioxus-desktop | No | Yes |
| RUSTSEC-2024-0420 | gtk-sys 0.18.2 | wry → dioxus-desktop | No | Yes |
| RUSTSEC-2024-0419 | gtk3-macros 0.18.2 | gtk → wry → dioxus-desktop | No | Yes |
| RUSTSEC-2024-0411 | gdkwayland-sys 0.18.2 | tao → dioxus-desktop | No | Yes |
| RUSTSEC-2024-0414 | gdkx11-sys 0.18.2 | tao → dioxus-desktop | No | Yes |
| RUSTSEC-2024-0436 | paste 1.0.15 | rav1e → ravif → image → dioxus-desktop | No | Yes |
| RUSTSEC-2024-0370 | proc-macro-error 1.0.4 | gtk3-macros/glib-macros → gtk → dioxus-desktop | No | Yes |
| RUSTSEC-2023-0089 | atomic-polyfill 1.0.3 | heapless → postcard → loro → openwand-session | No | No (Loro CRDT path) |
| RUSTSEC-2025-0057 | fxhash 0.2.1 | selectors → kuchikiki → wry → dioxus-desktop | No | Yes |

**Summary:** 12 of 13 unmaintained advisories are transitive through Dioxus desktop rendering.
1 (atomic-polyfill) is transitive through Loro CRDT, reaching openwand-session via non-desktop
path. However, atomic-polyfill is only used on targets without native atomic operations;
on x86_64 (the current target), it is dead code in the final binary.

### Unsound advisories (2)

| Advisory | Crate | Severity | Dependency Path | Affects OpenWand? |
|----------|-------|----------|----------------|-------------------|
| RUSTSEC-2024-0429 | glib 0.18.5 | Unsound Iterator impl | gtk → dioxus-desktop | No — rendering path only |
| RUSTSEC-2026-0097 | rand 0.7.3 | Unsound with custom logger | selectors → kuchikiki → dioxus-desktop | No — CSS selector path only |

**Summary:** Both unsound advisories are in transitive desktop-rendering dependencies.
Neither touches OpenWand data, crypto, network, or storage paths.

### Direct dependencies with advisories

None. Zero OpenWand direct dependencies have vulnerability or unmaintained advisories.

---

## Deferred Risk Status

### DEFERRED-001: openwand-app clippy -D warnings (57 style warnings)
- **Status:** Accepted non-blocking
- **Category:** Cosmetic
- **Scope:** `cargo clippy -p openwand-app --all-features -- -D warnings` produces 57 warnings
- **Detail:** All 57 are in `#[cfg(test)]` test helper functions and test-only structs. Zero affect production code quality.
- **Resolution path:** Add crate-level `#![allow(...)]` or refactor test helpers into a separate test-support crate.

### DEFERRED-002: cargo audit dependency warnings
- **Status:** Closed by recording
- **Category:** Dependency
- **Detail:** 15 warnings (13 unmaintained + 2 unsound), all transitive via Dioxus desktop stack or Loro CRDT. Zero direct dependency issues. Zero vulnerabilities. Re-verified Wave 82A.
- **Resolution path:** Re-evaluate when Dioxus/Loro release updates with newer transitive deps.

### DEFERRED-003: unsafe-env-test claim correction
- **Status:** Closed by claim correction
- **Category:** Documentation
- **Detail:** HB-G4 updated from "Zero `unsafe` in OpenWand code" to "Zero `unsafe` in OpenWand production code (test-only env var manipulation excepted)." The 2 `unsafe` blocks in `provider_registry.rs` are in `#[cfg(test)]` for `std::env::set_var`. No production code uses `unsafe`.
- **Underlying fact unchanged:** test-only `unsafe` blocks still exist.

### DEFERRED-004: trace immutability claim correction
- **Status:** Partially closed by 92A-92B + 94A documentation
- **Category:** Architecture / Verification
- **Detail:** README.md updated: "immutable evidence chain" changed to "append-only evidence chain (structural hash-chaining; immutability enforcement deferred to verifier, not yet implemented)." Wave 92A implemented `TraceVerifier` that validates chain continuity (prev_hash to entry_hash linkage per stream), ordering, duplicates, and well-formedness. Wave 92B exposed it via `openwand trace-verify` CLI with tamper detection tests. Wave 94A documented the verifier authority boundary and caveats.
- **What is now closed:** No verifier existed (closed 92A). No CLI verification path existed (closed 92B). No deterministic chain-continuity tamper detection existed (closed 92A-92B). No authority-boundary documentation existed (closed 94A).
- **What remains open:** Backend-specific hash recomputation (verifier validates linkage, not BLAKE3 correctness). Full cryptographic hash correctness verification. Formal immutability proof. Physical-layer tamper prevention (SQLite file is technically mutable by direct access — verifier detects after the fact, does not prevent).

### DEFERRED-005: MutationHelper live-event correctness
- **Status:** Closed with rationale and tests
- **Category:** Runtime correctness
- **Detail:** 3 direct tests added in `crates/session/src/mutation.rs`:
  - `mutation_helper_apply_emits_agent_event` — proves emission after apply
  - `mutation_helper_apply_trace_first_then_event` — proves ordering (trace → event)
  - `mutation_helper_apply_event_send_failure_does_not_abort_mutation` — proves best-effort emission
- **Architectural guarantee:** MutationHelper operates in single-writer mode. SessionRunner holds the only instance, gated by run_lock. `let _ = tx.send(...)` makes AgentEvent emission observational and best-effort; trace append remains the durable record.
- **Underlying fact unchanged:** no concurrent mutation tests exist, but concurrent access is architecturally prevented.

### DEFERRED-006: STATE.md and documentation update
- **Status:** Closed by update
- **Category:** Documentation
- **Detail:** STATE.md fully rewritten, KNOWN_GAPS.md updated with halt-era closures and current gaps, WAVES.md extended through 69G, UI_DESIGN_SYSTEM.md token names corrected, README.md immutability claims corrected.

### DEFERRED-007: Local branch publication
- **Status:** Accepted non-blocking / publication pending by user decision
- **Category:** Publication process
- **Detail:** 23 commits ahead of origin/master (Wave 50A through 69G). Not pushed in this wave.
- **Resolution path:** User decides when and how to publish.

### DEFERRED-008: Sandbox TOCTOU boundary
- **Status:** ✅ Closed by NtCreateFile handle-relative traversal (78C) — Unix (73B) + Windows (78C)
- **Category:** Filesystem security
- **Threat model:** A local concurrent filesystem adversary (a separate process running on the same machine) replaces a validated directory with a symlink between the time `resolve_workspace_path()` canonicalizes and validates the path and the time `std::fs::write()` (or `create_dir_all()`) follows the path to write. This is a TOCTOU (time-of-check/time-of-use) race.
- **What is fixed:** Direct path traversal (`../../../etc/passwd`), static symlink escapes, Windows drive/UNC prefixes, and parent directory (`..`) components are all rejected at validation time. Production-path E2E test proves `../../../etc/escape.txt` is blocked even when policy approves the write.
- **What was hardened (72B):** `write_file_no_follow()` uses `FILE_FLAG_NO_REPARSE_POINT` on Windows and `O_NOFOLLOW` on Unix to prevent following symlinks at the **final path component** during write.
- **What was hardened (73B — Unix):** `WorkspaceWriteHandle` uses `openat` + `dirfd` + `O_NOFOLLOW` to walk each path component relative to the parent directory's file descriptor. Intermediate directory symlinks are detected (`ELOOP`) and rejected. Directory creation uses `mkdirat` + immediate `openat(O_NOFOLLOW)` to close the mkdir-to-open race. On Linux, macOS, and FreeBSD, the intermediate-directory TOCTOU is **fully closed**.
- **What was hardened (78C — Windows):** `WorkspaceWriteHandle.windows_verify_path_components()` uses `NtCreateFile` with `OBJECT_ATTRIBUTES.RootDirectory` to open each intermediate directory relative to a parent handle. `NtQueryInformationFile(FileBasicInformation)` checks for reparse points on each opened handle. Directory creation uses `NtCreateFile(FILE_CREATE)` + immediate re-open with `FILE_OPEN_REPARSE_POINT` for race verification. The intermediate-directory TOCTOU is **fully closed** on Windows, matching the Unix `openat` approach.
- **What remains (Windows final component):** Between the handle-relative reparse check and the `write_file_no_follow()` call, there is a small window where an adversary could replace a regular file with a symlink. `write_file_no_follow()` uses `FILE_FLAG_NO_REPARSE_POINT` which prevents following the symlink, so the write fails safely. This is the same residual accepted in 72B.
- **Risk acceptance rationale:** Intermediate-directory TOCTOU is fully closed on both Unix and Windows. The final-component residual is handled by `FILE_FLAG_NO_REPARSE_POINT` (safe failure mode). The remaining window is between handle-relative verification and the actual write, not between path resolution and write.
- **Resolution path:** CLOSED. VB-1 resolved.

### DEFERRED-009: Hosted provider validation
- **Status:** ✅ Closed by Z.AI validation (77B)
- **Category:** Testing
- **Detail:** Z.AI hosted endpoint validated with two models (glm-4.5-air, glm-5.1) across simple turn, trace attribution, tool calling, and sandbox refusal. Functional equivalence via MCP API source. OpenAI direct, Anthropic, and other hosted providers remain untested.
- **Resolution path:** CLOSED for beta criterion. Additional providers are post-beta.

### DEFERRED-010: Desktop UI rendering validation
- **Status:** ✅ Closed by Windows UI Automation validation (77C)
- **Category:** Testing
- **Detail:** Desktop UI validated through Windows UI Automation API. 53 accessible elements verified. Full interaction path exercised: launch → shell renders → session creation → send triggers run → state transitions → error display → run completion. Tab switching and visual styling not validated.
- **Resolution path:** CLOSED for beta criterion. Visual styling and tab switching are post-beta.
