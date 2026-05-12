//! Shared browser process and `WebDriver` lifecycle helpers.

use std::{
    env,
    net::{SocketAddr, TcpStream},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::Error;

const DEFAULT_WEBDRIVER_PORT: u16 = 9515;

pub(crate) fn maybe_spawn_chromedriver(webdriver_url: &str) -> Result<Option<ChildGuard>, Error> {
    let Some(addr) = webdriver_addr(webdriver_url) else {
        return Ok(None);
    };

    if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
        return Ok(None);
    }

    let mut command =
        Command::new(env::var("CHROMEDRIVER").unwrap_or_else(|_| "chromedriver".into()));

    command.arg(format!("--port={}", addr.port()));

    let child = quiet_spawn(&mut command)
        .map_err(|error| Error::Command(format!("failed to spawn ChromeDriver: {error}")))?;

    wait_for_tcp(addr, Duration::from_secs(15), "ChromeDriver")?;

    Ok(Some(ChildGuard { child }))
}

pub(crate) fn webdriver_url(explicit: Option<String>) -> String {
    explicit
        .or_else(|| env::var("WEBDRIVER_URL").ok())
        .unwrap_or_else(|| format!("http://127.0.0.1:{DEFAULT_WEBDRIVER_PORT}"))
}

pub(crate) fn webdriver_addr(url: &str) -> Option<SocketAddr> {
    let rest = url.strip_prefix("http://")?;

    let host_port = rest.split('/').next()?;

    let (host, port) = host_port.rsplit_once(':')?;

    if host != "127.0.0.1" && host != "localhost" {
        return None;
    }

    let port = port.parse().ok()?;

    Some(SocketAddr::from(([127, 0, 0, 1], port)))
}

pub(crate) fn wait_for_tcp(addr: SocketAddr, timeout: Duration, label: &str) -> Result<(), Error> {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return Ok(());
        }

        thread::sleep(Duration::from_millis(100));
    }

    Err(Error::Timeout(format!(
        "timed out waiting for {label} at {addr}"
    )))
}

pub(crate) fn quiet_spawn(command: &mut Command) -> std::io::Result<Child> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

pub(crate) struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    pub(crate) const fn new(child: Child) -> Self {
        Self { child }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        drop(self.child.kill());
        drop(self.child.wait());
    }
}
