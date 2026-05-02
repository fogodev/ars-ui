//! Browser example catalog and runner.

use std::{
    fmt::{self, Display},
    path::Path,
    process::{Command, Stdio},
};

/// Canonical names for all browser widget examples.
pub const EXAMPLE_NAMES: [&str; 6] = [
    "widgets-leptos",
    "widgets-dioxus",
    "widgets-leptos-css",
    "widgets-dioxus-css",
    "widgets-leptos-tailwind",
    "widgets-dioxus-tailwind",
];

/// Framework used by an example.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Framework {
    /// Leptos CSR example served through Trunk.
    Leptos,

    /// Dioxus web example served through the Dioxus CLI.
    Dioxus,
}

impl Display for Framework {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Leptos => f.write_str("leptos"),
            Self::Dioxus => f.write_str("dioxus"),
        }
    }
}

/// One runnable browser example.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Example {
    /// Public example name.
    pub name: &'static str,

    /// Relative path to the example crate.
    pub path: &'static str,

    /// Example framework.
    pub framework: Framework,
}

const CATALOG: [Example; 6] = [
    Example {
        name: "widgets-leptos",
        path: "examples/widgets-leptos",
        framework: Framework::Leptos,
    },
    Example {
        name: "widgets-dioxus",
        path: "examples/widgets-dioxus",
        framework: Framework::Dioxus,
    },
    Example {
        name: "widgets-leptos-css",
        path: "examples/widgets-leptos-css",
        framework: Framework::Leptos,
    },
    Example {
        name: "widgets-dioxus-css",
        path: "examples/widgets-dioxus-css",
        framework: Framework::Dioxus,
    },
    Example {
        name: "widgets-leptos-tailwind",
        path: "examples/widgets-leptos-tailwind",
        framework: Framework::Leptos,
    },
    Example {
        name: "widgets-dioxus-tailwind",
        path: "examples/widgets-dioxus-tailwind",
        framework: Framework::Dioxus,
    },
];

/// Returns the complete browser example catalog.
#[must_use]
pub const fn catalog() -> &'static [Example] {
    &CATALOG
}

/// Resolves an example by name.
///
/// # Errors
///
/// Returns a diagnostic listing valid example names when `name` is unknown.
pub fn resolve(name: &str) -> Result<Example, String> {
    CATALOG
        .iter()
        .copied()
        .find(|example| example.name == name)
        .ok_or_else(|| {
            format!(
                "unknown example {name:?}; expected one of: {}",
                EXAMPLE_NAMES.join(", ")
            )
        })
}

/// Formats the example catalog for terminal output.
#[must_use]
pub fn list() -> String {
    let mut output = String::new();

    for example in CATALOG {
        output.push_str(example.name);
        output.push('\t');
        output.push_str(&example.framework.to_string());
        output.push('\t');
        output.push_str(example.path);
        output.push('\n');
    }

    output
}

/// Runs a browser example dev server.
///
/// # Errors
///
/// Returns an error when the example is unknown or the underlying server exits
/// unsuccessfully.
pub fn serve(name: &str, port: Option<u16>, open: bool, hot_reload: bool) -> Result<(), String> {
    let example = resolve(name)?;

    let status = match example.framework {
        Framework::Leptos => serve_leptos(example.path, port, open),
        Framework::Dioxus => serve_dioxus(example.path, port, open, hot_reload),
    }
    .map_err(|err| format!("failed to start {name}: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("{name} exited with status {status}"))
    }
}

fn serve_leptos(
    path: &str,
    port: Option<u16>,
    open: bool,
) -> Result<std::process::ExitStatus, String> {
    leptos_command(path, port, open)?
        .status()
        .map_err(|err| err.to_string())
}

fn leptos_command(path: &str, port: Option<u16>, open: bool) -> Result<Command, String> {
    let mut command = Command::new("trunk");

    let target_dir = std::env::current_dir()
        .map_err(|err| err.to_string())?
        .join("target/examples");

    command.arg("serve").arg("--open").arg(open.to_string());

    if let Some(port) = port {
        command.arg("--port").arg(port.to_string());
    }

    command
        .env("CARGO_TARGET_DIR", target_dir)
        .env_remove("NO_COLOR")
        .current_dir(Path::new(path))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    Ok(command)
}

fn serve_dioxus(
    path: &str,
    port: Option<u16>,
    open: bool,
    hot_reload: bool,
) -> Result<std::process::ExitStatus, String> {
    dioxus_command(path, port, open, hot_reload)?
        .status()
        .map_err(|err| err.to_string())
}

fn dioxus_command(
    path: &str,
    port: Option<u16>,
    open: bool,
    hot_reload: bool,
) -> Result<Command, String> {
    let mut command = Command::new("dx");

    let target_dir = std::env::current_dir()
        .map_err(|err| err.to_string())?
        .join("target/examples");

    command
        .arg("serve")
        .arg("--web")
        .arg("--hot-reload")
        .arg(hot_reload.to_string())
        .arg("--open")
        .arg(open.to_string());

    if let Some(port) = port {
        command.arg("--port").arg(port.to_string());
    }

    command
        .env("CARGO_TARGET_DIR", target_dir)
        .env_remove("NO_COLOR")
        .current_dir(Path::new(path))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    Ok(command)
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
    fn dioxus_serve_keeps_cli_interactive_mode_enabled() {
        let command = dioxus_command("examples/widgets-dioxus-tailwind", Some(5200), false, false)
            .expect("dioxus command should build");

        let args = args(&command);

        assert!(
            !args
                .windows(2)
                .any(|window| window == ["--interactive", "false"]),
            "Dioxus serve must not disable interactive stdin shortcuts"
        );
    }
}
