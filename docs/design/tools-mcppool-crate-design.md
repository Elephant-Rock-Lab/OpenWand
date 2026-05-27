# OpenWand Tools + MCP Pool Crate Design

**Date:** 2026-05-26  
**Status:** Design — locked  
**Crates:** `openwand-tools`, `openwand-mcp-pool`  
**Depends on:** `openwand-core`, `rmcp`  
**Blocks:** Batch 1 tool execution  

---

## North Star

> LLM sees OpenWand tool names. Policy evaluates OpenWand descriptors. Session calls OpenWand ToolExecutor. MCP is only one backend.

Two crates, one seam. Session depends on `openwand-tools` only. The app wires `openwand-mcp-pool` into the composite executor. Session never sees MCP.

```
openwand-session
  → openwand-tools::ToolExecutor

openwand-tools
  → openwand-core (ToolEffect, IDs)
  → openwand-mcp-pool (McpToolGateway)

openwand-mcp-pool
  → openwand-core
  → rmcp

openwand-app
  → wires McpServerPool into OpenWandToolExecutor
  → wires ToolExecutor into Session
```

No rmcp types escape `openwand-mcp-pool`. No `openwand-tools` types leak downward into `openwand-mcp-pool`.

---

## Crate Boundaries

### openwand-tools

| Contains | Does NOT contain |
|---|---|
| `ToolExecutor` trait | rmcp dependency |
| `ToolDef` — full tool descriptor | MCP server lifecycle |
| `ToolResult` — execution result | Policy evaluation |
| `ToolEffect` resolution from annotations | LLM client |
| `LocalToolRegistry` — built-in tools | Session loop |
| `CompositeToolExecutor` — local + MCP dispatch | Loro, trace, memory |
| Tool result normalization + truncation | |
| Canonical name resolution | |
| `McpDiscoveredTool` → `ToolDef` conversion | |

### openwand-mcp-pool

| Contains | Does NOT contain |
|---|---|
| `McpToolGateway` trait (what tools crate calls) | `ToolExecutor` trait |
| `McpServerPool` — manages all connections | `ToolDef` |
| `McpServerConfig` — server configuration | `ToolAnnotations` |
| `McpServerConnection` — one live server | Policy types |
| `McpServerRunner` — wraps rmcp Peer | Local tools |
| `McpDiscoveredTool` — pool's own DTO | LLM, Loro, trace |
| `McpToolAnnotations` — pool's own DTO | Session loop |
| Server lifecycle: start/stop/reconnect | |
| Tool discovery from MCP servers | |
| Health checking | |
| Dynamic tool refresh on notifications | |
| Server config persistence | |

### Dependency Rules

```text
openwand-session  depends on  openwand-tools        (NOT mcp-pool)
openwand-tools    depends on  openwand-core, openwand-mcp-pool
openwand-mcp-pool depends on  openwand-core, rmcp    (NOT tools)
openwand-policy   depends on  openwand-core          (NOT tools)
openwand-app      wires       mcp-pool → tools → session
```

`ToolEffect` lives in `openwand-core` (not in policy) so that `openwand-tools` does not need to depend on the policy crate.

---

## Crate Layout

### openwand-tools

```
openwand-tools/
  Cargo.toml
  src/
    lib.rs
    executor.rs          — ToolExecutor trait
    descriptor.rs        — ToolDef, ToolSource, ToolAnnotations
    result.rs            — ToolResult, ToolCallContext
    registry.rs          — ToolRegistry: aggregates local + MCP tools
    composite.rs         — CompositeToolExecutor: the main implementation
    local.rs             — LocalTool trait + LocalToolRegistry
    normalize.rs         — result normalization, truncation
    naming.rs            — canonical name generation and resolution
    effect.rs            — resolve_declared_effect (annotations as hints)
    error.rs             — ToolError
    local_tools/
      mod.rs
      read_file.rs       — read_file tool (Batch 1)
      search_files.rs    — search_files tool (Batch 1)
      list_directory.rs  — list_directory tool (Batch 1)
```

### openwand-mcp-pool

```
openwand-mcp-pool/
  Cargo.toml
  src/
    lib.rs
    gateway.rs           — McpToolGateway trait
    pool.rs              — McpServerPool: manages all connections
    server.rs            — McpServerConnection + lifecycle state machine
    runner.rs            — McpServerRunner: wraps rmcp Peer
    config.rs            — McpServerConfig, McpTransportConfig
    discovered.rs        — McpDiscoveredTool, McpToolAnnotations (pool's own DTOs)
    call.rs              — tools/call normalization
    notifications.rs     — ClientHandler impl for tool_list_changed
    health.rs            — health checking, reconnection
    persistence.rs       — save/load server configs
    error.rs             — McpPoolError
```

---

## Dependencies

### openwand-tools

```toml
[package]
name = "openwand-tools"
version.workspace = true
edition.workspace = true

[dependencies]
openwand-core = { path = "../core" }
openwand-mcp-pool = { path = "../mcp-pool" }

async-trait = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
tokio = { workspace = true, features = ["fs", "process", "sync"] }
tracing = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true, features = ["serde"] }
walkdir = "2"
ignore = "0.4"          # .gitignore-aware file walking
```

### openwand-mcp-pool

```toml
[package]
name = "openwand-mcp-pool"
version.workspace = true
edition.workspace = true

[dependencies]
openwand-core = { path = "../core" }

rmcp = { version = "=1.7.0", default-features = false, features = [
    "client",
    "transport-child-process",
    "transport-worker",
] }

async-trait = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
tokio = { workspace = true, features = ["sync", "time", "process"] }
tracing = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true, features = ["serde"] }

[features]
default = ["stdio"]
stdio = []
http = ["rmcp/transport-streamable-http-client"]
```

Batch 1 enables `stdio` only. HTTP is a feature flag for Batch 2.

---

## Core Types (openwand-core addition)

### ToolEffect

Lives in `openwand-core` so it's available to tools, policy, and mcp-pool without cross-dependencies:

```rust
// In openwand-core/src/tool_vocab.rs (new file)

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolEffect {
    Read,
    Search,
    Write,
    Delete,
    Execute,
    Network,
    Git,
    DependencyChange,
    PolicyChange,
    PersistenceChange,
    AuthChange,
    Unknown,
}
```

This is the same enum already in the policy crate design. Moving it to core avoids a tools → policy dependency.

---

## ToolDef

```rust
// openwand-tools/src/descriptor.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    /// Canonical name exposed to the LLM and policy.
    /// Examples: "local__read_file", "mcp__github__get_issue"
    pub name: String,

    /// Human-readable display name for UI.
    pub display_name: Option<String>,

    /// Tool description sent to the LLM.
    pub description: String,

    /// JSON Schema for tool parameters.
    pub parameters_schema: serde_json::Value,

    /// Optional JSON Schema for tool output.
    pub output_schema: Option<serde_json::Value>,

    /// Where this tool comes from.
    pub source: ToolSource,

    /// Declared side effect. Used by policy for risk assessment.
    /// Resolved via precedence chain (not raw MCP annotations).
    pub declared_effect: ToolEffect,

    /// Additional risk hints for policy evaluation.
    pub risk_hints: Vec<String>,

    /// Tags for filtering and categorization.
    pub tags: Vec<String>,

    /// Original MCP annotations (if from MCP server). Hints only, not authority.
    pub annotations: Option<ToolAnnotations>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnnotations {
    pub read_only_hint: Option<bool>,
    pub destructive_hint: Option<bool>,
    pub idempotent_hint: Option<bool>,
    pub open_world_hint: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolSource {
    Local,
    Mcp {
        server: String,
        remote_name: String,
    },
}
```

---

## Canonical Tool Names

```rust
// openwand-tools/src/naming.rs

/// Generate a canonical tool name from source.
pub fn canonical_tool_name(source: &ToolSource, local_name: &str) -> String {
    match source {
        ToolSource::Local => format!("local__{}", local_name),
        ToolSource::Mcp { server, .. } => format!("mcp__{}__{}", server, local_name),
    }
}

/// Parse a canonical name back into components.
pub fn parse_canonical_name(name: &str) -> Option<(String, Option<String>)> {
    let parts: Vec<&str> = name.splitn(3, "__").collect();
    match parts.as_slice() {
        ["local", tool_name] => Some((tool_name.to_string(), None)),
        ["mcp", rest] => {
            let subparts: Vec<&str> = rest.splitn(2, "__").collect();
            match subparts.as_slice() {
                [server, remote] => Some((remote.to_string(), Some(server.to_string()))),
                _ => None,
            }
        }
        _ => None,
    }
}
```

---

## Declared Effect Resolution

MCP annotations are hints, not authority. The precedence chain:

```rust
// openwand-tools/src/effect.rs

/// Resolve the declared effect for a tool.
///
/// Precedence:
/// 1. Built-in local tool declaration (authoritative)
/// 2. Per-tool config override from mcp_servers config
/// 3. Per-server config default
/// 4. MCP annotations as hints (only if server trust allows)
/// 5. ToolEffect::Unknown (blocked by Batch 1 policy)
pub fn resolve_declared_effect(
    source: &ToolSource,
    annotations: Option<&ToolAnnotations>,
    config_overrides: &ToolEffectOverrides,
) -> ToolEffect {
    match source {
        ToolSource::Local => {
            // Local tools declare their own effect — authoritative
            // This is set directly in the local tool definition
            unreachable!("Local tools set declared_effect directly")
        }
        ToolSource::Mcp { server, remote_name } => {
            // 2. Per-tool config override
            if let Some(effect) = config_overrides.tool_effect(server, remote_name) {
                return effect.clone();
            }

            // 3. Per-server config default
            if let Some(effect) = config_overrides.server_default(server) {
                return effect.clone();
            }

            // 4. MCP annotations as hints
            if let Some(ann) = annotations {
                if ann.read_only_hint.unwrap_or(false) {
                    return ToolEffect::Read;
                }
                if ann.destructive_hint.unwrap_or(false) {
                    return ToolEffect::Delete;
                }
                if ann.open_world_hint.unwrap_or(false) {
                    return ToolEffect::Network;
                }
            }

            // 5. Unknown
            ToolEffect::Unknown
        }
    }
}

/// Configuration overrides for tool effects.
/// Loaded from mcp_servers config.
pub struct ToolEffectOverrides {
    server_defaults: HashMap<String, ToolEffect>,
    tool_overrides: HashMap<(String, String), ToolEffect>,  // (server, tool_name) → effect
}
```

Example config:

```toml
[[mcp.servers]]
name = "github"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]

[mcp.servers.default_effect]
effect = "Read"

[mcp.servers.tool_effects]
"search_repositories" = "Search"
"get_issue" = "Read"
"create_issue" = "Network"
"update_file" = "Write"
```

---

## ToolExecutor Trait

```rust
// openwand-tools/src/executor.rs

/// The trait openwand-session calls. Infallible at the boundary —
/// tool failures are normal agent-loop events, not Rust control flow.
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// List all available tools with their descriptors.
    fn available_tools(&self) -> Vec<ToolDef>;

    /// Get a single tool descriptor by canonical name.
    fn get_descriptor(&self, name: &str) -> Option<ToolDef>;

    /// Execute a tool call. Always returns ToolResult — never errors.
    /// Tool failures become ToolResult { is_error: true, ... }.
    async fn execute(
        &self,
        call: &ToolCall,
        context: &ToolCallContext,
    ) -> ToolResult;

    /// Refresh MCP tool discovery from all connected servers.
    async fn refresh_mcp_tools(&self) -> Result<ToolRefreshReport, ToolError>;
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: ToolCallId,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct ToolRefreshReport {
    pub servers_checked: u32,
    pub tools_added: u32,
    pub tools_removed: u32,
    pub errors: Vec<String>,
}
```

---

## ToolResult

```rust
// openwand-tools/src/result.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
    pub output: String,
    pub is_error: bool,
    pub duration_ms: u64,
    pub truncated: bool,
    pub original_size: Option<usize>,
}

impl ToolResult {
    /// Create a success result with normalization.
    pub fn success(
        tool_call_id: ToolCallId,
        tool_name: String,
        raw_output: String,
        duration_ms: u64,
    ) -> Self {
        let (output, truncated, original_size) = normalize_output(raw_output, &tool_name);
        Self {
            tool_call_id,
            tool_name,
            output,
            is_error: false,
            duration_ms,
            truncated,
            original_size,
        }
    }

    /// Create an error result.
    pub fn error(
        tool_call_id: ToolCallId,
        tool_name: String,
        error_message: String,
        duration_ms: u64,
    ) -> Self {
        Self {
            tool_call_id,
            tool_name,
            output: error_message,
            is_error: true,
            duration_ms,
            truncated: false,
            original_size: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolCallContext {
    pub working_directory: String,
    pub session_id: SessionId,
    pub cancellation: CancellationToken,
}
```

---

## Tool Registry

```rust
// openwand-tools/src/registry.rs

pub struct ToolRegistry {
    local_tools: HashMap<String, LocalToolEntry>,
    mcp_tools: RwLock<HashMap<String, McpToolEntry>>,
}

struct LocalToolEntry {
    def: ToolDef,
    handler: Arc<dyn LocalToolHandler>,
}

struct McpToolEntry {
    def: ToolDef,
    server_name: String,
    remote_name: String,
}

pub enum ToolLookup<'a> {
    Local(&'a Arc<dyn LocalToolHandler>),
    Mcp { server_name: String, remote_name: String },
}

impl ToolRegistry {
    pub fn new() -> Self { ... }

    /// Register a local tool.
    pub fn register_local(&mut self, handler: Arc<dyn LocalToolHandler>) {
        let def = handler.definition();
        let name = def.name.clone();
        self.local_tools.insert(name, LocalToolEntry { def, handler });
    }

    /// Update MCP tools from pool discovery.
    /// Replaces all tools for the given server.
    pub fn update_mcp_tools(&self, server_name: &str, tools: Vec<ToolDef>) {
        let mut mcp = self.mcp_tools.write().unwrap();

        // Remove old tools from this server
        mcp.retain(|_, entry| entry.server_name != server_name);

        // Add new tools
        for def in tools {
            let remote_name = match &def.source {
                ToolSource::Mcp { remote_name, .. } => remote_name.clone(),
                _ => continue,
            };
            let name = def.name.clone();
            mcp.insert(name, McpToolEntry {
                def,
                server_name: server_name.to_string(),
                remote_name,
            });
        }
    }

    /// Get all available tools.
    pub fn all_tools(&self) -> Vec<ToolDef> {
        let mut tools: Vec<ToolDef> = self.local_tools.values()
            .map(|e| e.def.clone())
            .collect();
        tools.extend(self.mcp_tools.read().unwrap().values().map(|e| e.def.clone()));
        tools
    }

    /// Look up a tool by canonical name.
    pub fn get(&self, name: &str) -> Option<ToolLookup<'_>> {
        if let Some(entry) = self.local_tools.get(name) {
            return Some(ToolLookup::Local(&entry.handler));
        }
        if let Some(entry) = self.mcp_tools.read().unwrap().get(name) {
            return Some(ToolLookup::Mcp {
                server_name: entry.server_name.clone(),
                remote_name: entry.remote_name.clone(),
            });
        }
        None
    }

    /// Get a single tool descriptor.
    pub fn get_descriptor(&self, name: &str) -> Option<ToolDef> {
        if let Some(entry) = self.local_tools.get(name) {
            return Some(entry.def.clone());
        }
        if let Some(entry) = self.mcp_tools.read().unwrap().get(name) {
            return Some(entry.def.clone());
        }
        None
    }
}
```

---

## Local Tools

### LocalToolHandler Trait

```rust
// openwand-tools/src/local.rs

#[async_trait]
pub trait LocalToolHandler: Send + Sync {
    /// Return the tool definition with declared_effect set authoritatively.
    fn definition(&self) -> ToolDef;

    /// Execute the tool. Returns raw output string on success.
    async fn execute(
        &self,
        args: serde_json::Value,
        context: &ToolCallContext,
    ) -> Result<String, ToolError>;
}
```

### Batch 1 Local Tools

| Tool | Effect | Description |
|---|---|---|
| `read_file` | Read | Read file contents with line range support |
| `search_files` | Search | Search files by content (regex) with .gitignore awareness |
| `list_directory` | Read | List directory contents |

### read_file Sketch

```rust
pub struct ReadFileTool;

#[async_trait]
impl LocalToolHandler for ReadFileTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "local__read_file".into(),
            display_name: Some("Read File".into()),
            description: "Read the contents of a file. Returns file content with line numbers.".into(),
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the file" },
                    "offset": { "type": "integer", "description": "Start line (1-indexed, optional)" },
                    "limit": { "type": "integer", "description": "Max lines to read (optional)" }
                },
                "required": ["path"]
            }),
            output_schema: None,
            source: ToolSource::Local,
            declared_effect: ToolEffect::Read,  // Authoritative for local tools
            risk_hints: vec![],
            tags: vec!["filesystem".into()],
            annotations: Some(ToolAnnotations {
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(true),
                open_world_hint: Some(false),
            }),
        }
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        context: &ToolCallContext,
    ) -> Result<String, ToolError> {
        let path = args["path"].as_str().ok_or(ToolError::MissingArgument("path"))?;
        let offset = args["offset"].as_u64().map(|n| n as usize);
        let limit = args["limit"].as_u64().map(|n| n as usize);

        let full_path = std::path::Path::new(&context.working_directory).join(path);
        let canonical = full_path.canonicalize()
            .map_err(|_| ToolError::FileNotFound(path.into()))?;
        let workdir = std::path::Path::new(&context.working_directory).canonicalize()
            .map_err(|e| ToolError::IoError(e.to_string()))?;

        if !canonical.starts_with(&workdir) {
            return Err(ToolError::PathEscapesWorkspace(path.into()));
        }

        let content = tokio::fs::read_to_string(&canonical).await
            .map_err(|e| ToolError::IoError(e.to_string()))?;

        let lines: Vec<&str> = content.lines().collect();
        let start = offset.unwrap_or(1).saturating_sub(1);
        let end = limit
            .map(|l| (start + l).min(lines.len()))
            .unwrap_or(lines.len());

        let result: Vec<String> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:>6} | {}", start + i + 1, line))
            .collect();

        Ok(result.join("\n"))
    }
}
```

---

## Tool Result Normalization

```rust
// openwand-tools/src/normalize.rs

const MAX_INLINE_CHARS: usize = 50_000;

pub fn normalize_output(raw: String, tool_name: &str) -> (String, bool, Option<usize>) {
    let original_size = raw.len();
    if raw.len() <= MAX_INLINE_CHARS {
        return (raw, false, None);
    }

    let truncated = format!(
        "{}\n\n... [truncated {} chars, original size: {} chars]",
        &raw[..MAX_INLINE_CHARS],
        raw.len() - MAX_INLINE_CHARS,
        raw.len(),
    );
    (truncated, true, Some(original_size))
}
```

---

## McpToolGateway Trait

The interface `openwand-tools` uses to talk to `openwand-mcp-pool`. Pool implements this trait. No `ToolDef` or `ToolAnnotations` in this interface — only pool's own DTOs.

```rust
// openwand-mcp-pool/src/gateway.rs

/// The interface openwand-tools calls to dispatch MCP tool calls.
#[async_trait]
pub trait McpToolGateway: Send + Sync {
    /// Execute a tool call on a specific server.
    async fn execute_tool(
        &self,
        server_name: &str,
        remote_name: &str,
        arguments: serde_json::Value,
        cancellation: CancellationToken,
    ) -> Result<McpToolResult, McpPoolError>;

    /// Discover tools from all connected servers.
    /// Returns pool's own DTOs, not ToolDef.
    async fn discover_all_tools(
        &self,
    ) -> HashMap<String, Result<Vec<McpDiscoveredTool>, McpPoolError>>;

    /// Ensure all auto-start servers are running.
    async fn ensure_started(&self) -> Vec<McpPoolError>;

    /// Get health status for all servers.
    async fn health_check_all(&self) -> Vec<ServerHealth>;
}
```

---

## MCP Pool Types (pool's own DTOs)

### McpDiscoveredTool

```rust
// openwand-mcp-pool/src/discovered.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDiscoveredTool {
    pub remote_name: String,
    pub title: Option<String>,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub server_name: String,
    pub annotations: Option<McpToolAnnotations>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolAnnotations {
    pub read_only_hint: Option<bool>,
    pub destructive_hint: Option<bool>,
    pub idempotent_hint: Option<bool>,
    pub open_world_hint: Option<bool>,
}

impl McpDiscoveredTool {
    pub fn from_rmcp(tool: rmcp::model::Tool, server_name: &str) -> Self {
        let annotations = tool.annotations.as_ref().map(|a| McpToolAnnotations {
            read_only_hint: a.read_only_hint,
            destructive_hint: a.destructive_hint,
            idempotent_hint: a.idempotent_hint,
            open_world_hint: a.open_world_hint,
        });

        Self {
            remote_name: tool.name.to_string(),
            title: tool.title.clone(),
            description: tool.description
                .map(|d| d.to_string())
                .unwrap_or_default(),
            input_schema: tool.schema_as_json_value(),
            output_schema: tool.output_schema.map(|s| serde_json::Value::Object(s.as_ref().clone())),
            server_name: server_name.to_string(),
            annotations,
        }
    }
}
```

### McpToolResult

```rust
// openwand-mcp-pool/src/call.rs

#[derive(Debug, Clone)]
pub struct McpToolResult {
    pub output: String,
    pub is_error: bool,
}

/// Normalize rmcp Content into a plain string.
pub fn normalize_mcp_content(content: Vec<rmcp::model::Content>) -> String {
    let mut parts = Vec::new();
    for item in content {
        match item.raw {
            rmcp::model::RawContent::Text(text) => {
                parts.push(text.text);
            }
            rmcp::model::RawContent::Image(img) => {
                parts.push(format!("[image: {} base64]", img.mime_type));
            }
            rmcp::model::RawContent::Resource(res) => {
                match res.resource {
                    rmcp::model::ResourceContents::TextResourceContents { text, uri, .. } => {
                        parts.push(format!("{}: {}", uri, text));
                    }
                    rmcp::model::ResourceContents::BlobResourceContents { blob, uri, .. } => {
                        parts.push(format!("[binary resource: {}, {} bytes]", uri, blob.len()));
                    }
                }
            }
            rmcp::model::RawContent::Audio(audio) => {
                parts.push(format!("[audio: {} base64]", audio.mime_type));
            }
        }
    }
    parts.join("\n")
}
```

---

## MCP Server Config

```rust
// openwand-mcp-pool/src/config.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Unique name for this server (e.g., "filesystem", "github")
    pub name: String,

    /// How to connect
    pub transport: McpTransportConfig,

    /// Whether to auto-start on OpenWand launch
    pub auto_start: bool,

    /// Health check interval (seconds). None = no health check.
    pub health_check_interval: Option<u64>,

    /// Whether this server is enabled
    pub enabled: bool,

    /// Default ToolEffect for tools from this server when no per-tool override exists.
    /// None = infer from annotations or fall back to Unknown.
    pub default_effect: Option<ToolEffect>,

    /// Per-tool effect overrides. Key = remote tool name.
    pub tool_effects: HashMap<String, ToolEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpTransportConfig {
    /// Start a child process and communicate via stdio
    Stdio {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },

    /// Connect to an HTTP endpoint (Batch 2+)
    #[cfg(feature = "http")]
    Http {
        url: String,
        headers: HashMap<String, String>,
    },
}
```

---

## MCP Server Lifecycle

```rust
// openwand-mcp-pool/src/server.rs

pub struct McpServerConnection {
    name: String,
    config: McpServerConfig,
    state: RwLock<McpServerState>,
    runner: RwLock<Option<McpServerRunner>>,
    restart_count: AtomicU32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum McpServerState {
    Configured,        // Config exists, not started
    Starting,          // Process/transport launching
    Initializing,      // MCP handshake in progress
    Ready,             // Connected, tools available
    Degraded { reason: String },  // Connected but unhealthy
    Restarting,        // Reconnecting after failure
    Stopped,           // Intentionally stopped
}
```

Lifecycle:

```text
Configured → Starting → Initializing → Ready
                                       ↓ (health failure)
                                    Degraded → Restarting → Starting → ...
                                       ↓ (shutdown)
                                    Stopped
```

Stdio shutdown: close stdin, wait for exit, escalate to SIGTERM, then SIGKILL.

---

## MCP Server Pool

```rust
// openwand-mcp-pool/src/pool.rs

pub struct McpServerPool {
    configs: RwLock<HashMap<String, McpServerConfig>>,
    connections: RwLock<HashMap<String, McpServerConnection>>,
    tool_refresh_tx: mpsc::Sender<String>,
    tool_refresh_rx: Mutex<mpsc::Receiver<String>>,
}

#[async_trait]
impl McpToolGateway for McpServerPool {
    async fn execute_tool(
        &self,
        server_name: &str,
        remote_name: &str,
        arguments: serde_json::Value,
        cancellation: CancellationToken,
    ) -> Result<McpToolResult, McpPoolError> {
        let connections = self.connections.read().await;
        let conn = connections.get(server_name)
            .ok_or(McpPoolError::ServerNotFound(server_name.to_string()))?;

        let state = conn.state.read().await;
        if !matches!(*state, McpServerState::Ready) {
            return Err(McpPoolError::NotConnected(server_name.to_string()));
        }
        drop(state);

        let runner = conn.runner.read().await;
        let runner = runner.as_ref()
            .ok_or(McpPoolError::NotConnected(server_name.to_string()))?;

        let params = rmcp::model::CallToolRequestParams::new(remote_name)
            .with_arguments(
                serde_json::from_value(arguments).unwrap_or_default()
            );

        let result = tokio::select! {
            r = runner.peer().call_tool(params) => r,
            _ = cancellation.cancelled() => {
                return Err(McpPoolError::Cancelled);
            }
        }.map_err(|e| McpPoolError::ToolExecution(server_name.to_string(), e.to_string()))?;

        let output = normalize_mcp_content(result.content);

        Ok(McpToolResult {
            output,
            is_error: result.is_error.unwrap_or(false),
        })
    }

    async fn discover_all_tools(
        &self,
    ) -> HashMap<String, Result<Vec<McpDiscoveredTool>, McpPoolError>> {
        let mut results = HashMap::new();
        let connections = self.connections.read().await;
        for (name, conn) in connections.iter() {
            let state = conn.state.read().await;
            if !matches!(*state, McpServerState::Ready) {
                results.insert(name.clone(), Err(McpPoolError::NotConnected(name.clone())));
                continue;
            }
            drop(state);

            let runner = conn.runner.read().await;
            match runner.as_ref() {
                Some(r) => {
                    match r.list_all_tools().await {
                        Ok(tools) => {
                            let discovered: Vec<McpDiscoveredTool> = tools.into_iter()
                                .map(|t| McpDiscoveredTool::from_rmcp(t, name))
                                .collect();
                            results.insert(name.clone(), Ok(discovered));
                        }
                        Err(e) => {
                            results.insert(name.clone(), Err(McpPoolError::ToolDiscovery(
                                name.clone(), e.to_string(),
                            )));
                        }
                    }
                }
                None => {
                    results.insert(name.clone(), Err(McpPoolError::NotConnected(name.clone())));
                }
            }
        }
        results
    }

    async fn ensure_started(&self) -> Vec<McpPoolError> {
        let mut errors = Vec::new();
        let configs = self.configs.read().await;
        for (name, config) in configs.iter() {
            if config.auto_start && config.enabled {
                if let Err(e) = self.start_server(name).await {
                    errors.push(e);
                }
            }
        }
        errors
    }

    async fn health_check_all(&self) -> Vec<ServerHealth> {
        let mut results = Vec::new();
        let connections = self.connections.read().await;
        for (name, conn) in connections.iter() {
            let state = conn.state.read().await.clone();
            let healthy = match &state {
                McpServerState::Ready => conn.health_check().await.is_ok(),
                _ => false,
            };
            results.push(ServerHealth {
                server_name: name.clone(),
                state,
                healthy,
            });
        }
        results
    }
}

#[derive(Debug, Clone)]
pub struct ServerHealth {
    pub server_name: String,
    pub state: McpServerState,
    pub healthy: bool,
}
```

---

## McpServerRunner

Wraps the rmcp Peer:

```rust
// openwand-mcp-pool/src/runner.rs

pub struct McpServerRunner {
    peer: rmcp::service::Peer<rmcp::service::RoleClient>,
}

impl McpServerRunner {
    pub fn peer(&self) -> &rmcp::service::Peer<rmcp::service::RoleClient> {
        &self.peer
    }

    pub async fn list_all_tools(&self) -> Result<Vec<rmcp::model::Tool>, McpPoolError> {
        self.peer.list_all_tools().await
            .map_err(|e| McpPoolError::ToolDiscovery(String::new(), e.to_string()))
    }

    pub async fn health_check(&self) -> Result<(), McpPoolError> {
        self.peer.list_tools(None).await
            .map_err(|e| McpPoolError::HealthCheck(String::new(), e.to_string()))?;
        Ok(())
    }
}
```

---

## ClientHandler for Notifications

```rust
// openwand-mcp-pool/src/notifications.rs

pub struct OpenWandClientHandler {
    server_name: String,
    tool_refresh_tx: mpsc::Sender<String>,
}

impl rmcp::handler::client::ClientHandler for OpenWandClientHandler {
    fn get_info(&self) -> rmcp::model::ClientInfo {
        rmcp::model::ClientInfo::new(
            rmcp::model::ClientCapabilities::default(),
            rmcp::model::Implementation::new("openwand", env!("CARGO_PKG_VERSION")),
        )
    }

    async fn on_tool_list_changed(
        &self,
        _context: rmcp::service::NotificationContext<rmcp::service::RoleClient>,
    ) {
        tracing::info!(server = %self.server_name, "MCP server tool list changed");
        let _ = self.tool_refresh_tx.send(self.server_name.clone()).await;
    }

    async fn on_progress(
        &self,
        params: rmcp::model::ProgressNotificationParam,
        _context: rmcp::service::NotificationContext<rmcp::service::RoleClient>,
    ) {
        tracing::debug!(
            server = %self.server_name,
            progress_token = ?params.progress_token,
            progress = params.progress,
            total = ?params.total,
            "MCP tool progress"
        );
    }

    async fn on_logging_message(
        &self,
        params: rmcp::model::LoggingMessageNotificationParam,
        _context: rmcp::service::NotificationContext<rmcp::service::RoleClient>,
    ) {
        tracing::debug!(
            server = %self.server_name,
            level = ?params.level,
            "MCP server log"
        );
    }
}
```

---

## Conversion: Pool DTO → ToolDef

This happens in `openwand-tools`, not in `openwand-mcp-pool`:

```rust
// openwand-tools/src/descriptor.rs

impl ToolDef {
    /// Convert a discovered MCP tool into a ToolDef.
    /// Uses the effect resolution precedence chain.
    pub fn from_mcp_discovered(
        tool: McpDiscoveredTool,
        config: &McpServerConfig,
    ) -> Self {
        let canonical_name = format!("mcp__{}__{}", config.name, tool.remote_name);
        let remote_name = tool.remote_name.clone();
        let server_name = config.name.clone();

        let declared_effect = Self::resolve_mcp_effect(&config, &tool);

        Self {
            name: canonical_name,
            display_name: tool.title,
            description: tool.description,
            parameters_schema: tool.input_schema,
            output_schema: tool.output_schema,
            source: ToolSource::Mcp { server: server_name, remote_name },
            declared_effect,
            risk_hints: vec![],
            tags: vec![format!("mcp:{}", config.name)],
            annotations: tool.annotations.map(|a| ToolAnnotations {
                read_only_hint: a.read_only_hint,
                destructive_hint: a.destructive_hint,
                idempotent_hint: a.idempotent_hint,
                open_world_hint: a.open_world_hint,
            }),
        }
    }

    fn resolve_mcp_effect(
        config: &McpServerConfig,
        tool: &McpDiscoveredTool,
    ) -> ToolEffect {
        // 2. Per-tool config override
        if let Some(effect) = config.tool_effects.get(&tool.remote_name) {
            return effect.clone();
        }

        // 3. Per-server default
        if let Some(effect) = &config.default_effect {
            return effect.clone();
        }

        // 4. MCP annotations as hints
        if let Some(ann) = &tool.annotations {
            if ann.read_only_hint.unwrap_or(false) {
                return ToolEffect::Read;
            }
            if ann.destructive_hint.unwrap_or(false) {
                return ToolEffect::Delete;
            }
            if ann.open_world_hint.unwrap_or(false) {
                return ToolEffect::Network;
            }
        }

        // 5. Unknown
        ToolEffect::Unknown
    }
}
```

---

## CompositeToolExecutor

The main `ToolExecutor` implementation that session uses:

```rust
// openwand-tools/src/composite.rs

pub struct CompositeToolExecutor {
    registry: RwLock<ToolRegistry>,
    mcp_gateway: Arc<dyn McpToolGateway>,
    effect_overrides: ToolEffectOverrides,
}

#[async_trait]
impl ToolExecutor for CompositeToolExecutor {
    fn available_tools(&self) -> Vec<ToolDef> {
        self.registry.read().unwrap().all_tools()
    }

    fn get_descriptor(&self, name: &str) -> Option<ToolDef> {
        self.registry.read().unwrap().get_descriptor(name)
    }

    async fn execute(
        &self,
        call: &ToolCall,
        context: &ToolCallContext,
    ) -> ToolResult {
        let start = std::time::Instant::now();
        let registry = self.registry.read().unwrap();

        let lookup = match registry.get(&call.name) {
            Some(l) => l,
            None => {
                return ToolResult::error(
                    call.id.clone(),
                    call.name.clone(),
                    format!("Tool '{}' not found", call.name),
                    start.elapsed().as_millis() as u64,
                );
            }
        };

        match lookup {
            ToolLookup::Local(handler) => {
                match handler.execute(call.arguments.clone(), context).await {
                    Ok(raw_output) => ToolResult::success(
                        call.id.clone(),
                        call.name.clone(),
                        raw_output,
                        start.elapsed().as_millis() as u64,
                    ),
                    Err(e) => ToolResult::error(
                        call.id.clone(),
                        call.name.clone(),
                        e.to_string(),
                        start.elapsed().as_millis() as u64,
                    ),
                }
            }
            ToolLookup::Mcp { server_name, remote_name } => {
                // Drop registry lock before async MCP call
                drop(registry);

                match self.mcp_gateway.execute_tool(
                    &server_name,
                    &remote_name,
                    call.arguments.clone(),
                    context.cancellation.clone(),
                ).await {
                    Ok(mcp_result) => {
                        let (output, truncated, original_size) = normalize_output(
                            mcp_result.output,
                            &call.name,
                        );
                        ToolResult {
                            tool_call_id: call.id.clone(),
                            tool_name: call.name.clone(),
                            output,
                            is_error: mcp_result.is_error,
                            duration_ms: start.elapsed().as_millis() as u64,
                            truncated,
                            original_size,
                        }
                    }
                    Err(e) => ToolResult::error(
                        call.id.clone(),
                        call.name.clone(),
                        e.to_string(),
                        start.elapsed().as_millis() as u64,
                    ),
                }
            }
        }
    }

    async fn refresh_mcp_tools(&self) -> Result<ToolRefreshReport, ToolError> {
        let discovered = self.mcp_gateway.discover_all_tools().await;

        // We need configs for effect resolution — get them from the pool
        // TODO: pass configs into this method or store them
        let mut report = ToolRefreshReport::default();

        for (server_name, result) in discovered {
            report.servers_checked += 1;
            match result {
                Ok(tools) => {
                    // Convert pool DTOs to ToolDefs
                    // (needs server config for effect resolution)
                    let tool_defs: Vec<ToolDef> = tools.into_iter()
                        .map(|t| ToolDef::from_mcp_discovered(t, &/* config */))
                        .collect();
                    report.tools_added += tool_defs.len() as u32;
                    self.registry.read().unwrap()
                        .update_mcp_tools(&server_name, tool_defs);
                }
                Err(e) => {
                    report.errors.push(format!("{}: {}", server_name, e));
                }
            }
        }

        Ok(report)
    }
}
```

---

## MCP Pool Errors

```rust
// openwand-mcp-pool/src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum McpPoolError {
    #[error("Server '{0}' not found")]
    ServerNotFound(String),

    #[error("Server '{0}' is not connected")]
    NotConnected(String),

    #[error("Server '{0}' connection failed: {1}")]
    ConnectionFailed(String, String),

    #[error("Tool discovery failed on '{0}': {1}")]
    ToolDiscovery(String, String),

    #[error("Tool execution failed on '{0}': {1}")]
    ToolExecution(String, String),

    #[error("Health check failed on '{0}': {1}")]
    HealthCheck(String, String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Cancelled")]
    Cancelled,
}
```

---

## App Wiring

```rust
// In openwand-app (NOT in tools or mcp-pool crates)

async fn build_tool_executor(
    mcp_configs: Vec<McpServerConfig>,
) -> Arc<dyn ToolExecutor> {
    // 1. Create MCP pool
    let mcp_pool = Arc::new(McpServerPool::new(mcp_configs));
    mcp_pool.ensure_started().await;

    // 2. Create tool executor with local tools
    let mut registry = ToolRegistry::new();
    registry.register_local(Arc::new(ReadFileTool));
    registry.register_local(Arc::new(SearchFilesTool));
    registry.register_local(Arc::new(ListDirectoryTool));

    let executor = CompositeToolExecutor::new(registry, mcp_pool);

    // 3. Discover MCP tools
    executor.refresh_mcp_tools().await?;

    Arc::new(executor)
}

// Then in session construction:
let session = Session::new(
    /* ... */,
    tool_executor,  // Arc<dyn ToolExecutor>
    /* ... */,
);
```

---

## Session Integration

Session only sees `Arc<dyn ToolExecutor>`:

```rust
// In openwand-session

// Get tool manifest for LLM
let tool_defs = self.tools.available_tools();

// Convert to policy descriptors
let policy_descriptors: Vec<PolicyToolDescriptor> = tool_defs.iter()
    .map(|td| PolicyToolDescriptor {
        name: td.name.clone(),
        source: match &td.source {
            ToolSource::Local => PolicyToolSource::Local,
            ToolSource::Mcp { server, .. } => PolicyToolSource::Mcp { server: server.clone() },
        },
        declared_effect: td.declared_effect.clone(),
        risk_hints: td.risk_hints.clone(),
        tags: td.tags.clone(),
    })
    .collect();

// Convert to LLM tool definitions
let llm_tool_defs: Vec<LlmToolDef> = tool_defs.iter()
    .map(|td| LlmToolDef {
        name: td.name.clone(),
        description: td.description.clone(),
        parameters_schema: td.parameters_schema.clone(),
    })
    .collect();

// During tool execution (after policy approval):
let result = self.tools.execute(&call, &context).await;
// Always returns ToolResult — never errors
```

---

## Batch 1 Scope

| Aspect | Batch 1 | Later |
|---|---|---|
| Local tools | read_file, search_files, list_directory | write_file, shell, git, code_edit |
| MCP transport | **stdio only** | Streamable HTTP (Batch 2) |
| MCP servers | 0-2 configured servers | Unlimited, dynamic add/remove |
| Tool annotations | Hints → effect resolution chain | Per-argument risk modifiers |
| Tool refresh | On session start + manual | Automatic on notification |
| Result normalization | Truncation at 50K chars | Save to file reference |
| Health check | Basic (list_tools ping) | Heartbeat, auto-reconnect |
| Effect resolution | Config override → annotations → Unknown | Trusted server profiles |
| Canonical names | local__ / mcp__server__ | Configurable prefix format |

---

## Summary

| Decision | Locked |
|---|---|
| Two crates: tools (dispatch) + mcp-pool (lifecycle) | ✅ |
| Session depends only on tools, not mcp-pool | ✅ |
| App wires mcp-pool into CompositeToolExecutor | ✅ |
| rmcp types never escape mcp-pool | ✅ |
| tools types never leak into mcp-pool | ✅ |
| Pool returns own DTOs (McpDiscoveredTool) | ✅ |
| Tools crate converts pool DTOs to ToolDef | ✅ |
| ToolEffect in openwand-core (not policy) | ✅ |
| MCP annotations are hints, not authority | ✅ |
| Canonical names: local__ / mcp__server__ | ✅ |
| ToolExecutor::execute is infallible | ✅ |
| ToolSource::Mcp stores both server and remote_name | ✅ |
| Batch 1: stdio only, no HTTP | ✅ |

**Estimated LOC:**
- `openwand-tools`: ~1,400 LOC (trait + 3 tools + registry + composite + naming + effect)
- `openwand-mcp-pool`: ~900 LOC (pool + connection + runner + discovery + notifications)
- **Total: ~2,300 LOC**
