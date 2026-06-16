//! ars-ui workspace task runner.

use std::{env, path::PathBuf, process, sync};

use clap::{Parser, Subcommand};
use xtask::{adapter, ci, coverage, crap, e2e, examples, lint, manifest, mcp, mutants, spec, test};

/// ars-ui workspace task runner.
#[derive(Parser)]
#[command(name = "xtask", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::Cli;

    #[test]
    fn issue_deps_rejects_dry_run_with_apply() {
        let result = Cli::try_parse_from([
            "xtask",
            "spec",
            "issue-deps",
            "--adapter",
            "leptos",
            "--dry-run",
            "--apply",
        ]);

        assert!(result.is_err());
    }
}

#[derive(Subcommand)]
enum Command {
    /// Run CI pipeline steps locally (alias: `cargo xci`).
    ///
    /// `--profile full` (default) mirrors GitHub Actions exactly — all 20 gates,
    /// ~25 min locally. `--profile fast` runs the curated pre-push subset: fmt,
    /// clippy, unit, integration, adapter, spec-compile-snippets,
    /// error-variant-coverage, mutual-exclusion, snapshot-count, and a
    /// native-only coverage check. Positional `steps` override the profile and
    /// run as-is, so `cargo xci fmt clippy` still works.
    Ci {
        /// Steps to run (default: every step in the selected profile).
        ///
        /// Use `feature-matrix` to run all five feature-flag groups at once.
        /// When supplied, this list overrides `--profile`.
        #[arg(value_enum)]
        steps: Vec<ci::Step>,

        /// Pipeline profile when no positional steps are given. Defaults to
        /// `full` for backward compatibility; use `fast` (alias:
        /// `cargo xci-fast`) for the routine pre-push gate.
        #[arg(long, value_enum, default_value_t = ci::Profile::Full)]
        profile: ci::Profile,

        /// Forward `--message-format` to cargo check/clippy (e.g. `json` for
        /// rust-analyzer's `overrideCommand`).
        #[arg(long)]
        message_format: Option<String>,
    },

    /// Format the workspace in place (alias: `cargo xfmt`).
    Fmt {
        /// Format one Rust source buffer from stdin to stdout.
        ///
        /// This mode is rustfmt-compatible for editor integrations.
        #[arg(long)]
        stdin: bool,
    },

    /// Run workspace clippy without `-D warnings` (alias: `cargo xclippy`).
    ///
    /// Same ars-i18n backend splitting as `cargo xci clippy`, but warnings
    /// stay as warnings — suitable for rust-analyzer and interactive use.
    Clippy {
        /// Forward `--message-format` to cargo clippy (e.g. `json` for
        /// rust-analyzer's `overrideCommand`).
        #[arg(long)]
        message_format: Option<String>,
    },

    /// Start MCP stdio server exposing all workspace tools.
    #[cfg(feature = "mcp")]
    Mcp,

    /// Spec navigation commands.
    Spec {
        #[command(subcommand)]
        cmd: SpecCommand,
    },

    /// Adapter component development helpers.
    Adapter {
        #[command(subcommand)]
        cmd: AdapterCommand,
    },

    /// Code coverage threshold enforcement.
    Coverage {
        #[command(subcommand)]
        cmd: CoverageCommand,
    },

    /// Run browser examples.
    Examples {
        #[command(subcommand)]
        cmd: ExamplesCommand,
    },

    /// Run browser E2E harnesses.
    E2e {
        #[command(subcommand)]
        cmd: E2eCommand,
    },

    /// Repository testing policy lints.
    Lint {
        #[command(subcommand)]
        cmd: LintCommand,
    },

    /// Run workspace tests through cargo-nextest.
    Test {
        #[command(subcommand)]
        cmd: Option<TestCommand>,
    },

    /// Run mutation testing via cargo-mutants (alias: `cargo xmutants`).
    ///
    /// Applies the same per-crate `--features` profile as the Nightly CI
    /// mutation matrix so local runs match CI (for example
    /// `ars-components/i18n` for snapshot/i18n-gated code paths).
    Mutants {
        /// Crate package name (for example `ars-components`).
        #[arg(short = 'p', long = "package")]
        package: String,

        /// Limit mutations to a single source file.
        #[arg(short = 'f', long = "file")]
        file: Option<PathBuf>,

        /// List mutants without running tests.
        #[arg(long)]
        list: bool,

        /// Shard selector forwarded to cargo-mutants (`k/n`, zero-based `k`).
        #[arg(long)]
        shard: Option<String>,

        /// Output directory for mutation reports.
        #[arg(long, default_value = "mutants.out")]
        output: PathBuf,

        /// Override the workspace default `--features` string for this package.
        #[arg(long)]
        features: Option<String>,
    },

    /// Run the cargo-crap CRAP report locally (alias: `cargo xcrap`).
    ///
    /// Reads an existing lcov report (generate one with `cargo xci coverage`)
    /// and prints functions whose complexity-vs-coverage CRAP score exceeds the
    /// threshold. Unlike the `cargo xci crap` CI gate, this never fails the
    /// process — it is a readable local report.
    ///
    /// Pass `--update-baseline` to (re)generate the committed regression
    /// baseline from the lcov instead, e.g. after a deliberate complexity
    /// increase or to refresh it from CI's merged `lcov.info`.
    Crap {
        /// Path to the lcov coverage report.
        #[arg(long, default_value = "lcov.info")]
        lcov: PathBuf,

        /// CRAP score threshold (defaults to the gate's pinned threshold).
        #[arg(long)]
        threshold: Option<u32>,

        /// Regenerate the committed baseline instead of printing a report.
        #[arg(long)]
        update_baseline: bool,
    },

    /// Run the native-only coverage gate locally (alias: `cargo xcov`).
    ///
    /// Instruments and checks only the crates whose CI coverage is native-only
    /// (the agnostic library crates + xtask) — fast, and needs no wasm/clang
    /// toolchain. This is the same check the `fast` pre-push profile runs, so it
    /// catches agnostic-crate threshold regressions before push. Pass `--full`
    /// to run the complete native+wasm coverage gate instead (mirrors CI;
    /// requires the wasm toolchain + clang-22).
    Cov {
        /// Run the full native+wasm coverage gate (mirrors CI) instead of the
        /// fast native-only check.
        #[arg(long)]
        full: bool,
    },
}

#[derive(Subcommand)]
enum CoverageCommand {
    /// Generate experimental wasm lcov for a single package.
    Wasm {
        /// Crate name (e.g., "ars-dom").
        #[arg(long)]
        package: String,

        /// Path to write the generated lcov file.
        #[arg(long)]
        file: PathBuf,

        /// Cargo feature to enable. May be passed multiple times.
        #[arg(long = "feature")]
        features: Vec<String>,

        /// Disable default features for the package.
        #[arg(long)]
        no_default_features: bool,

        /// Extra arguments forwarded to `cargo test` after `--`.
        #[arg(last = true)]
        extra_test_args: Vec<String>,
    },

    /// Merge multiple lcov files without double-counting duplicate lines.
    Merge {
        /// Output lcov path.
        #[arg(long)]
        output: PathBuf,

        /// Input lcov files to merge.
        #[arg(long = "file", required = true)]
        files: Vec<PathBuf>,
    },

    /// Check a single crate's coverage against thresholds.
    Check {
        /// Path to lcov.info file.
        #[arg(long)]
        file: PathBuf,

        /// Crate name (e.g., "ars-core").
        #[arg(long)]
        package: String,

        /// Minimum line coverage percentage (0–100).
        #[arg(long)]
        min: f64,

        /// Minimum branch coverage percentage (0–100).
        #[arg(long)]
        branch_min: f64,
    },

    /// Check all crates against spec-defined thresholds.
    CheckAll {
        /// Path to lcov.info file.
        #[arg(long)]
        file: PathBuf,
    },
}

#[derive(Subcommand)]
enum AdapterCommand {
    /// Scaffold adapter component files and required test placeholders.
    Scaffold {
        /// Component name in kebab or snake case.
        component: String,

        /// Component category, such as "input" or "utility".
        #[arg(long)]
        category: String,

        /// Include only the Leptos adapter.
        #[arg(long, conflicts_with = "dioxus_only")]
        leptos_only: bool,

        /// Include only the Dioxus adapter.
        #[arg(long, conflicts_with = "leptos_only")]
        dioxus_only: bool,

        /// Include Form/Fieldset composition test placeholders.
        #[arg(long)]
        form_control: bool,
    },
}

#[derive(Subcommand)]
enum LintCommand {
    /// Compare per-component adapter test counts.
    AdapterParity {
        /// Directory containing Leptos adapter tests.
        #[arg(long, default_value = "crates/ars-leptos/tests")]
        leptos_test_dir: PathBuf,

        /// Directory containing Dioxus adapter tests.
        #[arg(long, default_value = "crates/ars-dioxus/tests")]
        dioxus_test_dir: PathBuf,

        /// Directory containing Leptos adapter source modules.
        #[arg(long, default_value = "crates/ars-leptos/src")]
        leptos_src_dir: PathBuf,

        /// Directory containing Dioxus adapter source modules.
        #[arg(long, default_value = "crates/ars-dioxus/src")]
        dioxus_src_dir: PathBuf,

        /// Maximum allowed per-component count delta.
        #[arg(long, default_value_t = 2)]
        tolerance: usize,
    },

    /// Enforce per-component snapshot count budgets.
    SnapshotCount {
        /// Directory tree containing `.snap` files.
        #[arg(long, default_value = "crates")]
        snapshots_dir: PathBuf,

        /// Minimum snapshot count per detected state variant.
        #[arg(long, default_value_t = 3)]
        min_per_variant: usize,

        /// Hard ceiling — maximum snapshots per component regardless of
        /// anatomy size.
        #[arg(long, default_value_t = 40)]
        max_per_component: usize,

        /// Multiplier applied to `state_variants × anatomy_parts` when
        /// computing the per-component soft budget. The soft budget is
        /// `min(per_part_per_variant × variants × parts,
        /// max_per_component)`.
        #[arg(long, default_value_t = 3)]
        per_part_per_variant: usize,
    },

    /// Verify each error enum variant appears in a test function.
    ErrorVariantCoverage {
        /// Glob selecting Rust source files containing the enum.
        #[arg(long, default_value = "crates/ars-core/src/**/*.rs")]
        source_glob: String,

        /// Glob selecting Rust test files to inspect.
        #[arg(long, default_value = "crates/ars-core/tests/**/*.rs")]
        test_glob: String,

        /// Enum name whose variants must be exercised.
        #[arg(long, default_value = "ComponentError")]
        enum_name: String,
    },
}

#[derive(Subcommand)]
enum ExamplesCommand {
    /// List runnable browser examples.
    List,

    /// Serve one browser example.
    Serve {
        /// Example name, such as "widgets-leptos".
        name: String,

        /// Port for the dev server.
        #[arg(long)]
        port: Option<u16>,

        /// Whether to open a browser.
        #[arg(long, default_value_t = true, default_missing_value = "true", num_args = 0..=1)]
        open: bool,

        /// Whether Dioxus examples should enable CLI hot reload.
        #[arg(long, default_value_t = false, default_missing_value = "true", num_args = 0..=1)]
        hot_reload: bool,
    },

    /// Run one Dioxus example in desktop mode.
    Desktop {
        /// Dioxus example name, such as "widgets-dioxus-tailwind".
        name: String,

        /// Whether Dioxus should enable CLI hot reload.
        #[arg(long, default_value_t = false, default_missing_value = "true", num_args = 0..=1)]
        hot_reload: bool,
    },
}

#[derive(Subcommand)]
enum E2eCommand {
    /// Run all input component E2E harnesses against a widget example.
    Input {
        /// Adapter example to exercise.
        #[arg(long, value_enum, default_value_t = e2e::Adapter::Leptos)]
        adapter: e2e::Adapter,

        /// Port for the example server.
        #[arg(long)]
        port: Option<u16>,

        /// `WebDriver` endpoint. Defaults to `WEBDRIVER_URL` or local `ChromeDriver`
        /// on port 9515.
        #[arg(long)]
        webdriver_url: Option<String>,

        /// Use an already-running example server instead of spawning one.
        #[arg(long)]
        no_server: bool,

        /// Run Chrome with a visible browser window.
        #[arg(long)]
        headed: bool,
    },

    /// Run all navigation component E2E harnesses against a widget example.
    Navigation {
        /// Adapter example to exercise.
        #[arg(long, value_enum, default_value_t = e2e::Adapter::Leptos)]
        adapter: e2e::Adapter,

        /// Port for the example server.
        #[arg(long)]
        port: Option<u16>,

        /// `WebDriver` endpoint. Defaults to `WEBDRIVER_URL` or local `ChromeDriver`
        /// on port 9515.
        #[arg(long)]
        webdriver_url: Option<String>,

        /// Use an already-running example server instead of spawning one.
        #[arg(long)]
        no_server: bool,

        /// Run Chrome with a visible browser window.
        #[arg(long)]
        headed: bool,
    },

    /// Run all utility component E2E harnesses against a widget example.
    Utility {
        /// Adapter example to exercise.
        #[arg(long, value_enum, default_value_t = e2e::Adapter::Leptos)]
        adapter: e2e::Adapter,

        /// Port for the example server.
        #[arg(long)]
        port: Option<u16>,

        /// `WebDriver` endpoint. Defaults to `WEBDRIVER_URL` or local `ChromeDriver`
        /// on port 9515.
        #[arg(long)]
        webdriver_url: Option<String>,

        /// Use an already-running example server instead of spawning one.
        #[arg(long)]
        no_server: bool,

        /// Run Chrome with a visible browser window.
        #[arg(long)]
        headed: bool,
    },

    /// Run Dioxus desktop-mode E2E smoke checks.
    Desktop {
        /// Dioxus desktop example to exercise.
        #[arg(long, value_enum, default_value_t = e2e::DesktopExample::DioxusTailwind)]
        example: e2e::DesktopExample,
    },

    /// Run browser smoke checks against public widgets examples.
    Widgets {
        /// Public widgets example to exercise.
        #[arg(long, value_enum, default_value_t = e2e::WidgetsExample::LeptosTailwind)]
        example: e2e::WidgetsExample,

        /// Port for the example server.
        #[arg(long)]
        port: Option<u16>,

        /// `WebDriver` endpoint. Defaults to `WEBDRIVER_URL` or local `ChromeDriver`
        /// on port 9515.
        #[arg(long)]
        webdriver_url: Option<String>,

        /// Use an already-running example server instead of spawning one.
        #[arg(long)]
        no_server: bool,

        /// Run Chrome with a visible browser window.
        #[arg(long)]
        headed: bool,
    },
}

#[derive(Subcommand)]
enum TestCommand {
    /// Run the workspace all-target test stage with ars-i18n backend splitting.
    Unit,

    /// Run browser-executed wasm tests for ars-i18n's web-intl backend.
    I18nBrowser,

    /// Run browser-executed wasm tests for ars-dom's web feature.
    DomBrowser,

    /// Run the release verification test stage.
    Release,

    /// Run the integration test stage.
    Integration,

    /// Run the adapter and harness test stage.
    Adapter,

    /// Run the ars-core feature-matrix tests.
    FeatureMatrixCore,

    /// Run the ars-i18n feature-matrix tests.
    FeatureMatrixI18n,

    /// Run the subsystem feature-matrix tests.
    FeatureMatrixSubsystems,

    /// Run the ars-leptos feature-matrix tests.
    FeatureMatrixLeptos,

    /// Run the ars-dioxus feature-matrix tests.
    FeatureMatrixDioxus,
}

#[derive(Subcommand)]
enum SpecCommand {
    /// Show component metadata.
    Info {
        /// Component name (e.g., "checkbox", "date-picker").
        component: String,
    },

    /// List all files needed to review a component.
    Deps {
        /// Component name.
        component: String,
    },

    /// List all components in a category.
    Category {
        /// Category name (e.g., "selection", "overlay").
        name: String,
    },

    /// Find components depending on a shared type.
    Reverse {
        /// Shared type name (e.g., "selection-patterns").
        shared_type: String,
    },

    /// List a component and its related components.
    Related {
        /// Component name.
        component: String,
    },

    /// Report component dependency metadata.
    ComponentDeps {
        /// Optional positional component name. Use "all" for every component.
        name: Option<String>,

        /// Component name, or "all".
        #[arg(long)]
        component: Option<String>,

        /// Report every component.
        #[arg(long)]
        all: bool,

        /// Adapter filter: "leptos" or "dioxus".
        #[arg(long)]
        adapter: Option<String>,
    },

    /// Report or synchronize adapter issue dependency metadata.
    IssueDeps {
        /// Adapter filter: "leptos" or "dioxus".
        #[arg(long)]
        adapter: String,

        /// Component name, or "all".
        #[arg(long, default_value = "all")]
        component: String,

        /// Print the expected issue dependency changes without mutating GitHub.
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,

        /// Apply missing native GitHub dependencies.
        #[arg(long)]
        apply: bool,
    },

    /// List files for a review profile.
    Profile {
        /// Profile name (e.g., "accessibility").
        name: String,
    },

    /// Output heading structure of a spec file.
    Toc {
        /// Path to spec file (relative to spec/ or absolute).
        file: String,
    },

    /// Validate frontmatter against manifest.
    Validate,

    /// Validate a component adapter implementation sketch.
    ValidateSketch {
        /// Path to the sketch markdown file.
        file: PathBuf,
    },

    /// Lint Rust code blocks under §1.1 / §1.2 of every component spec
    /// for missing `///` doc comments and missing `Debug`/`Clone` derives
    /// on `Props`/`Api`.
    LintCode,

    /// Parse every fenced Rust code block across the spec (foundation,
    /// components, leptos-components, dioxus-components, shared, testing)
    /// with `syn::parse_file` and report syntax errors. Skips blocks tagged
    /// `rust,no_check` / `rust,ignore` / `rust,no_run`.
    CompileSnippets {
        /// Auto-tag every failing block's opening fence with `,no_check`
        /// in place. Useful for converting pre-existing partial-Rust
        /// snippets into explicitly-opted-out blocks in a single pass.
        #[arg(long)]
        fix: bool,
    },

    /// List adapter files for a framework.
    Adapters {
        /// Framework: "leptos" or "dioxus".
        framework: String,
    },

    /// Get a compact summary of a component.
    Digest {
        /// Component name.
        component: String,
    },

    /// Get full implementation context for a component.
    Context {
        /// Component name.
        component: String,

        /// Framework: "leptos" or "dioxus" (includes adapter spec).
        #[arg(long)]
        framework: Option<String>,

        /// Include testing specs.
        #[arg(long)]
        include_testing: bool,
    },

    /// Search spec content by keyword/regex.
    Search {
        /// Regex pattern to search for.
        query: String,

        /// Category filter.
        #[arg(long)]
        category: Option<String>,

        /// Section filter (states, events, props, accessibility, anatomy, i18n, forms).
        #[arg(long)]
        section: Option<String>,

        /// Tier filter (stateless, stateful, complex).
        #[arg(long)]
        tier: Option<String>,
    },
}

/// Discover the spec root or exit with a diagnostic.
fn discover_spec_root() -> manifest::SpecRoot {
    let cwd = env::current_dir().expect("cannot read current directory");

    match manifest::SpecRoot::discover(&cwd) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "xtask CLI dispatch keeps command routing visible in one match"
)]
fn main() {
    let cli = Cli::parse();

    match cli.command {
        // ── CI ────────────────────────────────────────────────────────
        Command::Ci {
            steps,
            profile,
            message_format,
        } => {
            if let Err(e) = ci::run(steps, profile, message_format.as_deref()) {
                eprintln!("error: {e}");

                process::exit(1);
            }
        }

        // ── Format (dev) ─────────────────────────────────────────────
        Command::Fmt { stdin } => {
            let result = if stdin {
                ci::format_stdin()
            } else {
                ci::format_workspace()
            };

            if let Err(e) = result {
                eprintln!("error: {e}");

                process::exit(1);
            }
        }

        // ── Clippy (dev) ─────────────────────────────────────────────
        Command::Clippy { message_format } => {
            if let Err(e) = ci::clippy_workspace(message_format.as_deref(), false) {
                eprintln!("error: {e}");

                process::exit(1);
            }
        }

        // ── MCP ───────────────────────────────────────────────────────
        #[cfg(feature = "mcp")]
        Command::Mcp => {
            let root = sync::Arc::new(discover_spec_root());
            let rt = tokio::runtime::Runtime::new().expect("cannot create tokio runtime");
            rt.block_on(mcp::serve(root)).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });
        }

        // ── Coverage ──────────────────────────────────────────────────
        Command::Coverage { cmd } => {
            let result = match cmd {
                CoverageCommand::Wasm {
                    package,
                    file,
                    features,
                    no_default_features,
                    extra_test_args,
                } => coverage::generate_wasm_lcov(&coverage::WasmCoverageOptions {
                    package,
                    output: file,
                    features,
                    no_default_features,
                    extra_test_args,
                }),

                CoverageCommand::Merge { output, files } => coverage::merge_files(&files, &output),

                CoverageCommand::Check {
                    file,
                    package,
                    min,
                    branch_min,
                } => coverage::check(&file, &package, min, branch_min),

                CoverageCommand::CheckAll { file } => {
                    coverage::check_all(&file, &coverage::default_thresholds())
                }
            };

            match result {
                Ok(output) => print!("{output}"),
                Err(e) => {
                    eprintln!("{e}");

                    process::exit(1);
                }
            }
        }

        // ── Lint ──────────────────────────────────────────────────────
        Command::Lint { cmd } => {
            let result = match cmd {
                LintCommand::AdapterParity {
                    leptos_test_dir,
                    dioxus_test_dir,
                    leptos_src_dir,
                    dioxus_src_dir,
                    tolerance,
                } => lint::check_adapter_parity(&lint::AdapterParityOptions {
                    leptos_test_dir,
                    dioxus_test_dir,
                    leptos_src_dir,
                    dioxus_src_dir,
                    tolerance,
                    ..lint::AdapterParityOptions::workspace_defaults()
                }),

                LintCommand::SnapshotCount {
                    snapshots_dir,
                    min_per_variant,
                    max_per_component,
                    per_part_per_variant,
                } => lint::check_snapshot_count(&lint::SnapshotCountOptions {
                    snapshots_dir,
                    min_per_variant,
                    max_per_component,
                    per_part_per_variant,
                }),

                LintCommand::ErrorVariantCoverage {
                    source_glob,
                    test_glob,
                    enum_name,
                } => lint::check_error_variant_coverage(&lint::ErrorVariantCoverageOptions {
                    source_glob,
                    test_glob,
                    enum_name,
                }),
            };

            match result {
                Ok(output) => print!("{output}"),
                Err(e) => {
                    eprintln!("{e}");

                    process::exit(1);
                }
            }
        }

        // ── Examples ─────────────────────────────────────────────────
        Command::Examples { cmd } => {
            let result = match cmd {
                ExamplesCommand::List => {
                    print!("{}", examples::list());

                    Ok(())
                }

                ExamplesCommand::Serve {
                    name,
                    port,
                    open,
                    hot_reload,
                } => examples::serve(&name, port, open, hot_reload),

                ExamplesCommand::Desktop { name, hot_reload } => {
                    examples::serve_desktop(&name, hot_reload)
                }
            };

            if let Err(e) = result {
                eprintln!("error: {e}");

                process::exit(1);
            }
        }

        // ── E2E ───────────────────────────────────────────────────────
        Command::E2e { cmd } => {
            let result = match cmd {
                E2eCommand::Input {
                    adapter,
                    port,
                    webdriver_url,
                    no_server,
                    headed,
                } => e2e::run_input(&e2e::Options {
                    adapter,
                    port,
                    webdriver_url,
                    no_server,
                    headless: !headed,
                }),

                E2eCommand::Navigation {
                    adapter,
                    port,
                    webdriver_url,
                    no_server,
                    headed,
                } => e2e::run_navigation(&e2e::Options {
                    adapter,
                    port,
                    webdriver_url,
                    no_server,
                    headless: !headed,
                }),

                E2eCommand::Utility {
                    adapter,
                    port,
                    webdriver_url,
                    no_server,
                    headed,
                } => e2e::run_utility(&e2e::Options {
                    adapter,
                    port,
                    webdriver_url,
                    no_server,
                    headless: !headed,
                }),

                E2eCommand::Desktop { example } => e2e::run_desktop(example),

                E2eCommand::Widgets {
                    example,
                    port,
                    webdriver_url,
                    no_server,
                    headed,
                } => e2e::run_widgets(
                    example,
                    &e2e::Options {
                        adapter: e2e::Adapter::Leptos,
                        port,
                        webdriver_url,
                        no_server,
                        headless: !headed,
                    },
                ),
            };

            if let Err(e) = result {
                eprintln!("error: {e}");

                process::exit(1);
            }
        }

        // ── Mutants ───────────────────────────────────────────────────
        Command::Mutants {
            package,
            file,
            list,
            shard,
            output,
            features,
        } => {
            if let Err(e) = mutants::run(&mutants::Options {
                package,
                file,
                list,
                shard,
                output,
                features,
            }) {
                eprintln!("error: {e}");

                process::exit(1);
            }
        }

        // ── Crap (dev) ───────────────────────────────────────────────
        Command::Crap {
            lcov,
            threshold,
            update_baseline,
        } => {
            let action = if update_baseline {
                crap::Action::UpdateBaseline
            } else {
                crap::Action::Report
            };

            if let Err(e) = crap::run(&crap::Options {
                action,
                lcov,
                baseline: PathBuf::from(crap::BASELINE_PATH),
                threshold: threshold.unwrap_or(crap::CRAP_THRESHOLD),
            }) {
                eprintln!("error: {e}");

                process::exit(1);
            }
        }

        // ── Coverage (dev) ───────────────────────────────────────────
        Command::Cov { full } => {
            let step = if full {
                ci::Step::Coverage
            } else {
                ci::Step::CoverageNative
            };

            if let Err(e) = ci::run(vec![step], ci::Profile::Full, None) {
                eprintln!("error: {e}");

                process::exit(1);
            }
        }

        // ── Test ──────────────────────────────────────────────────────
        Command::Test { cmd } => {
            let stage = cmd.map(|cmd| match cmd {
                TestCommand::Unit => test::Stage::Unit,
                TestCommand::I18nBrowser => test::Stage::I18nBrowser,
                TestCommand::DomBrowser => test::Stage::DomBrowser,
                TestCommand::Release => test::Stage::Release,
                TestCommand::Integration => test::Stage::Integration,
                TestCommand::Adapter => test::Stage::Adapter,
                TestCommand::FeatureMatrixCore => test::Stage::FeatureMatrixCore,
                TestCommand::FeatureMatrixI18n => test::Stage::FeatureMatrixI18n,
                TestCommand::FeatureMatrixSubsystems => test::Stage::FeatureMatrixSubsystems,
                TestCommand::FeatureMatrixLeptos => test::Stage::FeatureMatrixLeptos,
                TestCommand::FeatureMatrixDioxus => test::Stage::FeatureMatrixDioxus,
            });

            if let Err(e) = test::run(stage) {
                eprintln!("error: {e}");

                process::exit(1);
            }
        }

        // ── Spec ──────────────────────────────────────────────────────
        Command::Spec { cmd } => {
            let root = discover_spec_root();

            let result = match cmd {
                SpecCommand::Info { component } => spec::info::execute(&root, &component),

                SpecCommand::Deps { component } => spec::deps::execute(&root, &component),

                SpecCommand::Category { name } => spec::category::execute(&root, &name),

                SpecCommand::Reverse { shared_type } => spec::reverse::execute(&root, &shared_type),

                SpecCommand::Related { component } => spec::related::execute(&root, &component),

                SpecCommand::ComponentDeps {
                    name,
                    component,
                    all,
                    adapter,
                } => {
                    let component = if all {
                        Some("all")
                    } else {
                        component.as_deref().or(name.as_deref())
                    };

                    spec::component_deps::execute(&root, component, adapter.as_deref())
                }

                SpecCommand::IssueDeps {
                    adapter,
                    component,
                    dry_run: _,
                    apply,
                } => spec::issue_deps::execute(&root, &adapter, Some(&component), !apply),

                SpecCommand::Profile { name } => spec::profile::execute(&root, &name),

                SpecCommand::Toc { file } => spec::toc::execute(&root, &file),

                SpecCommand::Validate => {
                    let report = spec::validate::execute(&root);

                    if let Ok(text) = &report
                        && text.contains("error(s) found:")
                    {
                        print!("{text}");

                        process::exit(1);
                    }

                    report
                }

                SpecCommand::ValidateSketch { file } => {
                    let report = spec::sketch::execute(&file);

                    if let Ok(text) = &report
                        && text.contains("sketch error(s) found:")
                    {
                        print!("{text}");

                        process::exit(1);
                    }

                    report
                }

                SpecCommand::LintCode => {
                    let report = spec::lint_code::execute(&root);

                    if let Ok(text) = &report
                        && text.contains("finding(s) across")
                    {
                        print!("{text}");

                        process::exit(1);
                    }

                    report
                }

                SpecCommand::CompileSnippets { fix } => {
                    let report = spec::compile_snippets::execute(&root, fix);

                    if let Ok(text) = &report
                        && text.contains("finding(s) across")
                    {
                        print!("{text}");

                        process::exit(1);
                    }

                    report
                }

                SpecCommand::Adapters { framework } => spec::adapters::execute(&root, &framework),

                SpecCommand::Digest { component } => spec::digest::execute(&root, &component),

                SpecCommand::Context {
                    component,
                    framework,
                    include_testing,
                } => {
                    spec::context::execute(&root, &component, framework.as_deref(), include_testing)
                }

                SpecCommand::Search {
                    query,
                    category,
                    section,
                    tier,
                } => spec::search::execute(
                    &root,
                    &query,
                    category.as_deref(),
                    section.as_deref(),
                    tier.as_deref(),
                ),
            };

            match result {
                Ok(output) => print!("{output}"),
                Err(e) => {
                    eprintln!("error: {e}");

                    process::exit(1);
                }
            }
        }

        // ── Adapter helpers ──────────────────────────────────────────
        Command::Adapter { cmd } => {
            let result = match cmd {
                AdapterCommand::Scaffold {
                    component,
                    category,
                    leptos_only,
                    dioxus_only,
                    form_control,
                } => adapter::scaffold(&adapter::ScaffoldOptions {
                    component,
                    category,
                    leptos: !dioxus_only,
                    dioxus: !leptos_only,
                    form_control,
                    root: env::current_dir().expect("cannot read current directory"),
                }),
            };

            match result {
                Ok(output) => print!("{output}"),

                Err(e) => {
                    eprintln!("error: {e}");

                    process::exit(1);
                }
            }
        }
    }
}
