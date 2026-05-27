//! Policy engine trait.

use async_trait::async_trait;

use crate::decision::PolicyEvaluation;
use crate::error::PolicyError;
use crate::request::{PolicyRequest, ToolFilterRequest};
use crate::tool::PolicyToolDescriptor;

/// The policy engine evaluates tool calls and filters tool manifests.
///
/// Implementation must be deterministic: same inputs → same outputs.
/// Fail-closed: evaluation error → block the tool call.
#[async_trait]
pub trait PolicyEngine: Send + Sync {
    /// Evaluate a single tool call against all applicable rules.
    /// Returns a full PolicyEvaluation with findings, risk, and decision.
    async fn evaluate_tool_call(
        &self,
        request: PolicyRequest,
    ) -> Result<PolicyEvaluation, PolicyError>;

    /// Filter the tool manifest for prompt-surface reduction.
    /// Defense-in-depth — NOT the authority boundary.
    /// Even if a hidden tool call appears, evaluate_tool_call must still block it.
    async fn filter_tools(
        &self,
        request: ToolFilterRequest,
    ) -> Result<Vec<PolicyToolDescriptor>, PolicyError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prove PolicyEngine is object-safe.
    #[test]
    fn policy_engine_trait_object_compiles() {
        fn _uses_arc_dyn(_store: std::sync::Arc<dyn PolicyEngine>) {}
    }
}
