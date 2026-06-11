# Deferred Audit Risks

Wave 69F introduced this ledger. Wave 69G closes or reclassifies items.

## Cargo Audit Results (DEFERRED-002 — closed by recording)

**Run date:** 2026-06-11
**Result:** 0 vulnerabilities, 16 warnings

### Unmaintained advisories (14)

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
- **Detail:** 16 warnings (14 unmaintained + 2 unsound), all transitive via Dioxus desktop stack or Loro CRDT. Zero direct dependency issues. Zero vulnerabilities.
- **Resolution path:** Re-evaluate when Dioxus/Loro release updates with newer transitive deps.

### DEFERRED-003: unsafe-env-test claim correction
- **Status:** Closed by claim correction
- **Category:** Documentation
- **Detail:** HB-G4 updated from "Zero `unsafe` in OpenWand code" to "Zero `unsafe` in OpenWand production code (test-only env var manipulation excepted)." The 2 `unsafe` blocks in `provider_registry.rs` are in `#[cfg(test)]` for `std::env::set_var`. No production code uses `unsafe`.
- **Underlying fact unchanged:** test-only `unsafe` blocks still exist.

### DEFERRED-004: trace immutability claim correction
- **Status:** Closed by documentation downgrade
- **Category:** Documentation
- **Detail:** README.md updated: "immutable evidence chain" → "append-only evidence chain (structural hash-chaining; immutability enforcement deferred to verifier, not yet implemented)." Trace store deletion/mutation prevention is not enforced by a runtime verifier.
- **Underlying fact unchanged:** no trace verifier exists.

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
- **Status:** Accepted residual risk
- **Category:** Filesystem security
- **Threat model:** A local concurrent filesystem adversary (a separate process running on the same machine) replaces a validated directory with a symlink between the time `resolve_workspace_path()` canonicalizes and validates the path and the time `std::fs::write()` (or `create_dir_all()`) follows the path to write. This is a TOCTOU (time-of-check/time-of-use) race.
- **What is fixed:** Direct path traversal (`../../../etc/passwd`), static symlink escapes, Windows drive/UNC prefixes, and parent directory (`..`) components are all rejected at validation time. Production-path E2E test proves `../../../etc/escape.txt` is blocked even when policy approves the write.
- **What remains:** Handle-relative filesystem operations (e.g., Windows `CreateFile` with `FILE_FLAG_NO_REPARSE_POINT`, Linux `O_NOFOLLOW`) would close the TOCTOU gap by ensuring the write target cannot be replaced between validation and use. This requires platform-specific unsafe code or a native crate.
- **Risk acceptance rationale:** The adversary model requires local concurrent filesystem access on the same machine as OpenWand, with timing precision to win the race window. This is not a model-driven or network-accessible attack. The existing sandbox blocks all static path manipulations.
- **Resolution path:** Handle-relative writes in a future wave, or explicit documentation that OpenWand assumes a non-adversarial local filesystem.
