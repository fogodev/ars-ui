//! CRAP-metric gate via [`cargo-crap`](https://crates.io/crates/cargo-crap).
//!
//! Powers `cargo xcrap` and the `crap` CI step. The CRAP score
//! (`complexity² · (1 − coverage)³ + complexity`) flags functions that are both
//! cyclomatically complex and poorly covered.
//!
//! ## Why regression mode, not an absolute threshold
//!
//! At 100% coverage `CRAP == cyclomatic complexity`, so an absolute
//! `--fail-above` threshold rejects every large `match` regardless of how well
//! it is tested. This workspace is built on state machines: each stateful
//! component has a `Machine::transition` that is intrinsically a wide
//! `match (state, event)` with complexity 30–80, all exhaustively tested. An
//! absolute gate would fight that architecture and fail more components as the
//! catalog grows.
//!
//! Instead the gate runs in **regression mode**: it compares against a committed
//! [`BASELINE_PATH`] and fails only when an *already-complex* function
//! (`--min` [`CRAP_THRESHOLD`]) gets *worse* than the baseline — more complexity
//! or less coverage. Brand-new functions are tolerated (the per-crate coverage
//! thresholds already ensure new code is tested); routine edits to simple
//! functions below the threshold are ignored. The net effect: the coverage gate
//! guarantees new code is tested, and this gate guarantees shipped code does not
//! rot. The gate reuses the merged `lcov.info` the coverage step produces, so it
//! never recompiles.

use std::{
    fmt::{self, Display},
    io,
    path::{Path, PathBuf},
    process::{self, Stdio},
};

/// Pinned `cargo-crap` version — keep in sync with `.github/workflows/ci.yml`
/// and the CLAUDE.md note. Pinning is deliberate: cargo-crap's scoring can
/// change between releases, and an unpinned install would silently shift the
/// gate without a code change.
pub const CARGO_CRAP_VERSION: &str = "0.2.2";

/// CRAP score at or above which a function is tracked by the gate (forwarded as
/// `--min`) and marked "crappy" in reports (`--threshold`). Matches cargo-crap's
/// built-in default and the canonical `Crap4J` cutoff. Functions below this are
/// ignored by the regression gate, so routine edits to simple code never fail
/// CI.
pub const CRAP_THRESHOLD: u32 = 30;

/// Repo-root path of the committed CRAP baseline. The gate fails when a tracked
/// function regresses against this file; regenerate it with
/// `cargo xcrap --update-baseline` (from CI's merged `lcov.info`) when a
/// deliberate complexity increase lands.
pub const BASELINE_PATH: &str = ".crap-baseline.json";

/// Files skipped at walk time (`--exclude`, relative to `--path`). Honored
/// because the gate runs under `--path .` (not `--workspace`, where `--exclude`
/// is a no-op). Every entry must carry a justification.
const EXCLUDE_GLOBS: &[&str] = &[
    // Build artifacts and generated output — never first-party source.
    "target/**",
    // Demo/example apps are not shipped library surface; their complexity is not
    // a maintenance-risk concern the gate should track. `--workspace` skipped
    // them (non-member walk); `--path .` would otherwise sweep them in.
    "examples/**",
    // `ars-derive` is a proc-macro crate: its code runs inside the compiler, so
    // `cargo-llvm-cov` never instruments it. Every function would otherwise be
    // scored at 0% coverage and dominate the baseline with noise. It is
    // exercised via `crates/ars-core/tests/derive_contract.rs`.
    "crates/ars-derive/**",
];

/// What the cargo-crap invocation should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// CI gate: fail if any tracked (`--min` [`CRAP_THRESHOLD`]) function's CRAP
    /// score regressed against the baseline.
    Gate,

    /// Local human-readable report. Never fails the process.
    Report,

    /// (Re)generate the baseline JSON from the current lcov.
    UpdateBaseline,
}

/// Options for a cargo-crap invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// What to do.
    pub action: Action,

    /// Path to the lcov coverage report (typically `lcov.info`).
    pub lcov: PathBuf,

    /// Path to the baseline JSON (typically [`BASELINE_PATH`]).
    pub baseline: PathBuf,

    /// CRAP score cutoff forwarded as `--min` (gate) or `--threshold` (report).
    pub threshold: u32,
}

/// Errors returned by the CRAP gate.
#[derive(Debug)]
pub enum Error {
    /// `cargo-crap` is not installed or not on `PATH`.
    MissingTool {
        /// Human-readable tool name.
        tool: String,

        /// Suggested install command.
        install_hint: String,
    },

    /// The lcov coverage report does not exist.
    MissingLcov {
        /// The path that was expected to hold the report.
        path: PathBuf,
    },

    /// The subprocess exited unsuccessfully (a CRAP regression, or a cargo-crap
    /// error).
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

            Self::MissingLcov { path } => {
                write!(
                    f,
                    "coverage report not found: {}\n  run `cargo xci coverage` first to generate it",
                    path.display()
                )
            }

            Self::CommandFailed { code } => {
                write!(f, "cargo crap reported a CRAP regression")?;

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

/// Builds the `cargo crap …` argument vector for `options`.
///
/// The leading `"crap"` is the subcommand name cargo injects when a binary is
/// invoked as `cargo crap`; cargo-crap tolerates it. Split out as a pure
/// function so the argument surface is unit-testable.
fn build_args(options: &Options) -> Vec<String> {
    // `--path .` (not `--workspace`) so cargo-crap emits repo-relative paths
    // (`./crates/...`). Absolute `--workspace` paths bind the baseline to the
    // machine that generated it (`/Users/...` locally vs `/home/runner/...` on
    // CI), making every `(file, function, line)` key mismatch on CI — which
    // silently turns the regression gate into a no-op. Relative paths match on
    // any checkout. `--exclude` is honored under `--path` (it is a no-op only
    // under `--workspace`).
    let mut args = vec![
        "crap".to_owned(),
        "--path".to_owned(),
        ".".to_owned(),
        "--lcov".to_owned(),
        options.lcov.display().to_string(),
    ];

    match options.action {
        Action::Gate => {
            // Track only functions at/above the threshold (`--min`) and fail
            // when one of them regressed against the baseline. New functions and
            // sub-threshold churn are tolerated.
            args.push("--baseline".to_owned());
            args.push(options.baseline.display().to_string());
            args.push("--fail-regression".to_owned());
            args.push("--min".to_owned());
            args.push(options.threshold.to_string());
        }

        Action::Report => {
            args.push("--threshold".to_owned());
            args.push(options.threshold.to_string());
        }

        Action::UpdateBaseline => {
            // A complete baseline (no `--min`) is required: a function that
            // crosses from below the threshold to above it must be present in
            // the baseline at its old score, or the increase reads as "new" and
            // escapes the gate.
            args.push("--format".to_owned());
            args.push("json".to_owned());
            args.push("--output".to_owned());
            args.push(options.baseline.display().to_string());
        }
    }

    for glob in EXCLUDE_GLOBS {
        args.push("--exclude".to_owned());
        args.push((*glob).to_owned());
    }

    args
}

/// Runs cargo-crap for `options`.
///
/// In [`Action::Gate`] mode, a missing baseline is bootstrapped: the baseline is
/// generated from the current lcov and the gate passes with a notice, so the
/// first run on a repo without a committed baseline does not fail spuriously.
///
/// # Errors
///
/// Returns [`Error::MissingTool`] when `cargo-crap` is unavailable,
/// [`Error::MissingLcov`] when the coverage report is absent,
/// [`Error::CommandFailed`] when the subprocess exits non-zero (a CRAP
/// regression), or [`Error::Io`] on spawn failures.
pub fn run(options: &Options) -> Result<(), Error> {
    preflight_cargo_crap()?;
    preflight_lcov(&options.lcov)?;

    if options.action == Action::Gate && !options.baseline.exists() {
        eprintln!(
            "note: baseline {} not found; generating it from {} (gate is advisory until committed)",
            options.baseline.display(),
            options.lcov.display()
        );

        let bootstrap = Options {
            action: Action::UpdateBaseline,
            ..options.clone()
        };

        run_cargo_crap(&bootstrap)?;

        eprintln!(
            "note: wrote {} — commit it to activate the regression gate",
            options.baseline.display()
        );

        return Ok(());
    }

    run_cargo_crap(options)
}

fn run_cargo_crap(options: &Options) -> Result<(), Error> {
    let args = build_args(options);

    eprintln!("  > cargo {}", args.join(" "));

    let status = process::Command::new("cargo")
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::CommandFailed {
            code: status.code(),
        })
    }
}

fn preflight_cargo_crap() -> Result<(), Error> {
    let output = process::Command::new("cargo")
        .args(["crap", "--version"])
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    Err(Error::MissingTool {
        tool: "cargo-crap".into(),
        install_hint: format!("cargo install cargo-crap --locked --version {CARGO_CRAP_VERSION}"),
    })
}

fn preflight_lcov(lcov: &Path) -> Result<(), Error> {
    if lcov.exists() {
        Ok(())
    } else {
        Err(Error::MissingLcov {
            path: lcov.to_path_buf(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(action: Action) -> Options {
        Options {
            action,
            lcov: PathBuf::from("lcov.info"),
            baseline: PathBuf::from(BASELINE_PATH),
            threshold: CRAP_THRESHOLD,
        }
    }

    fn values_after<'a>(args: &'a [String], flag: &str) -> Vec<&'a str> {
        args.windows(2)
            .filter(|pair| pair[0] == flag)
            .map(|pair| pair[1].as_str())
            .collect()
    }

    fn has_flag(args: &[String], flag: &str) -> bool {
        args.iter().any(|arg| arg == flag)
    }

    #[test]
    fn cargo_crap_version_matches_ci_pin() {
        assert_eq!(CARGO_CRAP_VERSION, "0.2.2");
    }

    #[test]
    fn threshold_is_thirty() {
        assert_eq!(CRAP_THRESHOLD, 30);
    }

    #[test]
    fn all_actions_use_relative_path_and_lcov_not_workspace() {
        for action in [Action::Gate, Action::Report, Action::UpdateBaseline] {
            let args = build_args(&options(action));

            assert_eq!(args[0], "crap");
            // Portable relative paths require `--path .`, never `--workspace`
            // (which emits machine-absolute paths and breaks the baseline on CI).
            assert_eq!(values_after(&args, "--path"), vec!["."]);
            assert!(!has_flag(&args, "--workspace"));
            assert_eq!(values_after(&args, "--lcov"), vec!["lcov.info"]);
        }
    }

    #[test]
    fn gate_uses_baseline_regression_and_min() {
        let args = build_args(&options(Action::Gate));

        assert_eq!(values_after(&args, "--baseline"), vec![BASELINE_PATH]);
        assert!(has_flag(&args, "--fail-regression"));
        assert_eq!(values_after(&args, "--min"), vec!["30"]);
        // The gate must never use the absolute fail-above mode.
        assert!(!has_flag(&args, "--fail-above"));
    }

    #[test]
    fn report_uses_threshold_and_never_fails() {
        let args = build_args(&options(Action::Report));

        assert_eq!(values_after(&args, "--threshold"), vec!["30"]);
        assert!(!has_flag(&args, "--fail-regression"));
        assert!(!has_flag(&args, "--fail-above"));
        assert!(!has_flag(&args, "--baseline"));
    }

    #[test]
    fn update_baseline_writes_complete_json_without_min() {
        let args = build_args(&options(Action::UpdateBaseline));

        assert_eq!(values_after(&args, "--format"), vec!["json"]);
        assert_eq!(values_after(&args, "--output"), vec![BASELINE_PATH]);
        assert!(!has_flag(&args, "--fail-regression"));
        // No `--min`: the baseline must record every function, including
        // sub-threshold ones that could later cross the threshold.
        assert!(!has_flag(&args, "--min"));
    }

    #[test]
    fn every_action_forwards_exclude_globs() {
        for action in [Action::Gate, Action::Report, Action::UpdateBaseline] {
            let args = build_args(&options(action));
            let excludes = values_after(&args, "--exclude");

            for glob in EXCLUDE_GLOBS {
                assert!(
                    excludes.contains(glob),
                    "{action:?} missing --exclude {glob}"
                );
            }

            assert_eq!(excludes.len(), EXCLUDE_GLOBS.len());
            // `--exclude` is honored only under `--path`; never `--allow` here.
            assert!(!has_flag(&args, "--allow"));
        }
    }

    #[test]
    fn exclude_globs_skip_target_and_unmeasured_proc_macro_crate() {
        assert!(
            EXCLUDE_GLOBS.contains(&"crates/ars-derive/**"),
            "ars-derive is unmeasured by llvm-cov and must be excluded"
        );
        assert!(
            EXCLUDE_GLOBS.contains(&"target/**"),
            "build artifacts must be excluded"
        );
    }

    #[test]
    fn committed_baseline_uses_portable_relative_paths() {
        // Guards Codex review #700's P1: a baseline with absolute paths
        // (`/Users/...` or `/home/runner/...`) silently no-ops the gate on CI.
        // Every entry's `file` must be repo-relative.
        let baseline = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(BASELINE_PATH);

        let content =
            std::fs::read_to_string(&baseline).expect("committed .crap-baseline.json must exist");

        let parsed: serde_json::Value =
            serde_json::from_str(&content).expect("baseline must be valid JSON");

        let entries = parsed["entries"]
            .as_array()
            .expect("baseline must have an entries array");

        assert!(!entries.is_empty(), "baseline must not be empty");

        for entry in entries {
            let file = entry["file"].as_str().expect("entry.file must be a string");
            assert!(
                !file.starts_with('/'),
                "baseline entry has a machine-absolute path (breaks the gate on CI): {file}"
            );
        }
    }

    #[test]
    fn error_display_is_actionable() {
        let missing_tool = Error::MissingTool {
            tool: "cargo-crap".into(),
            install_hint: "cargo install cargo-crap".into(),
        }
        .to_string();

        assert!(missing_tool.contains("cargo-crap"));
        assert!(missing_tool.contains("install"));

        let missing_lcov = Error::MissingLcov {
            path: PathBuf::from("lcov.info"),
        }
        .to_string();

        assert!(missing_lcov.contains("lcov.info"));
        assert!(missing_lcov.contains("cargo xci coverage"));

        let failed = Error::CommandFailed { code: Some(1) }.to_string();

        assert!(failed.contains("regression"));
        assert!(failed.contains("exit 1"));

        // No exit code → no parenthetical.
        assert!(
            !Error::CommandFailed { code: None }
                .to_string()
                .contains("exit")
        );
    }
}
