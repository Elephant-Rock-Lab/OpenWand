//! Shared session runtime assembly.
//!
//! Both `cmd_run` and the eval runner build sessions the same way.
//! This module extracts the shared assembly to prevent forked implementations.

use openwand_core::SessionId;
use openwand_llm::adapters::openai_compatible::OpenAiCompatibleClient;
use openwand_llm::LlmClient;
use openwand_llm::provider_config::ProviderTargetConfig;
use openwand_llm::provider_registry::ProviderRegistry;
use openwand_memory::{MemoryReadStore, MemoryStore, SqliteMemoryStore};
use openwand_policy::{BuiltinPolicyEngine, PolicyEngine};
use openwand_session::runner::SessionRunner;
use openwand_store::backends::sqlite::{SqliteStore, SqliteStoreConfig};
use openwand_store::StoredEvent;
use openwand_tools::composite::CompositeToolExecutor;
use openwand_tools::executor::ToolExecutor;
use openwand_trace::TraceStore;
use std::path::Path;
use std::sync::Arc;

/// Assembled session runtime — all components wired and ready.
pub struct SessionRuntime {
    pub runner: SessionRunner,
    pub trace: Arc<dyn TraceStore<StoredEvent>>,
    pub trace_for_coordinator: Arc<dyn TraceStore<StoredEvent>>,
    pub memory_store: Arc<dyn MemoryStore>,
    pub memory_read: Arc<dyn MemoryReadStore>,
    pub session_id: SessionId,
}

/// Build the complete session runtime from a database path and working directory.
///
/// This is the single source of truth for session assembly.
/// Both the CLI `run` command and the eval runner call this function.
pub async fn build_session_runtime(
    db_path: &str,
    working_directory: &str,
) -> anyhow::Result<SessionRuntime> {
    // 1. Open SQLite store (trace + registry)
    let store = SqliteStore::open(SqliteStoreConfig::file(db_path)).await?;
    let trace: Arc<dyn TraceStore<StoredEvent>> = Arc::new(store);

    // 2. Open SQLite memory store (same file, own migrations)
    let memory_store = SqliteMemoryStore::open(Path::new(db_path))?;
    let memory_read: Arc<dyn MemoryReadStore> = Arc::new(memory_store);

    // 3. Create LLM client
    let llm: Arc<dyn LlmClient> = Arc::new(OpenAiCompatibleClient::new());

    // 4. Create tools executor
    let tools: Arc<dyn ToolExecutor> = Arc::new(
        CompositeToolExecutor::local_only(openwand_tools::local::batch2_local_tools())
    );

    // 5. Create policy engine
    let policy: Arc<dyn PolicyEngine> = Arc::new(build_write_policy());

    // 6. Second trace store connection for coordinator
    let trace_for_coordinator: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        SqliteStore::open(SqliteStoreConfig::file(db_path)).await?
    );

    // 7. Memory store with write access for coordinator
    let memory_for_coordinator: Arc<dyn MemoryStore> = Arc::new(
        SqliteMemoryStore::open(Path::new(db_path))?
    );

    // 8. Create session runner
    let session_id = SessionId::new();
    let runner = SessionRunner::new(
        session_id.clone(),
        trace,
        llm,
        tools,
        policy,
        memory_read.clone(),
        working_directory.to_string(),
    );

    Ok(SessionRuntime {
        runner,
        trace: trace_for_coordinator.clone(),
        trace_for_coordinator,
        memory_store: memory_for_coordinator,
        memory_read,
        session_id,
    })
}

/// Build session runtime using a provider registry.
/// Resolves the provider from the registry by target ID.
pub async fn build_session_runtime_with_provider(
    db_path: &str,
    working_directory: &str,
    provider_configs: Vec<ProviderTargetConfig>,
    target_id: &str,
) -> anyhow::Result<SessionRuntime> {
    let registry = ProviderRegistry::new(provider_configs);
    let llm = registry.build_client(target_id)?;

    // 1. Open SQLite store (trace + registry)
    let store = SqliteStore::open(SqliteStoreConfig::file(db_path)).await?;
    let trace: Arc<dyn TraceStore<StoredEvent>> = Arc::new(store);

    // 2. Open SQLite memory store
    let memory_store = SqliteMemoryStore::open(Path::new(db_path))?;
    let memory_read: Arc<dyn MemoryReadStore> = Arc::new(memory_store);

    // 3. Create tools executor
    let tools: Arc<dyn ToolExecutor> = Arc::new(
        CompositeToolExecutor::local_only(openwand_tools::local::batch2_local_tools())
    );

    // 4. Create policy engine
    let policy: Arc<dyn PolicyEngine> = Arc::new(build_write_policy());

    // 5. Second trace store connection for coordinator
    let trace_for_coordinator: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        SqliteStore::open(SqliteStoreConfig::file(db_path)).await?
    );

    // 6. Memory store with write access for coordinator
    let memory_for_coordinator: Arc<dyn MemoryStore> = Arc::new(
        SqliteMemoryStore::open(Path::new(db_path))?
    );

    // 7. Create session runner
    let session_id = SessionId::new();
    let runner = SessionRunner::new(
        session_id.clone(),
        trace,
        llm,
        tools,
        policy,
        memory_read.clone(),
        working_directory.to_string(),
    );

    Ok(SessionRuntime {
        runner,
        trace: trace_for_coordinator.clone(),
        trace_for_coordinator,
        memory_store: memory_for_coordinator,
        memory_read,
        session_id,
    })
}

/// Build the write policy — shared between run and eval paths.
pub fn build_write_policy() -> BuiltinPolicyEngine {
    BuiltinPolicyEngine::new(vec![
        openwand_policy::PolicyRule {
            id: openwand_policy::PolicyRuleId("allow-read".into()),
            name: "Allow read-effect tools".into(),
            enabled: true,
            priority: 0,
            class: openwand_policy::RuleClass::BuiltinDefault,
            matcher: openwand_policy::ToolMatcher::ToolEffect {
                effect: openwand_core::tool_vocab::ToolEffect::Read,
            },
            effect: openwand_policy::PolicyEffect::Allow {
                risk: openwand_core::risk::RiskLevelSnapshot::Low,
                confirmation: openwand_core::mode::ConfirmationLevel::Auto,
            },
            reason_code: "allow_read".into(),
            summary: "Allow read-effect tool calls.".into(),
        },
        openwand_policy::PolicyRule {
            id: openwand_policy::PolicyRuleId("allow-search".into()),
            name: "Allow search-effect tools".into(),
            enabled: true,
            priority: 0,
            class: openwand_policy::RuleClass::BuiltinDefault,
            matcher: openwand_policy::ToolMatcher::ToolEffect {
                effect: openwand_core::tool_vocab::ToolEffect::Search,
            },
            effect: openwand_policy::PolicyEffect::Allow {
                risk: openwand_core::risk::RiskLevelSnapshot::Low,
                confirmation: openwand_core::mode::ConfirmationLevel::Auto,
            },
            reason_code: "allow_search".into(),
            summary: "Allow search-effect tool calls.".into(),
        },
        // Write requires explicit approval
        openwand_policy::PolicyRule {
            id: openwand_policy::PolicyRuleId("write-requires-approve".into()),
            name: "Write-effect tools require user approval".into(),
            enabled: true,
            priority: 0,
            class: openwand_policy::RuleClass::BuiltinDefault,
            matcher: openwand_policy::ToolMatcher::ToolEffect {
                effect: openwand_core::tool_vocab::ToolEffect::Write,
            },
            effect: openwand_policy::PolicyEffect::Allow {
                risk: openwand_core::risk::RiskLevelSnapshot::Medium,
                confirmation: openwand_core::mode::ConfirmationLevel::Approve,
            },
            reason_code: "write_requires_approval".into(),
            summary: "Write-effect tools require explicit user approval.".into(),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_write_policy_has_read_search_write_rules() {
        let policy = build_write_policy();
        // Should have at least 3 rules
        assert!(policy.rules().len() >= 3, "Expected >= 3 rules");
    }

    #[test]
    fn build_write_policy_has_read_effect_rule() {
        let policy = build_write_policy();
        let has_read = policy.rules().iter().any(|r| {
            matches!(r.matcher, openwand_policy::ToolMatcher::ToolEffect {
                effect: openwand_core::tool_vocab::ToolEffect::Read,
            })
        });
        assert!(has_read, "No read rule found");
    }

    #[test]
    fn build_write_policy_has_write_approval_rule() {
        let policy = build_write_policy();
        let has_write = policy.rules().iter().any(|r| {
            matches!(r.matcher, openwand_policy::ToolMatcher::ToolEffect {
                effect: openwand_core::tool_vocab::ToolEffect::Write,
            })
        });
        assert!(has_write, "No write rule found");
    }
}
