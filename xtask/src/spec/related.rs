//! `spec related` — list a component and its related components.

use std::fmt::Write;

use crate::manifest::{self, ManifestError, SpecRoot};

/// Return the component and all its related components with deps.
///
/// # Errors
///
/// Returns [`ManifestError::ComponentNotFound`] if the component is not in the manifest.
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
                    writeln!(
                        out,
                        "foundation_deps: {}",
                        rel_comp.foundation_deps.join(", ")
                    )
                    .expect("write to String");
                }
                if !rel_comp.shared_deps.is_empty() {
                    writeln!(out, "shared_deps: {}", rel_comp.shared_deps.join(", "))
                        .expect("write to String");
                }
            } else {
                writeln!(out, "### {rel}").expect("write to String");
                writeln!(out, "# WARNING: not found in manifest").expect("write to String");
            }
        }
    }
    Ok(out)
}
