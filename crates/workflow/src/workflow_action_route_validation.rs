//! Validation for workflow action route records.

use crate::workflow_action_route::*;

/// Generate content-addressed route ID from determinative fields.
pub fn action_route_id_for(
    execution_id: &str,
    stage_id: &str,
    action_request_id: &str,
    idempotency_key: &str,
) -> WorkflowActionRouteId {
    let preimage = format!("{}:{}:{}:{}", execution_id, stage_id, action_request_id, idempotency_key);
    let hash = blake3::hash(preimage.as_bytes());
    WorkflowActionRouteId(format!("war_{}", hash.to_hex()))
}

/// Validate that a route prompt contains no executable fields.
pub fn validate_route_prompt_no_executable_fields(prompt: &WorkflowActionRoutePrompt) -> Result<(), String> {
    let forbidden_patterns = ["tool_name", "tool_args", "command", "shell", "script",
                              "cwd", "env", "function_ref", "provider_request"];
    let all_text = format!("{}{}{}{}{}",
        prompt.capability_category, prompt.purpose,
        prompt.expected_input_summary, prompt.expected_output_summary,
        prompt.safety_constraints.join(""));
    for pattern in &forbidden_patterns {
        if all_text.to_lowercase().contains(pattern) {
            return Err(format!("Route prompt contains forbidden pattern: {}", pattern));
        }
    }
    Ok(())
}

/// Validate that a route prompt includes the governance constraint.
/// Patch 4: anti-tool-call guard text must be present in the session instruction.
pub fn validate_route_prompt_includes_governance_constraint(prompt: &WorkflowActionRoutePrompt) -> Result<(), String> {
    let instruction = prompt.to_session_instruction();
    if !instruction.contains("Do not treat this workflow action request as a direct tool call") {
        return Err("Route prompt must include anti-tool-call governance constraint".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_route_id_is_deterministic() {
        let id1 = action_route_id_for("wfx_1", "s1", "ar_1", "key");
        let id2 = action_route_id_for("wfx_1", "s1", "ar_1", "key");
        assert_eq!(id1, id2);
    }

    #[test]
    fn action_route_id_differs_for_different_inputs() {
        let id1 = action_route_id_for("wfx_1", "s1", "ar_1", "key1");
        let id2 = action_route_id_for("wfx_1", "s1", "ar_1", "key2");
        assert_ne!(id1, id2);
    }
}
