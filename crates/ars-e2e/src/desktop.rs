//! Desktop-mode E2E smoke checks for Dioxus examples.

use std::{
    fmt::{self, Display},
    path::Path,
    process::{Command, Stdio},
};

use clap::ValueEnum;

use crate::Error;

/// Dioxus desktop example covered by an E2E smoke check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Example {
    /// Plain Dioxus widgets example.
    Dioxus,

    /// Dioxus CSS widgets example.
    DioxusCss,

    /// Dioxus Tailwind widgets example.
    DioxusTailwind,
}

impl Example {
    const fn package(self) -> &'static str {
        match self {
            Self::Dioxus => "widgets-dioxus",
            Self::DioxusCss => "widgets-dioxus-css",
            Self::DioxusTailwind => "widgets-dioxus-tailwind",
        }
    }

    const fn path(self) -> &'static str {
        match self {
            Self::Dioxus => "examples/widgets-dioxus",
            Self::DioxusCss => "examples/widgets-dioxus-css",
            Self::DioxusTailwind => "examples/widgets-dioxus-tailwind",
        }
    }
}

impl Display for Example {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.package())
    }
}

/// Runtime options for desktop E2E smoke checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Dioxus desktop example to build.
    pub example: Example,
}

/// Runs the desktop smoke check.
///
/// # Errors
///
/// Returns an error when the Dioxus desktop build fails.
pub fn run(options: &Options) -> Result<(), Error> {
    let status = desktop_build_command(options).status().map_err(|error| {
        Error::Command(format!(
            "failed to run desktop E2E smoke for {}: {error}",
            options.example
        ))
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::Command(format!(
            "desktop E2E smoke for {} exited with {status}",
            options.example
        )))
    }
}

/// Builds the Dioxus desktop smoke command.
#[must_use]
pub fn desktop_build_command(options: &Options) -> Command {
    let mut command = Command::new("dx");

    command
        .arg("build")
        .arg("--platform")
        .arg("desktop")
        .arg("--package")
        .arg(options.example.package())
        .arg("--no-default-features")
        .arg("--features")
        .arg("desktop")
        .env("CARGO_TARGET_DIR", examples_target_dir())
        .env_remove("NO_COLOR")
        .current_dir(Path::new(options.example.path()))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    command
}

fn examples_target_dir() -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| Path::new(".").to_path_buf())
        .join("target/examples")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn desktop_build_command_targets_dioxus_desktop_features() {
        let command = desktop_build_command(&Options {
            example: Example::Dioxus,
        });

        assert_eq!(command.get_program().to_string_lossy(), "dx");
        assert_eq!(
            command.get_current_dir(),
            Some(Path::new("examples/widgets-dioxus"))
        );
        assert_eq!(
            args(&command),
            [
                "build",
                "--platform",
                "desktop",
                "--package",
                "widgets-dioxus",
                "--no-default-features",
                "--features",
                "desktop",
            ]
        );
    }

    #[test]
    fn desktop_build_command_targets_dioxus_css_desktop_features() {
        let command = desktop_build_command(&Options {
            example: Example::DioxusCss,
        });

        assert_eq!(
            command.get_current_dir(),
            Some(Path::new("examples/widgets-dioxus-css"))
        );
        assert!(args(&command).contains(&"widgets-dioxus-css".to_string()));
    }

    #[test]
    fn desktop_build_command_targets_dioxus_tailwind_desktop_features() {
        let command = desktop_build_command(&Options {
            example: Example::DioxusTailwind,
        });

        assert_eq!(
            command.get_current_dir(),
            Some(Path::new("examples/widgets-dioxus-tailwind"))
        );
        assert!(args(&command).contains(&"widgets-dioxus-tailwind".to_string()));
    }
}
