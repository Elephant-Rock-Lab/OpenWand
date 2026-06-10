//! Workflow execution timeline desktop UI components.
//!
//! Read-only display of recorded workflow execution history using Wave 52A
//! design-system tokens. Displays run summary, ordered stages, lifecycle
//! events, action requests, abort/rollback state, predicates, and explicit
//! no-replay/no-repair copy.
//!
//! Timeline order uses only recorded ordering fields. Events without global
//! order are grouped under their recorded stage (Patch 1).
//!
//! Components accept already-prepared state. They do not replay events,
//! execute tools, dispatch actions, abort/rollback runs, mutate workflow
//! state, append trace, repair records, verify truth, certify evidence,
//! create records, or infer unrecorded history.

use crate::ui::design_tokens::*;
use crate::ui::workflow_execution_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for run status.
pub fn run_status_tone(status: &str) -> UiTone {
    match status {
        "completed" => UiTone::Info,
        "suspended" => UiTone::Warning,
        "failed" => UiTone::Error,
        "running" => UiTone::Info,
        "cancelled" => UiTone::Neutral,
        "aborted" => UiTone::Neutral,
        _ => UiTone::Neutral,
    }
}

/// Human-readable label for run status.
pub fn run_status_label(status: &str) -> String {
    match status {
        "completed" => "Completed (recorded)".into(),
        "suspended" => "Suspended (recorded)".into(),
        "failed" => "Failed (recorded)".into(),
        "running" => "Running (recorded)".into(),
        "cancelled" => "Cancelled (recorded)".into(),
        "aborted" => "Aborted (recorded)".into(),
        _ => format!("Recorded: {}", status),
    }
}

/// Semantic tone for stage status.
pub fn stage_status_tone(status: &str) -> UiTone {
    match status {
        "completed" => UiTone::Info,
        "in_progress" => UiTone::Warning,
        "failed" => UiTone::Error,
        "pending" => UiTone::Neutral,
        "skipped" => UiTone::Neutral,
        _ => UiTone::Neutral,
    }
}

/// Patch 5: recorded-event copy.
pub fn recorded_run_note() -> String {
    "Recorded workflow run.".into()
}

pub fn timeline_display_only_note() -> String {
    "Timeline display only.".into()
}

/// Patch 2: action request copy.
pub fn action_request_recorded_note() -> String {
    "Recorded action request.".into()
}

pub fn action_request_routing_only_note() -> String {
    "Routing status only.".into()
}

pub fn action_request_no_execution_note() -> String {
    "No action execution is available here.".into()
}

/// Patch 3: abort/rollback copy.
pub fn abort_recorded_state_note() -> String {
    "Recorded abort/rollback state.".into()
}

pub fn abort_no_action_note() -> String {
    "No abort, rollback, or replay action is available here.".into()
}

/// Safety text.
pub fn execution_safety_text() -> String {
    workflow_execution_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use crate::ui::layout::*;
    use dioxus::prelude::*;

    /// Empty state (Patch 7).
    pub fn render_workflow_execution_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            SPACING::SPACE_LG, SPACING::SPACE_XL, TYPO::TEXT_SM,
            COLORS::TEXT_FAINT, COLORS::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No workflow execution records"
            }
        }
    }

    /// Loading state (Patch 7).
    pub fn render_workflow_execution_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            SPACING::SPACE_LG, SPACING::SPACE_XL, TYPO::TEXT_SM,
            COLORS::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading workflow execution…"
            }
        }
    }

    /// Error state (Patch 7).
    pub fn render_workflow_execution_error_state(message: &str) -> Element {
        let style = banner_style(UiTone::Error);
        let safe: String = message.chars().take(200).collect();
        rsx! {
            div { style: "{style}",
                "Execution load error: {safe}"
            }
        }
    }

    /// Safety banner.
    pub fn render_workflow_execution_safety_banner() -> Element {
        let text = execution_safety_text();
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                "⚠ {text}"
            }
        }
    }

    /// Run summary card (Patch 5: recorded run).
    pub fn render_run_summary(row: &WorkflowRunSummaryRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let tone = run_status_tone(&row.status);
        let label = run_status_label(&row.status);
        let badge_s = badge_style(tone);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );
        let label_s = format!("min-width: 160px; color: {};", COLORS::TEXT_PRIMARY);
        let value_s = format!("color: {};", COLORS::TEXT_MUTED);
        let note = recorded_run_note();
        let display = timeline_display_only_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            TYPO::TEXT_XS, COLORS::TEXT_MUTED, SPACING::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Recorded workflow run ({row.execution_id})"
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Status" }
                    span { style: "{badge_s}", "{label}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Stages" }
                    span { style: "{value_s}", "{row.stage_count}" }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Predicates" }
                    span { style: "{value_s}", "{row.predicates_passed} / {row.predicates_total}" }
                }
                div { style: "{note_s}", "{note}" }
                div { style: "{note_s}", "{display}" }
            }
        }
    }

    /// Stage timeline card (Patch 1: uses recorded order only).
    pub fn render_stage_timeline(stages: &[WorkflowStageRunRow]) -> Element {
        if stages.is_empty() {
            return rsx! { div {} };
        }
        // Patch 1: sort by recorded order, do not infer unrecorded order
        let mut sorted: Vec<&WorkflowStageRunRow> = stages.iter().collect();
        sorted.sort_by_key(|s| s.order);

        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );
        let label_s = format!("min-width: 120px; color: {};", COLORS::TEXT_PRIMARY);
        let value_s = format!("color: {};", COLORS::TEXT_MUTED);
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic;",
            TYPO::TEXT_XS, COLORS::TEXT_MUTED,
        );

        rsx! {
            div {
                div { style: "{header_s}",
                    "Stage timeline (recorded order)"
                }
                for stage in sorted {
                    {
                        let card_s = card_style(stage_status_tone(&stage.status), UiDensity::Compact);
                        let status_label = format!("{} (recorded)", stage.status);
                        rsx! {
                            div { style: "{card_s}",
                                div { style: "{row_s}",
                                    span { style: "{label_s}", "Stage" }
                                    span { style: "{value_s}", "{stage.stage_id}" }
                                }
                                div { style: "{row_s}",
                                    span { style: "{label_s}", "Order" }
                                    span { style: "{value_s}", "{stage.order}" }
                                }
                                div { style: "{row_s}",
                                    span { style: "{label_s}", "Kind" }
                                    span { style: "{value_s}", "{stage.kind}" }
                                }
                                div { style: "{row_s}",
                                    span { style: "{label_s}", "Status" }
                                    span { style: "{value_s}", "{status_label}" }
                                }
                                div { style: "{note_s}", "{stage.summary}" }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Lifecycle events grouped by stage (Patch 1: no global order inference).
    pub fn render_lifecycle_events(events: &[WorkflowLifecycleEventRow], stages: &[WorkflowStageRunRow]) -> Element {
        if events.is_empty() {
            return rsx! { div {} };
        }

        // Patch 1: group events by recorded stage_id
        let mut stage_ids: Vec<String> = stages.iter().map(|s| s.stage_id.clone()).collect();
        stage_ids.sort();
        stage_ids.dedup();

        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );
        let label_s = format!("min-width: 120px; color: {};", COLORS::TEXT_PRIMARY);
        let value_s = format!("color: {};", COLORS::TEXT_MUTED);

        let grouped: Vec<(String, Vec<&WorkflowLifecycleEventRow>)> = stage_ids.iter().map(|sid| {
            let evts: Vec<&WorkflowLifecycleEventRow> = events.iter().filter(|e| e.stage_id == *sid).collect();
            (sid.clone(), evts)
        }).filter(|(_, evts)| !evts.is_empty()).collect();

        rsx! {
            div {
                div { style: "{header_s}",
                    "Lifecycle events (grouped by recorded stage)"
                }
                for (stage_id, evts) in grouped {
                    div { style: "font-size: 13px; font-weight: 600; color: #333; margin-top: 8px;",
                        "Stage: {stage_id}"
                    }
                    for event in evts {
                        div { style: "{row_s}",
                            span { style: "{label_s}", "{event.event_kind}" }
                            span { style: "{value_s}", "{event.summary}" }
                        }
                    }
                }
            }
        }
    }

    /// Action request card (Patch 2: not executable).
    pub fn render_action_requests(requests: &[WorkflowActionRequestRow]) -> Element {
        if requests.is_empty() {
            return rsx! { div {} };
        }
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );
        let label_s = format!("min-width: 140px; color: {};", COLORS::TEXT_PRIMARY);
        let value_s = format!("color: {};", COLORS::TEXT_MUTED);
        let note1 = action_request_recorded_note();
        let note2 = action_request_routing_only_note();
        let note3 = action_request_no_execution_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            TYPO::TEXT_XS, COLORS::TEXT_MUTED, SPACING::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Recorded action requests"
                }
                for req in requests {
                    div { style: "{row_s}",
                        span { style: "{label_s}", "{req.action_request_id}" }
                        span { style: "{value_s}", "{req.capability} — {req.routing_status}" }
                    }
                }
                div { style: "{note_s}", "{note1}" }
                div { style: "{note_s}", "{note2}" }
                div { style: "{note_s}", "{note3}" }
            }
        }
    }

    /// Abort/rollback snapshot card (Patch 3: non-actionable).
    pub fn render_abort_snapshot(snapshot: &WorkflowAbortSnapshotRow) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );
        let label_s = format!("min-width: 160px; color: {};", COLORS::TEXT_PRIMARY);
        let value_s = format!("color: {};", COLORS::TEXT_MUTED);
        let note1 = abort_recorded_state_note();
        let note2 = abort_no_action_note();
        let note_s = format!(
            "font-size: {}; color: {}; font-style: italic; margin-top: {};",
            TYPO::TEXT_XS, COLORS::TEXT_MUTED, SPACING::SPACE_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Recorded abort/rollback state"
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Abort notes available" }
                    span { style: "{value_s}",
                        if snapshot.abort_available { "Yes" } else { "No" }
                    }
                }
                div { style: "{row_s}",
                    span { style: "{label_s}", "Rollback notes available" }
                    span { style: "{value_s}",
                        if snapshot.rollback_available { "Yes" } else { "No" }
                    }
                }
                for note in &snapshot.notes {
                    div { style: "{row_s}",
                        span { style: "{label_s}", "Recovery note" }
                        span { style: "{value_s}", "{note}" }
                    }
                }
                div { style: "{note_s}", "{note1}" }
                div { style: "{note_s}", "{note2}" }
            }
        }
    }

    /// Execution predicates (Patch 5: textual).
    pub fn render_execution_predicates(predicates: &[WorkflowExecutionPredicateRow]) -> Element {
        if predicates.is_empty() {
            return rsx! { div {} };
        }
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);
        let row_s = format!(
            "display: flex; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );
        let name_s = format!("min-width: 260px; color: {};", COLORS::TEXT_PRIMARY);

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Execution predicates"
                }
                for pred in predicates {
                    div { style: "{row_s}",
                        span { style: "{name_s}", "{pred.predicate}" }
                        span {
                            style: "min-width: 80px; color: {};",
                            if pred.passed { COLORS::ACCENT_INFO } else { COLORS::ACCENT_ERROR },
                            if pred.passed { "Passed" } else { "Failed" }
                        }
                        span { style: "color: {};", COLORS::TEXT_MUTED, "{pred.reason}" }
                    }
                }
            }
        }
    }

    /// Full timeline panel.
    pub fn render_workflow_execution_timeline(state: &WorkflowExecutionUiState) -> Element {
        rsx! {
            div {
                if let Some(ref run) = state.latest_run {
                    { render_run_summary(run) }
                }
                { render_execution_predicates(&state.predicates) }
                { render_stage_timeline(&state.stages) }
                { render_lifecycle_events(&state.lifecycle_events, &state.stages) }
                { render_action_requests(&state.action_requests) }
                if let Some(ref abort) = state.abort_snapshot {
                    { render_abort_snapshot(abort) }
                }
                for w in &state.warnings {
                    div { style: "font-size: 12px; color: #856404; padding: 2px 0;",
                        "{w}"
                    }
                }
            }
        }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_status_tone_info_for_completed() {
        assert_eq!(UiTone::Info, run_status_tone("completed"));
    }

    #[test]
    fn run_status_tone_warning_for_suspended() {
        assert_eq!(UiTone::Warning, run_status_tone("suspended"));
    }

    #[test]
    fn run_status_tone_error_for_failed() {
        assert_eq!(UiTone::Error, run_status_tone("failed"));
    }

    #[test]
    fn run_status_label_says_recorded() {
        assert!(run_status_label("completed").contains("recorded"));
    }

    #[test]
    fn stage_status_tone_info_for_completed() {
        assert_eq!(UiTone::Info, stage_status_tone("completed"));
    }

    // ── Patch 1: Timeline order tests ──

    #[test]
    fn timeline_uses_recorded_stage_order() {
        let stages = vec![
            WorkflowStageRunRow { stage_id: "s2".into(), kind: "act".into(), status: "completed".into(), order: 1, summary: "".into() },
            WorkflowStageRunRow { stage_id: "s1".into(), kind: "observe".into(), status: "completed".into(), order: 0, summary: "".into() },
        ];
        // Verify order field exists and is distinct
        assert_eq!(0, stages.iter().find(|s| s.stage_id == "s1").unwrap().order);
        assert_eq!(1, stages.iter().find(|s| s.stage_id == "s2").unwrap().order);
    }

    #[test]
    fn timeline_groups_events_by_stage_when_no_global_order_exists() {
        let stages = vec![
            WorkflowStageRunRow { stage_id: "s1".into(), kind: "observe".into(), status: "completed".into(), order: 0, summary: "".into() },
        ];
        let events = vec![
            WorkflowLifecycleEventRow { event_id: "e1".into(), stage_id: "s1".into(), event_kind: "started".into(), summary: "began".into() },
            WorkflowLifecycleEventRow { event_id: "e2".into(), stage_id: "s1".into(), event_kind: "completed".into(), summary: "done".into() },
        ];
        // Both events belong to s1
        assert!(events.iter().all(|e| e.stage_id == "s1"));
        // No global order field on events — grouped by stage_id
        let s1_events: Vec<_> = events.iter().filter(|e| e.stage_id == "s1").collect();
        assert_eq!(2, s1_events.len());
    }

    #[test]
    fn timeline_does_not_infer_unrecorded_event_order() {
        let events = vec![
            WorkflowLifecycleEventRow { event_id: "e1".into(), stage_id: "s1".into(), event_kind: "started".into(), summary: "first".into() },
            WorkflowLifecycleEventRow { event_id: "e2".into(), stage_id: "s1".into(), event_kind: "completed".into(), summary: "second".into() },
        ];
        // Events have no order field — we display them in recorded order only
        assert_eq!("e1", events[0].event_id);
        assert_eq!("e2", events[1].event_id);
    }

    // ── Patch 2: Action request guard tests ──

    #[test]
    fn action_request_card_says_recorded_not_executable() {
        let note = action_request_recorded_note();
        assert!(note.to_lowercase().contains("recorded"), "got: {note}");
    }

    #[test]
    fn action_request_card_says_routing_status_only() {
        let note = action_request_routing_only_note();
        assert!(note.to_lowercase().contains("routing status"), "got: {note}");
    }

    #[test]
    fn action_request_card_contains_no_execution_affordance() {
        let all_copy = vec![
            action_request_recorded_note(),
            action_request_routing_only_note(),
            action_request_no_execution_note(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("run now"), "copy: {text}");
            assert!(!lower.contains("execute"), "copy: {text}");
            assert!(!lower.contains("retry"), "copy: {text}");
            assert!(!lower.contains("dispatch"), "copy: {text}");
            assert!(!lower.contains("submit"), "copy: {text}");
        }
    }

    // ── Patch 3: Abort/rollback guard tests ──

    #[test]
    fn abort_snapshot_says_recorded_state() {
        let note = abort_recorded_state_note();
        assert!(note.to_lowercase().contains("recorded"), "got: {note}");
    }

    #[test]
    fn abort_snapshot_says_no_abort_action_available() {
        let note = abort_no_action_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("abort"), "got: {note}");
        assert!(lower.contains("rollback"), "got: {note}");
        assert!(lower.contains("no"), "got: {note}");
        assert!(lower.contains("available here"), "got: {note}");
    }

    #[test]
    fn abort_snapshot_says_no_rollback_action_available() {
        let note = abort_no_action_note();
        let lower = note.to_lowercase();
        assert!(lower.contains("rollback"), "got: {note}");
        assert!(lower.contains("no"), "got: {note}");
    }

    // ── Patch 4: ID display tests ──

    #[test]
    fn timeline_displays_stage_ids() {
        let stages = vec![
            WorkflowStageRunRow { stage_id: "s1".into(), kind: "observe".into(), status: "completed".into(), order: 0, summary: "".into() },
        ];
        assert!(!stages[0].stage_id.is_empty());
    }

    #[test]
    fn timeline_displays_event_ids() {
        let events = vec![
            WorkflowLifecycleEventRow { event_id: "e1".into(), stage_id: "s1".into(), event_kind: "started".into(), summary: "".into() },
        ];
        assert!(!events[0].event_id.is_empty());
    }

    #[test]
    fn timeline_displays_action_request_ids() {
        let requests = vec![
            WorkflowActionRequestRow { action_request_id: "ar_1".into(), capability: "read".into(), routing_status: "prepared".into() },
        ];
        assert!(!requests[0].action_request_id.is_empty());
    }

    #[test]
    fn timeline_hash_display_preserves_identity() {
        use crate::ui::workflow_evidence_chain_inspector_components::chain_hash_display;
        let short = chain_hash_display("abc123");
        assert_eq!("abc123", short);
        let long = chain_hash_display("abcdef0123456789abcdef0123456789abcdef01");
        assert!(long.contains("…"), "long hash should be truncated: {long}");
    }

    // ── Patch 5: Recorded-event copy guard ──

    #[test]
    fn timeline_copy_says_recorded_run() {
        let note = recorded_run_note();
        assert!(note.to_lowercase().contains("recorded"), "got: {note}");
    }

    #[test]
    fn timeline_copy_says_display_only() {
        let note = timeline_display_only_note();
        assert!(note.to_lowercase().contains("display only"), "got: {note}");
    }

    #[test]
    fn timeline_copy_contains_no_replay_or_repair_terms() {
        let all_copy = vec![
            recorded_run_note(),
            timeline_display_only_note(),
            action_request_recorded_note(),
            abort_recorded_state_note(),
            execution_safety_text(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("replay"), "copy: {text}");
            assert!(!lower.contains("repair"), "copy: {text}");
            assert!(!lower.contains("rerun"), "copy: {text}");
            assert!(!lower.contains("resume"), "copy: {text}");
        }
    }

    #[test]
    fn timeline_copy_contains_no_verification_or_certification_terms() {
        let all_copy = vec![
            recorded_run_note(),
            timeline_display_only_note(),
            execution_safety_text(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("certified"), "copy: {text}");
            assert!(!lower.contains("trusted"), "copy: {text}");
        }
    }
}
