//! `spec adapters` — list adapter files for a framework.

use std::{collections::BTreeMap, fmt::Write};

use crate::manifest::{Error, SpecRoot};

/// Return all adapter files for a framework, grouped by category.
///
/// # Errors
///
/// Returns [`ManifestError::UnknownFramework`] if the framework is not `"leptos"` or `"dioxus"`.
pub fn execute(root: &SpecRoot, framework: &str) -> Result<String, Error> {
    let m = &root.manifest;
    let adapters = match framework {
        "leptos" => &m.leptos_adapters,
        "dioxus" => &m.dioxus_adapters,
        _ => return Err(Error::UnknownFramework(framework.to_string())),
    };
    if adapters.is_empty() {
        return Ok(format!("No {framework} adapter files registered.\n"));
    }
    let mut by_category = BTreeMap::<_, Vec<_>>::new();
    for (name, path) in adapters {
        let category = m
            .components
            .get(name)
            .map_or_else(|| "uncategorized".to_string(), |c| c.category.clone());
        by_category.entry(category).or_default().push((name, path));
    }
    let mut out = String::new();
    writeln!(
        out,
        "# {framework} adapter files ({} total)",
        adapters.len()
    )
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
