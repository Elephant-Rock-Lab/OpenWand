//! OpenWand — Conjure results from intent.
//!
//! Wave 05: CLI binary with subcommand structure.

use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
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
struct Cli {
    /// LLM provider base URL (OpenAI-compatible)
    #[arg(long, global = true, default_value = "http://localhost:1234/v1")]
    base_url: String,

    /// Model name
    #[arg(long, global = true, default_value = "default")]
    model: String,

    /// API key (optional for local servers)
    #[arg(long, global = true)]
    api_key: Option<String>,

    /// Path to SQLite database
    #[arg(long, global = true, default_value = "openwand.db")]
    db: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run an agent turn (default when no subcommand given)
    Run {
        /// The user message to send
        message: Option<String>,
    },

    /// Explain why a session produced its results
    Explain {
        /// Session ID to explain
        session_id: String,
    },

    /// Verify trace integrity for a session
    #[command(name = "trace-verify")]
    TraceVerify {
        /// Session ID to verify
        session_id: String,
    },

    /// Rebuild session projection from trace
    #[command(name = "session-rebuild")]
    SessionRebuild {
        /// Session ID to rebuild
        session_id: String,
    },

    /// Evaluation scenarios for real-model quality measurement
    #[cfg(feature = "real-model-eval")]
    Eval {
        #[command(subcommand)]
        eval_cmd: EvalCommands,
    },
}

#[cfg(feature = "real-model-eval")]
#[derive(Subcommand, Debug)]
enum EvalCommands {
    /// List available evaluation scenarios
    List,

    /// Run evaluation scenarios
    Run {
        /// Scenario ID to run ("all" for every scenario)
        #[arg(long, default_value = "all")]
        scenario: String,

        /// Base URL for the LLM provider
        #[arg(long)]
        base_url: Option<String>,

        /// Model name
        #[arg(long, default_value = "qwen3")]
        model: String,

        /// Output directory for reports
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },
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

    let mut cli = Cli::parse();

    let command = cli.command.take().unwrap_or(Commands::Run { message: None });
    match command {
        Commands::Run { message } => cmd_run(&cli, message).await,
        Commands::Explain { session_id } => cmd_explain(&cli, &session_id).await,
        Commands::TraceVerify { session_id } => cmd_trace_verify(&cli, &session_id).await,
        Commands::SessionRebuild { session_id } => cmd_session_rebuild(&cli, &session_id).await,

        #[cfg(feature = "real-model-eval")]
        Commands::Eval { eval_cmd } => cmd_eval(eval_cmd).await,
    }
}

// ── Subcommand: run (existing behavior) ────────────────────────────────────

async fn cmd_run(cli: &Cli, message: Option<String>) -> Result<()> {

    // 1. Open SQLite store (trace + registry)
    let store = SqliteStore::open(SqliteStoreConfig::file(&cli.db)).await?;
    let trace: Arc<dyn TraceStore<StoredEvent>> = Arc::new(store);

    // 2. Open SQLite memory store (same file, own migrations)
    let memory_store = SqliteMemoryStore::open(std::path::Path::new(&cli.db))?;
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
    let message = message.unwrap_or_else(|| {
        "Hello! Can you tell me a short joke?".to_string()
    });

    println!("╔══════════════════════════════════════════╗");
    println!("║          OpenWand Reality Smoke          ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("Provider: {}", cli.base_url);
    println!("Model:    {}", cli.model);
    println!("Database: {}", cli.db);
    println!("Memory:   SQLite (same file)");
    println!();
    println!("User: {message}");
    println!("────────────────────────────────────────────");

    // 7. Create session runner
    let session_id = SessionId::new();

    // Need a second trace store connection for the coordinator
    let trace_for_coordinator: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        SqliteStore::open(SqliteStoreConfig::file(&cli.db)).await?
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
        model: cli.model.clone(),
        base_url: Some(cli.base_url.clone()),
        api_key: cli.api_key.clone(),
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
        Arc::new(SqliteMemoryStore::open(std::path::Path::new(&cli.db))?)
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

// ── Subcommand: explain ────────────────────────────────────────────────────

async fn cmd_explain(_cli: &Cli, session_id: &str) -> Result<()> {
    println!("╔══════════════════════════════════════════╗");
    println!("║       OpenWand Trust Explanation         ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("Session: {}", session_id);
    println!();
    println!("(Explanation rendering will be wired in Wave 05 commit 5)");
    Ok(())
}

// ── Subcommand: trace-verify ───────────────────────────────────────────────

async fn cmd_trace_verify(_cli: &Cli, session_id: &str) -> Result<()> {
    println!("╔══════════════════════════════════════════╗");
    println!("║       OpenWand Trace Verification        ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("Session: {}", session_id);
    println!();
    println!("(Trace verification will be wired in Wave 05 commit 7)");
    Ok(())
}

// ── Subcommand: session-rebuild ────────────────────────────────────────────

async fn cmd_session_rebuild(_cli: &Cli, session_id: &str) -> Result<()> {
    println!("╔══════════════════════════════════════════╗");
    println!("║       OpenWand Session Rebuild           ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("Session: {}", session_id);
    println!();
    println!("(Session rebuild will be wired in Wave 05 commit 7)");
    Ok(())
}

#[cfg(feature = "real-model-eval")]
async fn cmd_eval(cmd: EvalCommands) -> Result<()> {
    use openwand_app::eval_model::*;

    let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("eval");

    match cmd {
        EvalCommands::List => {
            let scenarios = load_eval_fixtures(&fixture_dir)
                .map_err(|e| anyhow::anyhow!("Failed to load eval fixtures: {}", e))?;
            println!("╔══════════════════════════════════════════╗");
            println!("║       OpenWand Eval Scenarios            ║");
            println!("╚══════════════════════════════════════════╝");
            println!();
            for s in &scenarios {
                println!("  {} — {}", s.id, s.title);
                println!("    Turns: {}", s.turns.len());
                println!("    Tags: {:?}", s.tags);
                println!();
            }
            println!("Total: {} scenarios", scenarios.len());
        }
        EvalCommands::Run { scenario, base_url, model, output_dir } => {
            let scenarios = load_eval_fixtures(&fixture_dir)
                .map_err(|e| anyhow::anyhow!("Failed to load eval fixtures: {}", e))?;

            let to_run: Vec<&EvalScenario> = if scenario == "all" {
                scenarios.iter().collect()
            } else {
                scenarios.iter().filter(|s| s.id == scenario).collect()
            };

            if to_run.is_empty() {
                anyhow::bail!("No scenarios matched '{}'", scenario);
            }

            println!("╔══════════════════════════════════════════╗");
            println!("║       OpenWand Eval Run                  ║");
            println!("╚══════════════════════════════════════════╝");
            println!();
            println!("Model: {}", model);
            if let Some(ref url) = base_url {
                println!("Base URL: {}", url);
            } else {
                println!("Base URL: (not specified)");
            }
            println!("Scenarios: {}", to_run.len());
            println!("Output: {}", output_dir);
            println!();

            // Create output directory
            std::fs::create_dir_all(&output_dir)
                .context("Failed to create output directory")?;

            for s in &to_run {
                println!("Running: {} ...", s.id);
                println!("  (real provider execution — stub for deterministic validation)");

                // Write a stub report for now
                let report = EvalRunReport {
                    report_schema_version: EVAL_REPORT_SCHEMA_VERSION,
                    scenario_id: s.id.clone(),
                    provider: ProviderRealitySnapshot::unknown(),
                    memory: MemoryEvalResult {
                        included_claims_seen: vec![],
                        excluded_claims_seen: vec![],
                        missing_required: vec![],
                        unexpected_included: vec![],
                        prompt_panel_equivalent: true,
                    },
                    tools: ToolEvalResult {
                        requested_tools: vec![],
                        executed_tools: vec![],
                        blocked_tools: vec![],
                        forbidden_requested: vec![],
                    },
                    policy: PolicyEvalResult {
                        gates_seen: vec![],
                        required_approvals_seen: vec![],
                        unexpected_allows: vec![],
                    },
                    patch: PatchEvalResult {
                        planned: false,
                        applied: false,
                        preimage_verified: false,
                        postimage_verified: false,
                        rollback_available: false,
                        changed_files_match_expected: true,
                    },
                    explain: ExplainEvalResult {
                        memory_matches: true,
                        policy_matches: true,
                        tool_matches: true,
                        completion_matches: true,
                    },
                    rebuild: RebuildEvalResult {
                        events_replayed: 0,
                        state_matches: true,
                        divergences: vec![],
                    },
                    score: EvalScore::from_dimensions(vec![]),
                };

                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let report_path = format!("{}/{}_{}.json", output_dir, timestamp, s.id);
                let json = serde_json::to_string_pretty(&report)
                    .context("Failed to serialize report")?;
                std::fs::write(&report_path, json)
                    .context("Failed to write report")?;
                println!("  Report: {}", report_path);
                println!();
            }

            println!("Done.");
        }
    }
    Ok(())
}
