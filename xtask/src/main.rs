//! ars-ui workspace task runner.

use std::{env, path::PathBuf, process, sync};

use clap::{Parser, Subcommand};
use xtask::{ci, coverage, manifest, mcp, spec, test};

/// ars-ui workspace task runner.
#[derive(Parser)]
#[command(name = "xtask", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run CI pipeline steps locally (alias: `cargo xci`).
    Ci {
        /// Steps to run (default: all in pipeline order).
        ///
        /// Use `feature-matrix` to run all five feature-flag groups at once.
        #[arg(value_enum)]
        steps: Vec<ci::Step>,

        /// Forward `--message-format` to cargo check/clippy (e.g. `json` for
        /// rust-analyzer's `overrideCommand`).
        #[arg(long)]
        message_format: Option<String>,
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

    /// Code coverage threshold enforcement.
    Coverage {
        #[command(subcommand)]
        cmd: CoverageCommand,
    },

    /// Run workspace tests through cargo-nextest.
    Test {
        #[command(subcommand)]
        cmd: Option<TestCommand>,
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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        // ── CI ────────────────────────────────────────────────────────
        Command::Ci {
            steps,
            message_format,
        } => {
            if let Err(e) = ci::run(steps, message_format.as_deref()) {
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
    }
}
