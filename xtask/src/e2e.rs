//! Dispatches browser E2E harnesses.

use std::{
    fmt::{self, Display},
    process::{Command, Stdio},
};

use clap::ValueEnum;

/// Adapter fixture covered by an E2E run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Adapter {
    /// Run against the Leptos E2E fixture.
    Leptos,

    /// Run against the Dioxus E2E fixture.
    Dioxus,
}

impl Adapter {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Leptos => "leptos",
            Self::Dioxus => "dioxus",
        }
    }
}

/// Dioxus desktop example covered by an E2E smoke check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum DesktopExample {
    /// Plain Dioxus widgets example.
    Dioxus,

    /// Dioxus CSS widgets example.
    DioxusCss,

    /// Dioxus Tailwind widgets example.
    DioxusTailwind,
}

impl DesktopExample {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Dioxus => "dioxus",
            Self::DioxusCss => "dioxus-css",
            Self::DioxusTailwind => "dioxus-tailwind",
        }
    }
}

/// Component category covered by an E2E run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// Run navigation component E2E harnesses.
    Navigation,

    /// Run utility component E2E harnesses.
    Utility,
}

impl Category {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Navigation => "navigation",
            Self::Utility => "utility",
        }
    }
}

/// Runtime options for an E2E harness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Adapter fixture to exercise.
    pub adapter: Adapter,

    /// Port used by the example server.
    pub port: Option<u16>,

    /// `WebDriver` endpoint. Defaults to `WEBDRIVER_URL`, then local `ChromeDriver`
    /// on port 9515.
    pub webdriver_url: Option<String>,

    /// Use an already-running example server instead of spawning one.
    pub no_server: bool,

    /// Whether Chrome should run without a visible browser window.
    pub headless: bool,
}

/// Runs browser E2E harnesses for a component category through the standalone E2E crate.
///
/// # Errors
///
/// Returns an error when the standalone harness cannot be spawned or exits
/// unsuccessfully.
pub fn run_category(category: Category, options: &Options) -> Result<(), Error> {
    let status = category_command(category, options)
        .status()
        .map_err(|error| {
            Error::Command(format!(
                "failed to run ars-e2e {}: {error}",
                category.as_str()
            ))
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::Command(format!(
            "ars-e2e {} exited with {status}",
            category.as_str()
        )))
    }
}

/// Runs the navigation browser E2E harnesses through the standalone E2E crate.
///
/// # Errors
///
/// Returns an error when the standalone harness cannot be spawned or exits
/// unsuccessfully.
pub fn run_navigation(options: &Options) -> Result<(), Error> {
    run_category(Category::Navigation, options)
}

/// Runs the utility browser E2E harnesses through the standalone E2E crate.
///
/// # Errors
///
/// Returns an error when the standalone harness cannot be spawned or exits
/// unsuccessfully.
pub fn run_utility(options: &Options) -> Result<(), Error> {
    run_category(Category::Utility, options)
}

/// Runs Dioxus desktop E2E smoke checks through the standalone E2E crate.
///
/// # Errors
///
/// Returns an error when the standalone harness cannot be spawned or exits
/// unsuccessfully.
pub fn run_desktop(example: DesktopExample) -> Result<(), Error> {
    let status = desktop_command(example)
        .status()
        .map_err(|error| Error::Command(format!("failed to run ars-e2e desktop: {error}")))?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::Command(format!(
            "ars-e2e desktop exited with {status}"
        )))
    }
}

/// Builds the cargo command used to dispatch category E2E harnesses.
#[must_use]
pub fn category_command(category: Category, options: &Options) -> Command {
    let mut command = Command::new("cargo");

    command
        .arg("run")
        .arg("-p")
        .arg("ars-e2e")
        .arg("--")
        .arg(category.as_str())
        .arg("--adapter")
        .arg(options.adapter.as_str());

    if let Some(port) = options.port {
        command.arg("--port").arg(port.to_string());
    }

    if let Some(webdriver_url) = &options.webdriver_url {
        command.arg("--webdriver-url").arg(webdriver_url);
    }

    if options.no_server {
        command.arg("--no-server");
    }

    if !options.headless {
        command.arg("--headed");
    }

    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    command
}

/// Builds the cargo command used to dispatch navigation E2E harnesses.
#[must_use]
pub fn navigation_command(options: &Options) -> Command {
    category_command(Category::Navigation, options)
}

/// Builds the cargo command used to dispatch desktop E2E smoke checks.
#[must_use]
pub fn desktop_command(example: DesktopExample) -> Command {
    let mut command = Command::new("cargo");

    command
        .arg("run")
        .arg("-p")
        .arg("ars-e2e")
        .arg("--")
        .arg("desktop")
        .arg("--example")
        .arg(example.as_str())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    command
}

/// Error returned by E2E dispatch.
#[derive(Debug)]
pub enum Error {
    /// The standalone harness could not be spawned or exited unsuccessfully.
    Command(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Command(error) => f.write_str(error),
        }
    }
}

impl std::error::Error for Error {}

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
    fn navigation_command_dispatches_to_standalone_e2e_crate() {
        let command = navigation_command(&Options {
            adapter: Adapter::Dioxus,
            port: Some(6123),
            webdriver_url: Some("http://127.0.0.1:9999".to_string()),
            no_server: true,
            headless: true,
        });

        assert_eq!(command.get_program().to_string_lossy(), "cargo");
        assert_eq!(
            args(&command),
            [
                "run",
                "-p",
                "ars-e2e",
                "--",
                "navigation",
                "--adapter",
                "dioxus",
                "--port",
                "6123",
                "--webdriver-url",
                "http://127.0.0.1:9999",
                "--no-server",
            ]
        );
    }

    #[test]
    fn category_command_dispatches_utility_harnesses() {
        let options = Options {
            adapter: Adapter::Leptos,
            port: None,
            webdriver_url: None,
            no_server: false,
            headless: true,
        };

        let utility = category_command(Category::Utility, &options);

        assert!(args(&utility).contains(&"utility".to_string()));
    }

    #[test]
    fn category_command_can_request_visible_browser() {
        let command = navigation_command(&Options {
            adapter: Adapter::Leptos,
            port: None,
            webdriver_url: None,
            no_server: false,
            headless: false,
        });

        assert!(args(&command).contains(&"--headed".to_string()));
    }

    #[test]
    fn desktop_command_dispatches_plain_example_to_standalone_e2e_crate() {
        let command = desktop_command(DesktopExample::Dioxus);

        assert_eq!(
            args(&command),
            [
                "run",
                "-p",
                "ars-e2e",
                "--",
                "desktop",
                "--example",
                "dioxus",
            ]
        );
    }

    #[test]
    fn desktop_command_supports_css_and_tailwind_examples() {
        let css = desktop_command(DesktopExample::DioxusCss);
        let tailwind = desktop_command(DesktopExample::DioxusTailwind);

        assert!(args(&css).contains(&"dioxus-css".to_string()));
        assert!(args(&tailwind).contains(&"dioxus-tailwind".to_string()));
    }
}
