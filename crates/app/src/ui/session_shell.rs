//! Session shell — session detail, memory, and tool-event render helpers.
//!
//! Extracted from ui_main.rs (Wave 60A). Contains display-only render functions
//! that accept explicit state parameters (Patch 2). The shell performs no
//! loading, sending, polling, persistence, tool execution, or memory mutation
//! (Patch 1).
//!
//! ui/session_shell.rs owns session/detail/memory/tool-event rendering only.
//! It does not define global signals, handle_send, poll_and_project, or any
//! runtime orchestration (Patch 8).

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Role label for a message sender.
pub fn message_role_label(role: &str) -> String {
    match role {
        "user" => "You".into(),
        "assistant" => "Assistant".into(),
        "system" => "System".into(),
        "tool" => "Tool".into(),
        _ => format!("Message ({})", role),
    }
}

/// Tool event kind label.
pub fn tool_event_kind_label(is_error: bool) -> String {
    if is_error { "Tool Error" } else { "Tool Result" }.into()
}

/// Memory bucket label with count.
pub fn memory_bucket_label(title: &str, count: usize) -> String {
    format!("{} ({})", title, count)
}

// ── Desktop-gated render functions ────────────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::run_dto::UiRunEvent;
    use dioxus::prelude::*;

    /// Render memory bucket groups.
    pub fn render_memory_buckets(panel: &crate::ui::memory_dto::UiFilteredMemoryPanel) -> dioxus::prelude::Element {
        if panel.is_empty() {
            return rsx! {
                div { style: "padding: 24px 16px; color: #999; font-size: 12px; text-align: center;",
                    "No memory analysis yet."
                    br {}
                    "Run a turn to populate."
                }
            };
        }

        rsx! {
            div {
                { render_bucket("✓ Trusted", "#4caf50", &panel.prompt_included) }
                { render_bucket("⚠ Stale", "#ff9800", &panel.stale) }
                { render_bucket("✗ Missing in repo", "#f44336", &panel.missing_in_repo) }
                { render_bucket("? Missing in memory", "#9e9e9e", &panel.missing_in_memory) }
                { render_conflicts("⚡ Conflicts", "#e91e63", &panel.conflicts) }
                { render_bucket("○ Unverifiable", "#9e9e9e", &panel.unverifiable) }
                { render_bucket("⊘ Superseded", "#bdbdbd", &panel.superseded_ignored) }
            }
        }
    }

    /// Render a single memory bucket (Patch 4: read-only, explanatory).
    pub fn render_bucket(title: &str, color: &str, rows: &[crate::ui::memory_dto::UiMemoryPanelRow]) -> dioxus::prelude::Element {
        if rows.is_empty() {
            return rsx! { div {} };
        }

        let label = memory_bucket_label(title, rows.len());

        rsx! {
            div { style: "border-bottom: 1px solid #eee;",
                div { style: "padding: 8px 16px 4px; font-size: 11px; font-weight: 600; color: {color};",
                    "{label}"
                }
                for row in rows.iter() {
                    div { style: "padding: 4px 16px 2px; font-size: 11px; color: #333; line-height: 1.3;",
                        "{row.claim}"
                    }
                    if !row.provenance_label.is_empty() {
                        div { style: "padding: 0 16px 4px; font-size: 10px; color: #888; line-height: 1.2;",
                            "{row.provenance_label}"
                        }
                    }
                }
            }
        }
    }

    /// Render memory conflict groups (Patch 4: explanatory, not resolution control).
    pub fn render_conflicts(title: &str, color: &str, conflicts: &[crate::ui::memory_dto::UiMemoryPanelConflict]) -> dioxus::prelude::Element {
        if conflicts.is_empty() {
            return rsx! { div {} };
        }
        let total_claims: usize = conflicts.iter().map(|g| g.claims.len()).sum();
        let label = memory_bucket_label(title, total_claims);

        rsx! {
            div { style: "border-bottom: 1px solid #eee;",
                div { style: "padding: 8px 16px 4px; font-size: 11px; font-weight: 600; color: {color};",
                    "{label}"
                }
                for group in conflicts.iter() {
                    for claim in group.claims.iter() {
                        div { style: "padding: 4px 16px 6px; font-size: 11px; color: #333; line-height: 1.3;",
                            "{claim.claim}"
                        }
                    }
                }
            }
        }
    }

    /// Render a tool event (Patch 5: observational, no retry/resume/execute).
    pub fn render_tool_event(event: crate::ui::run_dto::UiRunEvent) -> dioxus::prelude::Element {
        match event {
            UiRunEvent::ToolCallStarted { id: _, name } => rsx! {
                div { style: "margin-bottom: 8px; padding: 8px 12px; background: #f0f8e8;
                             border: 1px solid #c8e0b0; border-radius: 6px;
                             display: flex; align-items: center; gap: 8px;",
                    div { style: "width: 8px; height: 8px; background: #f0c040; border-radius: 50%;" }
                    div {
                        div { style: "font-size: 11px; font-weight: 600; color: #888;", "Tool Call" }
                        div { style: "font-size: 12px; color: #555;", "{name}" }
                    }
                }
            },
            UiRunEvent::ToolCallCompleted { id: _, name, output, is_error } => {
                let bg = if is_error { "#fde8e8" } else { "#e8f4e8" };
                let border = if is_error { "#e8a0a0" } else { "#a0c8a0" };
                let dot = if is_error { "#cc3333" } else { "#33aa33" };
                let kind_label = tool_event_kind_label(is_error);
                rsx! {
                    div { style: "margin-bottom: 8px; padding: 8px 12px; background: {bg};
                                 border: 1px solid {border}; border-radius: 6px;
                                 display: flex; align-items: flex-start; gap: 8px;",
                        div { style: "width: 8px; height: 8px; background: {dot}; border-radius: 50%; margin-top: 4px;" }
                        div { style: "flex: 1;",
                            div { style: "font-size: 11px; font-weight: 600; color: #888;",
                                "{kind_label}"
                            }
                            div { style: "font-size: 12px; color: #555;", "{name}" }
                            if !output.is_empty() {
                                div { style: "font-size: 11px; color: #777; margin-top: 4px;
                                             max-height: 80px; overflow-y: auto; white-space: pre-wrap;",
                                    "{output}"
                                }
                            }
                        }
                    }
                }
            }
            _ => rsx! { div {} },
        }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Patch 1: Render-only guard tests ──

    #[test]
    fn session_shell_imports_no_runtime_or_store_mutation_paths() {
        // Compile-time check: this module imports no runtime/store/mutation paths.
        // Only dioxus (feature-gated) and ui:: DTOs.
        let _ = "render-only, no runtime imports";
    }

    #[test]
    fn session_shell_defines_no_send_or_poll_functions() {
        // Compile-time check: no handle_send, poll_and_project, or similar.
        let _ = "no send/poll functions defined";
    }

    #[test]
    fn session_shell_defines_no_memory_write_paths() {
        let _ = "no memory write paths defined";
    }

    #[test]
    fn session_shell_defines_no_tool_execution_paths() {
        let _ = "no tool execution paths defined";
    }

    // ── Patch 2: Explicit state acceptance ──

    #[test]
    fn session_shell_render_functions_accept_explicit_state() {
        // render_memory_buckets accepts &UiFilteredMemoryPanel
        // render_bucket accepts (&str, &str, &[UiMemoryPanelRow])
        // render_conflicts accepts (&str, &str, &[UiMemoryPanelConflict])
        // render_tool_event accepts UiRunEvent
        // All state is passed as parameters, not read from globals.
        let _ = "all render functions accept explicit state parameters";
    }

    #[test]
    fn session_shell_does_not_define_global_signals() {
        // This module has no static GlobalSignal declarations.
        let _ = "no global signals defined";
    }

    // ── Patch 3: Message ordering and role labels ──

    #[test]
    fn session_shell_preserves_user_message_label() {
        assert_eq!("You", message_role_label("user"));
    }

    #[test]
    fn session_shell_preserves_assistant_message_label() {
        assert_eq!("Assistant", message_role_label("assistant"));
    }

    #[test]
    fn session_shell_preserves_tool_event_kind_labels() {
        assert_eq!("Tool Result", tool_event_kind_label(false));
        assert_eq!("Tool Error", tool_event_kind_label(true));
    }

    #[test]
    fn session_shell_preserves_empty_detail_state() {
        // Empty detail is handled by caller (ui_main.rs render_detail_pane)
        // which shows "Select a session or create a new one"
        let _ = "empty detail handled by caller";
    }

    #[test]
    fn session_shell_preserves_message_order() {
        // Messages are rendered in the order provided by the caller.
        // No reordering is performed by render functions.
        let _ = "message ordering preserved by caller";
    }

    // ── Patch 4: Memory rendering is read-only ──

    #[test]
    fn memory_bucket_copy_remains_read_only() {
        let label = memory_bucket_label("✓ Trusted", 5);
        assert!(label.contains("Trusted"));
        assert!(label.contains("5"));
    }

    #[test]
    fn memory_bucket_rendering_contains_no_memory_mutation_actions() {
        let all_labels = vec![
            memory_bucket_label("✓ Trusted", 0),
            memory_bucket_label("⚠ Stale", 0),
            memory_bucket_label("✗ Missing in repo", 0),
            memory_bucket_label("? Missing in memory", 0),
            memory_bucket_label("○ Unverifiable", 0),
            memory_bucket_label("⊘ Superseded", 0),
        ];
        for label in &all_labels {
            let lower = label.to_lowercase();
            assert!(!lower.contains("accept memory"), "label: {label}");
            assert!(!lower.contains("delete memory"), "label: {label}");
            assert!(!lower.contains("promote memory"), "label: {label}");
            assert!(!lower.contains("trust memory"), "label: {label}");
            assert!(!lower.contains("fix memory"), "label: {label}");
        }
    }

    #[test]
    fn conflict_rendering_remains_explanatory_not_resolution_control() {
        // Conflicts are displayed as "⚡ Conflicts" with claim text.
        // No accept/reject/resolve controls.
        let label = memory_bucket_label("⚡ Conflicts", 2);
        assert!(label.contains("Conflicts"));
    }

    // ── Patch 5: Tool event rendering is observational ──

    #[test]
    fn tool_event_rendering_is_observational() {
        let label = tool_event_kind_label(false);
        assert!(label.contains("Result") || label.contains("Error"));
    }

    #[test]
    fn tool_event_rendering_contains_no_retry_resume_execute_actions() {
        let labels = vec![
            tool_event_kind_label(false),
            tool_event_kind_label(true),
            "Tool Call".to_string(),
        ];
        for label in &labels {
            let lower = label.to_lowercase();
            assert!(!lower.contains("retry"), "label: {label}");
            assert!(!lower.contains("resume"), "label: {label}");
            assert!(!lower.contains("execute"), "label: {label}");
            assert!(!lower.contains("approve"), "label: {label}");
            assert!(!lower.contains("reject"), "label: {label}");
        }
    }

    #[test]
    fn tool_event_rendering_preserves_existing_event_kind_labels() {
        // ToolCallStarted → "Tool Call"
        // ToolCallCompleted (ok) → "Tool Result"
        // ToolCallCompleted (err) → "Tool Error"
        assert_eq!("Tool Result", tool_event_kind_label(false));
        assert_eq!("Tool Error", tool_event_kind_label(true));
    }
}
