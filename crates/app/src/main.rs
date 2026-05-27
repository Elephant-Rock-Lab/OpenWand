//! OpenWand — Conjure results from intent.
//!
//! Wave 02a: CLI smoke test binary.
//! Composes all crates and runs one real LLM turn through the spine.

use anyhow::Result;
use clap::Parser;
use openwand_core::SessionId;
use openwand_llm::adapters::openai_compatible::OpenAiCompatibleClient;
use openwand_llm::LlmClient;
use openwand_memory::{MemoryError, MemoryQuery, MemoryReadStore, RetrievalContext};
use openwand_policy::{BuiltinPolicyEngine, PolicyEngine};
use openwand_session::config::RunConfig;
use openwand_session::message::MessageContent;
use openwand_session::runner::SessionRunner;
use openwand_store::backends::sqlite::{SqliteStore, SqliteStoreConfig};
use openwand_store::StoredEvent;
use openwand_tools::composite::CompositeToolExecutor;
use openwand_tools::executor::ToolExecutor;
use openwand_trace::TraceStore;
use async_trait::async_trait;
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

/// Stub memory store for smoke test — returns empty context.
struct StubMemoryStore;

#[async_trait]
impl MemoryReadStore for StubMemoryStore {
    async fn search(&self, _query: MemoryQuery) -> std::result::Result<RetrievalContext, MemoryError> {
        Ok(RetrievalContext::empty())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // 1. Open SQLite store
    let store = SqliteStore::open(SqliteStoreConfig::file(&args.db)).await?;
    let trace: Arc<dyn TraceStore<StoredEvent>> = Arc::new(store);

    // 2. Create LLM client
    let llm: Arc<dyn LlmClient> = Arc::new(OpenAiCompatibleClient::new());

    // 3. Create tools executor (local tools only, no MCP)
    let tools: Arc<dyn ToolExecutor> = Arc::new(
        CompositeToolExecutor::local_only(openwand_tools::local::batch1_local_tools())
    );

    // 4. Create policy engine (smoke-test profile: Read + Search only)
    let allow_read_rule = openwand_policy::PolicyRule {
        id: openwand_policy::PolicyRuleId("smoke-allow-read".into()),
        name: "Allow read-effect tools (smoke)".into(),
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
        reason_code: "smoke_allow_read".into(),
        summary: "Allow read-effect tool calls for smoke testing.".into(),
    };
    let allow_search_rule = openwand_policy::PolicyRule {
        id: openwand_policy::PolicyRuleId("smoke-allow-search".into()),
        name: "Allow search-effect tools (smoke)".into(),
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
        reason_code: "smoke_allow_search".into(),
        summary: "Allow search-effect tool calls for smoke testing.".into(),
    };
    let policy: Arc<dyn PolicyEngine> = Arc::new(BuiltinPolicyEngine::new(vec![
        allow_read_rule,
        allow_search_rule,
    ]));

    // 5. Create memory store (stub)
    let memory: Arc<dyn MemoryReadStore> = Arc::new(StubMemoryStore);

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
    println!();
    println!("User: {message}");
    println!("────────────────────────────────────────────");

    // 7. Create session runner
    let runner = SessionRunner::new(
        SessionId::new(),
        trace,
        llm,
        tools,
        policy,
        memory,
        std::env::current_dir()?.to_string_lossy().to_string(),
    );

    // 8. Run the turn
    let mut run_config = RunConfig::default();
    run_config.mode = openwand_core::mode::InteractionMode::Direct;
    run_config.llm_target = Some(openwand_llm::LlmTarget {
        provider: openwand_llm::LlmProvider::Custom {
            name: "lm-studio".into(),
        },
        model: args.model.clone(),
        base_url: Some(args.base_url.clone()),
        api_key: args.api_key.clone(),
    });
    let result = runner.run_turn(message, run_config).await?;

    println!("────────────────────────────────────────────");
    println!("✓ Turn complete");
    println!("  Stop reason:   {:?}", result.stop_reason);
    println!("  Steps:         {}", result.steps_completed);
    println!("  Tools called:  {}", result.tools_executed);
    println!("  Recoverable:   {}", result.recoverable);

    // 9. Show Loro projection
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

    // 10. Show stale status
    let stale = runner.loro_state().projection_is_stale().map_err(|e| anyhow::anyhow!("{e}"))?;
    println!();
    if stale {
        println!("⚠ Loro projection is stale");
    } else {
        println!("✓ Loro projection is fresh");
    }

    Ok(())
}
