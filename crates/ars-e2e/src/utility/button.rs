//! Browser E2E harness for the Button component.

use std::time::{Duration, Instant};

use serde_json::Value;
use thirtyfour::prelude::*;
use tokio::time;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    assertions::{
        assert_accessibility_tree_has_role_and_name, assert_focus_indicator_visible,
        assert_forced_colors_focus_indicator_visible,
    },
    axe::run_axe,
    fixtures::{FixtureOptions, start_fixture_session},
    utility::{
        active_id, assert_attr, assert_attr_present, assert_bool_attr, dispatch_pointer_sequence,
        element_by_id, open_utility_panel,
    },
};

/// Runtime options for the Button E2E harness.
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

/// Runs the Button browser E2E harness.
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

    let run = run_button_flow(&session.driver, &session.url, adapter).await;

    let quit = session.quit().await;

    run?;
    quit?;

    Ok(())
}

pub(super) async fn run_button_flow(
    driver: &WebDriver,
    url: &str,
    adapter: Adapter,
) -> Result<(), Error> {
    open_utility_panel(driver, url).await?;

    driver
        .execute(
            "document.documentElement.lang ||= 'en';",
            Vec::<Value>::new(),
        )
        .await?;

    run_axe(driver).await?;

    let prefix = id_prefix(adapter);

    assert_accessibility_tree_has_role_and_name(driver, "button", "Default").await?;
    assert_accessibility_tree_has_role_and_name(driver, "button", "Primary").await?;

    for suffix in [
        "default",
        "primary",
        "secondary",
        "destructive",
        "outline",
        "ghost",
        "link",
        "sm",
        "md",
        "lg",
        "icon",
        "submit",
        "reset",
    ] {
        assert_button_root(driver, &format!("{prefix}-{suffix}")).await?;
    }

    assert_variant_attrs(driver, prefix).await?;
    assert_size_attrs(driver, prefix).await?;
    assert_disabled_state(driver, prefix).await?;
    assert_loading_state(driver, prefix).await?;
    assert_form_attrs(driver, prefix).await?;
    assert_as_child_links(driver, prefix).await?;
    assert_keyboard_focus_sequence(driver, prefix).await?;

    Ok(())
}

async fn assert_button_root(driver: &WebDriver, id: &str) -> Result<(), Error> {
    let element = element_by_id(driver, id).await?;

    assert_attr(&element, "data-ars-scope", "button").await?;
    assert_attr(&element, "data-ars-part", "root").await?;

    Ok(())
}

async fn assert_variant_attrs(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    for (suffix, expected) in [
        ("default", "default"),
        ("primary", "primary"),
        ("secondary", "secondary"),
        ("destructive", "destructive"),
        ("outline", "outline"),
        ("ghost", "ghost"),
        ("link", "link"),
    ] {
        let button = element_by_id(driver, &format!("{prefix}-{suffix}")).await?;

        assert_attr(&button, "data-ars-variant", expected).await?;
        assert_enabled(&button, &format!("{prefix}-{suffix}")).await?;
    }

    Ok(())
}

async fn assert_size_attrs(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    for (suffix, expected) in [("sm", "sm"), ("md", "md"), ("lg", "lg"), ("icon", "icon")] {
        let button = element_by_id(driver, &format!("{prefix}-{suffix}")).await?;

        assert_attr(&button, "data-ars-size", expected).await?;
    }

    let icon = element_by_id(driver, &format!("{prefix}-icon")).await?;

    let rect = element_rect(&icon).await?;

    if (rect.width - rect.height).abs() <= 6.0 {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "icon button must render as a square hit target, got {rect:?}"
        )))
    }
}

async fn assert_disabled_state(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    let disabled = element_by_id(driver, &format!("{prefix}-disabled")).await?;

    assert_bool_attr(&disabled, "data-ars-disabled").await?;
    assert_attr_present(&disabled, "disabled").await?;

    if disabled.is_enabled().await? {
        return Err(Error::Assertion(
            "disabled Button must not be enabled in the browser".to_string(),
        ));
    }

    disabled.focus().await?;

    if active_id(driver).await?.as_deref() == Some(&format!("{prefix}-disabled")) {
        return Err(Error::Assertion(
            "disabled Button must not become the active element".to_string(),
        ));
    }

    Ok(())
}

async fn assert_loading_state(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    let loading = element_by_id(driver, &format!("{prefix}-loading")).await?;

    assert_attr(&loading, "aria-busy", "true").await?;
    assert_bool_attr(&loading, "data-ars-loading").await?;

    loading
        .find(By::Css("[data-ars-part='loading-indicator']"))
        .await?;

    Ok(())
}

async fn assert_form_attrs(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    let submit = element_by_id(driver, &format!("{prefix}-submit")).await?;

    assert_attr(&submit, "type", "submit").await?;
    assert_attr(&submit, "name", "intent").await?;
    assert_attr(&submit, "value", "save").await?;
    assert_attr(&submit, "formaction", "/submit").await?;
    assert_attr(&submit, "formmethod", "post").await?;
    assert_attr(&submit, "formenctype", "application/x-www-form-urlencoded").await?;
    assert_attr(&submit, "formtarget", "_self").await?;
    assert_attr_present(&submit, "formnovalidate").await?;

    let reset = element_by_id(driver, &format!("{prefix}-reset")).await?;

    assert_attr(&reset, "type", "reset").await?;

    Ok(())
}

async fn assert_as_child_links(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    let docs = element_by_id(driver, &format!("{prefix}-as-child-docs")).await?;
    let primary = element_by_id(driver, &format!("{prefix}-as-child-primary")).await?;

    assert_link_button(&docs, "link").await?;
    assert_link_button(&primary, "primary").await?;

    docs.click().await?;

    let current_url = driver.current_url().await?;

    if current_url.as_str().contains("#variants") {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "ButtonAsChild link click must preserve anchor navigation, got {current_url}"
        )))
    }
}

async fn assert_link_button(element: &WebElement, variant: &str) -> Result<(), Error> {
    let tag = element.tag_name().await?;

    if tag != "a" {
        return Err(Error::Assertion(format!(
            "ButtonAsChild root must remain an anchor, got tag {tag:?}"
        )));
    }

    assert_attr(element, "data-ars-scope", "button").await?;
    assert_attr(element, "data-ars-part", "root").await?;
    assert_attr(element, "data-ars-variant", variant).await?;
    assert_attr(element, "href", "#variants").await?;

    Ok(())
}

async fn assert_keyboard_focus_sequence(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    focus_by_tabbing_to(driver, &format!("{prefix}-primary")).await?;

    assert_focus_indicator_visible(driver).await?;
    assert_forced_colors_focus_indicator_visible(driver).await?;

    driver
        .action_chain()
        .send_keys(Key::Enter)
        .perform()
        .await?;

    driver
        .action_chain()
        .send_keys(Key::Space)
        .perform()
        .await?;

    assert_eq_active_id(driver, &format!("{prefix}-primary")).await?;

    let seen = collect_forward_tab_ids(driver, 28).await?;

    if seen.iter().any(|id| id == &format!("{prefix}-disabled")) {
        return Err(Error::Assertion(format!(
            "disabled Button must be skipped by keyboard Tab navigation; saw {seen:?}"
        )));
    }

    if seen.iter().any(|id| id == &format!("{prefix}-loading")) {
        focus_by_tabbing_to(driver, &format!("{prefix}-loading")).await?;

        assert_focus_indicator_visible(driver).await?;
    } else {
        return Err(Error::Assertion(format!(
            "loading Button should stay keyboard focusable while exposing busy state; saw {seen:?}"
        )));
    }

    for expected in [
        format!("{prefix}-secondary"),
        format!("{prefix}-destructive"),
        format!("{prefix}-outline"),
        format!("{prefix}-ghost"),
        format!("{prefix}-link"),
        format!("{prefix}-submit"),
        format!("{prefix}-reset"),
    ] {
        if !seen.iter().any(|id| id == &expected) {
            return Err(Error::Assertion(format!(
                "keyboard Tab navigation should reach enabled Button {expected:?}; saw {seen:?}"
            )));
        }
    }

    let primary = element_by_id(driver, &format!("{prefix}-primary")).await?;

    dispatch_pointer_sequence(driver, &primary).await?;

    Ok(())
}

async fn focus_by_tabbing_to(driver: &WebDriver, id: &str) -> Result<(), Error> {
    driver
        .execute(
            "document.body.setAttribute('tabindex', '-1'); document.body.focus();",
            Vec::<Value>::new(),
        )
        .await?;

    let deadline = Instant::now() + Duration::from_secs(5);

    while Instant::now() < deadline {
        driver.action_chain().send_keys(Key::Tab).perform().await?;

        if active_id(driver).await?.as_deref() == Some(id) {
            return Ok(());
        }

        time::sleep(Duration::from_millis(20)).await;
    }

    Err(Error::Timeout(format!("timed out tabbing to #{id}")))
}

async fn collect_forward_tab_ids(driver: &WebDriver, count: usize) -> Result<Vec<String>, Error> {
    let mut ids = Vec::new();

    for _ in 0..count {
        driver.action_chain().send_keys(Key::Tab).perform().await?;

        if let Some(id) = active_id(driver).await?
            && !id.is_empty()
        {
            ids.push(id);
        }
    }

    Ok(ids)
}

async fn assert_eq_active_id(driver: &WebDriver, expected: &str) -> Result<(), Error> {
    let actual = active_id(driver).await?;

    if actual.as_deref() == Some(expected) {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "expected active element #{expected}, got {actual:?}"
        )))
    }
}

async fn assert_enabled(element: &WebElement, id: &str) -> Result<(), Error> {
    if element.is_enabled().await? {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "enabled Button #{id} must be enabled in the browser"
        )))
    }
}

async fn element_rect(element: &WebElement) -> Result<Rect, Error> {
    let result = element
        .handle()
        .execute(
            r#"
            const rect = arguments[0].getBoundingClientRect();

            return { width: rect.width, height: rect.height };
            "#,
            vec![element.to_json()?],
        )
        .await?;
    let value = result.json();

    Ok(Rect {
        width: number_prop(value, "width"),
        height: number_prop(value, "height"),
    })
}

#[derive(Debug)]
struct Rect {
    width: f64,
    height: f64,
}

fn number_prop(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or_default()
}

const fn id_prefix(adapter: Adapter) -> &'static str {
    match adapter {
        Adapter::Leptos => "leptos-fixture",
        Adapter::Dioxus => "dioxus-fixture",
    }
}
