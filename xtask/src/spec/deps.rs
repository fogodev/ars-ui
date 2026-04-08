//! `spec deps` — list all files needed to review a component.

use std::fmt::Write;

use crate::manifest::{self, Error, SpecRoot};

/// Return the file set for reviewing a component.
///
/// # Errors
///
/// Returns [`ManifestError::ComponentNotFound`] if the component is not in the manifest.
pub fn execute(root: &SpecRoot, component: &str) -> Result<String, Error> {
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
                writeln!(out, "# WARNING: unknown foundation dep '{dep}'")
                    .expect("write to String");
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
                writeln!(out, "# WARNING: unknown related component '{rel}'")
                    .expect("write to String");
            }
        }
        writeln!(out).expect("write to String");
    }
    Ok(out)
}
