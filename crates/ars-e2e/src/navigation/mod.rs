//! E2E harnesses for navigation components.

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    fixtures::{FixtureOptions, start_fixture_session},
};

/// Browser E2E coverage for the Tabs component.
pub mod tabs;

/// Runtime options for navigation E2E harnesses.
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

/// Runs all navigation browser E2E harnesses.
///
/// # Errors
///
/// Returns an error when the example server, `ChromeDriver`, `WebDriver` session,
/// or browser assertions fail.
pub async fn run(options: Options) -> Result<(), Error> {
    let session = start_fixture_session(FixtureOptions {
        adapter: options.adapter,
        port: options.port,
        webdriver_url: options.webdriver_url,
        no_server: options.no_server,
        headless: options.headless,
    })
    .await?;

    let run = tabs::run_tabs_flow(&session.driver, &session.url).await;

    let quit = session.quit().await;

    run?;
    quit?;

    Ok(())
}
