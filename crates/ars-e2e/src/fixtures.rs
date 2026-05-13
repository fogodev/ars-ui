//! Shared internal fixture server helpers for E2E harnesses.

use std::{
    env,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use clap::ValueEnum;
use thirtyfour::{ChromeCapabilities, ChromiumLikeCapabilities, prelude::*};

use crate::{
    Error,
    browser::{ChildGuard, maybe_spawn_chromedriver, quiet_spawn, wait_for_tcp, webdriver_url},
};

const DEFAULT_LEPTOS_PORT: u16 = 5200;
const DEFAULT_DIOXUS_PORT: u16 = 5201;

/// Adapter fixture covered by an E2E run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Adapter {
    /// Run against the Leptos E2E fixture.
    Leptos,

    /// Run against the Dioxus E2E fixture.
    Dioxus,
}

impl Adapter {
    pub(crate) const fn fixture_name(self) -> &'static str {
        match self {
            Self::Leptos => "ars-e2e-fixture-leptos",
            Self::Dioxus => "ars-e2e-fixture-dioxus",
        }
    }

    const fn fixture_path(self) -> &'static str {
        match self {
            Self::Leptos => "crates/ars-e2e/fixtures/leptos",
            Self::Dioxus => "crates/ars-e2e/fixtures/dioxus",
        }
    }

    pub(crate) const fn default_port(self) -> u16 {
        match self {
            Self::Leptos => DEFAULT_LEPTOS_PORT,
            Self::Dioxus => DEFAULT_DIOXUS_PORT,
        }
    }
}

pub(crate) fn spawn_fixture_server(adapter: Adapter, port: u16) -> Result<ChildGuard, Error> {
    let name = adapter.fixture_name();

    let mut command = server_command(adapter, port).map_err(|error| {
        Error::Command(format!("failed to build {name} server command: {error}"))
    })?;

    let child = quiet_spawn(&mut command)
        .map_err(|error| Error::Command(format!("failed to spawn {name}: {error}")))?;

    Ok(ChildGuard::new(child))
}

pub(crate) struct FixtureOptions {
    pub(crate) adapter: Adapter,
    pub(crate) port: Option<u16>,
    pub(crate) webdriver_url: Option<String>,
    pub(crate) no_server: bool,
    pub(crate) headless: bool,
}

pub(crate) struct FixtureSession {
    pub(crate) driver: WebDriver,
    pub(crate) url: String,
    _server: Option<ChildGuard>,
    _chromedriver: Option<ChildGuard>,
}

impl FixtureSession {
    pub(crate) async fn quit(self) -> Result<(), Error> {
        self.driver.quit().await?;

        Ok(())
    }
}

pub(crate) async fn start_fixture_session(
    options: FixtureOptions,
) -> Result<FixtureSession, Error> {
    let port = options
        .port
        .unwrap_or_else(|| options.adapter.default_port());

    let url = format!("http://127.0.0.1:{port}/");

    let server = if options.no_server {
        None
    } else {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return Err(Error::Command(format!(
                "fixture server port {port} is already in use; stop the existing server or pass --port"
            )));
        }

        Some(spawn_fixture_server(options.adapter, port)?)
    };

    wait_for_tcp(
        SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_secs(90),
        "fixture server",
    )?;

    let webdriver_url = webdriver_url(options.webdriver_url);

    let chromedriver = maybe_spawn_chromedriver(&webdriver_url)?;

    let caps = chrome_capabilities(options.headless)?;

    let driver = WebDriver::new(&webdriver_url, caps).await?;

    Ok(FixtureSession {
        driver,
        url,
        _server: server,
        _chromedriver: chromedriver,
    })
}

fn chrome_capabilities(headless: bool) -> WebDriverResult<ChromeCapabilities> {
    let mut caps = DesiredCapabilities::chrome();

    if headless {
        caps.add_arg("--headless=new")?;
    }

    Ok(caps)
}

pub(crate) fn server_command(adapter: Adapter, port: u16) -> Result<Command, String> {
    match adapter {
        Adapter::Leptos => leptos_command(adapter.fixture_path(), port),
        Adapter::Dioxus => dioxus_command(adapter.fixture_path(), port),
    }
}

fn leptos_command(path: &str, port: u16) -> Result<Command, String> {
    let mut command = Command::new("trunk");

    let target_dir = examples_target_dir()?;

    command
        .arg("serve")
        .arg("--open")
        .arg("false")
        .arg("--port")
        .arg(port.to_string())
        .env("CARGO_TARGET_DIR", target_dir)
        .env_remove("NO_COLOR")
        .current_dir(Path::new(path))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    Ok(command)
}

fn dioxus_command(path: &str, port: u16) -> Result<Command, String> {
    let mut command = Command::new("dx");

    let target_dir = examples_target_dir()?;

    command
        .arg("serve")
        .arg("--web")
        .arg("--hot-reload")
        .arg("false")
        .arg("--open")
        .arg("false")
        .arg("--port")
        .arg(port.to_string())
        .env("CARGO_TARGET_DIR", target_dir)
        .env_remove("NO_COLOR")
        .current_dir(Path::new(path))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    Ok(command)
}

fn examples_target_dir() -> Result<PathBuf, String> {
    Ok(env::current_dir()
        .map_err(|error| error.to_string())?
        .join("target/examples"))
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
    fn leptos_server_command_uses_internal_fixture() {
        let command = server_command(Adapter::Leptos, 5200).expect("command should build");

        assert_eq!(command.get_program().to_string_lossy(), "trunk");
        assert_eq!(
            command.get_current_dir(),
            Some(Path::new("crates/ars-e2e/fixtures/leptos"))
        );
        assert_eq!(
            args(&command),
            ["serve", "--open", "false", "--port", "5200"]
        );
    }

    #[test]
    fn dioxus_server_command_uses_internal_fixture_without_hot_reload() {
        let command = server_command(Adapter::Dioxus, 5201).expect("command should build");

        assert_eq!(command.get_program().to_string_lossy(), "dx");
        assert_eq!(
            command.get_current_dir(),
            Some(Path::new("crates/ars-e2e/fixtures/dioxus"))
        );
        assert_eq!(
            args(&command),
            [
                "serve",
                "--web",
                "--hot-reload",
                "false",
                "--open",
                "false",
                "--port",
                "5201",
            ]
        );
    }

    #[test]
    fn chrome_capabilities_adds_headless_arg_by_default() {
        let caps = chrome_capabilities(true).expect("Chrome capabilities should build");

        assert!(caps.args().contains(&"--headless=new".to_string()));
    }

    #[test]
    fn chrome_capabilities_can_leave_browser_visible() {
        let caps = chrome_capabilities(false).expect("Chrome capabilities should build");

        assert!(!caps.args().contains(&"--headless=new".to_string()));
    }
}
