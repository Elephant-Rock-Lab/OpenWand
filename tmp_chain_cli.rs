    /// Evidence chain inspector commands
    #[command(name = "workflow-evidence-chain")]
    WorkflowEvidenceChain {
        chain_cmd: WorkflowEvidenceChainCommands,
        #[arg(long, default_value = "eval_reports")] output_dir: String,
    },

