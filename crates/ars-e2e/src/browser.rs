//! Shared browser process and `WebDriver` lifecycle helpers.

use std::{
    env, fs,
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

    Ok(Some(wait_for_tcp_with_child_guard(
        child,
        addr,
        Duration::from_secs(15),
        "ChromeDriver",
    )?))
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

fn wait_for_tcp_with_child_guard(
    child: Child,
    addr: SocketAddr,
    timeout: Duration,
    label: &str,
) -> Result<ChildGuard, Error> {
    let guard = ChildGuard::new(child);

    wait_for_tcp(addr, timeout, label)?;

    Ok(guard)
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
    log_path: Option<std::path::PathBuf>,
}

impl ChildGuard {
    pub(crate) const fn new(child: Child) -> Self {
        Self {
            child,
            log_path: None,
        }
    }

    pub(crate) const fn new_with_log(child: Child, log_path: std::path::PathBuf) -> Self {
        Self {
            child,
            log_path: Some(log_path),
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        drop(self.child.kill());
        drop(self.child.wait());

        if let Some(log_path) = &self.log_path {
            drop(fs::remove_file(log_path));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn guarded_tcp_wait_kills_child_on_timeout() {
        let child = Command::new("sh")
            .arg("-c")
            .arg("while true; do sleep 1; done")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("test child should spawn");

        let pid = child.id().to_string();

        let addr = SocketAddr::from(([127, 0, 0, 1], 9));

        let result =
            wait_for_tcp_with_child_guard(child, addr, Duration::from_millis(1), "test child");

        assert!(matches!(result, Err(Error::Timeout(_))));

        for _ in 0..20 {
            if !process_exists(&pid) {
                return;
            }

            thread::sleep(Duration::from_millis(10));
        }

        panic!("child process was still alive after guarded timeout");
    }

    #[cfg(unix)]
    fn process_exists(pid: &str) -> bool {
        Command::new("kill")
            .arg("-0")
            .arg(pid)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
}
