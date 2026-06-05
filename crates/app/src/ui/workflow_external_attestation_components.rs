//! UI external attestation components — desktop-gated render placeholder.

pub fn render_external_attestation_placeholder() -> String {
    "External attestation screen — desktop-gated placeholder.".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_placeholder_returns_non_empty() {
        let s = render_external_attestation_placeholder();
        assert!(!s.is_empty());
    }
}
