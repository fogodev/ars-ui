//! Workspace test orchestration built on cargo-nextest.
//!
//! This module powers `cargo xtest` and the CI test stages. It runs the
//! workspace's cargo-based test surface with `cargo nextest` and reports
//! aggregate executed and passed test counts across all invoked stages.

use std::{
    ffi::OsStr,
    fmt::{self, Display},
    io::{self, Write},
    process::{self, Output},
    sync::OnceLock,
};

use regex::Regex;

use crate::{
    ci::feature_matrix::{Group, group_def},
    i18n,
};

/// Test stages supported by `cargo xtest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Workspace all-target test stage with ars-i18n backend splitting.
    Unit,

    /// Browser-executed wasm tests for ars-i18n's `web-intl` backend.
    I18nBrowser,

    /// Browser-executed wasm tests for ars-dom's web feature.
    DomBrowser,

    /// Release-profile test verification.
    Release,

    /// Integration tests with explicit filters.
    Integration,

    /// Adapter and harness tests.
    Adapter,

    /// Feature matrix tests for `ars-core`.
    FeatureMatrixCore,

    /// Feature matrix tests for `ars-i18n`.
    FeatureMatrixI18n,

    /// Feature matrix tests for subsystem crates.
    FeatureMatrixSubsystems,

    /// Feature matrix tests for `ars-leptos`.
    FeatureMatrixLeptos,

    /// Feature matrix tests for `ars-dioxus`.
    FeatureMatrixDioxus,
}

/// Aggregate test execution totals.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    /// Total number of tests executed across all stages.
    pub total_run: u64,

    /// Total number of tests that passed across all stages.
    pub total_passed: u64,
}

impl Summary {
    const fn add_assign(&mut self, other: Self) {
        self.total_run += other.total_run;
        self.total_passed += other.total_passed;
    }
}

/// Errors returned by `cargo xtest`.
#[derive(Debug)]
pub enum Error {
    /// A required external tool is not available.
    MissingTool {
        /// Human-readable tool name.
        tool: String,

        /// Suggested install command or hint.
        install_hint: String,
    },

    /// A subprocess exited unsuccessfully.
    CommandFailed {
        /// Display form of the command that failed.
        command: String,

        /// Exit code, if available.
        code: Option<i32>,
    },

    /// IO error while spawning or reading subprocess output.
    Io(io::Error),

    /// One or more test invocations failed.
    Failed {
        /// Human-readable summary of totals and failures.
        summary: String,
    },
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

            Self::CommandFailed { command, code } => {
                write!(f, "command failed")?;
                if let Some(code) = code {
                    write!(f, " (exit code {code})")?;
                }
                write!(f, ": {command}")
            }

            Self::Io(error) => write!(f, "IO error: {error}"),

            Self::Failed { summary } => write!(f, "{summary}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Default)]
struct StageResult {
    summary: Summary,
    failures: Vec<String>,
}

/// Run all test stages, or a single selected stage.
///
/// # Errors
///
/// Returns [`Error::Failed`] if any stage command fails, or another error if
/// required tools are missing or subprocesses cannot be spawned.
pub fn run(stage: Option<Stage>) -> Result<Summary, Error> {
    let stages = stage.map_or_else(
        || {
            vec![
                Stage::Unit,
                Stage::I18nBrowser,
                Stage::DomBrowser,
                Stage::Release,
                Stage::Integration,
                Stage::Adapter,
                Stage::FeatureMatrixCore,
                Stage::FeatureMatrixI18n,
                Stage::FeatureMatrixSubsystems,
                Stage::FeatureMatrixLeptos,
                Stage::FeatureMatrixDioxus,
            ]
        },
        |stage| vec![stage],
    );

    let mut totals = Summary::default();

    let mut failures = Vec::new();

    for stage in stages {
        let result = run_stage_inner(stage)?;

        totals.add_assign(result.summary);

        failures.extend(result.failures);
    }

    let summary = render_summary(&totals, &failures);

    if failures.is_empty() {
        print!("{summary}");

        Ok(totals)
    } else {
        Err(Error::Failed { summary })
    }
}

/// Run a single test stage.
///
/// # Errors
///
/// Returns [`Error::Failed`] if any command in the stage fails.
pub fn run_stage(stage: Stage) -> Result<Summary, Error> {
    let result = run_stage_inner(stage)?;

    let summary = render_summary(&result.summary, &result.failures);

    if result.failures.is_empty() {
        print!("{summary}");

        Ok(result.summary)
    } else {
        Err(Error::Failed { summary })
    }
}

pub(crate) fn run_feature_matrix_group(group: Group) -> Result<Summary, Error> {
    preflight_nextest()?;

    let result = run_feature_matrix_group_inner(group)?;

    let summary = render_summary(&result.summary, &result.failures);

    if result.failures.is_empty() {
        print!("{summary}");

        Ok(result.summary)
    } else {
        Err(Error::Failed { summary })
    }
}

fn run_stage_inner(stage: Stage) -> Result<StageResult, Error> {
    preflight_for_stage(stage)?;

    eprintln!("==> {}", stage_name(stage));

    match stage {
        Stage::Unit => run_unit_stage(),
        Stage::I18nBrowser => run_i18n_browser_stage(),
        Stage::DomBrowser => run_dom_browser_stage(),
        Stage::Release => run_release_stage(),
        Stage::Integration => run_invocations(integration_invocations()),
        Stage::Adapter => run_invocations(adapter_invocations()),
        Stage::FeatureMatrixCore => run_feature_matrix_group_inner(Group::Core),
        Stage::FeatureMatrixI18n => run_feature_matrix_group_inner(Group::I18n),
        Stage::FeatureMatrixSubsystems => run_feature_matrix_group_inner(Group::Subsystems),
        Stage::FeatureMatrixLeptos => run_feature_matrix_group_inner(Group::Leptos),
        Stage::FeatureMatrixDioxus => run_feature_matrix_group_inner(Group::Dioxus),
    }
}

fn run_i18n_browser_stage() -> Result<StageResult, Error> {
    let run = run_shell_command(
        "i18n browser",
        "wasm-pack",
        &[
            "test",
            "--headless",
            "--chrome",
            "crates/ars-i18n",
            "--no-default-features",
            "--features",
            "std,web-intl",
        ],
    )?;

    let mut result = StageResult::default();

    result
        .summary
        .add_assign(parse_wasm_pack_summary(&run.output));

    if !run.success {
        result
            .failures
            .push(format_failure("i18n browser", &run.command, run.code));
    }

    Ok(result)
}

fn run_dom_browser_stage() -> Result<StageResult, Error> {
    let run = run_shell_command(
        "dom browser",
        "wasm-pack",
        &["test", "--headless", "--chrome", "crates/ars-dom"],
    )?;

    let mut result = StageResult::default();

    result
        .summary
        .add_assign(parse_wasm_pack_summary(&run.output));

    if !run.success {
        result
            .failures
            .push(format_failure("dom browser", &run.command, run.code));
    }

    Ok(result)
}

fn run_unit_stage() -> Result<StageResult, Error> {
    let [icu4x_features, web_intl_features] = i18n::i18n_feature_lists().map_err(Error::Io)?;

    let invocations = unit_invocation_specs(&icu4x_features, &web_intl_features);

    run_owned_invocations(&invocations)
}

fn unit_invocation_specs(icu4x_features: &str, web_intl_features: &str) -> Vec<OwnedInvocation> {
    vec![
        OwnedInvocation {
            label: "unit workspace".into(),
            args: vec![
                "--workspace".into(),
                "--all-targets".into(),
                "--all-features".into(),
                "--exclude".into(),
                "ars-i18n".into(),
            ],
        },
        OwnedInvocation {
            label: "unit i18n (icu4x)".into(),
            args: vec![
                "-p".into(),
                "ars-i18n".into(),
                "--all-targets".into(),
                "--no-default-features".into(),
                "--features".into(),
                icu4x_features.into(),
            ],
        },
        OwnedInvocation {
            label: "unit i18n (web-intl)".into(),
            args: vec![
                "-p".into(),
                "ars-i18n".into(),
                "--all-targets".into(),
                "--no-default-features".into(),
                "--features".into(),
                web_intl_features.into(),
            ],
        },
    ]
}

fn run_release_stage() -> Result<StageResult, Error> {
    let [icu4x_features, web_intl_features] = i18n::i18n_feature_lists().map_err(Error::Io)?;

    let invocations = release_invocation_specs(&icu4x_features, &web_intl_features);

    run_owned_invocations(&invocations)
}

fn release_invocation_specs(icu4x_features: &str, web_intl_features: &str) -> Vec<OwnedInvocation> {
    vec![
        OwnedInvocation {
            label: "release workspace".into(),
            args: vec![
                "--workspace".into(),
                "--all-targets".into(),
                "--all-features".into(),
                "--exclude".into(),
                "ars-i18n".into(),
                "--release".into(),
            ],
        },
        OwnedInvocation {
            label: "release i18n (icu4x)".into(),
            args: vec![
                "-p".into(),
                "ars-i18n".into(),
                "--all-targets".into(),
                "--no-default-features".into(),
                "--features".into(),
                icu4x_features.into(),
                "--release".into(),
            ],
        },
        OwnedInvocation {
            label: "release i18n (web-intl)".into(),
            args: vec![
                "-p".into(),
                "ars-i18n".into(),
                "--all-targets".into(),
                "--no-default-features".into(),
                "--features".into(),
                web_intl_features.into(),
                "--release".into(),
            ],
        },
    ]
}

fn integration_invocations() -> &'static [Invocation<'static>] {
    static INTEGRATION_ARGS: [&str; 3] = ["-p", "ars-core", "service_applies_transitions"];

    static INTEGRATION: [Invocation<'static>; 1] =
        [Invocation::nextest("integration", &INTEGRATION_ARGS)];

    &INTEGRATION
}

fn adapter_invocations() -> &'static [Invocation<'static>] {
    static ADAPTER_ARGS: [&str; 12] = [
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
    ];

    static ADAPTER: [Invocation<'static>; 1] = [Invocation::nextest("adapter", &ADAPTER_ARGS)];

    &ADAPTER
}

fn run_feature_matrix_group_inner(group: Group) -> Result<StageResult, Error> {
    let mut result = StageResult::default();

    let def = group_def(group);

    for combo in def.combos {
        let label = format!("feature-test {}", combo.args.join(" "));

        let mut args = combo.args.to_vec();

        args.push("--lib");

        let run = run_nextest_command(&label, &args)?;

        result.summary.add_assign(run.summary);

        if !run.success {
            result
                .failures
                .push(format_failure(&label, &run.command, run.code));
        }
    }

    Ok(result)
}

fn run_invocations(invocations: &[Invocation<'_>]) -> Result<StageResult, Error> {
    let mut result = StageResult::default();

    for invocation in invocations {
        match invocation.kind {
            InvocationKind::Nextest => {
                let run = run_nextest_command(invocation.label, invocation.args)?;

                result.summary.add_assign(run.summary);

                if !run.success {
                    result
                        .failures
                        .push(format_failure(invocation.label, &run.command, run.code));
                }
            }
        }
    }

    Ok(result)
}

fn run_owned_invocations(invocations: &[OwnedInvocation]) -> Result<StageResult, Error> {
    let mut result = StageResult::default();

    for invocation in invocations {
        let args = invocation
            .args
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();

        let run = run_nextest_command(&invocation.label, &args)?;

        result.summary.add_assign(run.summary);

        if !run.success {
            result
                .failures
                .push(format_failure(&invocation.label, &run.command, run.code));
        }
    }

    Ok(result)
}

fn preflight_nextest() -> Result<(), Error> {
    let status = process::Command::new("cargo")
        .args(["nextest", "--version"])
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
        .status()
        .map_err(Error::Io)?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::MissingTool {
            tool: "cargo-nextest".into(),
            install_hint: "cargo install cargo-nextest --locked".into(),
        })
    }
}

fn preflight_wasm_pack() -> Result<(), Error> {
    let status = process::Command::new("wasm-pack")
        .arg("--version")
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
        .status()
        .map_err(Error::Io)?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::MissingTool {
            tool: "wasm-pack".into(),
            install_hint: "cargo install wasm-pack --locked".into(),
        })
    }
}

fn preflight_for_stage(stage: Stage) -> Result<(), Error> {
    match stage {
        Stage::I18nBrowser | Stage::DomBrowser => preflight_wasm_pack(),
        Stage::Unit
        | Stage::Release
        | Stage::Integration
        | Stage::Adapter
        | Stage::FeatureMatrixCore
        | Stage::FeatureMatrixI18n
        | Stage::FeatureMatrixSubsystems
        | Stage::FeatureMatrixLeptos
        | Stage::FeatureMatrixDioxus => preflight_nextest(),
    }
}

fn nextest_summary_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();

    REGEX.get_or_init(|| {
        Regex::new(r"Summary \[[^\]]+\]\s+(?P<run>\d+) tests? run:\s+(?P<passed>\d+) passed")
            .expect("valid nextest summary regex")
    })
}

fn ansi_escape_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();

    REGEX.get_or_init(|| Regex::new(r"\x1B\[[0-?]*[ -/]*[@-~]").expect("valid ansi escape regex"))
}

fn parse_nextest_summary(output: &str) -> Summary {
    let normalized = ansi_escape_regex().replace_all(output, "");

    nextest_summary_regex()
        .captures_iter(&normalized)
        .last()
        .and_then(|captures| {
            let total_run = captures.name("run")?.as_str().parse::<u64>().ok()?;

            let total_passed = captures.name("passed")?.as_str().parse::<u64>().ok()?;

            Some(Summary {
                total_run,
                total_passed,
            })
        })
        .unwrap_or_default()
}

fn wasm_pack_summary_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();

    REGEX.get_or_init(|| {
        Regex::new(
            r"test result: .*? (?P<passed>\d+) passed; (?P<failed>\d+) failed; (?P<ignored>\d+) ignored; (?P<other>\d+) (?:measured|filtered out)",
        )
        .expect("valid wasm-pack summary regex")
    })
}

fn parse_wasm_pack_summary(output: &str) -> Summary {
    let normalized = ansi_escape_regex().replace_all(output, "");

    let mut summary = Summary::default();

    for captures in wasm_pack_summary_regex().captures_iter(&normalized) {
        let passed = captures
            .name("passed")
            .and_then(|value| value.as_str().parse::<u64>().ok());

        let failed = captures
            .name("failed")
            .and_then(|value| value.as_str().parse::<u64>().ok());

        let ignored = captures
            .name("ignored")
            .and_then(|value| value.as_str().parse::<u64>().ok());

        let other = captures
            .name("other")
            .and_then(|value| value.as_str().parse::<u64>().ok());

        if let (Some(passed), Some(failed), Some(ignored), Some(other)) =
            (passed, failed, ignored, other)
        {
            summary.total_run += passed + failed + ignored + other;
            summary.total_passed += passed;
        }
    }

    summary
}

fn run_nextest_command(label: &str, args: &[&str]) -> Result<CommandResult, Error> {
    eprintln!("  [nextest] {label}");

    let mut command = process::Command::new("cargo");

    command
        .args(["nextest", "run", "--no-fail-fast"])
        .args(args);

    configure_color_output(&mut command);

    let display = format!("{command:?}");

    let output = command.output().map_err(Error::Io)?;

    print_output(&output)?;

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(CommandResult {
        command: display,
        code: output.status.code(),
        success: output.status.success(),
        summary: parse_nextest_summary(&combined),
        output: combined,
    })
}

fn run_shell_command(label: &str, program: &str, args: &[&str]) -> Result<CommandResult, Error> {
    eprintln!("  [{program}] {label}");

    let mut command = process::Command::new(program);

    command.args(args);

    let display = format!("{command:?}");

    let output = command.output().map_err(Error::Io)?;

    print_output(&output)?;

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(CommandResult {
        command: display,
        code: output.status.code(),
        success: output.status.success(),
        summary: if program == "wasm-pack" {
            parse_wasm_pack_summary(&combined)
        } else {
            Summary::default()
        },
        output: combined,
    })
}

fn configure_color_output(command: &mut process::Command) {
    if should_force_color_output(
        std::env::var_os("NO_COLOR").as_deref(),
        std::env::var_os("CARGO_TERM_COLOR").as_deref(),
    ) {
        command.env("CARGO_TERM_COLOR", "always");
    }
}

const fn should_force_color_output(
    no_color: Option<&OsStr>,
    cargo_term_color: Option<&OsStr>,
) -> bool {
    no_color.is_none() && cargo_term_color.is_none()
}

fn print_output(output: &Output) -> Result<(), Error> {
    io::stdout().write_all(&output.stdout).map_err(Error::Io)?;

    io::stderr().write_all(&output.stderr).map_err(Error::Io)?;

    Ok(())
}

fn format_failure(label: &str, command: &str, code: Option<i32>) -> String {
    let mut message = format!("{label}: {command}");

    if let Some(code) = code {
        message.push_str(&format!(" (exit code {code})"));
    }

    message
}

fn render_summary(summary: &Summary, failures: &[String]) -> String {
    let mut output = String::new();

    output.push_str(&format!("Total tests run: {}\n", summary.total_run));
    output.push_str(&format!("Total tests passed: {}\n", summary.total_passed));

    if failures.is_empty() {
        output.push_str("All test commands passed.\n");
    } else {
        output.push_str(&format!("Failed commands: {}\n", failures.len()));

        for failure in failures {
            output.push_str(&format!(" - {failure}\n"));
        }
    }

    output
}

const fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Unit => "unit",
        Stage::I18nBrowser => "i18n-browser",
        Stage::DomBrowser => "dom-browser",
        Stage::Release => "release",
        Stage::Integration => "integration",
        Stage::Adapter => "adapter",
        Stage::FeatureMatrixCore => "feature-matrix-core",
        Stage::FeatureMatrixI18n => "feature-matrix-i18n",
        Stage::FeatureMatrixSubsystems => "feature-matrix-subsystems",
        Stage::FeatureMatrixLeptos => "feature-matrix-leptos",
        Stage::FeatureMatrixDioxus => "feature-matrix-dioxus",
    }
}

#[derive(Debug, Clone, Copy)]
struct Invocation<'a> {
    label: &'a str,
    args: &'a [&'a str],
    kind: InvocationKind,
}

#[derive(Debug, Clone)]
struct OwnedInvocation {
    label: String,
    args: Vec<String>,
}

impl<'a> Invocation<'a> {
    const fn nextest(label: &'a str, args: &'a [&'a str]) -> Self {
        Self {
            label,
            args,
            kind: InvocationKind::Nextest,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum InvocationKind {
    Nextest,
}

#[derive(Debug, Default)]
struct CommandResult {
    command: String,
    code: Option<i32>,
    success: bool,
    output: String,
    summary: Summary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_nextest_summary_extracts_run_and_passed_counts() {
        let output = "\
────────────
 Summary [   0.058s] 54 tests run: 53 passed, 1 failed, 0 skipped
";

        let summary = parse_nextest_summary(output);

        assert_eq!(
            summary,
            Summary {
                total_run: 54,
                total_passed: 53,
            }
        );
    }

    #[test]
    fn parse_nextest_summary_defaults_to_zero_without_summary_line() {
        assert_eq!(parse_nextest_summary("no summary here"), Summary::default());
    }

    #[test]
    fn parse_nextest_summary_accepts_singular_test_wording() {
        let output = "Summary [   0.010s] 1 test run: 1 passed, 0 failed, 0 skipped";

        assert_eq!(
            parse_nextest_summary(output),
            Summary {
                total_run: 1,
                total_passed: 1,
            }
        );
    }

    #[test]
    fn parse_nextest_summary_ignores_ansi_color_codes() {
        let output = "\
\u{1b}[1m\u{1b}[38;5;230m     Summary\u{1b}[0m [   4.010s] \u{1b}[1m875\u{1b}[0m tests run: \u{1b}[1m\u{1b}[38;5;154m875 passed\u{1b}[0m, \u{1b}[1m\u{1b}[38;5;214m0 skipped\u{1b}[0m
";

        assert_eq!(
            parse_nextest_summary(output),
            Summary {
                total_run: 875,
                total_passed: 875,
            }
        );
    }

    #[test]
    fn render_summary_lists_failures() {
        let summary = Summary {
            total_run: 12,
            total_passed: 10,
        };

        let rendered = render_summary(&summary, &["unit: cargo nextest run".into()]);

        assert!(rendered.contains("Total tests run: 12"));
        assert!(rendered.contains("Total tests passed: 10"));
        assert!(rendered.contains("Failed commands: 1"));
    }

    #[test]
    fn parse_wasm_pack_summary_extracts_run_and_passed_counts() {
        let output = "\
test result: ok. 11 passed; 0 failed; 0 ignored; 0 filtered out;
test result: ok. 0 passed; 0 failed; 0 ignored; 0 filtered out;
";

        assert_eq!(
            parse_wasm_pack_summary(output),
            Summary {
                total_run: 11,
                total_passed: 11,
            }
        );
    }

    #[test]
    fn unit_stage_splits_workspace_and_i18n_backends() {
        let invocations = unit_invocation_specs("std,icu4x", "std,web-intl");

        assert_eq!(invocations.len(), 3);
        assert_eq!(invocations[0].label, "unit workspace");
        assert_eq!(
            invocations[0].args,
            [
                "--workspace",
                "--all-targets",
                "--all-features",
                "--exclude",
                "ars-i18n"
            ]
        );
        assert_eq!(invocations[1].label, "unit i18n (icu4x)");
        assert_eq!(
            invocations[1].args,
            [
                "-p",
                "ars-i18n",
                "--all-targets",
                "--no-default-features",
                "--features",
                "std,icu4x",
            ]
        );
        assert_eq!(invocations[2].label, "unit i18n (web-intl)");
    }

    #[test]
    fn stage_name_reports_i18n_browser() {
        assert_eq!(stage_name(Stage::I18nBrowser), "i18n-browser");
    }

    #[test]
    fn stage_name_reports_dom_browser() {
        assert_eq!(stage_name(Stage::DomBrowser), "dom-browser");
    }

    #[test]
    fn release_stage_splits_workspace_and_i18n_backends() {
        let invocations = release_invocation_specs("std,icu4x", "std,web-intl");

        assert_eq!(invocations.len(), 3);
        assert_eq!(invocations[0].label, "release workspace");
        assert_eq!(
            invocations[0].args,
            [
                "--workspace",
                "--all-targets",
                "--all-features",
                "--exclude",
                "ars-i18n",
                "--release",
            ]
        );
        assert_eq!(invocations[1].label, "release i18n (icu4x)");
        assert_eq!(
            invocations[1].args,
            [
                "-p",
                "ars-i18n",
                "--all-targets",
                "--no-default-features",
                "--features",
                "std,icu4x",
                "--release",
            ]
        );
        assert_eq!(invocations[2].label, "release i18n (web-intl)");
    }

    #[test]
    fn should_force_color_output_defaults_to_true_when_unset() {
        assert!(should_force_color_output(None, None));
    }

    #[test]
    fn should_force_color_output_respects_existing_color_choice() {
        assert!(!should_force_color_output(None, Some("never".as_ref())));
    }

    #[test]
    fn should_force_color_output_respects_no_color() {
        assert!(!should_force_color_output(Some("1".as_ref()), None));
    }
}
