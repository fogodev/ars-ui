//! Shared error type for E2E harnesses.

use std::fmt::{self, Display};

use thirtyfour::error::WebDriverError;

/// Error returned by E2E harnesses.
#[derive(Debug)]
pub enum Error {
    /// A process command could not be constructed or spawned.
    Command(String),

    /// A browser assertion failed.
    Assertion(String),

    /// A server, driver, or DOM condition did not become ready in time.
    Timeout(String),

    /// The `WebDriver` client returned an error.
    WebDriver(WebDriverError),
}

impl From<WebDriverError> for Error {
    fn from(error: WebDriverError) -> Self {
        Self::WebDriver(error)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Command(error) | Self::Assertion(error) | Self::Timeout(error) => {
                f.write_str(error)
            }
            Self::WebDriver(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Error {}
