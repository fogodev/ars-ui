//! `spec_digest` — compact component summary extraction.
//!
//! Parses a component's spec file and extracts content under well-known section
//! headings to produce a condensed overview suitable for LLM context windows.

use std::{fmt::Write, fs};

use crate::manifest::{self, Error, SpecRoot};

/// Extract a compact digest of a component's key sections.
///
/// Reads the component spec file and extracts content under known section
/// headings (State Machine, Accessibility, Props, Events, etc.).
///
/// # Errors
///
/// Returns [`ManifestError`] if the component is not found or the file cannot be read.
pub fn execute(root: &SpecRoot, component: &str) -> Result<String, Error> {
    let (key, comp) = manifest::find_component(&root.manifest, component)?;
    let file_path = root.path.join(&comp.path);
    let content = fs::read_to_string(&file_path).map_err(Error::Io)?;

    let mut out = String::new();
    writeln!(out, "# Digest: {component}").expect("write to String");
    writeln!(out, "path: {}", comp.path).expect("write to String");
    writeln!(out, "category: {}", comp.category).expect("write to String");
    writeln!(
        out,
        "foundation_deps: [{}]",
        comp.foundation_deps.join(", ")
    )
    .expect("write to String");
    writeln!(out, "shared_deps: [{}]", comp.shared_deps.join(", ")).expect("write to String");
    if let Some(path) = root.manifest.leptos_adapters.get(key) {
        writeln!(out, "leptos_adapter: {path}").expect("write to String");
    }
    if let Some(path) = root.manifest.dioxus_adapters.get(key) {
        writeln!(out, "dioxus_adapter: {path}").expect("write to String");
    }
    writeln!(out).expect("write to String");

    // Sections to extract, with heading prefixes to match
    let sections_of_interest: &[(&str, &[&str])] = &[
        ("States", &["1.1 States"]),
        ("Events", &["1.2 Events"]),
        ("Context", &["1.3 Context"]),
        ("Props", &["1.4 Props", "1.1 Props"]),
        ("Connect / API", &["Connect"]),
        ("Anatomy", &["2. Anatomy"]),
        ("Accessibility", &["3. Accessibility", "2. Accessibility"]),
        ("Internationalization", &["Internationalization"]),
        ("Form Integration", &["Form Integration"]),
    ];

    for (label, heading_prefixes) in sections_of_interest {
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
    let lines = content.lines().collect::<Vec<_>>();
    let mut start_idx = None;
    let mut start_level = 0;

    for (i, line) in lines.iter().enumerate() {
        if let Some((level, text)) = manifest::parse_heading(line)
            && heading_prefixes.iter().any(|prefix| text.contains(prefix))
        {
            start_idx = Some(i + 1);
            start_level = level;
            break;
        }
    }

    let start = start_idx?;
    let mut end = lines.len();

    for (i, line) in lines.iter().enumerate().skip(start) {
        if let Some((level, _)) = manifest::parse_heading(line)
            && level <= start_level
        {
            end = i;
            break;
        }
    }

    let section = lines[start..end].join("\n");
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
        let result = extract_section(content, &["1.1 States"]);
        assert_eq!(result.as_deref(), Some("state A\nstate B"));
    }

    #[test]
    fn extract_section_not_found() {
        let content = "# Title\n## 2. Anatomy\nstuff";
        let result = extract_section(content, &["1.1 States"]);
        assert!(result.is_none());
    }

    #[test]
    fn extract_section_multiple_prefixes() {
        let content = "# Title\n## 2. Accessibility\nrole: checkbox\n## 3. Other\nstuff";
        let result = extract_section(content, &["3. Accessibility", "2. Accessibility"]);
        assert_eq!(result.as_deref(), Some("role: checkbox"));
    }

    #[test]
    fn extract_section_empty_content() {
        let content = "# Title\n## 1.1 States\n## 1.2 Events\nevent X";
        let result = extract_section(content, &["1.1 States"]);
        assert!(result.is_none());
    }
}
