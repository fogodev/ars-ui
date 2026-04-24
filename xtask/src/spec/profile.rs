//! `spec profile` — list files for a review profile.

use std::fmt::Write;

use crate::manifest::{Error, SpecRoot};

/// Return files in a named review profile.
///
/// # Errors
///
/// Returns [`ManifestError::ProfileNotFound`] if the profile name does not exist.
pub fn execute(root: &SpecRoot, name: &str) -> Result<String, Error> {
    let profiles =
        root.manifest
            .review_profiles
            .as_ref()
            .ok_or_else(|| Error::ProfileNotFound {
                name: name.to_string(),
                available: vec![],
            })?;

    let profile = profiles
        .get(name)
        .or_else(|| profiles.get(&name.replace('-', "_")))
        .or_else(|| profiles.get(&name.replace('_', "-")))
        .ok_or_else(|| Error::ProfileNotFound {
            name: name.to_string(),
            available: profiles.keys().cloned().collect(),
        })?;

    let mut out = String::new();

    writeln!(out, "# Review profile: {name}").expect("write to String");
    writeln!(out).expect("write to String");
    writeln!(out, "## Files always loaded").expect("write to String");

    for file in &profile.files_always {
        writeln!(out, "{file}").expect("write to String");
    }

    Ok(out)
}
