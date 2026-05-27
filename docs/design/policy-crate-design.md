# OpenWand Policy Crate Design

**Date:** 2026-05-26  
**Status:** Design — locked  
**Crate:** `openwand-policy`  
**Depends on:** `openwand-core`  
**Blocks:** Batch 1 ToolGate, all subsequent batches  

---

## Core Principle

> Policy is the trust gate between "LLM wants to do X" and "X actually happens." Policy evaluation is deterministic, conservative, and fail-closed. Every tool action passes through the same governance seam from day one.

The trust principle from the architecture:

```
AI proposes. Deterministic systems constrain. Evidence promotes. Users govern consequential change.
```

And the fail-closed contract:

```
Policy failure blocks the tool call, not the session.
The session continues. The tool does not execute.
```

---

## Crate Boundary

### Depends on

```
openwand-core  (IDs, RiskLevelSnapshot, ConfirmationLevel, InteractionMode, GateId)
async-trait
serde, serde_json
thiserror
tracing
```

### Does NOT depend on

```
openwand-session  (would create cycle)
openwand-tools    (would create cycle)
openwand-trace    (policy doesn't write trace — session does)
openwand-memory
openwand-llm
openwand-store
loro, rig, rmcp
```

### Contains

- `PolicyEngine` trait + one built-in implementation
- Rule model: matchers, effects, three rule classes
- Risk assessment: tool-declared base effect + argument modifiers
- Gate evaluation: aggregate all findings, then apply block dominance
- Mode mapping: confirmation floor, never risk override
- Tool filtering: prompt-surface reduction (defense-in-depth)
- Fail-closed construction: evaluation error → block

### Does NOT contain

- LLM-assisted semantic gates (v2+)
- Rate limiting / budget gates (future gate family)
- Approval grant persistence (runtime, not policy)
- Session overrides (v2+)
- Rule hot-reloading (v2+)

---

## Module Structure

```
openwand-policy/
  Cargo.toml
  src/
    lib.rs            — re-exports
    engine.rs         — PolicyEngine trait
    builtin.rs        — BuiltinPolicyEngine::batch1()
    request.rs        — PolicyRequest, ToolFilterRequest, PolicyContext
    tool.rs           — PolicyToolCall, PolicyToolDescriptor, ToolEffect
    rule.rs           — PolicyRule, RuleClass, ToolMatcher, PolicyEffect, PolicyRuleId
    risk.rs           — RiskAssessor trait, BasicRiskAssessor, RiskReason
    decision.rs       — PolicyEvaluation, GateDecision, GateFinding
    scope.rs          — ScopeStack: builtin → global → project
    error.rs          — PolicyError
    mapping.rs        — confirmation_for(), apply_mode_floor()
```

---

## Core Types

### PolicyToolCall and PolicyToolDescriptor

Policy defines its own DTOs. Session adapts from session/tool types into these:

```rust
// tool.rs

/// Neutral tool call representation for policy evaluation.
/// Session constructs this from its internal ToolCall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyToolCall {
    pub id: ToolCallId,
    pub name: String,
    pub arguments: serde_json::Value,
    pub declared_effect: ToolEffect,  // imported from openwand-core
}

/// Neutral tool descriptor for manifest filtering.
/// Tools register this with the policy engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyToolDescriptor {
    pub name: String,
    pub source: PolicyToolSource,
    pub declared_effect: ToolEffect,  // imported from openwand-core
    pub risk_hints: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyToolSource {
    Local,
    Mcp { server: String },
    System,
}

// ToolEffect is defined in openwand-core::tool_vocab.
// Policy imports it. It is NOT redefined here.
```

### PolicyContext and Requests

```rust
// request.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyContext {
    pub working_directory: String,
    pub model: String,
    pub session_id: SessionId,
    pub recent_gate_history: Vec<GateResultSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRequest {
    pub tool_call: PolicyToolCall,
    pub mode: InteractionMode,
    pub context: PolicyContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFilterRequest {
    pub tools: Vec<PolicyToolDescriptor>,
    pub mode: InteractionMode,
    pub context: PolicyContext,
}
```

---

## PolicyEngine Trait

```rust
// engine.rs

#[async_trait::async_trait]
pub trait PolicyEngine: Send + Sync {
    /// Evaluate a single tool call against all applicable rules.
    /// Returns a full PolicyEvaluation with findings, risk, and decision.
    async fn evaluate_tool_call(
        &self,
        request: PolicyRequest,
    ) -> Result<PolicyEvaluation, PolicyError>;

    /// Filter the tool manifest for prompt-surface reduction.
    /// This is defense-in-depth, NOT the authority boundary.
    /// Even if a hidden tool call appears, evaluate_tool_call must still block it.
    async fn filter_tools(
        &self,
        request: ToolFilterRequest,
    ) -> Result<Vec<PolicyToolDescriptor>, PolicyError>;
}
```

---

## Decision Types

### GateDecision

Three-way decision: execute now, execute after confirmation, or blocked.

```rust
// decision.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateDecision {
    /// Tool call may execute immediately.
    Allow,

    /// Tool call may execute after the user provides confirmation.
    RequireConfirmation {
        level: ConfirmationLevel,
    },

    /// Tool call is blocked. It will not execute.
    Block {
        reason: String,
    },
}

impl GateDecision {
    pub fn allows_execution(&self) -> bool {
        matches!(self, Self::Allow)
    }

    pub fn requires_confirmation(&self) -> bool {
        matches!(self, Self::RequireConfirmation { .. })
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Block { .. })
    }
}
```

### GateFinding

Individual gate evaluation result. All findings are collected before finalization.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateFinding {
    pub rule_id: Option<PolicyRuleId>,
    pub result: GateFindingResult,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateFindingResult {
    Allow,
    Require { risk: RiskLevelSnapshot },
    Block,
}
```

### PolicyEvaluation

The full evaluation result. Session constructs `GateEvent::Evaluated` from this.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    /// Unique ID for this gate evaluation
    pub gate_id: GateId,

    /// The final decision
    pub decision: GateDecision,

    /// Assessed risk level
    pub risk_level: RiskLevelSnapshot,

    /// Required confirmation level (derived from risk × mode)
    pub confirmation_level: ConfirmationLevel,

    /// All gate findings collected during evaluation
    pub findings: Vec<GateFinding>,

    /// Which rules matched and contributed to the decision
    pub matched_rules: Vec<PolicyRuleId>,

    /// Machine-readable reason code for trace
    pub reason_code: String,

    /// Human-readable summary
    pub summary: String,

    /// Whether a rollback plan is required (for Escalate)
    pub rollback_required: bool,

    /// Suggested rollback plan (if available)
    pub rollback_plan: Option<String>,
}

impl PolicyEvaluation {
    /// Construct a fail-closed evaluation from a policy error.
    /// The tool call is blocked. The session continues.
    pub fn fail_closed(error: PolicyError) -> Self {
        Self {
            gate_id: GateId::new(),
            decision: GateDecision::Block {
                reason: "Policy evaluation failed; tool call blocked.".into(),
            },
            risk_level: RiskLevelSnapshot::Critical,
            confirmation_level: ConfirmationLevel::Escalate,
            findings: vec![],
            matched_rules: vec![],
            reason_code: "policy_evaluation_failed".into(),
            summary: format!("Policy evaluation failed: {}", error.safe_message()),
            rollback_required: false,
            rollback_plan: None,
        }
    }
}
```

---

## Rule Model

### PolicyRule

```rust
// rule.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: PolicyRuleId,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,          // higher = evaluated first
    pub class: RuleClass,
    pub matcher: ToolMatcher,
    pub effect: PolicyEffect,
    pub reason_code: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRuleId(pub String);

/// Three rule classes. MandatoryDeny cannot be weakened by config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleClass {
    /// Cannot be overridden by user/project config.
    /// Examples: unknown tool effect, malformed args, policy evaluation failure.
    MandatoryDeny,

    /// Can be overridden by user/project config.
    /// Examples: read auto-approve, write needs approval.
    BuiltinDefault,

    /// From user config (global or project).
    UserOverride,
}
```

### ToolMatcher

Compositional matching. Supports exact, prefix, effect, tag, and boolean combinators.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolMatcher {
    /// Matches any tool call
    Any,

    /// Matches by exact tool name
    ToolName { exact: String },

    /// Matches by tool name prefix (e.g., "file_" matches "file_read", "file_write")
    ToolNamePrefix { prefix: String },

    /// Matches by declared tool effect
    ToolEffect { effect: ToolEffect },

    /// Matches by tool tag
    ToolTag { tag: String },

    /// All matchers must match
    All { matchers: Vec<ToolMatcher> },

    /// At least one matcher must match
    AnyOf { matchers: Vec<ToolMatcher> },

    /// Negation
    Not { matcher: Box<ToolMatcher> },
}

impl ToolMatcher {
    pub fn matches(&self, call: &PolicyToolCall, descriptor: Option<&PolicyToolDescriptor>) -> bool {
        match self {
            Self::Any => true,
            Self::ToolName { exact } => call.name == *exact,
            Self::ToolNamePrefix { prefix } => call.name.starts_with(prefix),
            Self::ToolEffect { effect } => call.declared_effect == *effect,
            Self::ToolTag { tag } => descriptor
                .map(|d| d.tags.contains(tag))
                .unwrap_or(false),
            Self::All { matchers } => matchers
                .iter()
                .all(|m| m.matches(call, descriptor)),
            Self::AnyOf { matchers } => matchers
                .iter()
                .any(|m| m.matches(call, descriptor)),
            Self::Not { matcher } => !matcher.matches(call, descriptor),
        }
    }
}
```

### PolicyEffect

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyEffect {
    Allow {
        risk: RiskLevelSnapshot,
        confirmation: ConfirmationLevel,
    },
    Block,
}
```

---

## Gate Evaluation

### Algorithm

1. Collect all rules whose matcher matches the tool call
2. Evaluate each matching rule, producing a `GateFinding`
3. Apply block dominance: if any finding is Block, the final decision is Block
4. Otherwise, aggregate risk from all findings (take maximum)
5. Apply mode floor to confirmation level
6. Construct `PolicyEvaluation`

```rust
// eval.rs (conceptual — lives inside builtin.rs or a dedicated module)

fn evaluate_rules(
    rules: &[PolicyRule],
    request: &PolicyRequest,
    tool_descriptor: Option<&PolicyToolDescriptor>,
) -> Vec<GateFinding> {
    let mut findings = Vec::new();

    for rule in rules {
        if !rule.enabled {
            continue;
        }

        if rule.matcher.matches(&request.tool_call, tool_descriptor) {
            let result = match &rule.effect {
                PolicyEffect::Allow { risk, .. } => GateFindingResult::Require { risk: risk.clone() },
                PolicyEffect::Block => GateFindingResult::Block,
            };

            findings.push(GateFinding {
                rule_id: Some(rule.id.clone()),
                result,
                reason: rule.summary.clone(),
            });
        }
    }

    findings
}

fn finalize_decision(
    findings: Vec<GateFinding>,
    matched_rules: Vec<PolicyRuleId>,
    mode: &InteractionMode,
) -> PolicyEvaluation {
    let gate_id = GateId::new();

    // Block dominance: any Block finding → final Block
    let blocked_findings: Vec<_> = findings.iter()
        .filter(|f| matches!(f.result, GateFindingResult::Block))
        .collect();

    if !blocked_findings.is_empty() {
        let reasons: Vec<String> = blocked_findings.iter()
            .map(|f| f.reason.clone())
            .collect();
        return PolicyEvaluation {
            gate_id,
            decision: GateDecision::Block {
                reason: reasons.join("; "),
            },
            risk_level: RiskLevelSnapshot::Critical,
            confirmation_level: ConfirmationLevel::Escalate,
            findings,
            matched_rules,
            reason_code: "blocked_by_rule".into(),
            summary: format!("Blocked: {}", reasons.join("; ")),
            rollback_required: false,
            rollback_plan: None,
        };
    }

    // Aggregate risk: maximum risk from all findings
    let max_risk = findings.iter()
        .filter_map(|f| match &f.result {
            GateFindingResult::Require { risk } => Some(risk),
            _ => None,
        })
        .max_by(|a, b| risk_order(a).cmp(&risk_order(b)))
        .cloned()
        .unwrap_or(RiskLevelSnapshot::Low);

    // If all findings are Allow, the decision is Allow
    let all_allow = findings.iter().all(|f| matches!(f.result, GateFindingResult::Allow));

    if all_allow && findings.is_empty() {
        // No rules matched → fail-closed: block unknown
        return PolicyEvaluation {
            gate_id,
            decision: GateDecision::Block {
                reason: "No policy rule matched; defaulting to block.".into(),
            },
            risk_level: RiskLevelSnapshot::Critical,
            confirmation_level: ConfirmationLevel::Escalate,
            findings,
            matched_rules,
            reason_code: "no_matching_rule".into(),
            summary: "No policy rule matched this tool call.".into(),
            rollback_required: false,
            rollback_plan: None,
        };
    }

    let base_confirmation = confirmation_for_risk(&max_risk);
    let confirmation = apply_mode_floor(mode, &max_risk, &base_confirmation);

    let decision = match confirmation {
        ConfirmationLevel::Auto => GateDecision::Allow,
        other => GateDecision::RequireConfirmation { level: other },
    };

    PolicyEvaluation {
        gate_id,
        decision,
        risk_level: max_risk,
        confirmation_level: confirmation,
        findings,
        matched_rules,
        reason_code: "evaluated".into(),
        summary: format!("Policy evaluation: {:?}", decision),
        rollback_required: confirmation == ConfirmationLevel::Escalate,
        rollback_plan: None,
    }
}

fn risk_order(risk: &RiskLevelSnapshot) -> u8 {
    match risk {
        RiskLevelSnapshot::Low => 0,
        RiskLevelSnapshot::Medium => 1,
        RiskLevelSnapshot::High => 2,
        RiskLevelSnapshot::Critical => 3,
    }
}
```

### Mode Floor

`InteractionMode` adjusts confirmation behavior, never risk assessment.

```rust
// mapping.rs

/// Base confirmation level for a given risk level.
/// This is the policy's recommendation before mode adjustment.
pub fn confirmation_for_risk(risk: &RiskLevelSnapshot) -> ConfirmationLevel {
    match risk {
        RiskLevelSnapshot::Low => ConfirmationLevel::Auto,
        RiskLevelSnapshot::Medium => ConfirmationLevel::Inform,
        RiskLevelSnapshot::High => ConfirmationLevel::Approve,
        RiskLevelSnapshot::Critical => ConfirmationLevel::Escalate,
    }
}

/// Apply the mode floor. InteractionMode can only raise confirmation, never lower it.
pub fn apply_mode_floor(
    mode: &InteractionMode,
    risk: &RiskLevelSnapshot,
    base: &ConfirmationLevel,
) -> ConfirmationLevel {
    match mode {
        InteractionMode::Direct => base.clone(),
        InteractionMode::AutoRouting => base.clone(),
        InteractionMode::Conversational => {
            // Conversational mode floors Low to Inform
            match risk {
                RiskLevelSnapshot::Low => ConfirmationLevel::Inform,
                _ => base.clone(),
            }
        }
        InteractionMode::Custom { .. } => base.clone(),
    }
}
```

---

## Built-in Rules

### Batch 1 Rules

```rust
// builtin.rs

pub fn batch1_builtin_rules() -> Vec<PolicyRule> {
    vec![
        // ── MandatoryDeny ──

        PolicyRule {
            id: PolicyRuleId("mandatory_unknown_effect".into()),
            name: "Block unknown tool effects".into(),
            enabled: true,
            priority: 200,
            class: RuleClass::MandatoryDeny,
            matcher: ToolMatcher::ToolEffect { effect: ToolEffect::Unknown },
            effect: PolicyEffect::Block,
            reason_code: "unknown_tool_effect".into(),
            summary: "Tool effect is unknown; blocked for safety.".into(),
        },

        // ── BuiltinDefault: reads ──

        PolicyRule {
            id: PolicyRuleId("allow_read".into()),
            name: "Allow read-only tools".into(),
            enabled: true,
            priority: 100,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect { effect: ToolEffect::Read },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Low,
                confirmation: ConfirmationLevel::Auto,
            },
            reason_code: "read_only_tool".into(),
            summary: "Read-only tool call allowed.".into(),
        },

        PolicyRule {
            id: PolicyRuleId("allow_search".into()),
            name: "Allow search tools".into(),
            enabled: true,
            priority: 100,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect { effect: ToolEffect::Search },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Low,
                confirmation: ConfirmationLevel::Auto,
            },
            reason_code: "search_tool".into(),
            summary: "Search tool call allowed.".into(),
        },

        // ── BuiltinDefault: writes (blocked in Batch 1) ──

        PolicyRule {
            id: PolicyRuleId("batch1_block_writes".into()),
            name: "Block write tools (Batch 1)".into(),
            enabled: true,
            priority: 90,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::AnyOf {
                matchers: vec![
                    ToolMatcher::ToolEffect { effect: ToolEffect::Write },
                    ToolMatcher::ToolEffect { effect: ToolEffect::Delete },
                    ToolMatcher::ToolEffect { effect: ToolEffect::Execute },
                    ToolMatcher::ToolEffect { effect: ToolEffect::Network },
                    ToolMatcher::ToolEffect { effect: ToolEffect::Git },
                    ToolMatcher::ToolEffect { effect: ToolEffect::DependencyChange },
                    ToolMatcher::ToolEffect { effect: ToolEffect::PolicyChange },
                    ToolMatcher::ToolEffect { effect: ToolEffect::PersistenceChange },
                    ToolMatcher::ToolEffect { effect: ToolEffect::AuthChange },
                ],
            },
            effect: PolicyEffect::Block,
            reason_code: "batch1_writes_disabled".into(),
            summary: "Batch 1 policy blocks non-read tool calls.".into(),
        },
    ]
}
```

### Batch 2 Expansion (for reference, not implemented yet)

```text
Write (new file)          → Allow + Medium + Inform
Write (modify existing)   → RequireConfirmation + High + Approve
Delete                    → RequireConfirmation + Critical + Escalate
Execute (shell)           → RequireConfirmation + Critical + Escalate
Network                   → RequireConfirmation + High + Approve
Git mutation              → RequireConfirmation + High + Approve
DependencyChange          → RequireConfirmation + Critical + Escalate
PolicyChange              → Block (mandatory — policy mutations go through separate gate)
PersistenceChange         → RequireConfirmation + High + Approve
AuthChange                → Block (mandatory — auth changes require separate flow)
```

---

## Risk Assessment

### v1: Static Risk from ToolEffect

For Batch 1, risk is entirely determined by `ToolEffect`:

| Effect | Risk | Confirmation | Batch 1 Decision |
|---|---|---|---|
| Read | Low | Auto | Allow |
| Search | Low | Auto | Allow |
| Write | Medium | Inform | **Blocked** (Batch 1) |
| Delete | Critical | Escalate | **Blocked** |
| Execute | Critical | Escalate | **Blocked** |
| Network | High | Approve | **Blocked** |
| Git | High | Approve | **Blocked** |
| DependencyChange | Critical | Escalate | **Blocked** |
| PolicyChange | Critical | Block | **Blocked** (MandatoryDeny) |
| PersistenceChange | High | Approve | **Blocked** |
| AuthChange | Critical | Block | **Blocked** (MandatoryDeny) |
| Unknown | Critical | Block | **Blocked** (MandatoryDeny) |

### v2: Dynamic Risk with Argument Modifiers

```rust
// risk.rs (v2 expansion, not yet implemented)

pub trait RiskAssessor: Send + Sync {
    fn assess(&self, request: &PolicyRequest) -> RiskAssessment;
}

pub struct RiskAssessment {
    pub level: RiskLevelSnapshot,
    pub reasons: Vec<RiskReason>,
    pub confirmation: ConfirmationLevel,
    pub rollback_required: bool,
}

pub struct RiskReason {
    pub source: String,
    pub description: String,
    pub risk_delta: i8,
}
```

Future argument-aware modifiers:

```text
path_in_dotfiles       → +1
path_in_ssh            → +2
path_in_tmp            → -1
touches_3_plus_files   → +1
touches_cargo_toml     → +2
command_has_pipe       → +1
command_has_redirect   → +1
network_external       → +2
```

---

## Scope Stack

```rust
// scope.rs

/// Cascading rule resolution: mandatory → builtin → global user → project.
/// MandatoryDeny rules cannot be weakened.
/// UserOverride rules can override BuiltinDefault rules.
pub struct ScopeStack {
    rules: Vec<PolicyRule>,
}

impl ScopeStack {
    pub fn new(
        builtin: Vec<PolicyRule>,
        global_overrides: Vec<PolicyRule>,
        project_overrides: Vec<PolicyRule>,
    ) -> Self {
        let mut rules = Vec::new();

        // 1. Mandatory deny rules — always present, cannot be removed
        rules.extend(builtin.iter().filter(|r| r.class == RuleClass::MandatoryDeny).cloned());

        // 2. User overrides — can override BuiltinDefault rules
        rules.extend(global_overrides);
        rules.extend(project_overrides);

        // 3. Builtin defaults — skipped if user override exists for same tool
        for rule in builtin.iter().filter(|r| r.class == RuleClass::BuiltinDefault) {
            let has_override = rules.iter().any(|r| {
                r.class == RuleClass::UserOverride && rule_applies_to_same_tools(r, rule)
            });
            if !has_override {
                rules.push(rule.clone());
            }
        }

        // Sort by priority (higher first)
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        Self { rules }
    }

    pub fn batch1() -> Self {
        Self::new(batch1_builtin_rules(), vec![], vec![])
    }

    pub fn rules(&self) -> &[PolicyRule] {
        &self.rules
    }
}
```

---

## BuiltinPolicyEngine

```rust
// builtin.rs

pub struct BuiltinPolicyEngine {
    scope: ScopeStack,
    tool_registry: RwLock<Vec<PolicyToolDescriptor>>,
}

impl BuiltinPolicyEngine {
    pub fn batch1() -> Self {
        Self {
            scope: ScopeStack::batch1(),
            tool_registry: RwLock::new(Vec::new()),
        }
    }

    pub fn with_overrides(
        global_overrides: Vec<PolicyRule>,
        project_overrides: Vec<PolicyRule>,
    ) -> Self {
        Self {
            scope: ScopeStack::new(
                batch1_builtin_rules(),
                global_overrides,
                project_overrides,
            ),
            tool_registry: RwLock::new(Vec::new()),
        }
    }

    pub fn register_tool(&self, descriptor: PolicyToolDescriptor) {
        self.tool_registry.write().unwrap().push(descriptor);
    }
}

#[async_trait::async_trait]
impl PolicyEngine for BuiltinPolicyEngine {
    async fn evaluate_tool_call(
        &self,
        request: PolicyRequest,
    ) -> Result<PolicyEvaluation, PolicyError> {
        // Validate input
        if request.tool_call.name.is_empty() {
            return Ok(PolicyEvaluation::fail_closed(
                PolicyError::MalformedRequest("Tool name is empty".into()),
            ));
        }

        // Look up tool descriptor
        let registry = self.tool_registry.read().unwrap();
        let descriptor = registry.iter()
            .find(|d| d.name == request.tool_call.name);

        // Evaluate all matching rules
        let rules = self.scope.rules();
        let findings = evaluate_rules(rules, &request, descriptor);

        // Collect matched rule IDs
        let matched_rules = findings.iter()
            .filter_map(|f| f.rule_id.clone())
            .collect();

        // Finalize decision
        Ok(finalize_decision(findings, matched_rules, &request.mode))
    }

    async fn filter_tools(
        &self,
        request: ToolFilterRequest,
    ) -> Result<Vec<PolicyToolDescriptor>, PolicyError> {
        // Filter out blocked tools from the manifest
        let allowed: Vec<PolicyToolDescriptor> = request.tools.into_iter()
            .filter(|tool| {
                let fake_call = PolicyToolCall {
                    id: ToolCallId::new(),
                    name: tool.name.clone(),
                    arguments: serde_json::Value::Null,
                    declared_effect: tool.declared_effect.clone(),
                };
                let findings = evaluate_rules(
                    self.scope.rules(),
                    &PolicyRequest {
                        tool_call: fake_call,
                        mode: request.mode.clone(),
                        context: request.context.clone(),
                    },
                    Some(tool),
                );
                !findings.iter().any(|f| matches!(f.result, GateFindingResult::Block))
            })
            .collect();
        Ok(allowed)
    }
}
```

---

## Error Types

```rust
// error.rs

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("Malformed policy request: {0}")]
    MalformedRequest(String),

    #[error("Rule evaluation error: {0}")]
    EvaluationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl PolicyError {
    /// Safe message for inclusion in trace events.
    /// No internal paths or secrets.
    pub fn safe_message(&self) -> String {
        match self {
            Self::MalformedRequest(msg) => format!("Malformed request: {}", msg),
            Self::EvaluationError(msg) => format!("Evaluation error: {}", msg),
            Self::ConfigError(msg) => format!("Config error: {}", msg),
        }
    }
}
```

---

## Session Integration

Session calls policy through the trait. Session adapts its types to policy DTOs:

```rust
// In openwand-session (not in this crate)

let policy_request = PolicyRequest {
    tool_call: PolicyToolCall {
        id: call.id.clone(),
        name: call.name.clone(),
        arguments: call.arguments.clone(),
        declared_effect: tool_descriptor_to_effect(&call.name, &descriptor),
    },
    mode: self.config.mode.clone(),
    context: PolicyContext {
        working_directory: self.config.working_directory.clone(),
        model: self.config.model(),
        session_id: self.session_id.clone(),
        recent_gate_history: vec![],
    },
};

// Fail-closed: wrap evaluation in fail_closed()
let evaluation = match self.policy.evaluate_tool_call(policy_request).await {
    Ok(eval) => eval,
    Err(e) => PolicyEvaluation::fail_closed(e),
};

// Session constructs the trace event
self.apply_mutation(
    OpenWandTraceEvent::Gate(GateEvent::Evaluated {
        gate_id: evaluation.gate_id.to_string(),
        gate_kind: "tool_gate".into(),
        passed: evaluation.decision.allows_execution(),
        risk_level: Some(evaluation.risk_level),
        reason_code: Some(evaluation.reason_code),
        summary: evaluation.summary,
    }),
    vec![],
    None,
).await.ok();
```

---

## Approval Grants (v2 concept, not implemented)

Approval grants are runtime decisions, separate from policy rules:

```rust
/// Runtime approval grant — NOT a policy rule.
/// Lives in openwand-session, not openwand-policy.
pub struct ApprovalGrant {
    pub approval_request_id: ApprovalRequestId,
    pub tool_call_id: ToolCallId,
    pub expires_at: DateTime<Utc>,
    pub scope: ApprovalGrantScope,
}

pub enum ApprovalGrantScope {
    SingleCall,
    Tool { name: String, duration: Duration },
    Session,
}
```

Policy rules are durable governance. Approval grants are runtime decisions. Mixing them makes audit and rollback harder.

---

## Test Plan

### Batch 1 Tests

```rust
#[tokio::test]
async fn batch1_allows_read_tool() {
    let engine = BuiltinPolicyEngine::batch1();
    engine.register_tool(PolicyToolDescriptor {
        name: "read_file".into(),
        source: PolicyToolSource::Local,
        declared_effect: ToolEffect::Read,
        risk_hints: vec![],
        tags: vec!["filesystem".into()],
    });

    let result = engine.evaluate_tool_call(read_file_request()).await.unwrap();
    assert!(result.decision.allows_execution());
    assert_eq!(result.risk_level, RiskLevelSnapshot::Low);
    assert_eq!(result.confirmation_level, ConfirmationLevel::Auto);
}

#[tokio::test]
async fn batch1_blocks_write_tool() {
    let engine = BuiltinPolicyEngine::batch1();
    engine.register_tool(PolicyToolDescriptor {
        name: "write_file".into(),
        source: PolicyToolSource::Local,
        declared_effect: ToolEffect::Write,
        risk_hints: vec![],
        tags: vec!["filesystem".into()],
    });

    let result = engine.evaluate_tool_call(write_file_request()).await.unwrap();
    assert!(result.decision.is_blocked());
}

#[tokio::test]
async fn unknown_effect_blocks() {
    let engine = BuiltinPolicyEngine::batch1();
    let result = engine.evaluate_tool_call(unknown_tool_request()).await.unwrap();
    assert!(result.decision.is_blocked());
    assert_eq!(result.risk_level, RiskLevelSnapshot::Critical);
}

#[tokio::test]
async fn fail_closed_on_error() {
    let error = PolicyError::EvaluationError("test failure".into());
    let evaluation = PolicyEvaluation::fail_closed(error);
    assert!(evaluation.decision.is_blocked());
    assert_eq!(evaluation.risk_level, RiskLevelSnapshot::Critical);
}

#[tokio::test]
async fn no_matching_rule_blocks() {
    let engine = BuiltinPolicyEngine::batch1();
    // A tool with a known effect but no registered descriptor
    let result = engine.evaluate_tool_call(orphan_tool_request()).await.unwrap();
    assert!(result.decision.is_blocked());
}

#[tokio::test]
async fn mandatory_deny_cannot_be_overridden() {
    let engine = BuiltinPolicyEngine::with_overrides(
        vec![PolicyRule {
            id: PolicyRuleId("override_unknown".into()),
            name: "Allow unknown".into(),
            enabled: true,
            priority: 300,
            class: RuleClass::UserOverride,
            matcher: ToolMatcher::ToolEffect { effect: ToolEffect::Unknown },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Low,
                confirmation: ConfirmationLevel::Auto,
            },
            reason_code: "user_override".into(),
            summary: "User wants to allow unknown tools.".into(),
        }],
        vec![],
    );
    let result = engine.evaluate_tool_call(unknown_tool_request()).await.unwrap();
    // MandatoryDeny rule still blocks despite user override
    assert!(result.decision.is_blocked());
}

#[tokio::test]
async fn filter_tools_hides_blocked_tools() {
    let engine = BuiltinPolicyEngine::batch1();
    let tools = vec![
        make_descriptor("read_file", ToolEffect::Read),
        make_descriptor("write_file", ToolEffect::Write),
        make_descriptor("delete_file", ToolEffect::Delete),
    ];
    let result = engine.filter_tools(ToolFilterRequest {
        tools,
        mode: InteractionMode::Direct,
        context: test_context(),
    }).await.unwrap();
    let names: Vec<&str> = result.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, &["read_file"]);
}

#[tokio::test]
async fn mode_floor_conversational_low_becomes_inform() {
    let level = apply_mode_floor(
        &InteractionMode::Conversational,
        &RiskLevelSnapshot::Low,
        &ConfirmationLevel::Auto,
    );
    assert_eq!(level, ConfirmationLevel::Inform);
}
```

---

## Summary

| Aspect | v1 (Batch 1) | v2+ |
|---|---|---|
| Rules | Built-in: allow reads/search, block everything else | User/project config in TOML, argument-aware modifiers |
| Risk model | Static per ToolEffect | Dynamic: base effect + argument modifiers |
| Gates | Structural only | Structural + LLM-assisted semantic |
| Scope | Global built-ins only | Global user → project overrides |
| Filtering | Hide blocked tools | Mode-aware + context-aware filtering |
| Rate limits | None | Separate gate family |
| Approval grants | None (session handles via Loro) | Explicit grant objects with expiry |
| Rule format | Rust structs | Rust structs + TOML config files |

**Cargo.toml:**

```toml
[package]
name = "openwand-policy"
version.workspace = true
edition.workspace = true

[dependencies]
openwand-core = { path = "../core" }
async-trait = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tokio = { workspace = true, features = ["sync"] }
```
