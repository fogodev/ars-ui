# xtask MCP Spec Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `tools/spec-tool` with a `cargo xtask` crate that exposes all spec navigation commands via both CLI and MCP stdio server, adding search, digest, and context-loading capabilities.

**Architecture:** Single `xtask/` crate with library functions in `spec/` modules, a `Tool` trait for auto-exposure, and an MCP server using `rmcp` 1.3 with dynamic dispatch. CLI calls library functions directly; MCP wraps them via `ToolRegistry`.

**Tech Stack:** Rust (edition 2024), clap 4, rmcp 1.3, tokio 1, serde/serde_json, regex 1

**Design spec:** `docs/superpowers/specs/2026-04-03-xtask-mcp-spec-server-design.md`

---

### Task 1: Create xtask Crate Skeleton

**Files:**
- Create: `xtask/Cargo.toml`
- Create: `xtask/src/main.rs`
- Create: `.cargo/config.toml`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create .cargo/config.toml with xtask alias**

```toml
[alias]
xtask = "run --package xtask --"
```

- [ ] **Step 2: Create xtask/Cargo.toml**

```toml
[package]
name               = "xtask"
version            = "0.1.0"
edition.workspace  = true
license.workspace  = true
rust-version.workspace = true

[lints]
workspace = true

[dependencies]
clap       = { version = "4", features = ["derive"] }
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
toml       = "1"
regex      = "1"

# MCP support (feature-gated)
rmcp   = { version = "1.3", features = ["server", "transport-io"], optional = true }
tokio  = { version = "1", features = ["macros", "rt-multi-thread"], optional = true }
anyhow = { version = "1", optional = true }

[features]
default = ["mcp"]
mcp = ["dep:rmcp", "dep:tokio", "dep:anyhow"]
```

- [ ] **Step 3: Create xtask/src/main.rs stub**

```rust
//! ars-ui workspace task runner.

fn main() {
    println!("xtask stub");
}
```

- [ ] **Step 4: Update workspace Cargo.toml — replace spec-tool with xtask**

In the `[workspace] members` list, replace `"tools/spec-tool"` with `"xtask"`.

- [ ] **Step 5: Verify it compiles**

Run: `cargo build -p xtask`
Expected: builds successfully

- [ ] **Step 6: Verify alias works**

Run: `cargo xtask`
Expected: prints "xtask stub"

- [ ] **Step 7: Commit**

```bash
git add .cargo/config.toml xtask/ Cargo.toml
git commit -m "feat: create xtask crate skeleton with cargo alias"
```

---

### Task 2: Extract Manifest Types into manifest.rs

**Files:**
- Create: `xtask/src/manifest.rs`
- Modify: `xtask/src/main.rs`

- [ ] **Step 1: Write tests for manifest loading**

Create `xtask/src/manifest.rs` with types and tests:

```rust
//! Manifest types and parsing for `spec/manifest.toml`.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

/// Parsed `spec/manifest.toml`.
#[derive(Debug, Deserialize)]
pub struct Manifest {
    /// Foundation document paths keyed by name.
    pub foundation: BTreeMap<String, String>,
    /// Shared type file paths keyed by name.
    pub shared: BTreeMap<String, String>,
    /// All components keyed by lowercase name.
    pub components: BTreeMap<String, Component>,
    /// Named review profiles.
    pub review_profiles: Option<BTreeMap<String, ReviewProfile>>,
    /// Leptos adapter file paths keyed by component name.
    #[serde(default)]
    pub leptos_adapters: BTreeMap<String, String>,
    /// Dioxus adapter file paths keyed by component name.
    #[serde(default)]
    pub dioxus_adapters: BTreeMap<String, String>,
}

/// A single component entry in the manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct Component {
    /// Relative path to the spec file.
    pub path: String,
    /// Category name (e.g., "selection", "overlay").
    pub category: String,
    /// Required foundation modules.
    pub foundation_deps: Vec<String>,
    /// Required shared types.
    #[serde(default)]
    pub shared_deps: Vec<String>,
    /// Related component names.
    #[serde(default)]
    pub related: Vec<String>,
    /// Whether this component is internal/unstable.
    #[serde(default)]
    pub internal: bool,
}

/// A named review profile.
#[derive(Debug, Deserialize)]
pub struct ReviewProfile {
    /// Files always loaded for this profile.
    pub files_always: Vec<String>,
}

/// Parsed YAML frontmatter from a spec file.
#[derive(Debug, Default)]
pub struct Frontmatter {
    /// Component name.
    pub component: Option<String>,
    /// Category name.
    pub category: Option<String>,
    /// Foundation dependencies.
    pub foundation_deps: Option<Vec<String>>,
    /// Shared dependencies.
    pub shared_deps: Option<Vec<String>>,
    /// Related components.
    pub related: Option<Vec<String>>,
}

/// Loaded spec root: directory path + parsed manifest.
#[derive(Debug)]
pub struct SpecRoot {
    /// Path to the `spec/` directory.
    pub path: PathBuf,
    /// Parsed manifest.
    pub manifest: Manifest,
}

/// Errors from manifest operations.
#[derive(Debug)]
pub enum ManifestError {
    /// Could not find `spec/manifest.toml` in any parent directory.
    NotFound,
    /// IO error reading a file.
    Io(std::io::Error),
    /// TOML parse error.
    Parse(toml::de::Error),
    /// Component not found in manifest.
    ComponentNotFound {
        /// The name that was looked up.
        name: String,
        /// Available component names for hints.
        available: Vec<String>,
    },
    /// Category not found.
    CategoryNotFound {
        /// The name that was looked up.
        name: String,
        /// Available category names.
        available: Vec<String>,
    },
    /// Shared type not found.
    SharedTypeNotFound {
        /// The name that was looked up.
        name: String,
        /// Available shared type names.
        available: Vec<String>,
    },
    /// Review profile not found.
    ProfileNotFound {
        /// The name that was looked up.
        name: String,
        /// Available profile names.
        available: Vec<String>,
    },
    /// Unknown framework name.
    UnknownFramework(String),
    /// Frontmatter parse error.
    FrontmatterError(String),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => {
                write!(f, "could not find spec/manifest.toml in any parent directory")
            }
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Parse(e) => write!(f, "parse error: {e}"),
            Self::ComponentNotFound { name, available } => {
                write!(f, "component '{name}' not found. available: {}", available.join(", "))
            }
            Self::CategoryNotFound { name, available } => {
                write!(f, "category '{name}' not found. available: {}", available.join(", "))
            }
            Self::SharedTypeNotFound { name, available } => {
                write!(f, "shared type '{name}' not found. available: {}", available.join(", "))
            }
            Self::ProfileNotFound { name, available } => {
                write!(f, "profile '{name}' not found. available: {}", available.join(", "))
            }
            Self::UnknownFramework(name) => {
                write!(f, "unknown framework '{name}', use 'leptos' or 'dioxus'")
            }
            Self::FrontmatterError(msg) => write!(f, "frontmatter error: {msg}"),
        }
    }
}

impl std::error::Error for ManifestError {}

impl SpecRoot {
    /// Discover and load the spec root by walking up from `start_dir`.
    pub fn discover(start_dir: &Path) -> Result<Self, ManifestError> {
        let mut dir = start_dir.to_path_buf();
        loop {
            let candidate = dir.join("spec").join("manifest.toml");
            if candidate.exists() {
                let spec_path = dir.join("spec");
                let content =
                    fs::read_to_string(&candidate).map_err(ManifestError::Io)?;
                let manifest: Manifest =
                    toml::from_str(&content).map_err(ManifestError::Parse)?;
                return Ok(Self {
                    path: spec_path,
                    manifest,
                });
            }
            if !dir.pop() {
                return Err(ManifestError::NotFound);
            }
        }
    }
}

/// Look up a component key with fuzzy matching (case-insensitive, hyphen/underscore).
pub fn find_component_key(manifest: &Manifest, name: &str) -> Result<String, ManifestError> {
    let key = name.to_lowercase();
    if manifest.components.contains_key(&key) {
        return Ok(key);
    }
    let alt = key.replace('_', "-");
    if manifest.components.contains_key(&alt) {
        return Ok(alt);
    }
    Err(ManifestError::ComponentNotFound {
        name: name.to_string(),
        available: manifest.components.keys().cloned().collect(),
    })
}

/// Look up a component by name.
pub fn find_component<'a>(
    manifest: &'a Manifest,
    name: &str,
) -> Result<(&'a str, &'a Component), ManifestError> {
    let key = find_component_key(manifest, name)?;
    let comp = &manifest.components[&key];
    // Return a reference to the key stored in the map
    let key_ref = manifest
        .components
        .keys()
        .find(|k| **k == key)
        .expect("key was just found");
    Ok((key_ref, comp))
}

/// Resolve a foundation dependency name to its file path.
pub fn resolve_foundation(manifest: &Manifest, dep: &str) -> Option<&str> {
    manifest.foundation.get(dep).map(String::as_str)
}

/// Generate the category context file path.
pub fn category_file(category: &str) -> String {
    format!("components/{category}/_category.md")
}

/// Extract YAML frontmatter from spec file content.
pub fn extract_frontmatter(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = &trimmed[3..];
    let end = after_first.find("\n---")?;
    Some(after_first[..end].trim().to_string())
}

/// Extract legacy HTML comment header.
pub fn extract_html_comment_header(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    let idx = trimmed.find("<!--")?;
    if idx > 200 {
        return None;
    }
    let after = &trimmed[idx + 4..];
    let end = after.find("-->")?;
    Some(after[..end].trim().to_string())
}

/// Parse YAML frontmatter string into a `Frontmatter`.
pub fn parse_frontmatter_yaml(yaml_str: &str) -> Result<Frontmatter, ManifestError> {
    let mut fm = Frontmatter::default();

    for line in yaml_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "component" => fm.component = Some(value.to_string()),
                "category" => fm.category = Some(value.to_string()),
                "foundation_deps" => fm.foundation_deps = Some(parse_yaml_list(value)),
                "shared_deps" => fm.shared_deps = Some(parse_yaml_list(value)),
                "related" => fm.related = Some(parse_yaml_list(value)),
                _ => {}
            }
        }
    }

    Ok(fm)
}

/// Parse a YAML-style list value like `[item1, item2]`.
pub fn parse_yaml_list(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed == "[]" || trimmed.is_empty() {
        return Vec::new();
    }
    let inner = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
    if inner.is_empty() {
        return Vec::new();
    }
    inner.split(',').map(|s| s.trim().to_string()).collect()
}

/// Parse a markdown heading line into level and text.
pub fn parse_heading(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
    if hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    Some((hashes as u8, rest.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_yaml_list_basic() {
        assert_eq!(
            parse_yaml_list("[accessibility, architecture]"),
            vec!["accessibility", "architecture"]
        );
    }

    #[test]
    fn parse_yaml_list_empty() {
        assert!(parse_yaml_list("[]").is_empty());
        assert!(parse_yaml_list("").is_empty());
    }

    #[test]
    fn parse_heading_levels() {
        assert_eq!(parse_heading("# Title"), Some((1, "Title")));
        assert_eq!(parse_heading("## Section"), Some((2, "Section")));
        assert_eq!(parse_heading("### Sub"), Some((3, "Sub")));
        assert_eq!(parse_heading("not a heading"), None);
        assert_eq!(parse_heading("#nospace"), None);
    }

    #[test]
    fn extract_frontmatter_basic() {
        let content = "---\ncomponent: Checkbox\ncategory: input\n---\n# Checkbox";
        let fm = extract_frontmatter(content).expect("should extract");
        assert!(fm.contains("component: Checkbox"));
    }

    #[test]
    fn extract_frontmatter_missing() {
        assert!(extract_frontmatter("# No frontmatter").is_none());
    }

    #[test]
    fn parse_frontmatter_yaml_basic() {
        let yaml = "component: Checkbox\ncategory: input\nfoundation_deps: [architecture, accessibility]";
        let fm = parse_frontmatter_yaml(yaml).expect("should parse");
        assert_eq!(fm.component.as_deref(), Some("Checkbox"));
        assert_eq!(fm.category.as_deref(), Some("input"));
        assert_eq!(
            fm.foundation_deps.as_deref(),
            Some(&["architecture".to_string(), "accessibility".to_string()][..])
        );
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p xtask`
Expected: all tests pass

- [ ] **Step 3: Wire up manifest.rs in main.rs**

Replace `xtask/src/main.rs`:

```rust
//! ars-ui workspace task runner.

mod manifest;

fn main() {
    let cwd = std::env::current_dir().expect("cannot read current directory");
    let root = match manifest::SpecRoot::discover(&cwd) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    println!("Spec root: {}", root.path.display());
    println!("Components: {}", root.manifest.components.len());
}
```

- [ ] **Step 4: Verify it loads the manifest**

Run: `cargo xtask`
Expected: prints `Spec root: .../spec` and `Components: 111` (or current count)

- [ ] **Step 5: Commit**

```bash
git add xtask/src/manifest.rs xtask/src/main.rs
git commit -m "feat(xtask): extract manifest types with parsing and tests"
```

---

### Task 3: Migrate Spec Commands into Modules

**Files:**
- Create: `xtask/src/spec/mod.rs`
- Create: `xtask/src/spec/info.rs`
- Create: `xtask/src/spec/deps.rs`
- Create: `xtask/src/spec/category.rs`
- Create: `xtask/src/spec/reverse.rs`
- Create: `xtask/src/spec/related.rs`
- Create: `xtask/src/spec/profile.rs`
- Create: `xtask/src/spec/toc.rs`
- Create: `xtask/src/spec/validate.rs`
- Create: `xtask/src/spec/adapters.rs`

Each module has one public function that takes `&SpecRoot` + parameters and returns `Result<String, ManifestError>`. The logic is migrated from the existing `cmd_*` functions — the only change is returning a `String` via `write!` / `writeln!` instead of `println!`.

- [ ] **Step 1: Create spec/mod.rs**

```rust
//! Spec navigation commands.

pub mod adapters;
pub mod category;
pub mod deps;
pub mod info;
pub mod profile;
pub mod related;
pub mod reverse;
pub mod toc;
pub mod validate;
```

- [ ] **Step 2: Create spec/info.rs**

```rust
//! `spec_info` — show component metadata.

use std::fmt::Write;

use crate::manifest::{self, ManifestError, SpecRoot};

/// Return component metadata as text.
pub fn execute(root: &SpecRoot, component: &str) -> Result<String, ManifestError> {
    let (key, comp) = manifest::find_component(&root.manifest, component)?;
    let mut out = String::new();

    writeln!(out, "component: {component}").expect("write to String");
    writeln!(out, "path: {}", comp.path).expect("write to String");
    writeln!(out, "category: {}", comp.category).expect("write to String");
    writeln!(out, "foundation_deps: [{}]", comp.foundation_deps.join(", ")).expect("write to String");
    writeln!(out, "shared_deps: [{}]", comp.shared_deps.join(", ")).expect("write to String");
    writeln!(out, "related: [{}]", comp.related.join(", ")).expect("write to String");
    if comp.internal {
        writeln!(out, "internal: true").expect("write to String");
    }
    if let Some(path) = root.manifest.leptos_adapters.get(key) {
        writeln!(out, "leptos_adapter: {path}").expect("write to String");
    }
    if let Some(path) = root.manifest.dioxus_adapters.get(key) {
        writeln!(out, "dioxus_adapter: {path}").expect("write to String");
    }

    Ok(out)
}
```

- [ ] **Step 3: Create spec/deps.rs**

```rust
//! `spec_deps` — list all files needed to review a component.

use std::fmt::Write;

use crate::manifest::{self, ManifestError, SpecRoot};

/// Return the file set for reviewing a component.
pub fn execute(root: &SpecRoot, component: &str) -> Result<String, ManifestError> {
    let (key, comp) = manifest::find_component(&root.manifest, component)?;
    let m = &root.manifest;
    let mut out = String::new();

    writeln!(out, "# Files to load for reviewing {component}:").expect("write to String");
    writeln!(out).expect("write to String");

    writeln!(out, "## Component").expect("write to String");
    writeln!(out, "{}", comp.path).expect("write to String");
    writeln!(out).expect("write to String");

    if !comp.foundation_deps.is_empty() {
        writeln!(out, "## Foundation deps").expect("write to String");
        for dep in &comp.foundation_deps {
            if let Some(path) = manifest::resolve_foundation(m, dep) {
                writeln!(out, "{path}").expect("write to String");
            } else {
                writeln!(out, "# WARNING: unknown foundation dep '{dep}'").expect("write to String");
            }
        }
        writeln!(out).expect("write to String");
    }

    if !comp.shared_deps.is_empty() {
        writeln!(out, "## Shared deps").expect("write to String");
        for dep in &comp.shared_deps {
            if let Some(path) = m.shared.get(dep) {
                writeln!(out, "{path}").expect("write to String");
            } else {
                writeln!(out, "# WARNING: unknown shared dep '{dep}'").expect("write to String");
            }
        }
        writeln!(out).expect("write to String");
    }

    writeln!(out, "## Category context").expect("write to String");
    writeln!(out, "{}", manifest::category_file(&comp.category)).expect("write to String");
    writeln!(out).expect("write to String");

    let has_leptos = m.leptos_adapters.get(key);
    let has_dioxus = m.dioxus_adapters.get(key);
    if has_leptos.is_some() || has_dioxus.is_some() {
        writeln!(out, "## Adapter examples").expect("write to String");
        if let Some(path) = has_leptos {
            writeln!(out, "{path}  (Leptos)").expect("write to String");
        }
        if let Some(path) = has_dioxus {
            writeln!(out, "{path}  (Dioxus)").expect("write to String");
        }
        writeln!(out).expect("write to String");
    }

    if !comp.related.is_empty() {
        writeln!(out, "## Related components").expect("write to String");
        for rel in &comp.related {
            if let Some(rel_comp) = m.components.get(rel) {
                writeln!(out, "{}", rel_comp.path).expect("write to String");
            } else {
                writeln!(out, "# WARNING: unknown related component '{rel}'").expect("write to String");
            }
        }
        writeln!(out).expect("write to String");
    }

    Ok(out)
}
```

- [ ] **Step 4: Create spec/category.rs**

```rust
//! `spec_category` — list all components in a category.

use std::fmt::Write;

use crate::manifest::{self, ManifestError, SpecRoot};

/// Return all components in a category with metadata.
pub fn execute(root: &SpecRoot, name: &str) -> Result<String, ManifestError> {
    let m = &root.manifest;
    let components: Vec<_> = m
        .components
        .iter()
        .filter(|(_, c)| c.category == name)
        .collect();

    if components.is_empty() {
        let mut cats: Vec<&str> = m.components.values().map(|c| c.category.as_str()).collect();
        cats.sort();
        cats.dedup();
        return Err(ManifestError::CategoryNotFound {
            name: name.to_string(),
            available: cats.into_iter().map(String::from).collect(),
        });
    }

    let mut out = String::new();
    writeln!(out, "# Category: {name}").expect("write to String");
    writeln!(out).expect("write to String");
    writeln!(out, "## Category context").expect("write to String");
    writeln!(out, "{}", manifest::category_file(name)).expect("write to String");
    writeln!(out).expect("write to String");

    writeln!(out, "## Components ({} total)", components.len()).expect("write to String");
    for (key, comp) in &components {
        writeln!(out).expect("write to String");
        writeln!(out, "### {key}").expect("write to String");
        writeln!(out, "path: {}", comp.path).expect("write to String");
        if !comp.foundation_deps.is_empty() {
            writeln!(out, "foundation_deps: {}", comp.foundation_deps.join(", ")).expect("write to String");
        }
        if !comp.shared_deps.is_empty() {
            writeln!(out, "shared_deps: {}", comp.shared_deps.join(", ")).expect("write to String");
        }
        if !comp.related.is_empty() {
            writeln!(out, "related: {}", comp.related.join(", ")).expect("write to String");
        }
        if comp.internal {
            writeln!(out, "internal: true").expect("write to String");
        }
    }

    Ok(out)
}
```

- [ ] **Step 5: Create spec/reverse.rs**

```rust
//! `spec_reverse` — find components depending on a shared type.

use std::fmt::Write;

use crate::manifest::{ManifestError, SpecRoot};

/// Return components that depend on a shared type.
pub fn execute(root: &SpecRoot, shared_type: &str) -> Result<String, ManifestError> {
    let m = &root.manifest;
    if !m.shared.contains_key(shared_type) {
        return Err(ManifestError::SharedTypeNotFound {
            name: shared_type.to_string(),
            available: m.shared.keys().cloned().collect(),
        });
    }

    let dependents: Vec<_> = m
        .components
        .iter()
        .filter(|(_, c)| c.shared_deps.iter().any(|d| d == shared_type))
        .collect();

    let mut out = String::new();
    writeln!(out, "# Components depending on shared/{shared_type}").expect("write to String");
    writeln!(out).expect("write to String");

    if dependents.is_empty() {
        writeln!(out, "(none)").expect("write to String");
    } else {
        writeln!(out, "## Shared type file").expect("write to String");
        writeln!(out, "{}", m.shared[shared_type]).expect("write to String");
        writeln!(out).expect("write to String");
        writeln!(out, "## Dependent components ({} total)", dependents.len()).expect("write to String");
        for (key, comp) in &dependents {
            writeln!(out, "  {key}: {}", comp.path).expect("write to String");
        }
    }

    Ok(out)
}
```

- [ ] **Step 6: Create spec/related.rs**

```rust
//! `spec_related` — list a component and its related components.

use std::fmt::Write;

use crate::manifest::{self, ManifestError, SpecRoot};

/// Return the component and all its related components with deps.
pub fn execute(root: &SpecRoot, component: &str) -> Result<String, ManifestError> {
    let (_, comp) = manifest::find_component(&root.manifest, component)?;
    let m = &root.manifest;
    let mut out = String::new();

    writeln!(out, "# {component} and related components:").expect("write to String");
    writeln!(out).expect("write to String");
    writeln!(out, "## Primary").expect("write to String");
    writeln!(out, "{}", comp.path).expect("write to String");

    if comp.related.is_empty() {
        writeln!(out).expect("write to String");
        writeln!(out, "## Related").expect("write to String");
        writeln!(out, "(none)").expect("write to String");
    } else {
        writeln!(out).expect("write to String");
        writeln!(out, "## Related components").expect("write to String");
        for rel in &comp.related {
            writeln!(out).expect("write to String");
            if let Some(rel_comp) = m.components.get(rel) {
                writeln!(out, "### {rel}").expect("write to String");
                writeln!(out, "path: {}", rel_comp.path).expect("write to String");
                if !rel_comp.foundation_deps.is_empty() {
                    writeln!(out, "foundation_deps: {}", rel_comp.foundation_deps.join(", ")).expect("write to String");
                }
                if !rel_comp.shared_deps.is_empty() {
                    writeln!(out, "shared_deps: {}", rel_comp.shared_deps.join(", ")).expect("write to String");
                }
            } else {
                writeln!(out, "### {rel}").expect("write to String");
                writeln!(out, "# WARNING: not found in manifest").expect("write to String");
            }
        }
    }

    Ok(out)
}
```

- [ ] **Step 7: Create spec/profile.rs**

```rust
//! `spec_profile` — list files for a review profile.

use std::fmt::Write;

use crate::manifest::{ManifestError, SpecRoot};

/// Return files in a named review profile.
pub fn execute(root: &SpecRoot, name: &str) -> Result<String, ManifestError> {
    let profiles = root.manifest.review_profiles.as_ref().ok_or_else(|| {
        ManifestError::ProfileNotFound {
            name: name.to_string(),
            available: vec![],
        }
    })?;

    let profile = profiles
        .get(name)
        .or_else(|| profiles.get(&name.replace('-', "_")))
        .or_else(|| profiles.get(&name.replace('_', "-")))
        .ok_or_else(|| ManifestError::ProfileNotFound {
            name: name.to_string(),
            available: profiles.keys().cloned().collect(),
        })?;

    let mut out = String::new();
    writeln!(out, "# Review profile: {name}").expect("write to String");
    writeln!(out).expect("write to String");
    writeln!(out, "## Files always loaded").expect("write to String");
    for file in &profile.files_always {
        writeln!(out, "{file}").expect("write to String");
    }

    Ok(out)
}
```

- [ ] **Step 8: Create spec/toc.rs**

```rust
//! `spec_toc` — output heading structure of a spec file.

use std::{fmt::Write, fs, path::Path};

use crate::manifest::{self, ManifestError, SpecRoot};

/// Return the heading structure (table of contents) of a spec file.
pub fn execute(root: &SpecRoot, file: &str) -> Result<String, ManifestError> {
    let file_path = if Path::new(file).is_absolute() || Path::new(file).exists() {
        Path::new(file).to_path_buf()
    } else {
        root.path.join(file)
    };

    let content = fs::read_to_string(&file_path).map_err(ManifestError::Io)?;
    let mut out = String::new();

    for (line_num, line) in content.lines().enumerate() {
        if let Some((level, text)) = manifest::parse_heading(line) {
            let indent = match level {
                1 => "",
                2 => "  ",
                3 => "    ",
                4 => "      ",
                _ => "        ",
            };
            let prefix = "#".repeat(level as usize);
            writeln!(out, "{indent}{prefix} {text}  (L{ln})", ln = line_num + 1)
                .expect("write to String");
        }
    }

    Ok(out)
}
```

Note: Added line numbers (`L{n}`) to the toc output — this is more useful for LLM consumers than the original, and the MCP version benefits from knowing line numbers. The CLI output changes slightly but is strictly more informative.

- [ ] **Step 9: Create spec/validate.rs**

```rust
//! `spec_validate` — validate frontmatter against manifest.

use std::{fmt::Write, fs};

use crate::manifest::{self, ManifestError, SpecRoot};

/// Validate all spec file frontmatter against manifest entries.
/// Returns the validation report. Errors in validation are reported
/// in the output text, not as `Err`.
pub fn execute(root: &SpecRoot) -> Result<String, ManifestError> {
    let m = &root.manifest;
    let mut errors: Vec<String> = Vec::new();
    let mut checked = 0u32;

    for (name, comp) in &m.components {
        let file_path = root.path.join(&comp.path);
        if !file_path.exists() {
            errors.push(format!("[{name}] file not found: {}", comp.path));
            continue;
        }

        if comp.path.starts_with("foundation/") {
            checked += 1;
            continue;
        }

        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("[{name}] cannot read {}: {e}", comp.path));
                continue;
            }
        };

        if let Some(frontmatter) = manifest::extract_frontmatter(&content) {
            match manifest::parse_frontmatter_yaml(&frontmatter) {
                Ok(fm) => {
                    if let Some(fm_cat) = &fm.category {
                        if fm_cat != &comp.category {
                            errors.push(format!(
                                "[{name}] category mismatch: manifest='{}' file='{fm_cat}'",
                                comp.category
                            ));
                        }
                    }
                    if let Some(fm_fdeps) = &fm.foundation_deps {
                        let mut manifest_deps = comp.foundation_deps.clone();
                        let mut file_deps = fm_fdeps.clone();
                        manifest_deps.sort();
                        file_deps.sort();
                        if manifest_deps != file_deps {
                            errors.push(format!(
                                "[{name}] foundation_deps mismatch:\n  manifest: {manifest_deps:?}\n  file:     {file_deps:?}"
                            ));
                        }
                    }
                    if let Some(fm_sdeps) = &fm.shared_deps {
                        let mut manifest_deps = comp.shared_deps.clone();
                        let mut file_deps = fm_sdeps.clone();
                        manifest_deps.sort();
                        file_deps.sort();
                        if manifest_deps != file_deps {
                            errors.push(format!(
                                "[{name}] shared_deps mismatch:\n  manifest: {manifest_deps:?}\n  file:     {file_deps:?}"
                            ));
                        }
                    }
                    if let Some(fm_related) = &fm.related {
                        let mut manifest_rel = comp.related.clone();
                        let mut file_rel = fm_related.clone();
                        manifest_rel.sort();
                        file_rel.sort();
                        if manifest_rel != file_rel {
                            errors.push(format!(
                                "[{name}] related mismatch:\n  manifest: {manifest_rel:?}\n  file:     {file_rel:?}"
                            ));
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("[{name}] invalid YAML frontmatter: {e}"));
                }
            }
        } else if manifest::extract_html_comment_header(&content).is_some() {
            errors.push(format!(
                "[{name}] uses HTML comment header (not yet converted to YAML frontmatter)"
            ));
        } else {
            errors.push(format!(
                "[{name}] no frontmatter or HTML comment header found"
            ));
        }

        checked += 1;
    }

    for (framework, adapters) in [
        ("leptos", &m.leptos_adapters),
        ("dioxus", &m.dioxus_adapters),
    ] {
        for (name, path) in adapters {
            let file_path = root.path.join(path);
            if !file_path.exists() {
                errors.push(format!("[{framework}:{name}] adapter file not found: {path}"));
                continue;
            }
            if !m.components.contains_key(name) {
                errors.push(format!(
                    "[{framework}:{name}] adapter key has no matching [components.{name}] entry"
                ));
            }
            checked += 1;
        }
    }

    let mut out = String::new();
    writeln!(out, "Validated {checked} files (components + adapters).").expect("write to String");

    if errors.is_empty() {
        writeln!(out, "All checks passed.").expect("write to String");
    } else {
        writeln!(out).expect("write to String");
        writeln!(out, "{} error(s) found:", errors.len()).expect("write to String");
        for err in &errors {
            writeln!(out, "  {err}").expect("write to String");
        }
    }

    Ok(out)
}
```

- [ ] **Step 10: Create spec/adapters.rs**

```rust
//! `spec_adapters` — list adapter files for a framework.

use std::{collections::BTreeMap, fmt::Write};

use crate::manifest::{ManifestError, SpecRoot};

/// Return all adapter files for a framework, grouped by category.
pub fn execute(root: &SpecRoot, framework: &str) -> Result<String, ManifestError> {
    let m = &root.manifest;
    let adapters = match framework {
        "leptos" => &m.leptos_adapters,
        "dioxus" => &m.dioxus_adapters,
        _ => return Err(ManifestError::UnknownFramework(framework.to_string())),
    };

    if adapters.is_empty() {
        return Ok(format!("No {framework} adapter files registered.\n"));
    }

    let mut by_category: BTreeMap<String, Vec<(&String, &String)>> = BTreeMap::new();
    for (name, path) in adapters {
        let category = m
            .components
            .get(name)
            .map_or_else(|| "uncategorized".to_string(), |c| c.category.clone());
        by_category.entry(category).or_default().push((name, path));
    }

    let mut out = String::new();
    writeln!(out, "# {framework} adapter files ({} total)", adapters.len())
        .expect("write to String");
    writeln!(out).expect("write to String");
    for (category, entries) in &by_category {
        writeln!(out, "## {category}").expect("write to String");
        for (name, path) in entries {
            writeln!(out, "  {name}: {path}").expect("write to String");
        }
        writeln!(out).expect("write to String");
    }

    Ok(out)
}
```

- [ ] **Step 11: Verify compilation**

Run: `cargo build -p xtask`
Expected: builds successfully with no errors

- [ ] **Step 12: Commit**

```bash
git add xtask/src/spec/
git commit -m "feat(xtask): migrate 9 spec commands from spec-tool into modules"
```

---

### Task 4: Wire Up CLI with clap

**Files:**
- Create: `xtask/src/lib.rs`
- Modify: `xtask/src/main.rs`

- [ ] **Step 1: Create lib.rs**

```rust
//! ars-ui workspace task runner — library.

pub mod manifest;
pub mod spec;
```

- [ ] **Step 2: Rewrite main.rs with clap subcommands**

```rust
//! ars-ui workspace task runner.

use std::process;

use clap::{Parser, Subcommand};

/// ars-ui workspace task runner.
#[derive(Parser)]
#[command(name = "xtask", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Spec navigation commands.
    Spec {
        #[command(subcommand)]
        cmd: SpecCommand,
    },
}

#[derive(Subcommand)]
enum SpecCommand {
    /// Show component metadata.
    Info {
        /// Component name (e.g., "checkbox", "date-picker").
        component: String,
    },
    /// List all files needed to review a component.
    Deps {
        /// Component name.
        component: String,
    },
    /// List all components in a category.
    Category {
        /// Category name (e.g., "selection", "overlay").
        name: String,
    },
    /// Find components depending on a shared type.
    Reverse {
        /// Shared type name (e.g., "selection-patterns").
        shared_type: String,
    },
    /// List a component and its related components.
    Related {
        /// Component name.
        component: String,
    },
    /// List files for a review profile.
    Profile {
        /// Profile name (e.g., "accessibility").
        name: String,
    },
    /// Output heading structure of a spec file.
    Toc {
        /// Path to spec file (relative to spec/ or absolute).
        file: String,
    },
    /// Validate frontmatter against manifest.
    Validate,
    /// List adapter files for a framework.
    Adapters {
        /// Framework: "leptos" or "dioxus".
        framework: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().expect("cannot read current directory");
    let root = match xtask::manifest::SpecRoot::discover(&cwd) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    let result = match cli.command {
        Command::Spec { cmd } => match cmd {
            SpecCommand::Info { component } => xtask::spec::info::execute(&root, &component),
            SpecCommand::Deps { component } => xtask::spec::deps::execute(&root, &component),
            SpecCommand::Category { name } => xtask::spec::category::execute(&root, &name),
            SpecCommand::Reverse { shared_type } => {
                xtask::spec::reverse::execute(&root, &shared_type)
            }
            SpecCommand::Related { component } => {
                xtask::spec::related::execute(&root, &component)
            }
            SpecCommand::Profile { name } => xtask::spec::profile::execute(&root, &name),
            SpecCommand::Toc { file } => xtask::spec::toc::execute(&root, &file),
            SpecCommand::Validate => {
                let report = xtask::spec::validate::execute(&root);
                // Exit with code 1 if validation found errors (matches old spec-tool behavior)
                if let Ok(ref text) = report {
                    if text.contains("error(s) found:") {
                        print!("{text}");
                        process::exit(1);
                    }
                }
                report
            }
            SpecCommand::Adapters { framework } => {
                xtask::spec::adapters::execute(&root, &framework)
            }
        },
    };

    match result {
        Ok(output) => print!("{output}"),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}
```

- [ ] **Step 3: Verify CLI parity**

Run these and compare with old spec-tool:

```bash
cargo xtask spec info checkbox
cargo xtask spec deps checkbox
cargo xtask spec category input
cargo xtask spec reverse selection-patterns
cargo xtask spec validate
cargo xtask spec toc components/input/checkbox.md
cargo xtask spec adapters leptos
```

Expected: same content as `cargo run -p spec-tool -- <cmd>` (minor whitespace differences are acceptable, line numbers in toc are new and fine)

- [ ] **Step 4: Commit**

```bash
git add xtask/src/lib.rs xtask/src/main.rs
git commit -m "feat(xtask): wire up CLI with clap subcommands for all 9 spec commands"
```

---

### Task 5: Define Tool Trait and ToolRegistry

**Files:**
- Create: `xtask/src/tool.rs`
- Modify: `xtask/src/lib.rs`

- [ ] **Step 1: Write test for ToolRegistry**

Create `xtask/src/tool.rs`:

```rust
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

impl From<crate::manifest::ManifestError> for ToolError {
    fn from(e: crate::manifest::ManifestError) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

/// A tool that can be invoked via CLI or MCP.
pub trait Tool: Send + Sync {
    /// Unique tool name (e.g., "spec_info").
    fn name(&self) -> &str;

    /// Description for MCP tool listing.
    fn description(&self) -> &str;

    /// JSON Schema for the tool's input parameters.
    fn input_schema(&self) -> Value;

    /// Execute the tool with JSON input, returning text output.
    fn execute(&self, input: Value) -> Result<String, ToolError>;
}

/// Collects `Tool` implementations from all xtask modules.
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { tools: Vec::new() }
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

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
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
```

- [ ] **Step 2: Add tool module to lib.rs**

```rust
//! ars-ui workspace task runner — library.

pub mod manifest;
pub mod spec;
pub mod tool;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p xtask`
Expected: all tests pass

- [ ] **Step 4: Commit**

```bash
git add xtask/src/tool.rs xtask/src/lib.rs
git commit -m "feat(xtask): define Tool trait and ToolRegistry for MCP auto-exposure"
```

---

### Task 6: Implement Tool for All Spec Commands

**Files:**
- Create: `xtask/src/spec/tools.rs`
- Modify: `xtask/src/spec/mod.rs`

- [ ] **Step 1: Create spec/tools.rs with Tool impls for all 9 commands**

This module provides a `register_all()` function that registers all spec tools. Each tool impl deserializes JSON params and calls the corresponding module function.

```rust
//! MCP tool implementations for spec commands.

use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::manifest::SpecRoot;
use crate::tool::{Tool, ToolError, ToolRegistry};

/// Register all spec tools into a registry.
pub fn register_all(registry: &mut ToolRegistry, root: Arc<SpecRoot>) {
    registry.register(Box::new(InfoTool(Arc::clone(&root))));
    registry.register(Box::new(DepsTool(Arc::clone(&root))));
    registry.register(Box::new(CategoryTool(Arc::clone(&root))));
    registry.register(Box::new(ReverseTool(Arc::clone(&root))));
    registry.register(Box::new(RelatedTool(Arc::clone(&root))));
    registry.register(Box::new(ProfileTool(Arc::clone(&root))));
    registry.register(Box::new(TocTool(Arc::clone(&root))));
    registry.register(Box::new(ValidateTool(Arc::clone(&root))));
    registry.register(Box::new(AdaptersTool(Arc::clone(&root))));
}

fn get_string(input: &Value, field: &str) -> Result<String, ToolError> {
    input
        .get(field)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| ToolError {
            message: format!("missing required parameter: {field}"),
        })
}

// --- spec_info ---

struct InfoTool(Arc<SpecRoot>);

impl Tool for InfoTool {
    fn name(&self) -> &str { "spec_info" }
    fn description(&self) -> &str {
        "Get component metadata: category, deps, adapter paths"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "component": { "type": "string", "description": "Component name (e.g. checkbox, date-picker)" }
            },
            "required": ["component"]
        })
    }
    fn execute(&self, input: Value) -> Result<String, ToolError> {
        let component = get_string(&input, "component")?;
        Ok(super::info::execute(&self.0, &component)?)
    }
}

// --- spec_deps ---

struct DepsTool(Arc<SpecRoot>);

impl Tool for DepsTool {
    fn name(&self) -> &str { "spec_deps" }
    fn description(&self) -> &str {
        "List all files needed to review a component"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "component": { "type": "string", "description": "Component name" }
            },
            "required": ["component"]
        })
    }
    fn execute(&self, input: Value) -> Result<String, ToolError> {
        let component = get_string(&input, "component")?;
        Ok(super::deps::execute(&self.0, &component)?)
    }
}

// --- spec_category ---

struct CategoryTool(Arc<SpecRoot>);

impl Tool for CategoryTool {
    fn name(&self) -> &str { "spec_category" }
    fn description(&self) -> &str {
        "List all components in a category with their metadata"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "category": { "type": "string", "description": "Category name (e.g. input, selection, overlay)" }
            },
            "required": ["category"]
        })
    }
    fn execute(&self, input: Value) -> Result<String, ToolError> {
        let name = get_string(&input, "category")?;
        Ok(super::category::execute(&self.0, &name)?)
    }
}

// --- spec_reverse ---

struct ReverseTool(Arc<SpecRoot>);

impl Tool for ReverseTool {
    fn name(&self) -> &str { "spec_reverse" }
    fn description(&self) -> &str {
        "Find all components depending on a shared type"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "shared_type": { "type": "string", "description": "Shared type name (e.g. selection-patterns, date-time-types)" }
            },
            "required": ["shared_type"]
        })
    }
    fn execute(&self, input: Value) -> Result<String, ToolError> {
        let shared_type = get_string(&input, "shared_type")?;
        Ok(super::reverse::execute(&self.0, &shared_type)?)
    }
}

// --- spec_related ---

struct RelatedTool(Arc<SpecRoot>);

impl Tool for RelatedTool {
    fn name(&self) -> &str { "spec_related" }
    fn description(&self) -> &str {
        "List a component and all its related components with deps"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "component": { "type": "string", "description": "Component name" }
            },
            "required": ["component"]
        })
    }
    fn execute(&self, input: Value) -> Result<String, ToolError> {
        let component = get_string(&input, "component")?;
        Ok(super::related::execute(&self.0, &component)?)
    }
}

// --- spec_profile ---

struct ProfileTool(Arc<SpecRoot>);

impl Tool for ProfileTool {
    fn name(&self) -> &str { "spec_profile" }
    fn description(&self) -> &str {
        "List files for a review profile"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "profile": { "type": "string", "description": "Profile name (e.g. accessibility, state_machine)" }
            },
            "required": ["profile"]
        })
    }
    fn execute(&self, input: Value) -> Result<String, ToolError> {
        let name = get_string(&input, "profile")?;
        Ok(super::profile::execute(&self.0, &name)?)
    }
}

// --- spec_toc ---

struct TocTool(Arc<SpecRoot>);

impl Tool for TocTool {
    fn name(&self) -> &str { "spec_toc" }
    fn description(&self) -> &str {
        "Output heading structure of a spec file with line numbers"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file": { "type": "string", "description": "Path to spec file (relative to spec/ or absolute)" }
            },
            "required": ["file"]
        })
    }
    fn execute(&self, input: Value) -> Result<String, ToolError> {
        let file = get_string(&input, "file")?;
        Ok(super::toc::execute(&self.0, &file)?)
    }
}

// --- spec_validate ---

struct ValidateTool(Arc<SpecRoot>);

impl Tool for ValidateTool {
    fn name(&self) -> &str { "spec_validate" }
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

// --- spec_adapters ---

struct AdaptersTool(Arc<SpecRoot>);

impl Tool for AdaptersTool {
    fn name(&self) -> &str { "spec_adapters" }
    fn description(&self) -> &str {
        "List all adapter files for a framework, grouped by category"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "framework": { "type": "string", "description": "Framework: leptos or dioxus" }
            },
            "required": ["framework"]
        })
    }
    fn execute(&self, input: Value) -> Result<String, ToolError> {
        let framework = get_string(&input, "framework")?;
        Ok(super::adapters::execute(&self.0, &framework)?)
    }
}
```

- [ ] **Step 2: Add tools module to spec/mod.rs**

```rust
//! Spec navigation commands.

pub mod adapters;
pub mod category;
pub mod deps;
pub mod info;
pub mod profile;
pub mod related;
pub mod reverse;
pub mod toc;
pub mod tools;
pub mod validate;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p xtask`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add xtask/src/spec/tools.rs xtask/src/spec/mod.rs
git commit -m "feat(xtask): implement Tool trait for all 9 spec commands"
```

---

### Task 7: Implement spec_search

**Files:**
- Create: `xtask/src/spec/search.rs`
- Modify: `xtask/src/spec/mod.rs`
- Modify: `xtask/src/spec/tools.rs`

- [ ] **Step 1: Write tests for search**

Create `xtask/src/spec/search.rs`:

```rust
//! `spec_search` — keyword/regex search across spec content.

use std::{fmt::Write, fs};

use regex::Regex;

use crate::manifest::{self, ManifestError, SpecRoot};

/// Section filter — maps to known heading patterns in the spec template.
#[derive(Debug, Clone, Copy)]
pub enum SectionFilter {
    /// ## 1. State Machine (stateful/complex) or ## 1. API (stateless)
    States,
    /// ### 1.2 Events
    Events,
    /// ### 1.4 Props or ### 1.1 Props
    Props,
    /// ## 3. Accessibility (or ## 2. for stateless after Anatomy renumbering)
    Accessibility,
    /// ## 2. Anatomy
    Anatomy,
    /// ## N. Internationalization
    Internationalization,
    /// ## N. Form Integration
    FormIntegration,
}

impl SectionFilter {
    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "states" | "state_machine" | "state-machine" => Some(Self::States),
            "events" => Some(Self::Events),
            "props" => Some(Self::Props),
            "accessibility" | "a11y" => Some(Self::Accessibility),
            "anatomy" => Some(Self::Anatomy),
            "i18n" | "internationalization" => Some(Self::Internationalization),
            "forms" | "form_integration" | "form-integration" => Some(Self::FormIntegration),
            _ => None,
        }
    }

    /// Heading patterns that indicate this section.
    fn heading_patterns(&self) -> &[&str] {
        match self {
            Self::States => &["State Machine", "API"],
            Self::Events => &["Events"],
            Self::Props => &["Props"],
            Self::Accessibility => &["Accessibility"],
            Self::Anatomy => &["Anatomy"],
            Self::Internationalization => &["Internationalization"],
            Self::FormIntegration => &["Form Integration"],
        }
    }
}

/// A single search hit.
struct SearchHit {
    file: String,
    line_num: usize,
    line: String,
    section: String,
}

/// Search spec content.
///
/// - `query`: regex pattern to search for.
/// - `category`: optional category filter.
/// - `section`: optional section filter.
/// - `tier`: optional tier filter (stateless, stateful, complex).
pub fn execute(
    root: &SpecRoot,
    query: &str,
    category: Option<&str>,
    section: Option<&str>,
    tier: Option<&str>,
) -> Result<String, ManifestError> {
    let re = Regex::new(query).map_err(|e| ManifestError::FrontmatterError(format!("invalid regex: {e}")))?;
    let section_filter = section.and_then(SectionFilter::from_str);

    let m = &root.manifest;
    let mut hits: Vec<SearchHit> = Vec::new();

    // Determine which files to search
    let files: Vec<(&str, &str)> = m
        .components
        .iter()
        .filter(|(_, c)| category.is_none() || category == Some(c.category.as_str()))
        .map(|(name, c)| (name.as_str(), c.path.as_str()))
        .collect();

    for (name, rel_path) in &files {
        // Tier filter: read frontmatter to check tier
        if tier.is_some() {
            let file_path = root.path.join(rel_path);
            if let Ok(content) = fs::read_to_string(&file_path) {
                if let Some(fm_str) = manifest::extract_frontmatter(&content) {
                    // Simple check: look for "tier: <value>" in frontmatter
                    let has_tier = fm_str.lines().any(|l| {
                        l.trim()
                            .strip_prefix("tier:")
                            .is_some_and(|v| Some(v.trim()) == tier)
                    });
                    if !has_tier {
                        continue;
                    }
                }
            }
        }

        let file_path = root.path.join(rel_path);
        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut current_section = String::new();
        let mut in_matching_section = section_filter.is_none();
        let mut section_level: u8 = 0;

        for (line_idx, line) in content.lines().enumerate() {
            // Track current section
            if let Some((level, text)) = manifest::parse_heading(line) {
                current_section = text.to_string();

                if let Some(ref filter) = section_filter {
                    let patterns = filter.heading_patterns();
                    if level == 2 {
                        in_matching_section = patterns.iter().any(|p| text.contains(p));
                        if in_matching_section {
                            section_level = level;
                        }
                    } else if level <= section_level && section_level > 0 {
                        // Exited the section
                        in_matching_section = false;
                    }
                }
            }

            if in_matching_section && re.is_match(line) {
                hits.push(SearchHit {
                    file: rel_path.to_string(),
                    line_num: line_idx + 1,
                    line: line.to_string(),
                    section: current_section.clone(),
                });
            }
        }
    }

    let mut out = String::new();
    writeln!(out, "# Search results for /{query}/").expect("write to String");
    if let Some(cat) = category {
        writeln!(out, "Category filter: {cat}").expect("write to String");
    }
    if let Some(sec) = section {
        writeln!(out, "Section filter: {sec}").expect("write to String");
    }
    if let Some(t) = tier {
        writeln!(out, "Tier filter: {t}").expect("write to String");
    }
    writeln!(out, "Matches: {}", hits.len()).expect("write to String");
    writeln!(out).expect("write to String");

    for hit in &hits {
        writeln!(out, "{}:L{} [{}]", hit.file, hit.line_num, hit.section).expect("write to String");
        writeln!(out, "  {}", hit.line.trim()).expect("write to String");
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_filter_parsing() {
        assert!(SectionFilter::from_str("states").is_some());
        assert!(SectionFilter::from_str("a11y").is_some());
        assert!(SectionFilter::from_str("props").is_some());
        assert!(SectionFilter::from_str("nonexistent").is_none());
    }
}
```

- [ ] **Step 2: Add search module and tool registration**

Add `pub mod search;` to `spec/mod.rs`.

Add to `spec/tools.rs` `register_all()`:
```rust
registry.register(Box::new(SearchTool(Arc::clone(&root))));
```

Add to `spec/tools.rs`:
```rust
// --- spec_search ---

struct SearchTool(Arc<SpecRoot>);

impl Tool for SearchTool {
    fn name(&self) -> &str { "spec_search" }
    fn description(&self) -> &str {
        "Search spec content by keyword/regex with optional category, section, and tier filters"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Regex pattern to search for" },
                "category": { "type": "string", "description": "Optional category filter (e.g. input, overlay)" },
                "section": { "type": "string", "description": "Optional section filter: states, events, props, accessibility, anatomy, i18n, forms" },
                "tier": { "type": "string", "description": "Optional tier filter: stateless, stateful, complex" }
            },
            "required": ["query"]
        })
    }
    fn execute(&self, input: Value) -> Result<String, ToolError> {
        let query = get_string(&input, "query")?;
        let category = input.get("category").and_then(Value::as_str);
        let section = input.get("section").and_then(Value::as_str);
        let tier = input.get("tier").and_then(Value::as_str);
        Ok(super::search::execute(&self.0, &query, category, section, tier)?)
    }
}
```

- [ ] **Step 3: Add search to CLI**

Add to the `SpecCommand` enum in `main.rs`:
```rust
/// Search spec content by keyword/regex.
Search {
    /// Regex pattern to search for.
    query: String,
    /// Category filter.
    #[arg(long)]
    category: Option<String>,
    /// Section filter (states, events, props, accessibility, anatomy, i18n, forms).
    #[arg(long)]
    section: Option<String>,
    /// Tier filter (stateless, stateful, complex).
    #[arg(long)]
    tier: Option<String>,
},
```

Add to the match arm:
```rust
SpecCommand::Search { query, category, section, tier } => {
    xtask::spec::search::execute(
        &root,
        &query,
        category.as_deref(),
        section.as_deref(),
        tier.as_deref(),
    )
}
```

- [ ] **Step 4: Test search**

Run: `cargo xtask spec search "SelectionChanged"`
Expected: returns hits across multiple component specs

Run: `cargo xtask spec search "focus" --section accessibility --category overlay`
Expected: returns focus-related hits in accessibility sections of overlay components

- [ ] **Step 5: Commit**

```bash
git add xtask/src/spec/search.rs xtask/src/spec/mod.rs xtask/src/spec/tools.rs xtask/src/main.rs
git commit -m "feat(xtask): add spec_search with keyword/regex and section filtering"
```

---

### Task 8: Implement spec_digest

**Files:**
- Create: `xtask/src/spec/digest.rs`
- Modify: `xtask/src/spec/mod.rs`
- Modify: `xtask/src/spec/tools.rs`
- Modify: `xtask/src/main.rs`

- [ ] **Step 1: Create spec/digest.rs**

```rust
//! `spec_digest` — pre-computed component summary.

use std::{fmt::Write, fs};

use crate::manifest::{self, ManifestError, SpecRoot};

/// Extract a compact digest of a component's key sections.
///
/// Reads the component spec file and extracts content under known section
/// headings (State Machine, Accessibility, Props, Events, etc.).
pub fn execute(root: &SpecRoot, component: &str) -> Result<String, ManifestError> {
    let (key, comp) = manifest::find_component(&root.manifest, component)?;
    let file_path = root.path.join(&comp.path);
    let content = fs::read_to_string(&file_path).map_err(ManifestError::Io)?;

    let mut out = String::new();

    // Header
    writeln!(out, "# Digest: {component}").expect("write to String");
    writeln!(out, "path: {}", comp.path).expect("write to String");
    writeln!(out, "category: {}", comp.category).expect("write to String");
    writeln!(out, "foundation_deps: [{}]", comp.foundation_deps.join(", ")).expect("write to String");
    writeln!(out, "shared_deps: [{}]", comp.shared_deps.join(", ")).expect("write to String");
    if let Some(path) = root.manifest.leptos_adapters.get(key) {
        writeln!(out, "leptos_adapter: {path}").expect("write to String");
    }
    if let Some(path) = root.manifest.dioxus_adapters.get(key) {
        writeln!(out, "dioxus_adapter: {path}").expect("write to String");
    }
    writeln!(out).expect("write to String");

    // Extract sections by heading
    let sections_of_interest = [
        ("States", &["### 1.1 States"][..]),
        ("Events", &["### 1.2 Events"]),
        ("Context", &["### 1.3 Context"]),
        ("Props", &["### 1.4 Props", "### 1.1 Props"]),
        ("Connect / API", &["### 1.7 Connect", "### 1.6 Connect", "### 1.5 Connect", "### 1.2 Connect"]),
        ("Anatomy", &["## 2. Anatomy"]),
        ("Accessibility", &["## 3. Accessibility"]),
        ("Internationalization", &["Internationalization"]),
        ("Form Integration", &["Form Integration"]),
    ];

    for (label, heading_prefixes) in &sections_of_interest {
        if let Some(section_content) = extract_section(&content, heading_prefixes) {
            writeln!(out, "## {label}").expect("write to String");
            writeln!(out, "{section_content}").expect("write to String");
            writeln!(out).expect("write to String");
        }
    }

    Ok(out)
}

/// Extract content under a section heading until the next heading of equal or higher level.
fn extract_section(content: &str, heading_prefixes: &[&str]) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut start_idx = None;
    let mut start_level: u8 = 0;

    // Find the first matching heading
    for (i, line) in lines.iter().enumerate() {
        if let Some((level, text)) = manifest::parse_heading(line) {
            if heading_prefixes.iter().any(|prefix| {
                // Match if the heading text starts with the prefix (after stripping ## markup)
                let clean_prefix = prefix.trim_start_matches('#').trim();
                text.starts_with(clean_prefix)
            }) {
                start_idx = Some(i + 1);
                start_level = level;
                break;
            }
        }
    }

    let start = start_idx?;
    let mut end = lines.len();

    // Find the end: next heading of same or higher level
    for i in start..lines.len() {
        if let Some((level, _)) = manifest::parse_heading(lines[i]) {
            if level <= start_level {
                end = i;
                break;
            }
        }
    }

    let section: String = lines[start..end]
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .join("\n");

    let trimmed = section.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_section_basic() {
        let content = "# Title\n## 1. State Machine\n### 1.1 States\nstate A\nstate B\n### 1.2 Events\nevent X\n## 2. Anatomy\npart root";
        let result = extract_section(content, &["### 1.1 States"]);
        assert_eq!(result.as_deref(), Some("state A\nstate B"));
    }

    #[test]
    fn extract_section_not_found() {
        let content = "# Title\n## 2. Anatomy\nstuff";
        let result = extract_section(content, &["### 1.1 States"]);
        assert!(result.is_none());
    }
}
```

- [ ] **Step 2: Wire up module, tool, and CLI**

Add `pub mod digest;` to `spec/mod.rs`.

Add to `spec/tools.rs` `register_all()`:
```rust
registry.register(Box::new(DigestTool(Arc::clone(&root))));
```

Add to `spec/tools.rs`:
```rust
// --- spec_digest ---

struct DigestTool(Arc<SpecRoot>);

impl Tool for DigestTool {
    fn name(&self) -> &str { "spec_digest" }
    fn description(&self) -> &str {
        "Get a compact summary of a component: states, events, props, accessibility, anatomy"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "component": { "type": "string", "description": "Component name" }
            },
            "required": ["component"]
        })
    }
    fn execute(&self, input: Value) -> Result<String, ToolError> {
        let component = get_string(&input, "component")?;
        Ok(super::digest::execute(&self.0, &component)?)
    }
}
```

Add to CLI in `main.rs`:
```rust
/// Get a compact summary of a component.
Digest {
    /// Component name.
    component: String,
},
```

Match arm:
```rust
SpecCommand::Digest { component } => xtask::spec::digest::execute(&root, &component),
```

- [ ] **Step 3: Test digest**

Run: `cargo xtask spec digest checkbox`
Expected: compact summary with States, Events, Context, Props, Accessibility, etc.

- [ ] **Step 4: Commit**

```bash
git add xtask/src/spec/digest.rs xtask/src/spec/mod.rs xtask/src/spec/tools.rs xtask/src/main.rs
git commit -m "feat(xtask): add spec_digest for compact component summaries"
```

---

### Task 9: Implement spec_context

**Files:**
- Create: `xtask/src/spec/context.rs`
- Modify: `xtask/src/spec/mod.rs`
- Modify: `xtask/src/spec/tools.rs`
- Modify: `xtask/src/main.rs`

- [ ] **Step 1: Create spec/context.rs**

```rust
//! `spec_context` — dependency-aware context loading.

use std::{fmt::Write, fs};

use crate::manifest::{self, ManifestError, SpecRoot};

/// Return the full implementation context for a component.
///
/// Concatenates the component spec with all its dependencies (foundation,
/// shared, adapter, testing) with file-boundary markers.
pub fn execute(
    root: &SpecRoot,
    component: &str,
    framework: Option<&str>,
    include_testing: bool,
) -> Result<String, ManifestError> {
    let (key, comp) = manifest::find_component(&root.manifest, component)?;
    let m = &root.manifest;
    let mut out = String::new();

    writeln!(out, "# Implementation context for {component}").expect("write to String");
    writeln!(out).expect("write to String");

    // 1. Foundation deps
    for dep in &comp.foundation_deps {
        if let Some(path) = manifest::resolve_foundation(m, dep) {
            append_file(&root.path, path, &mut out);
        }
    }

    // 2. Shared deps
    for dep in &comp.shared_deps {
        if let Some(path) = m.shared.get(dep) {
            append_file(&root.path, path, &mut out);
        }
    }

    // 3. Component spec
    append_file(&root.path, &comp.path, &mut out);

    // 4. Framework adapter spec
    if let Some(fw) = framework {
        let adapters = match fw {
            "leptos" => &m.leptos_adapters,
            "dioxus" => &m.dioxus_adapters,
            _ => return Err(ManifestError::UnknownFramework(fw.to_string())),
        };
        if let Some(path) = adapters.get(key) {
            append_file(&root.path, path, &mut out);
        }
    }

    // 5. Testing spec (if requested)
    if include_testing {
        // Include the testing overview and the tier-relevant testing files
        let testing_files = ["testing/00-overview.md"];
        for tf in &testing_files {
            let full = root.path.join(tf);
            if full.exists() {
                append_file(&root.path, tf, &mut out);
            }
        }
    }

    Ok(out)
}

/// Append a file's content with a boundary marker.
fn append_file(spec_root: &std::path::Path, rel_path: &str, out: &mut String) {
    let full_path = spec_root.join(rel_path);
    match fs::read_to_string(&full_path) {
        Ok(content) => {
            writeln!(out, "--- FILE: {rel_path} ---").expect("write to String");
            writeln!(out, "{content}").expect("write to String");
            writeln!(out).expect("write to String");
        }
        Err(e) => {
            writeln!(out, "--- FILE: {rel_path} (ERROR: {e}) ---").expect("write to String");
            writeln!(out).expect("write to String");
        }
    }
}
```

- [ ] **Step 2: Wire up module, tool, and CLI**

Add `pub mod context;` to `spec/mod.rs`.

Add to `spec/tools.rs` `register_all()`:
```rust
registry.register(Box::new(ContextTool(Arc::clone(&root))));
```

Add to `spec/tools.rs`:
```rust
// --- spec_context ---

struct ContextTool(Arc<SpecRoot>);

impl Tool for ContextTool {
    fn name(&self) -> &str { "spec_context" }
    fn description(&self) -> &str {
        "Get full implementation context: component spec + all deps concatenated with file markers"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "component": { "type": "string", "description": "Component name" },
                "framework": { "type": "string", "description": "Optional: leptos or dioxus — includes adapter spec" },
                "include_testing": { "type": "boolean", "description": "Include testing spec (default false)" }
            },
            "required": ["component"]
        })
    }
    fn execute(&self, input: Value) -> Result<String, ToolError> {
        let component = get_string(&input, "component")?;
        let framework = input.get("framework").and_then(Value::as_str);
        let include_testing = input
            .get("include_testing")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Ok(super::context::execute(
            &self.0,
            &component,
            framework,
            include_testing,
        )?)
    }
}
```

Add to CLI in `main.rs`:
```rust
/// Get full implementation context for a component.
Context {
    /// Component name.
    component: String,
    /// Framework: "leptos" or "dioxus" (includes adapter spec).
    #[arg(long)]
    framework: Option<String>,
    /// Include testing specs.
    #[arg(long)]
    include_testing: bool,
},
```

Match arm:
```rust
SpecCommand::Context { component, framework, include_testing } => {
    xtask::spec::context::execute(
        &root,
        &component,
        framework.as_deref(),
        include_testing,
    )
}
```

- [ ] **Step 3: Test context**

Run: `cargo xtask spec context checkbox --framework leptos | head -20`
Expected: starts with "# Implementation context for checkbox", then foundation file markers

Run: `cargo xtask spec context checkbox | wc -l`
Expected: substantial output (several thousand lines — all deps concatenated)

- [ ] **Step 4: Commit**

```bash
git add xtask/src/spec/context.rs xtask/src/spec/mod.rs xtask/src/spec/tools.rs xtask/src/main.rs
git commit -m "feat(xtask): add spec_context for dependency-aware context loading"
```

---

### Task 10: Implement MCP Server

**Files:**
- Create: `xtask/src/mcp.rs`
- Modify: `xtask/src/lib.rs`
- Modify: `xtask/src/main.rs`

- [ ] **Step 1: Create mcp.rs**

```rust
//! MCP stdio server with dynamic tool dispatch.

use std::sync::Arc;

use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt,
    model::*,
    service::RequestContext,
    transport::stdio,
};
use serde_json::json;

use crate::manifest::SpecRoot;
use crate::tool::ToolRegistry;

// Alias the rmcp protocol type to avoid confusion with our Tool trait.
use rmcp::model::Tool as McpTool;

/// MCP server backed by a ToolRegistry.
struct McpServer {
    registry: ToolRegistry,
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("ars-ui-xtask", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "ars-ui workspace tools. Use spec_* tools to navigate the 400+ file spec corpus."
                    .to_string(),
            )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let tools: Vec<McpTool> = self
            .registry
            .tools()
            .iter()
            .map(|t| {
                let schema = t.input_schema();
                let schema_obj: JsonObject = serde_json::from_value(schema)
                    .unwrap_or_else(|_| serde_json::Map::new());
                McpTool::new(t.name(), t.description(), schema_obj)
            })
            .collect();

        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let name = request.name.as_ref();
        let tool = self.registry.find(name).ok_or_else(|| {
            ErrorData::method_not_found(format!("unknown tool: {name}"), None)
        })?;

        let input: serde_json::Value = request
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or(json!({}));

        match tool.execute(input) {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }
}

/// Start the MCP stdio server.
pub async fn serve(root: Arc<SpecRoot>) -> anyhow::Result<()> {
    // Build the tool registry
    let mut registry = ToolRegistry::new();
    crate::spec::tools::register_all(&mut registry, root);

    let server = McpServer { registry };
    let service = server.serve(stdio()).await
        .map_err(|e| anyhow::anyhow!("MCP server failed to start: {e}"))?;
    service.waiting().await
        .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;

    Ok(())
}
```

- [ ] **Step 2: Add mcp module to lib.rs (feature-gated)**

```rust
//! ars-ui workspace task runner — library.

pub mod manifest;
pub mod spec;
pub mod tool;

#[cfg(feature = "mcp")]
pub mod mcp;
```

- [ ] **Step 3: Add MCP subcommand to CLI**

(Note: `anyhow` is already in Cargo.toml from Task 1 as an optional dep in the `mcp` feature.)

Add to the `Command` enum in `main.rs`:
```rust
/// Start MCP stdio server exposing all workspace tools.
#[cfg(feature = "mcp")]
Mcp,
```

Add to the match in `main()`, before the existing `Command::Spec` arm:
```rust
#[cfg(feature = "mcp")]
Command::Mcp => {
    let root = Arc::new(root);
    let rt = tokio::runtime::Runtime::new().expect("cannot create tokio runtime");
    rt.block_on(xtask::mcp::serve(root)).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        process::exit(1);
    });
    return;
}
```

Add the necessary import at the top of `main.rs`:
```rust
#[cfg(feature = "mcp")]
use std::sync::Arc;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p xtask`
Expected: compiles with MCP feature (default)

Run: `cargo build -p xtask --no-default-features`
Expected: compiles without MCP dependencies

- [ ] **Step 5: Test MCP server starts**

Run (in one terminal):
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}' | cargo xtask mcp
```
Expected: JSON response with `serverInfo` containing `name: "ars-ui-xtask"` and `capabilities.tools`

- [ ] **Step 6: Commit**

```bash
git add xtask/src/mcp.rs xtask/src/lib.rs xtask/src/main.rs xtask/Cargo.toml
git commit -m "feat(xtask): add MCP stdio server with dynamic tool dispatch"
```

---

### Task 11: Integration and Cleanup

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `CLAUDE.md`
- Create or modify: `.claude/settings.local.json` (MCP registration)
- Delete: `tools/spec-tool/`

- [ ] **Step 1: Verify all tests pass**

Run: `cargo test -p xtask`
Expected: all tests pass

- [ ] **Step 2: Verify all 12 CLI commands work**

```bash
cargo xtask spec info checkbox
cargo xtask spec deps checkbox
cargo xtask spec category input
cargo xtask spec reverse selection-patterns
cargo xtask spec related checkbox
cargo xtask spec profile accessibility
cargo xtask spec toc components/input/checkbox.md
cargo xtask spec validate
cargo xtask spec adapters leptos
cargo xtask spec search "SelectionChanged"
cargo xtask spec digest checkbox
cargo xtask spec context checkbox --framework leptos
```

Expected: all produce reasonable output, no errors

- [ ] **Step 3: Update CLAUDE.md — replace spec-tool references**

Find all occurrences of `cargo run -p spec-tool` and replace with `cargo xtask spec`. Specifically:

- `cargo run -p spec-tool -- reverse <shared-type>` becomes `cargo xtask spec reverse <shared-type>`
- `cargo run -p spec-tool -- info <component>` becomes `cargo xtask spec info <component>`
- `cargo run -p spec-tool -- toc <file>` becomes `cargo xtask spec toc <file>`
- `cargo run -p spec-tool -- validate` becomes `cargo xtask spec validate`

Also add the new commands to the CLAUDE.md spec-tool section:
```markdown
# New capabilities:
cargo xtask spec search <query> [--category <cat>] [--section <sec>] [--tier <tier>]
cargo xtask spec digest <component>
cargo xtask spec context <component> [--framework <fw>] [--include-testing]
```

- [ ] **Step 4: Register MCP server in .claude/settings.local.json**

Add to the `mcpServers` section:
```json
"xtask": {
  "command": "cargo",
  "args": ["xtask", "mcp"]
}
```

- [ ] **Step 5: Delete tools/spec-tool/**

Remove the old crate. The workspace Cargo.toml was already updated in Task 1 to reference `xtask` instead.

```bash
rm -rf tools/spec-tool
```

If `tools/` is now empty, remove it too.

- [ ] **Step 6: Verify workspace builds clean**

Run: `cargo build`
Expected: full workspace builds with no errors

Run: `cargo test -p xtask`
Expected: all tests pass

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(xtask): complete migration from spec-tool, register MCP server

- All 9 spec-tool commands migrated to cargo xtask spec
- 3 new tools: search, digest, context
- MCP stdio server with dynamic auto-exposure via Tool trait
- Updated CLAUDE.md references
- Registered MCP server in .claude/settings.local.json
- Deleted tools/spec-tool/"
```
