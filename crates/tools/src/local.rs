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

/// Create the standard batch-1 set of local tools.
pub fn batch1_local_tools() -> BuiltinToolProvider {
    let mut provider = BuiltinToolProvider::new();
    provider.register_fn(file_read_descriptor(), file_read_handler);
    provider.register_fn(file_list_descriptor(), file_list_handler);
    provider.register_fn(file_search_descriptor(), file_search_handler);
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
    fn batch1_local_tools_have_read_or_search_effect() {
        let provider = batch1_local_tools();
        for tool in provider.available_descriptors() {
            assert!(
                matches!(tool.declared_effect, ToolEffect::Read | ToolEffect::Search),
                "tool {} has unexpected effect {:?}",
                tool.name,
                tool.declared_effect
            );
        }
    }
}
