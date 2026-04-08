//! Manifest types and parsing for `spec/manifest.toml`.

use std::{
    collections::BTreeMap,
    fmt, fs, io,
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

/// Loaded spec root: directory path plus parsed manifest.
#[derive(Debug)]
pub struct SpecRoot {
    /// Path to the `spec/` directory.
    pub path: PathBuf,
    /// Parsed manifest.
    pub manifest: Manifest,
}

/// Errors from manifest operations.
#[derive(Debug)]
pub enum Error {
    /// Could not find `spec/manifest.toml` in any parent directory.
    NotFound,
    /// IO error reading a file.
    Io(io::Error),
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
    UnknownFramework(
        /// The framework name that was provided.
        String,
    ),
    /// Frontmatter parse error.
    FrontmatterError(
        /// Description of the parsing failure.
        String,
    ),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => {
                write!(
                    f,
                    "could not find spec/manifest.toml in any parent directory"
                )
            }
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Parse(e) => write!(f, "parse error: {e}"),
            Self::ComponentNotFound { name, available } => {
                write!(
                    f,
                    "component '{name}' not found. available: {}",
                    available.join(", ")
                )
            }
            Self::CategoryNotFound { name, available } => {
                write!(
                    f,
                    "category '{name}' not found. available: {}",
                    available.join(", ")
                )
            }
            Self::SharedTypeNotFound { name, available } => {
                write!(
                    f,
                    "shared type '{name}' not found. available: {}",
                    available.join(", ")
                )
            }
            Self::ProfileNotFound { name, available } => {
                write!(
                    f,
                    "profile '{name}' not found. available: {}",
                    available.join(", ")
                )
            }
            Self::UnknownFramework(name) => {
                write!(f, "unknown framework '{name}', use 'leptos' or 'dioxus'")
            }
            Self::FrontmatterError(msg) => write!(f, "frontmatter error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl SpecRoot {
    /// Discover and load the spec root by walking up from `start_dir`.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::NotFound`] if no `spec/manifest.toml` exists in any
    /// ancestor, [`ManifestError::Io`] on read failure, or [`ManifestError::Parse`]
    /// on invalid TOML.
    pub fn discover(start_dir: &Path) -> Result<Self, Error> {
        let mut dir = start_dir.to_path_buf();
        loop {
            let candidate = dir.join("spec").join("manifest.toml");
            if candidate.exists() {
                let spec_path = dir.join("spec");
                let content = fs::read_to_string(&candidate).map_err(Error::Io)?;
                let manifest: Manifest = toml::from_str(&content).map_err(Error::Parse)?;
                return Ok(Self {
                    path: spec_path,
                    manifest,
                });
            }
            if !dir.pop() {
                return Err(Error::NotFound);
            }
        }
    }
}

/// Look up a component key with fuzzy matching (case-insensitive, hyphen/underscore).
///
/// # Errors
///
/// Returns [`ManifestError::ComponentNotFound`] if no match exists.
pub fn find_component_key(manifest: &Manifest, name: &str) -> Result<String, Error> {
    let key = name.to_lowercase();
    if manifest.components.contains_key(&key) {
        return Ok(key);
    }
    let alt = key.replace('_', "-");
    if manifest.components.contains_key(&alt) {
        return Ok(alt);
    }
    Err(Error::ComponentNotFound {
        name: name.to_string(),
        available: manifest.components.keys().cloned().collect(),
    })
}

/// Look up a component by name. Returns the canonical key and the component.
///
/// # Errors
///
/// Returns [`ManifestError::ComponentNotFound`] if no match exists.
pub fn find_component<'a>(
    manifest: &'a Manifest,
    name: &str,
) -> Result<(&'a str, &'a Component), Error> {
    let key = find_component_key(manifest, name)?;
    let key_ref = manifest
        .components
        .keys()
        .find(|k| **k == key)
        .expect("key was just found via find_component_key");
    Ok((key_ref, &manifest.components[&key]))
}

/// Resolve a foundation dependency name to its file path.
pub fn resolve_foundation<'a>(manifest: &'a Manifest, dep: &str) -> Option<&'a str> {
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

/// Parse YAML frontmatter string into a [`Frontmatter`].
///
/// # Errors
///
/// Returns [`ManifestError::FrontmatterError`] on malformed YAML.
pub fn parse_frontmatter_yaml(yaml_str: &str) -> Result<Frontmatter, Error> {
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
        let yaml =
            "component: Checkbox\ncategory: input\nfoundation_deps: [architecture, accessibility]";
        let fm = parse_frontmatter_yaml(yaml).expect("should parse");
        assert_eq!(fm.component.as_deref(), Some("Checkbox"));
        assert_eq!(fm.category.as_deref(), Some("input"));
        assert_eq!(
            fm.foundation_deps.as_deref(),
            Some(&["architecture".to_string(), "accessibility".to_string()][..])
        );
    }
}
