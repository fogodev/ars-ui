//! Browser E2E harness for the Tabs component.

use std::time::{Duration, Instant};

use serde_json::{Value, json};
use thirtyfour::prelude::*;
use tokio::time;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    assertions::{
        assert_accessibility_tree_has_role_and_name, assert_active_role,
        assert_active_text_contains, assert_focus_indicator_visible,
        assert_forced_colors_focus_indicator_visible, assert_selected_indicator_visible,
    },
    axe::run_axe,
    fixtures::{FixtureOptions, start_fixture_session},
};

/// Runtime options for the tabs E2E harness.
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

/// Runs the tabs browser E2E harness.
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

    let run = run_tabs_flow(&session.driver, &session.url).await;

    let quit = session.quit().await;

    run?;
    quit?;

    Ok(())
}

pub(super) async fn run_tabs_flow(driver: &WebDriver, url: &str) -> Result<(), Error> {
    driver.goto(url).await?;

    let navigation = find_tab(driver, "Navigation").await?;

    navigation.click().await?;

    driver
        .execute(
            "document.documentElement.lang ||= 'en';",
            Vec::<Value>::new(),
        )
        .await?;

    run_axe(driver).await?;

    assert_accessibility_tree_has_role_and_name(driver, "tab", "Navigation").await?;
    assert_accessibility_tree_has_role_and_name(driver, "tab", "Overview").await?;
    assert_visible_tab_roledescription(driver, "Overview").await?;

    let overview = find_tab(driver, "Overview").await?;

    overview.click().await?;

    assert_selected_tab_contains(driver, "Overview").await?;

    overview.focus().await?;

    assert_active_tab_contains(driver, "Overview").await?;

    driver
        .action_chain()
        .send_keys(Key::Right)
        .perform()
        .await?;

    assert_active_tab_contains(driver, "Keyboard").await?;
    assert_selected_tab_contains(driver, "Keyboard").await?;
    assert_focus_indicator_visible(driver).await?;
    assert_forced_colors_focus_indicator_visible(driver).await?;

    let keyboard = find_tab(driver, "Keyboard").await?;

    assert_selected_indicator_visible(&keyboard, "Keyboard").await?;

    driver.action_chain().send_keys(Key::Tab).perform().await?;

    assert_active_role(driver, "tabpanel").await?;
    assert_active_text_contains(driver, "Arrow keys move focus across tabs").await?;

    driver
        .action_chain()
        .key_down(Key::Shift)
        .send_keys(Key::Tab)
        .key_up(Key::Shift)
        .perform()
        .await?;

    assert_active_tab_contains(driver, "Keyboard").await?;

    dispatch_pointer_tap(driver, "Overview", "touch").await?;

    assert_selected_tab_contains(driver, "Overview").await?;

    dispatch_pointer_tap(driver, "Keyboard", "pen").await?;

    assert_selected_tab_contains(driver, "Keyboard").await?;

    drag_tab_onto(driver, "Overview", "Keyboard").await?;

    assert_visible_tab_order(driver, &["Keyboard", "Overview", "Closable", "Disabled"]).await?;

    wait_for_active_tab_contains(driver, "Overview").await?;

    driver
        .action_chain()
        .key_down(Key::Control)
        .send_keys(Key::Right)
        .key_up(Key::Control)
        .perform()
        .await?;

    assert_visible_tab_order(driver, &["Keyboard", "Closable", "Overview", "Disabled"]).await?;

    wait_for_active_tab_contains(driver, "Overview").await?;

    driver
        .action_chain()
        .key_down(Key::Control)
        .send_keys(Key::Left)
        .key_up(Key::Control)
        .perform()
        .await?;

    assert_visible_tab_order(driver, &["Keyboard", "Overview", "Closable", "Disabled"]).await?;

    wait_for_active_tab_contains(driver, "Overview").await?;

    driver
        .action_chain()
        .send_keys(Key::Right)
        .perform()
        .await?;

    assert_active_tab_contains(driver, "Closable").await?;

    driver
        .action_chain()
        .send_keys(Key::Delete)
        .perform()
        .await?;

    wait_for_tab_absent(driver, "Closable").await?;

    wait_for_active_tab_contains(driver, "Overview").await?;

    Ok(())
}

async fn find_tab(driver: &WebDriver, label: &str) -> Result<WebElement, Error> {
    wait_for_visible_tab(driver, label).await
}

async fn wait_for_tab_absent(driver: &WebDriver, label: &str) -> Result<(), Error> {
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        match find_tab(driver, label).await {
            Ok(_) if Instant::now() >= deadline => {
                return Err(Error::Assertion(format!("tab {label:?} is still present")));
            }

            Ok(_) => time::sleep(Duration::from_millis(50)).await,

            Err(Error::WebDriver(_)) | Err(Error::Timeout(_)) => return Ok(()),

            Err(error) => return Err(error),
        }
    }
}

async fn wait_for_visible_tab(driver: &WebDriver, label: &str) -> Result<WebElement, Error> {
    let deadline = Instant::now() + Duration::from_secs(15);

    let mut last_error = None;

    while Instant::now() < deadline {
        match visible_tab(driver, label).await {
            Ok(Some(element)) => return Ok(element),

            Ok(None) => time::sleep(Duration::from_millis(100)).await,

            Err(error) => {
                last_error = Some(error.to_string());

                time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    Err(Error::Timeout(format!(
        "timed out waiting for visible tab {label:?}; last WebDriver error: {}",
        last_error.unwrap_or_else(|| "none".to_string())
    )))
}

async fn visible_tab(driver: &WebDriver, label: &str) -> Result<Option<WebElement>, Error> {
    let tabs = driver.find_all(By::Css("[role='tab']")).await?;

    for tab in tabs.into_iter().rev() {
        let text = tab.text().await?;

        if text.contains(label) && tab.is_displayed().await? {
            return Ok(Some(tab));
        }
    }

    Ok(None)
}

async fn visible_tab_order(driver: &WebDriver) -> Result<Vec<String>, Error> {
    let tabs = driver.find_all(By::Css("[role='tab']")).await?;

    let mut labels = Vec::new();

    for tab in tabs {
        if tab.is_displayed().await? {
            labels.push(tab.text().await?);
        }
    }

    Ok(labels
        .into_iter()
        .filter(|label| ["Overview", "Keyboard", "Closable", "Disabled"].contains(&label.as_str()))
        .collect())
}

async fn assert_visible_tab_order(driver: &WebDriver, expected: &[&str]) -> Result<(), Error> {
    let deadline = Instant::now() + Duration::from_secs(5);

    let expected_vec = expected
        .iter()
        .map(|item| (*item).to_string())
        .collect::<Vec<_>>();

    let mut last_order = Vec::new();

    while Instant::now() < deadline {
        let order = visible_tab_order(driver).await?;

        if order == expected_vec {
            return Ok(());
        }

        last_order = order;

        time::sleep(Duration::from_millis(50)).await;
    }

    Err(Error::Assertion(format!(
        "expected visible tab order {expected_vec:?}, got {last_order:?}"
    )))
}

async fn drag_tab_onto(driver: &WebDriver, source: &str, target: &str) -> Result<(), Error> {
    let source_tab = find_tab(driver, source).await?;
    let target_tab = find_tab(driver, target).await?;

    driver
        .execute(
            r#"
            const source = arguments[0];
            const target = arguments[1];

            const sourceRect = source.getBoundingClientRect();
            const targetRect = target.getBoundingClientRect();

            const dataTransfer = new DataTransfer();

            const eventInit = (rect, type) => ({
                bubbles: true,
                cancelable: true,
                composed: true,
                clientX: rect.left + rect.width / 2,
                clientY: rect.top + rect.height / 2,
                screenX: window.screenX + rect.left + rect.width / 2,
                screenY: window.screenY + rect.top + rect.height / 2,
                dataTransfer,
                button: 0,
                buttons: type === "dragend" ? 0 : 1,
            });

            for (const [el, type, rect] of [
                [source, "dragstart", sourceRect],
                [target, "dragenter", targetRect],
                [target, "dragover", targetRect],
                [target, "drop", targetRect],
                [source, "dragend", targetRect],
            ]) {
                el.dispatchEvent(new DragEvent(type, eventInit(rect, type)));
            }
            "#,
            vec![source_tab.to_json()?, target_tab.to_json()?],
        )
        .await?;

    Ok(())
}

async fn dispatch_pointer_tap(
    driver: &WebDriver,
    label: &str,
    pointer_type: &str,
) -> Result<(), Error> {
    let tab = find_tab(driver, label).await?;

    let args = vec![tab.to_json()?, json!(pointer_type)];

    driver
        .execute(
            r#"
            const el = arguments[0];

            const pointerType = arguments[1];

            for (const type of ["pointerdown", "pointerup", "click"]) {
                const init = {
                    bubbles: true,
                    cancelable: true,
                    composed: true,
                    pointerType,
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
            args,
        )
        .await?;

    Ok(())
}

async fn assert_active_tab_contains(driver: &WebDriver, expected: &str) -> Result<(), Error> {
    assert_active_role(driver, "tab").await?;
    assert_active_text_contains(driver, expected).await
}

async fn wait_for_active_tab_contains(driver: &WebDriver, expected: &str) -> Result<(), Error> {
    let deadline = Instant::now() + Duration::from_secs(5);

    let mut last_error = None;

    while Instant::now() < deadline {
        if let Err(error) = assert_active_tab_contains(driver, expected).await {
            last_error = Some(error.to_string());

            time::sleep(Duration::from_millis(50)).await;
        } else {
            return Ok(());
        }
    }

    Err(Error::Timeout(format!(
        "timed out waiting for active tab {expected:?}; last error: {}; visible tabs: {}",
        last_error.unwrap_or_else(|| "none".to_string()),
        visible_tabs_debug(driver)
            .await
            .unwrap_or_else(|error| error.to_string())
    )))
}

async fn visible_tabs_debug(driver: &WebDriver) -> Result<String, Error> {
    let tabs = driver.find_all(By::Css("[role='tab']")).await?;

    let mut rows = Vec::new();

    for tab in tabs {
        if !tab.is_displayed().await? {
            continue;
        }

        rows.push(format!(
            "{{text={:?}, id={:?}, selected={:?}, tabindex={:?}}}",
            tab.text().await?,
            tab.attr("id").await?,
            tab.attr("aria-selected").await?,
            tab.attr("tabindex").await?,
        ));
    }

    Ok(rows.join(", "))
}

async fn assert_visible_tab_roledescription(driver: &WebDriver, label: &str) -> Result<(), Error> {
    let tab = find_tab(driver, label).await?;

    let roledescription = tab.attr("aria-roledescription").await?;

    if roledescription.as_deref() == Some("draggable tab") {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "visible reorderable tab {label:?} must expose aria-roledescription='draggable tab', got {roledescription:?}"
        )))
    }
}

async fn assert_selected_tab_contains(driver: &WebDriver, expected: &str) -> Result<(), Error> {
    let tab = find_tab(driver, expected).await?;

    let selected = tab.attr("aria-selected").await?;

    if selected.as_deref() == Some("true") {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "expected tab {expected:?} to have aria-selected=true, got {selected:?}"
        )))
    }
}
