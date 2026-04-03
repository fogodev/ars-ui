//! MCP tool implementations for spec commands.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::{
    manifest::SpecRoot,
    tool::{Tool, ToolError, ToolRegistry},
};

/// Register all spec tools into a registry.
pub fn register_all(registry: &mut ToolRegistry, root: &Arc<SpecRoot>) {
    registry.register(Box::new(InfoTool(Arc::clone(root))));
    registry.register(Box::new(DepsTool(Arc::clone(root))));
    registry.register(Box::new(CategoryTool(Arc::clone(root))));
    registry.register(Box::new(ReverseTool(Arc::clone(root))));
    registry.register(Box::new(RelatedTool(Arc::clone(root))));
    registry.register(Box::new(ProfileTool(Arc::clone(root))));
    registry.register(Box::new(TocTool(Arc::clone(root))));
    registry.register(Box::new(ValidateTool(Arc::clone(root))));
    registry.register(Box::new(AdaptersTool(Arc::clone(root))));
    registry.register(Box::new(SearchTool(Arc::clone(root))));
}

/// Extract a required string parameter from JSON input.
fn get_string(input: &Value, field: &str) -> Result<String, ToolError> {
    input
        .get(field)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| ToolError {
            message: format!("missing required parameter: {field}"),
        })
}

#[derive(Debug)]
struct InfoTool(Arc<SpecRoot>);

impl Tool for InfoTool {
    fn name(&self) -> &str {
        "spec_info"
    }

    fn description(&self) -> &str {
        "Get component metadata: category, deps, adapter paths"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "component": {
                    "type": "string",
                    "description": "Component name (e.g. checkbox, date-picker)"
                }
            },
            "required": ["component"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, ToolError> {
        Ok(super::info::execute(
            &self.0,
            &get_string(&input, "component")?,
        )?)
    }
}

#[derive(Debug)]
struct DepsTool(Arc<SpecRoot>);

impl Tool for DepsTool {
    fn name(&self) -> &str {
        "spec_deps"
    }

    fn description(&self) -> &str {
        "List all files needed to review a component"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "component": {
                    "type": "string",
                    "description": "Component name"
                }
            },
            "required": ["component"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, ToolError> {
        Ok(super::deps::execute(
            &self.0,
            &get_string(&input, "component")?,
        )?)
    }
}

#[derive(Debug)]
struct CategoryTool(Arc<SpecRoot>);

impl Tool for CategoryTool {
    fn name(&self) -> &str {
        "spec_category"
    }

    fn description(&self) -> &str {
        "List all components in a category with their metadata"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "description": "Category name (e.g. input, selection, overlay)"
                }
            },
            "required": ["category"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, ToolError> {
        Ok(super::category::execute(
            &self.0,
            &get_string(&input, "category")?,
        )?)
    }
}

#[derive(Debug)]
struct ReverseTool(Arc<SpecRoot>);

impl Tool for ReverseTool {
    fn name(&self) -> &str {
        "spec_reverse"
    }

    fn description(&self) -> &str {
        "Find all components depending on a shared type"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "shared_type": {
                    "type": "string",
                    "description": "Shared type name (e.g. selection-patterns, date-time-types)"
                }
            },
            "required": ["shared_type"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, ToolError> {
        Ok(super::reverse::execute(
            &self.0,
            &get_string(&input, "shared_type")?,
        )?)
    }
}

#[derive(Debug)]
struct RelatedTool(Arc<SpecRoot>);

impl Tool for RelatedTool {
    fn name(&self) -> &str {
        "spec_related"
    }

    fn description(&self) -> &str {
        "List a component and all its related components with deps"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "component": {
                    "type": "string",
                    "description": "Component name"
                }
            },
            "required": ["component"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, ToolError> {
        Ok(super::related::execute(
            &self.0,
            &get_string(&input, "component")?,
        )?)
    }
}

#[derive(Debug)]
struct ProfileTool(Arc<SpecRoot>);

impl Tool for ProfileTool {
    fn name(&self) -> &str {
        "spec_profile"
    }

    fn description(&self) -> &str {
        "List files for a review profile"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "profile": {
                    "type": "string",
                    "description": "Profile name (e.g. accessibility, state_machine)"
                }
            },
            "required": ["profile"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, ToolError> {
        Ok(super::profile::execute(
            &self.0,
            &get_string(&input, "profile")?,
        )?)
    }
}

#[derive(Debug)]
struct TocTool(Arc<SpecRoot>);

impl Tool for TocTool {
    fn name(&self) -> &str {
        "spec_toc"
    }

    fn description(&self) -> &str {
        "Output heading structure of a spec file with line numbers"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file": {
                    "type": "string",
                    "description": "Path to spec file (relative to spec/ or absolute)"
                }
            },
            "required": ["file"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, ToolError> {
        Ok(super::toc::execute(&self.0, &get_string(&input, "file")?)?)
    }
}

#[derive(Debug)]
struct ValidateTool(Arc<SpecRoot>);

impl Tool for ValidateTool {
    fn name(&self) -> &str {
        "spec_validate"
    }

    fn description(&self) -> &str {
        "Validate that YAML frontmatter in spec files matches manifest.toml"
    }

    fn input_schema(&self) -> Value {
        json!({ "type": "object" })
    }

    fn execute(&self, _input: Value) -> Result<String, ToolError> {
        Ok(super::validate::execute(&self.0)?)
    }
}

#[derive(Debug)]
struct AdaptersTool(Arc<SpecRoot>);

impl Tool for AdaptersTool {
    fn name(&self) -> &str {
        "spec_adapters"
    }

    fn description(&self) -> &str {
        "List all adapter files for a framework, grouped by category"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "framework": {
                    "type": "string",
                    "description": "Framework: leptos or dioxus"
                }
            },
            "required": ["framework"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, ToolError> {
        Ok(super::adapters::execute(
            &self.0,
            &get_string(&input, "framework")?,
        )?)
    }
}

#[derive(Debug)]
struct SearchTool(Arc<SpecRoot>);

impl Tool for SearchTool {
    fn name(&self) -> &str {
        "spec_search"
    }

    fn description(&self) -> &str {
        "Search spec content by keyword/regex with optional category, section, and tier filters"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "category": {
                    "type": "string",
                    "description": "Optional category filter (e.g. input, overlay)"
                },
                "section": {
                    "type": "string",
                    "description": "Optional section filter: states, events, props, accessibility, anatomy, i18n, forms"
                },
                "tier": {
                    "type": "string",
                    "description": "Optional tier filter: stateless, stateful, complex"
                }
            },
            "required": ["query"]
        })
    }

    fn execute(&self, input: Value) -> Result<String, ToolError> {
        let query = get_string(&input, "query")?;
        let category = input.get("category").and_then(Value::as_str);
        let section = input.get("section").and_then(Value::as_str);
        let tier = input.get("tier").and_then(Value::as_str);
        Ok(super::search::execute(
            &self.0, &query, category, section, tier,
        )?)
    }
}
