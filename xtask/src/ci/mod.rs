//! Local CI pipeline runner.
//!
//! Mirrors the jobs declared in `.github/workflows/ci.yml` so that the xtask
//! crate is the single source of truth for CI commands. Run the full pipeline
//! with `cargo xci` or pick individual steps with `cargo xci check clippy`.

pub(crate) mod feature_matrix;

use std::{fmt, io, path::Path, process};

/// CI pipeline steps, matching the GitHub Actions job names.
///
/// Steps are listed in pipeline dependency order for sequential local
/// execution. Use [`run`] to execute one or more steps with fail-fast
/// semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum Step {
    /// `cargo +nightly fmt --all --check`
    Fmt,
    /// `cargo check --workspace --all-features`
    Check,
    /// `cargo clippy --workspace --all-targets --all-features -- -D warnings`
    Clippy,
    /// Unit tests for core crates.
    Unit,
    /// Integration tests.
    Integration,
    /// Adapter harness tests (Leptos + Dioxus).
    Adapter,
    /// Generate coverage and check per-crate thresholds.
    Coverage,
    /// Meta-step: run all five feature-matrix groups.
    FeatureMatrix,
    /// Feature flags — ars-core (15 combos).
    FeatureMatrixCore,
    /// Feature flags — ars-i18n (11 combos + wasm32).
    FeatureMatrixI18n,
    /// Feature flags — subsystem crates (13 combos + wasm32).
    FeatureMatrixSubsystems,
    /// Feature flags — ars-leptos (3 combos).
    FeatureMatrixLeptos,
    /// Feature flags — ars-dioxus (4 combos + wasm32).
    FeatureMatrixDioxus,
}

/// Default pipeline order when no steps are specified.
///
/// `FeatureMatrix` is intentionally absent — the pipeline runs each sub-group
/// individually so progress is visible per group.
const PIPELINE_ORDER: &[Step] = &[
    Step::Fmt,
    Step::Check,
    Step::Clippy,
    Step::Unit,
    Step::Integration,
    Step::Adapter,
    Step::Coverage,
    Step::FeatureMatrixCore,
    Step::FeatureMatrixI18n,
    Step::FeatureMatrixSubsystems,
    Step::FeatureMatrixLeptos,
    Step::FeatureMatrixDioxus,
];

/// Errors from CI operations.
#[derive(Debug)]
pub enum Error {
    /// A cargo subprocess exited with a non-zero status.
    StepFailed {
        /// The step that failed.
        step: Step,
        /// The command that was run (for display).
        command: String,
        /// Process exit code, if available.
        code: Option<i32>,
    },
    /// A required tool is not installed.
    MissingTool {
        /// Human-readable tool name.
        tool: String,
        /// How to install it.
        install_hint: String,
    },
    /// IO error spawning a subprocess.
    Io(io::Error),
    /// Coverage threshold check failed.
    Coverage(crate::coverage::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StepFailed {
                step,
                command,
                code,
            } => {
                write!(f, "{} failed", step_name(*step))?;
                if let Some(code) = code {
                    write!(f, " (exit code {code})")?;
                }
                write!(f, ": {command}")
            }
            Self::MissingTool { tool, install_hint } => {
                write!(
                    f,
                    "missing required tool: {tool}\n  install: {install_hint}"
                )
            }
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Coverage(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the specified CI steps, or the full pipeline if none are given.
///
/// Execution is sequential and fail-fast: the first step that fails stops the
/// pipeline and returns its error.
///
/// # Errors
///
/// Returns [`CiError`] on the first step that fails — either a subprocess
/// non-zero exit, a missing tool, or a coverage threshold violation.
pub fn run(steps: Vec<Step>) -> Result<(), Error> {
    let steps = resolve_steps(steps);

    for (i, step) in steps.iter().enumerate() {
        print_header(*step, i + 1, steps.len());
        run_step(*step)?;
        print_pass(*step);
    }

    print_summary(&steps);
    Ok(())
}

// ---------------------------------------------------------------------------
// Step resolution
// ---------------------------------------------------------------------------

/// Resolve the user-provided step list into the concrete list to execute.
///
/// - Empty input → full [`PIPELINE_ORDER`].
/// - `FeatureMatrix` → expands to the five individual groups.
/// - Everything else passes through as-is.
fn resolve_steps(steps: Vec<Step>) -> Vec<Step> {
    if steps.is_empty() {
        return PIPELINE_ORDER.to_vec();
    }

    let mut resolved = Vec::with_capacity(steps.len());
    for step in steps {
        if step == Step::FeatureMatrix {
            resolved.extend_from_slice(&[
                Step::FeatureMatrixCore,
                Step::FeatureMatrixI18n,
                Step::FeatureMatrixSubsystems,
                Step::FeatureMatrixLeptos,
                Step::FeatureMatrixDioxus,
            ]);
        } else {
            resolved.push(step);
        }
    }
    resolved
}

// ---------------------------------------------------------------------------
// Step dispatch
// ---------------------------------------------------------------------------

/// Execute a single CI step.
fn run_step(step: Step) -> Result<(), Error> {
    match step {
        Step::Fmt => run_fmt(),
        Step::Check => run_check(),
        Step::Clippy => run_clippy(),
        Step::Unit => run_unit(),
        Step::Integration => run_integration(),
        Step::Adapter => run_adapter(),
        Step::Coverage => run_coverage(),
        Step::FeatureMatrix => {
            unreachable!("FeatureMatrix is expanded by resolve_steps")
        }
        Step::FeatureMatrixCore => feature_matrix::run_group(feature_matrix::Group::Core),
        Step::FeatureMatrixI18n => feature_matrix::run_group(feature_matrix::Group::I18n),
        Step::FeatureMatrixSubsystems => {
            feature_matrix::run_group(feature_matrix::Group::Subsystems)
        }
        Step::FeatureMatrixLeptos => feature_matrix::run_group(feature_matrix::Group::Leptos),
        Step::FeatureMatrixDioxus => feature_matrix::run_group(feature_matrix::Group::Dioxus),
    }
}

// ---------------------------------------------------------------------------
// Individual step implementations
// ---------------------------------------------------------------------------

fn run_fmt() -> Result<(), Error> {
    preflight_nightly()?;
    cargo(Step::Fmt, &["+nightly", "fmt", "--all", "--check"])
}

fn run_check() -> Result<(), Error> {
    cargo(Step::Check, &["check", "--workspace", "--all-features"])
}

fn run_clippy() -> Result<(), Error> {
    cargo(
        Step::Clippy,
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )
}

fn run_unit() -> Result<(), Error> {
    cargo(
        Step::Unit,
        &[
            "test",
            "-p",
            "ars-a11y",
            "-p",
            "ars-core",
            "-p",
            "ars-collections",
            "-p",
            "ars-dom",
            "-p",
            "ars-interactions",
            "-p",
            "ars-forms",
            "--all-targets",
            "--all-features",
        ],
    )
}

fn run_integration() -> Result<(), Error> {
    cargo(
        Step::Integration,
        &["test", "-p", "ars-core", "service_applies_transitions"],
    )
}

fn run_adapter() -> Result<(), Error> {
    cargo(
        Step::Adapter,
        &[
            "test",
            "-p",
            "ars-test-harness",
            "-p",
            "ars-test-harness-leptos",
            "-p",
            "ars-test-harness-dioxus",
            "-p",
            "ars-leptos",
            "-p",
            "ars-dioxus",
            "--all-targets",
            "--all-features",
        ],
    )
}

fn run_coverage() -> Result<(), Error> {
    preflight_llvm_cov()?;

    // Generate lcov via cargo-llvm-cov.
    cargo(
        Step::Coverage,
        &[
            "llvm-cov",
            "--workspace",
            "--exclude",
            "ars-leptos",
            "--exclude",
            "ars-dioxus",
            "--exclude",
            "ars-test-harness-leptos",
            "--exclude",
            "ars-test-harness-dioxus",
            "--exclude",
            "ars-derive",
            "--exclude",
            "xtask",
            "--lcov",
            "--output-path",
            "lcov.info",
        ],
    )?;

    // Check thresholds programmatically (reuses coverage module).
    let thresholds = crate::coverage::default_thresholds();
    let lcov_path = Path::new("lcov.info");
    match crate::coverage::check_all(lcov_path, &thresholds) {
        Ok(output) => {
            eprint!("{output}");
            Ok(())
        }
        Err(e) => Err(Error::Coverage(e)),
    }
}

// ---------------------------------------------------------------------------
// Subprocess helper
// ---------------------------------------------------------------------------

/// Run `cargo <args>`, inheriting stdout/stderr.
///
/// Returns `Ok(())` on exit-code 0, or `CiError::StepFailed` otherwise.
pub(crate) fn cargo(step: Step, args: &[&str]) -> Result<(), Error> {
    let display_cmd = format!("cargo {}", args.join(" "));
    eprintln!("  > {display_cmd}");

    let status = process::Command::new("cargo")
        .args(args)
        .status()
        .map_err(Error::Io)?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::StepFailed {
            step,
            command: display_cmd,
            code: status.code(),
        })
    }
}

// ---------------------------------------------------------------------------
// Preflight checks
// ---------------------------------------------------------------------------

/// Verify the nightly toolchain is available.
fn preflight_nightly() -> Result<(), Error> {
    let output = process::Command::new("rustup")
        .args(["run", "nightly", "rustc", "--version"])
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
        .status()
        .map_err(Error::Io)?;

    if !output.success() {
        return Err(Error::MissingTool {
            tool: "nightly toolchain".into(),
            install_hint: "rustup toolchain install nightly".into(),
        });
    }
    Ok(())
}

/// Verify `cargo-llvm-cov` is installed.
fn preflight_llvm_cov() -> Result<(), Error> {
    let output = process::Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
        .status()
        .map_err(Error::Io)?;

    if !output.success() {
        return Err(Error::MissingTool {
            tool: "cargo-llvm-cov".into(),
            install_hint: "cargo install cargo-llvm-cov --locked".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Output formatting
// ---------------------------------------------------------------------------

/// Human-readable name for a step (kebab-case).
const fn step_name(step: Step) -> &'static str {
    match step {
        Step::Fmt => "fmt",
        Step::Check => "check",
        Step::Clippy => "clippy",
        Step::Unit => "unit",
        Step::Integration => "integration",
        Step::Adapter => "adapter",
        Step::Coverage => "coverage",
        Step::FeatureMatrix => "feature-matrix",
        Step::FeatureMatrixCore => "feature-matrix-core",
        Step::FeatureMatrixI18n => "feature-matrix-i18n",
        Step::FeatureMatrixSubsystems => "feature-matrix-subsystems",
        Step::FeatureMatrixLeptos => "feature-matrix-leptos",
        Step::FeatureMatrixDioxus => "feature-matrix-dioxus",
    }
}

fn print_header(step: Step, current: usize, total: usize) {
    eprint!("{}", format_header(step, current, total));
}

fn print_pass(step: Step) {
    eprint!("{}", format_pass(step));
}

fn print_summary(steps: &[Step]) {
    eprint!("{}", format_summary(steps));
}

/// Format the header banner for a step (testable).
fn format_header(step: Step, current: usize, total: usize) -> String {
    format!("\n=== [{current}/{total}] {} ===\n\n", step_name(step))
}

/// Format the pass message for a step (testable).
fn format_pass(step: Step) -> String {
    format!("\n  {} passed\n", step_name(step))
}

/// Format the final summary (testable).
fn format_summary(steps: &[Step]) -> String {
    use fmt::Write as _;
    let mut out = String::from("\n=== CI Summary ===\n");
    for step in steps {
        writeln!(out, "  {}: passed", step_name(*step)).expect("write to String");
    }
    writeln!(out, "\nAll {} steps passed.", steps.len()).expect("write to String");
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_steps_resolve_to_pipeline_order() {
        let resolved = resolve_steps(vec![]);
        assert_eq!(resolved, PIPELINE_ORDER);
    }

    #[test]
    fn explicit_steps_pass_through() {
        let input = vec![Step::Check, Step::Clippy];
        let resolved = resolve_steps(input.clone());
        assert_eq!(resolved, input);
    }

    #[test]
    fn feature_matrix_expands_to_five_groups() {
        let resolved = resolve_steps(vec![Step::FeatureMatrix]);
        assert_eq!(resolved.len(), 5);
        assert_eq!(resolved[0], Step::FeatureMatrixCore);
        assert_eq!(resolved[1], Step::FeatureMatrixI18n);
        assert_eq!(resolved[2], Step::FeatureMatrixSubsystems);
        assert_eq!(resolved[3], Step::FeatureMatrixLeptos);
        assert_eq!(resolved[4], Step::FeatureMatrixDioxus);
    }

    #[test]
    fn feature_matrix_expands_in_context() {
        let resolved = resolve_steps(vec![Step::Fmt, Step::FeatureMatrix, Step::Coverage]);
        assert_eq!(resolved.len(), 7); // 1 + 5 + 1
        assert_eq!(resolved[0], Step::Fmt);
        assert_eq!(resolved[6], Step::Coverage);
    }

    #[test]
    fn pipeline_order_does_not_contain_meta_step() {
        assert!(
            !PIPELINE_ORDER.contains(&Step::FeatureMatrix),
            "PIPELINE_ORDER must not include the FeatureMatrix meta-step"
        );
    }

    #[test]
    fn step_names_are_kebab_case() {
        for &step in PIPELINE_ORDER {
            let name = step_name(step);
            assert!(
                !name.contains('_') && !name.contains(' '),
                "step name {name:?} is not kebab-case"
            );
        }
    }

    /// Every `CiStep` variant has a non-empty name.
    #[test]
    fn step_name_covers_all_variants() {
        let all = [
            Step::Fmt,
            Step::Check,
            Step::Clippy,
            Step::Unit,
            Step::Integration,
            Step::Adapter,
            Step::Coverage,
            Step::FeatureMatrix,
            Step::FeatureMatrixCore,
            Step::FeatureMatrixI18n,
            Step::FeatureMatrixSubsystems,
            Step::FeatureMatrixLeptos,
            Step::FeatureMatrixDioxus,
        ];
        for step in all {
            assert!(!step_name(step).is_empty(), "{step:?} has empty name");
        }
    }

    // -- CiError::Display tests -----------------------------------------------

    #[test]
    fn display_step_failed_with_code() {
        let err = Error::StepFailed {
            step: Step::Clippy,
            command: "cargo clippy".into(),
            code: Some(101),
        };
        let msg = err.to_string();
        assert!(msg.contains("clippy failed"), "got: {msg}");
        assert!(msg.contains("exit code 101"), "got: {msg}");
        assert!(msg.contains("cargo clippy"), "got: {msg}");
    }

    #[test]
    fn display_step_failed_without_code() {
        let err = Error::StepFailed {
            step: Step::Fmt,
            command: "cargo +nightly fmt --all --check".into(),
            code: None,
        };
        let msg = err.to_string();
        assert!(msg.contains("fmt failed"), "got: {msg}");
        assert!(!msg.contains("exit code"), "got: {msg}");
    }

    #[test]
    fn display_missing_tool() {
        let err = Error::MissingTool {
            tool: "nightly toolchain".into(),
            install_hint: "rustup toolchain install nightly".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("nightly toolchain"), "got: {msg}");
        assert!(
            msg.contains("rustup toolchain install nightly"),
            "got: {msg}"
        );
    }

    #[test]
    fn display_io_error() {
        let err = Error::Io(io::Error::new(io::ErrorKind::NotFound, "no cargo"));
        let msg = err.to_string();
        assert!(msg.contains("IO error"), "got: {msg}");
        assert!(msg.contains("no cargo"), "got: {msg}");
    }

    #[test]
    fn display_coverage_error() {
        let err = Error::Coverage(crate::coverage::Error::NoSourceFiles {
            package: "ars-core".into(),
        });
        let msg = err.to_string();
        assert!(msg.contains("ars-core"), "got: {msg}");
    }

    // -- Output formatting tests ----------------------------------------------

    #[test]
    fn format_header_contains_step_and_progress() {
        let hdr = format_header(Step::Clippy, 3, 12);
        assert!(hdr.contains("[3/12]"), "got: {hdr}");
        assert!(hdr.contains("clippy"), "got: {hdr}");
    }

    #[test]
    fn format_pass_contains_step_name() {
        let msg = format_pass(Step::Unit);
        assert!(msg.contains("unit passed"), "got: {msg}");
    }

    #[test]
    fn format_summary_lists_all_steps() {
        let steps = vec![Step::Fmt, Step::Check];
        let summary = format_summary(&steps);
        assert!(summary.contains("fmt: passed"), "got: {summary}");
        assert!(summary.contains("check: passed"), "got: {summary}");
        assert!(summary.contains("All 2 steps passed"), "got: {summary}");
    }

    #[test]
    fn format_summary_empty() {
        let summary = format_summary(&[]);
        assert!(summary.contains("All 0 steps passed"), "got: {summary}");
    }
}
