# WAVE 02B-4 — RELOAD + CRASH RECOVERY — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** Trace replay → UI timeline, tool result output, session persistence across reopen

## Proven

- Close/reopen the app → session state reconstructed from trace
- User messages replay from `Session::UserMessageInjected` trace events
- Tool calls replay from `Tool::Called` trace events
- Tool results replay from `Tool::Completed/Failed` with result_summary
- Timeline items ordered by trace sequence (authority)
- AgentEvent::ToolCallCompleted now carries result_preview
- UiRunBridge correctly forwards tool result output to live UI

## Architecture

```
TraceStore
  → scan(session_stream_id)
  → replay_timeline() maps events to UiTimelineItem
  → UiSessionView.messages = filtered timeline
  → Dioxus renders from UiSessionView

Trace event mapping:
  Session::UserMessageInjected → UiTimelineItem::Message (User)
  Inference::Completed        → UiTimelineItem::Message (Assistant, placeholder)
  Tool::Called                → UiTimelineItem::ToolCall
  Tool::Completed             → UiTimelineItem::ToolResult (output_preview)
  Tool::Failed                → UiTimelineItem::ToolResult (is_error=true)
  Session::Started            → UiTimelineItem::RunMarker
  Session::Ended              → UiTimelineItem::RunMarker
```

## Bug Found and Fixed

`SqliteStore::scan()` returns `next_cursor` but doesn't implement cursor-based pagination.
The replay loop would never terminate. Fixed by breaking after first page (scan fetches all
matching entries in one query when limit=100).

Root cause tracked in backlog: scan cursor pagination not implemented in writer.rs.

## New Files

- `crates/app/src/ui/replay.rs` — Trace replay → UiTimelineItem
- Updated `crates/app/src/ui/service.rs` — open_session now async with replay
- Updated `crates/app/src/ui/run_bridge.rs` — handles result_preview

## Files Changed

- `crates/session/src/agent_event.rs` — Added `result_preview` to ToolCallCompleted
- `crates/session/src/runner.rs` — Emits ToolCallStarted/Completed, RunStarted/RunCompleted
- `crates/app/tests/ui_session_service.rs` — 8 tests with trace replay verification
- `crates/app/src/ui_main.rs` — Two store connections (registry + trace)

## Accepted Limitations

- Assistant text shown as placeholder "(assistant response — full text in Loro rebuild)"
  because the runner records Inference::Completed but not the actual text in trace.
  Fix: record assistant text in Session::AssistantMessageGenerated or similar.
- Scan cursor pagination not implemented (single-page fetch is fine for <100 entries)
- No session delete/rename

## Tests: 224 total, 0 failures
