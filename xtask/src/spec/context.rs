//! `spec_context` — dependency-aware context loading.

use std::{fmt::Write, fs, path::Path};

use crate::manifest::{self, ManifestError, SpecRoot};

/// Return the full implementation context for a component.
///
/// Concatenates the component spec with all its dependencies (foundation,
/// shared, adapter) with file-boundary markers.
///
/// # Errors
///
/// Returns `ManifestError` if the component is not found or files cannot be read.
pub fn execute(
    root: &SpecRoot,
    component: &str,
    framework: Option<&str>,
    include_testing: bool,
) -> Result<String, ManifestError> {
    let (key, comp) = manifest::find_component(&root.manifest, component)?;
    let m = &root.manifest;
    let mut out = String::new();

    writeln!(out, "# Implementation context for {component}").expect("write to String");
    writeln!(out).expect("write to String");

    // 1. Foundation deps
    for dep in &comp.foundation_deps {
        if let Some(path) = manifest::resolve_foundation(m, dep) {
            append_file(&root.path, path, &mut out);
        }
    }

    // 2. Shared deps
    for dep in &comp.shared_deps {
        if let Some(path) = m.shared.get(dep) {
            append_file(&root.path, path, &mut out);
        }
    }

    // 3. Component spec
    append_file(&root.path, &comp.path, &mut out);

    // 4. Framework adapter spec
    if let Some(fw) = framework {
        let adapters = match fw {
            "leptos" => &m.leptos_adapters,
            "dioxus" => &m.dioxus_adapters,
            _ => return Err(ManifestError::UnknownFramework(fw.to_string())),
        };
        if let Some(path) = adapters.get(key) {
            append_file(&root.path, path, &mut out);
        }
    }

    // 5. Testing spec (if requested)
    if include_testing {
        let testing_overview = "testing/00-overview.md";
        let full = root.path.join(testing_overview);
        if full.exists() {
            append_file(&root.path, testing_overview, &mut out);
        }
    }

    Ok(out)
}

/// Append a file's content with a boundary marker.
fn append_file(spec_root: &Path, rel_path: &str, out: &mut String) {
    let full_path = spec_root.join(rel_path);
    match fs::read_to_string(&full_path) {
        Ok(content) => {
            writeln!(out, "--- FILE: {rel_path} ---").expect("write to String");
            writeln!(out, "{content}").expect("write to String");
            writeln!(out).expect("write to String");
        }
        Err(e) => {
            writeln!(out, "--- FILE: {rel_path} (ERROR: {e}) ---").expect("write to String");
            writeln!(out).expect("write to String");
        }
    }
}
