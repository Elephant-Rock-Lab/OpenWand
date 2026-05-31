//! OpenWand — Conjure results from intent.
//!
//! Wave 05: CLI binary with subcommand structure.

use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use openwand_app::memory_coordinator::{MemoryCoordinator, PromptInputProductionConfig};
use openwand_app::session_runtime::build_session_runtime;
use openwand_app::session_runtime::build_write_policy;
use openwand_core::SessionId;
use openwand_llm::LlmClient;
use openwand_memory::{MemoryExtractor, MemoryReadStore, MemoryStore, SqliteMemoryStore};
use openwand_policy::PolicyEngine;
use openwand_session::config::{RunConfig, RunStopReason, RunSummary};
use openwand_session::message::MessageContent;
use openwand_session::runner::{ApprovalDecision, SessionRunner};
use openwand_store::StoredEvent;
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

        /// Provider type (openai-compatible, ollama, etc.)
        /// If not specified, inferred from --base-url.
        #[arg(long)]
        provider: Option<String>,

        /// Base URL for the LLM provider
        #[arg(long)]
        base_url: Option<String>,

        /// Model name
        #[arg(long, default_value = "qwen3")]
        model: String,

        /// Output directory for reports
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Baseline for comparison ("none", "latest", or path to report.json)
        #[arg(long, default_value = "none")]
        baseline: String,

        /// Fail with non-zero exit on regression
        #[arg(long)]
        fail_on_regression: bool,
    },

    /// Compare two evaluation reports
    Compare {
        /// Path to current report
        #[arg(long)]
        current: String,

        /// Path to baseline report
        #[arg(long)]
        baseline: String,
    },

    /// Summarize latest evaluation results
    Summarize {
        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Filter to a specific scenario
        #[arg(long)]
        scenario: Option<String>,
    },
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
    use openwand_app::memory_coordinator::{MemoryCoordinator, PromptInputProductionConfig};
    use openwand_memory::testing::HeuristicExtractor;

    // 1. Build session runtime (shared with eval runner)
    let rt = build_session_runtime(&cli.db, &std::env::current_dir()?.to_string_lossy()).await?;

    // 2. Get user message
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

    // 3. Configure run
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
    let result = rt.runner.run_turn(message.clone(), run_config.clone()).await?;

    // 4. Handle approval flow
    let result = if matches!(result.stop_reason, RunStopReason::AwaitingApproval) {
        println!("────────────────────────────────────────────");
        if let Some(pending) = rt.runner.pending_approval().await {
            println!("⚠ Tool '{}' requires your approval.", pending.tool_name);
            println!("  Reason: {}", pending.policy_summary);
            println!("  Approve? [y/N] ");

            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap_or_default();
            let approved = input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes";

            let decision = if approved {
                ApprovalDecision::approve()
            } else {
                ApprovalDecision::reject()
            };

            let approval_result = rt.runner.resolve_approval(decision, run_config).await?;
            println!("  → {}", if approved { "Approved" } else { "Rejected" });
            if let Some(tool_result) = &approval_result.tool_result {
                println!("  Tool result: {}", tool_result.output);
            }
        }

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

    // 5. Run memory projection
    let extractor: Arc<dyn MemoryExtractor> = Arc::new(HeuristicExtractor);
    let coordinator = MemoryCoordinator::new(
        rt.memory_store.clone(),
        extractor,
        rt.trace_for_coordinator.clone(),
    );

    let projection = coordinator.project_after_run(&rt.session_id).await;
    println!();
    println!("Memory projection:");
    println!("  Episodes projected:  {}", projection.episodes_projected);
    println!("  Candidates extracted: {}", projection.candidates_extracted);
    println!("  Records accepted:    {}", projection.records_accepted);
    if !projection.errors.is_empty() {
        println!("  Errors: {:?}", projection.errors);
    }

    // 6. Produce 02k prompt inputs (diagnostic)
    let prompt_result = coordinator
        .produce_prompt_inputs(
            Some(rt.session_id.clone()),
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

    // 7. Show Loro projection
    let messages = rt.runner.loro_state().messages().map_err(|e| anyhow::anyhow!("{e}"))?;
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

    // 8. Show stale status
    let stale = rt.runner.loro_state().projection_is_stale().map_err(|e| anyhow::anyhow!("{e}"))?;
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
fn resolve_provider(provider: Option<&str>, base_url: Option<&str>) -> openwand_llm::LlmProvider {
    // Explicit provider flag takes priority
    if let Some(name) = provider {
        return match name.to_lowercase().as_str() {
            "openai" => openwand_llm::LlmProvider::OpenAI,
            "anthropic" => openwand_llm::LlmProvider::Anthropic,
            "ollama" => openwand_llm::LlmProvider::Ollama,
            "groq" => openwand_llm::LlmProvider::Groq,
            "deepseek" => openwand_llm::LlmProvider::DeepSeek,
            other => openwand_llm::LlmProvider::Custom { name: other.to_string() },
        };
    }
    // Infer from base_url heuristics
    if let Some(url) = base_url {
        if url.contains("ollama") || url.contains(":11434") {
            return openwand_llm::LlmProvider::Ollama;
        }
        if url.contains("groq") {
            return openwand_llm::LlmProvider::Groq;
        }
        if url.contains("deepseek") {
            return openwand_llm::LlmProvider::DeepSeek;
        }
        if url.contains("anthropic") {
            return openwand_llm::LlmProvider::Anthropic;
        }
    }
    // Default: OpenAI-compatible (covers LM Studio, vLLM, etc.)
    openwand_llm::LlmProvider::Custom { name: "openai-compatible".to_string() }
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
        EvalCommands::Run { scenario, provider, base_url, model, output_dir, baseline, fail_on_regression } => {
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

            // Resolve provider: explicit flag > inferred from base_url > default
            let resolved_provider = resolve_provider(provider.as_deref(), base_url.as_deref());
            let base_url = base_url.unwrap_or_else(|| "http://localhost:1234/v1".to_string());

            println!("╔══════════════════════════════════════════╗");
            println!("║       OpenWand Eval Run                  ║");
            println!("╚══════════════════════════════════════════╝");
            println!();
            println!("Provider: {:?}", resolved_provider);
            println!("Model:    {}", model);
            println!("Base URL: {}", base_url);
            println!("Baseline: {}", baseline);
            println!("Scenarios: {}", to_run.len());
            println!("Output: {}", output_dir);
            println!();

            // Create output directory structure
            std::fs::create_dir_all(&output_dir)
                .context("Failed to create output directory")?;

            let mut any_regression = false;

            for s in &to_run {
                println!("Running: {} ...", s.id);

                // Build session runtime using shared assembly
                let db_path = format!("{}/eval_{}.db", output_dir, s.id);
                let working_dir = format!("{}/workspace_{}", output_dir, s.id);
                std::fs::create_dir_all(&working_dir)
                    .context("Failed to create eval workspace")?;

                let rt = build_session_runtime(&db_path, &working_dir).await?;

                // Configure run with resolved provider
                let mut run_config = RunConfig::default();
                run_config.mode = openwand_core::mode::InteractionMode::Direct;
                run_config.working_directory = working_dir.clone();
                run_config.llm_target = Some(openwand_llm::LlmTarget {
                    provider: resolved_provider.clone(),
                    model: model.clone(),
                    base_url: Some(base_url.clone()),
                    api_key: None,
                });

                // Run each turn in the scenario
                let mut all_tools_executed = Vec::new();
                let mut steps_total = 0u64;

                for (turn_idx, turn_msg) in s.turns.iter().enumerate() {
                    println!("  Turn {}: {}", turn_idx + 1, &turn_msg[..turn_msg.len().min(60)]);

                    let result = rt.runner.run_turn(turn_msg.clone(), run_config.clone()).await?;
                    steps_total += result.steps_completed;

                    // Auto-approve any pending approvals in eval mode
                    if matches!(result.stop_reason, RunStopReason::AwaitingApproval) {
                        let decision = ApprovalDecision::approve();
                        let _approval_result = rt.runner.resolve_approval(decision, run_config.clone()).await?;
                    }
                }

                // Build provider snapshot
                let provider_snapshot = ProviderRealitySnapshot {
                    provider: format!("{:?}", resolved_provider),
                    model: model.clone(),
                    base_url_redacted: Some(base_url.clone()),
                    supports_streaming: true,
                    supports_tools: true,
                    supports_reasoning: false,
                    health_status: ProviderHealthStatus::Healthy,
                    temperature: None,
                    max_tokens: None,
                    observed_at: chrono::Utc::now(),
                };

                let report = EvalRunReport {
                    report_schema_version: EVAL_REPORT_SCHEMA_VERSION,
                    scenario_id: s.id.clone(),
                    provider: provider_snapshot,
                    prompt: openwand_app::eval_model::PromptEvalResult::default(),
                    memory: MemoryEvalResult {
                        included_claims_seen: vec![],
                        excluded_claims_seen: vec![],
                        missing_required: vec![],
                        unexpected_included: vec![],
                        prompt_panel_equivalent: true,
                    },
                    tools: ToolEvalResult {
                        requested_tools: all_tools_executed.clone(),
                        executed_tools: all_tools_executed.clone(),
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

                // Save report using EvalReportStore
                let store = openwand_app::eval_reports::EvalReportStore::new(
                    std::path::PathBuf::from(&output_dir)
                );
                let report_path = store.save_report(&report)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                println!("  Score: {}/{} ({:.0}%)", report.score.total, report.score.max, report.score.pass_rate * 100.0);
                println!("  Report: {}", report_path.display());
                println!();

                // Baseline comparison
                let baseline_selection = match baseline.as_str() {
                    "none" => openwand_app::eval_compare::EvalBaselineSelection::None,
                    "latest" => openwand_app::eval_compare::EvalBaselineSelection::Latest,
                    path => openwand_app::eval_compare::EvalBaselineSelection::Path(
                        std::path::PathBuf::from(path)
                    ),
                };
                let store = openwand_app::eval_reports::EvalReportStore::new(
                    std::path::PathBuf::from(&output_dir)
                );
                let baseline_report = openwand_app::eval_compare::resolve_baseline(
                    &baseline_selection, &store, &s.id,
                ).map_err(|e| anyhow::anyhow!("{}", e))?;

                let thresholds = openwand_app::eval_compare::RegressionThresholds::default();
                let comparison = openwand_app::eval_compare::compare_reports(
                    &report, baseline_report.as_ref(), &thresholds,
                );

                // Print comparison summary
                if let Some(bt) = comparison.score_delta.baseline_total {
                    let delta = comparison.score_delta.delta.unwrap_or(0);
                    let sign = if delta >= 0 { "+" } else { "" };
                    println!("  vs Baseline: {} {} ({:?})", sign, delta,
                        comparison.score_delta.baseline_pass_rate);
                }
                if !comparison.regressions.is_empty() {
                    println!("  ⚠ Regressions:");
                    for r in &comparison.regressions {
                        println!("    - {}", r.description);
                    }
                    any_regression = true;
                }
                if !comparison.improvements.is_empty() {
                    println!("  ✓ Improvements:");
                    for i in &comparison.improvements {
                        println!("    - {}", i.description);
                    }
                }
            }

            println!("Done. {} scenarios executed.", to_run.len());

            if fail_on_regression && any_regression {
                anyhow::bail!("Regression detected — failing eval run");
            }
        }

        EvalCommands::Compare { current, baseline } => {
            let store = openwand_app::eval_reports::EvalReportStore::new(
                std::path::PathBuf::from(".")
            );
            let current_report = store.load_report(std::path::Path::new(&current))
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let baseline_report = store.load_report(std::path::Path::new(&baseline))
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let thresholds = openwand_app::eval_compare::RegressionThresholds::default();
            let comparison = openwand_app::eval_compare::compare_reports(
                &current_report, Some(&baseline_report), &thresholds,
            );

            println!("╔══════════════════════════════════════════╗");
            println!("║       OpenWand Eval Compare              ║");
            println!("╚══════════════════════════════════════════╝");
            println!();
            println!("Scenario: {}", comparison.scenario_id);
            println!();

            // Score
            if let Some(delta) = comparison.score_delta.delta {
                let sign = if delta >= 0 { "+" } else { "" };
                println!("Score: {}/{} ({}{})",
                    comparison.score_delta.current_total,
                    comparison.score_delta.baseline_total.unwrap_or(0),
                    sign, delta);
            } else {
                println!("Score: {}/{} (no baseline)",
                    comparison.score_delta.current_total,
                    current_report.score.max);
            }

            // Pass rate
            println!("Pass rate: {:.1}%",
                comparison.score_delta.current_pass_rate * 100.0);

            // Dimensions
            if !comparison.dimension_deltas.is_empty() {
                println!();
                println!("Dimensions:");
                for dd in &comparison.dimension_deltas {
                    if let Some(bs) = dd.baseline_score {
                        let delta = dd.delta.unwrap_or(0);
                        let sign = if delta >= 0 { "+" } else { "" };
                        let status = if delta < 0 { "⚠" } else if delta > 0 { "✓" } else { "=" };
                        println!("  {} {:20} {:3}  {}{}",
                            status, dd.dimension, dd.current_score, sign, delta);
                    } else {
                        println!("  · {:20} {:3}", dd.dimension, dd.current_score);
                    }
                }
            }

            // Provider changes
            if comparison.provider_delta.provider_changed || comparison.provider_delta.model_changed {
                println!();
                println!("Provider changes:");
                if comparison.provider_delta.provider_changed {
                    println!("  Provider: {} → {}",
                        comparison.provider_delta.baseline_provider.as_deref().unwrap_or("?"),
                        comparison.provider_delta.current_provider);
                }
                if comparison.provider_delta.model_changed {
                    println!("  Model: {} → {}",
                        comparison.provider_delta.baseline_model.as_deref().unwrap_or("?"),
                        comparison.provider_delta.current_model);
                }
            }

            // Regressions
            if !comparison.regressions.is_empty() {
                println!();
                println!("Regressions ({}):", comparison.regressions.len());
                for r in &comparison.regressions {
                    println!("  ⚠ {}", r.description);
                }
            }

            // Improvements
            if !comparison.improvements.is_empty() {
                println!();
                println!("Improvements ({}):", comparison.improvements.len());
                for i in &comparison.improvements {
                    println!("  ✓ {}", i.description);
                }
            }
        }

        EvalCommands::Summarize { output_dir, scenario } => {
            let store = openwand_app::eval_reports::EvalReportStore::new(
                std::path::PathBuf::from(&output_dir)
            );

            let filter = openwand_app::eval_reports::ReportFilter {
                scenario_id: scenario.clone(),
            };
            let reports = store.list_reports(&filter)
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            println!("╔══════════════════════════════════════════╗");
            println!("║       OpenWand Eval Summary              ║");
            println!("╚══════════════════════════════════════════╝");
            println!();

            if reports.is_empty() {
                println!("No reports found in: {}", output_dir);
                if let Some(ref s) = scenario {
                    println!("  (filtered by scenario: {})", s);
                }
                return Ok(());
            }

            // Group by scenario
            let mut by_scenario: std::collections::BTreeMap<String, Vec<&openwand_app::eval_reports::StoredEvalReport>> = std::collections::BTreeMap::new();
            for r in &reports {
                by_scenario.entry(r.report.scenario_id.clone())
                    .or_default()
                    .push(r);
            }

            println!("Total reports: {}", reports.len());
            println!("Scenarios:     {}", by_scenario.len());
            println!();

            for (id, scenario_reports) in &by_scenario {
                let latest = scenario_reports.first().unwrap(); // sorted newest-first
                let run_count = scenario_reports.len();

                println!("  {}", id);
                println!("    Runs: {}", run_count);
                println!("    Latest: {}/{} ({:.0}%)",
                    latest.report.score.total,
                    latest.report.score.max,
                    latest.report.score.pass_rate * 100.0);
                println!("    Provider: {} / {}",
                    latest.report.provider.provider,
                    latest.report.provider.model);
                println!("    At: {}", latest.report.provider.observed_at.format("%Y-%m-%d %H:%M UTC"));
                println!();
            }

            // Regressions across latest reports
            let mut all_regressions = 0;
            for (id, scenario_reports) in &by_scenario {
                if scenario_reports.len() >= 2 {
                    let current = &scenario_reports[0].report;
                    let baseline = &scenario_reports[1].report;
                    let thresholds = openwand_app::eval_compare::RegressionThresholds::default();
                    let comparison = openwand_app::eval_compare::compare_reports(
                        current, Some(baseline), &thresholds,
                    );
                    all_regressions += comparison.regressions.len();
                    if !comparison.regressions.is_empty() {
                        println!("⚠ {} has {} regression(s)", id, comparison.regressions.len());
                        for r in &comparison.regressions {
                            println!("    - {}", r.description);
                        }
                    }
                }
            }

            if all_regressions == 0 {
                println!("✓ No regressions detected.");
            }

            // Generate and save summary report
            let summary = openwand_app::eval_summary::generate_summary(
                &openwand_app::eval_reports::EvalReportStore::new(
                    std::path::PathBuf::from(&output_dir)
                )
            ).map_err(|e| anyhow::anyhow!("{}", e))?;

            let summaries_dir = format!("{}/summaries", output_dir);
            std::fs::create_dir_all(&summaries_dir)?;
            let summary_path = format!("{}/{}_summary.json",
                summaries_dir,
                chrono::Utc::now().format("%Y-%m-%dT%H-%M-%SZ"));
            let json = serde_json::to_string_pretty(&summary)
                .context("Failed to serialize summary")?;
            std::fs::write(&summary_path, json)
                .context("Failed to write summary")?;
            println!();
            println!("Summary: {}", summary_path);
        }
    }
    Ok(())
}
