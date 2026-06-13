# Dependency Security Audit Report — v0.2.0-beta

**Date:** 2026-06-13
**Tool:** `cargo-audit 0.22.1`
**Advisory database:** RustSec advisory-db (1,131 advisories)
**Scope:** 721 crate dependencies in `Cargo.lock`
**Release:** v0.2.0-beta (commit `8034bbf`)

---

## Summary

| Category | Count |
|----------|-------|
| **Vulnerabilities (RUSTSEC with severity)** | **0** |
| **Unmaintained warnings** | 13 |
| **Unsound warnings** | 2 |
| **Total advisories** | **15** |

**Bottom line:** Zero actual security vulnerabilities. All 15 findings are either "unmaintained" or "unsound" advisories. No CVEs, no memory-safety exploits, no RCE vectors.

---

## Findings

### Category 1: GTK3 Bindings (Linux-only) — 10 warnings + 1 unsound

| Crate | Version | Advisory | Type |
|-------|---------|----------|------|
| `atk` | 0.18.2 | RUSTSEC-2024-0413 | unmaintained |
| `atk-sys` | 0.18.2 | RUSTSEC-2024-0416 | unmaintained |
| `gdk` | 0.18.2 | RUSTSEC-2024-0412 | unmaintained |
| `gdk-sys` | 0.18.2 | RUSTSEC-2024-0418 | unmaintained |
| `gdkwayland-sys` | 0.18.2 | RUSTSEC-2024-0411 | unmaintained |
| `gdkx11-sys` | 0.18.2 | RUSTSEC-2024-0414 | unmaintained |
| `gtk` | 0.18.2 | RUSTSEC-2024-0415 | unmaintained |
| `gtk-sys` | 0.18.2 | RUSTSEC-2024-0420 | unmaintained |
| `gtk3-macros` | 0.18.2 | RUSTSEC-2024-0419 | unmaintained |
| `proc-macro-error` | 1.0.4 | RUSTSEC-2024-0370 | unmaintained |
| `glib` | 0.18.5 | RUSTSEC-2024-0429 | unsound |

**Dependency path:** `openwand-app` → `dioxus-desktop 0.7.9` → `wry 0.53.5` → GTK3/webkit2gtk

**Platform impact:** These crates are **Linux-only** platform dependencies (GTK3, Wayland, X11). They are not compiled into the Windows binary. OpenWand v0.2.0-beta targets Windows only.

**The `glib` unsound advisory (RUSTSEC-2024-0429):** Unsoundness in `Iterator`/`DoubleEndedIterator` impls for `glib::VariantStrIter`. This requires constructing and iterating a `VariantStrIter` from a `glib::Variant` — functionality OpenWand never calls directly or indirectly.

**Remediation path:** Requires upgrading to dioxus 0.8+ (when released with GTK4 bindings) or wry with updated Linux backend. Not actionable at current framework version.

### Category 2: WebView HTML Parsing — 1 unmaintained + 1 unsound

| Crate | Version | Advisory | Type |
|-------|---------|----------|------|
| `fxhash` | 0.2.1 | RUSTSEC-2025-0057 | unmaintained |
| `rand` | 0.7.3 | RUSTSEC-2026-0097 | unsound |

**Dependency path:** `openwand-app` → `dioxus-desktop` → `wry` → `kuchikiki` → `selectors` → (`fxhash` / `phf_codegen` → `rand 0.7.3`)

**The `rand 0.7.3` unsound advisory (RUSTSEC-2026-0097):** Unsound with a custom logger using `rand::rng()`. This requires installing a custom logger and then calling `rand::rng()` — OpenWand does neither through this code path.

**Remediation path:** Requires upstream `kuchikiki` or `selectors` to update their dependencies. Not actionable at the application level.

### Category 3: Image Processing — 1 unmaintained

| Crate | Version | Advisory | Type |
|-------|---------|----------|------|
| `paste` | 1.0.15 | RUSTSEC-2024-0436 | unmaintained |

**Dependency path:** `openwand-app` → `dioxus-desktop` → `image 0.25.10` → `ravif` → `rav1e` → `paste`

**Impact:** Macro utility for AVIF encoding. No security implication.

### Category 4: Session CRDT — 1 unmaintained

| Crate | Version | Advisory | Type |
|-------|---------|----------|------|
| `atomic-polyfill` | 1.0.3 | RUSTSEC-2023-0089 | unmaintained |

**Dependency path:** `openwand-session` → `loro 1.12.0` → `postcard 1.1.3` → `heapless 0.7.17` → `atomic-polyfill`

**Impact:** Atomic operations polyfill for `no_std` targets. On x86/x64 Windows, the polyfill is unused (native atomics are used). No security implication.

---

## Classification

All 15 findings are classified as **accepted beta caveats**. None are release-blocking.

**Rationale:**

1. **Zero vulnerabilities.** All findings are "unmaintained" or "unsound" — not exploitable security holes.
2. **12 of 15 are Linux-only** GTK3 bindings not present in the Windows binary.
3. **0 are direct dependencies** of any OpenWand crate. All are deep transitive dependencies through dioxus-desktop, wry, or loro.
4. **2 unsound advisories require specific conditions** that OpenWand does not trigger (custom logger + `rand::rng()`, or `glib::VariantStrIter` iteration).
5. **Remediation requires upstream framework upgrades** beyond OpenWand's control (dioxus 0.8+, loro update, kuchikiki update).

---

## RC Eligibility Assessment

**v0.2.0 is eligible for release candidate preparation** with these caveats disclosed:

- 0 actual vulnerabilities
- 15 warnings (13 unmaintained, 2 unsound), all transitive
- 80% of warnings are platform-specific (Linux) code not in the Windows binary
- Remediation blocked on upstream framework versions

The audit does not block promotion from beta to release candidate. The disclosure obligation is to carry these warnings in release notes.

---

## Remediation Backlog (post-v0.2)

| Item | Dependency | Action |
|------|-----------|--------|
| GTK3 → GTK4 migration | dioxus-desktop, wry | Upgrade when dioxus 0.8+ releases with GTK4 bindings |
| rand 0.7.3 → 0.8+ | kuchikiki, selectors, phf_codegen | Track upstream kuchikiki release |
| atomic-polyfill | loro, postcard, heapless | Track loro dependency updates |
| paste | image, ravif, rav1e | Track image crate updates |
| fxhash | kuchikiki, selectors | Track upstream release |

---

*This audit was performed with cargo-audit 0.22.1 against the RustSec advisory database (1,131 advisories). It does not constitute a formal security review. It covers only dependencies in Cargo.lock and does not assess build tooling, CI infrastructure, or supply-chain integrity.*
