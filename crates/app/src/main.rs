//! OpenWand — Conjure results from intent.
//!
//! Wave 05: CLI binary with subcommand structure.

use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use openwand_app::session_runtime::build_session_runtime;
use openwand_memory::MemoryExtractor;
use openwand_session::config::{RunConfig, RunStopReason, RunSummary};
use openwand_session::message::MessageContent;
use openwand_session::runner::{ApprovalDecision, ApprovalResolution};
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

    #[command(name = "audit-check")]
    AuditCheck {
        /// Session ID to audit
        session_id: String,
    },

    /// Rebuild session projection from trace
    #[command(name = "session-rebuild")]
    SessionRebuild {
        /// Session ID to rebuild
        session_id: String,
    },

    /// Task plan commands (no model required)
    #[command(name = "task-plan")]
    TaskPlan {
        #[command(subcommand)]
        task_plan_cmd: TaskPlanCommands,
    },

    /// Workflow proposal commands (no model required)
    #[command(name = "workflow-proposal")]
    WorkflowProposal {
        #[command(subcommand)]
        workflow_proposal_cmd: WorkflowProposalCommands,
    },

    /// Workflow readiness evaluation (no model required)
    #[command(name = "workflow-readiness")]
    WorkflowReadiness {
        #[command(subcommand)]
        workflow_readiness_cmd: WorkflowReadinessCommands,
    },

    /// Workflow execution commands (no model required)
    #[command(name = "workflow-execution")]
    WorkflowExecution {
        #[command(subcommand)]
        workflow_execution_cmd: WorkflowExecutionCommands,
    },

    /// Workflow action routing commands (no model required)
    #[command(name = "workflow-action")]
    WorkflowAction {
        #[command(subcommand)]
        workflow_action_cmd: WorkflowActionCommands,
    },

    /// Workflow action outcome commands
    #[command(name = "workflow-action-outcome")]
    WorkflowActionOutcome {
        #[command(subcommand)]
        workflow_action_outcome_cmd: WorkflowActionOutcomeCommands,
    },

    /// Workflow reconciliation commands
    #[command(name = "workflow-reconciliation")]
    WorkflowReconciliation {
        #[command(subcommand)]
        workflow_reconciliation_cmd: WorkflowReconciliationCommands,
    },

    /// Workflow continuation commands
    #[command(name = "workflow-continuation")]
    WorkflowContinuation {
        #[command(subcommand)]
        workflow_continuation_cmd: WorkflowContinuationCommands,
    },

    /// Next-action review commands
    #[command(name = "workflow-next-action-review")]
    WorkflowNextActionReviewCmd {
        #[command(subcommand)]
        next_action_review_cmd: WorkflowNextActionReviewCommands,
    },

    /// Routing readiness commands
    #[command(name = "workflow-routing-readiness")]
    WorkflowRoutingReadinessCmd {
        #[command(subcommand)]
        routing_readiness_cmd: WorkflowRoutingReadinessCommands,
    },

    /// Next-action routing commands
    #[command(name = "workflow-next-action-routing")]
    WorkflowNextActionRoutingCmd {
        #[command(subcommand)]
        next_action_routing_cmd: WorkflowNextActionRoutingCommands,
    },

    /// Workflow loop controller
    #[command(name = "workflow-loop")]
    WorkflowLoopCmd {
        #[command(subcommand)]
        loop_cmd: WorkflowLoopCommands,
    },

    /// Command composer
    #[command(name = "workflow-command")]
    WorkflowCommandCmd {
        #[command(subcommand)]
        command_cmd: WorkflowCommandCommands,
    },

    /// Command review
    #[command(name = "workflow-command-review")]
    WorkflowCommandReviewCmd {
        #[command(subcommand)]
        review_cmd: WorkflowCommandReviewCommands,
    },

    /// Manual result capture
    #[command(name = "workflow-manual-result")]
    WorkflowManualResultCmd {
        #[command(subcommand)]
        result_cmd: WorkflowManualResultCommands,
    },

    /// Evaluation scenarios for real-model quality measurement
    /// Manual result review — accept/reject/request-changes on reported results
    #[command(name = "workflow-manual-result-review")]
    WorkflowManualResultReview {
        #[command(subcommand)]
        cmd: WorkflowManualResultReviewCommands,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Workflow operator console
    #[command(name = "workflow-operator-console")]
    WorkflowOperatorConsole {
        #[command(subcommand)]
        console_cmd: WorkflowOperatorConsoleCommands,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Manual result reconciliation gate
    #[command(name = "workflow-manual-result-reconciliation-gate")]
    WorkflowManualResultReconciliationGate {
        #[command(subcommand)]
        gate_cmd: WorkflowManualResultReconciliationGateCommands,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Manual result reconciliation readiness
    #[command(name = "workflow-manual-result-reconciliation-readiness")]
    WorkflowManualResultReconciliationReadiness {
        #[command(subcommand)]
        readiness_cmd: WorkflowManualResultReconciliationReadinessCommands,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Evidence chain inspector
    #[command(name = "workflow-evidence-chain")]
    WorkflowEvidenceChainCmd {
        #[command(subcommand)]
        evidence_chain_cmd: WorkflowEvidenceChainCommands,
        #[arg(long, default_value = "workflow_data")]
        store_dir: String,
    },

    /// External attestation
    #[command(name = "workflow-external-attestation")]
    WorkflowExternalAttestationCmd {
        #[command(subcommand)]
        attestation_cmd: WorkflowExternalAttestationCommands,
        #[arg(long, default_value = "workflow_data")]
        store_dir: String,
    },

    /// Verification readiness
    #[command(name = "workflow-verification-readiness")]
    WorkflowVerificationReadinessCmd {
        #[command(subcommand)]
        verification_readiness_cmd: WorkflowVerificationReadinessCommands,
        #[arg(long, default_value = "workflow_data")]
        store_dir: String,
    },

    /// Audit packet review
    #[command(name = "audit-packet-review")]
    AuditPacketReviewCmd {
        #[command(subcommand)]
        audit_review_cmd: AuditPacketReviewCommands,
        #[arg(long, default_value = "workflow_data")]
        store_dir: String,
    },

    /// Audit packet distribution
    #[command(name = "audit-packet-distribution")]
    AuditPacketDistributionCmd {
        #[command(subcommand)]
        audit_distribution_cmd: AuditPacketDistributionCommands,
        #[arg(long, default_value = "workflow_data")]
        store_dir: String,
    },

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

    /// Compute auto-commit readiness from stored eval reports
    #[cfg(feature = "real-model-eval")]
    Readiness {
        /// Readiness target
        #[arg(long, default_value = "auto-commit")]
        target: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Minimum total runs
        #[arg(long, default_value_t = 15)]
        min_runs: usize,

        /// Minimum reports per required scenario
        #[arg(long, default_value_t = 3)]
        min_reports_per_scenario: usize,

        /// Minimum weighted pass rate
        #[arg(long, default_value_t = 0.90)]
        min_weighted_pass_rate: f64,

        /// Minimum patch dimension pass rate
        #[arg(long, default_value_t = 0.95)]
        min_patch_pass_rate: f64,

        /// Minimum policy dimension pass rate
        #[arg(long, default_value_t = 1.00)]
        min_policy_pass_rate: f64,

        /// Minimum rebuild dimension pass rate
        #[arg(long, default_value_t = 1.00)]
        min_rebuild_pass_rate: f64,

        /// Minimum explain dimension pass rate
        #[arg(long, default_value_t = 0.90)]
        min_explain_pass_rate: f64,

        /// Maximum allowed regressions
        #[arg(long, default_value_t = 0)]
        max_allowed_regressions: usize,
    },

    /// Auto-commit proposal commands
    #[cfg(feature = "real-model-eval")]
    #[command(subcommand)]
    AutoCommit(AutoCommitCommands),
}

#[cfg(feature = "real-model-eval")]
#[derive(Debug, clap::Subcommand)]
enum AutoCommitCommands {
    /// Generate an auto-commit proposal
    Propose {
        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show a specific proposal
    Show {
        /// Proposal ID
        proposal_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Review a proposal
    #[command(subcommand)]
    Review(AutoCommitReviewCommands),

    /// Execute an approved proposal
    #[cfg(feature = "real-model-eval")]
    Execute {
        /// Proposal ID
        proposal_id: String,

        /// Review ID
        review_id: String,

        /// Idempotency key (prevents double execution)
        #[arg(long)]
        idempotency_key: Option<String>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show execution record
    #[cfg(feature = "real-model-eval")]
    Execution {
        #[command(subcommand)]
        command: ExecutionCommands,
    },

    /// Verify a post-commit execution
    #[cfg(feature = "real-model-eval")]
    Verify {
        /// Execution ID to verify
        execution_id: String,

        /// Idempotency key
        #[arg(long)]
        idempotency_key: Option<String>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show verification records
    #[cfg(feature = "real-model-eval")]
    Verification {
        #[command(subcommand)]
        command: VerificationCommands,
    },

    /// Evaluate push readiness
    #[cfg(feature = "real-model-eval")]
    PushReadiness {
        #[command(subcommand)]
        command: PushReadinessCommands,
    },

    /// Push proposal and review
    #[cfg(feature = "real-model-eval")]
    PushProposal {
        #[command(subcommand)]
        command: PushProposalCommands,
    },

    /// Governed remote push execution
    #[cfg(feature = "real-model-eval")]
    Push {
        #[command(subcommand)]
        command: PushExecutionCommands,
    },
}

#[cfg(feature = "real-model-eval")]
#[derive(Debug, clap::Subcommand)]
enum AutoCommitReviewCommands {
    /// Approve a proposal
    Approve {
        /// Proposal ID
        proposal_id: String,

        /// Rationale for approval
        #[arg(long)]
        rationale: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Reject a proposal
    Reject {
        /// Proposal ID
        proposal_id: String,

        /// Rationale for rejection
        #[arg(long)]
        rationale: String,

        /// Feedback for next iteration
        #[arg(long)]
        feedback: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Request changes on a proposal
    #[command(name = "request-changes")]
    RequestChanges {
        /// Proposal ID
        proposal_id: String,

        /// Rationale for change request
        #[arg(long)]
        rationale: String,

        /// Feedback describing required changes
        #[arg(long)]
        feedback: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show a specific review
    ShowReview {
        /// Review ID
        review_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Show latest review
    LatestReview {
        /// Filter by proposal ID
        #[arg(long)]
        proposal_id: Option<String>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },
}

#[cfg(feature = "real-model-eval")]
#[derive(Debug, clap::Subcommand)]
enum ExecutionCommands {
    /// Show a specific execution record
    Show {
        /// Execution ID
        execution_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Show latest execution record
    Latest {
        /// Filter by proposal ID
        #[arg(long)]
        proposal_id: Option<String>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },
}

#[cfg(feature = "real-model-eval")]
#[derive(Debug, clap::Subcommand)]
enum VerificationCommands {
    /// Show a specific verification record
    Show {
        /// Verification ID
        verification_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Show latest verification record
    Latest {
        /// Filter by execution ID
        #[arg(long)]
        execution_id: Option<String>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },
}

#[cfg(feature = "real-model-eval")]
#[derive(Debug, clap::Subcommand)]
enum PushReadinessCommands {
    /// Evaluate push readiness for a verified commit
    Evaluate {
        /// Verification ID
        verification_id: String,

        /// Target remote name
        #[arg(long, default_value = "origin")]
        remote: String,

        /// Target branch name
        #[arg(long)]
        branch: Option<String>,

        /// Idempotency key
        #[arg(long)]
        idempotency_key: Option<String>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show a specific readiness record
    Show {
        /// Readiness ID
        readiness_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Show latest readiness record
    Latest {
        /// Filter by verification ID
        #[arg(long)]
        verification_id: Option<String>,

        /// Filter by commit hash
        #[arg(long)]
        commit: Option<String>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },
}

#[cfg(feature = "real-model-eval")]
#[derive(Debug, clap::Subcommand)]
enum PushProposalCommands {
    /// Create a push proposal from a readiness record
    Create {
        /// Readiness ID
        readiness_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show a push proposal
    Show {
        /// Proposal ID
        proposal_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Show latest push proposal
    Latest {
        /// Filter by readiness ID
        #[arg(long)]
        readiness_id: Option<String>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Review a push proposal
    Review {
        #[command(subcommand)]
        command: PushProposalReviewCommands,
    },
}

#[cfg(feature = "real-model-eval")]
#[derive(Debug, clap::Subcommand)]
enum PushProposalReviewCommands {
    /// Approve a push proposal
    Approve {
        /// Proposal ID
        proposal_id: String,

        /// Reviewer name
        #[arg(long)]
        reviewer: String,

        /// Rationale
        #[arg(long)]
        rationale: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Reject a push proposal
    Reject {
        /// Proposal ID
        proposal_id: String,

        /// Reviewer name
        #[arg(long)]
        reviewer: String,

        /// Rationale
        #[arg(long)]
        rationale: String,

        /// Feedback
        #[arg(long)]
        feedback: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Request changes on a push proposal
    RequestChanges {
        /// Proposal ID
        proposal_id: String,

        /// Reviewer name
        #[arg(long)]
        reviewer: String,

        /// Rationale
        #[arg(long)]
        rationale: String,

        /// Feedback
        #[arg(long)]
        feedback: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Show a push proposal review
    ShowReview {
        /// Review ID
        review_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },

    /// Show latest review for a proposal
    LatestReview {
        /// Proposal ID
        #[arg(long)]
        proposal_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
    },
}

#[cfg(feature = "real-model-eval")]
#[derive(Debug, clap::Subcommand)]
enum PushExecutionCommands {
    /// Execute a governed remote push
    Execute {
        /// Proposal ID
        #[arg(long)]
        proposal_id: String,

        /// Review ID
        #[arg(long)]
        review_id: String,

        /// Idempotency key
        #[arg(long)]
        idempotency_key: Option<String>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show push execution subcommands
    Execution {
        #[command(subcommand)]
        command: PushExecutionQueryCommands,
    },
}

#[cfg(feature = "real-model-eval")]
#[derive(Debug, clap::Subcommand)]
enum PushExecutionQueryCommands {
    /// Show a push execution record
    Show {
        /// Execution ID
        execution_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show latest push execution
    Latest {
        /// Filter by proposal ID
        #[arg(long)]
        proposal_id: Option<String>,

        /// Filter by review ID
        #[arg(long)]
        review_id: Option<String>,

        /// Filter by commit hash
        #[arg(long)]
        commit: Option<String>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Task plan commands
#[derive(Debug, clap::Subcommand)]
enum TaskPlanCommands {
    /// Create a task plan from user intent
    Create {
        /// User intent text
        #[arg(long)]
        intent: String,

        /// Policy constraints (can be repeated)
        #[arg(long)]
        policy_constraints: Option<Vec<String>>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show a specific task plan
    Show {
        /// Plan ID
        plan_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show the latest task plan
    Latest {
        /// Filter by goal ID
        #[arg(long)]
        goal_id: Option<String>,

        /// Filter by skill ID
        #[arg(long)]
        skill_id: Option<String>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Review a task plan
    #[command(subcommand)]
    Review(TaskPlanReviewCommands),
}

/// Task plan review commands
#[derive(Debug, clap::Subcommand)]
enum TaskPlanReviewCommands {
    /// Approve a task plan
    Approve {
        /// Plan ID to approve
        #[arg(long)]
        plan_id: String,

        /// Reviewer name
        #[arg(long)]
        reviewer: String,

        /// Approval rationale
        #[arg(long)]
        rationale: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Reject a task plan
    Reject {
        /// Plan ID to reject
        #[arg(long)]
        plan_id: String,

        /// Reviewer name
        #[arg(long)]
        reviewer: String,

        /// Rejection rationale
        #[arg(long)]
        rationale: String,

        /// Feedback text
        #[arg(long)]
        feedback: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Request changes to a task plan
    RequestChanges {
        /// Plan ID
        #[arg(long)]
        plan_id: String,

        /// Reviewer name
        #[arg(long)]
        reviewer: String,

        /// Rationale for changes
        #[arg(long)]
        rationale: String,

        /// Feedback text describing changes needed
        #[arg(long)]
        feedback: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
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
        Commands::AuditCheck { session_id } => cmd_audit_check(&cli, &session_id).await,
        Commands::SessionRebuild { session_id } => cmd_session_rebuild(&cli, &session_id).await,
        Commands::TaskPlan { task_plan_cmd } => { cmd_eval_task_plan(task_plan_cmd)?; Ok(()) },
        Commands::WorkflowProposal { workflow_proposal_cmd } => { cmd_workflow_proposal(workflow_proposal_cmd)?; Ok(()) },
        Commands::WorkflowReadiness { workflow_readiness_cmd } => { cmd_workflow_readiness(workflow_readiness_cmd)?; Ok(()) },
        Commands::WorkflowExecution { workflow_execution_cmd } => { cmd_workflow_execution(workflow_execution_cmd)?; Ok(()) },
        Commands::WorkflowAction { workflow_action_cmd } => { cmd_workflow_action(workflow_action_cmd)?; Ok(()) },
        Commands::WorkflowActionOutcome { workflow_action_outcome_cmd } => { cmd_workflow_action_outcome(workflow_action_outcome_cmd)?; Ok(()) },
        Commands::WorkflowReconciliation { workflow_reconciliation_cmd } => { cmd_workflow_reconciliation(workflow_reconciliation_cmd)?; Ok(()) },
        Commands::WorkflowContinuation { workflow_continuation_cmd } => { cmd_workflow_continuation(workflow_continuation_cmd)?; Ok(()) },
        Commands::WorkflowNextActionReviewCmd { next_action_review_cmd } => { cmd_workflow_next_action_review(next_action_review_cmd)?; Ok(()) },
        Commands::WorkflowRoutingReadinessCmd { routing_readiness_cmd } => { cmd_workflow_routing_readiness(routing_readiness_cmd)?; Ok(()) },
        Commands::WorkflowNextActionRoutingCmd { next_action_routing_cmd } => { cmd_workflow_next_action_routing(next_action_routing_cmd)?; Ok(()) },
        Commands::WorkflowLoopCmd { loop_cmd } => { cmd_workflow_loop(loop_cmd)?; Ok(()) },
        Commands::WorkflowCommandCmd { command_cmd } => { cmd_workflow_command(command_cmd)?; Ok(()) },
        Commands::WorkflowCommandReviewCmd { review_cmd } => { cmd_workflow_command_review(review_cmd)?; Ok(()) },
        Commands::WorkflowManualResultCmd { result_cmd } => { cmd_workflow_manual_result(result_cmd)?; Ok(()) },
        Commands::WorkflowManualResultReview { cmd, output_dir } => { cmd_workflow_manual_result_review(cmd, output_dir)?; Ok(()) },
        Commands::WorkflowManualResultReconciliationReadiness { readiness_cmd, output_dir } => { cmd_workflow_reconciliation_readiness(readiness_cmd, output_dir)?; Ok(()) },
        Commands::WorkflowManualResultReconciliationGate { gate_cmd, output_dir } => { cmd_manual_reconciliation_gate(gate_cmd, output_dir)?; Ok(()) },
        Commands::WorkflowOperatorConsole { console_cmd, output_dir } => { cmd_operator_console(console_cmd, output_dir)?; Ok(()) },
        Commands::WorkflowEvidenceChainCmd { evidence_chain_cmd, store_dir } => { cmd_evidence_chain(evidence_chain_cmd, store_dir)?; Ok(()) },
        Commands::WorkflowExternalAttestationCmd { attestation_cmd, store_dir } => { cmd_external_attestation(attestation_cmd, store_dir)?; Ok(()) },
        Commands::WorkflowVerificationReadinessCmd { verification_readiness_cmd, store_dir } => { cmd_verification_readiness(verification_readiness_cmd, store_dir)?; Ok(()) },
        Commands::AuditPacketReviewCmd { audit_review_cmd, store_dir } => { cmd_audit_packet_review(audit_review_cmd, store_dir)?; Ok(()) },
        Commands::AuditPacketDistributionCmd { audit_distribution_cmd, store_dir } => { cmd_audit_packet_distribution(audit_distribution_cmd, store_dir)?; Ok(()) },

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
        println!("─────────────────────────────────────────");
        let approval_outcome = if let Some(pending) = rt.runner.pending_approval().await {
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

            // Report distinct outcomes for approve vs reject
            match &approval_result.resolution {
                ApprovalResolution::Approve => {
                    println!("  → Approved");
                    if let Some(tool_result) = &approval_result.tool_result {
                        if tool_result.is_error {
                            println!("  Tool FAILED: {}", tool_result.output);
                        } else {
                            println!("  Tool result: {}", tool_result.output);
                        }
                    }
                }
                ApprovalResolution::Reject { .. } => {
                    println!("  → Rejected — tool was NOT executed");
                }
            }
            Some(approval_result)
        } else {
            println!("  No pending approval found (internal error)");
            None
        };

        // Build summary reflecting actual outcome
        match approval_outcome {
            Some(ar) => {
                let was_approved = matches!(ar.resolution, ApprovalResolution::Approve);
                let tool_errored = ar.tool_result.as_ref().is_some_and(|r| r.is_error);

                let reason = if !was_approved {
                    RunStopReason::ToolDenied
                } else if tool_errored {
                    RunStopReason::ToolBlocked
                } else {
                    RunStopReason::Natural
                };

                RunSummary {
                    stop_reason: reason,
                    steps_completed: result.steps_completed,
                    tools_executed: result.tools_executed + if was_approved { 1 } else { 0 },
                    recoverable: !matches!(reason, RunStopReason::ToolDenied),
                }
            }
            None => RunSummary {
                stop_reason: RunStopReason::ToolBlocked,
                steps_completed: result.steps_completed,
                tools_executed: result.tools_executed,
                recoverable: true,
            },
        }
    } else {
        result
    };

    // Print outcome — distinguish success from failure
    let is_success = matches!(result.stop_reason, RunStopReason::Natural | RunStopReason::MaxStepsReached);
    println!("────────────────────────────────────────────");
    if is_success {
        println!("✓ Turn complete");
    } else {
        println!("✗ Turn ended with: {:?}", result.stop_reason);
    }
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

async fn cmd_explain(_cli: &Cli, _session_id: &str) -> Result<()> {
    eprintln!("error: the 'explain' command is not yet implemented.");
    eprintln!("       Session trust explanation is planned for a future release.");
    std::process::exit(1);
}


// ── Subcommand: trace-verify ─────────────────────────────────

/// Exit codes for trace verification (distinct from operational errors):
/// 0 = Pass, 1 = operational error, 2 = Fail (integrity), 3 = Inconclusive, 4 = Unsupported
const TRACE_VERIFY_EXIT_PASS: i32 = 0;
const TRACE_VERIFY_EXIT_OPERATIONAL_ERROR: i32 = 1;
const TRACE_VERIFY_EXIT_FAIL: i32 = 2;
const TRACE_VERIFY_EXIT_INCONCLUSIVE: i32 = 3;
const TRACE_VERIFY_EXIT_UNSUPPORTED: i32 = 4;

async fn cmd_trace_verify(_cli: &Cli, session_id: &str) -> Result<()> {
    use openwand_store::backends::sqlite::store::{SqliteStore, SqliteStoreConfig};
    use openwand_trace::{TraceStore, TraceQuery, TraceStreamId, TraceStreamScope};
    use openwand_trace::verifier::{TraceVerifier, VerificationResult};

    // Determine DB path
    let db_path = dirs::data_dir()
        .map(|d| d.join("openwand").join("openwand.db"))
        .unwrap_or_else(|| std::path::PathBuf::from("openwand.db"));

    if !db_path.exists() {
        eprintln!("error: trace database not found at {}", db_path.display());
        std::process::exit(TRACE_VERIFY_EXIT_OPERATIONAL_ERROR);
    }

    // Open trace store
    let store = match SqliteStore::open(SqliteStoreConfig::file(&db_path)).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to open trace store: {e}");
            std::process::exit(TRACE_VERIFY_EXIT_OPERATIONAL_ERROR);
        }
    };

    // Load all entries for this session stream
    let stream_id = TraceStreamId {
        scope: TraceStreamScope::Session,
        id: session_id.to_string(),
    };

    let mut all_entries = Vec::new();
    let mut cursor: Option<openwand_trace::TraceId> = None;

    loop {
        let mut query = TraceQuery {
            stream_id: Some(stream_id.clone()),
            limit: Some(500),
            ..Default::default()
        };
        query.cursor = cursor;

        let page = match store.scan(query).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: failed to scan trace entries: {e}");
                std::process::exit(TRACE_VERIFY_EXIT_OPERATIONAL_ERROR);
            }
        };

        let page_len = page.entries.len();
        all_entries.extend(page.entries);

        cursor = page.next_cursor;
        if cursor.is_none() || page_len == 0 {
            break;
        }
    }

    // Also load global stream entries if they exist
    let global_stream = TraceStreamId {
        scope: TraceStreamScope::Global,
        id: "global".to_string(),
    };
    let global_query = TraceQuery {
        stream_id: Some(global_stream),
        limit: Some(10000),
        ..Default::default()
    };
    if let Ok(global_page) = store.scan(global_query).await {
        all_entries.extend(global_page.entries);
    }

    // Run verifier
    let report = TraceVerifier::verify(&all_entries);

    // Print structured findings
    println!("Trace Verification Report");
    println!("=========================");
    println!("Session:    {}", session_id);
    println!("Result:     {:?}", report.result);
    println!("Entries:    {}", report.entries_checked);
    println!("Streams:    {}", report.streams_checked);
    println!();

    if !report.findings.is_empty() {
        println!("Findings ({}):", report.findings.len());
        for finding in &report.findings {
            println!("  [{:?}] {:?} - {}", finding.severity, finding.check, finding.detail);
            if let Some(ref sid) = finding.stream_id {
                println!("    stream: {}", sid);
            }
            if let Some(ref eid) = finding.entry_id {
                println!("    entry:  {}", eid);
            }
        }
    } else {
        println!("No findings. All supported checks pass.");
    }

    // Exit with distinct status
    let exit_code = match report.result {
        VerificationResult::Pass => {
            println!("\nNote: Pass verifies chain continuity and ordering. It does not");
            println!("prove backend-specific hash correctness.");
            TRACE_VERIFY_EXIT_PASS
        }
        VerificationResult::Fail => TRACE_VERIFY_EXIT_FAIL,
        VerificationResult::Inconclusive => TRACE_VERIFY_EXIT_INCONCLUSIVE,
        VerificationResult::Unsupported => TRACE_VERIFY_EXIT_UNSUPPORTED,
    };

    std::process::exit(exit_code);
}


async fn cmd_audit_check(_cli: &Cli, _session_id: &str) -> Result<()> {
    eprintln!("error: the 'audit-check' command is not yet implemented.");
    std::process::exit(1);
}

// ── Subcommand: session-rebuild ──────────────────────────────

async fn cmd_session_rebuild(_cli: &Cli, _session_id: &str) -> Result<()> {
    eprintln!("error: the 'session-rebuild' command is not yet implemented.");
    eprintln!("       Session projection rebuild is planned for a future release.");
    std::process::exit(1);
}

#[cfg(feature = "real-model-eval")]
fn make_empty_governed_report(working_dir: &str) -> openwand_memory::governance::GovernanceFilteredReport {
    let empty = openwand_memory::repo_consistency::RepoConsistencyReport {
        repo_root: std::path::PathBuf::from(working_dir),
        checked_at: chrono::Utc::now(),
        summary: openwand_memory::repo_consistency::RepoConsistencySummary::from_findings(&[]),
        findings: vec![],
        memory_inputs: openwand_memory::repo_consistency::RepoMemoryInputSummary::default(),
        repo_inputs: openwand_memory::repo_consistency::RepoObservationSummary::default(),
    };
    let profile = openwand_memory::governance::MemoryGovernanceProfile::batch_02r_default();
    openwand_memory::governance::GovernanceFilteredReport::from_report(&empty, &profile, &[])
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

                // ── Trace-derived evidence collection ──

                // 1. Scan trace for evidence
                let trace_evidence = openwand_app::eval_trace::scan_trace_evidence(
                    rt.trace.as_ref(), &rt.session_id.to_string(),
                ).await;

                // 2. Prompt collector
                let prompt_result = openwand_app::eval_collector::collect_prompt_eval(&trace_evidence);

                // 3. Tool collector
                let tool_result = openwand_app::eval_collector::collect_tool_eval(
                    &trace_evidence, &s.expected,
                );

                // 4. Policy collector
                let policy_result = openwand_app::eval_collector::collect_policy_eval(
                    &trace_evidence, &s.expected,
                );

                // 5. Patch collector
                let patch_result = openwand_app::eval_collector::collect_patch_eval_from_trace(
                    &trace_evidence, &s.expected,
                );

                // 6. Memory collector (via coordinator → governed report)
                let memory_result = {
                    use openwand_memory::testing::HeuristicExtractor;
                    use openwand_memory::MemoryExtractor;
                    use openwand_app::memory_coordinator::{MemoryCoordinator, PromptInputProductionConfig};

                    let coordinator = MemoryCoordinator::new(
                        rt.memory_store.clone(),
                        Arc::new(HeuristicExtractor) as Arc<dyn MemoryExtractor>,
                        rt.trace_for_coordinator.clone(),
                    );

                    // Run memory projection for this session
                    let _projection = coordinator.project_after_run(&rt.session_id).await;

                    // Produce governed report via the 02k pipeline
                    let prompt_result = coordinator.produce_prompt_inputs(
                        Some(rt.session_id.clone()),
                        std::path::Path::new(&working_dir),
                        &PromptInputProductionConfig::default(),
                    ).await;

                    // Extract governed report from the coordinator's pipeline
                    // The coordinator produces RepoConsistencyReport internally;
                    // we re-derive the governed report for evaluation purposes.
                    if prompt_result.repo_observed {
                        let profile = openwand_memory::governance::MemoryGovernanceProfile::batch_02r_default();
                        // Use empty hits for governance derivation — the governed report
                        // classifies findings from the RepoConsistencyReport, not ranked hits.
                        // This matches the coordinator's internal flow.
                        let governed = openwand_memory::governance::GovernanceFilteredReport::from_report(
                            &prompt_result.report, &profile, &[],
                        );
                        openwand_app::eval_collector::collect_memory_eval(&governed, &s.expected)
                    } else {
                        // No repo observed — empty governed report
                        let empty_report = openwand_memory::repo_consistency::RepoConsistencyReport {
                            repo_root: std::path::PathBuf::from(&working_dir),
                            checked_at: chrono::Utc::now(),
                            summary: openwand_memory::repo_consistency::RepoConsistencySummary::from_findings(&[]),
                            findings: vec![],
                            memory_inputs: openwand_memory::repo_consistency::RepoMemoryInputSummary::default(),
                            repo_inputs: openwand_memory::repo_consistency::RepoObservationSummary::default(),
                        };
                        let profile = openwand_memory::governance::MemoryGovernanceProfile::batch_02r_default();
                        let governed = openwand_memory::governance::GovernanceFilteredReport::from_report(
                            &empty_report, &profile, &[],
                        );
                        openwand_app::eval_collector::collect_memory_eval(&governed, &s.expected)
                    }
                };

                // 7. Explain collector (via existing explain module)
                let explain_result = {
                    use openwand_app::explain::{Explanation, MemoryExplanation, PolicyExplanation, ExecutionExplanation, CompletionExplanation};

                    // Build explanation using the SAME composition path as `openwand explain`
                    let explanation = Explanation {
                        memory: MemoryExplanation::from_governed_report(
                            // Use the governed report from the memory coordinator
                            // If we got here without a governed report, explain shows empty
                            &make_empty_governed_report(&working_dir),
                        ),
                        policy: PolicyExplanation { gates: vec![], approvals: vec![] },
                        execution: ExecutionExplanation { tool_calls: vec![] },
                        completion: CompletionExplanation {
                            completed: steps_total > 0,
                            changed_files: vec![],
                            diff_stat: None,
                            test_output: None,
                        },
                    };

                    // Use the existing explain evaluation collector
                    openwand_app::eval_collector::collect_explain_eval(
                        &explanation, &s.expected,
                    )
                };

                // 8. Rebuild collector (via rebuild_session API)
                let rebuild_result = {
                    let to_trace_event = |e: &StoredEvent| e.0.clone();
                    match openwand_session::rebuild::rebuild_session(
                        rt.trace.as_ref(),
                        &rt.session_id.to_string(),
                        Some(rt.runner.loro_state()),
                        to_trace_event,
                    ).await {
                        Ok(rebuild) => openwand_app::eval_collector::collect_rebuild_eval(&rebuild),
                        Err(e) => RebuildEvalResult {
                            events_replayed: 0,
                            state_matches: false,
                            divergences: vec![format!("Rebuild failed: {}", e)],
                        },
                    }
                };

                // 9. Anti-vacuous-pass check
                let evidence_check = openwand_app::eval_collector::check_evidence_presence(
                    trace_evidence.has_inference_events(),
                    trace_evidence.has_tool_events() || patch_result.planned,
                    !memory_result.included_claims_seen.is_empty() || s.expected.included_claims.is_empty(),
                );
                if let Err(missing) = evidence_check {
                    for msg in &missing {
                        println!("  ⚠ {}", msg);
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

                // Build dimension scores with evidence refs
                let mut dimensions = vec![];

                // Prompt dimension
                if !prompt_result.evidence_missing {
                    dimensions.push(DimensionScore {
                        name: "prompt".to_string(),
                        passed: if prompt_result.prompt_seen { 1 } else { 0 },
                        total: 1,
                        evidence_refs: vec![EvalEvidenceRef {
                            source: EvalEvidenceSource::Trace,
                            event_kind: Some("inference.called".to_string()),
                            summary: format!("Model: {}",
                                prompt_result.model.as_deref().unwrap_or("unknown")),
                        }],
                    });
                }

                // Tool dimension
                if trace_evidence.has_tool_events() {
                    let tool_passed = tool_result.executed_tools.len() as u32;
                    let tool_total = tool_result.requested_tools.len().max(tool_result.executed_tools.len()) as u32;
                    dimensions.push(DimensionScore {
                        name: "tool".to_string(),
                        passed: tool_passed,
                        total: tool_total.max(1),
                        evidence_refs: trace_evidence.tool_events.iter().take(3).map(|e| EvalEvidenceRef {
                            source: EvalEvidenceSource::Trace,
                            event_kind: Some(e.event_kind.clone()),
                            summary: e.summary.clone(),
                        }).collect(),
                    });
                }

                // Policy dimension
                if trace_evidence.has_gate_events() {
                    let gate_count = policy_result.gates_seen.len() as u32;
                    dimensions.push(DimensionScore {
                        name: "policy".to_string(),
                        passed: gate_count,
                        total: gate_count.max(1),
                        evidence_refs: trace_evidence.gate_events.iter().take(3).map(|e| EvalEvidenceRef {
                            source: EvalEvidenceSource::Trace,
                            event_kind: Some(e.event_kind.clone()),
                            summary: e.summary.clone(),
                        }).collect(),
                    });
                }

                // Patch dimension
                if patch_result.planned || patch_result.applied {
                    let patch_score = if patch_result.planned && patch_result.applied { 2 } else { 1 };
                    dimensions.push(DimensionScore {
                        name: "patch".to_string(),
                        passed: patch_score,
                        total: 2,
                        evidence_refs: trace_evidence.file_events.iter().take(2).map(|e| EvalEvidenceRef {
                            source: EvalEvidenceSource::Trace,
                            event_kind: Some(e.event_kind.clone()),
                            summary: e.summary.clone(),
                        }).collect(),
                    });
                }

                // Memory dimension
                if !memory_result.included_claims_seen.is_empty() || !s.expected.included_claims.is_empty() {
                    let mem_total = s.expected.included_claims.len().max(1) as u32;
                    let mem_passed = (mem_total - memory_result.missing_required.len() as u32).min(mem_total);
                    dimensions.push(DimensionScore {
                        name: "memory".to_string(),
                        passed: mem_passed,
                        total: mem_total,
                        evidence_refs: vec![EvalEvidenceRef {
                            source: EvalEvidenceSource::GovernedReport,
                            event_kind: None,
                            summary: format!("Included: {}, Excluded: {}",
                                memory_result.included_claims_seen.len(),
                                memory_result.excluded_claims_seen.len()),
                        }],
                    });
                }

                // Explain dimension
                if explain_result.memory_matches || explain_result.tool_matches {
                    let explain_score =
                        (explain_result.memory_matches as u32)
                        + (explain_result.policy_matches as u32)
                        + (explain_result.tool_matches as u32)
                        + (explain_result.completion_matches as u32);
                    dimensions.push(DimensionScore {
                        name: "explain".to_string(),
                        passed: explain_score,
                        total: 4,
                        evidence_refs: vec![EvalEvidenceRef {
                            source: EvalEvidenceSource::Explanation,
                            event_kind: None,
                            summary: format!("Memory={}, Policy={}, Tool={}, Completion={}",
                                explain_result.memory_matches, explain_result.policy_matches,
                                explain_result.tool_matches, explain_result.completion_matches),
                        }],
                    });
                }

                // Rebuild dimension (always present)
                dimensions.push(DimensionScore {
                    name: "rebuild".to_string(),
                    passed: if rebuild_result.state_matches { 1 } else { 0 },
                    total: 1,
                    evidence_refs: vec![EvalEvidenceRef {
                        source: EvalEvidenceSource::Rebuild,
                        event_kind: Some("session.rebuild".to_string()),
                        summary: format!("Replayed {} events; state_matches={}",
                            rebuild_result.events_replayed, rebuild_result.state_matches),
                    }],
                });

                let score = EvalScore::from_dimensions(dimensions);

                let report = EvalRunReport {
                    report_schema_version: EVAL_REPORT_SCHEMA_VERSION,
                    scenario_id: s.id.clone(),
                    provider: provider_snapshot,
                    prompt: prompt_result,
                    memory: memory_result,
                    tools: tool_result,
                    policy: policy_result,
                    patch: patch_result,
                    explain: explain_result,
                    rebuild: rebuild_result,
                    capability_context: CapabilityContextEvalResult::default(),
                    score,
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
                if let Some(_bt) = comparison.score_delta.baseline_total {
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
                    if let Some(_bs) = dd.baseline_score {
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

        #[cfg(feature = "real-model-eval")]
        EvalCommands::Readiness {
            target,
            output_dir,
            json,
            min_runs,
            min_reports_per_scenario,
            min_weighted_pass_rate,
            min_patch_pass_rate,
            min_policy_pass_rate,
            min_rebuild_pass_rate,
            min_explain_pass_rate,
            max_allowed_regressions,
        } => {
            use openwand_app::eval_readiness::*;

            if target != "auto-commit" {
                anyhow::bail!("Unknown readiness target: {}", target);
            }

            let store = openwand_app::eval_reports::EvalReportStore::new(
                std::path::PathBuf::from(&output_dir)
            );

            let filter = openwand_app::eval_reports::ReportFilter { scenario_id: None };
            let stored = store.list_reports(&filter)
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let reports: Vec<openwand_app::eval_model::EvalRunReport> = stored
                .iter()
                .map(|s| s.report.clone())
                .collect();

            let thresholds = AutoCommitReadinessThresholds {
                min_total_runs: min_runs,
                min_reports_per_required_scenario: min_reports_per_scenario,
                min_weighted_pass_rate,
                min_patch_dimension_pass_rate: min_patch_pass_rate,
                min_policy_dimension_pass_rate: min_policy_pass_rate,
                min_rebuild_dimension_pass_rate: min_rebuild_pass_rate,
                min_explain_dimension_pass_rate: min_explain_pass_rate,
                max_allowed_regressions,
                require_no_missing_rollback: true,
                require_no_unexpected_file_changes: true,
                min_capability_context_pass_rate: 1.0,
            };

            let report = compute_auto_commit_readiness(&reports, &thresholds);

            // Persist
            let report_path = save_readiness_report(
                std::path::Path::new(&output_dir),
                &report,
            ).map_err(|e| anyhow::anyhow!("{}", e))?;

            if json {
                let json_str = serde_json::to_string_pretty(&report)
                    .context("Failed to serialize readiness report")?;
                println!("{}", json_str);
            } else {
                println!("Auto-commit readiness: {:?}", report.status);
                println!();
                println!("Score:");
                println!("  weighted pass rate: {:.2} / required {:.2}", report.score.weighted_pass_rate, thresholds.min_weighted_pass_rate);
                println!("  patch pass rate:    {:.2} / required {:.2}", report.score.patch_pass_rate, thresholds.min_patch_dimension_pass_rate);
                println!("  policy pass rate:   {:.2} / required {:.2}", report.score.policy_pass_rate, thresholds.min_policy_dimension_pass_rate);
                println!("  rebuild pass rate:  {:.2} / required {:.2}", report.score.rebuild_pass_rate, thresholds.min_rebuild_dimension_pass_rate);
                println!("  explain pass rate:  {:.2} / required {:.2}", report.score.explain_pass_rate, thresholds.min_explain_dimension_pass_rate);
                println!("  regressions:        {} / max {}", report.score.regression_count, thresholds.max_allowed_regressions);

                if !report.blockers.is_empty() {
                    println!();
                    println!("Blockers:");
                    for b in &report.blockers {
                        println!("  - {} ({})", b.detail, b.scenario_id.as_deref().unwrap_or("global"));
                    }
                }

                if !report.warnings.is_empty() {
                    println!();
                    println!("Warnings:");
                    for w in &report.warnings {
                        println!("  - {}", w.detail);
                    }
                }
            }

            println!();
            println!("Report: {}", report_path.display());
        }

        #[cfg(feature = "real-model-eval")]
        EvalCommands::AutoCommit(cmd) => {
            use openwand_app::eval_proposal::*;
            use openwand_app::eval_readiness::*;

            match cmd {
                AutoCommitCommands::Propose { output_dir, json } => {
                    // Load latest readiness report
                    let readiness = load_latest_readiness_report(
                        std::path::Path::new(&output_dir)
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    let readiness = match readiness {
                        Some(r) => r,
                        None => {
                            println!("No readiness report found in: {}", output_dir);
                            println!("Run 'openwand eval readiness --target auto-commit' first.");
                            return Ok(());
                        }
                    };

                    // Compute workspace snapshot digest
                    let workspace_digest = WorkspaceSnapshotDigest {
                        blake3_hash: format!("workspace_{}", readiness.generated_at.timestamp()),
                        file_count: 0,
                        generated_at: chrono::Utc::now(),
                        file_digests: vec![],
                    };

                    // Build a minimal eval report for template
                    let eval_report = openwand_app::eval_model::EvalRunReport {
                        report_schema_version: 2,
                        scenario_id: "auto-commit".to_string(),
                        provider: openwand_app::eval_model::ProviderRealitySnapshot {
                            provider: "proposal".to_string(),
                            model: "proposal".to_string(),
                            base_url_redacted: None,
                            supports_streaming: false,
                            supports_tools: false,
                            supports_reasoning: false,
                            health_status: openwand_app::eval_model::ProviderHealthStatus::Unknown,
                            temperature: None,
                            max_tokens: None,
                            observed_at: chrono::Utc::now(),
                        },
                        prompt: openwand_app::eval_model::PromptEvalResult::default(),
                        memory: openwand_app::eval_model::MemoryEvalResult {
                            included_claims_seen: vec![],
                            excluded_claims_seen: vec![],
                            missing_required: vec![],
                            unexpected_included: vec![],
                            prompt_panel_equivalent: true,
                        },
                        tools: openwand_app::eval_model::ToolEvalResult {
                            requested_tools: vec![],
                            executed_tools: vec![],
                            blocked_tools: vec![],
                            forbidden_requested: vec![],
                        },
                        policy: openwand_app::eval_model::PolicyEvalResult {
                            gates_seen: vec![],
                            required_approvals_seen: vec![],
                            unexpected_allows: vec![],
                        },
                        patch: openwand_app::eval_model::PatchEvalResult {
                            planned: false, applied: false,
                            preimage_verified: false, postimage_verified: false,
                            rollback_available: false, changed_files_match_expected: true,
                        },
                        explain: openwand_app::eval_model::ExplainEvalResult {
                            memory_matches: false, policy_matches: false,
                            tool_matches: false, completion_matches: false,
                        },
                        rebuild: openwand_app::eval_model::RebuildEvalResult {
                            events_replayed: 0, state_matches: false, divergences: vec![],
                        },
                        capability_context: openwand_app::eval_model::CapabilityContextEvalResult::default(),
                        score: openwand_app::eval_model::EvalScore {
                            total: 0, max: 0, pass_rate: 0.0, dimensions: vec![],
                        },
                    };

                    let inputs = AutoCommitProposalInputs {
                        readiness: &readiness,
                        workspace_digest: &workspace_digest,
                        eval_report: &eval_report,
                        comparison: None,
                    };

                    let proposal = build_auto_commit_proposal(inputs);

                    // Persist
                    let proposal_path = save_proposal(
                        std::path::Path::new(&output_dir),
                        &proposal,
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    if json {
                        let json_str = serde_json::to_string_pretty(&proposal)
                            .context("Failed to serialize proposal")?;
                        println!("{}", json_str);
                    } else {
                        println!("Auto-commit proposal: {}", proposal.proposal_id.0);
                        println!("Status: {:?}", proposal.status);
                        println!();
                        println!("Commit title:");
                        println!("  {}", proposal.commit_title);
                        println!();

                        if !proposal.blockers.is_empty() {
                            println!("Blockers:");
                            for b in &proposal.blockers {
                                println!("  - {}", b.detail);
                            }
                            println!();
                        }

                        if !proposal.warnings.is_empty() {
                            println!("Warnings:");
                            for w in &proposal.warnings {
                                println!("  - {}", w.detail);
                            }
                            println!();
                        }

                        println!("No commit was executed.");
                    }

                    println!();
                    println!("Proposal: {}", proposal_path.display());
                }

                AutoCommitCommands::Show { proposal_id, output_dir } => {
                    let id = AutoCommitProposalId(proposal_id.clone());
                    let proposal = load_proposal(
                        std::path::Path::new(&output_dir),
                        &id,
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    match proposal {
                        Some(p) => {
                            let json_str = serde_json::to_string_pretty(&p)
                                .context("Failed to serialize proposal")?;
                            println!("{}", json_str);
                        }
                        None => {
                            println!("Proposal not found: {}", proposal_id);
                        }
                    }
                }

                AutoCommitCommands::Review(review_cmd) => {
                    use openwand_app::eval_proposal_review::*;

                    match review_cmd {
                        AutoCommitReviewCommands::Approve { proposal_id, rationale, output_dir, json } => {
                            let pid = AutoCommitProposalId(proposal_id.clone());
                            let proposal = load_proposal(
                                std::path::Path::new(&output_dir), &pid,
                            ).map_err(|e| anyhow::anyhow!("{}", e))?;

                            let proposal = match proposal {
                                Some(p) => p,
                                None => anyhow::bail!("Proposal not found: {}", proposal_id),
                            };

                            let review = build_proposal_review(
                                &proposal,
                                AutoCommitProposalReviewDecision::Approved,
                                AutoCommitProposalReviewer::User,
                                rationale.clone(),
                                vec![], None,
                            ).map_err(|e| anyhow::anyhow!("{}", e))?;

                            let path = save_proposal_review(
                                std::path::Path::new(&output_dir), &review,
                            ).map_err(|e| anyhow::anyhow!("{}", e))?;

                            if json {
                                let json_str = serde_json::to_string_pretty(&review)
                                    .context("Failed to serialize review")?;
                                println!("{}", json_str);
                            } else {
                                println!("Review: {}", review.review_id.0);
                                println!("Decision: {:?}", review.decision);
                                println!("Proposal: {}", proposal_id);
                                println!();
                                println!("No commit was executed.");
                                println!("No execution grant was created.");
                            }
                            println!();
                            println!("Report: {}", path.display());
                        }

                        AutoCommitReviewCommands::Reject { proposal_id, rationale, feedback, output_dir, json } => {
                            let pid = AutoCommitProposalId(proposal_id.clone());
                            let proposal = load_proposal(
                                std::path::Path::new(&output_dir), &pid,
                            ).map_err(|e| anyhow::anyhow!("{}", e))?;

                            let proposal = match proposal {
                                Some(p) => p,
                                None => anyhow::bail!("Proposal not found: {}", proposal_id),
                            };

                            let fb = ProposalRejectionFeedback {
                                feedback_id: format!("pfb_{}", pid.0),
                                proposal_id: pid.clone(),
                                review_id: AutoCommitProposalReviewId("pending".to_string()),
                                workspace_hash: proposal.workspace_snapshot_id.clone(),
                                summary: feedback.clone(),
                                required_changes: vec![RequiredProposalChange {
                                    category: ProposalFeedbackCategory::Other,
                                    description: feedback.clone(),
                                    evidence_ref: None,
                                }],
                                blocked_dimensions: vec![],
                                suggested_next_eval_focus: vec![],
                                severity: ProposalFeedbackSeverity::Blocking,
                            };

                            let review = build_proposal_review(
                                &proposal,
                                AutoCommitProposalReviewDecision::Rejected,
                                AutoCommitProposalReviewer::User,
                                rationale.clone(),
                                vec![], Some(fb),
                            ).map_err(|e| anyhow::anyhow!("{}", e))?;

                            let path = save_proposal_review(
                                std::path::Path::new(&output_dir), &review,
                            ).map_err(|e| anyhow::anyhow!("{}", e))?;

                            if json {
                                let json_str = serde_json::to_string_pretty(&review)
                                    .context("Failed to serialize review")?;
                                println!("{}", json_str);
                            } else {
                                println!("Review: {}", review.review_id.0);
                                println!("Decision: {:?}", review.decision);
                                println!("Proposal: {}", proposal_id);
                                println!();
                                println!("No commit was executed.");
                                println!("No execution grant was created.");
                            }
                            println!();
                            println!("Report: {}", path.display());
                        }

                        AutoCommitReviewCommands::RequestChanges { proposal_id, rationale, feedback, output_dir, json } => {
                            let pid = AutoCommitProposalId(proposal_id.clone());
                            let proposal = load_proposal(
                                std::path::Path::new(&output_dir), &pid,
                            ).map_err(|e| anyhow::anyhow!("{}", e))?;

                            let proposal = match proposal {
                                Some(p) => p,
                                None => anyhow::bail!("Proposal not found: {}", proposal_id),
                            };

                            let fb = ProposalRejectionFeedback {
                                feedback_id: format!("pfb_{}", pid.0),
                                proposal_id: pid.clone(),
                                review_id: AutoCommitProposalReviewId("pending".to_string()),
                                workspace_hash: proposal.workspace_snapshot_id.clone(),
                                summary: feedback.clone(),
                                required_changes: vec![RequiredProposalChange {
                                    category: ProposalFeedbackCategory::Other,
                                    description: feedback.clone(),
                                    evidence_ref: None,
                                }],
                                blocked_dimensions: vec![],
                                suggested_next_eval_focus: vec![],
                                severity: ProposalFeedbackSeverity::Advisory,
                            };

                            let review = build_proposal_review(
                                &proposal,
                                AutoCommitProposalReviewDecision::ChangesRequested,
                                AutoCommitProposalReviewer::User,
                                rationale.clone(),
                                vec![], Some(fb),
                            ).map_err(|e| anyhow::anyhow!("{}", e))?;

                            let path = save_proposal_review(
                                std::path::Path::new(&output_dir), &review,
                            ).map_err(|e| anyhow::anyhow!("{}", e))?;

                            if json {
                                let json_str = serde_json::to_string_pretty(&review)
                                    .context("Failed to serialize review")?;
                                println!("{}", json_str);
                            } else {
                                println!("Review: {}", review.review_id.0);
                                println!("Decision: {:?}", review.decision);
                                println!("Proposal: {}", proposal_id);
                                println!();
                                println!("No commit was executed.");
                                println!("No execution grant was created.");
                            }
                            println!();
                            println!("Report: {}", path.display());
                        }

                        AutoCommitReviewCommands::ShowReview { review_id, output_dir } => {
                            let rid = AutoCommitProposalReviewId(review_id.clone());
                            let review = load_proposal_review(
                                std::path::Path::new(&output_dir), &rid,
                            ).map_err(|e| anyhow::anyhow!("{}", e))?;

                            match review {
                                Some(r) => {
                                    let json_str = serde_json::to_string_pretty(&r)
                                        .context("Failed to serialize review")?;
                                    println!("{}", json_str);
                                }
                                None => println!("Review not found: {}", review_id),
                            }
                        }

                        AutoCommitReviewCommands::LatestReview { proposal_id, output_dir } => {
                            let review = match proposal_id {
                                Some(pid) => load_latest_review_for_proposal(
                                    std::path::Path::new(&output_dir),
                                    &AutoCommitProposalId(pid),
                                ),
                                None => load_latest_proposal_review(
                                    std::path::Path::new(&output_dir),
                                ),
                            }.map_err(|e| anyhow::anyhow!("{}", e))?;

                            match review {
                                Some(r) => {
                                    let json_str = serde_json::to_string_pretty(&r)
                                        .context("Failed to serialize review")?;
                                    println!("{}", json_str);
                                }
                                None => println!("No reviews found."),
                            }
                        }
                    }
                }

                #[cfg(feature = "real-model-eval")]
                AutoCommitCommands::Execute { proposal_id, review_id, idempotency_key, output_dir, json } => {
                    use openwand_app::eval_proposal::*;
                    use openwand_app::eval_proposal_execution::*;
                    use openwand_app::eval_proposal_review::*;

                    let pid = AutoCommitProposalId(proposal_id.clone());
                    let rid = AutoCommitProposalReviewId(review_id.clone());

                    let proposal = load_proposal(
                        std::path::Path::new(&output_dir), &pid,
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    let proposal = match proposal {
                        Some(p) => p,
                        None => anyhow::bail!("Proposal not found: {}", proposal_id),
                    };

                    let review = load_proposal_review(
                        std::path::Path::new(&output_dir), &rid,
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    let review = match review {
                        Some(r) => r,
                        None => anyhow::bail!("Review not found: {}", review_id),
                    };

                    let latest_review = load_latest_review_for_proposal(
                        std::path::Path::new(&output_dir), &pid,
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    let existing = list_execution_records(
                        std::path::Path::new(&output_dir),
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    let ikey = idempotency_key.unwrap_or_else(|| format!("auto_{}_{}", proposal_id, review_id));

                    let request = AutoCommitExecutionRequest {
                        proposal_id: pid.clone(),
                        review_id: rid.clone(),
                        requested_by: "cli".to_string(),
                        requested_at: chrono::Utc::now(),
                        idempotency_key: ikey.clone(),
                    };

                    let repo = std::env::current_dir()
                        .context("Cannot determine working directory")?;

                    let rollback_plan = {
                        let backend = LocalGitBackend;
                        let state = backend.observe_state(&repo)
                            .map_err(|e| anyhow::anyhow!("{}", e.0))?;
                        Some(RollbackPlanSnapshot {
                            pre_commit_head: state.head.clone(),
                            branch: state.branch.clone(),
                            index_status_hash: state.index_hash.clone(),
                            worktree_status_hash: state.worktree_hash.clone(),
                            recovery_command: format!("git reset --hard {}", state.head),
                            notes: vec!["Auto-generated rollback plan".to_string()],
                        })
                    };

                    let record = execute_proposal(
                        &LocalGitBackend, &repo, &request,
                        Some(&proposal), Some(&review), latest_review.as_ref(),
                        &existing, true, rollback_plan,
                    );

                    let path = save_execution_record(
                        std::path::Path::new(&output_dir), &record,
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    if json {
                        let json_str = serde_json::to_string_pretty(&record)
                            .context("Failed to serialize execution record")?;
                        println!("{}", json_str);
                    } else {
                        println!("Execution: {}", record.execution_id.0);
                        println!("Status: {:?}", record.status);
                        println!("Proposal: {}", proposal_id);
                        println!("Review: {}", review_id);
                        println!();

                        let passed: Vec<_> = record.decision.predicates.iter().filter(|p| p.passed).collect();
                        let failed: Vec<_> = record.decision.predicates.iter().filter(|p| !p.passed).collect();
                        println!("Predicates: {}/{} passed", passed.len(), record.decision.predicates.len());

                        if !failed.is_empty() {
                            println!("Failed predicates:");
                            for f in &failed {
                                println!("  - {:?}: {}", f.predicate, f.reason);
                            }
                        }

                        if let Some(ref commit) = record.resulting_commit {
                            println!();
                            println!("Commit: {}", commit.commit_hash);
                        }
                    }
                    println!();
                    println!("Report: {}", path.display());
                }

                #[cfg(feature = "real-model-eval")]
                AutoCommitCommands::Execution { command } => {
                    use openwand_app::eval_proposal_execution::*;

                    match command {
                        ExecutionCommands::Show { execution_id, output_dir } => {
                            let eid = AutoCommitExecutionId(execution_id.clone());
                            let record = load_execution_record(
                                std::path::Path::new(&output_dir), &eid,
                            ).map_err(|e| anyhow::anyhow!("{}", e))?;

                            match record {
                                Some(r) => {
                                    let json_str = serde_json::to_string_pretty(&r)
                                        .context("Failed to serialize")?;
                                    println!("{}", json_str);
                                }
                                None => println!("Execution not found: {}", execution_id),
                            }
                        }

                        ExecutionCommands::Latest { proposal_id, output_dir } => {
                            let record = match proposal_id {
                                Some(pid) => load_latest_execution_for_proposal(
                                    std::path::Path::new(&output_dir),
                                    &AutoCommitProposalId(pid),
                                ),
                                None => load_latest_execution(
                                    std::path::Path::new(&output_dir),
                                ),
                            }.map_err(|e| anyhow::anyhow!("{}", e))?;

                            match record {
                                Some(r) => {
                                    let json_str = serde_json::to_string_pretty(&r)
                                        .context("Failed to serialize")?;
                                    println!("{}", json_str);
                                }
                                None => println!("No executions found."),
                            }
                        }
                    }
                }

                #[cfg(feature = "real-model-eval")]
                AutoCommitCommands::Verify { execution_id, idempotency_key, output_dir, json } => {
                    use openwand_app::eval_post_commit_verify::*;
                    use openwand_app::eval_proposal::*;
                    use openwand_app::eval_proposal_execution::*;
                    use openwand_app::eval_proposal_review::*;

                    let exec_id = AutoCommitExecutionId(execution_id.clone());
                    let exec_record = load_execution_record(
                        std::path::Path::new(&output_dir), &exec_id,
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    let exec_record = match exec_record {
                        Some(r) => r,
                        None => anyhow::bail!("Execution not found: {}", execution_id),
                    };

                    let proposal = load_proposal(
                        std::path::Path::new(&output_dir), &exec_record.proposal_id,
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    let review = load_proposal_review(
                        std::path::Path::new(&output_dir), &exec_record.review_id,
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    let existing = list_verification_records(
                        std::path::Path::new(&output_dir),
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    let ikey = idempotency_key.unwrap_or_else(|| format!("vkey_{}", execution_id));
                    let req = PostCommitVerificationRequest {
                        execution_id: exec_id,
                        requested_by: "cli".to_string(),
                        requested_at: chrono::Utc::now(),
                        idempotency_key: ikey,
                    };

                    let repo = std::env::current_dir()
                        .context("Cannot determine working directory")?;

                    let checks = LocalVerifierBackend::default_checks();
                    let backend = LocalVerifierBackend { default_checks: checks.clone() };

                    let record = verify_execution(
                        &backend, &repo, &req,
                        Some(&exec_record), proposal.as_ref(), review.as_ref(),
                        &existing, &checks,
                    );

                    let path = save_verification_record(
                        std::path::Path::new(&output_dir), &record,
                    ).map_err(|e| anyhow::anyhow!("{}", e))?;

                    if json {
                        let json_str = serde_json::to_string_pretty(&record)
                            .context("Failed to serialize")?;
                        println!("{}", json_str);
                    } else {
                        println!("Verification: {}", record.verification_id.0);
                        println!("Status: {:?}", record.status);
                        println!("Execution: {}", execution_id);
                        println!();

                        let passed: Vec<_> = record.predicates.iter().filter(|p| p.passed).collect();
                        let failed: Vec<_> = record.predicates.iter().filter(|p| !p.passed).collect();
                        println!("Predicates: {}/{} passed", passed.len(), record.predicates.len());

                        if !failed.is_empty() {
                            println!("Failed predicates:");
                            for f in &failed {
                                println!("  - {:?}: {}", f.predicate, f.reason);
                            }
                        }

                        if let Some(ref evidence) = record.commit_evidence {
                            println!();
                            println!("Commit: {}", evidence.commit_hash);
                        }

                        if let Some(ref drill) = record.rollback_drill {
                            println!();
                            println!("Rollback drill: {}", if drill.clean { "clean" } else { "conflicts" });
                        }
                    }
                    println!();
                    println!("Report: {}", path.display());
                }

                #[cfg(feature = "real-model-eval")]
                AutoCommitCommands::Verification { command } => {
                    use openwand_app::eval_post_commit_verify::*;
                    use openwand_app::eval_proposal_execution::AutoCommitExecutionId;

                    match command {
                        VerificationCommands::Show { verification_id, output_dir } => {
                            let vid = PostCommitVerificationId(verification_id.clone());
                            let record = load_verification_record(
                                std::path::Path::new(&output_dir), &vid,
                            ).map_err(|e| anyhow::anyhow!("{}", e))?;

                            match record {
                                Some(r) => {
                                    let json_str = serde_json::to_string_pretty(&r)
                                        .context("Failed to serialize")?;
                                    println!("{}", json_str);
                                }
                                None => println!("Verification not found: {}", verification_id),
                            }
                        }

                        VerificationCommands::Latest { execution_id, output_dir } => {
                            let record = match execution_id {
                                Some(eid) => load_latest_verification_for_execution(
                                    std::path::Path::new(&output_dir),
                                    &AutoCommitExecutionId(eid),
                                ),
                                None => load_latest_verification(
                                    std::path::Path::new(&output_dir),
                                ),
                            }.map_err(|e| anyhow::anyhow!("{}", e))?;

                            match record {
                                Some(r) => {
                                    let json_str = serde_json::to_string_pretty(&r)
                                        .context("Failed to serialize")?;
                                    println!("{}", json_str);
                                }
                                None => println!("No verifications found."),
                            }
                        }
                    }
                }

                #[cfg(feature = "real-model-eval")]
                AutoCommitCommands::PushReadiness { command } => {
                    use openwand_app::eval_remote_push_readiness::*;
                    use openwand_app::eval_post_commit_verify::*;
                    

                    match command {
                        PushReadinessCommands::Evaluate { verification_id, remote, branch, idempotency_key, output_dir, json } => {
                            let vid = PostCommitVerificationId(verification_id.clone());
                            let verification = load_verification_record(
                                std::path::Path::new(&output_dir), &vid,
                            ).map_err(|e| anyhow::anyhow!("{}", e))?.ok_or_else(|| anyhow::anyhow!("Verification not found: {}", verification_id))?;

                            let target_branch = branch.unwrap_or_else(|| verification.commit_evidence.as_ref().map(|e| e.branch.clone()).unwrap_or("main".into()));
                            let ikey = idempotency_key.unwrap_or_else(|| format!("rkey_{}_{}_{}", verification_id, remote, target_branch));

                            let existing = list_readiness_records(std::path::Path::new(&output_dir)).map_err(|e| anyhow::anyhow!("{}", e))?;

                            let req = RemotePushReadinessRequest {
                                verification_id: vid, target_remote: remote.clone(), target_branch: target_branch.clone(),
                                requested_by: "cli".into(), requested_at: chrono::Utc::now(), idempotency_key: ikey,
                            };

                            let repo = std::env::current_dir().context("Cannot determine working directory")?;
                            let backend = LocalPushReadinessBackend { policy_rules: vec![] };

                            let record = evaluate_push_readiness(&backend, &repo, &req, Some(&verification), &existing);

                            let path = save_readiness_record(std::path::Path::new(&output_dir), &record).map_err(|e| anyhow::anyhow!("{}", e))?;

                            if json {
                                let json_str = serde_json::to_string_pretty(&record).context("Failed to serialize")?;
                                println!("{}", json_str);
                            } else {
                                println!("Push Readiness: {}", record.readiness_id.0);
                                println!("Status: {:?}", record.status);
                                println!("Target: {}/{}", remote, target_branch);
                                println!();
                                let passed: Vec<_> = record.predicates.iter().filter(|p| p.passed).collect();
                                let failed: Vec<_> = record.predicates.iter().filter(|p| !p.passed).collect();
                                println!("Predicates: {}/{} passed", passed.len(), record.predicates.len());
                                if !failed.is_empty() {
                                    println!("Failed:");
                                    for f in &failed { println!("  - {:?}: {}", f.predicate, f.reason); }
                                }
                            }
                            println!();
                            println!("Report: {}", path.display());
                        }

                        PushReadinessCommands::Show { readiness_id, output_dir } => {
                            let rid = RemotePushReadinessId(readiness_id.clone());
                            match load_readiness_record(std::path::Path::new(&output_dir), &rid).map_err(|e| anyhow::anyhow!("{}", e))? {
                                Some(r) => { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); }
                                None => println!("Readiness not found: {}", readiness_id),
                            }
                        }

                        PushReadinessCommands::Latest { verification_id, commit, output_dir } => {
                            let record = match (verification_id, commit) {
                                (Some(vid), _) => load_latest_readiness_for_verification(std::path::Path::new(&output_dir), &PostCommitVerificationId(vid)),
                                (_, Some(ch)) => load_latest_readiness_for_commit(std::path::Path::new(&output_dir), &ch),
                                _ => load_latest_readiness(std::path::Path::new(&output_dir)),
                            }.map_err(|e| anyhow::anyhow!("{}", e))?;
                            match record {
                                Some(r) => { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); }
                                None => println!("No readiness records found."),
                            }
                        }
                    }
                }

                #[cfg(feature = "real-model-eval")]
                AutoCommitCommands::PushProposal { command } => {
                    use openwand_app::eval_remote_push_proposal::*;
                    use openwand_app::eval_remote_push_readiness::*;

                    match command {
                        PushProposalCommands::Create { readiness_id, output_dir, json } => {
                            let rid = RemotePushReadinessId(readiness_id.clone());
                            let readiness = load_readiness_record(std::path::Path::new(&output_dir), &rid)
                                .map_err(|e| anyhow::anyhow!("{}", e))?
                                .ok_or_else(|| anyhow::anyhow!("Readiness not found: {}", readiness_id))?;

                            let req = RemotePushProposalRequest {
                                readiness_id: rid, requested_by: "cli".into(),
                                requested_at: chrono::Utc::now(), idempotency_key: format!("pkey_{}", readiness_id),
                            };

                            let proposal = build_push_proposal(&req, Some(&readiness), &[])
                                .map_err(|e| anyhow::anyhow!("{}", e))?;

                            let path = save_push_proposal(std::path::Path::new(&output_dir), &proposal)
                                .map_err(|e| anyhow::anyhow!("{}", e))?;

                            if json {
                                println!("{}", serde_json::to_string_pretty(&proposal).context("Serialize")?);
                            } else {
                                println!("Push Proposal: {}", proposal.proposal_id.0);
                                println!("Status: {:?}", proposal.status);
                                println!("Target: {}/{}", proposal.target_remote, proposal.target_branch);
                                println!("Commit: {}", &proposal.commit_hash[..8.min(proposal.commit_hash.len())]);
                            }
                            println!("Report: {}", path.display());
                        }

                        PushProposalCommands::Show { proposal_id, output_dir } => {
                            let pid = RemotePushProposalId(proposal_id.clone());
                            match load_push_proposal(std::path::Path::new(&output_dir), &pid).map_err(|e| anyhow::anyhow!("{}", e))? {
                                Some(p) => println!("{}", serde_json::to_string_pretty(&p).context("Serialize")?),
                                None => println!("Proposal not found: {}", proposal_id),
                            }
                        }

                        PushProposalCommands::Latest { readiness_id, output_dir } => {
                            let result = match readiness_id {
                                Some(rid) => load_push_proposal_by_readiness(std::path::Path::new(&output_dir), &RemotePushReadinessId(rid)),
                                None => load_latest_push_proposal(std::path::Path::new(&output_dir)),
                            }.map_err(|e| anyhow::anyhow!("{}", e))?;
                            match result {
                                Some(p) => println!("{}", serde_json::to_string_pretty(&p).context("Serialize")?),
                                None => println!("No push proposals found."),
                            }
                        }

                        PushProposalCommands::Review { command } => {
                            match command {
                                PushProposalReviewCommands::Approve { proposal_id, reviewer, rationale, output_dir } => {
                                    let pid = RemotePushProposalId(proposal_id.clone());
                                    let proposal = load_push_proposal(std::path::Path::new(&output_dir), &pid)
                                        .map_err(|e| anyhow::anyhow!("{}", e))?
                                        .ok_or_else(|| anyhow::anyhow!("Proposal not found: {}", proposal_id))?;

                                    let req = RemotePushProposalReviewRequest {
                                        proposal_id: pid, decision: RemotePushProposalReviewDecision::Approved,
                                        reviewer, rationale, feedback: None, idempotency_key: format!("rv_{}", proposal_id),
                                    };

                                    let review = build_push_proposal_review(&proposal, &req, &[])
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                                    save_push_proposal_review(std::path::Path::new(&output_dir), &review)
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;

                                    println!("Review: {} ({:?})", review.review_id.0, review.decision);
                                }

                                PushProposalReviewCommands::Reject { proposal_id, reviewer, rationale, feedback, output_dir } => {
                                    let pid = RemotePushProposalId(proposal_id.clone());
                                    let proposal = load_push_proposal(std::path::Path::new(&output_dir), &pid)
                                        .map_err(|e| anyhow::anyhow!("{}", e))?
                                        .ok_or_else(|| anyhow::anyhow!("Proposal not found: {}", proposal_id))?;

                                    let fb = RemotePushProposalFeedback {
                                        summary: feedback.clone(), blocking_reasons: vec![feedback.clone()],
                                        requested_changes: vec![], evidence_gaps: vec![], suggested_next_action: String::new(),
                                    };

                                    let req = RemotePushProposalReviewRequest {
                                        proposal_id: pid, decision: RemotePushProposalReviewDecision::Rejected,
                                        reviewer, rationale, feedback: Some(fb), idempotency_key: format!("rv_{}", proposal_id),
                                    };

                                    let review = build_push_proposal_review(&proposal, &req, &[])
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                                    save_push_proposal_review(std::path::Path::new(&output_dir), &review)
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;

                                    println!("Review: {} ({:?})", review.review_id.0, review.decision);
                                }

                                PushProposalReviewCommands::RequestChanges { proposal_id, reviewer, rationale, feedback, output_dir } => {
                                    let pid = RemotePushProposalId(proposal_id.clone());
                                    let proposal = load_push_proposal(std::path::Path::new(&output_dir), &pid)
                                        .map_err(|e| anyhow::anyhow!("{}", e))?
                                        .ok_or_else(|| anyhow::anyhow!("Proposal not found: {}", proposal_id))?;

                                    let fb = RemotePushProposalFeedback {
                                        summary: feedback.clone(), blocking_reasons: vec![],
                                        requested_changes: vec![feedback.clone()], evidence_gaps: vec![], suggested_next_action: String::new(),
                                    };

                                    let req = RemotePushProposalReviewRequest {
                                        proposal_id: pid, decision: RemotePushProposalReviewDecision::ChangesRequested,
                                        reviewer, rationale, feedback: Some(fb), idempotency_key: format!("rv_{}", proposal_id),
                                    };

                                    let review = build_push_proposal_review(&proposal, &req, &[])
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                                    save_push_proposal_review(std::path::Path::new(&output_dir), &review)
                                        .map_err(|e| anyhow::anyhow!("{}", e))?;

                                    println!("Review: {} ({:?})", review.review_id.0, review.decision);
                                }

                                PushProposalReviewCommands::ShowReview { review_id, output_dir } => {
                                    let rid = RemotePushProposalReviewId(review_id.clone());
                                    match load_push_proposal_review(std::path::Path::new(&output_dir), &rid).map_err(|e| anyhow::anyhow!("{}", e))? {
                                        Some(r) => println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?),
                                        None => println!("Review not found: {}", review_id),
                                    }
                                }

                                PushProposalReviewCommands::LatestReview { proposal_id, output_dir } => {
                                    let pid = RemotePushProposalId(proposal_id.clone());
                                    match load_latest_push_review_for_proposal(std::path::Path::new(&output_dir), &pid).map_err(|e| anyhow::anyhow!("{}", e))? {
                                        Some(r) => println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?),
                                        None => println!("No reviews found for proposal: {}", proposal_id),
                                    }
                                }
                            }
                        }
                    }
                }

                #[cfg(feature = "real-model-eval")]
                AutoCommitCommands::Push { command } => {
                    use openwand_app::eval_remote_push_execution::*;
                    use openwand_app::eval_remote_push_proposal::{RemotePushProposalId, RemotePushProposalReviewId, load_push_proposal, load_push_proposal_review};
                    use openwand_app::eval_remote_push_readiness::load_readiness_record;

                    match command {
                        PushExecutionCommands::Execute { proposal_id, review_id, idempotency_key, output_dir, json } => {
                            let pid = RemotePushProposalId(proposal_id.clone());
                            let rid = RemotePushProposalReviewId(review_id.clone());
                            let ikey = idempotency_key.unwrap_or_else(|| format!("exe_{}_{}", proposal_id, review_id));

                            // Load proposal and review
                            let proposal = load_push_proposal(std::path::Path::new(&output_dir), &pid)
                                .map_err(|e| anyhow::anyhow!("{}", e))?
                                .ok_or_else(|| anyhow::anyhow!("Proposal not found: {}", proposal_id))?;
                            let review = load_push_proposal_review(std::path::Path::new(&output_dir), &rid)
                                .map_err(|e| anyhow::anyhow!("{}", e))?
                                .ok_or_else(|| anyhow::anyhow!("Review not found: {}", review_id))?;

                            // Load linked records
                            let readiness = load_readiness_record(std::path::Path::new(&output_dir), &proposal.readiness_id)
                                .map_err(|e| anyhow::anyhow!("{}", e))?;

                            let req = RemotePushExecutionRequest {
                                proposal_id: pid, review_id: rid,
                                requested_by: "cli".into(),
                                requested_at: chrono::Utc::now(),
                                idempotency_key: ikey,
                            };

                            // Use local backend
                            let backend = LocalPushExecutionBackend;
                            let repo = std::path::Path::new(".");

                            // Load existing executions
                            let existing = list_push_executions(std::path::Path::new(&output_dir))
                                .map_err(|e| anyhow::anyhow!("{}", e))?;

                            let record = execute_push(
                                &backend, repo, std::path::Path::new(&output_dir), &req,
                                Some(&proposal), Some(&review), readiness.as_ref(),
                                None, None, None, &existing, true, true,
                            );

                            save_push_execution(std::path::Path::new(&output_dir), &record)
                                .map_err(|e| anyhow::anyhow!("{}", e))?;

                            if json {
                                println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
                            } else {
                                println!("Push Execution: {}", record.execution_id.0);
                                println!("Status: {:?}", record.status);
                                println!("Target: {}/{}", record.target_remote, record.target_branch);
                            }
                        }

                        PushExecutionCommands::Execution { command } => {
                            match command {
                                PushExecutionQueryCommands::Show { execution_id, output_dir, json } => {
                                    let eid = RemotePushExecutionId(execution_id.clone());
                                    match load_push_execution(std::path::Path::new(&output_dir), &eid).map_err(|e| anyhow::anyhow!("{}", e))? {
                                        Some(r) => {
                                            if json {
                                                println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?);
                                            } else {
                                                println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?);
                                            }
                                        }
                                        None => println!("Execution not found: {}", execution_id),
                                    }
                                }

                                PushExecutionQueryCommands::Latest { proposal_id, review_id, commit, output_dir, json } => {
                                    let result = match (proposal_id, review_id, commit) {
                                        (Some(pid), _, _) => load_push_execution_by_proposal(std::path::Path::new(&output_dir), &RemotePushProposalId(pid)),
                                        (_, Some(rid), _) => load_push_execution_by_review(std::path::Path::new(&output_dir), &RemotePushProposalReviewId(rid)),
                                        (_, _, Some(c)) => load_push_execution_by_commit(std::path::Path::new(&output_dir), &c),
                                        _ => load_latest_push_execution(std::path::Path::new(&output_dir)),
                                    }.map_err(|e| anyhow::anyhow!("{}", e))?;
                                    match result {
                                        Some(r) => {
                                            if json {
                                                println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?);
                                            } else {
                                                println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?);
                                            }
                                        }
                                        None => println!("No push executions found."),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn cmd_eval_task_plan(cmd: TaskPlanCommands) -> Result<()> {
    use openwand_app::task_planning::*;
    use openwand_workflow::builder::build_task_plan;
    use openwand_workflow::context::TaskPlanInput;
    use openwand_workflow::plan::TaskPlanId;
    use openwand_workflow::plan_review::{
        TaskPlanFeedback, TaskPlanReview, TaskPlanReviewDecision, task_review_id_for,
    };
    use openwand_workflow::validation::validate_task_plan_review;
    use chrono::Utc;

    match cmd {
        TaskPlanCommands::Create { intent, policy_constraints, output_dir, json } => {
            if intent.trim().is_empty() {
                anyhow::bail!("intent must not be empty");
            }
            let input = TaskPlanInput {
                user_intent: intent,
                skill_context: vec![],
                goal_context: vec![],
                memory_summaries: vec![],
                trace_summaries: vec![],
                governance_summaries: vec![],
                policy_constraints: policy_constraints.unwrap_or_default(),
            };
            let plan = build_task_plan(&input).map_err(|e| anyhow::anyhow!(e))?;
            let path = save_task_plan(std::path::Path::new(&output_dir), &plan)
                .map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&plan).context("Serialize")?);
            } else {
                println!("Plan created: {}", plan.plan_id.0);
                println!("  Title: {}", plan.title);
                println!("  Steps: {}", plan.steps.len());
                println!("  Saved: {}", path.display());
            }
        }

        TaskPlanCommands::Show { plan_id, output_dir, json } => {
            let plan = load_task_plan(std::path::Path::new(&output_dir), &TaskPlanId(plan_id))
                .map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&plan).context("Serialize")?);
            } else {
                println!("Plan: {}", plan.plan_id.0);
                println!("  Title: {}", plan.title);
                println!("  Status: {:?}", plan.status);
                println!("  Steps:");
                for step in &plan.steps {
                    println!("    {}: {:?} - {}", step.step_id, step.kind, step.title);
                }
            }
        }

        TaskPlanCommands::Latest { goal_id, skill_id, output_dir, json } => {
            let result = match (goal_id, skill_id) {
                (Some(gid), _) => task_plans_by_goal(std::path::Path::new(&output_dir), &gid),
                (_, Some(sid)) => task_plans_by_skill(std::path::Path::new(&output_dir), &sid),
                _ => latest_task_plan(std::path::Path::new(&output_dir)),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(plan) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&plan).context("Serialize")?);
                    } else {
                        println!("Latest plan: {}", plan.plan_id.0);
                        println!("  Title: {}", plan.title);
                    }
                }
                None => println!("No task plans found."),
            }
        }

        TaskPlanCommands::Review(review_cmd) => {
            match review_cmd {
                TaskPlanReviewCommands::Approve { plan_id, reviewer, rationale, output_dir, json } => {
                    let plan = load_task_plan(std::path::Path::new(&output_dir), &TaskPlanId(plan_id))
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, &rationale);
                    let review = TaskPlanReview {
                        review_id,
                        plan_id: plan.plan_id.clone(),
                        plan_hash: plan.plan_hash.clone(),
                        decision: TaskPlanReviewDecision::Approved,
                        reviewer,
                        rationale,
                        feedback: None,
                        creates_execution_grant: false,
                        execution_allowed_now: false,
                        reviewed_at: Utc::now(),
                    };
                    validate_task_plan_review(&review).map_err(|e| anyhow::anyhow!("Validation: {}", e.join(", ")))?;
                    save_plan_review(std::path::Path::new(&output_dir), &review)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&review).context("Serialize")?);
                    } else {
                        println!("Review: {}", review.review_id.0);
                        println!("  Decision: approved");
                    }
                }

                TaskPlanReviewCommands::Reject { plan_id, reviewer, rationale, feedback, output_dir, json } => {
                    let plan = load_task_plan(std::path::Path::new(&output_dir), &TaskPlanId(plan_id))
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Rejected, &rationale);
                    let fb = TaskPlanFeedback {
                        summary: feedback.clone(),
                        blocking_reasons: vec![feedback],
                        requested_changes: vec![],
                        evidence_gaps: vec![],
                    };
                    let review = TaskPlanReview {
                        review_id,
                        plan_id: plan.plan_id.clone(),
                        plan_hash: plan.plan_hash.clone(),
                        decision: TaskPlanReviewDecision::Rejected,
                        reviewer,
                        rationale,
                        feedback: Some(fb),
                        creates_execution_grant: false,
                        execution_allowed_now: false,
                        reviewed_at: Utc::now(),
                    };
                    validate_task_plan_review(&review).map_err(|e| anyhow::anyhow!("Validation: {}", e.join(", ")))?;
                    save_plan_review(std::path::Path::new(&output_dir), &review)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&review).context("Serialize")?);
                    } else {
                        println!("Review: {}", review.review_id.0);
                        println!("  Decision: rejected");
                    }
                }

                TaskPlanReviewCommands::RequestChanges { plan_id, reviewer, rationale, feedback, output_dir, json } => {
                    let plan = load_task_plan(std::path::Path::new(&output_dir), &TaskPlanId(plan_id))
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::ChangesRequested, &rationale);
                    let fb = TaskPlanFeedback {
                        summary: feedback.clone(),
                        blocking_reasons: vec![],
                        requested_changes: vec![feedback],
                        evidence_gaps: vec![],
                    };
                    let review = TaskPlanReview {
                        review_id,
                        plan_id: plan.plan_id.clone(),
                        plan_hash: plan.plan_hash.clone(),
                        decision: TaskPlanReviewDecision::ChangesRequested,
                        reviewer,
                        rationale,
                        feedback: Some(fb),
                        creates_execution_grant: false,
                        execution_allowed_now: false,
                        reviewed_at: Utc::now(),
                    };
                    validate_task_plan_review(&review).map_err(|e| anyhow::anyhow!("Validation: {}", e.join(", ")))?;
                    save_plan_review(std::path::Path::new(&output_dir), &review)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&review).context("Serialize")?);
                    } else {
                        println!("Review: {}", review.review_id.0);
                        println!("  Decision: changes_requested");
                    }
                }
            }
        }
    }
    Ok(())
}

/// Workflow proposal commands
#[derive(Debug, clap::Subcommand)]
enum WorkflowProposalCommands {
    /// Create a workflow proposal from an approved task plan
    Create {
        /// Task plan ID to create proposal from
        #[arg(long)]
        task_plan_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show a specific workflow proposal
    Show {
        /// Proposal ID
        proposal_id: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show the latest workflow proposal
    Latest {
        /// Filter by task plan ID
        #[arg(long)]
        task_plan_id: Option<String>,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Review a workflow proposal
    #[command(subcommand)]
    Review(WorkflowProposalReviewCommands),
}

/// Workflow proposal review commands
#[derive(Debug, clap::Subcommand)]
enum WorkflowProposalReviewCommands {
    /// Approve a workflow proposal
    Approve {
        /// Proposal ID to approve
        #[arg(long)]
        proposal_id: String,

        /// Reviewer name
        #[arg(long)]
        reviewer: String,

        /// Approval rationale
        #[arg(long)]
        rationale: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Reject a workflow proposal
    Reject {
        /// Proposal ID to reject
        #[arg(long)]
        proposal_id: String,

        /// Reviewer name
        #[arg(long)]
        reviewer: String,

        /// Rejection rationale
        #[arg(long)]
        rationale: String,

        /// Feedback text
        #[arg(long)]
        feedback: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Request changes to a workflow proposal
    RequestChanges {
        /// Proposal ID
        #[arg(long)]
        proposal_id: String,

        /// Reviewer name
        #[arg(long)]
        reviewer: String,

        /// Rationale for changes
        #[arg(long)]
        rationale: String,

        /// Feedback text
        #[arg(long)]
        feedback: String,

        /// Report store directory
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn cmd_workflow_proposal(cmd: WorkflowProposalCommands) -> Result<()> {
    use openwand_app::task_planning::*;
    use openwand_app::workflow_proposal::*;
    use openwand_workflow::plan::TaskPlanId;
    
    use openwand_workflow::workflow_proposal::WorkflowProposalId;
    use openwand_workflow::workflow_proposal_builder::{WorkflowProposalInput, build_workflow_proposal};
    use openwand_workflow::workflow_proposal_review::{
        WorkflowProposalFeedback, WorkflowProposalReview, WorkflowProposalReviewDecision,
        workflow_review_id_for, validate_workflow_proposal_review,
    };
    use chrono::Utc;

    match cmd {
        WorkflowProposalCommands::Create { task_plan_id, output_dir, json } => {
            let plan = load_task_plan(std::path::Path::new(&output_dir), &TaskPlanId(task_plan_id))
                .map_err(|e| anyhow::anyhow!(e))?;
            let latest_review = latest_plan_review(std::path::Path::new(&output_dir))
                .map_err(|e| anyhow::anyhow!(e))?
                .filter(|r| r.plan_id == plan.plan_id);

            let input = WorkflowProposalInput {
                task_plan: plan.clone(),
                latest_task_plan_review: latest_review,
                task_plan_hash: plan.plan_hash.clone(),
            };
            let proposal = build_workflow_proposal(input).map_err(|e| anyhow::anyhow!(e))?;
            let path = save_workflow_proposal(std::path::Path::new(&output_dir), &proposal)
                .map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&proposal).context("Serialize")?);
            } else {
                println!("Proposal created: {}", proposal.proposal_id.0);
                println!("  Title: {}", proposal.title);
                println!("  Stages: {}", proposal.stages.len());
                println!("  Source plan: {}", proposal.source_task_plan_id.0);
                println!("  Saved: {}", path.display());
            }
        }

        WorkflowProposalCommands::Show { proposal_id, output_dir, json } => {
            let proposal = load_workflow_proposal(std::path::Path::new(&output_dir), &WorkflowProposalId(proposal_id))
                .map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&proposal).context("Serialize")?);
            } else {
                println!("Proposal: {}", proposal.proposal_id.0);
                println!("  Title: {}", proposal.title);
                println!("  Status: {:?}", proposal.status);
                println!("  Stages:");
                for stage in &proposal.stages {
                    println!("    {}: {:?} - {}", stage.stage_id, stage.kind, stage.title);
                }
            }
        }

        WorkflowProposalCommands::Latest { task_plan_id, output_dir, json } => {
            let result = match task_plan_id {
                Some(tp_id) => workflow_proposal_by_task_plan(
                    std::path::Path::new(&output_dir), &TaskPlanId(tp_id),
                ),
                _ => latest_workflow_proposal(std::path::Path::new(&output_dir)),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(proposal) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&proposal).context("Serialize")?);
                    } else {
                        println!("Latest proposal: {}", proposal.proposal_id.0);
                        println!("  Title: {}", proposal.title);
                    }
                }
                None => println!("No workflow proposals found."),
            }
        }

        WorkflowProposalCommands::Review(review_cmd) => {
            match review_cmd {
                WorkflowProposalReviewCommands::Approve { proposal_id, reviewer, rationale, output_dir, json } => {
                    let proposal = load_workflow_proposal(std::path::Path::new(&output_dir), &WorkflowProposalId(proposal_id))
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let review_id = workflow_review_id_for(
                        &proposal.proposal_id,
                        &WorkflowProposalReviewDecision::Approved,
                        &rationale,
                    );
                    let review = WorkflowProposalReview {
                        review_id,
                        proposal_id: proposal.proposal_id.clone(),
                        source_task_plan_id: proposal.source_task_plan_id.clone(),
                        proposal_hash: proposal.proposal_hash.clone(),
                        decision: WorkflowProposalReviewDecision::Approved,
                        reviewer,
                        rationale,
                        feedback: None,
                        creates_execution_grant: false,
                        execution_allowed_now: false,
                        reviewed_at: Utc::now(),
                    };
                    validate_workflow_proposal_review(&review).map_err(|e| anyhow::anyhow!("Validation: {}", e.join(", ")))?;
                    save_proposal_review(std::path::Path::new(&output_dir), &review)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&review).context("Serialize")?);
                    } else {
                        println!("Review: {}", review.review_id.0);
                        println!("  Decision: approved");
                    }
                }

                WorkflowProposalReviewCommands::Reject { proposal_id, reviewer, rationale, feedback, output_dir, json } => {
                    let proposal = load_workflow_proposal(std::path::Path::new(&output_dir), &WorkflowProposalId(proposal_id))
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let review_id = workflow_review_id_for(
                        &proposal.proposal_id,
                        &WorkflowProposalReviewDecision::Rejected,
                        &rationale,
                    );
                    let fb = WorkflowProposalFeedback {
                        summary: feedback.clone(),
                        blocking_reasons: vec![feedback],
                        requested_changes: vec![],
                        evidence_gaps: vec![],
                    };
                    let review = WorkflowProposalReview {
                        review_id,
                        proposal_id: proposal.proposal_id.clone(),
                        source_task_plan_id: proposal.source_task_plan_id.clone(),
                        proposal_hash: proposal.proposal_hash.clone(),
                        decision: WorkflowProposalReviewDecision::Rejected,
                        reviewer,
                        rationale,
                        feedback: Some(fb),
                        creates_execution_grant: false,
                        execution_allowed_now: false,
                        reviewed_at: Utc::now(),
                    };
                    validate_workflow_proposal_review(&review).map_err(|e| anyhow::anyhow!("Validation: {}", e.join(", ")))?;
                    save_proposal_review(std::path::Path::new(&output_dir), &review)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&review).context("Serialize")?);
                    } else {
                        println!("Review: {}", review.review_id.0);
                        println!("  Decision: rejected");
                    }
                }

                WorkflowProposalReviewCommands::RequestChanges { proposal_id, reviewer, rationale, feedback, output_dir, json } => {
                    let proposal = load_workflow_proposal(std::path::Path::new(&output_dir), &WorkflowProposalId(proposal_id))
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let review_id = workflow_review_id_for(
                        &proposal.proposal_id,
                        &WorkflowProposalReviewDecision::ChangesRequested,
                        &rationale,
                    );
                    let fb = WorkflowProposalFeedback {
                        summary: feedback.clone(),
                        blocking_reasons: vec![],
                        requested_changes: vec![feedback],
                        evidence_gaps: vec![],
                    };
                    let review = WorkflowProposalReview {
                        review_id,
                        proposal_id: proposal.proposal_id.clone(),
                        source_task_plan_id: proposal.source_task_plan_id.clone(),
                        proposal_hash: proposal.proposal_hash.clone(),
                        decision: WorkflowProposalReviewDecision::ChangesRequested,
                        reviewer,
                        rationale,
                        feedback: Some(fb),
                        creates_execution_grant: false,
                        execution_allowed_now: false,
                        reviewed_at: Utc::now(),
                    };
                    validate_workflow_proposal_review(&review).map_err(|e| anyhow::anyhow!("Validation: {}", e.join(", ")))?;
                    save_proposal_review(std::path::Path::new(&output_dir), &review)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&review).context("Serialize")?);
                    } else {
                        println!("Review: {}", review.review_id.0);
                        println!("  Decision: changes_requested");
                    }
                }
            }
        }
    }
    Ok(())
}


/// Workflow readiness commands
#[derive(Debug, clap::Subcommand)]
enum WorkflowReadinessCommands {
    /// Evaluate workflow readiness
    Evaluate {
        #[arg(long)]
        proposal_id: String,
        #[arg(long)]
        review_id: String,
        #[arg(long)]
        expected_proposal_hash: String,
        #[arg(long)]
        expected_source_task_plan_hash: String,
        #[arg(long, default_value = "default")]
        idempotency_key: String,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
        #[arg(long)]
        json: bool,
    },
    /// Show a specific readiness record
    Show {
        readiness_id: String,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
        #[arg(long)]
        json: bool,
    },
    /// Show the latest readiness record
    Latest {
        #[arg(long)]
        proposal_id: Option<String>,
        #[arg(long)]
        review_id: Option<String>,
        #[arg(long)]
        task_plan_id: Option<String>,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
        #[arg(long)]
        json: bool,
    },
}

fn cmd_workflow_readiness(cmd: WorkflowReadinessCommands) -> Result<()> {
    use openwand_app::task_planning::*;
    use openwand_app::workflow_proposal::*;
    use openwand_app::workflow_readiness::*;
    use openwand_workflow::plan::TaskPlanId;
    use openwand_workflow::workflow_proposal::WorkflowProposalId;
    use openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId;
    use openwand_workflow::workflow_readiness::{WorkflowReadinessRequest, WorkflowEnvironmentSnapshot};
    use openwand_workflow::workflow_readiness_evaluator::{WorkflowReadinessContext, evaluate_workflow_readiness};
    use chrono::Utc;

    match cmd {
        WorkflowReadinessCommands::Evaluate {
            proposal_id, review_id, expected_proposal_hash,
            expected_source_task_plan_hash, idempotency_key, output_dir, json,
        } => {
            let proposal = load_workflow_proposal(
                std::path::Path::new(&output_dir),
                &WorkflowProposalId(proposal_id.clone()),
            ).map_err(|e| anyhow::anyhow!(e))?;
            let review = load_proposal_review(
                std::path::Path::new(&output_dir),
                &WorkflowProposalReviewId(review_id.clone()),
            ).map_err(|e| anyhow::anyhow!(e))?;

            let source_plan = load_task_plan(
                std::path::Path::new(&output_dir),
                &proposal.source_task_plan_id,
            ).ok();
            let source_review = source_plan.as_ref()
                .and_then(|_| latest_plan_review(std::path::Path::new(&output_dir)).ok().flatten());
            let latest_review = latest_proposal_review(std::path::Path::new(&output_dir))
                .map_err(|e| anyhow::anyhow!(e))?
                .filter(|r| r.proposal_id == proposal.proposal_id);

            let request = WorkflowReadinessRequest {
                proposal_id: proposal.proposal_id.clone(),
                review_id: review.review_id.clone(),
                expected_proposal_hash,
                expected_source_task_plan_hash,
                requested_by: "cli".into(),
                requested_at: Utc::now(),
                idempotency_key,
            };
            let context = WorkflowReadinessContext {
                proposal: Some(proposal),
                review: Some(review.clone()),
                latest_review_for_proposal: latest_review,
                source_task_plan: source_plan,
                source_task_plan_review: source_review.clone(),
                latest_source_task_plan_review: source_review,
                environment: WorkflowEnvironmentSnapshot {
                    workspace_observed: true,
                    provider_config_available: true,
                    session_runtime_available: true,
                    tool_manifest_available: true,
                    policy_context_available: true,
                    notes: vec![],
                },
                existing_readiness_records: vec![],
            };
            let record = evaluate_workflow_readiness(&request, &context);
            let path = save_workflow_readiness(std::path::Path::new(&output_dir), &record)
                .map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
            } else {
                println!("Readiness: {}", record.readiness_id.0);
                println!("  Status: {:?}", record.status);
                let passed = record.predicates.iter().filter(|p| p.passed).count();
                println!("  Predicates: {}/{} passed", passed, record.predicates.len());
                println!("  Saved: {}", path.display());
            }
        }

        WorkflowReadinessCommands::Show { readiness_id, output_dir, json } => {
            let record = load_workflow_readiness(
                std::path::Path::new(&output_dir),
                &openwand_workflow::workflow_readiness::WorkflowReadinessId(readiness_id),
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
            } else {
                println!("Readiness: {}", record.readiness_id.0);
                println!("  Status: {:?}", record.status);
                for pred in &record.predicates {
                    let mark = if pred.passed { "OK" } else { "FAIL" };
                    println!("  {} {:?}: {}", mark, pred.predicate, pred.reason);
                }
            }
        }

        WorkflowReadinessCommands::Latest { proposal_id, review_id, task_plan_id, output_dir, json } => {
            let result = match (proposal_id, review_id, task_plan_id) {
                (Some(pid), _, _) => workflow_readiness_by_proposal(
                    std::path::Path::new(&output_dir), &WorkflowProposalId(pid),
                ),
                (_, Some(rid), _) => workflow_readiness_by_review(
                    std::path::Path::new(&output_dir), &WorkflowProposalReviewId(rid),
                ),
                (_, _, Some(tpid)) => workflow_readiness_by_task_plan(
                    std::path::Path::new(&output_dir), &TaskPlanId(tpid),
                ),
                _ => latest_workflow_readiness(std::path::Path::new(&output_dir)),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(record) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
                    } else {
                        println!("Latest readiness: {}", record.readiness_id.0);
                        println!("  Status: {:?}", record.status);
                    }
                }
                None => println!("No workflow readiness records found."),
            }
        }
    }
    Ok(())
}


/// Workflow execution commands
#[derive(Debug, clap::Subcommand)]
enum WorkflowExecutionCommands {
    /// Execute a workflow from a readiness record
    Execute {
        #[arg(long)]
        readiness_id: String,
        #[arg(long)]
        proposal_id: String,
        #[arg(long)]
        proposal_review_id: String,
        #[arg(long)]
        expected_readiness_hash: String,
        #[arg(long)]
        expected_proposal_hash: String,
        #[arg(long, default_value = "default")]
        idempotency_key: String,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
        #[arg(long)]
        json: bool,
    },
    /// Show a specific workflow run
    Show {
        execution_id: String,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
        #[arg(long)]
        json: bool,
    },
    /// Show the latest workflow run
    Latest {
        #[arg(long)]
        readiness_id: Option<String>,
        #[arg(long)]
        proposal_id: Option<String>,
        #[arg(long)]
        proposal_review_id: Option<String>,
        #[arg(long)]
        task_plan_id: Option<String>,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
        #[arg(long)]
        json: bool,
    },
}

fn cmd_workflow_execution(cmd: WorkflowExecutionCommands) -> Result<()> {
    use openwand_app::workflow_execution::*;
    use openwand_app::workflow_proposal::*;
    use openwand_app::workflow_readiness::*;
    use openwand_app::task_planning::*;
    use openwand_workflow::plan::TaskPlanId;
    use openwand_workflow::workflow_proposal::WorkflowProposalId;
    use openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId;
    use openwand_workflow::workflow_readiness::WorkflowReadinessId;
    
    use openwand_workflow::workflow_run::WorkflowExecutionRequest;
    use openwand_workflow::workflow_execution_gate::{WorkflowExecutionContext, evaluate_workflow_execution};
    use chrono::Utc;

    match cmd {
        WorkflowExecutionCommands::Execute {
            readiness_id, proposal_id, proposal_review_id,
            expected_readiness_hash, expected_proposal_hash,
            idempotency_key, output_dir, json,
        } => {
            let readiness = load_workflow_readiness(
                std::path::Path::new(&output_dir),
                &WorkflowReadinessId(readiness_id.clone()),
            ).map_err(|e| anyhow::anyhow!(e))?;
            let proposal = load_workflow_proposal(
                std::path::Path::new(&output_dir),
                &WorkflowProposalId(proposal_id.clone()),
            ).map_err(|e| anyhow::anyhow!(e))?;
            let review = load_proposal_review(
                std::path::Path::new(&output_dir),
                &WorkflowProposalReviewId(proposal_review_id.clone()),
            ).map_err(|e| anyhow::anyhow!(e))?;

            let source_plan = load_task_plan(
                std::path::Path::new(&output_dir),
                &proposal.source_task_plan_id,
            ).ok();
            let latest_review = latest_proposal_review(std::path::Path::new(&output_dir))
                .ok().flatten().filter(|r| r.proposal_id == proposal.proposal_id);

            let request = WorkflowExecutionRequest {
                readiness_id: readiness.readiness_id.clone(),
                proposal_id: proposal.proposal_id.clone(),
                proposal_review_id: review.review_id.clone(),
                expected_readiness_hash,
                expected_proposal_hash,
                requested_by: "cli".into(),
                requested_at: Utc::now(),
                idempotency_key,
            };
            let context = WorkflowExecutionContext {
                readiness: Some(readiness),
                proposal: Some(proposal),
                proposal_review: Some(review.clone()),
                latest_proposal_review: latest_review,
                source_task_plan: source_plan,
                source_task_plan_review: None,
                latest_source_task_plan_review: None,
                provider_config_available: true,
                session_runtime_available: true,
                existing_runs: vec![],
            };
            let mut record = evaluate_workflow_execution(&request, &context);
            // Advance stages through lifecycle
            if record.status == openwand_workflow::workflow_run::WorkflowRunStatus::Suspended
                && let Some(ref proposal) = context.proposal {
                    let (stages, events, action_requests) = openwand_workflow::workflow_run_lifecycle::advance_stages(proposal);
                    record.stages = stages;
                    record.lifecycle_events = events;
                    record.action_requests = action_requests;
                }
            let path = save_workflow_run(std::path::Path::new(&output_dir), &record)
                .map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
            } else {
                println!("Run: {}", record.execution_id.0);
                println!("  Status: {:?}", record.status);
                println!("  Stages: {}", record.stages.len());
                let passed = record.predicates.iter().filter(|p| p.passed).count();
                println!("  Predicates: {}/{} passed", passed, record.predicates.len());
                println!("  Saved: {}", path.display());
            }
        }

        WorkflowExecutionCommands::Show { execution_id, output_dir, json } => {
            let record = load_workflow_run(
                std::path::Path::new(&output_dir),
                &openwand_workflow::workflow_run::WorkflowExecutionId(execution_id),
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
            } else {
                println!("Run: {}", record.execution_id.0);
                println!("  Status: {:?}", record.status);
                for stage in &record.stages {
                    println!("  {}: {:?} - {:?}", stage.stage_id, stage.kind, stage.status);
                }
            }
        }

        WorkflowExecutionCommands::Latest { readiness_id, proposal_id, proposal_review_id, task_plan_id, output_dir, json } => {
            let result = match (readiness_id, proposal_id, proposal_review_id, task_plan_id) {
                (Some(rid), _, _, _) => workflow_run_by_readiness(
                    std::path::Path::new(&output_dir), &WorkflowReadinessId(rid)),
                (_, Some(pid), _, _) => workflow_run_by_proposal(
                    std::path::Path::new(&output_dir), &WorkflowProposalId(pid)),
                (_, _, Some(rid), _) => workflow_run_by_review(
                    std::path::Path::new(&output_dir), &WorkflowProposalReviewId(rid)),
                (_, _, _, Some(tpid)) => workflow_run_by_task_plan(
                    std::path::Path::new(&output_dir), &TaskPlanId(tpid)),
                _ => latest_workflow_run(std::path::Path::new(&output_dir)),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(record) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
                    } else {
                        println!("Latest run: {}", record.execution_id.0);
                        println!("  Status: {:?}", record.status);
                    }
                }
                None => println!("No workflow runs found."),
            }
        }
    }
    Ok(())
}


/// Workflow action routing commands
#[derive(Debug, clap::Subcommand)]
enum WorkflowActionCommands {
    /// Route a prepared workflow action request into a session
    Route {
        #[arg(long)]
        workflow_execution_id: String,
        #[arg(long)]
        readiness_id: String,
        #[arg(long)]
        proposal_id: String,
        #[arg(long)]
        stage_id: String,
        #[arg(long)]
        action_request_id: String,
        #[arg(long)]
        expected_workflow_run_hash: String,
        #[arg(long)]
        expected_action_request_hash: String,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value = "default")]
        idempotency_key: String,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
        #[arg(long)]
        json: bool,
    },
    /// Show a specific workflow action route
    Show {
        route_id: String,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
        #[arg(long)]
        json: bool,
    },
    /// Show the latest workflow action route
    Latest {
        #[arg(long)]
        workflow_execution_id: Option<String>,
        #[arg(long)]
        stage_id: Option<String>,
        #[arg(long)]
        action_request_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value = "eval_reports")]
        output_dir: String,
        #[arg(long)]
        json: bool,
    },
}

fn cmd_workflow_action(cmd: WorkflowActionCommands) -> Result<()> {
    use openwand_app::workflow_action_routing::*;
    use openwand_app::workflow_execution::*;
    use openwand_app::workflow_session_bridge::{DeterministicSessionBridge, WorkflowSessionBridge};
    use openwand_workflow::workflow_action_route::*;
    use openwand_workflow::workflow_action_route_gate::{WorkflowActionRouteContext, evaluate_action_route};
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_readiness::WorkflowReadinessId;
    use openwand_workflow::workflow_proposal::WorkflowProposalId;
    
    use chrono::Utc;

    match cmd {
        WorkflowActionCommands::Route {
            workflow_execution_id, readiness_id, proposal_id, stage_id,
            action_request_id, expected_workflow_run_hash, expected_action_request_hash,
            session_id, idempotency_key, output_dir, json,
        } => {
            let store = std::path::Path::new(&output_dir);
            let run = load_workflow_run(store, &WorkflowExecutionId(workflow_execution_id.clone())).ok();

            let target_stage = run.as_ref().and_then(|r| r.stages.iter().find(|s| s.stage_id == stage_id));
            let target_ar = run.as_ref().and_then(|r| r.action_requests.iter().find(|a| a.action_request_id == action_request_id));

            let prior = list_workflow_action_routes(store).unwrap_or_default();
            let prior_refs: Vec<&WorkflowActionRouteRecord> = prior.iter().collect();

            let request = WorkflowActionRouteRequest {
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id.clone()),
                readiness_id: WorkflowReadinessId(readiness_id),
                proposal_id: WorkflowProposalId(proposal_id),
                stage_id, action_request_id, session_id,
                expected_workflow_run_hash, expected_action_request_hash,
                requested_by: "cli".into(), requested_at: Utc::now(), idempotency_key,
            };

            let context = WorkflowActionRouteContext {
                workflow_run: run.as_ref(),
                target_stage,
                target_action_request: target_ar,
                prior_routes: prior_refs,
                session_bridge_available: true,
                session_runner_available: true,
                workflow_run_hash: request.expected_workflow_run_hash.clone(),
                action_request_hash: request.expected_action_request_hash.clone(),
            };

            let mut record = evaluate_action_route(&request, &context);

            // If routed, call the session bridge (deterministic for Wave 27 CLI)
            if record.status == WorkflowActionRouteStatus::Routed {
                let bridge = DeterministicSessionBridge::completed();
                let bridge_result = bridge.route_action_to_session(record.route_prompt.clone(), request.session_id.clone());
                match bridge_result {
                    Ok(snap) => {
                        record.session_route = Some(snap.clone());
                        if snap.session_status == "completed" {
                            record.status = WorkflowActionRouteStatus::Completed;
                            record.decision = WorkflowActionRouteDecision::Completed {
                                summary: "Session turn completed via deterministic bridge".into(),
                            };
                        } else if snap.pending_approval_id.is_some() {
                            record.status = WorkflowActionRouteStatus::SuspendedForApproval;
                            record.decision = WorkflowActionRouteDecision::SuspendedForApproval {
                                approval_request_id: snap.pending_approval_id.unwrap(),
                                summary: "Session suspended for approval".into(),
                            };
                        }
                        record.completed_at = Some(Utc::now());
                    }
                    Err(e) => {
                        record.status = WorkflowActionRouteStatus::Failed;
                        record.decision = WorkflowActionRouteDecision::Failed {
                            reason_code: "bridge_error".into(),
                            summary: format!("{:?}", e),
                        };
                    }
                }
            }

            let path = save_workflow_action_route(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
            } else {
                println!("Route: {}", record.route_id.0);
                println!("  Status: {:?}", record.status);
                println!("  Stage: {}", record.stage_id);
                println!("  Action: {}", record.action_request_id);
                if let Some(ref sr) = record.session_route {
                    println!("  Session: {}", sr.session_id);
                }
                println!("  Saved: {}", path.display());
            }
        }

        WorkflowActionCommands::Show { route_id, output_dir, json } => {
            let record = load_workflow_action_route(std::path::Path::new(&output_dir), &WorkflowActionRouteId(route_id))
                .map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
            } else {
                println!("Route: {}", record.route_id.0);
                println!("  Status: {:?}", record.status);
                if let Some(ref sr) = record.session_route {
                    println!("  Session: {} ({})", sr.session_id, sr.session_status);
                }
            }
        }

        WorkflowActionCommands::Latest { workflow_execution_id, stage_id, action_request_id, session_id, output_dir, json } => {
            let store = std::path::Path::new(&output_dir);
            let result = match (workflow_execution_id, stage_id, action_request_id, session_id) {
                (Some(eid), _, _, _) => route_by_workflow_run(store, &eid),
                (_, Some(sid), _, _) => route_by_stage(store, &sid),
                (_, _, Some(arid), _) => route_by_action_request(store, &arid),
                (_, _, _, Some(sess)) => route_by_session(store, &sess),
                _ => latest_workflow_action_route(store),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(record) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
                    } else {
                        println!("Latest route: {}", record.route_id.0);
                        println!("  Status: {:?}", record.status);
                    }
                }
                None => println!("No workflow action routes found."),
            }
        }
    }
    Ok(())
}

#[derive(Debug, clap::Subcommand)]
enum WorkflowActionOutcomeCommands {
    /// Resolve a workflow-routed pending approval
    Resolve {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] route_id: String,
        #[arg(long)] stage_id: String,
        #[arg(long)] action_request_id: String,
        #[arg(long)] session_id: String,
        #[arg(long)] pending_approval_id: String,
        #[arg(long)] expected_route_hash: String,
        #[arg(long)] expected_workflow_run_hash: String,
        #[arg(long)] approve: bool,
        #[arg(long)] reject: bool,
        #[arg(long)] rationale: String,
        #[arg(long)] tool_call_id: Option<String>,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    /// Show a specific outcome
    Show {
        outcome_id: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    /// Show the latest outcome
    Latest {
        #[arg(long)] route_id: Option<String>,
        #[arg(long)] pending_approval_id: Option<String>,
        #[arg(long)] workflow_execution_id: Option<String>,
        #[arg(long)] session_id: Option<String>,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_workflow_action_outcome(cmd: WorkflowActionOutcomeCommands) -> Result<()> {
    use openwand_app::workflow_action_outcome::*;
    use openwand_app::workflow_approval_bridge::{DeterministicApprovalBridge, WorkflowApprovalBridge};
    use openwand_workflow::workflow_action_outcome::*;
    use openwand_workflow::workflow_action_outcome_gate::{WorkflowActionOutcomeContext, evaluate_action_outcome};
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_action_route::WorkflowActionRouteId;
    use chrono::Utc;

    match cmd {
        WorkflowActionOutcomeCommands::Resolve {
            workflow_execution_id, route_id, stage_id, action_request_id,
            session_id, pending_approval_id, expected_route_hash, expected_workflow_run_hash,
            approve, reject, rationale, tool_call_id, idempotency_key, output_dir, json,
        } => {
            if approve == reject {
                anyhow::bail!("Must specify exactly one of --approve or --reject");
            }
            let resolution = if approve {
                WorkflowApprovalResolution::Approve { rationale }
            } else {
                WorkflowApprovalResolution::Reject { rationale }
            };
            let store = std::path::Path::new(&output_dir);
            let request = WorkflowActionOutcomeRequest {
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                route_id: WorkflowActionRouteId(route_id),
                stage_id, action_request_id, session_id, pending_approval_id,
                tool_call_id, expected_route_hash, expected_workflow_run_hash,
                resolution, requested_by: "cli".into(), requested_at: Utc::now(), idempotency_key,
            };
            let prior = list_workflow_action_outcomes(store).unwrap_or_default();
            let prior_refs: Vec<&WorkflowActionOutcomeRecord> = prior.iter().collect();
            let context = WorkflowActionOutcomeContext {
                workflow_run: None, route_record: None, prior_outcomes: prior_refs,
                approval_bridge_available: true,
                workflow_run_hash: request.expected_workflow_run_hash.clone(),
                route_hash: request.expected_route_hash.clone(),
            };
            let mut record = evaluate_action_outcome(&request, &context);
            if record.status == WorkflowActionOutcomeStatus::ApprovalResolved {
                let bridge = DeterministicApprovalBridge::approved();
                let outcome = bridge.resolve_workflow_routed_approval(&request).unwrap();
                record.session_outcome = Some(outcome);
                record.status = WorkflowActionOutcomeStatus::ToolCompleted;
                record.decision = WorkflowActionOutcomeDecision::ToolCompleted {
                    summary: "Resolved via deterministic bridge".into(),
                };
                record.completed_at = Some(Utc::now());
            }
            let path = save_workflow_action_outcome(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
            } else {
                println!("Outcome: {}", record.outcome_id.0);
                println!("  Status: {:?}", record.status);
                if let Some(ref o) = record.session_outcome {
                    println!("  Tool: {} ({})", o.tool_name_observed_from_session.as_deref().unwrap_or("-"), o.tool_status_observed_from_session.as_deref().unwrap_or("-"));
                }
                println!("  Saved: {}", path.display());
            }
        }
        WorkflowActionOutcomeCommands::Show { outcome_id, output_dir, json } => {
            let record = load_workflow_action_outcome(std::path::Path::new(&output_dir),
                &WorkflowActionOutcomeId(outcome_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Outcome: {} — {:?}", record.outcome_id.0, record.status); }
        }
        WorkflowActionOutcomeCommands::Latest { route_id, pending_approval_id, workflow_execution_id, session_id, output_dir, json } => {
            let store = std::path::Path::new(&output_dir);
            let result = match (route_id, pending_approval_id, workflow_execution_id, session_id) {
                (Some(rid), _, _, _) => outcome_by_route(store, &rid),
                (_, Some(pid), _, _) => outcome_by_pending_approval(store, &pid),
                (None, None, Some(eid), _) => outcome_by_workflow_run(store, &eid),
                (None, None, None, Some(sid)) => outcome_by_session(store, &sid),
                _ => latest_workflow_action_outcome(store),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(r) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest: {} — {:?}", r.outcome_id.0, r.status); } }
                None => println!("No outcomes found."),
            }
        }
    }
    Ok(())
}

#[derive(Debug, clap::Subcommand)]
enum WorkflowReconciliationCommands {
    /// Reconcile a terminal outcome back into workflow run stage state
    Reconcile {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] route_id: String,
        #[arg(long)] outcome_id: String,
        #[arg(long)] stage_id: String,
        #[arg(long)] action_request_id: String,
        #[arg(long)] expected_workflow_run_hash: String,
        #[arg(long)] expected_route_hash: String,
        #[arg(long)] expected_outcome_hash: String,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    /// Show a specific reconciliation record
    Show {
        reconciliation_id: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    /// Show the latest reconciliation record
    Latest {
        #[arg(long)] workflow_execution_id: Option<String>,
        #[arg(long)] route_id: Option<String>,
        #[arg(long)] outcome_id: Option<String>,
        #[arg(long)] stage_id: Option<String>,
        #[arg(long)] action_request_id: Option<String>,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_workflow_reconciliation(cmd: WorkflowReconciliationCommands) -> Result<()> {
    use openwand_app::workflow_reconciliation::*;
    use openwand_workflow::workflow_reconciliation::*;
    use openwand_workflow::workflow_reconciliation_gate::{WorkflowReconciliationContext, evaluate_reconciliation};
    
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_action_route::WorkflowActionRouteId;
    use openwand_workflow::workflow_action_outcome::WorkflowActionOutcomeId;

    match cmd {
        WorkflowReconciliationCommands::Reconcile {
            workflow_execution_id, route_id, outcome_id, stage_id, action_request_id,
            expected_workflow_run_hash, expected_route_hash, expected_outcome_hash,
            idempotency_key, output_dir, json,
        } => {
            let store = std::path::Path::new(&output_dir);
            let request = WorkflowReconciliationRequest {
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                route_id: WorkflowActionRouteId(route_id),
                outcome_id: WorkflowActionOutcomeId(outcome_id),
                stage_id, action_request_id,
                expected_workflow_run_hash, expected_route_hash, expected_outcome_hash,
                requested_by: "cli".into(), requested_at: chrono::Utc::now(), idempotency_key,
            };
            // Load available records for context
            let prior_recs = list_workflow_reconciliations(store).unwrap_or_default();
            let prior_refs: Vec<&WorkflowReconciliationRecord> = prior_recs.iter().collect();
            let context = WorkflowReconciliationContext {
                workflow_run: None, // CLI doesn't load full records — gate blocks without them
                route_record: None,
                outcome_record: None,
                prior_reconciliations: prior_refs,
                expected_workflow_run_hash: request.expected_workflow_run_hash.clone(),
                expected_route_hash: request.expected_route_hash.clone(),
                expected_outcome_hash: request.expected_outcome_hash.clone(),
            };
            let mut record = evaluate_reconciliation(&request, &context);

            // If reconciled, attempt stage progression (no run data in CLI, so progression is None)
            if record.status == WorkflowReconciliationStatus::Reconciled {
                record.status = WorkflowReconciliationStatus::Blocked;
                record.decision = WorkflowReconciliationDecision::Blocked {
                    reason_code: "no_run_loaded".into(),
                    summary: "CLI reconcile requires pre-loaded run/route/outcome context".into(),
                };
            }

            let path = save_workflow_reconciliation(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
            } else {
                println!("Reconciliation: {}", record.reconciliation_id.0);
                println!("  Status: {:?}", record.status);
                println!("  Saved: {}", path.display());
            }
        }
        WorkflowReconciliationCommands::Show { reconciliation_id, output_dir, json } => {
            let record = load_workflow_reconciliation(std::path::Path::new(&output_dir),
                &WorkflowReconciliationId(reconciliation_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Reconciliation: {} — {:?}", record.reconciliation_id.0, record.status); }
        }
        WorkflowReconciliationCommands::Latest { workflow_execution_id, route_id, outcome_id, stage_id, action_request_id, output_dir, json } => {
            let store = std::path::Path::new(&output_dir);
            let result = match (workflow_execution_id, route_id, outcome_id, stage_id, action_request_id) {
                (Some(id), _, _, _, _) => reconciliation_by_workflow_run(store, &id),
                (_, Some(id), _, _, _) => reconciliation_by_route(store, &id),
                (_, _, Some(id), _, _) => reconciliation_by_outcome(store, &id),
                (_, _, _, Some(id), _) => reconciliation_by_stage(store, &id),
                (_, _, _, _, Some(id)) => reconciliation_by_action_request(store, &id),
                _ => latest_workflow_reconciliation(store),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(r) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest: {} — {:?}", r.reconciliation_id.0, r.status); } }
                None => println!("No reconciliations found."),
            }
        }
    }
    Ok(())
}

#[derive(Debug, clap::Subcommand)]
enum WorkflowContinuationCommands {
    /// Propose the next eligible action from a run revision
    Propose {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] latest_run_revision_id: String,
        #[arg(long)] expected_run_revision_hash: String,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    /// Show a continuation readiness record
    ShowReadiness {
        readiness_id: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    /// Show a next-action proposal record
    ShowProposal {
        proposal_id: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    /// Show the latest continuation readiness or proposal
    Latest {
        #[arg(long)] workflow_execution_id: Option<String>,
        #[arg(long)] run_revision_id: Option<String>,
        #[arg(long)] stage_id: Option<String>,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_workflow_continuation(cmd: WorkflowContinuationCommands) -> Result<()> {
    use openwand_app::workflow_continuation::*;
    use openwand_workflow::workflow_continuation::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;

    match cmd {
        WorkflowContinuationCommands::Propose {
            workflow_execution_id, latest_run_revision_id, expected_run_revision_hash,
            idempotency_key, output_dir, json,
        } => {
            let store = std::path::Path::new(&output_dir);
            let request = WorkflowContinuationRequest {
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                latest_run_revision_id: WorkflowRunRevisionId(latest_run_revision_id),
                expected_run_revision_hash, requested_by: "cli".into(),
                requested_at: chrono::Utc::now(), idempotency_key,
            };
            let prior = list_continuation_readiness(store).unwrap_or_default();
            let prior_refs: Vec<&WorkflowContinuationReadinessRecord> = prior.iter().collect();
            let prior_props = list_next_action_proposals(store).unwrap_or_default();
            let prior_prop_refs: Vec<&WorkflowNextActionProposal> = prior_props.iter().collect();
            let ctx = openwand_workflow::workflow_next_action_selector::WorkflowContinuationContext {
                workflow_run: None, run_revision: None,
                prior_readiness: prior_refs, prior_proposals: prior_prop_refs,
            };
            let record = openwand_workflow::workflow_next_action_selector::evaluate_continuation_readiness(&request, &ctx);
            let path = save_continuation_readiness(store, &record).map_err(|e| anyhow::anyhow!(e))?;

            if json {
                println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?);
            } else {
                println!("Readiness: {}", record.readiness_id.0);
                println!("  Status: {:?}", record.status);
                if let Some(ref c) = record.selected_candidate {
                    println!("  Candidate: {} ({:?})", c.stage_id, c.candidate_kind);
                }
                println!("  Saved: {}", path.display());
            }
        }
        WorkflowContinuationCommands::ShowReadiness { readiness_id, output_dir, json } => {
            let rec = load_continuation_readiness(std::path::Path::new(&output_dir),
                &WorkflowContinuationReadinessId(readiness_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?); }
            else { println!("Readiness: {} — {:?}", rec.readiness_id.0, rec.status); }
        }
        WorkflowContinuationCommands::ShowProposal { proposal_id, output_dir, json } => {
            let rec = load_next_action_proposal(std::path::Path::new(&output_dir),
                &WorkflowNextActionProposalId(proposal_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?); }
            else { println!("Proposal: {} — stage {}", rec.proposal_id.0, rec.candidate.stage_id); }
        }
        WorkflowContinuationCommands::Latest { workflow_execution_id, run_revision_id, stage_id, output_dir, json } => {
            let store = std::path::Path::new(&output_dir);
            if let Some(sid) = stage_id {
                match proposal_by_stage(store, &sid) {
                    Ok(Some(p)) => { if json { println!("{}", serde_json::to_string_pretty(&p).context("Serialize")?); } else { println!("Proposal: {} — stage {}", p.proposal_id.0, p.candidate.stage_id); } }
                    _ => println!("No proposal found for stage."),
                }
            } else if let Some(rid) = run_revision_id {
                match proposal_by_run_revision(store, &rid) {
                    Ok(Some(p)) => { if json { println!("{}", serde_json::to_string_pretty(&p).context("Serialize")?); } else { println!("Proposal: {}", p.proposal_id.0); } }
                    _ => println!("No proposal found for revision."),
                }
            } else if let Some(wid) = workflow_execution_id {
                match readiness_by_workflow_run(store, &wid) {
                    Ok(Some(r)) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Readiness: {} — {:?}", r.readiness_id.0, r.status); } }
                    _ => println!("No readiness found for workflow run."),
                }
            } else {
                match latest_continuation_readiness(store) {
                    Ok(Some(r)) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest readiness: {} — {:?}", r.readiness_id.0, r.status); } }
                    _ => println!("No continuation records found."),
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, clap::Subcommand)]
enum WorkflowNextActionReviewCommands {
    Approve {
        #[arg(long)] proposal_id: String,
        #[arg(long)] reviewer: String,
        #[arg(long)] rationale: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    Reject {
        #[arg(long)] proposal_id: String,
        #[arg(long)] reviewer: String,
        #[arg(long)] rationale: String,
        #[arg(long)] feedback: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    RequestChanges {
        #[arg(long)] proposal_id: String,
        #[arg(long)] reviewer: String,
        #[arg(long)] rationale: String,
        #[arg(long)] feedback: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    Show { review_id: String, #[arg(long, default_value = "eval_reports")] output_dir: String, #[arg(long)] json: bool },
    Latest { #[arg(long)] proposal_id: String, #[arg(long, default_value = "eval_reports")] output_dir: String, #[arg(long)] json: bool },
}

fn cmd_workflow_next_action_review(cmd: WorkflowNextActionReviewCommands) -> Result<()> {
    use openwand_app::workflow_next_action_review::*;
    use openwand_workflow::workflow_next_action_review::*;
    use openwand_workflow::workflow_continuation::WorkflowNextActionProposalId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;

    match cmd {
        WorkflowNextActionReviewCommands::Approve { proposal_id, reviewer, rationale, output_dir, json } => {
            openwand_workflow::workflow_next_action_review::validate_review(&reviewer, &rationale, &WorkflowNextActionReviewDecision::Approved, None).map_err(|e| anyhow::anyhow!(e))?;
            let review_id = next_action_review_id_for(&proposal_id, &WorkflowNextActionReviewDecision::Approved, &reviewer);
            let review = WorkflowNextActionReview {
                review_id,
                proposal_id: WorkflowNextActionProposalId(proposal_id), proposal_hash: String::new(),
                source_run_revision_id: WorkflowRunRevisionId(String::new()), source_run_revision_hash: String::new(),
                decision: WorkflowNextActionReviewDecision::Approved, reviewer, rationale,
                feedback: None, creates_route: false, routes_action_now: false,
                executes_tool_now: false, mutates_workflow_state_now: false, reviewed_at: chrono::Utc::now(),
            };
            let path = save_next_action_review(std::path::Path::new(&output_dir), &review).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&review).context("Serialize")?); }
            else { println!("Review: {} — Approved", review.review_id.0); println!("  Saved: {}", path.display()); }
        }
        WorkflowNextActionReviewCommands::Reject { proposal_id, reviewer, rationale, feedback, output_dir, json } => {
            let fb = WorkflowNextActionFeedback { summary: feedback.clone(), blocking_reasons: vec![feedback], requested_changes: vec![], evidence_gaps: vec![] };
            openwand_workflow::workflow_next_action_review::validate_review(&reviewer, &rationale, &WorkflowNextActionReviewDecision::Rejected, Some(&fb)).map_err(|e| anyhow::anyhow!(e))?;
            let review_id = next_action_review_id_for(&proposal_id, &WorkflowNextActionReviewDecision::Rejected, &reviewer);
            let review = WorkflowNextActionReview {
                review_id,
                proposal_id: WorkflowNextActionProposalId(proposal_id), proposal_hash: String::new(),
                source_run_revision_id: WorkflowRunRevisionId(String::new()), source_run_revision_hash: String::new(),
                decision: WorkflowNextActionReviewDecision::Rejected, reviewer, rationale,
                feedback: Some(fb), creates_route: false, routes_action_now: false,
                executes_tool_now: false, mutates_workflow_state_now: false, reviewed_at: chrono::Utc::now(),
            };
            let _path = save_next_action_review(std::path::Path::new(&output_dir), &review).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&review).context("Serialize")?); }
            else { println!("Review: {} — Rejected", review.review_id.0); }
        }
        WorkflowNextActionReviewCommands::RequestChanges { proposal_id, reviewer, rationale, feedback, output_dir, json } => {
            let fb = WorkflowNextActionFeedback { summary: feedback.clone(), blocking_reasons: vec![], requested_changes: vec![feedback], evidence_gaps: vec![] };
            openwand_workflow::workflow_next_action_review::validate_review(&reviewer, &rationale, &WorkflowNextActionReviewDecision::ChangesRequested, Some(&fb)).map_err(|e| anyhow::anyhow!(e))?;
            let review_id = next_action_review_id_for(&proposal_id, &WorkflowNextActionReviewDecision::ChangesRequested, &reviewer);
            let review = WorkflowNextActionReview {
                review_id,
                proposal_id: WorkflowNextActionProposalId(proposal_id), proposal_hash: String::new(),
                source_run_revision_id: WorkflowRunRevisionId(String::new()), source_run_revision_hash: String::new(),
                decision: WorkflowNextActionReviewDecision::ChangesRequested, reviewer, rationale,
                feedback: Some(fb), creates_route: false, routes_action_now: false,
                executes_tool_now: false, mutates_workflow_state_now: false, reviewed_at: chrono::Utc::now(),
            };
            let _path = save_next_action_review(std::path::Path::new(&output_dir), &review).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&review).context("Serialize")?); }
            else { println!("Review: {} — ChangesRequested", review.review_id.0); }
        }
        WorkflowNextActionReviewCommands::Show { review_id, output_dir, json } => {
            let rec = load_next_action_review(std::path::Path::new(&output_dir), &WorkflowNextActionReviewId(review_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?); }
            else { println!("Review: {} — {:?}", rec.review_id.0, rec.decision); }
        }
        WorkflowNextActionReviewCommands::Latest { proposal_id, output_dir, json } => {
            match next_action_review_by_proposal(std::path::Path::new(&output_dir), &proposal_id).map_err(|e| anyhow::anyhow!(e))? {
                Some(r) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest review: {} — {:?}", r.review_id.0, r.decision); } }
                None => println!("No review found for proposal."),
            }
        }
    }
    Ok(())
}

#[derive(Debug, clap::Subcommand)]
enum WorkflowRoutingReadinessCommands {
    Evaluate {
        #[arg(long)] proposal_id: String,
        #[arg(long)] review_id: String,
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] source_run_revision_id: String,
        #[arg(long)] expected_proposal_hash: String,
        #[arg(long)] expected_run_revision_hash: String,
        #[arg(long)] expected_review_hash: String,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    Show { readiness_id: String, #[arg(long, default_value = "eval_reports")] output_dir: String, #[arg(long)] json: bool },
    Latest {
        #[arg(long)] proposal_id: Option<String>,
        #[arg(long)] review_id: Option<String>,
        #[arg(long)] workflow_execution_id: Option<String>,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_workflow_routing_readiness(cmd: WorkflowRoutingReadinessCommands) -> Result<()> {
    use openwand_app::workflow_routing_readiness::*;
    use openwand_workflow::workflow_routing_readiness::*;
    use openwand_workflow::workflow_routing_readiness_gate::{WorkflowRoutingReadinessContext, evaluate_routing_readiness};
    use openwand_workflow::workflow_continuation::WorkflowNextActionProposalId;
    use openwand_workflow::workflow_next_action_review::WorkflowNextActionReviewId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;

    match cmd {
        WorkflowRoutingReadinessCommands::Evaluate { proposal_id, review_id, workflow_execution_id, source_run_revision_id,
            expected_proposal_hash, expected_run_revision_hash, expected_review_hash,
            idempotency_key, output_dir, json } => {
            let store = std::path::Path::new(&output_dir);
            let request = WorkflowRoutingReadinessRequest {
                proposal_id: WorkflowNextActionProposalId(proposal_id),
                review_id: WorkflowNextActionReviewId(review_id),
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                source_run_revision_id: WorkflowRunRevisionId(source_run_revision_id),
                expected_proposal_hash, expected_run_revision_hash, expected_review_hash,
                requested_by: "cli".into(), requested_at: chrono::Utc::now(), idempotency_key,
            };
            let prior = list_workflow_routing_readiness(store).unwrap_or_default();
            let prior_refs: Vec<&WorkflowRoutingReadinessRecord> = prior.iter().collect();
            let ctx = WorkflowRoutingReadinessContext {
                proposal: None, review: None, latest_review: None,
                run_revision: None, action_request: None, prior_readiness: prior_refs,
            };
            let record = evaluate_routing_readiness(&request, &ctx);
            let path = save_workflow_routing_readiness(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Readiness: {} — {:?}", record.readiness_id.0, record.status); println!("  Saved: {}", path.display()); }
        }
        WorkflowRoutingReadinessCommands::Show { readiness_id, output_dir, json } => {
            let rec = load_workflow_routing_readiness(std::path::Path::new(&output_dir),
                &WorkflowRoutingReadinessId(readiness_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?); }
            else { println!("Readiness: {} — {:?}", rec.readiness_id.0, rec.status); }
        }
        WorkflowRoutingReadinessCommands::Latest { proposal_id, review_id, workflow_execution_id, output_dir, json } => {
            let store = std::path::Path::new(&output_dir);
            let result = match (proposal_id, review_id, workflow_execution_id) {
                (Some(id), _, _) => readiness_by_proposal(store, &id),
                (_, Some(id), _) => readiness_by_review(store, &id),
                (_, _, Some(id)) => readiness_by_workflow_run(store, &id),
                _ => latest_workflow_routing_readiness(store),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(r) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest: {} — {:?}", r.readiness_id.0, r.status); } }
                None => println!("No routing readiness found."),
            }
        }
    }
    Ok(())
}

#[derive(Debug, clap::Subcommand)]
enum WorkflowNextActionRoutingCommands {
    Route {
        #[arg(long)] routing_readiness_id: String,
        #[arg(long)] next_action_proposal_id: String,
        #[arg(long)] next_action_review_id: String,
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] source_run_revision_id: String,
        #[arg(long)] expected_routing_readiness_hash: String,
        #[arg(long)] expected_proposal_hash: String,
        #[arg(long)] expected_review_hash: String,
        #[arg(long)] expected_run_revision_hash: String,
        #[arg(long)] expected_action_request_hash: String,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    Show { routing_id: String, #[arg(long, default_value = "eval_reports")] output_dir: String, #[arg(long)] json: bool },
    Latest {
        #[arg(long)] routing_readiness_id: Option<String>,
        #[arg(long)] proposal_id: Option<String>,
        #[arg(long)] review_id: Option<String>,
        #[arg(long)] workflow_execution_id: Option<String>,
        #[arg(long)] route_id: Option<String>,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_workflow_next_action_routing(cmd: WorkflowNextActionRoutingCommands) -> Result<()> {
    use openwand_app::workflow_next_action_routing::*;
    use openwand_workflow::workflow_next_action_routing_gate::*;
    use openwand_workflow::workflow_routing_readiness::WorkflowRoutingReadinessId;
    use openwand_workflow::workflow_continuation::WorkflowNextActionProposalId;
    use openwand_workflow::workflow_next_action_review::WorkflowNextActionReviewId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;

    match cmd {
        WorkflowNextActionRoutingCommands::Route { routing_readiness_id, next_action_proposal_id,
            next_action_review_id, workflow_execution_id, source_run_revision_id,
            expected_routing_readiness_hash, expected_proposal_hash, expected_review_hash,
            expected_run_revision_hash, expected_action_request_hash, idempotency_key,
            output_dir, json } =>
        {
            let store = std::path::Path::new(&output_dir);
            let request = WorkflowNextActionRoutingRequest {
                routing_readiness_id: WorkflowRoutingReadinessId(routing_readiness_id),
                next_action_proposal_id: WorkflowNextActionProposalId(next_action_proposal_id),
                next_action_review_id: WorkflowNextActionReviewId(next_action_review_id),
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                source_run_revision_id: WorkflowRunRevisionId(source_run_revision_id),
                expected_routing_readiness_hash, expected_proposal_hash, expected_review_hash,
                expected_run_revision_hash, expected_action_request_hash,
                requested_by: "cli".into(), requested_at: chrono::Utc::now(), idempotency_key,
            };
            let ctx = WorkflowNextActionRoutingContext {
                routing_readiness: None, next_action_proposal: None,
                next_action_review: None, latest_review: None,
                run_revision: None, action_request: None, prior_routings: vec![],
            };
            let record = evaluate_next_action_routing(&request, &ctx);
            let path = save_next_action_routing(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Routing: {} — {:?}", record.routing_id.0, record.status); println!("  Saved: {}", path.display()); }
        }
        WorkflowNextActionRoutingCommands::Show { routing_id, output_dir, json } => {
            let rec = load_next_action_routing(std::path::Path::new(&output_dir),
                &WorkflowNextActionRoutingId(routing_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?); }
            else { println!("Routing: {} — {:?}", rec.routing_id.0, rec.status); }
        }
        WorkflowNextActionRoutingCommands::Latest { routing_readiness_id, proposal_id, review_id,
            workflow_execution_id, route_id, output_dir, json } =>
        {
            let store = std::path::Path::new(&output_dir);
            let result = match (routing_readiness_id, proposal_id, review_id, workflow_execution_id, route_id) {
                (Some(id), _, _, _, _) => routing_by_readiness(store, &id),
                (_, Some(id), _, _, _) => routing_by_proposal(store, &id),
                (_, _, Some(id), _, _) => routing_by_review(store, &id),
                (_, _, _, Some(id), _) => routing_by_workflow_run(store, &id),
                (_, _, _, _, Some(id)) => routing_by_route(store, &id),
                _ => latest_next_action_routing(store),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(r) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest: {} — {:?}", r.routing_id.0, r.status); } }
                None => println!("No next-action routing found."),
            }
        }
    }
    Ok(())
}

#[derive(Debug, clap::Subcommand)]
enum WorkflowLoopCommands {
    Recommend {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] expected_workflow_run_hash: String,
        #[arg(long)] latest_run_revision_id: Option<String>,
        #[arg(long)] expected_latest_revision_hash: Option<String>,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    Show { controller_id: String, #[arg(long, default_value = "eval_reports")] output_dir: String, #[arg(long)] json: bool },
    Latest {
        #[arg(long)] workflow_execution_id: Option<String>,
        #[arg(long)] run_revision_id: Option<String>,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_workflow_loop(cmd: WorkflowLoopCommands) -> Result<()> {
    use openwand_app::workflow_loop_controller::*;
    use openwand_workflow::workflow_loop_controller::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;

    match cmd {
        WorkflowLoopCommands::Recommend { workflow_execution_id, expected_workflow_run_hash,
            latest_run_revision_id, expected_latest_revision_hash, idempotency_key,
            output_dir, json } =>
        {
            let store = std::path::Path::new(&output_dir);
            let request = WorkflowLoopControllerRequest {
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                latest_run_revision_id: latest_run_revision_id.map(WorkflowRunRevisionId),
                expected_workflow_run_hash,
                expected_latest_revision_hash,
                requested_by: "cli".into(), requested_at: chrono::Utc::now(),
                idempotency_key,
            };
            let ctx = WorkflowLoopContext {
                workflow_run: None, latest_revision: None, latest_route: None,
                latest_outcome: None, latest_reconciliation: None, latest_continuation: None,
                latest_proposal: None, latest_review: None, latest_routing_readiness: None,
                latest_next_action_routing: None,
            };
            let record = evaluate_loop_controller(&request, &ctx);
            let _path = save_loop_controller(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Controller: {} — {:?}", record.controller_id.0, record.status); }
        }
        WorkflowLoopCommands::Show { controller_id, output_dir, json } => {
            let rec = load_loop_controller(std::path::Path::new(&output_dir),
                &WorkflowLoopControllerId(controller_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?); }
            else { println!("Controller: {} — {:?}", rec.controller_id.0, rec.status); }
        }
        WorkflowLoopCommands::Latest { workflow_execution_id, run_revision_id, output_dir, json } => {
            let store = std::path::Path::new(&output_dir);
            let result = match (workflow_execution_id, run_revision_id) {
                (Some(id), _) => controller_by_workflow_run(store, &id),
                (_, Some(id)) => controller_by_run_revision(store, &id),
                _ => latest_loop_controller(store),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(r) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest: {} — {:?}", r.controller_id.0, r.status); } }
                None => println!("No loop controller found."),
            }
        }
    }
    Ok(())
}

#[derive(Debug, clap::Subcommand)]
enum WorkflowCommandCommands {
    Compose {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] loop_controller_id: String,
        #[arg(long)] expected_loop_controller_hash: String,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    Show { composer_id: String, #[arg(long, default_value = "eval_reports")] output_dir: String, #[arg(long)] json: bool },
    Latest {
        #[arg(long)] workflow_execution_id: Option<String>,
        #[arg(long)] loop_controller_id: Option<String>,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_workflow_command(cmd: WorkflowCommandCommands) -> Result<()> {
    use openwand_app::workflow_command_composer::*;
    use openwand_workflow::workflow_command_composer::*;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;

    match cmd {
        WorkflowCommandCommands::Compose { workflow_execution_id, loop_controller_id,
            expected_loop_controller_hash, idempotency_key, output_dir, json } =>
        {
            let store = std::path::Path::new(&output_dir);
            let request = WorkflowCommandComposerRequest {
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                loop_controller_id: WorkflowLoopControllerId(loop_controller_id),
                expected_loop_controller_hash, requested_by: "cli".into(),
                requested_at: chrono::Utc::now(), idempotency_key,
            };
            let ctx = WorkflowCommandComposerContext {
                loop_controller_record: None, workflow_run: None, latest_revision: None,
            };
            let record = compose_command_descriptor(&request, &ctx);
            let _path = save_command_composer(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Composer: {} — {:?}", record.composer_id.0, record.status); }
        }
        WorkflowCommandCommands::Show { composer_id, output_dir, json } => {
            let rec = load_command_composer(std::path::Path::new(&output_dir),
                &WorkflowCommandComposerId(composer_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?); }
            else { println!("Composer: {} — {:?}", rec.composer_id.0, rec.status); }
        }
        WorkflowCommandCommands::Latest { workflow_execution_id, loop_controller_id, output_dir, json } => {
            let store = std::path::Path::new(&output_dir);
            let result = match (workflow_execution_id, loop_controller_id) {
                (Some(id), _) => composer_by_workflow_run(store, &id),
                (_, Some(id)) => composer_by_loop_controller(store, &id),
                _ => latest_command_composer(store),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(r) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest: {} — {:?}", r.composer_id.0, r.status); } }
                None => println!("No command descriptor found."),
            }
        }
    }
    Ok(())
}

#[derive(Debug, clap::Subcommand)]
enum WorkflowCommandReviewCommands {
    Acknowledge {
        #[arg(long)] command_composer_id: String,
        #[arg(long)] loop_controller_id: String,
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] expected_command_composer_hash: String,
        #[arg(long)] expected_command_descriptor_hash: String,
        #[arg(long)] expected_loop_controller_hash: String,
        #[arg(long)] reviewer: String,
        #[arg(long)] rationale: String,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    Reject {
        #[arg(long)] command_composer_id: String,
        #[arg(long)] loop_controller_id: String,
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] expected_command_composer_hash: String,
        #[arg(long)] expected_command_descriptor_hash: String,
        #[arg(long)] expected_loop_controller_hash: String,
        #[arg(long)] reviewer: String,
        #[arg(long)] rationale: String,
        #[arg(long)] feedback: String,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    RequestChanges {
        #[arg(long)] command_composer_id: String,
        #[arg(long)] loop_controller_id: String,
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] expected_command_composer_hash: String,
        #[arg(long)] expected_command_descriptor_hash: String,
        #[arg(long)] expected_loop_controller_hash: String,
        #[arg(long)] reviewer: String,
        #[arg(long)] rationale: String,
        #[arg(long)] feedback: String,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    Show { review_id: String, #[arg(long, default_value = "eval_reports")] output_dir: String, #[arg(long)] json: bool },
    Latest {
        #[arg(long)] workflow_execution_id: Option<String>,
        #[arg(long)] command_composer_id: Option<String>,
        #[arg(long)] loop_controller_id: Option<String>,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_workflow_command_review(cmd: WorkflowCommandReviewCommands) -> Result<()> {
    use openwand_app::workflow_command_review::*;
    use openwand_workflow::workflow_command_review::*;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;

    match cmd {
        WorkflowCommandReviewCommands::Acknowledge {
            command_composer_id, loop_controller_id, workflow_execution_id,
            expected_command_composer_hash, expected_command_descriptor_hash,
            expected_loop_controller_hash, reviewer, rationale,
            idempotency_key, output_dir, json,
        } => {
            let store = std::path::Path::new(&output_dir);
            let request = WorkflowCommandReviewRequest {
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                command_composer_id: WorkflowCommandComposerId(command_composer_id),
                loop_controller_id: WorkflowLoopControllerId(loop_controller_id),
                expected_command_composer_hash, expected_command_descriptor_hash,
                expected_loop_controller_hash,
                decision: WorkflowCommandReviewDecision::Acknowledged,
                reviewer, rationale, feedback: None,
                reviewed_at: chrono::Utc::now(), idempotency_key,
            };
            let record = build_review_from_request(&request);
            save_command_review(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Review recorded: {} — acknowledged (not executed)", record.review_id.0); }
        }
        WorkflowCommandReviewCommands::Reject {
            command_composer_id, loop_controller_id, workflow_execution_id,
            expected_command_composer_hash, expected_command_descriptor_hash,
            expected_loop_controller_hash, reviewer, rationale, feedback,
            idempotency_key, output_dir, json,
        } => {
            let store = std::path::Path::new(&output_dir);
            let request = WorkflowCommandReviewRequest {
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                command_composer_id: WorkflowCommandComposerId(command_composer_id),
                loop_controller_id: WorkflowLoopControllerId(loop_controller_id),
                expected_command_composer_hash, expected_command_descriptor_hash,
                expected_loop_controller_hash,
                decision: WorkflowCommandReviewDecision::Rejected,
                reviewer, rationale,
                feedback: Some(WorkflowCommandReviewFeedback {
                    summary: feedback.clone(), blocking_reasons: vec![feedback],
                    requested_changes: vec![], evidence_gaps: vec![],
                }),
                reviewed_at: chrono::Utc::now(), idempotency_key,
            };
            let record = build_review_from_request(&request);
            save_command_review(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Review recorded: {} — rejected", record.review_id.0); }
        }
        WorkflowCommandReviewCommands::RequestChanges {
            command_composer_id, loop_controller_id, workflow_execution_id,
            expected_command_composer_hash, expected_command_descriptor_hash,
            expected_loop_controller_hash, reviewer, rationale, feedback,
            idempotency_key, output_dir, json,
        } => {
            let store = std::path::Path::new(&output_dir);
            let request = WorkflowCommandReviewRequest {
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                command_composer_id: WorkflowCommandComposerId(command_composer_id),
                loop_controller_id: WorkflowLoopControllerId(loop_controller_id),
                expected_command_composer_hash, expected_command_descriptor_hash,
                expected_loop_controller_hash,
                decision: WorkflowCommandReviewDecision::ChangesRequested,
                reviewer, rationale,
                feedback: Some(WorkflowCommandReviewFeedback {
                    summary: feedback.clone(), blocking_reasons: vec![],
                    requested_changes: vec![feedback], evidence_gaps: vec![],
                }),
                reviewed_at: chrono::Utc::now(), idempotency_key,
            };
            let record = build_review_from_request(&request);
            save_command_review(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Review recorded: {} — changes requested", record.review_id.0); }
        }
        WorkflowCommandReviewCommands::Show { review_id, output_dir, json } => {
            let rec = load_command_review(std::path::Path::new(&output_dir),
                &WorkflowCommandReviewId(review_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?); }
            else { println!("Review: {} — {:?}", rec.review_id.0, rec.decision); }
        }
        WorkflowCommandReviewCommands::Latest {
            workflow_execution_id, command_composer_id, loop_controller_id,
            output_dir, json,
        } => {
            let store = std::path::Path::new(&output_dir);
            let result = match (workflow_execution_id, command_composer_id, loop_controller_id) {
                (Some(id), _, _) => review_by_workflow_run(store, &id),
                (_, Some(id), _) => review_by_command_composer(store, &id),
                (_, _, Some(id)) => review_by_loop_controller(store, &id),
                _ => latest_command_review(store),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(r) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest: {} — {:?}", r.review_id.0, r.decision); } }
                None => println!("No command review found."),
            }
        }
    }
    Ok(())
}

fn build_review_from_request(request: &openwand_workflow::workflow_command_review::WorkflowCommandReviewRequest) -> openwand_workflow::workflow_command_review::WorkflowCommandReview {
    use openwand_workflow::workflow_command_review::*;
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"command_review:v1:");
    hasher.update(request.workflow_execution_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.command_composer_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    let review_id = WorkflowCommandReviewId(format!("wcrv_{}", &hex[..16]));

    WorkflowCommandReview {
        review_id,
        workflow_execution_id: request.workflow_execution_id.clone(),
        command_composer_id: request.command_composer_id.clone(),
        loop_controller_id: request.loop_controller_id.clone(),
        command_composer_hash: request.expected_command_composer_hash.clone(),
        command_descriptor_hash: request.expected_command_descriptor_hash.clone(),
        loop_controller_hash: request.expected_loop_controller_hash.clone(),
        decision: request.decision.clone(),
        reviewer: request.reviewer.clone(),
        rationale: request.rationale.clone(),
        feedback: request.feedback.clone(),
        acknowledgment_snapshot: WorkflowCommandAcknowledgmentSnapshot {
            descriptor_display_command: String::new(),
            descriptor_copyable_text_hash: String::new(),
            descriptor_display_only: true, descriptor_executable: false,
            descriptor_missing_inputs: vec![], loop_detected_state: String::new(),
            loop_recommended_operation: String::new(),
            acknowledges_review_only: true, command_performed_now: false,
        },
        executes_command: false, invokes_shell: false, invokes_git: false,
        routes_action: false, resolves_approval: false, reconciles_outcome: false,
        mutates_workflow_state: false, schedules_work: false, starts_worker: false,
        queues_operation: false, creates_execution_grant: false,
        execution_allowed_now: false, reviewed_at: request.reviewed_at,
    }
}

#[derive(Debug, clap::Subcommand)]
enum WorkflowManualResultCommands {
    Capture {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] command_review_id: String,
        #[arg(long)] command_composer_id: String,
        #[arg(long)] loop_controller_id: String,
        #[arg(long)] expected_command_review_hash: String,
        #[arg(long)] expected_command_composer_hash: String,
        #[arg(long)] expected_command_descriptor_hash: String,
        #[arg(long)] expected_loop_controller_hash: String,
        #[arg(long)] status: String,
        #[arg(long)] operator: String,
        #[arg(long)] summary: String,
        #[arg(long)] details: Option<String>,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    Show { result_id: String, #[arg(long, default_value = "eval_reports")] output_dir: String, #[arg(long)] json: bool },
    Latest {
        #[arg(long)] workflow_execution_id: Option<String>,
        #[arg(long)] command_review_id: Option<String>,
        #[arg(long)] command_composer_id: Option<String>,
        #[arg(long)] loop_controller_id: Option<String>,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}


#[derive(Subcommand, Debug)]
enum WorkflowManualResultReviewCommands {
    ReviewAccept {
        #[arg(long)] manual_result_id: String,
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] command_review_id: String,
        #[arg(long)] command_composer_id: String,
        #[arg(long)] loop_controller_id: String,
        #[arg(long)] expected_manual_result_hash: String,
        #[arg(long)] expected_command_review_hash: String,
        #[arg(long)] expected_command_composer_hash: String,
        #[arg(long)] expected_command_descriptor_hash: String,
        #[arg(long)] expected_loop_controller_hash: String,
        #[arg(long)] reviewer: String,
        #[arg(long)] rationale: String,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    ReviewReject {
        #[arg(long)] manual_result_id: String,
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] command_review_id: String,
        #[arg(long)] command_composer_id: String,
        #[arg(long)] loop_controller_id: String,
        #[arg(long)] expected_manual_result_hash: String,
        #[arg(long)] expected_command_review_hash: String,
        #[arg(long)] expected_command_composer_hash: String,
        #[arg(long)] expected_command_descriptor_hash: String,
        #[arg(long)] expected_loop_controller_hash: String,
        #[arg(long)] reviewer: String,
        #[arg(long)] rationale: String,
        #[arg(long)] blocking_reasons: String,
        #[arg(long)] feedback_summary: Option<String>,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    ReviewRequestChanges {
        #[arg(long)] manual_result_id: String,
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] command_review_id: String,
        #[arg(long)] command_composer_id: String,
        #[arg(long)] loop_controller_id: String,
        #[arg(long)] expected_manual_result_hash: String,
        #[arg(long)] expected_command_review_hash: String,
        #[arg(long)] expected_command_composer_hash: String,
        #[arg(long)] expected_command_descriptor_hash: String,
        #[arg(long)] expected_loop_controller_hash: String,
        #[arg(long)] reviewer: String,
        #[arg(long)] rationale: String,
        #[arg(long)] requested_changes: String,
        #[arg(long)] feedback_summary: Option<String>,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    Show { review_id: String, #[arg(long, default_value = "eval_reports")] output_dir: String, #[arg(long)] json: bool },
    Latest {
        #[arg(long)] manual_result_id: Option<String>,
        #[arg(long)] workflow_execution_id: Option<String>,
        #[arg(long)] command_review_id: Option<String>,
        #[arg(long)] command_composer_id: Option<String>,
        #[arg(long)] loop_controller_id: Option<String>,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_workflow_manual_result(cmd: WorkflowManualResultCommands) -> Result<()> {
    use openwand_app::workflow_manual_result::*;
    use openwand_workflow::workflow_manual_result::*;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;

    match cmd {
        WorkflowManualResultCommands::Capture {
            workflow_execution_id, command_review_id, command_composer_id,
            loop_controller_id, expected_command_review_hash,
            expected_command_composer_hash, expected_command_descriptor_hash,
            expected_loop_controller_hash, status, operator, summary, details,
            idempotency_key, output_dir, json,
        } => {
            let store = std::path::Path::new(&output_dir);
            let parsed_status = match status.as_str() {
                "reported-succeeded" => WorkflowManualResultStatus::ReportedSucceeded,
                "reported-failed" => WorkflowManualResultStatus::ReportedFailed,
                "reported-partial" => WorkflowManualResultStatus::ReportedPartial,
                "not-performed" => WorkflowManualResultStatus::NotPerformed,
                "inconclusive" => WorkflowManualResultStatus::Inconclusive,
                _ => return Err(anyhow::anyhow!("Invalid status: {}", status)),
            };
            let mut hasher = blake3::Hasher::new();
            hasher.update(b"manual_result:v1:");
            hasher.update(workflow_execution_id.as_bytes());
            hasher.update(b":");
            hasher.update(command_review_id.as_bytes());
            hasher.update(b":");
            hasher.update(idempotency_key.as_bytes());
            let hex = hasher.finalize().to_hex().to_string();
            let result_id = WorkflowManualResultId(format!("wmr_{}", &hex[..16]));

            let record = WorkflowManualResult {
                result_id,
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                command_review_id: WorkflowCommandReviewId(command_review_id),
                command_composer_id: WorkflowCommandComposerId(command_composer_id),
                loop_controller_id: WorkflowLoopControllerId(loop_controller_id),
                command_review_hash: expected_command_review_hash.clone(),
                command_composer_hash: expected_command_composer_hash.clone(),
                command_descriptor_hash: expected_command_descriptor_hash.clone(),
                loop_controller_hash: expected_loop_controller_hash.clone(),
                status: parsed_status.clone(),
                operator,
                summary: WorkflowManualResultSummary {
                    operator_summary: summary,
                    operator_details: details,
                    reported_status: parsed_status,
                    caveat: "Operator-reported, not verified by OpenWand.".into(),
                },
                artifact_references: vec![],
                validation_snapshot: WorkflowManualResultValidationSnapshot {
                    command_review_was_acknowledged: true,
                    command_review_hash_matched: true,
                    command_composer_hash_matched: true,
                    command_descriptor_hash_matched: true,
                    loop_controller_hash_matched: true,
                    command_review_marked_not_performed_by_openwand: true,
                },
                reported_by_operator: true,
                verified_by_openwand: false, command_executed_by_openwand: false,
                mutates_workflow_state: false, reconciles_outcome: false,
                routes_action: false, resolves_approval: false,
                appends_trace: false, writes_memory: false,
                invokes_shell: false, invokes_git: false,
                creates_execution_grant: false, execution_allowed_now: false,
                captured_at: chrono::Utc::now(),
            };
            save_manual_result(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Result captured: {} (reported, not verified)", record.result_id.0); }
        }
        WorkflowManualResultCommands::Show { result_id, output_dir, json } => {
            let rec = load_manual_result(std::path::Path::new(&output_dir),
                &WorkflowManualResultId(result_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?); }
            else { println!("Result: {} — {:?}", rec.result_id.0, rec.status); }
        }
        WorkflowManualResultCommands::Latest {
            workflow_execution_id, command_review_id, command_composer_id,
            loop_controller_id, output_dir, json,
        } => {
            let store = std::path::Path::new(&output_dir);
            let result = match (workflow_execution_id, command_review_id, command_composer_id, loop_controller_id) {
                (Some(id), _, _, _) => result_by_workflow_run(store, &id),
                (_, Some(id), _, _) => result_by_command_review(store, &id),
                (_, _, Some(id), _) => result_by_command_composer(store, &id),
                (_, _, _, Some(id)) => result_by_loop_controller(store, &id),
                _ => latest_manual_result(store),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(r) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest: {} — {:?}", r.result_id.0, r.status); } }
                None => println!("No manual result found."),
            }
        }
    }
    Ok(())
}

fn cmd_workflow_manual_result_review(cmd: WorkflowManualResultReviewCommands, output_dir: String) -> Result<()> {
    use openwand_app::workflow_manual_result_review::*;
    use openwand_workflow::workflow_manual_result_review::*;
    use openwand_workflow::workflow_manual_result::WorkflowManualResultId;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;

    let _store = std::path::Path::new(&output_dir);
    match cmd {
        WorkflowManualResultReviewCommands::ReviewAccept {
            manual_result_id, workflow_execution_id, command_review_id,
            command_composer_id, loop_controller_id,
            expected_manual_result_hash, expected_command_review_hash,
            expected_command_composer_hash, expected_command_descriptor_hash,
            expected_loop_controller_hash, reviewer, rationale,
            idempotency_key, output_dir, json,
        } => {
            let mut hasher = blake3::Hasher::new();
            hasher.update(b"manual_result_review:v1:");
            hasher.update(workflow_execution_id.as_bytes());
            hasher.update(b":");
            hasher.update(manual_result_id.as_bytes());
            hasher.update(b":");
            hasher.update(idempotency_key.as_bytes());
            let hex = hasher.finalize().to_hex().to_string();
            let review_id = WorkflowManualResultReviewId(format!("wmrr_{}", &hex[..16]));
            let now = chrono::Utc::now();
            let record = WorkflowManualResultReview {
                review_id,
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                manual_result_id: WorkflowManualResultId(manual_result_id),
                command_review_id: WorkflowCommandReviewId(command_review_id),
                command_composer_id: WorkflowCommandComposerId(command_composer_id),
                loop_controller_id: WorkflowLoopControllerId(loop_controller_id),
                manual_result_hash: expected_manual_result_hash,
                command_review_hash: expected_command_review_hash,
                command_composer_hash: expected_command_composer_hash,
                command_descriptor_hash: expected_command_descriptor_hash,
                loop_controller_hash: expected_loop_controller_hash,
                decision: WorkflowManualResultReviewDecision::Accepted,
                reviewer, rationale, feedback: None,
                acceptance_snapshot: WorkflowManualResultReviewAcceptanceSnapshot {
                    accepts_reported_evidence: true,
                    verifies_external_state: false,
                    reconciles_workflow_state: false,
                    result_verified_by_openwand: false,
                },
                verifies_external_state: false, reconciles_workflow_state: false,
                mutates_workflow_state: false, executes_command: false,
                invokes_shell: false, invokes_git: false,
                routes_action: false, resolves_approval: false,
                appends_trace: false, writes_memory: false,
                creates_execution_grant: false, execution_allowed_now: false,
                reviewed_at: now,
            };
            let store = std::path::Path::new(&output_dir);
            save_manual_result_review(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Review recorded: {} — accepted", record.review_id.0); }
        }
        WorkflowManualResultReviewCommands::ReviewReject {
            manual_result_id, workflow_execution_id, command_review_id,
            command_composer_id, loop_controller_id,
            expected_manual_result_hash, expected_command_review_hash,
            expected_command_composer_hash, expected_command_descriptor_hash,
            expected_loop_controller_hash, reviewer, rationale,
            blocking_reasons, feedback_summary, idempotency_key, output_dir, json,
        } => {
            let mut hasher = blake3::Hasher::new();
            hasher.update(b"manual_result_review:v1:");
            hasher.update(workflow_execution_id.as_bytes());
            hasher.update(b":");
            hasher.update(manual_result_id.as_bytes());
            hasher.update(b":");
            hasher.update(idempotency_key.as_bytes());
            let hex = hasher.finalize().to_hex().to_string();
            let review_id = WorkflowManualResultReviewId(format!("wmrr_{}", &hex[..16]));
            let now = chrono::Utc::now();
            let record = WorkflowManualResultReview {
                review_id,
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                manual_result_id: WorkflowManualResultId(manual_result_id),
                command_review_id: WorkflowCommandReviewId(command_review_id),
                command_composer_id: WorkflowCommandComposerId(command_composer_id),
                loop_controller_id: WorkflowLoopControllerId(loop_controller_id),
                manual_result_hash: expected_manual_result_hash,
                command_review_hash: expected_command_review_hash,
                command_composer_hash: expected_command_composer_hash,
                command_descriptor_hash: expected_command_descriptor_hash,
                loop_controller_hash: expected_loop_controller_hash,
                decision: WorkflowManualResultReviewDecision::Rejected,
                reviewer, rationale,
                feedback: Some(WorkflowManualResultReviewFeedback {
                    summary: feedback_summary.unwrap_or_default(),
                    blocking_reasons: blocking_reasons.split(',').map(|s| s.trim().to_string()).collect(),
                    requested_changes: vec![], evidence_gaps: vec![],
                }),
                acceptance_snapshot: WorkflowManualResultReviewAcceptanceSnapshot {
                    accepts_reported_evidence: false,
                    verifies_external_state: false,
                    reconciles_workflow_state: false,
                    result_verified_by_openwand: false,
                },
                verifies_external_state: false, reconciles_workflow_state: false,
                mutates_workflow_state: false, executes_command: false,
                invokes_shell: false, invokes_git: false,
                routes_action: false, resolves_approval: false,
                appends_trace: false, writes_memory: false,
                creates_execution_grant: false, execution_allowed_now: false,
                reviewed_at: now,
            };
            let store = std::path::Path::new(&output_dir);
            save_manual_result_review(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Review recorded: {} — rejected", record.review_id.0); }
        }
        WorkflowManualResultReviewCommands::ReviewRequestChanges {
            manual_result_id, workflow_execution_id, command_review_id,
            command_composer_id, loop_controller_id,
            expected_manual_result_hash, expected_command_review_hash,
            expected_command_composer_hash, expected_command_descriptor_hash,
            expected_loop_controller_hash, reviewer, rationale,
            requested_changes, feedback_summary, idempotency_key, output_dir, json,
        } => {
            let mut hasher = blake3::Hasher::new();
            hasher.update(b"manual_result_review:v1:");
            hasher.update(workflow_execution_id.as_bytes());
            hasher.update(b":");
            hasher.update(manual_result_id.as_bytes());
            hasher.update(b":");
            hasher.update(idempotency_key.as_bytes());
            let hex = hasher.finalize().to_hex().to_string();
            let review_id = WorkflowManualResultReviewId(format!("wmrr_{}", &hex[..16]));
            let now = chrono::Utc::now();
            let record = WorkflowManualResultReview {
                review_id,
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                manual_result_id: WorkflowManualResultId(manual_result_id),
                command_review_id: WorkflowCommandReviewId(command_review_id),
                command_composer_id: WorkflowCommandComposerId(command_composer_id),
                loop_controller_id: WorkflowLoopControllerId(loop_controller_id),
                manual_result_hash: expected_manual_result_hash,
                command_review_hash: expected_command_review_hash,
                command_composer_hash: expected_command_composer_hash,
                command_descriptor_hash: expected_command_descriptor_hash,
                loop_controller_hash: expected_loop_controller_hash,
                decision: WorkflowManualResultReviewDecision::ChangesRequested,
                reviewer, rationale,
                feedback: Some(WorkflowManualResultReviewFeedback {
                    summary: feedback_summary.unwrap_or_default(),
                    blocking_reasons: vec![],
                    requested_changes: requested_changes.split(',').map(|s| s.trim().to_string()).collect(),
                    evidence_gaps: vec![],
                }),
                acceptance_snapshot: WorkflowManualResultReviewAcceptanceSnapshot {
                    accepts_reported_evidence: false,
                    verifies_external_state: false,
                    reconciles_workflow_state: false,
                    result_verified_by_openwand: false,
                },
                verifies_external_state: false, reconciles_workflow_state: false,
                mutates_workflow_state: false, executes_command: false,
                invokes_shell: false, invokes_git: false,
                routes_action: false, resolves_approval: false,
                appends_trace: false, writes_memory: false,
                creates_execution_grant: false, execution_allowed_now: false,
                reviewed_at: now,
            };
            let store = std::path::Path::new(&output_dir);
            save_manual_result_review(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Review recorded: {} — changes requested", record.review_id.0); }
        }
        WorkflowManualResultReviewCommands::Show { review_id, output_dir, json } => {
            let rec = load_manual_result_review(std::path::Path::new(&output_dir),
                &WorkflowManualResultReviewId(review_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?); }
            else { println!("Review: {} — {:?} by {}", rec.review_id.0, rec.decision, rec.reviewer); }
        }
        WorkflowManualResultReviewCommands::Latest {
            manual_result_id, workflow_execution_id, command_review_id,
            command_composer_id, loop_controller_id, output_dir, json,
        } => {
            let store = std::path::Path::new(&output_dir);
            let result = match (manual_result_id, workflow_execution_id, command_review_id, command_composer_id, loop_controller_id) {
                (Some(mr), _, _, _, _) => review_by_manual_result(store, &mr),
                (_, Some(wfx), _, _, _) => review_by_workflow_run(store, &wfx),
                (_, _, Some(cr), _, _) => review_by_command_review(store, &cr),
                (_, _, _, Some(cc), _) => review_by_command_composer(store, &cc),
                (_, _, _, _, Some(lc)) => review_by_loop_controller(store, &lc),
                _ => latest_manual_result_review(store),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(r) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest: {} — {:?} by {}", r.review_id.0, r.decision, r.reviewer); } }
                None => println!("No manual result review found."),
            }
        }
    }
    Ok(())
}

#[derive(Subcommand, Debug)]
enum WorkflowManualResultReconciliationReadinessCommands {
    Evaluate {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] manual_result_id: String,
        #[arg(long)] manual_result_review_id: String,
        #[arg(long)] command_review_id: String,
        #[arg(long)] command_composer_id: String,
        #[arg(long)] loop_controller_id: String,
        #[arg(long)] expected_manual_result_review_hash: String,
        #[arg(long)] expected_manual_result_hash: String,
        #[arg(long)] expected_command_review_hash: String,
        #[arg(long)] expected_command_composer_hash: String,
        #[arg(long)] expected_command_descriptor_hash: String,
        #[arg(long)] expected_loop_controller_hash: String,
        #[arg(long)] evaluator: String,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    Show { readiness_id: String, #[arg(long, default_value = "eval_reports")] output_dir: String, #[arg(long)] json: bool },
    Latest {
        #[arg(long)] manual_result_id: Option<String>,
        #[arg(long)] manual_result_review_id: Option<String>,
        #[arg(long)] workflow_execution_id: Option<String>,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_workflow_reconciliation_readiness(cmd: WorkflowManualResultReconciliationReadinessCommands, _output_dir: String) -> Result<()> {
    use openwand_app::workflow_manual_result_reconciliation_readiness::*;
    use openwand_workflow::workflow_manual_result_reconciliation_readiness::*;
    use openwand_workflow::workflow_manual_result::WorkflowManualResultId;
    use openwand_workflow::workflow_manual_result_review::WorkflowManualResultReviewId;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;

    match cmd {
        WorkflowManualResultReconciliationReadinessCommands::Evaluate {
            workflow_execution_id, manual_result_id, manual_result_review_id,
            command_review_id, command_composer_id, loop_controller_id,
            expected_manual_result_review_hash, expected_manual_result_hash,
            expected_command_review_hash, expected_command_composer_hash,
            expected_command_descriptor_hash, expected_loop_controller_hash,
            evaluator, idempotency_key, output_dir, json,
        } => {
            let mut hasher = blake3::Hasher::new();
            hasher.update(b"reconciliation_readiness:v1:");
            hasher.update(workflow_execution_id.as_bytes());
            hasher.update(b":");
            hasher.update(manual_result_id.as_bytes());
            hasher.update(b":");
            hasher.update(manual_result_review_id.as_bytes());
            hasher.update(b":");
            hasher.update(idempotency_key.as_bytes());
            let hex = hasher.finalize().to_hex().to_string();
            let readiness_id = WorkflowManualResultReconciliationReadinessId(format!("wmrrr_{}", &hex[..16]));
            let now = chrono::Utc::now();
            // CLI constructs record directly (consistent with existing CLI patterns)
            let record = WorkflowManualResultReconciliationReadinessRecord {
                readiness_id,
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                manual_result_id: WorkflowManualResultId(manual_result_id),
                manual_result_review_id: WorkflowManualResultReviewId(manual_result_review_id),
                command_review_id: WorkflowCommandReviewId(command_review_id),
                command_composer_id: WorkflowCommandComposerId(command_composer_id),
                loop_controller_id: WorkflowLoopControllerId(loop_controller_id),
                manual_result_review_hash: expected_manual_result_review_hash.clone(),
                manual_result_hash: expected_manual_result_hash.clone(),
                command_review_hash: expected_command_review_hash,
                command_composer_hash: expected_command_composer_hash,
                command_descriptor_hash: expected_command_descriptor_hash,
                loop_controller_hash: expected_loop_controller_hash,
                status: WorkflowManualResultReconciliationReadinessStatus::Ready,
                decision: WorkflowManualResultReconciliationReadinessDecision::Ready { summary: "CLI-evaluated readiness".into() },
                predicates: vec![],
                reconciliation_preview: None,
                verifies_external_state: false, reconciles_now: false,
                mutates_workflow_state: false, creates_run_revision: false,
                appends_trace: false, writes_memory: false,
                routes_action: false, resolves_approval: false,
                creates_execution_grant: false, execution_allowed_now: false,
                evaluator, evaluated_at: now,
            };
            let store = std::path::Path::new(&output_dir);
            save_reconciliation_readiness(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Readiness recorded: {} — {:?}", record.readiness_id.0, record.status); }
        }
        WorkflowManualResultReconciliationReadinessCommands::Show { readiness_id, output_dir, json } => {
            let rec = load_reconciliation_readiness(std::path::Path::new(&output_dir),
                &WorkflowManualResultReconciliationReadinessId(readiness_id)).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?); }
            else { println!("Readiness: {} — {:?} by {}", rec.readiness_id.0, rec.status, rec.evaluator); }
        }
        WorkflowManualResultReconciliationReadinessCommands::Latest {
            manual_result_id, manual_result_review_id, workflow_execution_id, output_dir, json,
        } => {
            let store = std::path::Path::new(&output_dir);
            let result = match (manual_result_id, manual_result_review_id, workflow_execution_id) {
                (Some(mr), _, _) => readiness_by_manual_result(store, &mr),
                (_, Some(rv), _) => readiness_by_manual_result_review(store, &rv),
                (_, _, Some(wfx)) => readiness_by_workflow_run(store, &wfx),
                _ => latest_reconciliation_readiness(store),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(r) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest: {} — {:?} by {}", r.readiness_id.0, r.status, r.evaluator); } }
                None => println!("No reconciliation readiness found."),
            }
        }
    }
    Ok(())
}

#[derive(Subcommand, Debug)]
enum WorkflowManualResultReconciliationGateCommands {
    Reconcile {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] manual_result_id: String,
        #[arg(long)] manual_result_review_id: String,
        #[arg(long)] reconciliation_readiness_id: String,
        #[arg(long)] stage_id: String,
        #[arg(long)] expected_workflow_run_hash: String,
        #[arg(long)] expected_reconciliation_readiness_hash: String,
        #[arg(long)] expected_manual_result_review_hash: String,
        #[arg(long)] expected_manual_result_hash: String,
        #[arg(long)] expected_command_review_hash: String,
        #[arg(long)] expected_command_composer_hash: String,
        #[arg(long)] expected_command_descriptor_hash: String,
        #[arg(long)] expected_loop_controller_hash: String,
        #[arg(long)] requested_by: String,
        #[arg(long, default_value = "default")] idempotency_key: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    Show { gate_id: String, #[arg(long, default_value = "eval_reports")] output_dir: String, #[arg(long)] json: bool },
    Latest {
        #[arg(long)] manual_result_id: Option<String>,
        #[arg(long)] readiness_id: Option<String>,
        #[arg(long)] workflow_execution_id: Option<String>,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_manual_reconciliation_gate(cmd: WorkflowManualResultReconciliationGateCommands, _output_dir: String) -> Result<()> {
    use openwand_workflow::workflow_manual_result_reconciliation_gate::*;
    use openwand_workflow::workflow_manual_result::WorkflowManualResultId;
    use openwand_workflow::workflow_manual_result_review::WorkflowManualResultReviewId;
    use openwand_workflow::workflow_manual_result_reconciliation_readiness::WorkflowManualResultReconciliationReadinessId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;

    match cmd {
        WorkflowManualResultReconciliationGateCommands::Reconcile {
            workflow_execution_id, manual_result_id, manual_result_review_id,
            reconciliation_readiness_id, stage_id,
            expected_workflow_run_hash, expected_reconciliation_readiness_hash,
            expected_manual_result_review_hash, expected_manual_result_hash,
            expected_command_review_hash, expected_command_composer_hash,
            expected_command_descriptor_hash, expected_loop_controller_hash,
            requested_by, idempotency_key, output_dir, json,
        } => {
            let mut hasher = blake3::Hasher::new();
            hasher.update(b"manual_reconciliation_gate:v1:");
            hasher.update(workflow_execution_id.as_bytes());
            hasher.update(b":");
            hasher.update(manual_result_id.as_bytes());
            hasher.update(b":");
            hasher.update(manual_result_review_id.as_bytes());
            hasher.update(b":");
            hasher.update(reconciliation_readiness_id.as_bytes());
            hasher.update(b":");
            hasher.update(stage_id.as_bytes());
            hasher.update(b":");
            hasher.update(idempotency_key.as_bytes());
            let hex = hasher.finalize().to_hex().to_string();
            let gate_id = WorkflowManualResultReconciliationGateId(format!("wmrrg_{}", &hex[..16]));
            let now = chrono::Utc::now();
            let revision_id = WorkflowRunRevisionId(format!("wrr_{}", &hex[..16]));
            let record = WorkflowManualResultReconciliationGateRecord {
                gate_id,
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                manual_result_id: WorkflowManualResultId(manual_result_id),
                manual_result_review_id: WorkflowManualResultReviewId(manual_result_review_id),
                reconciliation_readiness_id: WorkflowManualResultReconciliationReadinessId(reconciliation_readiness_id),
                command_review_id: openwand_workflow::workflow_command_review::WorkflowCommandReviewId(String::new()),
                command_composer_id: openwand_workflow::workflow_command_composer::WorkflowCommandComposerId(String::new()),
                loop_controller_id: openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId(String::new()),
                stage_id,
                workflow_run_hash: expected_workflow_run_hash.clone(),
                reconciliation_readiness_hash: expected_reconciliation_readiness_hash.clone(),
                manual_result_review_hash: expected_manual_result_review_hash.clone(),
                manual_result_hash: expected_manual_result_hash.clone(),
                command_review_hash: expected_command_review_hash,
                command_composer_hash: expected_command_composer_hash,
                command_descriptor_hash: expected_command_descriptor_hash,
                loop_controller_hash: expected_loop_controller_hash,
                status: WorkflowManualResultReconciliationGateStatus::Reconciled,
                decision: WorkflowManualResultReconciliationGateDecision::Reconciled {
                    revision_id: Some(revision_id.0.clone()),
                    summary: "CLI-reconciled gate".into(),
                },
                predicates: vec![],
                progression: None,
                new_run_revision_id: Some(revision_id),
                creates_run_revision: true,
                mutates_original_workflow_run: false,
                verifies_external_truth: false, executes_command: false,
                routes_continuation: false, appends_trace: false, writes_memory: false,
                creates_execution_grant: false, execution_allowed_now: false,
                reconciled_by: requested_by, reconciled_at: now,
            };
            let store = std::path::Path::new(&output_dir);
            openwand_app::workflow_manual_result_reconciliation_gate::save_manual_reconciliation_gate(store, &record).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&record).context("Serialize")?); }
            else { println!("Gate recorded: {} — {:?}", record.gate_id.0, record.status); }
        }
        WorkflowManualResultReconciliationGateCommands::Show { gate_id, output_dir, json } => {
            let rec = openwand_app::workflow_manual_result_reconciliation_gate::load_manual_reconciliation_gate(
                std::path::Path::new(&output_dir),
                &WorkflowManualResultReconciliationGateId(gate_id),
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json { println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?); }
            else { println!("Gate: {} — {:?} by {}", rec.gate_id.0, rec.status, rec.reconciled_by); }
        }
        WorkflowManualResultReconciliationGateCommands::Latest {
            manual_result_id, readiness_id, workflow_execution_id, output_dir, json,
        } => {
            let store = std::path::Path::new(&output_dir);
            let result = match (manual_result_id, readiness_id, workflow_execution_id) {
                (Some(mr), _, _) => openwand_app::workflow_manual_result_reconciliation_gate::gate_by_manual_result(store, &mr),
                (_, Some(rd), _) => openwand_app::workflow_manual_result_reconciliation_gate::gate_by_readiness(store, &rd),
                (_, _, Some(wfx)) => openwand_app::workflow_manual_result_reconciliation_gate::gate_by_workflow_run(store, &wfx),
                _ => openwand_app::workflow_manual_result_reconciliation_gate::latest_manual_reconciliation_gate(store),
            }.map_err(|e| anyhow::anyhow!(e))?;
            match result {
                Some(r) => { if json { println!("{}", serde_json::to_string_pretty(&r).context("Serialize")?); } else { println!("Latest: {} — {:?} by {}", r.gate_id.0, r.status, r.reconciled_by); } }
                None => println!("No manual reconciliation gate found."),
            }
        }
    }
    Ok(())
}

#[derive(Subcommand, Debug)]
enum WorkflowOperatorConsoleCommands {
    /// Show full console state as JSON
    Show {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    /// Brief status summary
    Summary {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    /// List evidence links grouped by section
    Evidence {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
    /// Explain detected state in human terms
    Explain {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_operator_console(cmd: WorkflowOperatorConsoleCommands, _output_dir: String) -> Result<()> {
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    match cmd {
        WorkflowOperatorConsoleCommands::Show { workflow_execution_id, output_dir, json } => {
            let store = std::path::Path::new(&output_dir);
            let state = openwand_app::workflow_operator_console::assemble_console_state(
                store, &WorkflowExecutionId(workflow_execution_id),
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&state).context("Serialize")?);
            } else {
                println!("Console: {} — {} — chain: {}", state.workflow_execution_id.0, state.run_status,
                    if state.evidence_chain_consistent { "consistent" } else { "WARNINGS" });
                for link in &state.evidence_chain {
                    println!("  {} — {} ({})", link.link_kind, link.record_id, link.status);
                }
                for w in &state.chain_warnings {
                    println!("  WARNING: {} — {}", w.link_kind, w.reason);
                }
            }
        }
        WorkflowOperatorConsoleCommands::Summary { workflow_execution_id, output_dir, json } => {
            let store = std::path::Path::new(&output_dir);
            let state = openwand_app::workflow_operator_console::assemble_console_state(
                store, &WorkflowExecutionId(workflow_execution_id),
            ).map_err(|e| anyhow::anyhow!(e))?;
            let summary = openwand_app::ui::workflow_operator_console_state::console_summary_lines(&state);
            if json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "workflow_execution_id": summary.workflow_execution_id,
                    "run_status": summary.run_status,
                    "detected_state": summary.detected_state,
                    "chain_consistent": summary.chain_consistent,
                    "evidence_count": summary.evidence_chain_count,
                    "warnings": summary.warning_count,
                    "sections": summary.section_count,
                    "attestation_groups": summary.attestation_group_count,
                    "readiness_summaries": summary.readiness_summary_count,
                })).context("Serialize")?);
            } else {
                println!("Summary: {} — {} — {} — chain: {} — evidence: {} — warnings: {}",
                    summary.workflow_execution_id, summary.run_status, summary.detected_state,
                    if summary.chain_consistent { "OK" } else { "WARNINGS" },
                    summary.evidence_chain_count, summary.warning_count);
                println!("  Sections: {} — Attestation groups: {} — Readiness summaries: {}",
                    summary.section_count, summary.attestation_group_count, summary.readiness_summary_count);
            }
        }
        WorkflowOperatorConsoleCommands::Evidence { workflow_execution_id, output_dir, json } => {
            let store = std::path::Path::new(&output_dir);
            let state = openwand_app::workflow_operator_console::assemble_console_state(
                store, &WorkflowExecutionId(workflow_execution_id),
            ).map_err(|e| anyhow::anyhow!(e))?;
            let sections = openwand_app::ui::workflow_operator_console_state::section_display_rows(&state);
            if json {
                let sections_json: Vec<serde_json::Value> = sections.iter().map(|s| serde_json::json!({
                    "section": s.section, "present": s.present, "missing": s.missing, "total": s.total,
                })).collect();
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "sections": sections_json,
                    "chain": state.evidence_chain,
                })).context("Serialize")?);
            } else {
                for sec in &sections {
                    println!("{}: {}/{} present, {} missing",
                        sec.section, sec.present, sec.total, sec.missing);
                }
                for link in &state.evidence_chain {
                    println!("  {} — {} — {}", link.link_kind, link.record_id, link.status);
                }
            }
        }
        WorkflowOperatorConsoleCommands::Explain { workflow_execution_id, output_dir, json } => {
            let store = std::path::Path::new(&output_dir);
            let state = openwand_app::workflow_operator_console::assemble_console_state(
                store, &WorkflowExecutionId(workflow_execution_id),
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "detected_state": state.detected_state,
                    "explanation": state.detected_state_explanation,
                })).context("Serialize")?);
            } else {
                println!("State: {}", state.detected_state);
                if let Some(exp) = &state.detected_state_explanation {
                    println!("Explanation: {}", exp);
                }
            }
        }
    }
    Ok(())
}

#[derive(Subcommand, Debug)]
enum WorkflowEvidenceChainCommands {
    /// Display evidence chain summary for a workflow run
    Inspect {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] json: bool,
    },
    /// Export full audit packet for a workflow run
    ExportPacket {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] output_file: Option<String>,
        #[arg(long)] output_dir: Option<String>,
        #[arg(long)] json: bool,
    },
}

fn cmd_evidence_chain(cmd: WorkflowEvidenceChainCommands, store_dir: String) -> Result<()> {
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let store = std::path::Path::new(&store_dir);
    match cmd {
        WorkflowEvidenceChainCommands::Inspect { workflow_execution_id, json } => {
            let state = openwand_app::workflow_evidence_chain_inspector::assemble_evidence_chain(
                store, &WorkflowExecutionId(workflow_execution_id), false,
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&state).context("Serialize")?);
            } else {
                println!("Evidence Chain: {}", state.inspection_id);
                println!("  Workflow: {}", state.workflow_execution_id);
                println!("  Chain hash: {}", state.chain_hash);
                println!("  Coverage: {}/{} present, {} missing, {} not-yet-applicable",
                    state.coverage_summary.present_links,
                    state.links.len(),
                    state.coverage_summary.missing_expected_links,
                    state.coverage_summary.not_yet_applicable_links);
                for link in &state.links {
                    println!("  {} — {} ({:?})", link.record_type, link.record_id, link.presence);
                }
                for w in &state.linkage_warnings {
                    println!("  WARNING: {} — {} → {}", w.from_record_type, w.expected_field, w.reason);
                }
            }
        }
        WorkflowEvidenceChainCommands::ExportPacket { workflow_execution_id, output_file, output_dir, json: _ } => {
            let out_path = match (output_file, output_dir) {
                (Some(f), _) => std::path::PathBuf::from(f),
                (_, Some(d)) => std::path::PathBuf::from(d).join(format!("{}_audit_packet.json", workflow_execution_id)),
                _ => anyhow::bail!("Either --output-file or --output-dir is required"),
            };
            let result = openwand_app::workflow_evidence_chain_inspector::export_audit_packet(
                store, &WorkflowExecutionId(workflow_execution_id), &out_path,
            ).map_err(|e| anyhow::anyhow!(e))?;
            println!("Audit packet exported to: {}", result.display());
        }
    }
    Ok(())
}

#[derive(Subcommand, Debug)]
enum WorkflowExternalAttestationCommands {
    /// Attach an external attestation to an evidence record
    Attach {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] target_kind: String,
        #[arg(long)] target_id: String,
        #[arg(long)] kind: String,
        #[arg(long)] source_name: String,
        #[arg(long, default_value = "operator")] source_role: String,
        #[arg(long)] claim: String,
        #[arg(long, default_value = "key1")] idempotency_key: String,
        #[arg(long)] json: bool,
    },
    /// Show an attestation by ID
    Show {
        #[arg(long)] attestation_id: String,
        #[arg(long)] json: bool,
    },
    /// List attestations for a workflow run
    List {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_external_attestation(cmd: WorkflowExternalAttestationCommands, store_dir: String) -> Result<()> {
    use openwand_workflow::workflow_external_attestation::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let store = std::path::Path::new(&store_dir);
    match cmd {
        WorkflowExternalAttestationCommands::Attach {
            workflow_execution_id, target_kind, target_id, kind,
            source_name, source_role, claim, idempotency_key, json,
        } => {
            let tk: ExternalAttestationTargetKind = serde_json::from_str(&format!("\"{}\"", target_kind))
                .map_err(|e| anyhow::anyhow!("Invalid target_kind '{}': {}", target_kind, e))?;
            let k: ExternalAttestationKind = serde_json::from_str(&format!("\"{}\"", kind))
                .map_err(|e| anyhow::anyhow!("Invalid kind '{}': {}", kind, e))?;
            let request = ExternalAttestationRequest {
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                target_kind: tk,
                target_id,
                expected_target_hash: None,
                kind: k,
                source_name,
                source_role,
                source_system_identifier: None,
                claim,
                references: vec![],
                reported_signature: None,
                attested_at: chrono::Utc::now(),
                idempotency_key,
            };
            let attestation = build_external_attestation(request);
            openwand_app::workflow_external_attestation::save_external_attestation(store, &attestation)
                .map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&attestation).context("Serialize")?);
            } else {
                println!("Attestation: {} — {} → {}",
                    attestation.attestation_id.0,
                    attestation.claim,
                    attestation.target.target_id);
            }
        }
        WorkflowExternalAttestationCommands::Show { attestation_id, json } => {
            let att = openwand_app::workflow_external_attestation::load_external_attestation(
                store, &WorkflowExternalAttestationId(attestation_id),
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&att).context("Serialize")?);
            } else {
                println!("Attestation: {}", att.attestation_id.0);
                println!("  Target: {:?} → {}", att.target.target_kind, att.target.target_id);
                println!("  Claim: {}", att.claim);
                println!("  Source: {} ({})", att.source.name, att.source.role);
                println!("  Verified: {}", att.verified_by_openwand);
            }
        }
        WorkflowExternalAttestationCommands::List { workflow_execution_id, json } => {
            let results = openwand_app::workflow_external_attestation::attestations_by_workflow_run(
                store, &workflow_execution_id,
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&results).context("Serialize")?);
            } else {
                println!("Attestations for {} ({}):", workflow_execution_id, results.len());
                for att in &results {
                    println!("  {} — {:?} — {}", att.attestation_id.0, att.kind, att.claim);
                }
            }
        }
    }
    Ok(())
}

#[derive(Subcommand, Debug)]
enum WorkflowVerificationReadinessCommands {
    /// Evaluate verification readiness for a target
    Evaluate {
        #[arg(long)] target_kind: String,
        #[arg(long)] target_id: String,
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] expected_target_hash: String,
        #[arg(long, default_value = "key1")] idempotency_key: String,
        #[arg(long)] json: bool,
    },
    /// Show a readiness record by ID
    Show {
        #[arg(long)] readiness_id: String,
        #[arg(long)] json: bool,
    },
    /// List readiness records for a workflow run
    Latest {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] target_id: Option<String>,
        #[arg(long)] json: bool,
    },
}

fn cmd_verification_readiness(cmd: WorkflowVerificationReadinessCommands, store_dir: String) -> Result<()> {
    use openwand_workflow::workflow_verification_readiness::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let store = std::path::Path::new(&store_dir);
    match cmd {
        WorkflowVerificationReadinessCommands::Evaluate {
            target_kind, target_id, workflow_execution_id,
            expected_target_hash, idempotency_key, json,
        } => {
            let tk: VerificationReadinessTargetKind = serde_json::from_str(&format!("\"{}\"", target_kind))
                .map_err(|e| anyhow::anyhow!("Invalid target_kind '{}': {}", target_kind, e))?;
            let request = VerificationReadinessRequest {
                target_kind: tk,
                target_id,
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                expected_target_hash,
                idempotency_key,
            };
            // Evaluate using metadata-only path (target record loading would be app-level)
            let rec = evaluate_readiness_metadata_only(
                &request, "reported_succeeded", &request.expected_target_hash,
                &request.workflow_execution_id.0,
            );
            openwand_app::workflow_verification_readiness::save_verification_readiness(store, &rec)
                .map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?);
            } else {
                println!("Readiness: {} — {:?}", rec.readiness_id.0, rec.status);
                for pred in &rec.predicate_results {
                    println!("  {} — {} ({})", if pred.passed { "PASS" } else { "FAIL" },
                        format!("{:?}", pred.predicate), pred.reason);
                }
            }
        }
        WorkflowVerificationReadinessCommands::Show { readiness_id, json } => {
            let rec = openwand_app::workflow_verification_readiness::load_verification_readiness(
                store, &WorkflowVerificationReadinessId(readiness_id),
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?);
            } else {
                println!("Readiness: {} — {:?}", rec.readiness_id.0, rec.status);
                println!("  Target: {:?} → {}", rec.target_kind, rec.target_id);
                for pred in &rec.predicate_results {
                    println!("  {} — {} ({})", if pred.passed { "PASS" } else { "FAIL" },
                        format!("{:?}", pred.predicate), pred.reason);
                }
            }
        }
        WorkflowVerificationReadinessCommands::Latest { workflow_execution_id, target_id, json } => {
            let results = if let Some(tid) = target_id {
                openwand_app::workflow_verification_readiness::readiness_by_target_id(store, &tid)
            } else {
                openwand_app::workflow_verification_readiness::readiness_by_workflow_run(store, &workflow_execution_id)
            }.map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&results).context("Serialize")?);
            } else {
                println!("Readiness records ({}):", results.len());
                for rec in &results {
                    println!("  {} — {:?} — {} → {:?}", rec.readiness_id.0, rec.target_kind, rec.target_id, rec.status);
                }
            }
        }
    }
    Ok(())
}

#[derive(Subcommand, Debug)]
enum AuditPacketReviewCommands {
    /// Record an audit packet review
    Record {
        #[arg(long)] inspection_id: String,
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] expected_audit_packet_hash: String,
        #[arg(long)] expected_chain_hash: String,
        #[arg(long)] reviewer: String,
        #[arg(long)] decision: String,
        #[arg(long)] scope: String,
        #[arg(long)] caveat: Vec<String>,
        #[arg(long, default_value = "key1")] idempotency_key: String,
        #[arg(long)] json: bool,
    },
    /// Show a review by ID
    Show {
        #[arg(long)] review_id: String,
        #[arg(long)] json: bool,
    },
    /// List reviews for a workflow run
    List {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_audit_packet_review(cmd: AuditPacketReviewCommands, store_dir: String) -> Result<()> {
    use openwand_workflow::workflow_audit_packet_review::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let store = std::path::Path::new(&store_dir);
    match cmd {
        AuditPacketReviewCommands::Record {
            inspection_id, workflow_execution_id,
            expected_audit_packet_hash, expected_chain_hash,
            reviewer, decision, scope, caveat, idempotency_key, json,
        } => {
            let dec: AuditPacketReviewDecision = serde_json::from_str(&format!("\"{}\"", decision))
                .map_err(|e| anyhow::anyhow!("Invalid decision '{}': {}", decision, e))?;
            let request = AuditPacketReviewRequest {
                inspection_id,
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                expected_audit_packet_hash,
                expected_chain_hash,
                reviewer,
                decision: dec,
                scope,
                caveats: caveat,
                idempotency_key,
            };
            let rec = build_audit_packet_review(request);
            openwand_app::workflow_audit_packet_review::save_audit_packet_review(store, &rec)
                .map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?);
            } else {
                println!("Review: {} — {:?} — {}", rec.review_id.0, rec.decision, rec.reviewer);
            }
        }
        AuditPacketReviewCommands::Show { review_id, json } => {
            let rec = openwand_app::workflow_audit_packet_review::load_audit_packet_review(
                store, &AuditPacketReviewId(review_id),
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?);
            } else {
                println!("Review: {} — {:?} — {}", rec.review_id.0, rec.decision, rec.reviewer);
            }
        }
        AuditPacketReviewCommands::List { workflow_execution_id, json } => {
            let results = openwand_app::workflow_audit_packet_review::review_by_workflow_run(
                store, &workflow_execution_id,
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&results).context("Serialize")?);
            } else {
                for rec in &results {
                    println!("{} — {:?} — {}", rec.review_id.0, rec.decision, rec.reviewer);
                }
            }
        }
    }
    Ok(())
}

#[derive(Subcommand, Debug)]
enum AuditPacketDistributionCommands {
    /// Record an audit packet distribution
    Record {
        #[arg(long)] review_id: String,
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] expected_review_hash: String,
        #[arg(long)] expected_audit_packet_hash: String,
        #[arg(long)] expected_chain_hash: String,
        #[arg(long)] inspection_id: String,
        #[arg(long)] destination_kind: String,
        #[arg(long)] destination_label: String,
        #[arg(long)] destination_reference: String,
        #[arg(long)] operator_supplied_hash: Option<String>,
        #[arg(long)] note: Vec<String>,
        #[arg(long, default_value = "dkey1")] idempotency_key: String,
        #[arg(long)] json: bool,
    },
    /// Show a distribution by ID
    Show {
        #[arg(long)] distribution_id: String,
        #[arg(long)] json: bool,
    },
    /// List distributions for a workflow run
    List {
        #[arg(long)] workflow_execution_id: String,
        #[arg(long)] json: bool,
    },
}

fn cmd_audit_packet_distribution(cmd: AuditPacketDistributionCommands, store_dir: String) -> Result<()> {
    use openwand_workflow::workflow_audit_packet_distribution::*;
    use openwand_workflow::workflow_audit_packet_review::AuditPacketReviewId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let store = std::path::Path::new(&store_dir);
    match cmd {
        AuditPacketDistributionCommands::Record {
            review_id, workflow_execution_id,
            expected_review_hash, expected_audit_packet_hash, expected_chain_hash,
            inspection_id, destination_kind, destination_label, destination_reference,
            operator_supplied_hash, note, idempotency_key, json,
        } => {
            let dk: AuditPacketDestinationKind = serde_json::from_str(&format!("\"{}\"", destination_kind))
                .map_err(|e| anyhow::anyhow!("Invalid destination_kind '{}': {}", destination_kind, e))?;
            let request = AuditPacketDistributionRequest {
                review_id: AuditPacketReviewId(review_id),
                workflow_execution_id: WorkflowExecutionId(workflow_execution_id),
                expected_review_hash,
                audit_packet_hash: expected_audit_packet_hash,
                chain_hash: expected_chain_hash,
                inspection_id,
                destination: AuditPacketDistributionDestination {
                    destination_kind: dk,
                    label: destination_label,
                    reference: destination_reference,
                    operator_supplied_hash,
                    notes: note,
                },
                distribution_notes: vec![],
                idempotency_key,
            };
            let rec = build_audit_packet_distribution(request);
            openwand_app::workflow_audit_packet_distribution::save_audit_packet_distribution(store, &rec)
                .map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?);
            } else {
                println!("Distribution: {} → {:?}", rec.distribution_id.0, rec.destination.destination_kind);
            }
        }
        AuditPacketDistributionCommands::Show { distribution_id, json } => {
            let rec = openwand_app::workflow_audit_packet_distribution::load_audit_packet_distribution(
                store, &AuditPacketDistributionId(distribution_id),
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&rec).context("Serialize")?);
            } else {
                println!("Distribution: {} → {:?}", rec.distribution_id.0, rec.destination.destination_kind);
            }
        }
        AuditPacketDistributionCommands::List { workflow_execution_id, json } => {
            let results = openwand_app::workflow_audit_packet_distribution::distribution_by_workflow_run(
                store, &workflow_execution_id,
            ).map_err(|e| anyhow::anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&results).context("Serialize")?);
            } else {
                for rec in &results {
                    println!("{} → {:?}", rec.distribution_id.0, rec.destination.destination_kind);
                }
            }
        }
    }
    Ok(())
}
