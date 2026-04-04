# Cargo xtask with MCP Spec Server

## Context

The ars-ui spec corpus is 401 files / 208K lines / 10.4 MB across components, foundation docs, adapter specs, shared types, and testing specs. The existing `spec-tool` CLI (690 lines, 9 commands) helps navigate this via manifest lookups, but agents (Claude Code, Codex) still need to shell out and read multiple files to understand a component's full context.

This design replaces `tools/spec-tool` with a `cargo xtask` crate that:
1. Preserves all existing spec-tool commands under `cargo xtask spec <cmd>`.
2. Adds three new capabilities: full-text search, component digests, and dependency-aware context loading.
3. Exposes all tools via an MCP (Model Context Protocol) stdio server started with `cargo xtask mcp`.
4. Provides a `Tool` trait so any future xtask module automatically becomes available as an MCP tool.

Primary consumers: Claude Code and Codex (LLM agents), not human developers or other editors.

## Architecture

### Crate layout

```
xtask/
├── Cargo.toml
├── src/
│   ├── main.rs           # CLI entry (clap): subcommand dispatch
│   ├── lib.rs            # Re-exports, SpecRoot construction
│   ├── tool.rs           # Tool trait + ToolRegistry
│   ├── mcp.rs            # MCP stdio server with dynamic tool dispatch
│   ├── manifest.rs       # Manifest, Component, Frontmatter types + parsing
│   ├── spec/
│   │   ├── mod.rs        # Spec subcommand CLI group + tool registration
│   │   ├── info.rs       # spec_info: component metadata
│   │   ├── deps.rs       # spec_deps: files needed for review
│   │   ├── category.rs   # spec_category: components in a category
│   │   ├── reverse.rs    # spec_reverse: components using a shared type
│   │   ├── related.rs    # spec_related: related components with deps
│   │   ├── profile.rs    # spec_profile: files for a review profile
│   │   ├── toc.rs        # spec_toc: heading structure with line numbers
│   │   ├── validate.rs   # spec_validate: frontmatter validation
│   │   ├── adapters.rs   # spec_adapters: adapter files by framework
│   │   ├── search.rs     # spec_search: keyword/regex content search (NEW)
│   │   ├── digest.rs     # spec_digest: component summary extraction (NEW)
│   │   └── context.rs    # spec_context: concatenated implementation context (NEW)
```

### Workspace integration

- `.cargo/config.toml`: `[alias] xtask = "run --package xtask --"`
- `tools/spec-tool/` is deleted after migration.
- CLAUDE.md references update from `cargo run -p spec-tool` to `cargo xtask spec`.

### Dependencies

```toml
[package]
name = "xtask"
edition = "2024"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "1"
regex = "1"

# MCP support (feature-gated)
rmcp = { version = "1.3", features = ["server", "transport-io"], optional = true }
tokio = { version = "1", features = ["macros", "rt-multi-thread"], optional = true }
schemars = { version = "1", optional = true }

[features]
default = ["mcp"]
mcp = ["dep:rmcp", "dep:tokio", "dep:schemars"]
```

`mcp` is a default feature since the primary purpose is the MCP server. It can be disabled with `--no-default-features` for a lighter CLI-only build.

## Tool Trait (Auto-Exposure System)

```rust
use serde_json::Value;

/// Error returned by tool execution.
pub struct ToolError {
    pub message: String,
}

/// A tool that can be invoked via CLI or MCP.
///
/// Each tool has a unique name, a description for discovery, a JSON Schema
/// describing its parameters, and an execute method.
pub trait Tool: Send + Sync {
    /// Unique tool name (e.g., "spec_info"). Used for MCP dispatch.
    fn name(&self) -> &str;

    /// Description shown in MCP tools/list responses.
    fn description(&self) -> &str;

    /// JSON Schema for the tool's input parameters.
    fn input_schema(&self) -> Value;

    /// Execute the tool. Input is a JSON object matching input_schema.
    /// Returns text output suitable for LLM consumption.
    fn execute(&self, input: Value) -> Result<String, ToolError>;
}

/// Collects Tool implementations from all xtask modules.
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self { ... }
    pub fn register(&mut self, tool: Box<dyn Tool>) { ... }
    pub fn tools(&self) -> &[Box<dyn Tool>] { ... }
    pub fn find(&self, name: &str) -> Option<&dyn Tool> { ... }
}
```

**Adding a new module to MCP**: implement `Tool` for each tool type, then add registration calls in the module's `register()` function. The MCP server picks them up automatically.

## MCP Server

### Transport

Stdio (stdin/stdout JSON-RPC). Logging goes to stderr.

### Dynamic dispatch

The MCP server implements `ServerHandler` directly (not via `#[tool_router]`) to support runtime tool discovery:

```rust
struct McpServer {
    registry: ToolRegistry,
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder().enable_tools().build()
        ).with_instructions("ars-ui workspace tools")
    }

    async fn list_tools(&self) -> Vec<ToolInfo> {
        // Iterates registry, returns ToolInfo per registered tool
    }

    async fn call_tool(&self, name: &str, input: Value) -> CallToolResult {
        // Looks up tool by name in registry, calls execute()
    }
}
```

### Configuration

Register in `.claude/settings.local.json`:

```json
{
  "mcpServers": {
    "xtask": {
      "command": "cargo",
      "args": ["xtask", "mcp"]
    }
  }
}
```

## Tool Specifications

### Existing tools (migrated from spec-tool)

#### spec_info
- **Input**: `{ "component": "checkbox" }`
- **Output**: Component metadata — category, foundation_deps, shared_deps, related, internal flag, adapter paths.

#### spec_deps
- **Input**: `{ "component": "checkbox" }`
- **Output**: All file paths needed to review a component — component spec, foundation deps, shared deps, category context, adapter examples, related components.

#### spec_category
- **Input**: `{ "category": "input" }`
- **Output**: All components in the category with their metadata and dependency lists.

#### spec_reverse
- **Input**: `{ "shared_type": "selection-patterns" }`
- **Output**: The shared type file path, then all components that depend on it.

#### spec_related
- **Input**: `{ "component": "checkbox" }`
- **Output**: The component plus all explicitly related components with their deps.

#### spec_profile
- **Input**: `{ "profile": "accessibility" }`
- **Output**: File paths in the named review profile.

#### spec_toc
- **Input**: `{ "file": "spec/components/input/checkbox.md" }`
- **Output**: Indented heading structure with line numbers.

#### spec_validate
- **Input**: `{}` (no parameters)
- **Output**: Validation results — count of validated files, any mismatches between frontmatter and manifest.

#### spec_adapters
- **Input**: `{ "framework": "leptos" }`
- **Output**: All adapter files for the framework, grouped by category.

### New tools

#### spec_search
- **Input**: `{ "query": "focus trap", "category": "overlay" (optional), "section": "accessibility" (optional), "tier": "complex" (optional) }`
- **Description**: Keyword/regex search across spec file content with optional filters.
- **Implementation**: Walks spec files matching the filters, searches line-by-line with regex. Returns matches with file path, line number, and surrounding context (3 lines before/after).
- **Section filter values**: `states`, `events`, `props`, `accessibility`, `interactions`, `api`, `testing`, `examples` — mapped to known heading patterns (e.g., "## 1. State Machine" for `states`, "## 2. Accessibility" for `accessibility`).
- **Output**: List of matches, each with `file`, `line`, `section`, and `context`.

#### spec_digest
- **Input**: `{ "component": "checkbox" }`
- **Description**: Pre-computed component summary extracting key sections.
- **Implementation**: Reads the component spec, parses by heading structure, extracts:
  - **States**: state enum values and descriptions (from ## 1. State Machine)
  - **Events**: event names and when they fire (from ## 1, subsection events)
  - **Props/API**: public props and their types (from ## 3. API or ## 4. API)
  - **Accessibility**: ARIA roles, keyboard interactions (from ## 2. Accessibility)
  - **Tier**: stateless / stateful / complex
  - **Dependencies**: foundation + shared deps with one-line description each
- **Output**: Structured text summary, one section per heading, compact enough to fit in an LLM prompt (~200-500 lines per component).

#### spec_context
- **Input**: `{ "component": "checkbox", "framework": "leptos" (optional), "include_testing": false (optional, default false) }`
- **Description**: Returns the full text needed to implement a component — the component spec plus all its dependencies, concatenated with file-boundary markers.
- **Implementation**: Uses manifest to resolve the dependency graph, reads files in order:
  1. Foundation deps (in declaration order)
  2. Shared deps
  3. Component spec
  4. Adapter spec for the chosen framework (if specified)
  5. Testing spec sections relevant to this component's tier (if `include_testing` is true)
- **Output**: Concatenated text with `--- FILE: <path> ---` markers between sections. Each file's content is included in full.
- **Note**: Output can be large (10K-50K lines for complex components with all deps). The tool returns the full content — the MCP client (Claude Code) handles context management.

## Migration Plan

1. Create `xtask/` crate with the new structure.
2. Move `tools/spec-tool/src/main.rs` logic into `xtask/src/manifest.rs` and `xtask/src/spec/*.rs`, refactoring print-based functions to return typed data.
3. Implement `Tool` for each spec command.
4. Implement the three new tools (search, digest, context).
5. Implement the MCP server with dynamic dispatch.
6. Add `.cargo/config.toml` alias.
7. Update CLAUDE.md references.
8. Delete `tools/spec-tool/`.
9. Register MCP server in `.claude/settings.local.json`.

## Verification

1. **CLI parity**: `cargo xtask spec info checkbox` produces equivalent output to `cargo run -p spec-tool -- info checkbox` for all 9 existing commands.
2. **Validation**: `cargo xtask spec validate` passes on the full 401-file spec corpus.
3. **MCP initialization**: `cargo xtask mcp` responds correctly to MCP `initialize` and `tools/list` requests.
4. **Tool execution**: Each of the 12 MCP tools returns correct results when called via MCP protocol.
5. **Integration**: Register in `.claude/settings.local.json` and verify tools appear in Claude Code's tool list.
6. **New tools smoke test**:
   - `spec_search` with `query: "SelectionChanged"` returns hits across multiple component specs.
   - `spec_digest` for `checkbox` returns states, events, props, accessibility sections.
   - `spec_context` for `checkbox` with `framework: "leptos"` returns concatenated content with file markers.
