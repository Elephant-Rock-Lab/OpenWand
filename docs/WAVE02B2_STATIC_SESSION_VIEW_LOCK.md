# WAVE 02B-2 — STATIC SESSION VIEW — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** Dioxus static shell + UI session service + session list + detail pane

## Proven

- Desktop app opens with session list sidebar and detail pane
- "+ New" button creates a session and selects it
- Clicking a session in the list opens its detail view
- Empty sessions show empty-state message
- Session metadata (model, provider, base URL, working dir, phase) displayed
- Status bar shows operation feedback
- App persists sessions to SQLite (~/.openwand/openwand.db)

## UI Architecture

```
UiSessionService (bridge)
  → SessionRegistryStore (store trait)
  → SqliteStore (SQLite impl)
  → session_registry table (navigation metadata)

Dioxus Components:
  App → Sidebar (session list) + DetailPane (session view)
  GlobalSignals for shared state (SESSION_LIST, SELECTED_SESSION_ID, CURRENT_SESSION)
```

## Runtime Integration

- `tokio::task::block_in_place` + `Handle::current().block_on()` for store init
  inside Dioxus/tokio runtime (avoids "Cannot start runtime from within runtime")
- `spawn(async move { ... })` for click handlers that call the service

## Accepted Limitations

- Messages always empty (no trace replay yet — deferred to 02b-4)
- No live run view (deferred to 02b-3)
- No session deletion or rename (simple scope)

## Files Changed

- `crates/app/Cargo.toml` — Added dirs, chrono, thiserror, lib target
- `crates/app/src/lib.rs` — NEW: lib root exposing ui module
- `crates/app/src/ui/mod.rs` — NEW: UI module
- `crates/app/src/ui/dto.rs` — NEW: UiSessionSummary, UiSessionView, UiMessage DTOs
- `crates/app/src/ui/service.rs` — NEW: UiSessionService wrapping store
- `crates/app/src/ui_main.rs` — REPLACED: Full static shell with sidebar + detail
- `crates/app/tests/ui_session_service.rs` — NEW: 6 service acceptance tests
- `crates/app/tests/smoke_wiring.rs` — Added desktop_feature_does_not_affect_cli_build

## Tests: 211 total, 0 failures

- +6 UI service acceptance tests
- +1 desktop feature guard test
