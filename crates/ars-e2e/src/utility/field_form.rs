//! Browser E2E harness for Field, Fieldset, and Form utility adapters.

use std::time::{Duration, Instant};

use thirtyfour::prelude::*;
use tokio::time;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    axe::run_axe,
    utility::{assert_attr, assert_bool_attr, element_by_id, open_utility_panel},
};

/// Runs Field, Fieldset, and Form browser assertions inside the Utility fixture panel.
///
/// # Errors
///
/// Returns an error when `WebDriver` cannot find fixture nodes or an assertion fails.
pub async fn run_field_form_flow(
    driver: &WebDriver,
    url: &str,
    adapter: Adapter,
) -> Result<(), Error> {
    open_utility_panel(driver, url).await?;

    let prefix = id_prefix(adapter);

    let form = element_by_id(driver, &format!("{prefix}-account-form")).await?;

    assert_attr(&form, "data-ars-scope", "form").await?;
    assert_attr(&form, "data-ars-part", "root").await?;
    assert_attr(&form, "data-ars-state", "idle").await?;
    assert_attr(&form, "action", "/account").await?;
    assert_bool_attr(&form, "novalidate").await?;

    let fieldset = element_by_id(driver, &format!("{prefix}-account-fieldset")).await?;

    assert_attr(&fieldset, "data-ars-scope", "fieldset").await?;
    assert_attr(&fieldset, "data-ars-part", "root").await?;
    assert_bool_attr(&fieldset, "disabled").await?;

    assert_invalid_email_field(
        driver,
        prefix,
        "required",
        "email-required",
        "Email is required.",
        true,
        None,
    )
    .await?;

    assert_invalid_email_field(
        driver,
        prefix,
        "missing-at",
        "email-missing-at",
        "Include an @ in the email address.",
        false,
        Some("admin"),
    )
    .await?;

    assert_invalid_email_field(
        driver,
        prefix,
        "incomplete-domain",
        "email-incomplete-domain",
        "Enter a domain after @.",
        false,
        Some("admin@"),
    )
    .await?;

    assert_valid_email_field(driver, prefix).await?;

    let status = form
        .find(By::Css("[data-ars-part='status-region']"))
        .await?;

    assert_attr(&status, "role", "status").await?;
    assert_attr(&status, "aria-live", "polite").await?;
    assert_attr(&status, "aria-atomic", "true").await?;
    assert_text(&status, "Ready", "form status").await?;
    assert_status_is_not_field_error(&status).await?;

    driver
        .execute(
            "document.documentElement.lang = 'en';",
            Vec::<serde_json::Value>::new(),
        )
        .await?;

    run_axe(driver).await?;

    let locale_button = element_by_id(driver, &format!("{prefix}-locale-pt")).await?;

    locale_button.click().await?;

    driver
        .execute(
            "document.documentElement.lang = 'pt-BR';",
            Vec::<serde_json::Value>::new(),
        )
        .await?;

    let heading = element_by_id(driver, "field-form").await?;

    wait_for_text(
        &heading,
        "Primitivos de campo e formulário",
        "pt-BR Field/Form heading",
    )
    .await?;

    let required_error = element_by_id(
        driver,
        &format!("{prefix}-email-required-field-error-message"),
    )
    .await?;

    wait_for_text(
        &required_error,
        "E-mail é obrigatório.",
        "pt-BR required error",
    )
    .await?;

    let missing_at_error = element_by_id(
        driver,
        &format!("{prefix}-email-missing-at-field-error-message"),
    )
    .await?;

    wait_for_text(
        &missing_at_error,
        "Inclua um @ no endereço de e-mail.",
        "pt-BR missing-at error",
    )
    .await?;

    let status = form
        .find(By::Css("[data-ars-part='status-region']"))
        .await?;

    wait_for_text(&status, "Pronto", "pt-BR form status").await?;

    assert_status_is_not_field_error(&status).await?;

    run_axe(driver).await?;

    Ok(())
}

async fn assert_invalid_email_field(
    driver: &WebDriver,
    prefix: &str,
    slug: &str,
    name: &str,
    error_text: &str,
    required: bool,
    expected_value: Option<&str>,
) -> Result<(), Error> {
    let input = element_by_id(driver, &format!("{prefix}-email-{slug}-field-input")).await?;

    let error = element_by_id(
        driver,
        &format!("{prefix}-email-{slug}-field-error-message"),
    )
    .await?;

    assert_attr(&input, "data-ars-scope", "field").await?;
    assert_attr(&input, "data-ars-part", "input").await?;
    assert_attr(&input, "name", name).await?;

    assert_attr(
        &input,
        "aria-labelledby",
        &format!("{prefix}-email-{slug}-field-label"),
    )
    .await?;

    assert_attr(
        &input,
        "aria-describedby",
        &format!(
            "{prefix}-email-{slug}-field-description {prefix}-email-{slug}-field-error-message"
        ),
    )
    .await?;

    assert_attr(
        &input,
        "aria-errormessage",
        &format!("{prefix}-email-{slug}-field-error-message"),
    )
    .await?;

    assert_attr(&input, "aria-invalid", "true").await?;

    if required {
        assert_attr(&input, "aria-required", "true").await?;
        assert_bool_attr(&input, "required").await?;
    }

    if let Some(expected_value) = expected_value {
        let value = input.prop("value").await?;

        if value.as_deref() != Some(expected_value) {
            return Err(Error::Assertion(format!(
                "expected {slug} input value {expected_value:?}, got {value:?}"
            )));
        }
    }

    assert_text(&error, error_text, &format!("{slug} error")).await?;

    assert_error_below_input(driver, &input, &error, slug).await?;

    assert_invalid_visual_feedback(&input, slug).await
}

async fn assert_valid_email_field(driver: &WebDriver, prefix: &str) -> Result<(), Error> {
    let input = element_by_id(driver, &format!("{prefix}-email-valid-field-input")).await?;

    assert_attr(&input, "data-ars-scope", "field").await?;
    assert_attr(&input, "data-ars-part", "input").await?;
    assert_attr(&input, "name", "email-valid").await?;

    assert_attr(
        &input,
        "aria-labelledby",
        &format!("{prefix}-email-valid-field-label"),
    )
    .await?;

    assert_attr(
        &input,
        "aria-describedby",
        &format!("{prefix}-email-valid-field-description"),
    )
    .await?;

    assert_attr_absent(&input, "aria-invalid", "valid email").await?;
    assert_attr_absent(&input, "aria-errormessage", "valid email").await?;

    let value = input.prop("value").await?;

    if value.as_deref() != Some("admin@email.com") {
        return Err(Error::Assertion(format!(
            "expected valid email input value, got {value:?}"
        )));
    }

    let errors = driver
        .find_all(By::Id(format!("{prefix}-email-valid-field-error-message")))
        .await?;

    if !errors.is_empty() {
        return Err(Error::Assertion(
            "valid email field must not render an error message element".into(),
        ));
    }

    Ok(())
}

async fn assert_attr_absent(element: &WebElement, name: &str, context: &str) -> Result<(), Error> {
    let value = element.attr(name).await?;

    if value.is_none() {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "expected {name:?} to be absent for {context}, got {value:?}"
        )))
    }
}

async fn assert_text(element: &WebElement, expected: &str, context: &str) -> Result<(), Error> {
    let text = element.text().await?;

    if text == expected {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "expected {context} text {expected:?}, got {text:?}"
        )))
    }
}

async fn wait_for_text(element: &WebElement, expected: &str, context: &str) -> Result<(), Error> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut latest = String::new();

    while Instant::now() < deadline {
        latest = element.text().await?;

        if latest == expected {
            return Ok(());
        }

        time::sleep(Duration::from_millis(100)).await;
    }

    Err(Error::Assertion(format!(
        "expected {context} text {expected:?}, got {latest:?}"
    )))
}

async fn assert_status_is_not_field_error(status: &WebElement) -> Result<(), Error> {
    let text = status.text().await?;

    if text.contains("Email is required.")
        || text.contains("Include an @")
        || text.contains("Enter a domain")
        || text.contains("E-mail é obrigatório.")
        || text.contains("Inclua um @")
        || text.contains("Informe um domínio")
    {
        Err(Error::Assertion(format!(
            "form status region must not contain field validation errors, got {text:?}"
        )))
    } else {
        Ok(())
    }
}

async fn assert_error_below_input(
    driver: &WebDriver,
    input: &WebElement,
    error: &WebElement,
    slug: &str,
) -> Result<(), Error> {
    let result = driver
        .execute(
            r#"
            const input = arguments[0];
            const error = arguments[1];
            const sameField = input.closest('[data-ars-scope="field"][data-ars-part="root"]')
                === error.closest('[data-ars-scope="field"][data-ars-part="root"]');
            const follows = Boolean(input.compareDocumentPosition(error) & Node.DOCUMENT_POSITION_FOLLOWING);
            const inputRect = input.getBoundingClientRect();
            const errorRect = error.getBoundingClientRect();
            return {
                sameField,
                follows,
                errorBelowInput: errorRect.top >= inputRect.bottom - 1,
                visible: errorRect.width > 0 && errorRect.height > 0
            };
            "#,
            vec![input.to_json()?, error.to_json()?],
        )
        .await?;

    let value = result.json();

    let ok = value
        .get("sameField")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        && value
            .get("follows")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        && value
            .get("errorBelowInput")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        && value
            .get("visible")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

    if ok {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "{slug} error message must be visible below its input in the same Field root; result={value}"
        )))
    }
}

async fn assert_invalid_visual_feedback(input: &WebElement, slug: &str) -> Result<(), Error> {
    let result = input
        .handle()
        .execute(
            r#"
            const styles = getComputedStyle(arguments[0]);
            return {
                borderColor: styles.borderColor,
                boxShadow: styles.boxShadow
            };
            "#,
            vec![input.to_json()?],
        )
        .await?;

    let value = result.json();

    let border_color = value
        .get("borderColor")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");

    let box_shadow = value
        .get("boxShadow")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");

    if border_color.contains("220, 38, 38") && box_shadow.contains("220, 38, 38") {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "{slug} input must expose computed invalid visual feedback; computed={value}"
        )))
    }
}

const fn id_prefix(adapter: Adapter) -> &'static str {
    match adapter {
        Adapter::Leptos => "leptos-fixture",
        Adapter::Dioxus => "dioxus-fixture",
    }
}
