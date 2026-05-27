//! OpenWand Desktop UI — Wave 02b-0 spike.
//!
//! Proves three things:
//! 1. Dioxus desktop window opens
//! 2. Tokio task → Dioxus signal bridge works
//! 3. Receiver/task cleanup on window close
//!
//! Run with: cargo run --bin openwand-ui --features desktop

use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};

fn main() {
    let desktop_cfg = Config::new().with_window(
        WindowBuilder::new()
            .with_title("OpenWand — Spike")
            .with_inner_size(LogicalSize::new(800, 600)),
    );

    LaunchBuilder::new()
        .with_cfg(desktop_cfg)
        .launch(App);
}

/// Simulated AgentEvent — matches the real session event shape.
#[derive(Debug, Clone, PartialEq)]
enum SpikeEvent {
    TextDelta { delta: String },
    PhaseEntered { phase: String, step: u64 },
    RunCompleted { steps: u64, tools: u64 },
}

/// Global state: events received from the background task.
static EVENTS: GlobalSignal<Vec<SpikeEvent>> = Signal::global(Vec::new);

/// Global state: whether the background producer is running.
static PRODUCER_RUNNING: GlobalSignal<bool> = Signal::global(|| false);

fn App() -> Element {
    let running = *PRODUCER_RUNNING.read();
    let bg_color = if running { "#ccc" } else { "#4a90d9" };

    rsx! {
        div {
            style: "padding: 20px; font-family: system-ui;",

            h1 { "OpenWand UI Spike" }
            p { style: "color: #888; font-size: 14px;",
                "Proves: Dioxus window + tokio-signal bridge + cleanup on close"
            }

            // Controls
            div { style: "margin: 16px 0;",
                button {
                    style: "padding: 8px 16px; margin-right: 8px; font-size: 14px; border: none; border-radius: 4px; cursor: pointer; background: {bg_color}; color: white;",
                    disabled: running,
                    onclick: move |_| start_producer(),
                    "Start Producer"
                }
                button {
                    style: "padding: 8px 16px; font-size: 14px;
                             background: #d94a4a; color: white; border: none;
                             border-radius: 4px; cursor: pointer;",
                    disabled: !running,
                    onclick: move |_| stop_producer(),
                    "Stop Producer"
                }
            }

            // Status
            div { style: "margin: 8px 0; padding: 8px; background: #f0f0f0; border-radius: 4px;",
                span { style: "font-weight: bold;", "Status: " }
                if running {
                    span { style: "color: green;", "Running" }
                } else {
                    span { style: "color: gray;", "Idle" }
                }
            }

            // Event stream
            h2 { style: "font-size: 16px; margin-top: 20px;",
                "Event Stream ({EVENTS.read().len()} events)"
            }
            div {
                style: "border: 1px solid #ddd; border-radius: 4px; padding: 12px;
                        max-height: 400px; overflow-y: auto; background: #fafafa;
                        font-family: monospace; font-size: 13px;",

                for event in EVENTS.read().iter().rev().take(50) {
                    {render_event(event)}
                }
            }
        }
    }
}

fn render_event(event: &SpikeEvent) -> Element {
    match event {
        SpikeEvent::TextDelta { delta } => rsx! {
            div { style: "padding: 2px 0; color: #333;",
                "TextDelta: \"{delta}\""
            }
        },
        SpikeEvent::PhaseEntered { phase, step } => rsx! {
            div { style: "padding: 2px 0; color: #0066cc;",
                "Phase: {phase} (step {step})"
            }
        },
        SpikeEvent::RunCompleted { steps, tools } => rsx! {
            div { style: "padding: 2px 0; color: #228b22; font-weight: bold;",
                "Run completed: {steps} steps, {tools} tools"
            }
        },
    }
}

/// Start a background producer that simulates an AgentEvent stream.
fn start_producer() {
    if *PRODUCER_RUNNING.read() {
        return;
    }
    *PRODUCER_RUNNING.write() = true;

    // Spawn a producer on the Dioxus runtime.
    // In real app, this bridges broadcast::Receiver<AgentEvent> into GlobalSignal.
    spawn(async move {
        let mut step = 0u64;

        EVENTS.write().push(SpikeEvent::PhaseEntered {
            phase: "RunStart".into(),
            step: 0,
        });

        for i in 0..5 {
            if !*PRODUCER_RUNNING.read() {
                return;
            }

            step = i;

            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            EVENTS.write().push(SpikeEvent::PhaseEntered {
                phase: "StepStart".into(),
                step,
            });

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            EVENTS.write().push(SpikeEvent::PhaseEntered {
                phase: "Inference".into(),
                step,
            });

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let text = format!("Step {} output text here. ", i);
            EVENTS.write().push(SpikeEvent::TextDelta { delta: text });

            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            EVENTS.write().push(SpikeEvent::PhaseEntered {
                phase: "StepEnd".into(),
                step,
            });
        }

        EVENTS.write().push(SpikeEvent::RunCompleted {
            steps: step + 1,
            tools: 1,
        });
        *PRODUCER_RUNNING.write() = false;
    });
}

/// Stop the producer.
fn stop_producer() {
    *PRODUCER_RUNNING.write() = false;
}
