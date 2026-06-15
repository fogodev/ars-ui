//! Browser E2E harness for the Checkbox component.

use serde_json::Value;
use thirtyfour::prelude::*;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    axe::run_axe,
    input::{
        assert_attr, assert_attr_present, dispatch_pointer_sequence, element_by_id,
        open_input_panel,
    },
};

/// Runtime options for the Checkbox E2E harness.
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

/// Runs the Checkbox browser E2E harness.
///
/// # Errors
///
/// Returns an error when the example server, `ChromeDriver`, `WebDriver` session,
/// or browser assertions fail.
pub async fn run(options: Options) -> Result<(), Error> {
    let adapter = options.adapter;

    let session = crate::fixtures::start_fixture_session(crate::fixtures::FixtureOptions {
        adapter,
        port: options.port,
        webdriver_url: options.webdriver_url,
        no_server: options.no_server,
        headless: options.headless,
    })
    .await?;

    let run = run_checkbox_flow(&session.driver, &session.url, adapter).await;
    let quit = session.quit().await;

    run?;
    quit?;

    Ok(())
}

pub(super) async fn run_checkbox_flow(
    driver: &WebDriver,
    url: &str,
    adapter: Adapter,
) -> Result<(), Error> {
    open_input_panel(driver, url).await?;

    driver
        .execute(
            "document.documentElement.lang ||= 'en';",
            Vec::<Value>::new(),
        )
        .await?;

    run_axe(driver).await?;

    let prefix = id_prefix(adapter);

    assert_anatomy(driver, prefix).await?;
    assert_initial_states(driver, prefix).await?;
    assert_computed_visual_states(driver, prefix).await?;
    assert_pointer_toggle(driver, prefix).await?;
    assert_keyboard_toggle(driver, prefix).await?;
    assert_blocked_states(driver, prefix).await?;
    assert_form_value_and_reset(driver, prefix).await?;

    run_axe(driver).await?;

    Ok(())
}

async fn assert_anatomy(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    let root = element_by_id(driver, &format!("{prefix}-unchecked")).await?;

    assert_attr(&root, "data-ars-scope", "checkbox").await?;
    assert_attr(&root, "data-ars-part", "root").await?;

    let checkbox_control = control(&root).await?;

    assert_attr(&checkbox_control, "role", "checkbox").await?;
    assert_attr(&checkbox_control, "aria-checked", "false").await?;

    root.find(By::Css("[data-ars-part='label']")).await?;
    root.find(By::Css("[data-ars-part='indicator']")).await?;
    root.find(By::Css("[data-ars-part='hidden-input']")).await?;

    let invalid = element_by_id(driver, &format!("{prefix}-invalid")).await?;
    let invalid_control = control(&invalid).await?;

    assert_attr(&invalid_control, "aria-invalid", "true").await?;
    assert_attr_present(&invalid_control, "aria-describedby").await?;
    assert_attr_present(&invalid_control, "aria-errormessage").await?;

    invalid
        .find(By::Css("[data-ars-part='description']"))
        .await?;

    invalid
        .find(By::Css("[data-ars-part='error-message']"))
        .await?;

    Ok(())
}

async fn assert_initial_states(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    for (suffix, state, checked_attr) in [
        ("unchecked", "false", false),
        ("checked", "true", true),
        ("indeterminate", "mixed", false),
        ("controlled", "mixed", false),
    ] {
        let root = element_by_id(driver, &format!("{prefix}-{suffix}")).await?;
        let control = control(&root).await?;
        let input = hidden_input(&root).await?;

        assert_attr(&control, "aria-checked", state).await?;

        if input_checked(driver, &input).await? != checked_attr {
            return Err(Error::Assertion(format!(
                "{suffix} hidden input checked property must be {checked_attr}"
            )));
        }
    }

    Ok(())
}

async fn assert_pointer_toggle(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    let root = element_by_id(driver, &format!("{prefix}-unchecked")).await?;
    let control = control(&root).await?;

    dispatch_pointer_sequence(driver, &control).await?;

    assert_attr(&control, "aria-checked", "true").await
}

async fn assert_keyboard_toggle(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    let root = element_by_id(driver, &format!("{prefix}-controlled")).await?;
    let control = control(&root).await?;

    dispatch_pointer_sequence(driver, &control).await?;
    driver
        .execute(
            r#"
            arguments[0].dispatchEvent(new KeyboardEvent("keydown", {
                bubbles: true,
                cancelable: true,
                composed: true,
                key: " ",
                code: "Space"
            }));
            "#,
            vec![control.to_json()?],
        )
        .await?;

    assert_attr(&control, "aria-checked", "false").await
}

async fn assert_blocked_states(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    for suffix in ["disabled", "readonly"] {
        let root = element_by_id(driver, &format!("{prefix}-{suffix}")).await?;
        let control = control(&root).await?;
        let before = control.attr("aria-checked").await?;

        dispatch_pointer_sequence(driver, &control).await?;

        if control.attr("aria-checked").await? != before {
            return Err(Error::Assertion(format!(
                "{suffix} Checkbox must not change through pointer intent"
            )));
        }
    }

    Ok(())
}

async fn assert_computed_visual_states(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    let unchecked = computed_control_style(driver, prefix, "unchecked").await?;

    for suffix in ["checked", "indeterminate", "disabled", "invalid"] {
        let style = computed_control_style(driver, prefix, suffix).await?;

        if style.width <= 0.0 || style.height <= 0.0 {
            return Err(Error::Assertion(format!(
                "{suffix} checkbox control must have positive dimensions, got {}x{}",
                style.width, style.height
            )));
        }

        match suffix {
            "checked" | "indeterminate" => {
                if style.background_color == unchecked.background_color
                    && style.border_color == unchecked.border_color
                {
                    return Err(Error::Assertion(format!(
                        "{suffix} checkbox control must visually differ from unchecked"
                    )));
                }
            }
            "disabled" if style.opacity >= unchecked.opacity => {
                return Err(Error::Assertion(
                    "disabled checkbox control opacity must be lower than unchecked".into(),
                ));
            }
            "invalid" if style.border_color == unchecked.border_color => {
                return Err(Error::Assertion(
                    "invalid checkbox control border color must differ from unchecked".into(),
                ));
            }
            _ => {}
        }
    }

    Ok(())
}

#[derive(Debug)]
struct ComputedControlStyle {
    border_color: String,
    background_color: String,
    opacity: f64,
    width: f64,
    height: f64,
}

async fn computed_control_style(
    driver: &WebDriver,
    prefix: &str,
    suffix: &str,
) -> Result<ComputedControlStyle, Error> {
    let root = element_by_id(driver, &format!("{prefix}-{suffix}")).await?;
    let control = control(&root).await?;
    let style = driver
        .execute(
            r#"
            const style = getComputedStyle(arguments[0]);
            return {
                borderColor: style.borderColor,
                backgroundColor: style.backgroundColor,
                opacity: style.opacity,
                width: style.width,
                height: style.height
            };
            "#,
            vec![control.to_json()?],
        )
        .await?;

    let style = style.json();

    Ok(ComputedControlStyle {
        border_color: string_field(style, "borderColor", suffix)?,
        background_color: string_field(style, "backgroundColor", suffix)?,
        opacity: number_string_field(style, "opacity", suffix)?,
        width: px_field(style, "width", suffix)?,
        height: px_field(style, "height", suffix)?,
    })
}

fn string_field(style: &Value, key: &str, suffix: &str) -> Result<String, Error> {
    style
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::Assertion(format!("{suffix} checkbox missing computed {key}")))
}

fn number_string_field(style: &Value, key: &str, suffix: &str) -> Result<f64, Error> {
    string_field(style, key, suffix)?
        .parse::<f64>()
        .map_err(|error| {
            Error::Assertion(format!(
                "{suffix} checkbox computed {key} must be numeric: {error}"
            ))
        })
}

fn px_field(style: &Value, key: &str, suffix: &str) -> Result<f64, Error> {
    let raw = string_field(style, key, suffix)?;
    let Some(value) = raw.strip_suffix("px") else {
        return Err(Error::Assertion(format!(
            "{suffix} checkbox computed {key} must use px, got {raw}"
        )));
    };

    value.parse::<f64>().map_err(|error| {
        Error::Assertion(format!(
            "{suffix} checkbox computed {key} must be numeric px: {error}"
        ))
    })
}

async fn assert_form_value_and_reset(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    let root = element_by_id(driver, &format!("{prefix}-form-value")).await?;
    let control = control(&root).await?;
    let input = hidden_input(&root).await?;

    assert_attr(&input, "name", "notifications").await?;
    assert_attr(&input, "value", "email").await?;
    assert_attr(&control, "aria-checked", "true").await?;

    dispatch_pointer_sequence(driver, &control).await?;
    assert_attr(&control, "aria-checked", "false").await?;

    let submit = element_by_id(driver, &format!("{prefix}-submit")).await?;

    submit.click().await?;

    let status = element_by_id(driver, &format!("{prefix}-form-status")).await?;
    let status_text = status.text().await?;

    if !status_text.contains("notifications=none") {
        return Err(Error::Assertion(format!(
            "form submit status must reflect unchecked serialized value, got {status_text:?}"
        )));
    }

    let reset = element_by_id(driver, &format!("{prefix}-reset")).await?;

    reset.click().await?;

    assert_attr(&control, "aria-checked", "true").await?;

    let status_text = status.text().await?;

    if !status_text.contains("reset notifications=email") {
        return Err(Error::Assertion(format!(
            "form reset status must reflect restored default value, got {status_text:?}"
        )));
    }

    Ok(())
}

async fn control(root: &WebElement) -> Result<WebElement, Error> {
    Ok(root.find(By::Css("[data-ars-part='control']")).await?)
}

async fn hidden_input(root: &WebElement) -> Result<WebElement, Error> {
    Ok(root.find(By::Css("[data-ars-part='hidden-input']")).await?)
}

async fn input_checked(driver: &WebDriver, input: &WebElement) -> Result<bool, Error> {
    let checked = driver
        .execute(
            "return Boolean(arguments[0].checked);",
            vec![input.to_json()?],
        )
        .await?;

    checked.json().as_bool().ok_or_else(|| {
        Error::Assertion("hidden input checked property must be boolean".to_string())
    })
}

const fn id_prefix(adapter: Adapter) -> &'static str {
    match adapter {
        Adapter::Leptos => "leptos-fixture-checkbox",
        Adapter::Dioxus => "dioxus-fixture-checkbox",
    }
}
