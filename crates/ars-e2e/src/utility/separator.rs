//! Browser E2E harness for the `Separator` component.

use thirtyfour::prelude::*;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    utility::{assert_attr, element_by_id, open_utility_panel},
};

/// Runs the `Separator` browser assertions inside the Utility fixture panel.
///
/// # Errors
///
/// Returns an error when `WebDriver` cannot find the fixture nodes or an
/// assertion fails.
pub async fn run_separator_flow(
    driver: &WebDriver,
    url: &str,
    adapter: Adapter,
) -> Result<(), Error> {
    open_utility_panel(driver, url).await?;

    let prefix = id_prefix(adapter);

    let horizontal = element_by_id(driver, &format!("{prefix}-separator-horizontal")).await?;

    assert_attr(&horizontal, "data-ars-scope", "separator").await?;
    assert_attr(&horizontal, "data-ars-part", "root").await?;
    assert_attr(&horizontal, "role", "separator").await?;
    assert_attr(&horizontal, "aria-orientation", "horizontal").await?;
    assert_attr(&horizontal, "data-ars-orientation", "horizontal").await?;

    let vertical = element_by_id(driver, &format!("{prefix}-separator-vertical")).await?;

    assert_attr(&vertical, "role", "separator").await?;
    assert_attr(&vertical, "aria-orientation", "vertical").await?;
    assert_attr(&vertical, "data-ars-orientation", "vertical").await?;

    let as_child = element_by_id(driver, &format!("{prefix}-separator-as-child")).await?;

    let tag = as_child.tag_name().await?;

    assert_expected_tag(
        &tag,
        "div",
        "SeparatorAsChild root must stay consumer-owned",
    )?;

    assert_attr(&as_child, "data-ars-scope", "separator").await?;
    assert_attr(&as_child, "data-ars-part", "root").await?;
    assert_attr(&as_child, "role", "separator").await?;
    assert_attr(&as_child, "aria-orientation", "vertical").await?;
    assert_attr(&as_child, "data-ars-orientation", "vertical").await?;

    let decorative = element_by_id(driver, &format!("{prefix}-separator-decorative")).await?;

    assert_attr(&decorative, "data-ars-scope", "separator").await?;
    assert_attr(&decorative, "data-ars-part", "root").await?;
    assert_attr(&decorative, "role", "none").await?;

    if decorative.attr("aria-orientation").await?.is_some() {
        return Err(Error::Assertion(
            "decorative Separator must omit aria-orientation".to_string(),
        ));
    }

    Ok(())
}

const fn id_prefix(adapter: Adapter) -> &'static str {
    match adapter {
        Adapter::Leptos => "leptos-fixture",
        Adapter::Dioxus => "dioxus-fixture",
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_expected_tag_reports_wrong_consumer_root() {
        assert!(assert_expected_tag("div", "div", "component root").is_ok());

        let error =
            assert_expected_tag("hr", "div", "component root").expect_err("wrong tag should fail");

        assert!(error.to_string().contains("component root div"));
        assert!(error.to_string().contains("\"hr\""));
    }
}
