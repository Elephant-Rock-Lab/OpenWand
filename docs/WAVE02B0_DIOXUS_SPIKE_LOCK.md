# WAVE 02B-0 — DIOXUS + RUNTIME SPIKE — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** Dioxus desktop boot + async runtime + signal bridge spike

## Proven

- `openwand-ui.exe` launches a Dioxus desktop window (800×600, WebView2)
- Desktop UI is feature-gated (`desktop` feature) and does not affect CLI builds
- Dioxus 0.7 desktop runtime can run async tasks via `spawn(async { ... })`
- `GlobalSignal` can carry async-produced events into RSX rendering
- Window process exits cleanly under external termination (~24MB footprint)
- Dioxus + tokio runtime coexist without conflicts

## Dependencies Pinned

```toml
dioxus = { version = "=0.7.9", features = ["desktop"] }
dioxus-desktop = { version = "=0.7.9" }
```

## Accepted Limitation

Component-scoped receiver cleanup was not proven in this spike.

This is deferred to 02b-3 because the real cleanup boundary is not a synthetic producer
flag; it is the live `AgentEvent` subscription owned by the run/session view. That
implementation should use a component-lifetime cancellation primitive, likely
`CancellationToken`, and must be tested when the live run view exists.

## Files Added

- `crates/app/Cargo.toml` — Added `desktop` feature with dioxus deps, `openwand-ui` bin
- `crates/app/src/ui_main.rs` — Spike: window + async producer + GlobalSignal rendering

## Tests

197 existing tests still pass (zero regressions from desktop feature addition).
CLI binary (`openwand`) compiles independently without desktop feature.
