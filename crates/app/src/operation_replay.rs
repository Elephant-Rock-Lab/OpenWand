//! Operation replay: read-only correspondence verification.
use openwand_core::events::{OpenWandTraceEvent, ToolEvent, WorkflowEvent};
use openwand_store::StoredEvent;
use openwand_trace::entry::TraceEntry;

#[derive(Debug, Clone, PartialEq)]
pub enum DesktopOperation {
    WorkflowInitiation { workflow_execution_id: String },
    ApprovalResolution { approval_request_id: String, tool_call_id: Option<String> },
    EvidenceExport { workflow_execution_id: String, #[allow(dead_code)] artifact_path: Option<String>, #[allow(dead_code)] artifact_hash: Option<String> },
}
#[derive(Debug, Clone, PartialEq)]
pub enum ReplayCheck { ExpectedEventsPresent, OrderingValid, ApprovalMatchedByArid, ApprovalMatchedByToolCallId }
#[derive(Debug, Clone, PartialEq)]
pub enum ReplaySeverity { Error, Warning, Info }
#[derive(Debug, Clone, PartialEq)]
pub enum ReplayResult { Pass, Fail, Inconclusive, Unsupported }
#[derive(Debug, Clone)]
pub struct OperationReplayFinding { pub severity: ReplaySeverity, pub check: ReplayCheck, pub detail: String }
#[derive(Debug, Clone)]
pub struct OperationReplayReport { pub result: ReplayResult, pub operations_checked: usize, pub findings: Vec<OperationReplayFinding> }
impl OperationReplayReport { pub fn has_errors(&self) -> bool { self.findings.iter().any(|f| f.severity == ReplaySeverity::Error) } }
pub struct OperationReplayVerifier;

impl OperationReplayVerifier {
    pub fn verify(operations: &[DesktopOperation], entries: &[TraceEntry<StoredEvent>]) -> OperationReplayReport {
        if operations.is_empty() {
            return OperationReplayReport { result: ReplayResult::Pass, operations_checked: 0, findings: vec![] };
        }
        let mut all = Vec::new(); let mut hf=false; let mut hu=false; let mut hi=false;
        for op in operations {
            let (f, r) = Self::verify_single(op, entries);
            match r { ReplayResult::Fail=>hf=true, ReplayResult::Unsupported=>hu=true, ReplayResult::Inconclusive=>hi=true, _=>{} }
            all.extend(f);
        }
        let result = if hf { ReplayResult::Fail } else if hi { ReplayResult::Inconclusive } else if hu { ReplayResult::Unsupported } else { ReplayResult::Pass };
        OperationReplayReport { result, operations_checked: operations.len(), findings: all }
    }
    fn verify_single(op: &DesktopOperation, entries: &[TraceEntry<StoredEvent>]) -> (Vec<OperationReplayFinding>, ReplayResult) {
        match op {
            DesktopOperation::WorkflowInitiation { workflow_execution_id } => Self::verify_wf(workflow_execution_id, entries),
            DesktopOperation::ApprovalResolution { approval_request_id, tool_call_id } => Self::verify_appr(approval_request_id, tool_call_id.as_deref(), entries),
            DesktopOperation::EvidenceExport { workflow_execution_id, artifact_path, artifact_hash } => Self::verify_exp(workflow_execution_id, artifact_path.as_deref(), artifact_hash.as_deref(), entries),
        }
    }

    fn verify_wf(eid: &str, entries: &[TraceEntry<StoredEvent>]) -> (Vec<OperationReplayFinding>, ReplayResult) {
        let mut f = Vec::new();
        let wf: Vec<_> = entries.iter().filter(|e| e.event_kind.starts_with("workflow.")).collect();
        if wf.is_empty() {
            f.push(OperationReplayFinding { severity: ReplaySeverity::Info, check: ReplayCheck::ExpectedEventsPresent, detail: format!("No workflow.* events for execution {eid}. appends_trace=false.") });
            return (f, ReplayResult::Inconclusive);
        }
        let m: Vec<_> = wf.iter().filter(|e| { if let OpenWandTraceEvent::Workflow(w) = &e.event.0 { Self::wf_ref(w, eid) } else { false } }).collect();
        if m.is_empty() {
            f.push(OperationReplayFinding { severity: ReplaySeverity::Error, check: ReplayCheck::ExpectedEventsPresent, detail: format!("Workflow events exist but none reference {eid}.") });
            return (f, ReplayResult::Fail);
        }
        let s = m.iter().find_map(|e| (e.event_kind=="workflow.mod_started").then_some(e.global_sequence));
        let c = m.iter().find_map(|e| (e.event_kind=="workflow.mod_completed").then_some(e.global_sequence));
        if let (Some(ss), Some(cc)) = (s, c) && cc <= ss {
            f.push(OperationReplayFinding { severity: ReplaySeverity::Error, check: ReplayCheck::OrderingValid, detail: format!("mod_completed seq={cc} before mod_started seq={ss}") });
            return (f, ReplayResult::Fail);
        }
        f.push(OperationReplayFinding { severity: ReplaySeverity::Info, check: ReplayCheck::ExpectedEventsPresent, detail: format!("{} workflow events for {eid}, valid ordering.", m.len()) });
        (f, ReplayResult::Pass)
    }

    fn verify_appr(arid: &str, tcid: Option<&str>, entries: &[TraceEntry<StoredEvent>]) -> (Vec<OperationReplayFinding>, ReplayResult) {
        let mut f = Vec::new();
        let am = entries.iter().find(|e| match &e.event.0 {
            OpenWandTraceEvent::Tool(ToolEvent::Resumed { approval_request_id: Some(a), .. })
            | OpenWandTraceEvent::Tool(ToolEvent::Denied { approval_request_id: Some(a), .. }) => a.0 == arid,
            _ => false,
        });
        if let Some(re) = am {
            let tc = match &re.event.0 {
                OpenWandTraceEvent::Tool(ToolEvent::Resumed { tool_call_id, .. }) | OpenWandTraceEvent::Tool(ToolEvent::Denied { tool_call_id, .. }) => tool_call_id.0.clone(),
                _ => String::new(),
            };
            let susp = entries.iter().find(|e| match &e.event.0 {
                OpenWandTraceEvent::Tool(ToolEvent::Suspended { tool_call_id: s, .. }) => s.0 == tc, _ => false,
            });
            if let Some(s) = susp {
                if s.global_sequence >= re.global_sequence {
                    f.push(OperationReplayFinding { severity: ReplaySeverity::Error, check: ReplayCheck::OrderingValid, detail: format!("suspended seq={} after resolution seq={}", s.global_sequence, re.global_sequence) });
                    return (f, ReplayResult::Fail);
                }
                f.push(OperationReplayFinding { severity: ReplaySeverity::Info, check: ReplayCheck::ApprovalMatchedByArid, detail: format!("Approval {arid} matched by ARID. Valid ({}->{}).", s.global_sequence, re.global_sequence) });
                return (f, ReplayResult::Pass);
            }
            f.push(OperationReplayFinding { severity: ReplaySeverity::Warning, check: ReplayCheck::OrderingValid, detail: format!("ARID match but no preceding suspended for {tc}.") });
            return (f, ReplayResult::Inconclusive);
        }
        if let Some(tc) = tcid {
            let susp = entries.iter().find(|e| match &e.event.0 {
                OpenWandTraceEvent::Tool(ToolEvent::Suspended { tool_call_id: s, .. }) => s.0 == tc, _ => false,
            });
            let resol = entries.iter().find(|e| match &e.event.0 {
                OpenWandTraceEvent::Tool(ToolEvent::Resumed { tool_call_id: r, .. }) | OpenWandTraceEvent::Tool(ToolEvent::Denied { tool_call_id: r, .. }) => r.0 == tc, _ => false,
            });
            if let (Some(s), Some(r)) = (susp, resol) {
                if s.global_sequence >= r.global_sequence {
                    f.push(OperationReplayFinding { severity: ReplaySeverity::Error, check: ReplayCheck::OrderingValid, detail: format!("suspended seq={} after resolution seq={}", s.global_sequence, r.global_sequence) });
                    return (f, ReplayResult::Fail);
                }
                f.push(OperationReplayFinding { severity: ReplaySeverity::Warning, check: ReplayCheck::ApprovalMatchedByToolCallId, detail: format!("Matched by tool_call_id={tc}; canonical ARID not in trace. Valid ({}->{}).", s.global_sequence, r.global_sequence) });
                return (f, ReplayResult::Pass);
            }
            if susp.is_some() != resol.is_some() {
                f.push(OperationReplayFinding { severity: ReplaySeverity::Error, check: ReplayCheck::ExpectedEventsPresent, detail: format!("tool_call_id={tc} has {} but not {}.", if susp.is_some() {"suspended"} else {"resolved"}, if susp.is_some() {"resolved"} else {"suspended"}) });
                return (f, ReplayResult::Fail);
            }
        }
        f.push(OperationReplayFinding { severity: ReplaySeverity::Error, check: ReplayCheck::ExpectedEventsPresent, detail: format!("No evidence for arid={arid}{}", tcid.map(|t| format!(" or tcid={t}")).unwrap_or_default()) });
        (f, ReplayResult::Fail)
    }

    fn verify_exp(eid: &str, ap: Option<&str>, _ah: Option<&str>, entries: &[TraceEntry<StoredEvent>]) -> (Vec<OperationReplayFinding>, ReplayResult) {
        let mut f = Vec::new();
        let ae: Vec<_> = entries.iter().filter(|e| e.event_kind == "artifact.generated").collect();
        if ae.is_empty() {
            f.push(OperationReplayFinding { severity: ReplaySeverity::Info, check: ReplayCheck::ExpectedEventsPresent, detail: format!("No artifact.generated for export of {eid}. Export does not emit trace.") });
            return (f, ReplayResult::Unsupported);
        }
        let m = ae.iter().any(|e| {
            if let OpenWandTraceEvent::Artifact(openwand_core::events::ArtifactEvent::Generated { paths, artifact_kind, .. }) = &e.event.0 {
                let km = artifact_kind.contains("audit") || artifact_kind.contains("evidence") || artifact_kind.contains("export");
                let pm = ap.map(|x| paths.iter().any(|p| p == x)).unwrap_or(true);
                km && pm
            } else { false }
        });
        if m {
            f.push(OperationReplayFinding { severity: ReplaySeverity::Info, check: ReplayCheck::ExpectedEventsPresent, detail: format!("Found artifact.generated matching export for {eid}.") });
            return (f, ReplayResult::Pass);
        }
        f.push(OperationReplayFinding { severity: ReplaySeverity::Warning, check: ReplayCheck::ExpectedEventsPresent, detail: format!("artifact.generated events exist but none match export for {eid}.") });
        (f, ReplayResult::Inconclusive)
    }
    fn wf_ref(wf: &WorkflowEvent, eid: &str) -> bool {
        match wf {
            WorkflowEvent::StateChanged { mod_id, .. } => mod_id.as_deref() == Some(eid),
            WorkflowEvent::GatePassed { mod_id, .. } | WorkflowEvent::GateFailed { mod_id, .. } | WorkflowEvent::ActionExecuted { mod_id, .. } | WorkflowEvent::ModStarted { mod_id, .. } | WorkflowEvent::ModCompleted { mod_id, .. } => mod_id == eid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_core::events::*;
    use openwand_core::ids::{ApprovalRequestId, ToolCallId};
    use openwand_trace::actor::Actor;
    use openwand_trace::ids::TraceId;
    use openwand_trace::stream::{EntryHash, TraceStreamId, TraceStreamScope};

    fn mk(gs: u64, ss: u64, ev: OpenWandTraceEvent, ek: &str) -> TraceEntry<StoredEvent> {
        TraceEntry {
            id: TraceId::new(), stream_id: TraceStreamId { scope: TraceStreamScope::Session, id: "t".into() },
            stream_sequence: ss, global_sequence: gs, occurred_at: chrono::Utc::now(), actor: Actor::User,
            event: StoredEvent::from(ev), event_kind: ek.into(), event_schema_version: 1, trace_schema_version: 1,
            prev_hash: None, entry_hash: EntryHash(format!("h{gs}")),
        }
    }

    #[test] fn no_exec_calls() {
        let s = include_str!("operation_replay.rs");
        let impl_only = s.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!impl_only.contains("export_audit_packet")); assert!(!impl_only.contains("request_workflow_run"));
        assert!(!impl_only.contains("submit_approval_resolution")); assert!(!impl_only.contains("resolve_approval"));
        assert!(!impl_only.contains("ToolExecutor")); assert!(!impl_only.contains("advance_stages")); assert!(!impl_only.contains("save_workflow_run"));
    }
    #[test] fn is_read_only() {
        let s = include_str!("operation_replay.rs");
        let impl_only = s.split("#[cfg(test)]").next().unwrap_or("");
        assert!(!impl_only.contains(".append(")); assert!(!impl_only.contains("fn repair")); assert!(!impl_only.contains("fn execute")); assert!(!impl_only.contains("std::fs::write"));
    }
    #[test] fn empty_ops_pass() {
        let r = OperationReplayVerifier::verify(&[], &[]);
        assert_eq!(r.result, ReplayResult::Pass); assert_eq!(r.operations_checked, 0);
    }
    #[test] fn wf_no_events_inconclusive() {
        let e = vec![mk(1,1, OpenWandTraceEvent::Session(SessionEvent::Started { session_id: openwand_core::SessionId::new(), mode: openwand_core::mode::InteractionMode::Direct }), "session.started")];
        let o = vec![DesktopOperation::WorkflowInitiation { workflow_execution_id: "wfx".into() }];
        assert_eq!(OperationReplayVerifier::verify(&o, &e).result, ReplayResult::Inconclusive);
    }
    #[test] fn wf_matching_pass() {
        let id = "wfx"; let e = vec![
            mk(1,1, OpenWandTraceEvent::Workflow(WorkflowEvent::ModStarted { mod_id: id.into(), mod_name: "m".into() }), "workflow.mod_started"),
            mk(2,2, OpenWandTraceEvent::Workflow(WorkflowEvent::ModCompleted { mod_id: id.into(), mod_name: "m".into(), outcome: "done".into() }), "workflow.mod_completed"),
        ];
        let o = vec![DesktopOperation::WorkflowInitiation { workflow_execution_id: id.into() }];
        assert_eq!(OperationReplayVerifier::verify(&o, &e).result, ReplayResult::Pass);
    }
    #[test] fn wf_unrelated_fails() {
        let e = vec![mk(1,1, OpenWandTraceEvent::Workflow(WorkflowEvent::ModStarted { mod_id: "other".into(), mod_name: "m".into() }), "workflow.mod_started")];
        let o = vec![DesktopOperation::WorkflowInitiation { workflow_execution_id: "wfx".into() }];
        assert_eq!(OperationReplayVerifier::verify(&o, &e).result, ReplayResult::Fail);
    }
    #[test] fn wf_reversed_fails() {
        let id = "wfx"; let e = vec![
            mk(1,1, OpenWandTraceEvent::Workflow(WorkflowEvent::ModCompleted { mod_id: id.into(), mod_name: "m".into(), outcome: "done".into() }), "workflow.mod_completed"),
            mk(2,2, OpenWandTraceEvent::Workflow(WorkflowEvent::ModStarted { mod_id: id.into(), mod_name: "m".into() }), "workflow.mod_started"),
        ];
        let o = vec![DesktopOperation::WorkflowInitiation { workflow_execution_id: id.into() }];
        assert_eq!(OperationReplayVerifier::verify(&o, &e).result, ReplayResult::Fail);
    }
    #[test] fn appr_arid_pass() {
        let a = ApprovalRequestId::new(); let t = ToolCallId::new();
        let as_ = a.0.clone(); let ts_ = t.0.clone();
        let e = vec![
            mk(1,1, OpenWandTraceEvent::Tool(ToolEvent::Suspended { tool_call_id: t, tool_name: "s".into(), reason: "n".into(), approval_context: None }), "tool.suspended"),
            mk(2,2, OpenWandTraceEvent::Tool(ToolEvent::Resumed { tool_call_id: ToolCallId(ts_.clone()), tool_name: "s".into(), resolution: "approved".into(), approval_request_id: Some(ApprovalRequestId(as_.clone())) }), "tool.resumed"),
        ];
        let o = vec![DesktopOperation::ApprovalResolution { approval_request_id: as_, tool_call_id: Some(ts_) }];
        let r = OperationReplayVerifier::verify(&o, &e);
        assert_eq!(r.result, ReplayResult::Pass);
        assert!(r.findings.iter().any(|f| f.check == ReplayCheck::ApprovalMatchedByArid));
    }
    #[test] fn appr_fallback_tc_warns() {
        let t = ToolCallId::new(); let ts_ = t.0.clone();
        let e = vec![
            mk(1,1, OpenWandTraceEvent::Tool(ToolEvent::Suspended { tool_call_id: t, tool_name: "s".into(), reason: "n".into(), approval_context: None }), "tool.suspended"),
            mk(2,2, OpenWandTraceEvent::Tool(ToolEvent::Denied { tool_call_id: ToolCallId(ts_.clone()), tool_name: "s".into(), approval_request_id: None, reason: Some("r".into()) }), "tool.denied"),
        ];
        let o = vec![DesktopOperation::ApprovalResolution { approval_request_id: "unk".into(), tool_call_id: Some(ts_) }];
        let r = OperationReplayVerifier::verify(&o, &e);
        assert_eq!(r.result, ReplayResult::Pass);
        assert!(r.findings.iter().any(|f| f.check == ReplayCheck::ApprovalMatchedByToolCallId && f.severity == ReplaySeverity::Warning));
    }
    #[test] fn appr_no_match_fails() {
        let e = vec![mk(1,1, OpenWandTraceEvent::Tool(ToolEvent::Called { tool_call_id: ToolCallId::new(), tool_name: "r".into(), args_hash: "a".into(), invoker: openwand_core::tool_vocab::ToolInvoker::Llm }), "tool.called")];
        let o = vec![DesktopOperation::ApprovalResolution { approval_request_id: "x".into(), tool_call_id: Some("y".into()) }];
        assert_eq!(OperationReplayVerifier::verify(&o, &e).result, ReplayResult::Fail);
    }
    #[test] fn appr_orphaned_fails() {
        let t = ToolCallId::new(); let ts_ = t.0.clone();
        let e = vec![mk(1,1, OpenWandTraceEvent::Tool(ToolEvent::Suspended { tool_call_id: t, tool_name: "s".into(), reason: "n".into(), approval_context: None }), "tool.suspended")];
        let o = vec![DesktopOperation::ApprovalResolution { approval_request_id: "s".into(), tool_call_id: Some(ts_) }];
        assert_eq!(OperationReplayVerifier::verify(&o, &e).result, ReplayResult::Fail);
    }
    #[test] fn appr_reversed_fails() {
        let t = ToolCallId::new(); let ts_ = t.0.clone();
        let e = vec![
            mk(1,1, OpenWandTraceEvent::Tool(ToolEvent::Resumed { tool_call_id: ToolCallId(ts_.clone()), tool_name: "s".into(), resolution: "a".into(), approval_request_id: Some(ApprovalRequestId("a1".into())) }), "tool.resumed"),
            mk(2,2, OpenWandTraceEvent::Tool(ToolEvent::Suspended { tool_call_id: ToolCallId(ts_.clone()), tool_name: "s".into(), reason: "n".into(), approval_context: None }), "tool.suspended"),
        ];
        let o = vec![DesktopOperation::ApprovalResolution { approval_request_id: "a1".into(), tool_call_id: Some(ts_) }];
        assert_eq!(OperationReplayVerifier::verify(&o, &e).result, ReplayResult::Fail);
    }
    #[test] fn exp_no_events_unsupported() {
        let e = vec![mk(1,1, OpenWandTraceEvent::Session(SessionEvent::Started { session_id: openwand_core::SessionId::new(), mode: openwand_core::mode::InteractionMode::Direct }), "session.started")];
        let o = vec![DesktopOperation::EvidenceExport { workflow_execution_id: "w".into(), artifact_path: None, artifact_hash: None }];
        assert_eq!(OperationReplayVerifier::verify(&o, &e).result, ReplayResult::Unsupported);
    }
    #[test] fn exp_matching_pass() {
        let e = vec![mk(1,1, OpenWandTraceEvent::Artifact(ArtifactEvent::Generated { paths: vec!["e.zip".into()], artifact_kind: "audit_packet".into(), accuracy: openwand_core::snapshots::AccuracyRecordSnapshot { commit_hash: None, file_coverage: 1.0, sensitivity: "test".into() } }), "artifact.generated")];
        let o = vec![DesktopOperation::EvidenceExport { workflow_execution_id: "w".into(), artifact_path: Some("e.zip".into()), artifact_hash: None }];
        assert_eq!(OperationReplayVerifier::verify(&o, &e).result, ReplayResult::Pass);
    }
    #[test] fn exp_non_matching_inconclusive() {
        let e = vec![mk(1,1, OpenWandTraceEvent::Artifact(ArtifactEvent::Generated { paths: vec!["o.zip".into()], artifact_kind: "build".into(), accuracy: openwand_core::snapshots::AccuracyRecordSnapshot { commit_hash: None, file_coverage: 1.0, sensitivity: "test".into() } }), "artifact.generated")];
        let o = vec![DesktopOperation::EvidenceExport { workflow_execution_id: "w".into(), artifact_path: Some("e.zip".into()), artifact_hash: None }];
        assert_eq!(OperationReplayVerifier::verify(&o, &e).result, ReplayResult::Inconclusive);
    }
    #[test] fn mixed_ops_mixed_results() {
        let a = ApprovalRequestId::new(); let t = ToolCallId::new();
        let as_ = a.0.clone(); let ts_ = t.0.clone();
        let e = vec![
            mk(1,1, OpenWandTraceEvent::Tool(ToolEvent::Suspended { tool_call_id: t, tool_name: "s".into(), reason: "n".into(), approval_context: None }), "tool.suspended"),
            mk(2,2, OpenWandTraceEvent::Tool(ToolEvent::Resumed { tool_call_id: ToolCallId(ts_.clone()), tool_name: "s".into(), resolution: "a".into(), approval_request_id: Some(ApprovalRequestId(as_.clone())) }), "tool.resumed"),
        ];
        let o = vec![
            DesktopOperation::ApprovalResolution { approval_request_id: as_, tool_call_id: Some(ts_) },
            DesktopOperation::WorkflowInitiation { workflow_execution_id: "w".into() },
            DesktopOperation::EvidenceExport { workflow_execution_id: "w".into(), artifact_path: None, artifact_hash: None },
        ];
        let r = OperationReplayVerifier::verify(&o, &e);
        assert_eq!(r.result, ReplayResult::Inconclusive);
        assert_eq!(r.operations_checked, 3);
    }
}
