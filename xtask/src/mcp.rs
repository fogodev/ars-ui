//! MCP stdio server with dynamic tool dispatch.
//!
//! Exposes all registered [`crate::tool::Tool`] implementations over the
//! [Model Context Protocol](https://modelcontextprotocol.io/) using JSON-RPC
//! over stdin/stdout.

use std::{future, sync::Arc};

/// Alias the rmcp protocol type to avoid confusion with our [`crate::tool::Tool`] trait.
use rmcp::model::Tool as McpTool;
use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext,
    transport::stdio,
};
use serde_json::json;

use crate::{manifest::SpecRoot, tool::ToolRegistry};

/// MCP server backed by a [`ToolRegistry`].
///
/// Implements the [`ServerHandler`] trait so that every tool registered in the
/// registry is automatically exposed as an MCP tool.
struct McpServer {
    /// The registry holding all available tools.
    registry: ToolRegistry,
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "ars-ui-xtask",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "ars-ui workspace tools. Use spec_* tools to navigate the 400+ file spec corpus.",
            )
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        let tools: Vec<McpTool> = self
            .registry
            .tools()
            .iter()
            .map(|t| {
                let schema = t.input_schema();
                let schema_obj: JsonObject = serde_json::from_value(schema).unwrap_or_default();
                McpTool::new(t.name().to_owned(), t.description().to_owned(), schema_obj)
            })
            .collect();
        future::ready(Ok(ListToolsResult::with_all_items(tools)))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, ErrorData>> + Send + '_ {
        let name = request.name.clone();
        let result = match self.registry.find(&name) {
            Some(tool) => {
                let input: serde_json::Value = request
                    .arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or(json!({}));
                match tool.execute(input) {
                    Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
                }
            }
            None => Err(ErrorData::invalid_params(
                format!("unknown tool: {name}"),
                None,
            )),
        };
        future::ready(result)
    }
}

/// Start the MCP stdio server.
///
/// Builds a [`ToolRegistry`] from the given [`SpecRoot`], then serves all
/// tools over stdin/stdout using the MCP protocol.
///
/// # Errors
///
/// Returns an error if the server fails to start or encounters a fatal
/// transport error.
pub async fn serve(root: Arc<SpecRoot>) -> anyhow::Result<()> {
    let mut registry = ToolRegistry::new();
    crate::spec::tools::register_all(&mut registry, &root);

    let server = McpServer { registry };
    let service = server
        .serve(stdio())
        .await
        .map_err(|e| anyhow::anyhow!("MCP server failed to start: {e}"))?;
    service
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
    Ok(())
}
