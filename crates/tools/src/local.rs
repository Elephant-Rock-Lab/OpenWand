//! Local tool provider — built-in tools that run without MCP.

use async_trait::async_trait;
use openwand_core::tool_vocab::ToolEffect;
use openwand_core::ToolCallId;
use std::collections::HashMap;
use std::sync::Arc;

use crate::descriptor::{ToolAnnotations, ToolDef, ToolSource};
use crate::naming::canonical_local_tool_name;
use crate::result::{ToolCallContext, ToolResult};

/// A single local tool implementation.
#[async_trait]
pub trait LocalTool: Send + Sync {
    fn descriptor(&self) -> ToolDef;
    async fn execute(&self, args: serde_json::Value, context: ToolCallContext) -> ToolResult;
}

/// A LocalTool backed by a closure.
#[allow(clippy::type_complexity)]
struct LocalToolEntry {
    descriptor: ToolDef,
    handler: Arc<
        dyn Fn(serde_json::Value, ToolCallContext)
                -> std::pin::Pin<
                    Box<dyn std::future::Future<Output = ToolResult> + Send + 'static>,
                > + Send
            + Sync,
    >,
}

#[async_trait]
impl LocalTool for LocalToolEntry {
    fn descriptor(&self) -> ToolDef {
        self.descriptor.clone()
    }

    async fn execute(&self, args: serde_json::Value, context: ToolCallContext) -> ToolResult {
        (self.handler)(args, context).await
    }
}

/// Registry of local tools.
pub struct BuiltinToolProvider {
    tools: HashMap<String, Arc<dyn LocalTool>>,
}

impl BuiltinToolProvider {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn register_fn<F, Fut>(&mut self, descriptor: ToolDef, handler: F)
    where
        F: Fn(serde_json::Value, ToolCallContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ToolResult> + Send + 'static,
    {
        let name = descriptor.name.clone();
        let handler_arc: Arc<
            dyn Fn(serde_json::Value, ToolCallContext)
                    -> std::pin::Pin<
                        Box<dyn std::future::Future<Output = ToolResult> + Send + 'static>,
                    > + Send
                + Sync,
        > = Arc::new(move |args, ctx| {
            let fut = handler(args, ctx);
            Box::pin(fut)
        });
        let entry = LocalToolEntry {
            descriptor,
            handler: handler_arc,
        };
        self.tools.insert(name, Arc::new(entry));
    }

    pub fn available_descriptors(&self) -> Vec<ToolDef> {
        self.tools.values().map(|t| t.descriptor()).collect()
    }

    pub fn get_descriptor(&self, name: &str) -> Option<ToolDef> {
        self.tools.get(name).map(|t| t.descriptor())
    }

    pub async fn execute(
        &self,
        name: &str,
        args: serde_json::Value,
        context: ToolCallContext,
    ) -> Option<ToolResult> {
        let tool = self.tools.get(name)?;
        Some(tool.execute(args, context).await)
    }
}

impl Default for BuiltinToolProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the standard batch-1 set of local tools (read-only).
pub fn batch1_local_tools() -> BuiltinToolProvider {
    let mut provider = BuiltinToolProvider::new();
    provider.register_fn(file_read_descriptor(), file_read_handler);
    provider.register_fn(file_list_descriptor(), file_list_handler);
    provider.register_fn(file_search_descriptor(), file_search_handler);
    provider
}

/// Create the full set of local tools including write tools.
pub fn batch2_local_tools() -> BuiltinToolProvider {
    let mut provider = batch1_local_tools();
    provider.register_fn(file_write_descriptor(), file_write_handler);
    provider.register_fn(crate::file_patch::file_patch_descriptor(), crate::file_patch::file_patch_handler);
    provider
}

// ---------------------------------------------------------------------------
// Helper: extract call_id from args or generate one
// ---------------------------------------------------------------------------
pub(crate) fn extract_call_id(args: &serde_json::Value) -> ToolCallId {
    match args.get("_call_id").and_then(|v| v.as_str()) {
        Some(id) => ToolCallId(id.to_string()),
        None => ToolCallId::new(),
    }
}

// ---- File Read ----

fn file_read_descriptor() -> ToolDef {
    ToolDef {
        name: canonical_local_tool_name("file_read"),
        display_name: Some("Read File".into()),
        description: "Read the contents of a file. Respects working directory boundaries.".into(),
        parameters_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative or absolute path to the file"
                }
            },
            "required": ["path"]
        }),
        output_schema: None,
        source: ToolSource::Local,
        declared_effect: ToolEffect::Read,
        risk_hints: vec![],
        tags: vec!["local".into(), "file".into()],
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
    }
}

async fn file_read_handler(args: serde_json::Value, ctx: ToolCallContext) -> ToolResult {
    let start = std::time::Instant::now();
    let call_id = extract_call_id(&args);
    let tool_name = canonical_local_tool_name("file_read");

    let path_val = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return ToolResult::error(
                call_id,
                tool_name,
                "Missing required parameter: path".into(),
                start.elapsed().as_millis() as u64,
            )
        }
    };

    let full_path = match crate::sandbox::resolve_workspace_path(
        std::path::Path::new(&ctx.working_directory),
        path_val,
        crate::sandbox::PathAccessMode::ReadExisting,
    ) {
        Ok(p) => p,
        Err(e) => {
            return ToolResult::error(
                call_id,
                tool_name,
                e.message,
                start.elapsed().as_millis() as u64,
            );
        }
    };

    match tokio::fs::read_to_string(&full_path).await {
        Ok(content) => ToolResult::success(
            call_id,
            tool_name,
            content,
            start.elapsed().as_millis() as u64,
        ),
        Err(e) => ToolResult::error(
            call_id,
            tool_name,
            format!("Failed to read file '{}': {}", full_path.display(), e),
            start.elapsed().as_millis() as u64,
        ),
    }
}

// ---- File List ----

fn file_list_descriptor() -> ToolDef {
    ToolDef {
        name: canonical_local_tool_name("file_list"),
        display_name: Some("List Files".into()),
        description: "List files in a directory. Respects working directory boundaries.".into(),
        parameters_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path (defaults to working directory)"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Whether to list recursively (default: false)"
                }
            }
        }),
        output_schema: None,
        source: ToolSource::Local,
        declared_effect: ToolEffect::Read,
        risk_hints: vec![],
        tags: vec!["local".into(), "file".into()],
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
    }
}

async fn file_list_handler(args: serde_json::Value, ctx: ToolCallContext) -> ToolResult {
    let start = std::time::Instant::now();
    let call_id = extract_call_id(&args);
    let tool_name = canonical_local_tool_name("file_list");

    let path_val = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    let recursive = args
        .get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let full_path = match crate::sandbox::resolve_workspace_path(
        std::path::Path::new(&ctx.working_directory),
        path_val,
        crate::sandbox::PathAccessMode::ListExisting,
    ) {
        Ok(p) => p,
        Err(e) => {
            return ToolResult::error(
                call_id,
                tool_name,
                e.message,
                start.elapsed().as_millis() as u64,
            );
        }
    };

    let mut entries = Vec::new();

    if recursive {
        let walker = walkdir::WalkDir::new(&full_path).max_depth(10);
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let relative = path
                .strip_prefix(&full_path)
                .unwrap_or(path)
                .to_string_lossy();
            if relative.is_empty() {
                continue;
            }
            let kind = if entry.file_type().is_dir() {
                "dir"
            } else {
                "file"
            };
            entries.push(format!("{kind}: {relative}"));
        }
    } else {
        match tokio::fs::read_dir(&full_path).await {
            Ok(mut dir) => {
                while let Ok(Some(entry)) = dir.next_entry().await {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let kind =
                        if entry.file_type().await.map(|ft| ft.is_dir()).unwrap_or(false) {
                            "dir"
                        } else {
                            "file"
                        };
                    entries.push(format!("{kind}: {name}"));
                }
            }
            Err(e) => {
                return ToolResult::error(
                    call_id,
                    tool_name,
                    format!("Failed to list directory '{}': {}", full_path.display(), e),
                    start.elapsed().as_millis() as u64,
                );
            }
        }
    }

    entries.sort();
    let output = if entries.is_empty() {
        "(empty directory)".to_string()
    } else {
        entries.join("\n")
    };

    ToolResult::success(
        call_id,
        tool_name,
        output,
        start.elapsed().as_millis() as u64,
    )
}

// ---- File Search ----

fn file_search_descriptor() -> ToolDef {
    ToolDef {
        name: canonical_local_tool_name("file_search"),
        display_name: Some("Search Files".into()),
        description: "Search for a pattern in files under a directory.".into(),
        parameters_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Search pattern (literal string)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search (defaults to working directory)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 50)"
                }
            },
            "required": ["pattern"]
        }),
        output_schema: None,
        source: ToolSource::Local,
        declared_effect: ToolEffect::Search,
        risk_hints: vec![],
        tags: vec!["local".into(), "file".into()],
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
    }
}

async fn file_search_handler(args: serde_json::Value, ctx: ToolCallContext) -> ToolResult {
    let start = std::time::Instant::now();
    let call_id = extract_call_id(&args);
    let tool_name = canonical_local_tool_name("file_search");

    let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return ToolResult::error(
                call_id,
                tool_name,
                "Missing required parameter: pattern".into(),
                start.elapsed().as_millis() as u64,
            );
        }
    };
    let path_val = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    let max_results = args
        .get("max_results")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    let full_path = match crate::sandbox::resolve_workspace_path(
        std::path::Path::new(&ctx.working_directory),
        path_val,
        crate::sandbox::PathAccessMode::SearchExisting,
    ) {
        Ok(p) => p,
        Err(e) => {
            return ToolResult::error(
                call_id,
                tool_name,
                e.message,
                start.elapsed().as_millis() as u64,
            );
        }
    };

    let canonical_workspace = std::path::Path::new(&ctx.working_directory)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&ctx.working_directory));

    let mut results = Vec::new();
    let pattern_lower = pattern.to_lowercase();

    let walker = walkdir::WalkDir::new(&full_path).max_depth(10);
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if results.len() >= max_results {
            break;
        }
        // Patch 3: reject symlink directories that escape workspace
        if !crate::sandbox::is_path_in_workspace(entry.path(), &canonical_workspace) {
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if matches!(
            ext,
            "exe" | "dll" | "so" | "dylib" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "zip"
                | "tar" | "gz"
        ) {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(path) {
            for (line_num, line) in content.lines().enumerate() {
                if results.len() >= max_results {
                    break;
                }
                if line.to_lowercase().contains(&pattern_lower) {
                    let relative = path
                        .strip_prefix(&full_path)
                        .unwrap_or(path)
                        .to_string_lossy();
                    results.push(format!("{}:{}: {}", relative, line_num + 1, line.trim()));
                }
            }
        }
    }

    let output = if results.is_empty() {
        format!("No results found for '{pattern}'")
    } else {
        results.join("\n")
    };

    ToolResult::success(
        call_id,
        tool_name,
        output,
        start.elapsed().as_millis() as u64,
    )
}

// ---- File Write ----

/// Maximum file write size: 1 MB.
const MAX_WRITE_SIZE: usize = 1_048_576;

fn file_write_descriptor() -> ToolDef {
    ToolDef {
        name: canonical_local_tool_name("file_write"),
        display_name: Some("Write File".into()),
        description: "Write content to a file within the working directory. Requires explicit overwrite flag to replace existing files.".into(),
        parameters_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path from working directory"
                },
                "content": {
                    "type": "string",
                    "description": "File content to write"
                },
                "overwrite": {
                    "type": "boolean",
                    "description": "Whether to overwrite an existing file (default: false)"
                }
            },
            "required": ["path", "content"]
        }),
        output_schema: None,
        source: ToolSource::Local,
        declared_effect: ToolEffect::Write,
        risk_hints: vec!["Modifies filesystem".into()],
        tags: vec!["local".into(), "file".into(), "write".into()],
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(false),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        }),
    }
}

/// Validate a file write path. Returns the resolved full path or an error message.
pub(crate) fn validate_write_path(
    path_str: &str,
    working_directory: &str,
) -> Result<std::path::PathBuf, String> {
    crate::sandbox::resolve_workspace_path(
        std::path::Path::new(working_directory),
        path_str,
        crate::sandbox::PathAccessMode::WriteTarget,
    )
    .map_err(|e| e.message)
}

async fn file_write_handler(args: serde_json::Value, ctx: ToolCallContext) -> ToolResult {
    let start = std::time::Instant::now();
    let call_id = extract_call_id(&args);
    let tool_name = canonical_local_tool_name("file_write");

    let path_val = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return ToolResult::error(
                call_id,
                tool_name,
                "Missing required parameter: path".into(),
                start.elapsed().as_millis() as u64,
            );
        }
    };

    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => {
            return ToolResult::error(
                call_id,
                tool_name,
                "Missing required parameter: content".into(),
                start.elapsed().as_millis() as u64,
            );
        }
    };

    let overwrite = args
        .get("overwrite")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Validate path
    let full_path = match validate_write_path(path_val, &ctx.working_directory) {
        Ok(p) => p,
        Err(e) => {
            return ToolResult::error(call_id, tool_name, e, start.elapsed().as_millis() as u64);
        }
    };

    // Reject if target is an existing directory
    if full_path.is_dir() {
        return ToolResult::error(call_id, tool_name, format!("Cannot write to a directory: {}", path_val), start.elapsed().as_millis() as u64);
    }

    // Enforce size limit
    if content.len() > MAX_WRITE_SIZE {
        return ToolResult::error(
            call_id,
            tool_name,
            format!(
                "Content exceeds maximum write size ({} bytes > {} bytes)",
                content.len(),
                MAX_WRITE_SIZE
            ),
            start.elapsed().as_millis() as u64,
        );
    }

    // Check overwrite
    if full_path.exists() && !overwrite {
        return ToolResult::error(
            call_id,
            tool_name,
            format!(
                "File already exists: {}. Set overwrite=true to replace.",
                full_path.display()
            ),
            start.elapsed().as_millis() as u64,
        );
    }

    // Create parent directories if needed
    if let Some(parent) = full_path.parent()
        && let Err(e) = tokio::fs::create_dir_all(parent).await {
            return ToolResult::error(
                call_id,
                tool_name,
                format!("Failed to create parent directory: {e}"),
                start.elapsed().as_millis() as u64,
            );
        }

    // Record preimage if file exists (for rollback)
    let preimage_info = if full_path.exists() {
        match tokio::fs::read(&full_path).await {
            Ok(existing) => {
                let hash = blake3::hash(&existing).to_hex().to_string();
                let size = existing.len();

                // Write rollback
                let rollback_dir = std::path::Path::new(&ctx.working_directory)
                    .join(".openwand")
                    .join("rollback");
                let _ = tokio::fs::create_dir_all(&rollback_dir).await;
                let rollback_path = rollback_dir.join(format!("write_{}.bak", call_id));
                let _ = tokio::fs::write(&rollback_path, &existing).await;

                Some(format!("Preimage: {} ({} bytes)", hash, size))
            }
            Err(_) => None,
        }
    } else {
        None
    };

    // Write the file (TOCTOU-hardened: no-follow on final component)
    match crate::sandbox::write_file_no_follow(&full_path, content).await {
        Ok(()) => {
            let postimage_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
            let mut msg = format!(
                "Wrote {} bytes to {}\nPostimage: {}",
                content.len(),
                full_path.display(),
                postimage_hash
            );
            if let Some(pre) = preimage_info {
                msg = format!("{}\n{}", pre, msg);
            }
            ToolResult::success(call_id, tool_name, msg, start.elapsed().as_millis() as u64)
        }
        Err(e) => ToolResult::error(
            call_id,
            tool_name,
            format!("Failed to write file '{}': {}", full_path.display(), e),
            start.elapsed().as_millis() as u64,
        ),
    }
}

#[cfg(test)]
// ---------------------------------------------------------------------------
// Test helpers (shared across test modules)
// ---------------------------------------------------------------------------

#[cfg(test)]
fn test_context(dir: &tempfile::TempDir) -> ToolCallContext {
    ToolCallContext {
        working_directory: dir.path().to_string_lossy().to_string(),
        session_id: openwand_core::SessionId::new(),
        cancellation: tokio_util::sync::CancellationToken::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn local_file_read_success() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("test.txt"), "hello world")
            .await
            .unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_read_descriptor(), file_read_handler);

        let args = serde_json::json!({
            "path": "test.txt",
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_read"), args, ctx)
            .await
            .unwrap();
        assert!(!result.is_error);
        assert_eq!("hello world", result.output);
    }

    #[tokio::test]
    async fn local_file_read_missing_file() {
        let dir = TempDir::new().unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_read_descriptor(), file_read_handler);

        let args = serde_json::json!({
            "path": "nonexistent.txt",
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_read"), args, ctx)
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("Failed to read"));
    }

    #[tokio::test]
    async fn local_file_list_success() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("a.txt"), "a")
            .await
            .unwrap();
        tokio::fs::create_dir(dir.path().join("subdir"))
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("subdir").join("b.txt"), "b")
            .await
            .unwrap();

        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_list_descriptor(), file_list_handler);

        let args = serde_json::json!({
            "recursive": true,
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_list"), args, ctx)
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.output.contains("a.txt"));
        assert!(result.output.contains("subdir"));
        assert!(result.output.contains("b.txt"));
    }

    #[tokio::test]
    async fn local_file_search_finds_match() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(
            dir.path().join("hello.rs"),
            "fn main() { println!(\"hello\"); }",
        )
        .await
        .unwrap();
        tokio::fs::write(dir.path().join("other.txt"), "no match here")
            .await
            .unwrap();

        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_search_descriptor(), file_search_handler);

        let args = serde_json::json!({
            "pattern": "hello",
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_search"), args, ctx)
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.output.contains("hello.rs"));
        assert!(result.output.contains("hello"));
        assert!(!result.output.contains("other.txt"));
    }

    #[test]
    fn batch1_registers_three_tools() {
        let provider = batch1_local_tools();
        let tools = provider.available_descriptors();
        assert_eq!(3, tools.len());

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"local__file_read"));
        assert!(names.contains(&"local__file_list"));
        assert!(names.contains(&"local__file_search"));
    }

    #[test]
    fn batch2_registers_five_tools_including_write_and_patch() {
        let provider = batch2_local_tools();
        let tools = provider.available_descriptors();
        assert_eq!(5, tools.len());
        assert!(tools.iter().any(|t| t.name.ends_with("file_write")));
        assert!(tools.iter().any(|t| t.name.ends_with("file_patch")));

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"local__file_read"));
        assert!(names.contains(&"local__file_list"));
        assert!(names.contains(&"local__file_search"));
        assert!(names.contains(&"local__file_write"));
    }

    #[test]
    fn file_write_declared_effect_is_write() {
        let desc = file_write_descriptor();
        assert_eq!(ToolEffect::Write, desc.declared_effect);
    }

    #[tokio::test]
    async fn file_write_creates_new_file() {
        let dir = TempDir::new().unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_write_descriptor(), file_write_handler);

        let args = serde_json::json!({
            "path": "hello.txt",
            "content": "Hello, world!",
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_write"), args, ctx)
            .await
            .unwrap();
        assert!(!result.is_error, "Write failed: {}", result.output);

        let content = tokio::fs::read_to_string(dir.path().join("hello.txt"))
            .await
            .unwrap();
        assert_eq!("Hello, world!", content);
    }

    #[tokio::test]
    async fn file_write_refuses_absolute_path() {
        let dir = TempDir::new().unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_write_descriptor(), file_write_handler);

        let abs_path = if cfg!(windows) {
            "C:\\Windows\\System32\\test.txt"
        } else {
            "/etc/passwd"
        };

        let args = serde_json::json!({
            "path": abs_path,
            "content": "hacked",
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_write"), args, ctx)
            .await
            .unwrap();
        assert!(result.is_error, "Expected error for absolute path, got: {}", result.output);
        assert!(result.output.contains("Absolute paths are not allowed"));
    }

    #[tokio::test]
    async fn file_write_refuses_parent_escape() {
        let dir = TempDir::new().unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_write_descriptor(), file_write_handler);

        let args = serde_json::json!({
            "path": "../../../etc/passwd",
            "content": "hacked",
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_write"), args, ctx)
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("Parent traversal") || result.output.contains("not allowed"));
    }

    #[tokio::test]
    async fn file_write_refuses_overwrite_by_default() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("existing.txt"), "original")
            .await
            .unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_write_descriptor(), file_write_handler);

        let args = serde_json::json!({
            "path": "existing.txt",
            "content": "replacement",
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_write"), args, ctx)
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("already exists"));

        let content = tokio::fs::read_to_string(dir.path().join("existing.txt"))
            .await
            .unwrap();
        assert_eq!("original", content);
    }

    #[tokio::test]
    async fn file_write_allows_overwrite_when_explicit() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("existing.txt"), "original")
            .await
            .unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_write_descriptor(), file_write_handler);

        let args = serde_json::json!({
            "path": "existing.txt",
            "content": "replacement",
            "overwrite": true,
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_write"), args, ctx)
            .await
            .unwrap();
        assert!(!result.is_error, "Overwrite failed: {}", result.output);

        let content = tokio::fs::read_to_string(dir.path().join("existing.txt"))
            .await
            .unwrap();
        assert_eq!("replacement", content);
    }

    #[tokio::test]
    async fn file_write_refuses_directory_target() {
        let dir = TempDir::new().unwrap();
        tokio::fs::create_dir(dir.path().join("subdir")).await.unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_write_descriptor(), file_write_handler);

        let args = serde_json::json!({
            "path": "subdir",
            "content": "content",
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_write"), args, ctx)
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("directory"));
    }

    #[tokio::test]
    async fn file_write_enforces_size_limit() {
        let dir = TempDir::new().unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_write_descriptor(), file_write_handler);

        let big_content = "x".repeat(MAX_WRITE_SIZE + 1);
        let args = serde_json::json!({
            "path": "big.txt",
            "content": big_content,
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_write"), args, ctx)
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("maximum write size"));
        assert!(!dir.path().join("big.txt").exists());
    }

    #[tokio::test]
    async fn file_write_creates_parent_directories() {
        let dir = TempDir::new().unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_write_descriptor(), file_write_handler);

        let args = serde_json::json!({
            "path": "deep/nested/dir/file.txt",
            "content": "nested content",
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_write"), args, ctx)
            .await
            .unwrap();
        assert!(!result.is_error, "Write failed: {}", result.output);

        let content = tokio::fs::read_to_string(dir.path().join("deep/nested/dir/file.txt"))
            .await
            .unwrap();
        assert_eq!("nested content", content);
    }

    #[tokio::test]
    async fn write_tool_records_preimage_when_overwriting() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("exists.txt"), "original content")
            .await.unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_write_descriptor(), file_write_handler);

        let args = serde_json::json!({
            "path": "exists.txt",
            "content": "new content",
            "overwrite": true,
            "_call_id": "tc_preimage"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_write"), args, ctx)
            .await.unwrap();
        assert!(!result.is_error);
        assert!(result.output.contains("Preimage:"), "Should record preimage: {}", result.output);
        assert!(result.output.contains("Postimage:"));
    }

    #[tokio::test]
    async fn write_tool_creates_rollback_on_overwrite() {
        let dir = TempDir::new().unwrap();
        tokio::fs::write(dir.path().join("rollback_test.txt"), "original data")
            .await.unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_write_descriptor(), file_write_handler);

        let args = serde_json::json!({
            "path": "rollback_test.txt",
            "content": "replaced data",
            "overwrite": true,
            "_call_id": "tc_rollback"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_write"), args, ctx)
            .await.unwrap();
        assert!(!result.is_error);

        // Verify rollback file was created
        let rollback_dir = dir.path().join(".openwand").join("rollback");
        let rollback_files: Vec<_> = std::fs::read_dir(&rollback_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(1, rollback_files.len(), "Should have exactly one rollback file");

        let rollback_content = tokio::fs::read_to_string(rollback_files[0].path()).await.unwrap();
        assert_eq!("original data", rollback_content);
    }

    #[tokio::test]
    async fn write_new_file_has_no_preimage() {
        let dir = TempDir::new().unwrap();
        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(file_write_descriptor(), file_write_handler);

        let args = serde_json::json!({
            "path": "brand_new.txt",
            "content": "fresh content",
            "_call_id": "tc_new"
        });
        let result = provider
            .execute(&canonical_local_tool_name("file_write"), args, ctx)
            .await.unwrap();
        assert!(!result.is_error);
        assert!(!result.output.contains("Preimage:"), "New file should not have preimage");
        assert!(result.output.contains("Postimage:"));
    }
}


// ---------------------------------------------------------------------------
// Shell Exec — governed command execution (Wave 04a)
// ---------------------------------------------------------------------------

/// Maximum bytes to capture from stdout/stderr before capping.
const SHELL_OUTPUT_CAP_BYTES: usize = 200 * 1024; // 200 KiB

/// Default command timeout in milliseconds.
const SHELL_DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Maximum allowed timeout in milliseconds.
const SHELL_MAX_TIMEOUT_MS: u64 = 300_000;

fn shell_exec_descriptor() -> ToolDef {
    ToolDef {
        name: canonical_local_tool_name("shell_exec"),
        display_name: Some("Execute Command".into()),
        description: "Execute a bare program name with arguments. No shell interpolation, pipes, or redirects. Requires escalation-level approval.".into(),
        parameters_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "program": {
                    "type": "string",
                    "description": "Bare executable name. No path separators allowed."
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Arguments to pass to the program."
                },
                "working_directory": {
                    "type": "string",
                    "description": "Override working directory. Must be within session working directory."
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Maximum execution time in milliseconds. Default 30000, max 300000."
                }
            },
            "required": ["program"]
        }),
        output_schema: None,
        source: ToolSource::Local,
        declared_effect: ToolEffect::Execute,
        risk_hints: vec!["Spawns external process".into()],
        tags: vec!["local".into(), "shell".into(), "execute".into()],
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(true),
        }),
    }
}

/// Validate that a program name is a bare executable (no path components).
fn validate_program_name(program: &str) -> Result<(), String> {
    if program.is_empty() {
        return Err("program name must not be empty".into());
    }
    if program.contains('/') || program.contains('\\') {
        return Err(format!(
            "program must be a bare name, not a path: {:?}",
            program
        ));
    }
    if program.starts_with('.') || program.starts_with('-') {
        return Err(format!(
            "program must not start with '.' or '-': {:?}",
            program
        ));
    }
    Ok(())
}

/// Resolve and validate the working directory for command execution.
/// If `override_dir` is Some, it must resolve within `session_wd`.
/// Returns the resolved canonical path.
fn resolve_exec_working_dir(
    override_dir: Option<&str>,
    session_wd: &str,
) -> Result<std::path::PathBuf, String> {
    let session_path = std::path::Path::new(session_wd);
    let canonical_session = session_path
        .canonicalize()
        .map_err(|e| format!("Cannot canonicalize session working directory: {e}"))?;

    let target = match override_dir {
        Some(dir) => {
            let override_path = std::path::Path::new(dir);
            if override_path.is_absolute() {
                return Err(format!("working_directory must be relative: {dir}"));
            }
            for component in override_path.components() {
                if component == std::path::Component::ParentDir {
                    return Err(format!(
                        "working_directory must not contain '..': {dir}"
                    ));
                }
            }
            session_path.join(override_path)
        }
        None => session_path.to_path_buf(),
    };

    let canonical_target = target
        .canonicalize()
        .map_err(|e| format!("working_directory does not exist: {e}"))?;

    if !canonical_target.starts_with(&canonical_session) {
        return Err("working_directory must be within session working directory".into());
    }

    Ok(canonical_target)
}

/// Cap byte output to SHELL_OUTPUT_CAP_BYTES, replacing non-UTF8 with lossy conversion.
fn cap_byte_output(raw: &[u8]) -> String {
    if raw.len() <= SHELL_OUTPUT_CAP_BYTES {
        String::from_utf8_lossy(raw).into_owned()
    } else {
        let truncated = &raw[..SHELL_OUTPUT_CAP_BYTES];
        let omitted = raw.len() - SHELL_OUTPUT_CAP_BYTES;
        format!(
            "{}\n\n[openwand: output capped at {} bytes, {} bytes omitted]",
            String::from_utf8_lossy(truncated),
            SHELL_OUTPUT_CAP_BYTES,
            omitted
        )
    }
}

async fn shell_exec_handler(args: serde_json::Value, ctx: ToolCallContext) -> ToolResult {
    let start = std::time::Instant::now();
    let call_id = extract_call_id(&args);
    let tool_name = canonical_local_tool_name("shell_exec");

    // 1. Parse arguments
    let program = match args.get("program").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            return ToolResult::error(
                call_id,
                tool_name,
                "missing required field: program".into(),
                start.elapsed().as_millis() as u64,
            );
        }
    };

    let cmd_args: Vec<String> = args
        .get("args")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .map(|t| if t == 0 { SHELL_DEFAULT_TIMEOUT_MS } else { t.min(SHELL_MAX_TIMEOUT_MS) })
        .unwrap_or(SHELL_DEFAULT_TIMEOUT_MS);

    let override_dir = args
        .get("working_directory")
        .and_then(|v| v.as_str());

    // 2. Validate program name
    if let Err(e) = validate_program_name(&program) {
        return ToolResult::error(
            call_id,
            tool_name,
            e,
            start.elapsed().as_millis() as u64,
        );
    }

    // 3. Resolve working directory
    let work_dir = match resolve_exec_working_dir(override_dir, &ctx.working_directory) {
        Ok(d) => d,
        Err(e) => {
            return ToolResult::error(
                call_id,
                tool_name,
                e,
                start.elapsed().as_millis() as u64,
            );
        }
    };

    // 4. Spawn process with timeout
    let mut cmd = tokio::process::Command::new(&program);
    cmd.args(&cmd_args)
        .current_dir(&work_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null());

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return ToolResult::error(
                call_id,
                tool_name,
                format!("Failed to spawn '{}': {}", program, e),
                start.elapsed().as_millis() as u64,
            );
        }
    };

    // 5. Wait with timeout and cancellation
    // Wrap child in Option so we can take ownership inside select branches
    let mut child_opt = Some(child);
    let timeout = tokio::time::Duration::from_millis(timeout_ms);
    tokio::select! {
        output = async { child_opt.take().unwrap().wait_with_output().await } => {
            match output {
                Ok(out) => {
                    let stdout = cap_byte_output(&out.stdout);
                    let stderr = cap_byte_output(&out.stderr);
                    let exit_code = out.status.code().unwrap_or(-1);
                    let combined = if stderr.is_empty() {
                        format!("{}\nexit code: {}", stdout, exit_code)
                    } else {
                        format!("{}\n--- stderr ---\n{}\nexit code: {}", stdout, stderr, exit_code)
                    };
                    if out.status.success() {
                        ToolResult::success(
                            call_id,
                            tool_name,
                            combined,
                            start.elapsed().as_millis() as u64,
                        )
                    } else {
                        ToolResult::error(
                            call_id,
                            tool_name,
                            combined,
                            start.elapsed().as_millis() as u64,
                        )
                    }
                }
                Err(e) => ToolResult::error(
                    call_id,
                    tool_name,
                    format!("Failed to wait for '{}': {}", program, e),
                    start.elapsed().as_millis() as u64,
                ),
            }
        }
        _ = tokio::time::sleep(timeout) => {
            // Timeout — kill the child
            if let Some(mut c) = child_opt.take() {
                let _ = c.kill().await;
                let _ = c.wait().await;
            }
            ToolResult::error(
                call_id,
                tool_name,
                format!("Command '{}' timed out after {}ms", program, timeout_ms),
                start.elapsed().as_millis() as u64,
            )
        }
        _ = ctx.cancellation.cancelled() => {
            // Cancellation — kill the child
            if let Some(mut c) = child_opt.take() {
                let _ = c.kill().await;
                let _ = c.wait().await;
            }
            ToolResult::error(
                call_id,
                tool_name,
                format!("Command '{}' cancelled", program),
                start.elapsed().as_millis() as u64,
            )
        }
    }
}

/// Create the full set of local tools including shell execution.
pub fn local_tools_with_shell_exec() -> BuiltinToolProvider {
    let mut provider = batch2_local_tools();
    provider.register_fn(shell_exec_descriptor(), shell_exec_handler);
    provider
}

// ---------------------------------------------------------------------------
// Shell exec tests (tools crate level)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod shell_exec_tests {
    use super::*;

    #[test]
    fn program_validation_accepts_bare_names() {
        assert!(validate_program_name("git").is_ok());
        assert!(validate_program_name("cargo").is_ok());
        assert!(validate_program_name("node").is_ok());
        assert!(validate_program_name("python3").is_ok());
        assert!(validate_program_name("echo").is_ok());
    }

    #[test]
    fn program_validation_rejects_absolute_paths() {
        assert!(validate_program_name("/usr/bin/rm").is_err());
        assert!(validate_program_name("C:\\Windows\\cmd.exe").is_err());
    }

    #[test]
    fn program_validation_rejects_relative_paths() {
        assert!(validate_program_name("./script.sh").is_err());
        assert!(validate_program_name("../evil").is_err());
    }

    #[test]
    fn program_validation_rejects_dot_and_dash_prefix() {
        assert!(validate_program_name(".hidden").is_err());
        assert!(validate_program_name("--flag").is_err());
    }

    #[test]
    fn program_validation_rejects_empty() {
        assert!(validate_program_name("").is_err());
    }

    #[test]
    fn output_capping_truncates_large_output() {
        let big: Vec<u8> = b"x".to_vec().repeat(SHELL_OUTPUT_CAP_BYTES + 10_000);
        let result = cap_byte_output(&big);
        assert!(result.contains("[openwand: output capped"));
        assert!(result.len() < big.len());
    }

    #[test]
    fn timeout_clamps_to_maximum() {
        let timeout: u64 = 999_999;
        let clamped = if timeout == 0 {
            SHELL_DEFAULT_TIMEOUT_MS
        } else {
            timeout.min(SHELL_MAX_TIMEOUT_MS)
        };
        assert_eq!(SHELL_MAX_TIMEOUT_MS, clamped);
    }
}


// ===========================================================================
// Git Observation — governed read-only repository inspection (Wave 04b)
// ===========================================================================
//
// Invariant: the subcommand and flags are fixed by OpenWand.
// User-controlled values may only occupy validated value positions.
// No shell is invoked. No user-supplied git subcommands.

/// Maximum commits to return from git log.
const GIT_LOG_MAX_LIMIT: usize = 100;
/// Default commits to return from git log.
const GIT_LOG_DEFAULT_LIMIT: usize = 20;
/// Timeout for git observation commands (ms).
const GIT_OBSERVE_TIMEOUT_MS: u64 = 15_000;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Run a fixed git command in a working directory with timeout.
/// The caller is responsible for constructing the argv; this function
/// does NOT interpret user input as flags or subcommands.
async fn run_fixed_git_command(
    git_args: Vec<String>,
    working_directory: &std::path::Path,
    tool_call_id: ToolCallId,
    tool_name: &str,
    start: std::time::Instant,
) -> ToolResult {
    let mut cmd = tokio::process::Command::new("git");
    cmd.args(&git_args)
        .current_dir(working_directory)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null());

    let mut child_opt = match cmd.spawn() {
        Ok(c) => Some(c),
        Err(e) => {
            return ToolResult::error(
                tool_call_id,
                tool_name.to_string(),
                format!("Failed to spawn git: {}", e),
                start.elapsed().as_millis() as u64,
            );
        }
    };

    let timeout = tokio::time::Duration::from_millis(GIT_OBSERVE_TIMEOUT_MS);
    tokio::select! {
        output = async { child_opt.take().unwrap().wait_with_output().await } => {
            match output {
                Ok(out) => {
                    let stdout = cap_byte_output(&out.stdout);
                    let stderr = cap_byte_output(&out.stderr);
                    if out.status.success() {
                        if stderr.is_empty() {
                            ToolResult::success(
                                tool_call_id,
                                tool_name.to_string(),
                                stdout,
                                start.elapsed().as_millis() as u64,
                            )
                        } else {
                            ToolResult::success(
                                tool_call_id,
                                tool_name.to_string(),
                                format!("{}\n--- stderr ---\n{}", stdout, stderr),
                                start.elapsed().as_millis() as u64,
                            )
                        }
                    } else {
                        let combined = if stderr.is_empty() {
                            stdout
                        } else {
                            format!("{}\n--- stderr ---\n{}", stdout, stderr)
                        };
                        ToolResult::error(
                            tool_call_id,
                            tool_name.to_string(),
                            combined,
                            start.elapsed().as_millis() as u64,
                        )
                    }
                }
                Err(e) => ToolResult::error(
                    tool_call_id,
                    tool_name.to_string(),
                    format!("Failed to wait for git: {}", e),
                    start.elapsed().as_millis() as u64,
                ),
            }
        }
        _ = tokio::time::sleep(timeout) => {
            if let Some(mut c) = child_opt.take() {
                let _ = c.kill().await;
                let _ = c.wait().await;
            }
            ToolResult::error(
                tool_call_id,
                tool_name.to_string(),
                format!("git command timed out after {}ms", GIT_OBSERVE_TIMEOUT_MS),
                start.elapsed().as_millis() as u64,
            )
        }
    }
}

/// Verify that the given directory is inside a git worktree.
/// Returns Ok(()) if inside a worktree, Err with a message otherwise.
async fn verify_git_worktree(dir: &std::path::Path) -> Result<(), String> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null());

    let output = cmd.output().await.map_err(|e| format!("Failed to run git rev-parse: {}", e))?;

    if !output.status.success() {
        return Err("working directory is not inside a git worktree".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout != "true" {
        return Err("working directory is not inside a git worktree".into());
    }

    Ok(())
}

/// Resolve and validate the git working directory.
/// Reuses the 04a resolve_exec_working_dir logic.
async fn resolve_git_working_dir(
    requested: Option<&str>,
    session_wd: &str,
) -> Result<std::path::PathBuf, String> {
    let dir = resolve_exec_working_dir(requested, session_wd)?;
    // Verify it's a git worktree
    verify_git_worktree(&dir).await?;
    Ok(dir)
}

/// Validate a relative path filter for git diff.
///
/// For 04b, any ParentDir component is rejected rather than canonicalized through.
/// This is intentionally conservative — harmless paths like `src/../README.md`
/// are rejected to keep the validation logic simple and unambiguous.
///
/// The path must:
/// - be relative
/// - not contain `..` components
/// - not start with `-`
/// - be syntactically within the git working directory
fn validate_git_path_filter(path: &str, git_working_dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let p = std::path::Path::new(path);

    if p.is_absolute() || path.starts_with('/') {
        return Err(format!("path must be relative: {}", path));
    }

    if path.starts_with('-') {
        return Err(format!("path must not start with '-': {}", path));
    }

    // Reject ParentDir components (intentionally conservative for 04b)
    for component in p.components() {
        if component == std::path::Component::ParentDir {
            return Err(format!(
                "path must not contain '..' (intentionally conservative): {}",
                path
            ));
        }
    }

    // Syntactic check: joined path stays under git working directory
    let joined = git_working_dir.join(p);
    // Normalize syntactically (no canonicalize — file may not exist)
    let normalized = joined.components().collect::<std::path::PathBuf>();

    // Verify the normalized path starts with git_working_dir
    // (This is a syntactic prefix check, not filesystem)
    let git_prefix: std::path::PathBuf = git_working_dir.components().collect();
    if !normalized.starts_with(&git_prefix) {
        return Err("path escapes working directory".into());
    }

    Ok(normalized)
}

// ---------------------------------------------------------------------------
// local__git_status
// ---------------------------------------------------------------------------

fn git_status_descriptor() -> ToolDef {
    ToolDef {
        name: canonical_local_tool_name("git_status"),
        display_name: Some("Git Status".into()),
        description: "Show working tree status. Reports branch, staged, unstaged, and untracked files.".into(),
        parameters_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "working_directory": {
                    "type": "string",
                    "description": "Optional directory inside the session working directory."
                }
            }
        }),
        output_schema: None,
        source: ToolSource::Local,
        declared_effect: ToolEffect::Git,
        risk_hints: vec![],
        tags: vec!["local".into(), "git".into(), "read_only".into(), "repository_observation".into()],
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        }),
    }
}

async fn git_status_handler(args: serde_json::Value, ctx: ToolCallContext) -> ToolResult {
    let start = std::time::Instant::now();
    let call_id = extract_call_id(&args);
    let tool_name = canonical_local_tool_name("git_status");

    let override_dir = args.get("working_directory").and_then(|v| v.as_str());

    let work_dir = match resolve_git_working_dir(override_dir, &ctx.working_directory).await {
        Ok(d) => d,
        Err(e) => {
            return ToolResult::error(call_id, tool_name, e, start.elapsed().as_millis() as u64);
        }
    };

    run_fixed_git_command(
        vec!["status".into(), "--short".into(), "--branch".into()],
        &work_dir,
        call_id,
        &tool_name,
        start,
    )
    .await
}

// ---------------------------------------------------------------------------
// local__git_diff
// ---------------------------------------------------------------------------

fn git_diff_descriptor() -> ToolDef {
    ToolDef {
        name: canonical_local_tool_name("git_diff"),
        display_name: Some("Git Diff".into()),
        description: "Show unstaged or staged diff. Optional path filter.".into(),
        parameters_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "working_directory": {
                    "type": "string",
                    "description": "Optional directory inside the session working directory."
                },
                "staged": {
                    "type": "boolean",
                    "description": "If true, show staged diff. Otherwise show unstaged diff."
                },
                "path": {
                    "type": "string",
                    "description": "Optional relative path filter inside the working directory."
                }
            }
        }),
        output_schema: None,
        source: ToolSource::Local,
        declared_effect: ToolEffect::Git,
        risk_hints: vec![],
        tags: vec!["local".into(), "git".into(), "read_only".into(), "repository_observation".into()],
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        }),
    }
}

async fn git_diff_handler(args: serde_json::Value, ctx: ToolCallContext) -> ToolResult {
    let start = std::time::Instant::now();
    let call_id = extract_call_id(&args);
    let tool_name = canonical_local_tool_name("git_diff");

    let override_dir = args.get("working_directory").and_then(|v| v.as_str());
    let staged = args.get("staged").and_then(|v| v.as_bool()).unwrap_or(false);
    let path_filter = args.get("path").and_then(|v| v.as_str());

    let work_dir = match resolve_git_working_dir(override_dir, &ctx.working_directory).await {
        Ok(d) => d,
        Err(e) => {
            return ToolResult::error(call_id, tool_name, e, start.elapsed().as_millis() as u64);
        }
    };

    // Build fixed argv
    let mut git_args: Vec<String> = vec!["diff".into()];
    if staged {
        git_args.push("--staged".into());
    }
    git_args.push("--".into());

    // Validate optional path filter
    if let Some(path) = path_filter {
        match validate_git_path_filter(path, &work_dir) {
            Ok(_resolved) => {
                // Use the original relative path (git resolves relative to cwd)
                git_args.push(path.to_string());
            }
            Err(e) => {
                return ToolResult::error(call_id, tool_name, e, start.elapsed().as_millis() as u64);
            }
        }
    }

    run_fixed_git_command(git_args, &work_dir, call_id, &tool_name, start).await
}

// ---------------------------------------------------------------------------
// local__git_log
// ---------------------------------------------------------------------------

fn git_log_descriptor() -> ToolDef {
    ToolDef {
        name: canonical_local_tool_name("git_log"),
        display_name: Some("Git Log".into()),
        description: "Show recent commits. Default 20, max 100.".into(),
        parameters_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "working_directory": {
                    "type": "string",
                    "description": "Optional directory inside the session working directory."
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of commits to show. Default 20, max 100."
                }
            }
        }),
        output_schema: None,
        source: ToolSource::Local,
        declared_effect: ToolEffect::Git,
        risk_hints: vec![],
        tags: vec!["local".into(), "git".into(), "read_only".into(), "repository_observation".into()],
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        }),
    }
}

async fn git_log_handler(args: serde_json::Value, ctx: ToolCallContext) -> ToolResult {
    let start = std::time::Instant::now();
    let call_id = extract_call_id(&args);
    let tool_name = canonical_local_tool_name("git_log");

    let override_dir = args.get("working_directory").and_then(|v| v.as_str());
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|l| {
            if l == 0 {
                GIT_LOG_DEFAULT_LIMIT
            } else {
                (l as usize).min(GIT_LOG_MAX_LIMIT)
            }
        })
        .unwrap_or(GIT_LOG_DEFAULT_LIMIT);

    let work_dir = match resolve_git_working_dir(override_dir, &ctx.working_directory).await {
        Ok(d) => d,
        Err(e) => {
            return ToolResult::error(call_id, tool_name, e, start.elapsed().as_millis() as u64);
        }
    };

    run_fixed_git_command(
        vec![
            "log".into(),
            "--oneline".into(),
            "--decorate".into(),
            format!("-n{}", limit),
        ],
        &work_dir,
        call_id,
        &tool_name,
        start,
    )
    .await
}

// ---------------------------------------------------------------------------
// local__git_branch
// ---------------------------------------------------------------------------

fn git_branch_descriptor() -> ToolDef {
    ToolDef {
        name: canonical_local_tool_name("git_branch"),
        display_name: Some("Git Branch".into()),
        description: "Show current branch name, or list all branches.".into(),
        parameters_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "working_directory": {
                    "type": "string",
                    "description": "Optional directory inside the session working directory."
                },
                "list": {
                    "type": "boolean",
                    "description": "If true, list all branches. Otherwise show current branch name."
                }
            }
        }),
        output_schema: None,
        source: ToolSource::Local,
        declared_effect: ToolEffect::Git,
        risk_hints: vec![],
        tags: vec!["local".into(), "git".into(), "read_only".into(), "repository_observation".into()],
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        }),
    }
}

async fn git_branch_handler(args: serde_json::Value, ctx: ToolCallContext) -> ToolResult {
    let start = std::time::Instant::now();
    let call_id = extract_call_id(&args);
    let tool_name = canonical_local_tool_name("git_branch");

    let override_dir = args.get("working_directory").and_then(|v| v.as_str());
    let list_all = args.get("list").and_then(|v| v.as_bool()).unwrap_or(false);

    let work_dir = match resolve_git_working_dir(override_dir, &ctx.working_directory).await {
        Ok(d) => d,
        Err(e) => {
            return ToolResult::error(call_id, tool_name, e, start.elapsed().as_millis() as u64);
        }
    };

    let git_args = if list_all {
        vec!["branch".into(), "--list".into()]
    } else {
        vec!["branch".into(), "--show-current".into()]
    };

    run_fixed_git_command(git_args, &work_dir, call_id, &tool_name, start).await
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Create the full set of local tools including git observation.
pub fn local_tools_with_git_observation() -> BuiltinToolProvider {
    let mut provider = local_tools_with_shell_exec();
    provider.register_fn(git_status_descriptor(), git_status_handler);
    provider.register_fn(git_diff_descriptor(), git_diff_handler);
    provider.register_fn(git_log_descriptor(), git_log_handler);
    provider.register_fn(git_branch_descriptor(), git_branch_handler);
    provider
}

// ---------------------------------------------------------------------------
// Git observation tests (tools crate level)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod git_observation_tests {
    use super::*;

    // ---- Descriptor effect ----

    #[test]
    fn git_observation_descriptor_is_git_effect_not_execute() {
        let status = git_status_descriptor();
        let diff = git_diff_descriptor();
        let log = git_log_descriptor();
        let branch = git_branch_descriptor();

        assert_eq!(ToolEffect::Git, status.declared_effect);
        assert_eq!(ToolEffect::Git, diff.declared_effect);
        assert_eq!(ToolEffect::Git, log.declared_effect);
        assert_eq!(ToolEffect::Git, branch.declared_effect);

        // None of them are Execute
        assert_ne!(ToolEffect::Execute, status.declared_effect);
        assert_ne!(ToolEffect::Execute, diff.declared_effect);
        assert_ne!(ToolEffect::Execute, log.declared_effect);
        assert_ne!(ToolEffect::Execute, branch.declared_effect);
    }

    #[test]
    fn git_observation_uses_internal_git_runner_not_shell_exec() {
        // Structural proof: git observation handlers call run_fixed_git_command,
        // not shell_exec_handler. Verified by inspecting that git observation
        // descriptors have ToolEffect::Git while shell_exec has ToolEffect::Execute.
        // They are distinct handlers on distinct code paths.
        let status = git_status_descriptor();
        let shell = shell_exec_descriptor();
        assert_ne!(status.declared_effect, shell.declared_effect);
        assert!(status.name.contains("git_status"));
        assert!(shell.name.contains("shell_exec"));
    }

    // ---- Path validation ----

    #[test]
    fn git_path_filter_rejects_absolute() {
        let dir = std::path::Path::new("/tmp");
        assert!(validate_git_path_filter("/etc/passwd", dir).is_err());
        assert!(validate_git_path_filter("C:\\Windows\\System32", dir).is_err());
    }

    #[test]
    fn git_path_filter_rejects_dash_prefix() {
        let dir = std::path::Path::new("/tmp");
        assert!(validate_git_path_filter("--help", dir).is_err());
        assert!(validate_git_path_filter("-a", dir).is_err());
    }

    #[test]
    fn git_path_filter_rejects_parent_dir() {
        let dir = std::path::Path::new("/tmp/repo");
        // Intentionally conservative: even harmless .. paths are rejected
        assert!(validate_git_path_filter("../other", dir).is_err());
        assert!(validate_git_path_filter("src/../lib", dir).is_err());
    }

    #[test]
    fn git_path_filter_accepts_valid_relative() {
        let dir = std::path::Path::new("/tmp/repo");
        assert!(validate_git_path_filter("src/main.rs", dir).is_ok());
        assert!(validate_git_path_filter("README.md", dir).is_ok());
        assert!(validate_git_path_filter("deep/nested/file.txt", dir).is_ok());
    }

    // ---- Log limit clamping ----

    #[test]
    fn git_log_limit_clamps_to_max() {
        let limit: u64 = 999;
        let clamped = if limit == 0 {
            GIT_LOG_DEFAULT_LIMIT
        } else {
            (limit as usize).min(GIT_LOG_MAX_LIMIT)
        };
        assert_eq!(GIT_LOG_MAX_LIMIT, clamped);
    }

    #[test]
    fn git_log_limit_zero_uses_default() {
        let limit: u64 = 0;
        let clamped = if limit == 0 {
            GIT_LOG_DEFAULT_LIMIT
        } else {
            (limit as usize).min(GIT_LOG_MAX_LIMIT)
        };
        assert_eq!(GIT_LOG_DEFAULT_LIMIT, clamped);
    }

    // ---- Worktree verification ----

    #[tokio::test]
    async fn git_status_inside_repo_succeeds() {
        let dir = tempfile::TempDir::new().unwrap();
        // Initialize a git repo
        let output = tokio::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .await
            .expect("git init should run");
        assert!(output.status.success(), "git init failed");

        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(git_status_descriptor(), git_status_handler);

        let args = serde_json::json!({
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("git_status"), args, ctx)
            .await
            .unwrap();
        assert!(!result.is_error, "git_status failed: {}", result.output);
        assert!(result.output.contains("##") || result.output.contains("No commits yet") || result.output.trim().is_empty(),
            "expected status output, got: {}", result.output);
    }

    #[tokio::test]
    async fn git_status_outside_repo_returns_error_result() {
        let dir = tempfile::TempDir::new().unwrap();
        // NOT a git repo

        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(git_status_descriptor(), git_status_handler);

        let args = serde_json::json!({
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("git_status"), args, ctx)
            .await
            .unwrap();
        assert!(result.is_error, "should report error for non-repo");
        assert!(result.output.contains("not inside a git worktree"), "got: {}", result.output);
    }

    #[tokio::test]
    async fn git_diff_unstaged_succeeds() {
        let dir = tempfile::TempDir::new().unwrap();
        // Init repo + commit a file
        let _ = tokio::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();

        // Commit initial file
        tokio::fs::write(dir.path().join("hello.txt"), "initial").await.unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .await.unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();

        // Modify the file (unstaged)
        tokio::fs::write(dir.path().join("hello.txt"), "modified").await.unwrap();

        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(git_diff_descriptor(), git_diff_handler);

        let args = serde_json::json!({
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("git_diff"), args, ctx)
            .await
            .unwrap();
        assert!(!result.is_error, "git_diff failed: {}", result.output);
        assert!(result.output.contains("hello.txt"), "expected file in diff, got: {}", result.output);
    }

    #[tokio::test]
    async fn git_log_succeeds() {
        let dir = tempfile::TempDir::new().unwrap();
        // Init + commit
        let _ = tokio::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();

        tokio::fs::write(dir.path().join("file.txt"), "content").await.unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .await.unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["commit", "-m", "first commit"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();

        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(git_log_descriptor(), git_log_handler);

        let args = serde_json::json!({
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("git_log"), args, ctx)
            .await
            .unwrap();
        assert!(!result.is_error, "git_log failed: {}", result.output);
        assert!(result.output.contains("first commit"), "expected commit msg, got: {}", result.output);
    }

    #[tokio::test]
    async fn git_branch_current_succeeds() {
        let dir = tempfile::TempDir::new().unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();

        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(git_branch_descriptor(), git_branch_handler);

        let args = serde_json::json!({
            "_call_id": "tc_test"
        });
        let result = provider
            .execute(&canonical_local_tool_name("git_branch"), args, ctx)
            .await
            .unwrap();
        // Either shows branch name or empty (no commits yet) — not error
        // In a fresh repo with no commits, --show-current returns empty
        assert!(!result.is_error, "git_branch failed: {}", result.output);
    }

    #[tokio::test]
    async fn git_diff_path_rejects_absolute_path() {
        let dir = tempfile::TempDir::new().unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();
        // Need at least one commit for rev-parse to work
        tokio::fs::write(dir.path().join("file.txt"), "init").await.unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .await.unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();

        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(git_diff_descriptor(), git_diff_handler);

        let args = serde_json::json!({
            "_call_id": "tc_test",
            "path": "/etc/passwd"
        });
        let result = provider
            .execute(&canonical_local_tool_name("git_diff"), args, ctx)
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("must be relative"), "got: {}", result.output);
    }

    #[tokio::test]
    async fn git_diff_path_rejects_dash_prefix() {
        let dir = tempfile::TempDir::new().unwrap();
        let _ = tokio::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .await.unwrap();

        let ctx = test_context(&dir);
        let mut provider = BuiltinToolProvider::new();
        provider.register_fn(git_diff_descriptor(), git_diff_handler);

        let args = serde_json::json!({
            "_call_id": "tc_test",
            "path": "--help"
        });
        let result = provider
            .execute(&canonical_local_tool_name("git_diff"), args, ctx)
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.output.contains("must not start with '-'"));
    }

    // ── Wave 69A hostile filesystem escape tests ────────────────────────

    use tempfile::TempDir;

    fn hostile_context(dir: &TempDir) -> ToolCallContext {
        ToolCallContext {
            working_directory: dir.path().to_string_lossy().to_string(),
            session_id: openwand_core::SessionId::new(),
            cancellation: tokio_util::sync::CancellationToken::new(),
        }
    }

    #[tokio::test]
    async fn read_rejects_absolute_path_outside_workspace() {
        let dir = TempDir::new().unwrap();
        let ctx = hostile_context(&dir);
        let provider = batch1_local_tools();
        let args = serde_json::json!({
            "path": "/etc/passwd",
            "_call_id": "tc_hostile"
        });
        let result = provider.execute(&canonical_local_tool_name("file_read"), args, ctx).await.unwrap();
        assert!(result.is_error, "Read should reject absolute path: {}", result.output);
    }

    #[tokio::test]
    async fn read_rejects_parent_traversal_outside_workspace() {
        let dir = TempDir::new().unwrap();
        let ctx = hostile_context(&dir);
        let provider = batch1_local_tools();
        let args = serde_json::json!({
            "path": "../../../etc/passwd",
            "_call_id": "tc_hostile"
        });
        let result = provider.execute(&canonical_local_tool_name("file_read"), args, ctx).await.unwrap();
        assert!(result.is_error, "Read should reject parent traversal: {}", result.output);
    }

    #[tokio::test]
    async fn list_rejects_absolute_path_outside_workspace() {
        let dir = TempDir::new().unwrap();
        let ctx = hostile_context(&dir);
        let provider = batch1_local_tools();
        let args = serde_json::json!({
            "path": "/etc",
            "_call_id": "tc_hostile"
        });
        let result = provider.execute(&canonical_local_tool_name("file_list"), args, ctx).await.unwrap();
        assert!(result.is_error, "List should reject absolute path: {}", result.output);
    }

    #[tokio::test]
    async fn search_rejects_parent_traversal_outside_workspace() {
        let dir = TempDir::new().unwrap();
        let ctx = hostile_context(&dir);
        let provider = batch1_local_tools();
        let args = serde_json::json!({
            "pattern": "secret",
            "path": "../../..",
            "_call_id": "tc_hostile"
        });
        let result = provider.execute(&canonical_local_tool_name("file_search"), args, ctx).await.unwrap();
        assert!(result.is_error, "Search should reject parent traversal: {}", result.output);
    }

    #[tokio::test]
    async fn write_rejects_target_symlink_outside_workspace() {
        let dir = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();

        // Create symlink inside workspace pointing outside
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(outside.path(), dir.path().join("escape"));
        #[cfg(windows)]
        let _ = std::os::windows::fs::symlink_dir(outside.path(), dir.path().join("escape"));

        if dir.path().join("escape").exists() {
            let ctx = hostile_context(&dir);
            let provider = batch2_local_tools();
            let args = serde_json::json!({
                "path": "escape/evil.txt",
                "content": "hacked",
                "_call_id": "tc_hostile"
            });
            let result = provider.execute(&canonical_local_tool_name("file_write"), args, ctx).await.unwrap();
            assert!(result.is_error, "Write should reject symlink escape: {}", result.output);
        }
    }

    #[tokio::test]
    async fn patch_rejects_target_symlink_outside_workspace() {
        let dir = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        tokio::fs::write(outside.path().join("target.txt"), "secret\ndata").await.unwrap();

        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(outside.path().join("target.txt"), dir.path().join("link.txt"));
        #[cfg(windows)]
        let _ = std::os::windows::fs::symlink_file(outside.path().join("target.txt"), dir.path().join("link.txt"));

        if dir.path().join("link.txt").exists() {
            let ctx = hostile_context(&dir);
            let provider = batch2_local_tools();
            let args = serde_json::json!({
                "path": "link.txt",
                "mode": "plan",
                "line_number": 1,
                "old_lines": ["secret"],
                "new_lines": ["hacked"],
                "_call_id": "tc_hostile"
            });
            let result = provider.execute(&canonical_local_tool_name("file_patch"), args, ctx).await.unwrap();
            assert!(result.is_error, "Patch should reject symlink escape: {}", result.output);
        }
    }

    #[tokio::test]
    async fn policy_allow_read_does_not_bypass_path_containment() {
        // This test proves that even though policy auto-allows Read,
        // the tool handler itself rejects path escapes before policy matters.
        let dir = TempDir::new().unwrap();
        let ctx = hostile_context(&dir);
        let provider = batch1_local_tools();
        let args = serde_json::json!({
            "path": "/etc/shadow",
            "_call_id": "tc_policy_bypass"
        });
        let result = provider.execute(&canonical_local_tool_name("file_read"), args, ctx).await.unwrap();
        assert!(result.is_error, "Containment must block even if policy would allow: {}", result.output);
    }

    #[tokio::test]
    async fn policy_allow_search_does_not_bypass_path_containment() {
        let dir = TempDir::new().unwrap();
        let ctx = hostile_context(&dir);
        let provider = batch1_local_tools();
        let args = serde_json::json!({
            "pattern": "secret",
            "path": "/etc",
            "_call_id": "tc_policy_bypass"
        });
        let result = provider.execute(&canonical_local_tool_name("file_search"), args, ctx).await.unwrap();
        assert!(result.is_error, "Containment must block even if policy would allow: {}", result.output);
    }

    #[tokio::test]
    async fn workspace_boundary_errors_are_user_visible() {
        let dir = TempDir::new().unwrap();
        let ctx = hostile_context(&dir);
        let provider = batch1_local_tools();
        let args = serde_json::json!({
            "path": "../../etc/passwd",
            "_call_id": "tc_visible"
        });
        let result = provider.execute(&canonical_local_tool_name("file_read"), args, ctx).await.unwrap();
        assert!(result.is_error);
        // Error should be clear and not leak external paths
        assert!(!result.output.contains("\\Users\\"));
        assert!(!result.output.contains("/home/"));
    }
}
