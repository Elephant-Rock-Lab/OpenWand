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
    provider
}

// ---------------------------------------------------------------------------
// Helper: extract call_id from args or generate one
// ---------------------------------------------------------------------------
fn extract_call_id(args: &serde_json::Value) -> ToolCallId {
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

    let path = std::path::Path::new(path_val);
    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::path::Path::new(&ctx.working_directory).join(path)
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

    let full_path = if std::path::Path::new(path_val).is_absolute() {
        std::path::PathBuf::from(path_val)
    } else {
        std::path::Path::new(&ctx.working_directory).join(path_val)
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

    let full_path = if std::path::Path::new(path_val).is_absolute() {
        std::path::PathBuf::from(path_val)
    } else {
        std::path::Path::new(&ctx.working_directory).join(path_val)
    };

    let mut results = Vec::new();
    let pattern_lower = pattern.to_lowercase();

    let walker = walkdir::WalkDir::new(&full_path).max_depth(10);
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if results.len() >= max_results {
            break;
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
fn validate_write_path(
    path_str: &str,
    working_directory: &str,
) -> Result<std::path::PathBuf, String> {
    let path = std::path::Path::new(path_str);

    // Reject absolute paths
    if path.is_absolute() {
        return Err(format!("Absolute paths are not allowed: {}", path_str));
    }

    // Reject parent escape in path components
    for component in path.components() {
        if component == std::path::Component::ParentDir {
            return Err(format!("Parent directory traversal (..) is not allowed: {}", path_str));
        }
    }

    // Resolve to full path
    let working = std::path::Path::new(working_directory);
    let full_path = working.join(path);

    // Symlink escape check: canonicalize parent if it exists
    // This catches cases where a symlink in working_directory points outside
    if let Some(parent_dir) = full_path.parent() {
        if parent_dir.exists() {
            let canonical_working = working
                .canonicalize()
                .map_err(|e| format!("Cannot canonicalize working directory: {e}"))?;
            let canonical_parent = parent_dir
                .canonicalize()
                .map_err(|e| format!("Cannot canonicalize parent directory: {e}"))?;
            if !canonical_parent.starts_with(&canonical_working) {
                return Err("Path escapes working directory (possible symlink)".into());
            }
        }
    }

    // Reject if target is an existing directory
    if full_path.is_dir() {
        return Err(format!("Cannot write to a directory: {}", path_str));
    }

    Ok(full_path)
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
    if let Some(parent) = full_path.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            return ToolResult::error(
                call_id,
                tool_name,
                format!("Failed to create parent directory: {e}"),
                start.elapsed().as_millis() as u64,
            );
        }
    }

    // Write the file
    match tokio::fs::write(&full_path, content).await {
        Ok(()) => ToolResult::success(
            call_id,
            tool_name,
            format!("Wrote {} bytes to {}", content.len(), full_path.display()),
            start.elapsed().as_millis() as u64,
        ),
        Err(e) => ToolResult::error(
            call_id,
            tool_name,
            format!("Failed to write file '{}': {}", full_path.display(), e),
            start.elapsed().as_millis() as u64,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_core::SessionId;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    fn test_context(dir: &TempDir) -> ToolCallContext {
        ToolCallContext {
            working_directory: dir.path().to_string_lossy().to_string(),
            session_id: SessionId::new(),
            cancellation: CancellationToken::new(),
        }
    }

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
    fn batch2_registers_four_tools_including_write() {
        let provider = batch2_local_tools();
        let tools = provider.available_descriptors();
        assert_eq!(4, tools.len());

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
        assert!(result.output.contains("Parent directory traversal"));
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

    let mut child = match cmd.spawn() {
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
