#[derive(clap::Subcommand)]
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
