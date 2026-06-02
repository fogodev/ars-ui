//! `spec validate` — validate frontmatter against manifest.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    fs,
};

use crate::{
    manifest::{self, ComponentDependencyKind, Error, SpecRoot},
    spec::component_deps,
};

/// Validate all spec file frontmatter against manifest entries.
///
/// # Errors
///
/// Returns [`ManifestError::Io`] if a spec file cannot be read.
pub fn execute(root: &SpecRoot) -> Result<String, Error> {
    let m = &root.manifest;

    let mut errors = Vec::new();
    validate_component_dependencies(root, &mut errors);

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

fn validate_component_dependencies(root: &SpecRoot, errors: &mut Vec<String>) {
    let components = &root.manifest.components;
    let valid_frameworks = BTreeSet::from(["leptos", "dioxus"]);
    let mut requires_edges = BTreeMap::<String, Vec<String>>::new();

    for (name, comp) in components {
        for dep in &comp.component_deps {
            if !components.contains_key(&dep.component) {
                errors.push(format!(
                    "[{name}] unknown component_dep component '{}'",
                    dep.component
                ));
            }

            if dep.reason.trim().is_empty() {
                errors.push(format!(
                    "[{name}] component_dep '{}' has an empty reason",
                    dep.component
                ));
            }

            if dep.frameworks.is_empty() {
                errors.push(format!(
                    "[{name}] component_dep '{}' must list leptos and/or dioxus",
                    dep.component
                ));
            }

            for framework in &dep.frameworks {
                if !valid_frameworks.contains(framework.as_str()) {
                    errors.push(format!(
                        "[{name}] component_dep '{}' has invalid framework '{framework}'",
                        dep.component
                    ));
                }
            }

            if matches!(
                dep.kind,
                ComponentDependencyKind::Boundary | ComponentDependencyKind::Related
            ) && dep.blocking
            {
                errors.push(format!(
                    "[{name}] {} component_dep '{}' cannot be blocking",
                    dep.kind, dep.component
                ));
            }

            if dep.kind == ComponentDependencyKind::Requires {
                if dep.component == *name {
                    errors.push(format!(
                        "[{name}] requires component_dep cannot point to itself"
                    ));
                }

                requires_edges
                    .entry(name.clone())
                    .or_default()
                    .push(dep.component.clone());
            }

            if !dep.kind.can_block() && component_deps::is_blocking(dep.kind, dep.blocking) {
                errors.push(format!(
                    "[{name}] {} component_dep '{}' unexpectedly blocks",
                    dep.kind, dep.component
                ));
            }
        }
    }

    for start in components.keys() {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        detect_requires_cycle(
            start,
            start,
            &requires_edges,
            &mut visiting,
            &mut visited,
            errors,
        );
    }
}

fn detect_requires_cycle(
    start: &str,
    current: &str,
    edges: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    if !visiting.insert(current.to_owned()) {
        return;
    }

    if let Some(nexts) = edges.get(current) {
        for next in nexts {
            if next == start {
                errors.push(format!(
                    "[{start}] requires component_deps contain a cycle through '{current}'"
                ));
                continue;
            }

            if !visited.contains(next) {
                detect_requires_cycle(start, next, edges, visiting, visited, errors);
            }
        }
    }

    visiting.remove(current);
    visited.insert(current.to_owned());
}
