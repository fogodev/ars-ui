//! Browser E2E harness for the Dismissable component.

use std::time::{Duration, Instant};

use serde_json::Value;
use thirtyfour::prelude::*;
use tokio::time;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    assertions::assert_accessibility_tree_has_role_and_name,
    axe::run_axe,
    fixtures::{FixtureOptions, start_fixture_session},
    utility::{assert_attr, open_utility_panel},
};

/// Runtime options for the Dismissable E2E harness.
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

/// Runs the Dismissable browser E2E harness.
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

    let run = run_dismissable_flow(&session.driver, &session.url).await;

    let quit = session.quit().await;

    run?;
    quit?;

    Ok(())
}

pub(super) async fn run_dismissable_flow(driver: &WebDriver, url: &str) -> Result<(), Error> {
    open_utility_panel(driver, url).await?;

    driver
        .execute(
            "document.documentElement.lang ||= 'en';",
            Vec::<Value>::new(),
        )
        .await?;

    run_axe(driver).await?;

    assert_accessibility_tree_has_role_and_name(driver, "button", "Dismiss example region").await?;
    assert_region_anatomy(driver).await?;
    assert_inside_pointer_does_not_dismiss(driver).await?;
    assert_dismiss_button_keyboard_activation(driver).await?;
    assert_escape_dismisses(driver).await?;
    assert_outside_pointer_dismisses(driver).await?;
    assert_outside_focus_dismisses(driver).await?;

    Ok(())
}

async fn assert_region_anatomy(driver: &WebDriver) -> Result<(), Error> {
    let root = region_root(driver).await?;

    assert_attr(&root, "data-ars-scope", "dismissable").await?;
    assert_attr(&root, "data-ars-part", "root").await?;

    let buttons = dismiss_buttons(driver).await?;

    if buttons.len() != 2 {
        return Err(Error::Assertion(format!(
            "Dismissable Region must render start and end dismiss buttons, got {}",
            buttons.len()
        )));
    }

    for button in buttons {
        let tag = button.tag_name().await?;

        if tag != "button" {
            return Err(Error::Assertion(format!(
                "dismiss button must render as a native button, got tag {tag:?}"
            )));
        }

        assert_attr(&button, "data-ars-scope", "dismissable").await?;
        assert_attr(&button, "data-ars-part", "dismiss-button").await?;
        assert_attr(&button, "aria-label", "Dismiss example region").await?;
        assert_attr(&button, "type", "button").await?;
    }

    Ok(())
}

async fn assert_inside_pointer_does_not_dismiss(driver: &WebDriver) -> Result<(), Error> {
    let status_before = status_text(driver).await?;

    let card = driver.find(By::Css(".dismissable-card")).await?;

    card.click().await?;

    time::sleep(Duration::from_millis(150)).await;

    let status_after = status_text(driver).await?;
    if status_after == status_before {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "inside pointer interaction must not dismiss region; before={status_before:?} after={status_after:?}"
        )))
    }
}

async fn assert_dismiss_button_keyboard_activation(driver: &WebDriver) -> Result<(), Error> {
    let button = dismiss_buttons(driver)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| Error::Assertion("missing dismiss button".to_string()))?;

    button
        .handle()
        .execute("arguments[0].focus();", vec![button.to_json()?])
        .await?;

    button.send_keys(Key::Enter).await?;

    wait_for_status_contains(driver, "DismissButton").await
}

async fn assert_escape_dismisses(driver: &WebDriver) -> Result<(), Error> {
    region_root(driver).await?.focus().await?;

    driver
        .action_chain()
        .send_keys(Key::Escape)
        .perform()
        .await?;

    wait_for_status_contains(driver, "Escape").await
}

async fn assert_outside_pointer_dismisses(driver: &WebDriver) -> Result<(), Error> {
    driver
        .execute(
            r#"
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

                document.body.dispatchEvent(event);
            }
            "#,
            Vec::<Value>::new(),
        )
        .await?;

    wait_for_status_contains(driver, "OutsidePointer").await
}

async fn assert_outside_focus_dismisses(driver: &WebDriver) -> Result<(), Error> {
    driver
        .execute(
            r#"
            let button = document.getElementById("ars-e2e-outside-focus");

            if (!button) {
                button = document.createElement("button");
                button.id = "ars-e2e-outside-focus";
                button.textContent = "outside focus target";
                document.body.append(button);
            }

            button.focus();
            "#,
            Vec::<Value>::new(),
        )
        .await?;

    wait_for_status_contains(driver, "OutsideFocus").await
}

async fn region_root(driver: &WebDriver) -> Result<WebElement, Error> {
    driver
        .find(By::Css(
            "[data-ars-scope='dismissable'][data-ars-part='root']",
        ))
        .await
        .map_err(Error::from)
}

async fn dismiss_buttons(driver: &WebDriver) -> Result<Vec<WebElement>, Error> {
    Ok(driver
        .find_all(By::Css(
            "button[data-ars-scope='dismissable'][data-ars-part='dismiss-button']",
        ))
        .await?)
}

async fn status_text(driver: &WebDriver) -> Result<String, Error> {
    Ok(driver
        .find(By::Css(".dismissable-status"))
        .await?
        .text()
        .await?)
}

async fn wait_for_status_contains(driver: &WebDriver, expected: &str) -> Result<(), Error> {
    let deadline = Instant::now() + Duration::from_secs(5);

    let mut last = String::new();

    while Instant::now() < deadline {
        last = status_text(driver).await?;

        if last.contains(expected) {
            return Ok(());
        }

        time::sleep(Duration::from_millis(50)).await;
    }

    Err(Error::Timeout(format!(
        "timed out waiting for dismissable status to contain {expected:?}; last status={last:?}"
    )))
}
