//! Mutation-testing orchestration via [`cargo-mutants`](https://mutants.rs).
//!
//! Powers `cargo xmutants`, applying the same per-crate `--features` profile
//! as the Nightly CI mutation matrix so local runs do not fail on snapshot or
//! i18n-gated code paths that plain `cargo mutants` misses.

use std::{
    fmt::{self, Display},
    io,
    path::PathBuf,
    process::{self, Stdio},
};

/// Pinned `cargo-mutants` version — keep in sync with `nightly.yml` and
/// `AGENTS.md`.
pub const CARGO_MUTANTS_VERSION: &str = "27.0.0";

/// Options for a local mutation-testing run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Crate package name (for example `ars-components`).
    pub package: String,

    /// Optional source file limiting the mutation set.
    pub file: Option<PathBuf>,

    /// When `true`, list mutants without executing tests.
    pub list: bool,

    /// Optional shard selector (`k/n`, zero-based `k`).
    pub shard: Option<String>,

    /// Output directory forwarded to `cargo mutants --output`.
    pub output: PathBuf,

    /// Override the workspace default feature string for this package.
    pub features: Option<String>,
}

/// Errors returned by `cargo xmutants`.
#[derive(Debug)]
pub enum Error {
    /// `cargo-mutants` is not installed or not on `PATH`.
    MissingTool {
        /// Human-readable tool name.
        tool: String,

        /// Suggested install command.
        install_hint: String,
    },

    /// The subprocess exited unsuccessfully.
    CommandFailed {
        /// Exit code, if available.
        code: Option<i32>,
    },

    /// IO error while spawning the subprocess.
    Io(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTool { tool, install_hint } => {
                write!(
                    f,
                    "missing required tool: {tool}\n  install: {install_hint}"
                )
            }

            Self::CommandFailed { code } => {
                write!(f, "cargo mutants failed")?;

                if let Some(code) = code {
                    write!(f, " (exit {code})")?;
                }

                Ok(())
            }

            Self::Io(err) => err.fmt(f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Returns the default `--features` string for `package`, matching the Nightly
/// mutation-testing matrix. `None` means no `--features` flag is forwarded.
#[must_use]
pub fn default_features(package: &str) -> Option<&'static str> {
    match package {
        "ars-components" => Some("ars-components/i18n"),
        _ => None,
    }
}

/// Runs `cargo mutants` with workspace-aligned feature flags.
///
/// # Errors
///
/// Returns [`Error::MissingTool`] when `cargo-mutants` is unavailable,
/// [`Error::CommandFailed`] when the subprocess exits non-zero, or
/// [`Error::Io`] on spawn failures.
pub fn run(options: &Options) -> Result<(), Error> {
    preflight_cargo_mutants()?;

    let mut command = process::Command::new("cargo");
    command
        .arg("mutants")
        .arg("-p")
        .arg(&options.package)
        .arg("--output")
        .arg(&options.output);

    if let Some(file) = &options.file {
        command.arg("-f").arg(file);
    }

    if options.list {
        command.arg("--list");
    }

    if let Some(shard) = &options.shard {
        command.arg("--shard").arg(shard);
    }

    let features = options
        .features
        .as_deref()
        .or_else(|| default_features(&options.package));

    if let Some(features) = features
        && !features.is_empty()
    {
        command.arg("--features").arg(features);
    }

    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    let status = command.status()?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::CommandFailed {
            code: status.code(),
        })
    }
}

fn preflight_cargo_mutants() -> Result<(), Error> {
    let output = process::Command::new("cargo")
        .args(["mutants", "--version"])
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    Err(Error::MissingTool {
        tool: "cargo-mutants".into(),
        install_hint: format!(
            "cargo install cargo-mutants --locked --version {CARGO_MUTANTS_VERSION}"
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_features_matches_nightly_matrix() {
        assert_eq!(
            default_features("ars-components"),
            Some("ars-components/i18n")
        );
        assert_eq!(default_features("ars-collections"), None);
        assert_eq!(default_features("ars-core"), None);
        assert_eq!(default_features("ars-a11y"), None);
        assert_eq!(default_features("ars-forms"), None);
        assert_eq!(default_features("ars-interactions"), None);
    }

    #[test]
    fn cargo_mutants_version_matches_nightly_pin() {
        assert_eq!(CARGO_MUTANTS_VERSION, "27.0.0");
    }
}
