//! Local CI pipeline runner.
//!
//! Mirrors the jobs declared in `.github/workflows/ci.yml` so that the xtask
//! crate is the single source of truth for CI commands. Run the full pipeline
//! with `cargo xci` or pick individual steps with `cargo xci check clippy`.

pub(crate) mod feature_matrix;

use std::{
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
    process,
};

use crate::{coverage, i18n, lint, manifest, spec, test};

const MUTUAL_EXCLUSION_GUARD: &str = "features `icu4x` and `web-intl` are mutually exclusive";

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

    /// Browser-executed wasm tests for ars-i18n's web-intl backend.
    I18nBrowser,

    /// Browser-executed wasm tests for ars-dom's web feature.
    DomBrowser,

    /// Release-profile compile and test smoke checks.
    Release,

    /// Integration tests.
    Integration,

    /// Adapter harness tests (Leptos + Dioxus).
    Adapter,

    /// Compare per-component test counts between adapters.
    AdapterParity,

    /// Generate coverage and check per-crate thresholds.
    Coverage,

    /// Enforce per-component snapshot-count policy.
    SnapshotCount,

    /// Parse every fenced Rust code block in the spec corpus and report
    /// any `syn::parse_file` syntax errors. Catches structural Rust bugs
    /// in spec snippets before they ship.
    SpecCompileSnippets,

    /// Verify every `ComponentError` variant appears in tests.
    ErrorVariantCoverage,

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

    /// Verify the mutual-exclusion compile guard for ars-i18n backend features.
    MutualExclusion,
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
    Step::I18nBrowser,
    Step::DomBrowser,
    Step::Release,
    Step::Integration,
    Step::Adapter,
    Step::AdapterParity,
    Step::Coverage,
    Step::SnapshotCount,
    Step::SpecCompileSnippets,
    Step::ErrorVariantCoverage,
    Step::FeatureMatrixCore,
    Step::FeatureMatrixI18n,
    Step::FeatureMatrixSubsystems,
    Step::FeatureMatrixLeptos,
    Step::FeatureMatrixDioxus,
    Step::MutualExclusion,
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
    Coverage(coverage::Error),

    /// Repository lint check failed.
    Lint(lint::Error),

    /// Spec corpus check failed (e.g. `compile-snippets` reported a Rust
    /// syntax error in a fenced code block).
    Spec(manifest::Error),
}

impl Display for Error {
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

            Self::Lint(e) => write!(f, "{e}"),

            Self::Spec(e) => write!(f, "{e}"),
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
pub fn run(steps: Vec<Step>, message_format: Option<&str>) -> Result<(), Error> {
    let steps = resolve_steps(steps);

    for (i, step) in steps.iter().enumerate() {
        print_header(*step, i + 1, steps.len());
        run_step(*step, message_format)?;
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
fn run_step(step: Step, message_format: Option<&str>) -> Result<(), Error> {
    match step {
        Step::Fmt => run_fmt(),

        Step::Check => run_check(message_format),

        Step::Clippy => run_clippy(message_format),

        Step::Unit => run_unit(),

        Step::I18nBrowser => run_i18n_browser(),

        Step::DomBrowser => run_dom_browser(),

        Step::Release => run_release(),

        Step::Integration => run_integration(),

        Step::Adapter => run_adapter(),

        Step::AdapterParity => run_adapter_parity(),

        Step::Coverage => run_coverage(),

        Step::SnapshotCount => run_snapshot_count(),

        Step::SpecCompileSnippets => run_spec_compile_snippets(),

        Step::ErrorVariantCoverage => run_error_variant_coverage(),

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

        Step::MutualExclusion => run_mutual_exclusion(),
    }
}

// ---------------------------------------------------------------------------
// Individual step implementations
// ---------------------------------------------------------------------------

fn run_fmt() -> Result<(), Error> {
    preflight_nightly()?;

    cargo(Step::Fmt, &["+nightly", "fmt", "--all", "--check"])
}

fn run_check(message_format: Option<&str>) -> Result<(), Error> {
    // Exclude ars-i18n: its `icu4x` and `web-intl` features are mutually
    // exclusive, so `--all-features` would trigger a compile_error!.
    // Instead, check ars-i18n twice — once per backend — to cover all features.
    cargo_with_format(
        Step::Check,
        &[
            "check",
            "--workspace",
            "--all-features",
            "--exclude",
            "ars-i18n",
        ],
        message_format,
    )?;

    let [icu4x_features, web_intl_features] = i18n::i18n_feature_lists().map_err(Error::Io)?;

    for features in [&icu4x_features, &web_intl_features] {
        cargo_with_format(
            Step::Check,
            &[
                "check",
                "-p",
                "ars-i18n",
                "--all-targets",
                "--no-default-features",
                "--features",
                features,
            ],
            message_format,
        )?;
    }

    Ok(())
}

fn run_clippy(message_format: Option<&str>) -> Result<(), Error> {
    clippy_workspace(message_format, true)
}

/// Run workspace-wide clippy with ars-i18n backend splitting.
///
/// Used by both the CI `clippy` step (`deny_warnings = true`) and
/// `cargo xclippy` for development (`deny_warnings = false`).
///
/// # Errors
///
/// Returns [`Error::StepFailed`] if any clippy invocation exits non-zero,
/// or [`Error::Io`] if `crates/ars-i18n/Cargo.toml` cannot be read.
pub fn clippy_workspace(message_format: Option<&str>, deny_warnings: bool) -> Result<(), Error> {
    // Exclude ars-i18n: its `icu4x` and `web-intl` features are mutually
    // exclusive, so `--all-features` would trigger a compile_error!.
    // Instead, lint ars-i18n twice — once per backend — to cover all features.
    let mut workspace_args = vec![
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--exclude",
        "ars-i18n",
    ];

    if deny_warnings {
        workspace_args.extend_from_slice(&["--", "-D", "warnings"]);
    }

    cargo_with_format(Step::Clippy, &workspace_args, message_format)?;

    let [icu4x_features, web_intl_features] = i18n::i18n_feature_lists().map_err(Error::Io)?;

    for features in [&icu4x_features, &web_intl_features] {
        let mut args = vec![
            "clippy",
            "-p",
            "ars-i18n",
            "--all-targets",
            "--no-default-features",
            "--features",
            features.as_str(),
        ];

        if deny_warnings {
            args.extend_from_slice(&["--", "-D", "warnings"]);
        }

        cargo_with_format(Step::Clippy, &args, message_format)?;
    }

    Ok(())
}

fn run_unit() -> Result<(), Error> {
    run_test_stage(Step::Unit, test::Stage::Unit)
}

fn run_i18n_browser() -> Result<(), Error> {
    run_test_stage(Step::I18nBrowser, test::Stage::I18nBrowser)
}

fn run_dom_browser() -> Result<(), Error> {
    run_test_stage(Step::DomBrowser, test::Stage::DomBrowser)
}

fn run_release() -> Result<(), Error> {
    cargo(
        Step::Release,
        &[
            "check",
            "--workspace",
            "--all-features",
            "--release",
            "--exclude",
            "ars-i18n",
        ],
    )?;

    let [icu4x_features, web_intl_features] = i18n::i18n_feature_lists().map_err(Error::Io)?;

    for features in [&icu4x_features, &web_intl_features] {
        cargo(
            Step::Release,
            &[
                "check",
                "-p",
                "ars-i18n",
                "--release",
                "--all-targets",
                "--no-default-features",
                "--features",
                features.as_str(),
            ],
        )?;
    }

    run_test_stage(Step::Release, test::Stage::Release)
}

fn run_integration() -> Result<(), Error> {
    run_test_stage(Step::Integration, test::Stage::Integration)
}

fn run_adapter() -> Result<(), Error> {
    run_test_stage(Step::Adapter, test::Stage::Adapter)
}

fn run_adapter_parity() -> Result<(), Error> {
    match lint::check_adapter_parity(&lint::AdapterParityOptions {
        leptos_test_dir: PathBuf::from("crates/ars-leptos/tests"),
        dioxus_test_dir: PathBuf::from("crates/ars-dioxus/tests"),
        tolerance: 2,
    }) {
        Ok(output) => {
            eprint!("{output}");
            Ok(())
        }

        Err(error) => Err(Error::Lint(error)),
    }
}

fn run_snapshot_count() -> Result<(), Error> {
    match lint::check_snapshot_count(&lint::SnapshotCountOptions {
        snapshots_dir: PathBuf::from("crates"),
        min_per_variant: 3,
        max_per_component: 20,
    }) {
        Ok(output) => {
            eprint!("{output}");
            Ok(())
        }

        Err(error) => Err(Error::Lint(error)),
    }
}

fn run_spec_compile_snippets() -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let root = manifest::SpecRoot::discover(&cwd).map_err(Error::Spec)?;

    // `fix = false` — CI must never silently rewrite spec files. Maintainers
    // run `cargo xtask spec compile-snippets --fix` locally to apply fixes.
    let report = spec::compile_snippets::execute(&root, false).map_err(Error::Spec)?;

    if report.contains("finding(s) across") {
        eprint!("{report}");

        return Err(Error::StepFailed {
            step: Step::SpecCompileSnippets,
            command: "cargo xtask spec compile-snippets".to_string(),
            code: Some(1),
        });
    }

    eprint!("{report}");

    Ok(())
}

fn run_error_variant_coverage() -> Result<(), Error> {
    match lint::check_error_variant_coverage(&lint::ErrorVariantCoverageOptions {
        source_glob: "crates/ars-core/src/**/*.rs".to_owned(),
        test_glob: "crates/ars-core/tests/**/*.rs".to_owned(),
        enum_name: "ComponentError".to_owned(),
    }) {
        Ok(output) => {
            eprint!("{output}");
            Ok(())
        }

        Err(error) => Err(Error::Lint(error)),
    }
}

fn run_mutual_exclusion() -> Result<(), Error> {
    let args = [
        "check",
        "-p",
        "ars-i18n",
        "--no-default-features",
        "--features",
        "icu4x,web-intl",
    ];

    let display_cmd = format!("cargo {}", args.join(" "));

    eprintln!("  > {display_cmd}");

    let output = process::Command::new("cargo")
        .args(args)
        .output()
        .map_err(Error::Io)?;

    if output.status.success() {
        Err(Error::StepFailed {
            step: Step::MutualExclusion,
            command: format!("{display_cmd} (unexpected success)"),
            code: output.status.code(),
        })
    } else if is_expected_mutual_exclusion_failure(&String::from_utf8_lossy(&output.stderr)) {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);

        Err(Error::StepFailed {
            step: Step::MutualExclusion,
            command: format!(
                "{display_cmd} (unexpected failure; missing guard message)\nstderr:\n{}",
                stderr.trim()
            ),
            code: output.status.code(),
        })
    }
}

fn is_expected_mutual_exclusion_failure(stderr: &str) -> bool {
    stderr.contains(MUTUAL_EXCLUSION_GUARD)
}

fn run_coverage() -> Result<(), Error> {
    preflight_llvm_cov()?;

    preflight_nextest()?;

    coverage::preflight_nightly().map_err(Error::Coverage)?;

    let coverage_dir = Path::new("target").join("coverage");

    fs::create_dir_all(&coverage_dir).map_err(Error::Io)?;

    let native_lcov = coverage_dir.join("native.lcov");

    let merged_lcov = PathBuf::from("lcov.info");

    // Generate native lcov via cargo-llvm-cov + cargo-nextest.
    cargo(
        Step::Coverage,
        &[
            "+nightly",
            "llvm-cov",
            "nextest",
            "--branch",
            "--workspace",
            "--lcov",
            "--output-path",
            native_lcov.to_str().expect("valid utf-8 path"),
            "--no-fail-fast",
        ],
    )?;

    let mut reports = vec![native_lcov];

    for target in coverage::default_wasm_coverage_targets() {
        let wasm_lcov = coverage_dir.join(format!("{}-wasm.lcov", target.package));

        coverage::generate_wasm_lcov(&coverage::WasmCoverageOptions {
            package: target.package.to_owned(),
            output: wasm_lcov.clone(),
            features: target
                .features
                .iter()
                .map(|feature| (*feature).to_owned())
                .collect(),
            no_default_features: target.no_default_features,
            extra_test_args: Vec::new(),
        })
        .map_err(Error::Coverage)?;

        reports.push(wasm_lcov);
    }

    coverage::merge_files(&reports, &merged_lcov).map_err(Error::Coverage)?;

    // Check thresholds programmatically (reuses coverage module).
    let thresholds = coverage::default_thresholds();

    match coverage::check_all(&merged_lcov, &thresholds) {
        Ok(output) => {
            eprint!("{output}");
            Ok(())
        }
        Err(e) => Err(Error::Coverage(e)),
    }
}

fn run_test_stage(step: Step, stage: test::Stage) -> Result<(), Error> {
    test::run_stage(stage)
        .map(|_| ())
        .map_err(|error| map_test_error(step, error))
}

pub(crate) fn map_test_error(step: Step, error: test::Error) -> Error {
    match error {
        test::Error::MissingTool { tool, install_hint } => {
            Error::MissingTool { tool, install_hint }
        }

        test::Error::CommandFailed { command, code } => Error::StepFailed {
            step,
            command,
            code,
        },

        test::Error::Io(error) => Error::Io(error),

        test::Error::Failed { summary } => Error::Io(io::Error::other(summary)),
    }
}

// ---------------------------------------------------------------------------
// Subprocess helper
// ---------------------------------------------------------------------------

/// Run `cargo <args>` with an optional `--message-format` flag injected.
///
/// The flag is inserted before `--` if present, otherwise appended. This
/// lets rust-analyzer's `overrideCommand` request JSON diagnostics via
/// `cargo xtask ci clippy --message-format=json`.
fn cargo_with_format(step: Step, args: &[&str], message_format: Option<&str>) -> Result<(), Error> {
    if let Some(fmt) = message_format {
        let fmt_flag = format!("--message-format={fmt}");

        let mut full_args = Vec::with_capacity(args.len() + 1);

        let mut inserted = false;

        for &arg in args {
            if arg == "--" && !inserted {
                full_args.push(fmt_flag.as_str());
                inserted = true;
            }
            full_args.push(arg);
        }

        if !inserted {
            full_args.push(fmt_flag.as_str());
        }

        cargo(step, &full_args)
    } else {
        cargo(step, args)
    }
}

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

/// Verify `cargo-nextest` is installed.
fn preflight_nextest() -> Result<(), Error> {
    let output = process::Command::new("cargo")
        .args(["nextest", "--version"])
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
        .status()
        .map_err(Error::Io)?;

    if !output.success() {
        return Err(Error::MissingTool {
            tool: "cargo-nextest".into(),
            install_hint: "cargo install cargo-nextest --locked".into(),
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
        Step::I18nBrowser => "i18n-browser",
        Step::DomBrowser => "dom-browser",
        Step::Release => "release",
        Step::Integration => "integration",
        Step::Adapter => "adapter",
        Step::AdapterParity => "adapter-parity",
        Step::Coverage => "coverage",
        Step::SnapshotCount => "snapshot-count",
        Step::SpecCompileSnippets => "spec-compile-snippets",
        Step::ErrorVariantCoverage => "error-variant-coverage",
        Step::FeatureMatrix => "feature-matrix",
        Step::FeatureMatrixCore => "feature-matrix-core",
        Step::FeatureMatrixI18n => "feature-matrix-i18n",
        Step::FeatureMatrixSubsystems => "feature-matrix-subsystems",
        Step::FeatureMatrixLeptos => "feature-matrix-leptos",
        Step::FeatureMatrixDioxus => "feature-matrix-dioxus",
        Step::MutualExclusion => "mutual-exclusion",
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
            Step::I18nBrowser,
            Step::DomBrowser,
            Step::Release,
            Step::Integration,
            Step::Adapter,
            Step::AdapterParity,
            Step::Coverage,
            Step::SnapshotCount,
            Step::ErrorVariantCoverage,
            Step::FeatureMatrix,
            Step::FeatureMatrixCore,
            Step::FeatureMatrixI18n,
            Step::FeatureMatrixSubsystems,
            Step::FeatureMatrixLeptos,
            Step::FeatureMatrixDioxus,
            Step::MutualExclusion,
        ];

        for step in all {
            assert!(!step_name(step).is_empty(), "{step:?} has empty name");
        }
    }

    #[test]
    fn mutual_exclusion_guard_matches_expected_compile_error() {
        let stderr = format!("error: {MUTUAL_EXCLUSION_GUARD}");

        assert!(is_expected_mutual_exclusion_failure(&stderr));
    }

    #[test]
    fn mutual_exclusion_guard_rejects_unrelated_compile_failures() {
        assert!(!is_expected_mutual_exclusion_failure(
            "error[E0432]: unresolved import `missing`"
        ));
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
        let err = Error::Coverage(coverage::Error::NoSourceFiles {
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
