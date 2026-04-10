//! `spec reverse` — find components depending on a shared type.

use std::fmt::Write;

use crate::manifest::{Error, SpecRoot};

/// Return components that depend on a shared type.
///
/// # Errors
///
/// Returns [`ManifestError::SharedTypeNotFound`] if the shared type is not in the manifest.
pub fn execute(root: &SpecRoot, shared_type: &str) -> Result<String, Error> {
    let m = &root.manifest;
    if !m.shared.contains_key(shared_type) {
        return Err(Error::SharedTypeNotFound {
            name: shared_type.to_string(),
            available: m.shared.keys().cloned().collect(),
        });
    }
    let dependents = m
        .components
        .iter()
        .filter(|(_, c)| c.shared_deps.iter().any(|d| d == shared_type))
        .collect::<Vec<_>>();
    let mut out = String::new();
    writeln!(out, "# Components depending on shared/{shared_type}").expect("write to String");
    writeln!(out).expect("write to String");
    if dependents.is_empty() {
        writeln!(out, "(none)").expect("write to String");
    } else {
        writeln!(out, "## Shared type file").expect("write to String");
        writeln!(out, "{}", m.shared[shared_type]).expect("write to String");
        writeln!(out).expect("write to String");
        writeln!(out, "## Dependent components ({} total)", dependents.len())
            .expect("write to String");
        for (key, comp) in &dependents {
            writeln!(out, "  {key}: {}", comp.path).expect("write to String");
        }
    }
    Ok(out)
}
