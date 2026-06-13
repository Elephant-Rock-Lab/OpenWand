//! External attestation desktop UI components.
//!
//! Read-only display of recorded external attestation state using Wave 52A
//! design-system tokens. Displays attestation summary (target, kind, source,
//! claim, verification status) and safety copy.
//!
//! Components accept already-prepared state. They do not verify, certify,
//! trust-promote, reconcile, approve, execute, or mutate workflow state.

use crate::ui::design_tokens::*;
use crate::ui::workflow_external_attestation_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

pub fn attestation_kind_label(kind: &str) -> String {
    match kind {
        "CodeReviewApproval" => "Code review approval".into(),
        "ExternalTestResult" => "External test result".into(),
        "DeploymentConfirmation" => "Deployment confirmation".into(),
        "SecurityScan" => "Security scan".into(),
        "ManualSignOff" => "Manual sign-off".into(),
        _ => format!("Attestation: {}", kind),
    }
}

pub fn attestation_target_kind_label(target: &str) -> String {
    match target {
        "ManualResult" => "Manual result".into(),
        "WorkflowRun" => "Workflow run".into(),
        "StageOutcome" => "Stage outcome".into(),
        _ => format!("Target: {}", target),
    }
}

pub fn verified_tone(verified: bool) -> UiTone {
    if verified { UiTone::Success } else { UiTone::Warning }
}

pub fn attestation_safety_text() -> String {
    attestation_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use dioxus::prelude::*;

    pub fn render_external_attestation_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! { div { style: "{style}", "No external attestation records" } }
    }

    pub fn render_external_attestation_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM, colors::TEXT_MUTED,
        );
        rsx! { div { style: "{style}", "Loading attestation\u{2026}" } }
    }

    pub fn render_external_attestation_card(att: &AttestationSummaryRow) -> Element {
        let card = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD, spacing::SPACE_MD, spacing::SPACE_MD,
        );
        let tone = verified_tone(att.verified);
        let badge = tone_to_badge_color(tone);
        let verified_label = if att.verified { "Verified by OpenWand" } else { "Not verified (reported only)" };
        let target_label = attestation_target_kind_label(&att.target_kind);
        let kind_label = attestation_kind_label(&att.kind);

        rsx! {
            div { style: "{card}",
                // Header row: kind + verification badge
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: {spacing::SPACE_SM};",
                    span { style: "font-weight: 600; font-size: {typo::TEXT_SM}; color: {colors::TEXT_STRONG};",
                        "{kind_label}"
                    }
                    span { style: "background: {badge}; color: white; padding: 2px 8px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS};",
                        "{verified_label}"
                    }
                }

                // Claim
                div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY}; background: {colors::BG_SUBTLE}; padding: {spacing::SPACE_SM}; border-radius: {radius::RADIUS_SM}; margin-bottom: {spacing::SPACE_SM}; font-family: monospace;",
                    "{att.claim}"
                }

                // Source + target grid
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: {spacing::SPACE_MD}; font-size: {typo::TEXT_SM};",
                    div {
                        div { style: "color: {colors::TEXT_MUTED}; font-size: {typo::TEXT_XS};", "Source" }
                        div { style: "color: {colors::TEXT_STRONG};", "{att.source_name}" }
                    }
                    div {
                        div { style: "color: {colors::TEXT_MUTED}; font-size: {typo::TEXT_XS};", "Target" }
                        div { style: "color: {colors::TEXT_STRONG};", "{target_label}" }
                    }
                }

                // ID footer
                div { style: "margin-top: {spacing::SPACE_SM}; font-size: {typo::TEXT_XS}; color: {colors::TEXT_FAINT}; font-family: monospace;",
                    "{att.attestation_id} \u{2192} {att.target_id}"
                }
            }
        }
    }

    pub fn render_external_attestation_panel(attestations: &[AttestationSummaryRow]) -> Element {
        let title_style = format!(
            "font-size: {}; font-weight: 600; color: {}; margin-bottom: {};",
            typo::TEXT_BASE, colors::TEXT_STRONG, spacing::SPACE_SM,
        );

        rsx! {
            div {
                div { style: "{title_style}", "External Attestations" }

                if attestations.is_empty() {
                    {render_external_attestation_empty_state()}
                } else {
                    for att in attestations {
                        {render_external_attestation_card(att)}
                    }
                }

                {render_safety_footer()}
            }
        }
    }

    fn render_safety_footer() -> Element {
        let style = format!(
            "font-size: {}; color: {}; padding: {} 0; border-top: 1px solid {}; margin-top: {};",
            typo::TEXT_XS, colors::TEXT_FAINT, spacing::SPACE_SM,
            colors::BORDER_LIGHT, spacing::SPACE_MD,
        );
        let text = attestation_safety_text();
        rsx! { div { style: "{style}", "{text}" } }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attestation_kind_label_formats_known_kinds() {
        assert_eq!("Code review approval", attestation_kind_label("CodeReviewApproval"));
        assert_eq!("External test result", attestation_kind_label("ExternalTestResult"));
    }

    #[test]
    fn attestation_kind_label_falls_back_for_unknown() {
        let label = attestation_kind_label("CustomThing");
        assert!(label.contains("CustomThing"));
    }

    #[test]
    fn attestation_target_kind_label_formats_known_targets() {
        assert_eq!("Manual result", attestation_target_kind_label("ManualResult"));
        assert_eq!("Workflow run", attestation_target_kind_label("WorkflowRun"));
    }

    #[test]
    fn verified_tone_returns_success_for_true() {
        assert!(matches!(verified_tone(true), UiTone::Success));
    }

    #[test]
    fn verified_tone_returns_warning_for_false() {
        assert!(matches!(verified_tone(false), UiTone::Warning));
    }

    #[test]
    fn attestation_safety_text_matches_warning() {
        let text = attestation_safety_text();
        assert!(text.contains("reported evidence"));
        assert!(text.contains("not verification"));
    }
}
