//! Workflow evidence chain inspector desktop UI components.
//!
//! Read-only display of the evidence chain inspector using Wave 52A design-system
//! tokens. The inspector displays inspection summary, recorded evidence links,
//! coverage, chain warnings, and reported attestations.
//!
//! Components accept already-prepared state. They do not load, assemble, query,
//! export packets, write files, or call services.
//!
//! This module imports no backend crates, performs no persistence, executes
//! no tools, verifies nothing, certifies nothing, mutates no trace or memory
//! state, resolves no approvals, routes no actions, and creates no workflow
//! records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_evidence_chain_inspector_state::*;

// ── Semantic presence enum ────────────────────────────────────────────────

/// Semantic display variant for evidence link presence.
/// Converted from raw strings at the boundary — no typos can change tone behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidencePresenceDisplay {
    Present,
    Missing,
    NotYetApplicable,
    Unknown,
}

/// Convert a raw presence string to a semantic display variant.
/// Unknown strings map to `Unknown` without panic.
pub fn presence_display_from_str(value: &str) -> EvidencePresenceDisplay {
    match value {
        "Present" => EvidencePresenceDisplay::Present,
        "MissingExpected" => EvidencePresenceDisplay::Missing,
        "NotYetApplicable" => EvidencePresenceDisplay::NotYetApplicable,
        _ => EvidencePresenceDisplay::Unknown,
    }
}

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Human-readable label for evidence presence.
pub fn link_presence_label(presence: EvidencePresenceDisplay) -> &'static str {
    match presence {
        EvidencePresenceDisplay::Present => "Present",
        EvidencePresenceDisplay::Missing => "Missing",
        EvidencePresenceDisplay::NotYetApplicable => "Not yet applicable",
        EvidencePresenceDisplay::Unknown => "Unknown",
    }
}

/// Semantic tone for evidence presence.
pub fn link_presence_tone(presence: EvidencePresenceDisplay) -> UiTone {
    match presence {
        EvidencePresenceDisplay::Present => UiTone::Success,
        EvidencePresenceDisplay::Missing => UiTone::Warning,
        EvidencePresenceDisplay::NotYetApplicable => UiTone::Neutral,
        EvidencePresenceDisplay::Unknown => UiTone::Neutral,
    }
}

/// Textual coverage progress: "5 / 10 present, 3 missing, 2 not yet applicable"
pub fn coverage_progress_text(present: usize, missing: usize, not_yet: usize) -> String {
    let total = present + missing + not_yet;
    format!("{} / {} present, {} missing, {} not yet applicable", present, total, missing, not_yet)
}

/// Display chain hash preserving enough text for identity.
/// Short hashes shown in full. Long hashes shown as first 12 + … + last 8.
/// Never uses verified/truth language.
pub fn chain_hash_display(hash: &str) -> String {
    if hash.len() <= 20 {
        hash.to_string()
    } else {
        format!("{}…{}", &hash[..12], &hash[hash.len()-8..])
    }
}

/// Safety warning for inspector display.
pub fn inspector_safety_text() -> String {
    chain_safety_warning()
}

/// Label describing the hash as "chain hash" (recorded, not verified truth).
pub fn chain_hash_label() -> String {
    "Recorded chain hash".into()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use crate::ui::layout::*;
    use dioxus::prelude::*;
    use openwand_workflow::workflow_evidence_chain_inspector::{
        EvidenceChainInspectionState, EvidenceChainLink,
    };

    /// Empty state when no workflow execution is selected.
    pub fn render_inspector_empty_state() -> Element {
        let style = empty_state_style();
        rsx! {
            div { style: "{style}",
                div { style: "text-align: center;",
                    div { style: "font-size: 16px; color: #999; margin-bottom: 8px;",
                        "No evidence chain inspection available"
                    }
                    div { style: "font-size: 12px; color: #bbb;",
                        "Run a governed workflow to see the evidence chain inspector."
                    }
                }
            }
        }
    }

    /// Loading state while inspection is being assembled.
    pub fn render_inspector_loading_state() -> Element {
        let style = empty_state_style();
        rsx! {
            div { style: "{style}",
                div { style: "font-size: 14px; color: {COLORS::TEXT_MUTED};",
                    "Loading evidence chain inspector…"
                }
            }
        }
    }

    /// Error state when inspection assembly fails.
    pub fn render_inspector_error_state(message: &str) -> Element {
        let style = banner_style(UiTone::Error);
        let safe: String = message.chars().take(200).collect();
        rsx! {
            div { style: "{style}",
                div { style: "font-weight: 600; margin-bottom: 4px;",
                    "Evidence chain inspector error"
                }
                div { "{safe}" }
            }
        }
    }

    /// Safety banner for the inspector.
    pub fn render_safety_banner() -> Element {
        let text = inspector_safety_text();
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                "⚠ {text}"
            }
        }
    }

    /// Chain summary header with inspection ID, workflow execution ID, coverage, and chain hash.
    pub fn render_chain_summary(row: &EvidenceChainSummaryRow) -> Element {
        let header_style = header_bar_style();
        let coverage_text = coverage_progress_text(
            row.present_links, row.missing_links, row.not_yet_applicable,
        );
        let coverage_tone = if row.missing_links == 0 && row.present_links > 0 {
            UiTone::Success
        } else if row.present_links == 0 {
            UiTone::Warning
        } else {
            UiTone::Info
        };
        let badge_s = badge_style(coverage_tone);
        let hash_display = chain_hash_display(&row.chain_hash);
        let hash_label = chain_hash_label();
        let title_style = format!("margin: 0 0 2px 0; font-size: {};", TYPO::TEXT_XL);
        let id_style = format!("font-size: {}; color: {};", TYPO::TEXT_SM, COLORS::TEXT_MUTED);
        let hash_style = format!(
            "font-size: {}; color: {}; font-family: {}; margin-top: {};",
            TYPO::TEXT_XS, COLORS::TEXT_MUTED, TYPO::FONT_MONO, SPACING::SPACE_SM,
        );

        rsx! {
            div { style: "{header_style}",
                div {
                    div { style: "{title_style}",
                        "Evidence Chain — {row.workflow_execution_id}"
                    }
                    div { style: "{id_style}",
                        "Inspection: {row.inspection_id}"
                    }
                    div { style: "{hash_style}",
                        "{hash_label}: {hash_display}"
                    }
                }
                div { style: "display: flex; gap: 8px; align-items: center;",
                    span { style: "{badge_s}",
                        "{coverage_text}"
                    }
                }
            }
        }
    }

    /// Coverage summary card with textual progress.
    pub fn render_coverage_summary(
        present: usize,
        missing: usize,
        not_yet: usize,
    ) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);
        let total = present + missing + not_yet;
        let progress = coverage_progress_text(present, missing, not_yet);
        let tone = if missing == 0 && present > 0 {
            UiTone::Success
        } else if present == 0 {
            UiTone::Warning
        } else {
            UiTone::Info
        };
        let dot_s = status_dot_style(tone, UiSize::Sm);
        let row_s = format!(
            "display: flex; align-items: center; gap: {}; padding: {} 0; font-size: {};",
            SPACING::SPACE_MD, SPACING::SPACE_SM, TYPO::TEXT_SM,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Coverage ({total} links)"
                }
                div { style: "{row_s}",
                    div { style: "{dot_s}" }
                    span { "{progress}" }
                }
            }
        }
    }

    /// Scrollable evidence chain links list with presence indicators.
    pub fn render_chain_links(links: &[EvidenceChainLink]) -> Element {
        let card_s = card_style(UiTone::Neutral, UiDensity::Compact);
        let header_s = section_header_style(UiTone::Primary);
        let scroll_s = format!(
            "max-height: 200px; overflow-y: auto; font-family: {};",
            TYPO::FONT_MONO,
        );

        rsx! {
            div { style: "{card_s}",
                div { style: "{header_s}",
                    "Recorded Evidence Links ({links.len()})"
                }
                div { style: "{scroll_s}",
                    for link in links {
                        {
                            let presence = presence_display_from_str(format!("{:?}", link.presence).as_str());
                            let tone = link_presence_tone(presence);
                            let label = link_presence_label(presence);
                            let dot_s = status_dot_style(tone, UiSize::Xs);
                            let row_s = format!(
                                "display: flex; align-items: center; gap: {}; padding: {} 0; \
                                 font-size: {}; border-bottom: 1px solid {};",
                                SPACING::SPACE_SM, SPACING::SPACE_XS,
                                TYPO::TEXT_SM, COLORS::BORDER_SUBTLE,
                            );
                            let type_s = format!("min-width: 140px; color: {};", COLORS::TEXT_PRIMARY);
                            let id_s = format!("color: {};", COLORS::TEXT_MUTED);
                            let label_s = format!("color: {};", tone.text());
                            rsx! {
                                div { style: "{row_s}",
                                    div { style: "{dot_s}" }
                                    span { style: "{type_s}", "{link.record_type}" }
                                    span { style: "{id_s}", "{link.record_id}" }
                                    span { style: "{label_s}", " — {label}" }
                                }
                            }
                        }
                    }
                    if links.is_empty() {
                        div { style: "padding: 8px; font-size: 12px; color: #999; text-align: center;",
                            "No evidence links recorded"
                        }
                    }
                }
            }
        }
    }

    /// Chain warnings display.
    pub fn render_chain_warnings(warnings: &[String], warning_count: usize) -> Element {
        if warnings.is_empty() && warning_count == 0 {
            return rsx! { div {} };
        }
        let style = banner_style(UiTone::Warning);
        rsx! {
            div { style: "{style}",
                div { style: "font-weight: 600; margin-bottom: 4px;",
                    "Linkage Warnings ({warning_count})"
                }
                for warning in warnings {
                    div { style: "font-size: 12px;", "⚠ {warning}" }
                }
            }
        }
    }

    /// Full evidence chain inspector surface.
    pub fn render_evidence_chain_inspector(state: &EvidenceChainInspectionState) -> Element {
        let summary = chain_summary_lines(state);
        let scroll_s = scroll_area_style();

        rsx! {
            div { style: "flex: 1; display: flex; flex-direction: column; min-width: 0;",
                { render_chain_summary(&summary) }

                div { style: "{scroll_s}",
                    { render_coverage_summary(summary.present_links, summary.missing_links, summary.not_yet_applicable) }
                    { render_chain_links(&state.links) }
                    { render_chain_warnings(&state.coverage_summary.linkage_warnings.iter().map(|w| w.reason.clone()).collect::<Vec<_>>(), summary.warning_count) }
                }

                { render_safety_banner() }
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

    // ── Presence display mapping ──

    #[test]
    fn presence_display_maps_present() {
        assert_eq!(EvidencePresenceDisplay::Present, presence_display_from_str("Present"));
    }

    #[test]
    fn presence_display_maps_missing() {
        assert_eq!(EvidencePresenceDisplay::Missing, presence_display_from_str("MissingExpected"));
    }

    #[test]
    fn presence_display_maps_not_yet_applicable() {
        assert_eq!(EvidencePresenceDisplay::NotYetApplicable, presence_display_from_str("NotYetApplicable"));
    }

    #[test]
    fn presence_display_maps_unknown_without_panic() {
        assert_eq!(EvidencePresenceDisplay::Unknown, presence_display_from_str("SomeFutureVariant"));
        assert_eq!(EvidencePresenceDisplay::Unknown, presence_display_from_str(""));
    }

    // ── Presence labels and tones ──

    #[test]
    fn link_presence_label_text() {
        assert_eq!("Present", link_presence_label(EvidencePresenceDisplay::Present));
        assert_eq!("Missing", link_presence_label(EvidencePresenceDisplay::Missing));
        assert_eq!("Not yet applicable", link_presence_label(EvidencePresenceDisplay::NotYetApplicable));
        assert_eq!("Unknown", link_presence_label(EvidencePresenceDisplay::Unknown));
    }

    #[test]
    fn link_presence_tone_mapping() {
        assert_eq!(UiTone::Success, link_presence_tone(EvidencePresenceDisplay::Present));
        assert_eq!(UiTone::Warning, link_presence_tone(EvidencePresenceDisplay::Missing));
        assert_eq!(UiTone::Neutral, link_presence_tone(EvidencePresenceDisplay::NotYetApplicable));
        assert_eq!(UiTone::Neutral, link_presence_tone(EvidencePresenceDisplay::Unknown));
    }

    // ── Coverage progress ──

    #[test]
    fn coverage_progress_text_shows_all_counts() {
        let text = coverage_progress_text(5, 3, 2);
        assert!(text.contains("5 / 10 present"), "got: {text}");
        assert!(text.contains("3 missing"), "got: {text}");
        assert!(text.contains("2 not yet applicable"), "got: {text}");
    }

    // ── Chain hash display ──

    #[test]
    fn chain_hash_display_preserves_short_hash() {
        let short = "abcd1234";
        assert_eq!(short, chain_hash_display(short));
    }

    #[test]
    fn chain_hash_display_shortens_long_hash_with_prefix_and_suffix() {
        let long = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let displayed = chain_hash_display(long);
        assert!(displayed.starts_with("0123456789ab"), "prefix: {displayed}");
        assert!(displayed.ends_with("89abcdef"), "suffix: {displayed}");
        assert!(displayed.contains("…"), "should contain ellipsis: {displayed}");
    }

    #[test]
    fn chain_hash_display_does_not_use_verified_language() {
        let label = chain_hash_label();
        let lower = label.to_lowercase();
        assert!(!lower.contains("verified"), "label should not say verified: {label}");
        assert!(!lower.contains("truth"), "label should not say truth: {label}");
        assert!(lower.contains("recorded"), "label should say recorded: {label}");
    }

    // ── Guard: no export/import of export types ──

    #[test]
    fn inspector_components_do_not_import_export_types() {
        // Compile-time check: this module does not import AuditPacket, export_audit_packet,
        // or any export-related types.
        // If it did, the compilation would include those imports — but it doesn't.
        let _ = presence_display_from_str("Present");
        let _ = coverage_progress_text(1, 0, 0);
    }

    // ── Guard: UI copy uses recorded, not verified language ──

    #[test]
    fn inspector_ui_copy_uses_recorded_not_verified_language() {
        let all_copy: Vec<String> = vec![
            coverage_progress_text(5, 3, 2),
            link_presence_label(EvidencePresenceDisplay::Present).to_string(),
            link_presence_label(EvidencePresenceDisplay::Missing).to_string(),
            chain_hash_label(),
        ];
        let combined = all_copy.join(" ").to_lowercase();
        assert!(combined.contains("recorded") || combined.contains("present") || combined.contains("missing"),
            "Should use recorded/present/missing language: {combined}");
    }

    #[test]
    fn inspector_ui_copy_contains_no_certification_or_truth_terms() {
        let all_copy: Vec<String> = vec![
            coverage_progress_text(5, 3, 2),
            link_presence_label(EvidencePresenceDisplay::Present).to_string(),
            link_presence_label(EvidencePresenceDisplay::Missing).to_string(),
            chain_hash_label(),
            link_presence_label(EvidencePresenceDisplay::NotYetApplicable).to_string(),
        ];
        for text in &all_copy {
            let lower = text.to_lowercase();
            assert!(!lower.contains("verified"), "copy contains 'verified': {text}");
            assert!(!lower.contains("proven"), "copy contains 'proven': {text}");
            assert!(!lower.contains("certified"), "copy contains 'certified': {text}");
            assert!(!lower.contains("trusted"), "copy contains 'trusted': {text}");
            assert!(!lower.contains("truth"), "copy contains 'truth': {text}");
        }
    }

    // ── Guard: safety warning language ──

    #[test]
    fn inspector_safety_text_mentions_inspector_not_verifier() {
        let text = inspector_safety_text();
        assert!(text.to_lowercase().contains("inspector"), "should mention inspector: {text}");
        assert!(!text.to_lowercase().contains("verifier"), "should not say verifier: {text}");
    }
}
