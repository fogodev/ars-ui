//! `spec validate` — validate frontmatter against manifest.

use std::{fmt::Write, fs};

use crate::manifest::{self, Error, SpecRoot};

/// Validate all spec file frontmatter against manifest entries.
///
/// # Errors
///
/// Returns [`ManifestError::Io`] if a spec file cannot be read.
pub fn execute(root: &SpecRoot) -> Result<String, Error> {
    let m = &root.manifest;
    let mut errors = Vec::new();
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
                    if let Some(fm_cat) = &fm.category
                        && fm_cat != &comp.category
                    {
                        errors.push(format!(
                            "[{name}] category mismatch: manifest='{}' file='{fm_cat}'",
                            comp.category
                        ));
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
                errors.push(format!(
                    "[{framework}:{name}] adapter file not found: {path}"
                ));
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
