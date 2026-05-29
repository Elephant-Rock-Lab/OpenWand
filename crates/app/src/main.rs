//! OpenWand — Conjure results from intent.
//!
//! Wave 02g: CLI binary with real memory wiring.

use anyhow::Result;
use clap::Parser;
use openwand_app::memory_coordinator::{MemoryCoordinator, PromptInputProductionConfig};
use openwand_core::SessionId;
use openwand_llm::adapters::openai_compatible::OpenAiCompatibleClient;
use openwand_llm::LlmClient;
use openwand_memory::testing::HeuristicExtractor;
use openwand_memory::{MemoryExtractor, MemoryReadStore, MemoryStore, SqliteMemoryStore};
use openwand_policy::{BuiltinPolicyEngine, PolicyEngine};
use openwand_session::config::{RunConfig, RunStopReason, RunSummary};
use openwand_session::message::MessageContent;
use openwand_session::runner::{ApprovalDecision, SessionRunner};
use openwand_store::backends::sqlite::{SqliteStore, SqliteStoreConfig};
use openwand_store::StoredEvent;
use openwand_tools::composite::CompositeToolExecutor;
use openwand_tools::executor::ToolExecutor;
use openwand_trace::TraceStore;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "openwand", version, about = "Conjure results from intent")]
struct Args {
    /// LLM provider base URL (OpenAI-compatible)
    #[arg(long, default_value = "http://localhost:1234/v1")]
    base_url: String,

    /// Model name
    #[arg(long, default_value = "default")]
    model: String,

    /// API key (optional for local servers)
    #[arg(long)]
    api_key: Option<String>,

    /// Path to SQLite database
    #[arg(long, default_value = "openwand.db")]
    db: String,

    /// The user message to send
    message: Option<String>,
}

fn build_write_policy() -> BuiltinPolicyEngine {
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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // 1. Open SQLite store (trace + registry)
    let store = SqliteStore::open(SqliteStoreConfig::file(&args.db)).await?;
    let trace: Arc<dyn TraceStore<StoredEvent>> = Arc::new(store);

    // 2. Open SQLite memory store (same file, own migrations)
    let memory_store = SqliteMemoryStore::open(std::path::Path::new(&args.db))?;
    let memory_read: Arc<dyn MemoryReadStore> = Arc::new(memory_store);

    // 3. Create LLM client
    let llm: Arc<dyn LlmClient> = Arc::new(OpenAiCompatibleClient::new());

    // 4. Create tools executor (local tools including file_write)
    let tools: Arc<dyn ToolExecutor> = Arc::new(
        CompositeToolExecutor::local_only(openwand_tools::local::batch2_local_tools())
    );

    // 5. Create policy engine (Read/Search auto, Write requires approval)
    let policy: Arc<dyn PolicyEngine> = Arc::new(build_write_policy());

    // 6. Get user message
    let message = args.message.unwrap_or_else(|| {
        "Hello! Can you tell me a short joke?".to_string()
    });

    println!("╔══════════════════════════════════════════╗");
    println!("║          OpenWand Reality Smoke          ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("Provider: {}", args.base_url);
    println!("Model:    {}", args.model);
    println!("Database: {}", args.db);
    println!("Memory:   SQLite (same file)");
    println!();
    println!("User: {message}");
    println!("────────────────────────────────────────────");

    // 7. Create session runner
    let session_id = SessionId::new();

    // Need a second trace store connection for the coordinator
    let trace_for_coordinator: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        SqliteStore::open(SqliteStoreConfig::file(&args.db)).await?
    );

    let runner = SessionRunner::new(
        session_id.clone(),
        trace,
        llm,
        tools,
        policy,
        memory_read.clone(),
        std::env::current_dir()?.to_string_lossy().to_string(),
    );

    // 8. Configure run
    let mut run_config = RunConfig::default();
    run_config.mode = openwand_core::mode::InteractionMode::Conversational;
    run_config.llm_target = Some(openwand_llm::LlmTarget {
        provider: openwand_llm::LlmProvider::Custom {
            name: "lm-studio".into(),
        },
        model: args.model.clone(),
        base_url: Some(args.base_url.clone()),
        api_key: args.api_key.clone(),
    });
    let result = runner.run_turn(message.clone(), run_config.clone()).await?;

    // 8b. Handle approval flow
    let result = if matches!(result.stop_reason, RunStopReason::AwaitingApproval) {
        println!("────────────────────────────────────────────");
        if let Some(pending) = runner.pending_approval().await {
            println!("⚠ Tool '{}' requires your approval.", pending.tool_name);
            println!("  Reason: {}", pending.policy_summary);
            println!("  Approve? [y/N] ");

            // Read user input
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap_or_default();
            let approved = input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes";

            let decision = if approved {
                ApprovalDecision::approve()
            } else {
                ApprovalDecision::reject()
            };

            let approval_result = runner.resolve_approval(decision, run_config).await?;
            println!("  → {}", if approved { "Approved" } else { "Rejected" });
            if let Some(tool_result) = &approval_result.tool_result {
                println!("  Tool result: {}", tool_result.output);
            }
        }

        // Return a synthetic result
        RunSummary {
            stop_reason: RunStopReason::Natural,
            steps_completed: result.steps_completed,
            tools_executed: result.tools_executed + 1,
            recoverable: true,
        }
    } else {
        result
    };

    println!("────────────────────────────────────────────");
    println!("✓ Turn complete");
    println!("  Stop reason:   {:?}", result.stop_reason);
    println!("  Steps:         {}", result.steps_completed);
    println!("  Tools called:  {}", result.tools_executed);
    println!("  Recoverable:   {}", result.recoverable);

    // 9. Run memory projection after the turn
    let memory_for_coordinator: Arc<dyn MemoryStore> = {
        // Re-open memory store for write access through the store trait
        Arc::new(SqliteMemoryStore::open(std::path::Path::new(&args.db))?)
    };
    let extractor: Arc<dyn MemoryExtractor> = Arc::new(HeuristicExtractor);
    let coordinator = MemoryCoordinator::new(
        memory_for_coordinator,
        extractor,
        trace_for_coordinator,
    );

    let projection = coordinator.project_after_run(&session_id).await;
    println!();
    println!("Memory projection:");
    println!("  Episodes projected:  {}", projection.episodes_projected);
    println!("  Candidates extracted: {}", projection.candidates_extracted);
    println!("  Records accepted:    {}", projection.records_accepted);
    if !projection.errors.is_empty() {
        println!("  Errors: {:?}", projection.errors);
    }

    // 9b. Produce 02k prompt inputs (diagnostic — shows what the next turn would see)
    let prompt_result = coordinator
        .produce_prompt_inputs(
            Some(session_id.clone()),
            std::env::current_dir()?.as_path(),
            &PromptInputProductionConfig::default(),
        )
        .await;
    println!();
    println!("Prompt inputs:");
    println!("  Claims checked:  {}", prompt_result.claims_checked);
    println!("  Repo observed:   {}", prompt_result.repo_observed);
    if prompt_result.repo_observed {
        println!("  Supported:       {}", prompt_result.inputs.supported_claims.len());
        println!("  Unverifiable:    {}", prompt_result.inputs.unverifiable_claims_excluded.len());
        println!("  Missing gaps:    {}", prompt_result.inputs.missing_memory_gaps.len());
    }
    if !prompt_result.errors.is_empty() {
        println!("  Errors: {:?}", prompt_result.errors);
    }

    // 10. Show Loro projection
    let messages = runner.loro_state().messages().map_err(|e| anyhow::anyhow!("{e}"))?;
    println!();
    println!("Messages ({} total):", messages.len());
    for msg in &messages {
        let role = match msg.role {
            openwand_session::message::MessageRole::User => "👤 User",
            openwand_session::message::MessageRole::Assistant => "🤖 Assistant",
            openwand_session::message::MessageRole::Tool => "🔧 Tool",
        };
        let content_preview = match &msg.content {
            MessageContent::Text { text } => {
                if text.len() > 200 { format!("{}...", &text[..200]) } else { text.clone() }
            }
            MessageContent::ToolResult { result, is_error, .. } => {
                let icon = if *is_error { "❌" } else { "✅" };
                format!("{icon} {}", result.chars().take(100).collect::<String>())
            }
        };
        println!("  {role}: {content_preview}");
    }

    // 11. Show stale status
    let stale = runner.loro_state().projection_is_stale().map_err(|e| anyhow::anyhow!("{e}"))?;
    println!();
    if stale {
        println!("⚠ Loro projection is stale");
    } else {
        println!("✓ Loro projection is fresh");
    }

    Ok(())
}
