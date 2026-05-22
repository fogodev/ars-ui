//! Browser E2E harness for the `Heading` component.

use thirtyfour::prelude::*;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    utility::{assert_attr, element_by_id, open_utility_panel},
};

/// Runs the `Heading` browser assertions inside the Utility fixture panel.
///
/// The fixture nests its Heading demos under the panel's `<h2>` section
/// heading, so it starts the provider at `Level::Three` to keep the page-wide
/// hierarchy axe-clean (h1 → h2 → h3 → h4). Verifies that:
///
/// - an explicit `level=Three` renders `<h3>` with the documented
///   `data-ars-*` attrs and no redundant `role`/`aria-level`;
/// - `HeadingLevelProvider` publishes the starting level (`Three`) to
///   descendants → `<h3>`;
/// - `Section` increments the inherited level (`Three → Four`) → `<h4>`.
///
/// # Errors
///
/// Returns an error when `WebDriver` cannot find the fixture nodes or an
/// assertion fails.
pub async fn run_heading_flow(
    driver: &WebDriver,
    url: &str,
    adapter: Adapter,
) -> Result<(), Error> {
    open_utility_panel(driver, url).await?;

    let prefix = id_prefix(adapter);

    let level_three = element_by_id(driver, &format!("{prefix}-heading-level-three")).await?;

    assert_expected_tag(
        &level_three.tag_name().await?,
        "h3",
        "explicit level=Three must render an h3 root",
    )?;
    assert_attr(&level_three, "data-ars-scope", "heading").await?;
    assert_attr(&level_three, "data-ars-part", "root").await?;
    assert_no_attr(&level_three, "role").await?;
    assert_no_attr(&level_three, "aria-level").await?;

    let provided = element_by_id(driver, &format!("{prefix}-heading-provided")).await?;

    assert_expected_tag(
        &provided.tag_name().await?,
        "h3",
        "HeadingLevelProvider must publish Level::Three to descendants",
    )?;

    let section_child = element_by_id(driver, &format!("{prefix}-heading-section")).await?;

    assert_expected_tag(
        &section_child.tag_name().await?,
        "h4",
        "Section must increment Level::Three to Level::Four",
    )?;

    Ok(())
}

const fn id_prefix(adapter: Adapter) -> &'static str {
    match adapter {
        Adapter::Leptos => "leptos-fixture",
        Adapter::Dioxus => "dioxus-fixture",
    }
}

fn assert_expected_tag(actual: &str, expected: &str, context: &str) -> Result<(), Error> {
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "{context}: expected {expected}, got {actual:?}"
        )))
    }
}

async fn assert_no_attr(element: &WebElement, name: &str) -> Result<(), Error> {
    if element.attr(name).await?.is_none() {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "expected {name:?} to be absent on element {:?}",
            element.attr("id").await?.unwrap_or_default()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_expected_tag_compares_case_insensitively() {
        assert!(assert_expected_tag("H1", "h1", "tag check").is_ok());
        assert!(assert_expected_tag("h2", "h2", "tag check").is_ok());

        let err = assert_expected_tag("h3", "h1", "tag check").expect_err("mismatch should fail");

        assert!(err.to_string().contains("expected h1"));
        assert!(err.to_string().contains("\"h3\""));
    }
}
