//! Browser E2E harness for the ErrorBoundary component.

use serde_json::Value;
use thirtyfour::prelude::*;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    axe::run_axe,
    fixtures::{FixtureOptions, start_fixture_session},
    utility::{assert_attr, open_utility_panel},
};

/// Runtime options for the `ErrorBoundary` E2E harness.
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

/// Runs the `ErrorBoundary` browser E2E harness.
///
/// # Errors
///
/// Returns an error when the fixture server, `ChromeDriver`, `WebDriver` session,
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

    let run = run_error_boundary_flow(&session.driver, &session.url).await;

    let quit = session.quit().await;

    run?;
    quit?;

    Ok(())
}

pub(super) async fn run_error_boundary_flow(driver: &WebDriver, url: &str) -> Result<(), Error> {
    open_utility_panel(driver, url).await?;

    driver
        .execute(
            "document.documentElement.lang ||= 'en';",
            Vec::<Value>::new(),
        )
        .await?;

    run_axe(driver).await?;

    let alert = driver
        .find(By::Css("[data-ars-scope='error-boundary'][role='alert']"))
        .await?;

    assert_attr(&alert, "data-ars-scope", "error-boundary").await?;
    assert_attr(&alert, "data-ars-part", "root").await?;
    assert_attr(&alert, "data-ars-error", "true").await?;
    assert_attr(&alert, "data-ars-error-count", "1").await?;

    alert.find(By::Css("[data-ars-part='message']")).await?;

    let text = alert.text().await?;

    if !text.contains("A component encountered an error.") {
        return Err(Error::Assertion(format!(
            "ErrorBoundary alert should include default message, got {text:?}"
        )));
    }

    alert.find(By::Css("[data-ars-part='list']")).await?;
    alert.find(By::Css("[data-ars-part='item']")).await?;

    let healthy = driver.find(By::Css(".healthy-boundary")).await?;

    if healthy.text().await?.contains("Healthy child rendered") {
        Ok(())
    } else {
        Err(Error::Assertion(
            "healthy ErrorBoundary child should render unchanged".to_string(),
        ))
    }
}
