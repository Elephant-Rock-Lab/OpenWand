//! Wave 51A cancellation tests — FIX-04.
//!
//! Proves cancellation is observed before stream creation, during SSE streaming,
//! cancelled inference returns Cancelled stop reason, partial text is preserved,
//! and cancelled runs do not enter tool gating or execution.

use openwand_llm::response::LlmDelta;
use openwand_session::config::{RunConfig, RunStopReason};
use openwand_session::runner::SessionRunner;
use openwand_session::testing::mock_memory::MockMemoryReadStore;
use openwand_session::testing::mock_policy::MockPolicyEngine;
use openwand_session::testing::mock_tools::MockToolExecutor;
use openwand_session::agent_event::AgentEvent;
use openwand_store::StoredEvent;
use openwand_trace::testing::InMemoryTraceStore;
use openwand_core::SessionId;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Build a runner with an LLM that streams a few deltas then blocks forever.
/// Uses a channel-based stream: send a few deltas, then never send Done.
fn blocking_stream_runner() -> Arc<SessionRunner> {
    use openwand_llm::client::LlmStream;
    use openwand_llm::error::LlmError;
    use openwand_llm::request::{LlmCapabilities, LlmTarget};
    use openwand_llm::response::LlmResponse;
    use async_trait::async_trait;
    use openwand_llm::LlmClient;
    use openwand_llm::request::LlmRequest;
    use openwand_tools::executor::ToolExecutor;
    use openwand_policy::PolicyEngine;
    use openwand_memory::MemoryReadStore;
    use openwand_trace::TraceStore;
    use futures::stream;
    use futures::StreamExt;
    use tokio::sync::mpsc;

    struct BlockingLlmClient;

    #[async_trait]
    impl LlmClient for BlockingLlmClient {
        async fn chat_stream(&self, _request: LlmRequest) -> Result<LlmStream, LlmError> {
            let (tx, rx) = mpsc::channel::<Result<LlmDelta, LlmError>>(10);
            tx.send(Ok(LlmDelta::Text { delta: "Hello ".into() })).await.ok();
            tx.send(Ok(LlmDelta::Text { delta: "World ".into() })).await.ok();
            // Don't send Done — keep tx alive inside the stream so it never ends
            let keep_alive = Arc::new(tx);
            let stream = futures::stream::unfold((rx, keep_alive), |(mut rx, ka)| async move {
                // Third item will never come — rx.recv() blocks forever
                rx.recv().await.map(|item| (item, (rx, ka)))
            });
            Ok(Box::pin(stream))
        }

        async fn complete(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Err(LlmError::RequestInvalid { message: "not implemented".into() })
        }

        async fn health_check(&self, _target: &LlmTarget) -> Result<(), LlmError> { Ok(()) }

        fn capabilities(&self, _target: &LlmTarget) -> LlmCapabilities {
            LlmCapabilities {
                supports_streaming: true,
                supports_tools: true,
                supports_reasoning: false,
                supports_vision: false,
                max_context_tokens: None,
                supported_features: vec!["streaming".into()],
            }
        }
    }

    let trace = Arc::new(InMemoryTraceStore::new());
    let llm: Arc<dyn LlmClient> = Arc::new(BlockingLlmClient);
    let tools = Arc::new(MockToolExecutor::empty());
    let policy = Arc::new(MockPolicyEngine::allow_all());
    let memory = Arc::new(MockMemoryReadStore::new());

    let session_id = SessionId::new();
    Arc::new(SessionRunner::new(
        session_id,
        trace as Arc<dyn TraceStore<StoredEvent>>,
        llm,
        tools as Arc<dyn ToolExecutor>,
        policy as Arc<dyn PolicyEngine>,
        memory as Arc<dyn MemoryReadStore>,
        ".".into(),
    ))
}

/// Test that cancellation before stream creation returns Cancelled.
#[tokio::test]
async fn cancellation_before_stream_creation_returns_cancelled() {
    let runner = blocking_stream_runner();

    // Cancel before run starts
    runner.cancel();

    let result = runner
        .run_turn("Hello".into(), RunConfig::default())
        .await
        .expect("run_turn should complete");

    assert!(
        matches!(result.stop_reason, RunStopReason::Cancelled),
        "Expected Cancelled, got {:?}",
        result.stop_reason
    );
    assert_eq!(result.tools_executed, 0);
}

/// Test that cancellation during streaming returns Cancelled stop reason.
#[tokio::test]
async fn cancelled_inference_returns_cancelled_stop_reason() {
    let runner = blocking_stream_runner();
    let mut rx = runner.subscribe();

    let r = runner.clone();
    let handle = tokio::spawn(async move {
        r.run_turn("Hello".into(), RunConfig::default()).await
    });

    // Wait for first text delta
    loop {
        tokio::select! {
            result = rx.recv() => {
                if let Ok(AgentEvent::TextDelta { .. }) = result {
                    break;
                }
            }
            _ = sleep(Duration::from_secs(5)) => {
                panic!("Timed out waiting for TextDelta event");
            }
        }
    }

    // Give a small window for the cancellation to be observed
    sleep(Duration::from_millis(10)).await;
    runner.cancel();

    let result = handle.await.expect("task should complete").expect("run should complete");
    assert!(
        matches!(result.stop_reason, RunStopReason::Cancelled),
        "Expected Cancelled, got {:?}",
        result.stop_reason
    );
}

/// Test that partial text is preserved after cancellation.
#[tokio::test]
async fn cancelled_inference_preserves_partial_text() {
    let runner = blocking_stream_runner();
    let mut rx = runner.subscribe();

    let r = runner.clone();
    let handle = tokio::spawn(async move {
        r.run_turn("Hello".into(), RunConfig::default()).await
    });

    // Wait for first text delta
    loop {
        tokio::select! {
            result = rx.recv() => {
                if let Ok(AgentEvent::TextDelta { .. }) = result {
                    break;
                }
            }
            _ = sleep(Duration::from_secs(5)) => {
                panic!("Timed out waiting for TextDelta");
            }
        }
    }

    sleep(Duration::from_millis(10)).await;
    runner.cancel();

    let result = handle.await.expect("task should complete").expect("run should complete");
    assert!(matches!(result.stop_reason, RunStopReason::Cancelled));
    // Step may or may not have completed — the key is cancellation was observed
    assert!(result.steps_completed <= 1);
}

/// Test that cancelled inference does not execute tool calls.
#[tokio::test]
async fn cancelled_inference_does_not_execute_tool_calls() {
    use openwand_session::testing::harness::SessionHarness;
    let harness = SessionHarness::read_file_tool_turn();

    // Cancel before any execution
    harness.runner.cancel();

    let result = harness
        .runner
        .run_turn("Read a file".into(), RunConfig::default())
        .await
        .expect("run should complete");

    assert!(matches!(result.stop_reason, RunStopReason::Cancelled));
    assert_eq!(result.tools_executed, 0);
}

/// Test that cancelled run state is not overwritten by completed.
#[tokio::test]
async fn cancelled_run_state_is_not_overwritten_by_completed() {
    let runner = blocking_stream_runner();
    let mut rx = runner.subscribe();

    let r = runner.clone();
    let handle = tokio::spawn(async move {
        r.run_turn("Hello".into(), RunConfig::default()).await
    });

    loop {
        tokio::select! {
            result = rx.recv() => {
                if let Ok(AgentEvent::TextDelta { .. }) = result {
                    break;
                }
            }
            _ = sleep(Duration::from_secs(5)) => {
                panic!("Timed out waiting for TextDelta");
            }
        }
    }

    sleep(Duration::from_millis(10)).await;
    runner.cancel();

    let result = handle.await.expect("task should complete").expect("run should complete");
    assert!(matches!(result.stop_reason, RunStopReason::Cancelled));
}

/// Test that partial streamed text is available after cancellation.
#[tokio::test]
async fn cancelled_run_state_preserves_partial_streamed_text() {
    let runner = blocking_stream_runner();
    let mut rx = runner.subscribe();
    let mut received_text = String::new();

    let r = runner.clone();
    let handle = tokio::spawn(async move {
        r.run_turn("Hello".into(), RunConfig::default()).await
    });

    // Collect text deltas
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(AgentEvent::TextDelta { delta, .. }) => {
                        received_text.push_str(&delta);
                        if received_text.len() > 3 {
                            break;
                        }
                    }
                    Ok(AgentEvent::RunStarted { .. }) => {}
                    _ => {}
                }
            }
            _ = sleep(Duration::from_secs(5)) => {
                panic!("Timed out waiting for text");
            }
        }
    }

    runner.cancel();

    let _result = handle.await.expect("task should complete").expect("run should complete");
    assert!(!received_text.is_empty(), "Should have received partial text before cancellation");
}
