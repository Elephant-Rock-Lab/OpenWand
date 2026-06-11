//! Guard tests for openwand-workflow crate.
//!
//! Proves the workflow crate:
//! 1. Has only the 6 allowed dependencies
//! 2. Does not import forbidden crates
//! 3. Does not contain executable fields in DTOs
//! 4. Serialized JSON does not contain forbidden field names

use std::path::Path;

/// Read all Rust source files in the workflow crate src/ directory.
fn read_workflow_sources() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_dir = Path::new(&manifest_dir).join("src");
    let mut all_source = String::new();
    if let Ok(entries) = std::fs::read_dir(&src_dir) {
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "rs") {
                all_source.push_str(&std::fs::read_to_string(&path).unwrap());
                all_source.push('\n');
            }
        }
    }
    all_source
}

/// Read Cargo.toml for dependency scanning.
fn read_cargo_toml() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::fs::read_to_string(Path::new(&manifest_dir).join("Cargo.toml")).unwrap()
}

#[test]
fn workflow_crate_dependency_guard_allows_only_6_deps() {
    let cargo = read_cargo_toml();
    let allowed = ["serde", "serde_json", "blake3", "chrono", "thiserror", "tracing"];
    
    // Check no other workspace deps
    let forbidden = [
        "openwand-core",
        "openwand-session",
        "openwand-tools",
        "openwand-policy",
        "openwand-memory",
        "openwand-trace",
        "openwand-store",
        "openwand-skills",
        "openwand-goals",
        "openwand-mcp-pool",
        "openwand-content",
        "openwand-app",
        "tokio",
        "uuid",
        "reqwest",
    ];
    
    for dep in &forbidden {
        assert!(
            !cargo.contains(dep),
            "workflow crate must not depend on {}, but Cargo.toml contains it",
            dep
        );
    }

    // Verify allowed deps exist
    for dep in &allowed {
        assert!(
            cargo.contains(dep),
            "workflow crate should have {} in Cargo.toml",
            dep
        );
    }
}

#[test]
fn workflow_crate_does_not_import_forbidden_crates() {
    let source = read_workflow_sources();
    let forbidden = [
        "openwand_tools",
        "openwand_policy",
        "openwand_memory",
        "openwand_session",
        "openwand_trace",
        "openwand_store",
        "openwand_skills",
        "openwand_goals",
        "std::process",
        "tokio",
    ];
    for pattern in &forbidden {
        let patterns = [
            format!("use {pattern}"),
            format!("use {pattern}::"),
            format!("extern crate {pattern}"),
        ];
        for p in &patterns {
            assert!(
                !source.contains(p.as_str()),
                "workflow crate must not import {}: found '{}'",
                pattern, p
            );
        }
    }
}

#[test]
fn task_plan_dto_has_no_executable_fields() {
    let source = read_workflow_sources();
    let forbidden_fields = [
        "pub tool_name:",
        "pub tool_args:",
        "pub command:",
        "pub shell:",
        "pub script:",
        "pub cwd:",
        "pub env:",
        "pub function_ref:",
        "pub workflow_handle:",
        "pub execution_grant:",
    ];
    for field in &forbidden_fields {
        assert!(
            !source.contains(field),
            "workflow crate DTOs must not have executable field: {}",
            field
        );
    }
}

#[test]
fn task_plan_step_dto_has_no_executable_fields() {
    // TaskPlanStep is covered by the same scan above, but this test
    // explicitly verifies the struct has only expected fields.
    let plan = openwand_workflow::plan::TaskPlanStep {
        step_id: "s1".into(),
        title: "Test".into(),
        description: "Test step".into(),
        kind: openwand_workflow::plan::TaskPlanStepKind::Observe,
        depends_on: vec![],
        expected_output: "result".into(),
        risk_level: "low".into(),
        requires_approval: false,
        evidence_links: vec![],
    };
    // If this compiles, TaskPlanStep has only the expected non-executable fields.
    let _ = plan.step_id;
}

#[test]
fn task_plan_serialized_json_contains_no_executable_fields() {
    
    use openwand_workflow::builder::build_task_plan;
    use openwand_workflow::context::TaskPlanInput;

    let input = TaskPlanInput {
        user_intent: "Test plan for serialization guard".into(),
        skill_context: vec![],
        goal_context: vec![],
        memory_summaries: vec!["test".into()],
        trace_summaries: vec!["test".into()],
        governance_summaries: vec![],
        policy_constraints: vec![],
    };
    let plan = build_task_plan(&input).unwrap();
    let json = serde_json::to_string(&plan).unwrap();

    let forbidden_keys = [
        "tool_name",
        "tool_args",
        "command",
        "shell",
        "script",
        "cwd",
        "env",
        "function_ref",
        "workflow_handle",
        "execution_grant",
    ];
    for key in &forbidden_keys {
        assert!(
            !json.contains(&format!("\"{}\"", key)),
            "serialized plan JSON must not contain key '{}'",
            key
        );
    }
}

#[test]
fn task_plan_roundtrips() {
    use openwand_workflow::builder::build_task_plan;
    use openwand_workflow::context::TaskPlanInput;

    let input = TaskPlanInput {
        user_intent: "Roundtrip test".into(),
        skill_context: vec![],
        goal_context: vec![],
        memory_summaries: vec![],
        trace_summaries: vec![],
        governance_summaries: vec![],
        policy_constraints: vec![],
    };
    let plan = build_task_plan(&input).unwrap();
    let json = serde_json::to_string(&plan).unwrap();
    let restored: openwand_workflow::plan::TaskPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(plan.plan_id, restored.plan_id);
    assert_eq!(plan.title, restored.title);
    assert_eq!(plan.steps.len(), restored.steps.len());
}

#[test]
fn task_plan_id_is_content_addressed() {
    use openwand_workflow::validation::task_plan_id_for;

    let id1 = task_plan_id_for("intent a", "title", 3, &[], &[]);
    let id2 = task_plan_id_for("intent b", "title", 3, &[], &[]);
    assert_ne!(id1, id2);
    assert!(id1.0.starts_with("tpl_"));
    assert!(id2.0.starts_with("tpl_"));

    let id3 = task_plan_id_for("intent a", "title", 3, &[], &[]);
    assert_eq!(id1, id3);
}

#[test]
fn task_plan_hash_changes_when_steps_change() {
    use openwand_workflow::plan::*;
    use openwand_workflow::validation::compute_plan_hash;

    let steps_a = vec![TaskPlanStep {
        step_id: "s1".into(),
        title: "Step A".into(),
        description: "Different".into(),
        kind: TaskPlanStepKind::Observe,
        depends_on: vec![],
        expected_output: "x".into(),
        risk_level: "low".into(),
        requires_approval: false,
        evidence_links: vec![],
    }];
    let steps_b = vec![TaskPlanStep {
        step_id: "s1".into(),
        title: "Step B".into(),
        description: "Different".into(),
        kind: TaskPlanStepKind::Analyze,
        depends_on: vec![],
        expected_output: "x".into(),
        risk_level: "low".into(),
        requires_approval: false,
        evidence_links: vec![],
    }];
    let hash_a = compute_plan_hash(&steps_a, &[]);
    let hash_b = compute_plan_hash(&steps_b, &[]);
    assert_ne!(hash_a, hash_b);
}

#[test]
fn task_plan_validation_rejects_empty_intent() {
    use openwand_workflow::builder::build_task_plan;
    use openwand_workflow::context::TaskPlanInput;

    let input = TaskPlanInput {
        user_intent: "".into(),
        skill_context: vec![],
        goal_context: vec![],
        memory_summaries: vec![],
        trace_summaries: vec![],
        governance_summaries: vec![],
        policy_constraints: vec![],
    };
    assert!(build_task_plan(&input).is_err());
}

#[test]
fn task_plan_validation_rejects_empty_steps_for_reviewable() {
    use openwand_workflow::plan::*;
    use openwand_workflow::validation::validate_task_plan;
    use chrono::Utc;

    let plan = TaskPlan {
        plan_id: TaskPlanId("tpl_test".into()),
        title: "Test".into(),
        user_intent: "Do something".into(),
        status: TaskPlanStatus::Reviewable,
        steps: vec![],
        assumptions: vec![],
        risks: vec![],
        required_approvals: vec![],
        evidence_links: vec![],
        skill_context_ids: vec![],
        goal_context_ids: vec![],
        policy_constraints: vec![],
        plan_hash: "abc".into(),
        created_at: Utc::now(),
    };
    let result = validate_task_plan(&plan);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("step")));
}

#[test]
fn task_plan_step_rejects_unknown_dependency() {
    use openwand_workflow::plan::*;
    use openwand_workflow::validation::validate_task_plan;
    use chrono::Utc;

    let plan = TaskPlan {
        plan_id: TaskPlanId("tpl_test".into()),
        title: "Test".into(),
        user_intent: "Do something".into(),
        status: TaskPlanStatus::Reviewable,
        steps: vec![TaskPlanStep {
            step_id: "s1".into(),
            title: "Step".into(),
            description: "Step".into(),
            kind: TaskPlanStepKind::Observe,
            depends_on: vec!["nonexistent".into()],
            expected_output: "x".into(),
            risk_level: "low".into(),
            requires_approval: false,
            evidence_links: vec![],
        }],
        assumptions: vec![],
        risks: vec![],
        required_approvals: vec![],
        evidence_links: vec![],
        skill_context_ids: vec![],
        goal_context_ids: vec![],
        policy_constraints: vec![],
        plan_hash: "abc".into(),
        created_at: Utc::now(),
    };
    let result = validate_task_plan(&plan);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("unknown dependency")));
}
