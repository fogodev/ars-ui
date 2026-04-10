//! ars-ui workspace task runner.

use std::{env, path::PathBuf, process, sync};

use clap::{Parser, Subcommand};
use xtask::{ci, coverage, manifest, mcp, spec};

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
}

#[derive(Subcommand)]
enum CoverageCommand {
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
        } => match ci::run(steps, message_format.as_deref()) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("error: {e}");
                process::exit(1);
            }
        },

        // ── Clippy (dev) ─────────────────────────────────────────────
        Command::Clippy { message_format } => {
            match ci::clippy_workspace(message_format.as_deref(), false) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
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
                CoverageCommand::Check {
                    file,
                    package,
                    min,
                    branch_min,
                } => coverage::check(&file, &package, min, branch_min),
                CoverageCommand::CheckAll { file } => {
                    let thresholds = coverage::default_thresholds();
                    coverage::check_all(&file, &thresholds)
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
                    if let Ok(ref text) = report {
                        if text.contains("error(s) found:") {
                            print!("{text}");
                            process::exit(1);
                        }
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
