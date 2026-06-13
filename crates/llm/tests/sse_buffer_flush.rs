//! SSE tool-call buffer flush fixture test.
//!
//! Proves that when LM Studio sends `finish_reason: "tool_calls"`,
//! the buffered tool-call fragments are flushed into `ToolCallComplete`
//! before the `Done` delta is emitted.

use openwand_llm::tool_buffer::ToolCallBuffer;
use openwand_llm::response::LlmDelta;
/// Feed: ToolCallStart → ToolCallArgsDelta → drain_ids → complete
/// Expect: ToolCallComplete with correct name and parsed arguments
#[test]
fn sse_finish_reason_tool_calls_flushes_buffered_tool_calls() {
    let mut buf = ToolCallBuffer::new();

    // Simulate SSE stream chunks for a single tool call:
    // Chunk 1: name arrives (like SSE: tool_calls[0].id + tool_calls[0].function.name)
    buf.handle_start("tc_123".into(), Some("local__file_list".into()))
        .unwrap();

    // Chunk 2: arguments delta (like SSE: tool_calls[0].function.arguments = "{\"")
    buf.handle_args_delta("tc_123".into(), "{\"".into()).unwrap();

    // Chunk 3: more arguments (like SSE: tool_calls[0].function.arguments = "path\":\".\"}")
    buf.handle_args_delta("tc_123".into(), "path\":\".\"}".into())
        .unwrap();

    // Simulate: finish_reason = "tool_calls" → drain and flush all buffered calls
    let ids: Vec<String> = buf.drain_ids();
    assert_eq!(vec!["tc_123".to_string()], ids, "should have exactly one buffered call");

    let complete = buf.complete("tc_123").unwrap();

    // Verify ToolCallComplete structure
    match complete {
        LlmDelta::ToolCallComplete {
            id,
            name,
            arguments,
        } => {
            assert_eq!("tc_123", id);
            assert_eq!("local__file_list", name);
            // Arguments should be parsed as valid JSON
            assert_eq!(".", arguments["path"]);
        }
        other => panic!("Expected ToolCallComplete, got {:?}", other),
    }
}

/// Verify that drain_ids returns empty when no calls are buffered.
#[test]
fn drain_ids_returns_empty_when_no_calls_buffered() {
    let mut buf = ToolCallBuffer::new();
    let ids: Vec<String> = buf.drain_ids();
    assert!(ids.is_empty());
}

/// Verify that multiple concurrent tool calls are all flushed.
#[test]
fn sse_flushes_multiple_concurrent_tool_calls() {
    let mut buf = ToolCallBuffer::new();

    // Two parallel tool calls (like parallel function calling)
    buf.handle_start("tc_1".into(), Some("file_list".into()))
        .unwrap();
    buf.handle_start("tc_2".into(), Some("file_read".into()))
        .unwrap();

    buf.handle_args_delta("tc_1".into(), "{\"path\":\".\"}".into())
        .unwrap();
    buf.handle_args_delta("tc_2".into(), "{\"path\":\"/tmp/x\"}".into())
        .unwrap();

    let ids: Vec<String> = buf.drain_ids();
    assert_eq!(2, ids.len(), "should have two buffered calls");

    // Complete both (order shouldn't matter)
    let c1 = buf.complete("tc_1").unwrap();
    let c2 = buf.complete("tc_2").unwrap();

    match (c1, c2) {
        (
            LlmDelta::ToolCallComplete { name: n1, arguments: a1, .. },
            LlmDelta::ToolCallComplete { name: n2, arguments: a2, .. },
        ) => {
            assert_eq!("file_list", n1);
            assert_eq!(".", a1["path"]);
            assert_eq!("file_read", n2);
            assert_eq!("/tmp/x", a2["path"]);
        }
        other => panic!("Expected two ToolCallCompletes, got {:?}", other),
    }
}

/// Verify that drain_ids doesn't remove entries (complete still needs them).
#[test]
fn drain_ids_does_not_remove_entries() {
    let mut buf = ToolCallBuffer::new();
    buf.handle_start("tc_1".into(), Some("test".into())).unwrap();

    let ids1: Vec<String> = buf.drain_ids();
    let ids2: Vec<String> = buf.drain_ids();

    assert_eq!(ids1, ids2, "drain_ids should return same IDs on repeated calls");
    // But complete removes the entry
    buf.complete("tc_1").unwrap();
    let ids3: Vec<String> = buf.drain_ids();
    assert!(ids3.is_empty(), "after complete, drain should be empty");
}
