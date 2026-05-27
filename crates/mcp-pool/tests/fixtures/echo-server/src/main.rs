//! OpenWand Echo MCP Server — test fixture for CI.
//!
//! Minimal stdio MCP server with two read-only tools:
//! - `echo_read`: returns "echo: {text}"
//! - `echo_list`: returns a fixed list of items

use rmcp::{
    tool, tool_router,
    handler::server::wrapper::{Json, Parameters},
    ServiceExt,
};
use serde::{Deserialize, Serialize};
use rmcp::schemars::JsonSchema;

struct EchoServer;

#[derive(Deserialize, schemars::JsonSchema, Default)]
struct EchoReadInput {
    text: String,
}

#[derive(Serialize, schemars::JsonSchema)]
struct EchoReadOutput {
    echo: String,
}

#[derive(Serialize, schemars::JsonSchema)]
struct EchoListOutput {
    items: Vec<String>,
}

#[tool_router(server_handler)]
impl EchoServer {
    #[tool(
        name = "echo_read",
        description = "Echoes back the input text with a prefix. Read-only tool for testing."
    )]
    fn echo_read(
        &self,
        Parameters(EchoReadInput { text }): Parameters<EchoReadInput>,
    ) -> Json<EchoReadOutput> {
        Json(EchoReadOutput {
            echo: format!("echo: {text}"),
        })
    }

    #[tool(
        name = "echo_list",
        description = "Returns a fixed list of items. Read-only tool for testing."
    )]
    fn echo_list(&self) -> Json<EchoListOutput> {
        Json(EchoListOutput {
            items: vec!["file1.rs".into(), "file2.rs".into(), "README.md".into()],
        })
    }
}

#[tokio::main]
async fn main() {
    let server = EchoServer;
    let transport = (tokio::io::stdin(), tokio::io::stdout());
    let running = server.serve(transport).await.expect("Server failed");
    // Wait for shutdown
    running.waiting().await.ok();
}
