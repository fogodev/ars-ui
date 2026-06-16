//! Shared internal fixture server helpers for E2E harnesses.

use std::{
    env, fs,
    fs::OpenOptions,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use clap::ValueEnum;
use thirtyfour::{ChromeCapabilities, ChromiumLikeCapabilities, prelude::*};

use crate::{
    Error,
    browser::{ChildGuard, maybe_spawn_chromedriver, wait_for_tcp, webdriver_url},
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

    let log_path = fixture_log_path(name, port)?;

    let log_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&log_path)
        .map_err(|error| {
            Error::Command(format!(
                "failed to open fixture server log {}: {error}",
                log_path.display()
            ))
        })?;

    command
        .stdout(Stdio::from(log_file.try_clone().map_err(|error| {
            Error::Command(format!(
                "failed to clone fixture server log {}: {error}",
                log_path.display()
            ))
        })?))
        .stderr(Stdio::from(log_file));

    let child = command
        .spawn()
        .map_err(|error| Error::Command(format!("failed to spawn {name}: {error}")))?;

    wait_for_fixture_server(
        child,
        log_path,
        SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_secs(90),
        name,
    )
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

pub(crate) fn wait_for_fixture_server(
    mut child: Child,
    log_path: PathBuf,
    addr: SocketAddr,
    timeout: Duration,
    label: &str,
) -> Result<ChildGuard, Error> {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return Ok(ChildGuard::new_with_log(child, log_path));
        }

        if let Some(status) = child.try_wait().map_err(|error| {
            Error::Command(format!("failed to poll {label} process status: {error}"))
        })? {
            let log = fs::read_to_string(&log_path).unwrap_or_else(|error| {
                format!(
                    "failed to read fixture server log {}: {error}",
                    log_path.display()
                )
            });

            drop(fs::remove_file(&log_path));

            return Err(Error::Command(format!(
                "{label} exited before listening on {addr} with status {status}\n\n{log}"
            )));
        }

        thread::sleep(Duration::from_millis(100));
    }

    drop(child.kill());
    drop(child.wait());

    let log = fs::read_to_string(&log_path).unwrap_or_else(|error| {
        format!(
            "failed to read fixture server log {}: {error}",
            log_path.display()
        )
    });

    drop(fs::remove_file(&log_path));

    Err(Error::Timeout(format!(
        "timed out waiting for {label} at {addr}\n\n{log}"
    )))
}

fn fixture_log_path(name: &str, port: u16) -> Result<PathBuf, Error> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::Command(format!("system clock is before UNIX_EPOCH: {error}")))?
        .as_nanos();

    Ok(env::temp_dir().join(format!(
        "ars-ui-e2e-{name}-{port}-{}-{nanos}.log",
        std::process::id()
    )))
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

    #[test]
    #[cfg(unix)]
    fn fixture_server_wait_reports_early_exit_output() {
        let log_path = fixture_log_path("test-fixture", 5999).expect("log path should build");

        let log_file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&log_path)
            .expect("log file should open");

        let child = Command::new("sh")
            .arg("-c")
            .arg("echo fixture build failed >&2; exit 42")
            .stdout(Stdio::from(
                log_file.try_clone().expect("log file should clone"),
            ))
            .stderr(Stdio::from(log_file))
            .spawn()
            .expect("test child should spawn");

        let result = wait_for_fixture_server(
            child,
            log_path,
            SocketAddr::from(([127, 0, 0, 1], 9)),
            Duration::from_secs(1),
            "test fixture",
        );

        let Err(error) = result else {
            panic!("early exit should fail");
        };

        let message = error.to_string();

        assert!(message.contains("test fixture exited before listening"));
        assert!(message.contains("fixture build failed"));
    }

    #[test]
    #[cfg(unix)]
    fn fixture_server_wait_reports_timeout_output_and_removes_log() {
        let log_path =
            fixture_log_path("test-fixture-timeout", 5998).expect("log path should build");

        let log_file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&log_path)
            .expect("log file should open");

        let child = Command::new("sh")
            .arg("-c")
            .arg("echo fixture still building >&2; while true; do sleep 1; done")
            .stdout(Stdio::from(
                log_file.try_clone().expect("log file should clone"),
            ))
            .stderr(Stdio::from(log_file))
            .spawn()
            .expect("test child should spawn");

        let result = wait_for_fixture_server(
            child,
            log_path.clone(),
            SocketAddr::from(([127, 0, 0, 1], 9)),
            Duration::from_millis(1),
            "test fixture",
        );

        let Err(error) = result else {
            panic!("timeout should fail");
        };

        let message = error.to_string();

        assert!(message.contains("timed out waiting for test fixture"));
        assert!(message.contains("fixture still building"));
        assert!(
            !log_path.exists(),
            "fixture timeout should remove captured log file"
        );
    }
}
