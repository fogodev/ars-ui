//! Browser E2E harnesses for utility components.

use std::time::{Duration, Instant};

use serde_json::Value;
use thirtyfour::prelude::*;
use tokio::time;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    fixtures::{FixtureOptions, start_fixture_session},
};

/// Browser E2E harness for Button.
pub mod button;

/// Browser E2E harness for ClientOnly.
pub mod client_only;

/// Browser E2E harness for Dismissable.
pub mod dismissable;

/// Browser E2E harness for ErrorBoundary.
pub mod error_boundary;

/// Browser E2E harness for Separator.
pub mod separator;

/// Browser E2E harness for VisuallyHidden.
pub mod visually_hidden;

/// Browser E2E harness for ZIndexAllocator.
pub mod z_index_allocator;

/// Runtime options for utility E2E harnesses.
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

/// Runs all utility browser E2E harnesses.
///
/// # Errors
///
/// Returns an error when the example server, `ChromeDriver`, `WebDriver` session,
/// or browser assertions fail.
pub async fn run(options: Options) -> Result<(), Error> {
    let adapter = options.adapter;

    let session = start_fixture_session(FixtureOptions {
        adapter,
        port: options.port,
        webdriver_url: options.webdriver_url,
        no_server: options.no_server,
        headless: options.headless,
    })
    .await?;

    let run = async {
        button::run_button_flow(&session.driver, &session.url, adapter).await?;
        visually_hidden::run_visually_hidden_flow(&session.driver, &session.url, adapter).await?;
        separator::run_separator_flow(&session.driver, &session.url, adapter).await?;
        client_only::run_client_only_flow(&session.driver, &session.url, adapter).await?;
        z_index_allocator::run_z_index_allocator_flow(&session.driver, &session.url, adapter)
            .await?;
        dismissable::run_dismissable_flow(&session.driver, &session.url).await?;
        error_boundary::run_error_boundary_flow(&session.driver, &session.url).await
    }
    .await;

    let quit = session.quit().await;

    run?;
    quit?;

    Ok(())
}

pub(crate) async fn open_utility_panel(driver: &WebDriver, url: &str) -> Result<(), Error> {
    driver.goto(url).await?;

    let utility = visible_tab(driver, "Utility").await?;

    utility.click().await?;

    wait_for_text(driver, "Button variants").await
}

pub(crate) async fn element_by_id(driver: &WebDriver, id: &str) -> Result<WebElement, Error> {
    let element = driver.find(By::Id(id)).await?;

    if element.is_displayed().await? {
        Ok(element)
    } else {
        Err(Error::Assertion(format!("element #{id} must be visible")))
    }
}

pub(crate) async fn assert_attr(
    element: &WebElement,
    name: &str,
    expected: &str,
) -> Result<(), Error> {
    let value = element.attr(name).await?;

    if value.as_deref() == Some(expected) {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "expected {:?} on element {:?} to be {expected:?}, got {value:?}",
            name,
            element.attr("id").await?.unwrap_or_default()
        )))
    }
}

pub(crate) async fn assert_attr_present(element: &WebElement, name: &str) -> Result<(), Error> {
    let value = element.attr(name).await?;

    if value.is_some() {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "expected {:?} on element {:?} to be present",
            name,
            element.attr("id").await?.unwrap_or_default()
        )))
    }
}

pub(crate) async fn assert_bool_attr(element: &WebElement, name: &str) -> Result<(), Error> {
    let value = element.attr(name).await?;

    if matches!(value.as_deref(), Some("") | Some("true")) {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "expected boolean attr {:?} on element {:?} to be present, got {value:?}",
            name,
            element.attr("id").await?.unwrap_or_default()
        )))
    }
}

pub(crate) async fn active_id(driver: &WebDriver) -> Result<Option<String>, Error> {
    Ok(driver.active_element().await?.attr("id").await?)
}

pub(crate) async fn dispatch_pointer_sequence(
    driver: &WebDriver,
    element: &WebElement,
) -> Result<(), Error> {
    driver
        .execute(
            r#"
            const el = arguments[0];
            for (const type of ["pointerdown", "pointerup", "click"]) {
                const init = {
                    bubbles: true,
                    cancelable: true,
                    composed: true,
                    pointerType: "mouse",
                    pointerId: 1,
                    isPrimary: true,
                    button: 0,
                    buttons: type === "pointerdown" ? 1 : 0
                };
                const event = type === "click"
                    ? new MouseEvent(type, init)
                    : new PointerEvent(type, init);
                el.dispatchEvent(event);
            }
            "#,
            vec![element.to_json()?],
        )
        .await?;

    Ok(())
}

async fn visible_tab(driver: &WebDriver, label: &str) -> Result<WebElement, Error> {
    let deadline = Instant::now() + Duration::from_secs(15);

    let mut last_error = None;

    while Instant::now() < deadline {
        match visible_tab_now(driver, label).await {
            Ok(Some(tab)) => return Ok(tab),
            Ok(None) => time::sleep(Duration::from_millis(100)).await,
            Err(error) => {
                last_error = Some(error.to_string());
                time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    Err(Error::Timeout(format!(
        "timed out waiting for visible category tab {label:?}; last WebDriver error: {}",
        last_error.unwrap_or_else(|| "none".to_string())
    )))
}

async fn visible_tab_now(driver: &WebDriver, label: &str) -> Result<Option<WebElement>, Error> {
    let tabs = driver.find_all(By::Css("[role='tab']")).await?;

    for tab in tabs {
        let text = tab.text().await?;

        if text.contains(label) && tab.is_displayed().await? {
            return Ok(Some(tab));
        }
    }

    Ok(None)
}

async fn wait_for_text(driver: &WebDriver, text: &str) -> Result<(), Error> {
    let found = driver
        .execute(
            "return document.body && document.body.innerText.includes(arguments[0]);",
            vec![Value::String(text.to_string())],
        )
        .await?;

    if found.json().as_bool() == Some(true) {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "expected page text to contain {text:?}"
        )))
    }
}
