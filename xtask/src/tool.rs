//! Tool trait and registry for auto-exposure via MCP.

use serde_json::Value;

/// Error returned by tool execution.
#[derive(Debug)]
pub struct ToolError {
    /// Human-readable error message.
    pub message: String,
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ToolError {}

impl From<crate::manifest::Error> for ToolError {
    fn from(e: crate::manifest::Error) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

/// A tool that can be invoked via CLI or MCP.
///
/// Each tool has a unique name, a description for discovery, a JSON Schema
/// describing its parameters, and an execute method.
pub trait Tool: Send + Sync {
    /// Unique tool name (e.g., `"spec_info"`). Used for MCP dispatch.
    fn name(&self) -> &str;

    /// Description for MCP tool listing.
    fn description(&self) -> &str;

    /// JSON Schema for the tool's input parameters.
    fn input_schema(&self) -> Value;

    /// Execute the tool with JSON input, returning text output.
    ///
    /// # Errors
    ///
    /// Returns `ToolError` if execution fails.
    fn execute(&self, input: Value) -> Result<String, ToolError>;
}

/// Collects `Tool` implementations from all xtask modules.
#[derive(Default)]
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tool_count", &self.tools.len())
            .finish()
    }
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    /// Get all registered tools.
    pub fn tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }

    /// Find a tool by name.
    pub fn find(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.iter().find(|t| t.name() == name).map(|t| &**t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyTool;

    impl Tool for DummyTool {
        fn name(&self) -> &str {
            "dummy"
        }

        fn description(&self) -> &str {
            "A dummy tool"
        }

        fn input_schema(&self) -> Value {
            serde_json::json!({ "type": "object" })
        }

        fn execute(&self, _input: Value) -> Result<String, ToolError> {
            Ok("ok".to_string())
        }
    }

    #[test]
    fn registry_find() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(DummyTool));
        assert!(reg.find("dummy").is_some());
        assert!(reg.find("nonexistent").is_none());
    }

    #[test]
    fn registry_list() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(DummyTool));
        assert_eq!(reg.tools().len(), 1);
        assert_eq!(reg.tools()[0].name(), "dummy");
    }
}
