//! `spec toc` — output heading structure of a spec file.

use std::{fmt::Write, fs, path::Path};

use crate::manifest::{self, Error, SpecRoot};

/// Return the heading structure (table of contents) of a spec file.
///
/// # Errors
///
/// Returns [`ManifestError::Io`] if the file cannot be read.
pub fn execute(root: &SpecRoot, file: &str) -> Result<String, Error> {
    let file_path = if Path::new(file).is_absolute() || Path::new(file).exists() {
        Path::new(file).to_path_buf()
    } else {
        root.path.join(file)
    };

    let content = fs::read_to_string(&file_path).map_err(Error::Io)?;

    let mut out = String::new();

    for (line_num, line) in content.lines().enumerate() {
        if let Some((level, text)) = manifest::parse_heading(line) {
            let indent = match level {
                1 => "",
                2 => "  ",
                3 => "    ",
                4 => "      ",
                _ => "        ",
            };

            let prefix = "#".repeat(level as usize);

            writeln!(out, "{indent}{prefix} {text}  (L{})", line_num + 1).expect("write to String");
        }
    }

    Ok(out)
}
