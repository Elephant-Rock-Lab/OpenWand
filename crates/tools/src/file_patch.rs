
// ---- File Patch (plan/apply) ----
// Correction 4: Plan/apply in one tool with mode field.
// Correction 3: BLAKE3 for content hashing.

use crate::local::{extract_call_id, validate_write_path};
use crate::result::{ToolResult, ToolCallContext};
use crate::descriptor::{ToolDef, ToolAnnotations, ToolSource};
use crate::naming::canonical_local_tool_name;
use openwand_core::tool_vocab::ToolEffect;
use openwand_core::ToolCallId;

pub fn file_patch_descriptor() -> ToolDef {
    ToolDef {
        name: canonical_local_tool_name("file_patch"),
        display_name: Some("Patch File".into()),
        description: "Plan or apply a patch to a file. Plan validates without writing. Apply writes after approval.".into(),
        parameters_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Relative path to the file" },
                "mode": { "type": "string", "enum": ["plan", "apply"] },
                "line_number": { "type": "integer", "description": "1-based start line" },
                "old_lines": { "type": "array", "items": { "type": "string" } },
                "new_lines": { "type": "array", "items": { "type": "string" } },
                "plan_id": { "type": "string", "description": "From prior plan call" }
            },
            "required": ["path", "mode"]
        }),
        output_schema: None,
        source: ToolSource::Local,
        declared_effect: ToolEffect::Write,
        risk_hints: vec![],
        tags: vec!["local".into(), "file".into(), "write".into()],
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(false),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
    }
}

fn blake3_content_hash(content: &[u8]) -> String {
    blake3::hash(content).to_hex().to_string()
}

pub async fn file_patch_handler(args: serde_json::Value, ctx: ToolCallContext) -> ToolResult {
    let start = std::time::Instant::now();
    let call_id = extract_call_id(&args);
    let tool_name = "file_patch".to_string();
    let path_val = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return ToolResult::error(call_id, tool_name, "'path' is required".into(), start.elapsed().as_millis() as u64),
    };
    let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("plan");
    let full_path = match validate_write_path(path_val, &ctx.working_directory) {
        Ok(p) => p,
        Err(e) => return ToolResult::error(call_id, tool_name, e, start.elapsed().as_millis() as u64),
    };
    match mode {
        "plan" => file_patch_plan(call_id, tool_name, &full_path, path_val, &args, start).await,
        "apply" => file_patch_apply(call_id, tool_name, &full_path, path_val, &args, &ctx, start).await,
        _ => ToolResult::error(call_id, tool_name, format!("unknown mode '{}'", mode), start.elapsed().as_millis() as u64),
    }
}

async fn file_patch_plan(
    call_id: ToolCallId, tool_name: String, abs_path: &std::path::Path,
    rel_path: &str, args: &serde_json::Value, start: std::time::Instant,
) -> ToolResult {
    let line_number = match args.get("line_number").and_then(|v| v.as_u64()) {
        Some(n) => n as usize,
        None => return ToolResult::error(call_id, tool_name, "'line_number' required for plan".into(), start.elapsed().as_millis() as u64),
    };
    let old_lines: Vec<String> = args.get("old_lines").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    let new_lines: Vec<String> = args.get("new_lines").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    if old_lines.is_empty() && new_lines.is_empty() {
        return ToolResult::error(call_id, tool_name, "old_lines or new_lines required".into(), start.elapsed().as_millis() as u64);
    }
    let content = match tokio::fs::read_to_string(abs_path).await {
        Ok(c) => c,
        Err(e) => return ToolResult::error(call_id, tool_name, format!("read error: {}", e), start.elapsed().as_millis() as u64),
    };
    let preimage_hash = blake3_content_hash(content.as_bytes());
    let lines: Vec<&str> = content.lines().collect();
    if line_number == 0 || line_number > lines.len() + 1 {
        return ToolResult::error(call_id, tool_name, format!("line {} out of range ({} lines)", line_number, lines.len()), start.elapsed().as_millis() as u64);
    }
    let si = line_number - 1;
    for (i, expected) in old_lines.iter().enumerate() {
        let idx = si + i;
        if idx >= lines.len() || lines[idx] != expected {
            return ToolResult::error(call_id, tool_name, format!("preimage mismatch at line {}", idx + 1), start.elapsed().as_millis() as u64);
        }
    }
    let mut nc: Vec<String> = lines[..si].iter().map(|s| s.to_string()).collect();
    nc.extend(new_lines.iter().cloned());
    if si + old_lines.len() < lines.len() {
        nc.extend(lines[si + old_lines.len()..].iter().map(|s| s.to_string()));
    }
    let postimage_hash = blake3_content_hash(nc.join("\n").as_bytes());
    let plan_id = blake3_content_hash(format!("{}:{}:{}", preimage_hash, line_number, postimage_hash).as_bytes());
    let mut out = vec![
        format!("Patch plan for {}:", rel_path),
        format!("  Line {}: {} -> {} lines", line_number, old_lines.len(), new_lines.len()),
        format!("  Preimage: {}", preimage_hash),
        format!("  Postimage: {}", postimage_hash),
        format!("  Plan ID: {}", plan_id), String::new(),
    ];
    for l in &old_lines { out.push(format!("- {}", l)); }
    for l in &new_lines { out.push(format!("+ {}", l)); }
    ToolResult::success(call_id, tool_name, out.join("\n"), start.elapsed().as_millis() as u64)
}

async fn file_patch_apply(
    call_id: ToolCallId, tool_name: String, abs_path: &std::path::Path,
    rel_path: &str, args: &serde_json::Value, ctx: &ToolCallContext, start: std::time::Instant,
) -> ToolResult {
    let plan_id = match args.get("plan_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return ToolResult::error(call_id, tool_name, "'plan_id' required for apply".into(), start.elapsed().as_millis() as u64),
    };
    let content = match tokio::fs::read_to_string(abs_path).await {
        Ok(c) => c,
        Err(e) => return ToolResult::error(call_id, tool_name, format!("read error: {}", e), start.elapsed().as_millis() as u64),
    };
    let preimage_hash = blake3_content_hash(content.as_bytes());
    let rollback_dir = std::path::Path::new(&ctx.working_directory).join(".openwand").join("rollback");
    if let Err(e) = tokio::fs::create_dir_all(&rollback_dir).await {
        return ToolResult::error(call_id, tool_name, format!("rollback dir: {}", e), start.elapsed().as_millis() as u64);
    }
    let rollback_path = rollback_dir.join(format!("{}.bak", plan_id));
    if let Err(e) = tokio::fs::write(&rollback_path, &content).await {
        return ToolResult::error(call_id, tool_name, format!("rollback write: {}", e), start.elapsed().as_millis() as u64);
    }
    let line_number = match args.get("line_number").and_then(|v| v.as_u64()) {
        Some(n) => n as usize,
        None => return ToolResult::error(call_id, tool_name, "'line_number' required".into(), start.elapsed().as_millis() as u64),
    };
    let old_lines: Vec<String> = args.get("old_lines").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    let new_lines: Vec<String> = args.get("new_lines").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    let lines: Vec<&str> = content.lines().collect();
    let si = line_number - 1;
    for (i, expected) in old_lines.iter().enumerate() {
        let idx = si + i;
        if idx >= lines.len() || lines[idx] != expected {
            return ToolResult::error(call_id, tool_name, "preimage changed since plan".into(), start.elapsed().as_millis() as u64);
        }
    }
    let mut nc: Vec<String> = lines[..si].iter().map(|s| s.to_string()).collect();
    nc.extend(new_lines.iter().cloned());
    if si + old_lines.len() < lines.len() {
        nc.extend(lines[si + old_lines.len()..].iter().map(|s| s.to_string()));
    }
    let new_content = nc.join("\n");
    let postimage_hash = blake3_content_hash(new_content.as_bytes());
    if let Err(e) = tokio::fs::write(abs_path, &new_content).await {
        return ToolResult::error(call_id, tool_name, format!("write error: {}", e), start.elapsed().as_millis() as u64);
    }
    ToolResult::success(call_id, tool_name,
        format!("Patched {} (line {}): {} -> {} lines\nPreimage: {}\nPostimage: {}\nRollback: {}",
            rel_path, line_number, old_lines.len(), new_lines.len(), preimage_hash, postimage_hash, rollback_path.display()),
        start.elapsed().as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context(dir: &tempfile::TempDir) -> ToolCallContext {
        ToolCallContext {
            working_directory: dir.path().to_string_lossy().to_string(),
            session_id: openwand_core::SessionId::new(),
            cancellation: tokio_util::sync::CancellationToken::new(),
        }
    }

    #[tokio::test]
    async fn file_patch_plan_validates_preimage() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("test.txt"), "line1\nline2\nline3").await.unwrap();
        let ctx = test_context(&dir);
        let provider = crate::local::batch2_local_tools();
        let args = serde_json::json!({
            "_call_id": "tc_plan",
            "path": "test.txt",
            "mode": "plan",
            "line_number": 2,
            "old_lines": ["line2"],
            "new_lines": ["replaced"]
        });
        let result = provider.execute(&canonical_local_tool_name("file_patch"), args, ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.output.contains("Plan ID:"));
        assert!(result.output.contains("Preimage:"));
        assert!(result.output.contains("Postimage:"));
    }

    #[tokio::test]
    async fn file_patch_plan_rejects_preimage_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("test.txt"), "line1\nline2\nline3").await.unwrap();
        let ctx = test_context(&dir);
        let provider = crate::local::batch2_local_tools();
        let args = serde_json::json!({
            "_call_id": "tc_plan",
            "path": "test.txt",
            "mode": "plan",
            "line_number": 2,
            "old_lines": ["wrong_line"],
            "new_lines": ["replaced"]
        });
        let result = provider.execute(&canonical_local_tool_name("file_patch"), args, ctx).await.unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("preimage mismatch"));
    }

    #[tokio::test]
    async fn file_patch_apply_creates_rollback() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("test.txt"), "line1\nline2\nline3").await.unwrap();
        let ctx = test_context(&dir);
        let provider = crate::local::batch2_local_tools();
        let args = serde_json::json!({
            "_call_id": "tc_apply",
            "path": "test.txt",
            "mode": "apply",
            "plan_id": "test_plan_123",
            "line_number": 2,
            "old_lines": ["line2"],
            "new_lines": ["replaced"]
        });
        let result = provider.execute(&canonical_local_tool_name("file_patch"), args, ctx).await.unwrap();
        assert!(!result.is_error);
        assert!(result.output.contains("Patched test.txt"));
        assert!(result.output.contains("Rollback:"));

        // Verify file was patched
        let content = tokio::fs::read_to_string(dir.path().join("test.txt")).await.unwrap();
        assert!(content.contains("replaced"));
        assert!(!content.contains("line2"));

        // Verify rollback exists
        let rollback = dir.path().join(".openwand").join("rollback").join("test_plan_123.bak");
        assert!(rollback.exists());
        let rollback_content = tokio::fs::read_to_string(&rollback).await.unwrap();
        assert!(rollback_content.contains("line2"));
    }

    #[tokio::test]
    async fn file_patch_rejects_out_of_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = test_context(&dir);
        let provider = crate::local::batch2_local_tools();
        let args = serde_json::json!({
            "_call_id": "tc_oob",
            "path": "../etc/passwd",
            "mode": "plan",
            "line_number": 1,
            "old_lines": ["x"],
            "new_lines": ["y"]
        });
        let result = provider.execute(&canonical_local_tool_name("file_patch"), args, ctx).await.unwrap();
        assert!(result.is_error);
    }

    #[test]
    fn blake3_hash_is_deterministic() {
        let h1 = blake3_content_hash(b"hello");
        let h2 = blake3_content_hash(b"hello");
        assert_eq!(h1, h2);
        let h3 = blake3_content_hash(b"world");
        assert_ne!(h1, h3);
    }
}
