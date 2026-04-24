//! `spec category` — list all components in a category.

use std::fmt::Write;

use crate::manifest::{self, Error, SpecRoot};

/// Return all components in a category with metadata.
///
/// # Errors
///
/// Returns [`ManifestError::CategoryNotFound`] if no components match the category.
pub fn execute(root: &SpecRoot, name: &str) -> Result<String, Error> {
    let m = &root.manifest;

    let components = m
        .components
        .iter()
        .filter(|(_, c)| c.category == name)
        .collect::<Vec<_>>();

    if components.is_empty() {
        let mut cats = m
            .components
            .values()
            .map(|c| c.category.as_str())
            .collect::<Vec<_>>();

        cats.sort();

        cats.dedup();

        return Err(Error::CategoryNotFound {
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
            writeln!(out, "foundation_deps: {}", comp.foundation_deps.join(", "))
                .expect("write to String");
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
