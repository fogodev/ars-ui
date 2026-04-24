//! `spec info` — show component metadata.

use std::fmt::Write;

use crate::manifest::{self, Error, SpecRoot};

/// Return component metadata as text.
///
/// # Errors
///
/// Returns [`ManifestError::ComponentNotFound`] if the component is not in the manifest.
pub fn execute(root: &SpecRoot, component: &str) -> Result<String, Error> {
    let (key, comp) = manifest::find_component(&root.manifest, component)?;

    let mut out = String::new();

    writeln!(out, "component: {component}").expect("write to String");
    writeln!(out, "path: {}", comp.path).expect("write to String");
    writeln!(out, "category: {}", comp.category).expect("write to String");

    writeln!(
        out,
        "foundation_deps: [{}]",
        comp.foundation_deps.join(", ")
    )
    .expect("write to String");

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
