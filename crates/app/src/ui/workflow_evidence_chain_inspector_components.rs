//! UI evidence chain inspector components — desktop-gated render placeholder.

pub fn render_evidence_chain_placeholder() -> String {
    "Evidence chain inspector screen — desktop-gated placeholder.".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_placeholder_returns_non_empty() {
        let s = render_evidence_chain_placeholder();
        assert!(!s.is_empty());
    }
}
