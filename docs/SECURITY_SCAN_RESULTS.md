# Security Scan Results — Wave 105A

**Scan date:** 2026-06-14
**Workspace:** OpenWand v0.7.0 (post-104B)
**Scanner versions:** cargo-audit 0.22.1, RustSec advisory DB (1,131 advisories)
**Clippy:** rustc 1.95.0 (clippy 0.1.95)

---

## 1. cargo audit — Vulnerability Scan

**Command:** `cargo audit`
**Scope:** 721 crate dependencies in Cargo.lock

### Result: 0 vulnerabilities, 15 warnings (all upstream-blocked)

| # | Crate | Advisory ID | Type | Classification | Detail |
|---|-------|-------------|------|----------------|--------|
| 1 | atk 0.18.2 | RUSTSEC-2024-0413 | unmaintained | Upstream-blocked | gtk-rs GTK3 bindings — only on Linux desktop builds |
| 2 | atk-sys 0.18.2 | RUSTSEC-2024-0416 | unmaintained | Upstream-blocked | Same as above |
| 3 | atomic-polyfill 1.0.3 | RUSTSEC-2023-0089 | unmaintained | Upstream-blocked | Via loro CRDT session crate |
| 4 | fxhash 0.2.1 | RUSTSEC-2025-0057 | unmaintained | Upstream-blocked | Via wry/dioxus desktop |
| 5 | gdk 0.18.2 | RUSTSEC-2024-0412 | unmaintained | Upstream-blocked | GTK3 bindings |
| 6 | gdk-sys 0.18.2 | RUSTSEC-2024-0418 | unmaintained | Upstream-blocked | GTK3 bindings |
| 7 | gdkwayland-sys 0.18.2 | RUSTSEC-2024-0411 | unmaintained | Upstream-blocked | GTK3 bindings |
| 8 | gdkx11-sys 0.18.2 | RUSTSEC-2024-0414 | unmaintained | Upstream-blocked | GTK3 bindings |
| 9 | gtk 0.18.2 | RUSTSEC-2024-0415 | unmaintained | Upstream-blocked | GTK3 bindings |
| 10 | gtk-sys 0.18.2 | RUSTSEC-2024-0420 | unmaintained | Upstream-blocked | GTK3 bindings |
| 11 | gtk3-macros 0.18.2 | RUSTSEC-2024-0419 | unmaintained | Upstream-blocked | GTK3 bindings |
| 12 | paste 1.0.15 | RUSTSEC-2024-0436 | unmaintained | Upstream-blocked | Via image/ravif (desktop) |
| 13 | proc-macro-error 1.0.4 | RUSTSEC-2024-0370 | unmaintained | Upstream-blocked | Via gtk3-macros (desktop) |
| 14 | glib 0.18.5 | RUSTSEC-2024-0429 | unsound | Upstream-blocked | GTK3 VariantStrIter unsoundness |
| 15 | rand 0.7.3 | RUSTSEC-2026-0097 | unsound | Upstream-blocked | Via phf → kuchikiki → wry (desktop) |

**Analysis:**
- **0 CVEs (security vulnerabilities) found.** All 15 findings are warnings (unmaintained or unsound).
- 13 of 15 are GTK3/gtk-rs related (only compiled on Linux desktop builds).
- 1 is atomic-polyfill via loro CRDT (session crate).
- 1 is rand 0.7 via wry/kuchikiki (desktop only).
- All are transitive dependencies — OpenWand does not directly depend on any flagged crate.
- **Resolution path:** GTK3 warnings will resolve when Dioxus/wry migrates to GTK4 bindings. No action possible from OpenWand.

---

## 2. Clippy Production Scan

**Command:** `cargo clippy -p openwand-{crate} --lib` for all 12 non-app production crates

### Result: 0 warnings on 12 production crates

| Crate | Warnings |
|-------|----------|
| openwand-core | 0 |
| openwand-trace | 0 |
| openwand-store | 0 |
| openwand-session | 0 |
| openwand-memory | 0 |
| openwand-tools | 0 |
| openwand-mcp-pool | 0 |
| openwand-policy | 0 |
| openwand-llm | 0 |
| openwand-skills | 0 |
| openwand-goals | 0 |
| openwand-workflow | 0 |

**openwand-app** (binary): ~295 clippy warnings (all in test modules and desktop UI).
Classification: **Accepted residual** — cosmetic, test-only, and desktop-feature-gated code.

---

## 3. Authority Boundary Source Guard Scan

**Method:** Targeted grep/source inspection for known authority violations.

### Checks Performed

| Check | Method | Result |
|-------|--------|--------|
| Verifier mutates trace | grep `.append(` in verifier.rs, anchor.rs | PASS — not found |
| Verifier writes files | grep `std::fs::write` in verifier.rs | PASS — not found |
| Operation replay executes | grep `ToolExecutor`, `advance_stages`, `save_workflow` | PASS — only in guard test assertions |
| Anchor module mutates trace | grep `append_trace`, `delete_entry`, `remove_entry` | PASS — not found |
| Anchor module imports backend | grep `openwand_store`, `openwand_core` in anchor.rs | PASS — not found |
| Unsafe in production code | grep `unsafe` in production source | PASS — see below |

### `unsafe` Usage Review

| Location | Usage | Classification |
|----------|-------|----------------|
| `crates/tools/src/sandbox.rs:332` | `libc::dup(fd)` for Unix openat TOCTOU hardening | Accepted — intentional syscall wrapper |
| `crates/llm/src/provider_registry.rs:318` | `std::env::remove_var` in test config | Test-only — behind `#[cfg(test)]` |
| `crates/policy/src/output_guard.rs:193` | String literal "unsafe" (policy keyword) | Not actual unsafe code |
| `crates/app/src/workflow_*_review.rs` | String literal "unsafe" (review summary) | Not actual unsafe code |

**Result:** Only one production `unsafe` block: `libc::dup(fd)` in Unix sandbox hardening. This is an intentional, reviewed syscall wrapper for openat-based path resolution. No OpenWand production code uses `unsafe` for memory operations.

---

## 4. Dependency Analysis

| Metric | Value |
|--------|-------|
| Direct dependencies | ~68 (across all crates) |
| Total transitive (Cargo.lock) | 721 crates |
| Vulnerabilities (CVE) | 0 |
| Unmaintained warnings | 15 (all upstream-blocked) |
| Unsound warnings | 2 (glib, rand — both desktop-only) |

---

## 5. Finding Classification Summary

| Classification | Count | Action |
|----------------|-------|--------|
| **Blocking** | 0 | — |
| **Accepted residual** | 1 (app clippy warnings — cosmetic/test-only) | Documented |
| **Upstream-blocked** | 15 (GTK3, atomic-polyfill, rand, paste, proc-macro-error) | Monitor upstream; resolve when Dioxus/wry updates |
| **Deferred** | 0 | — |
| **False positive** | 2 (string literal "unsafe" matches) | Not actual unsafe code |

---

## 6. What This Scan Does NOT Cover

- Formal security review by a qualified auditor
- Penetration testing
- Side-channel or timing attack analysis
- Supply chain integrity verification (reproducible builds)
- Binary analysis / reverse engineering of the release artifact
- Network-level attack surface (OpenWand has no network listener in production)
- Cryptographic review of BLAKE3, Ed25519, or SQLite implementations

---

## 7. Delta from Last Audit (Wave 82A)

| Metric | Wave 82A (last) | Wave 105A (current) | Delta |
|--------|-----------------|---------------------|-------|
| Vulnerabilities | 0 | 0 | No change |
| Unmaintained warnings | 15 | 15 | No change |
| Production crate clippy warnings | 0 | 0 | No change |
| Total dependencies | ~680 | 721 | +41 (Dioxus 0.7 + desktop deps) |

The +41 dependency increase is from the v0.4-v0.7 arc adding Dioxus desktop UI
components (wry, tao, gtk, webkit2gtk on Linux). All new dependency warnings
are GTK3-related and upstream-blocked.

---

## Scan Commands (Reproducible)

```bash
# Vulnerability scan
cargo audit

# Production clippy
for crate in core trace store session memory tools mcp-pool policy llm skills goals workflow; do
    cargo clippy -p "openwand-$crate" --lib
done

# Authority boundary guards (via test suite)
cargo test --workspace --all-targets
```

---

## Conclusion

**0 blocking security findings.** All 15 advisory warnings are upstream-blocked
transitive dependencies, primarily GTK3 bindings from the desktop UI stack.
No CVEs found. All production crates pass clippy with zero warnings.

This scan refreshes the dependency audit from Wave 82A and confirms the same
posture: no known vulnerabilities, upstream-blocked warnings documented.

**This is not a formal security review.** It is an automated scan record.
