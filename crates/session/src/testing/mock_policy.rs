use async_trait::async_trait;
use openwand_core::mode::ConfirmationLevel;
use openwand_core::risk::RiskLevelSnapshot;
use openwand_core::GateId;
use openwand_policy::{
    GateDecision, PolicyEngine, PolicyError, PolicyEvaluation, PolicyRequest,
    ToolFilterRequest, PolicyToolDescriptor,
};
use tokio::sync::Mutex;

/// Behavior modes for the mock policy engine.
#[derive(Debug, Clone)]
pub enum MockPolicyBehavior {
    AllowAll,
    BlockToolName(String),
    /// Require confirmation for a specific tool name.
    RequireConfirmationFor(String),
    /// Require confirmation for multiple tool names, allow the rest.
    RequireConfirmationForMany(Vec<String>),
    Fail,
}

/// Mock policy engine for deterministic testing.
pub struct MockPolicyEngine {
    behavior: MockPolicyBehavior,
    evaluations: Mutex<Vec<PolicyRequest>>,
}

impl MockPolicyEngine {
    pub fn new(behavior: MockPolicyBehavior) -> Self {
        Self {
            behavior,
            evaluations: Mutex::new(Vec::new()),
        }
    }

    pub fn allow_all() -> Self {
        Self::new(MockPolicyBehavior::AllowAll)
    }

    pub fn block_tool(name: &str) -> Self {
        Self::new(MockPolicyBehavior::BlockToolName(name.into()))
    }

    pub fn require_confirmation_for(name: &str) -> Self {
        Self::new(MockPolicyBehavior::RequireConfirmationFor(name.into()))
    }

    pub fn require_confirmation_for_many(names: Vec<&str>) -> Self {
        Self::new(MockPolicyBehavior::RequireConfirmationForMany(
            names.into_iter().map(String::from).collect(),
        ))
    }

    pub fn fail() -> Self {
        Self::new(MockPolicyBehavior::Fail)
    }

    pub async fn evaluations(&self) -> Vec<PolicyRequest> {
        self.evaluations.lock().await.clone()
    }
}

fn allow_evaluation() -> PolicyEvaluation {
    PolicyEvaluation {
        gate_id: GateId::new(),
        decision: GateDecision::Allow,
        risk_level: RiskLevelSnapshot::Low,
        confirmation_level: ConfirmationLevel::Auto,
        findings: vec![],
        matched_rules: vec![],
        reason_code: "mock_allow".into(),
        summary: "Mock policy: allowed".into(),
        rollback_required: false,
        rollback_plan: None,
    }
}

fn require_confirmation_evaluation(tool_name: &str) -> PolicyEvaluation {
    PolicyEvaluation {
        gate_id: GateId::new(),
        decision: GateDecision::RequireConfirmation {
            level: ConfirmationLevel::Approve,
        },
        risk_level: RiskLevelSnapshot::Medium,
        confirmation_level: ConfirmationLevel::Approve,
        findings: vec![],
        matched_rules: vec![],
        reason_code: "mock_require_confirmation".into(),
        summary: format!("Mock policy: '{}' requires confirmation", tool_name),
        rollback_required: false,
        rollback_plan: None,
    }
}

fn block_evaluation(reason: &str) -> PolicyEvaluation {
    PolicyEvaluation {
        gate_id: GateId::new(),
        decision: GateDecision::Block {
            reason: reason.into(),
        },
        risk_level: RiskLevelSnapshot::Critical,
        confirmation_level: ConfirmationLevel::Escalate,
        findings: vec![],
        matched_rules: vec![],
        reason_code: "mock_block".into(),
        summary: format!("Mock policy: {reason}"),
        rollback_required: false,
        rollback_plan: None,
    }
}

#[async_trait]
impl PolicyEngine for MockPolicyEngine {
    async fn evaluate_tool_call(
        &self,
        request: PolicyRequest,
    ) -> Result<PolicyEvaluation, PolicyError> {
        self.evaluations.lock().await.push(request.clone());

        match &self.behavior {
            MockPolicyBehavior::AllowAll => Ok(allow_evaluation()),
            MockPolicyBehavior::BlockToolName(name)
                if request.tool_call.name == *name =>
            {
                Ok(block_evaluation("blocked_by_mock_policy"))
            }
            MockPolicyBehavior::BlockToolName(_) => Ok(allow_evaluation()),
            MockPolicyBehavior::RequireConfirmationFor(name)
                if request.tool_call.name == *name =>
            {
                Ok(require_confirmation_evaluation(name))
            }
            MockPolicyBehavior::RequireConfirmationFor(_) => Ok(allow_evaluation()),
            MockPolicyBehavior::RequireConfirmationForMany(names)
                if names.iter().any(|n| request.tool_call.name == *n) =>
            {
                Ok(require_confirmation_evaluation(&request.tool_call.name))
            }
            MockPolicyBehavior::RequireConfirmationForMany(_) => Ok(allow_evaluation()),
            MockPolicyBehavior::Fail => Err(PolicyError::Internal(
                "mock policy failure".into(),
            )),
        }
    }

    async fn filter_tools(
        &self,
        request: ToolFilterRequest,
    ) -> Result<Vec<PolicyToolDescriptor>, PolicyError> {
        Ok(request.tools)
    }
}
