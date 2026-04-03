//! ars-ui workspace task runner.

use std::process;

use clap::{Parser, Subcommand};

/// ars-ui workspace task runner.
#[derive(Parser)]
#[command(name = "xtask", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Spec navigation commands.
    Spec {
        #[command(subcommand)]
        cmd: SpecCommand,
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
}

fn main() {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().expect("cannot read current directory");
    let root = match xtask::manifest::SpecRoot::discover(&cwd) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    let result = match cli.command {
        Command::Spec { cmd } => match cmd {
            SpecCommand::Info { component } => xtask::spec::info::execute(&root, &component),
            SpecCommand::Deps { component } => xtask::spec::deps::execute(&root, &component),
            SpecCommand::Category { name } => xtask::spec::category::execute(&root, &name),
            SpecCommand::Reverse { shared_type } => {
                xtask::spec::reverse::execute(&root, &shared_type)
            }
            SpecCommand::Related { component } => xtask::spec::related::execute(&root, &component),
            SpecCommand::Profile { name } => xtask::spec::profile::execute(&root, &name),
            SpecCommand::Toc { file } => xtask::spec::toc::execute(&root, &file),
            SpecCommand::Validate => {
                let report = xtask::spec::validate::execute(&root);
                if let Ok(ref text) = report {
                    if text.contains("error(s) found:") {
                        print!("{text}");
                        process::exit(1);
                    }
                }
                report
            }
            SpecCommand::Adapters { framework } => {
                xtask::spec::adapters::execute(&root, &framework)
            }
        },
    };

    match result {
        Ok(output) => print!("{output}"),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}
