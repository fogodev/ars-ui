//! Browser E2E harness for the `VisuallyHidden` component.

use thirtyfour::prelude::*;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    utility::{assert_attr, assert_bool_attr, open_utility_panel},
};

/// Runs the `VisuallyHidden` browser assertions inside the Utility fixture panel.
///
/// # Errors
///
/// Returns an error when `WebDriver` cannot find the fixture nodes or an
/// assertion fails.
pub async fn run_visually_hidden_flow(
    driver: &WebDriver,
    url: &str,
    adapter: Adapter,
) -> Result<(), Error> {
    open_utility_panel(driver, url).await?;

    let prefix = id_prefix(adapter);

    let hidden = hidden_element_by_id(driver, &format!("{prefix}-visually-hidden-label")).await?;

    assert_attr(&hidden, "data-ars-scope", "visually-hidden").await?;
    assert_attr(&hidden, "data-ars-part", "root").await?;

    let class = hidden.attr("class").await?;

    if class_token_present(class.as_deref(), "ars-visually-hidden") {
        // Expected default hidden class.
    } else {
        return Err(Error::Assertion(format!(
            "default VisuallyHidden root must include ars-visually-hidden, got {class:?}"
        )));
    }

    let focusable = hidden_element_by_id(driver, &format!("{prefix}-focusable-skip")).await?;

    assert_attr(&focusable, "data-ars-scope", "visually-hidden").await?;
    assert_attr(&focusable, "data-ars-part", "root").await?;
    assert_bool_attr(&focusable, "data-ars-visually-hidden-focusable").await?;

    let as_child =
        hidden_element_by_id(driver, &format!("{prefix}-visually-hidden-as-child")).await?;

    let tag = as_child.tag_name().await?;

    assert_expected_tag(
        &tag,
        "span",
        "VisuallyHiddenAsChild root must stay consumer-owned",
    )?;

    assert_attr(&as_child, "data-ars-scope", "visually-hidden").await?;
    assert_attr(&as_child, "data-ars-part", "root").await?;

    Ok(())
}

async fn hidden_element_by_id(driver: &WebDriver, id: &str) -> Result<WebElement, Error> {
    driver.find(By::Id(id)).await.map_err(Error::from)
}

fn class_token_present(class: Option<&str>, token: &str) -> bool {
    class.is_some_and(|value| value.split_whitespace().any(|candidate| candidate == token))
}

fn assert_expected_tag(actual: &str, expected: &str, context: &str) -> Result<(), Error> {
    if actual == expected {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "{context} {expected}, got {actual:?}"
        )))
    }
}

const fn id_prefix(adapter: Adapter) -> &'static str {
    match adapter {
        Adapter::Leptos => "leptos-fixture",
        Adapter::Dioxus => "dioxus-fixture",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_token_present_matches_whole_tokens_only() {
        assert!(class_token_present(
            Some("foo ars-visually-hidden bar"),
            "ars-visually-hidden"
        ));
        assert!(!class_token_present(
            Some("foo ars-visually-hidden-extra bar"),
            "ars-visually-hidden"
        ));
        assert!(!class_token_present(None, "ars-visually-hidden"));
    }

    #[test]
    fn assert_expected_tag_reports_wrong_consumer_root() {
        assert!(assert_expected_tag("span", "span", "component root").is_ok());

        let error = assert_expected_tag("div", "span", "component root")
            .expect_err("wrong tag should fail");

        assert!(error.to_string().contains("component root span"));
        assert!(error.to_string().contains("\"div\""));
    }
}
