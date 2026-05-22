//! Browser E2E harness for the `Landmark` component.

use thirtyfour::prelude::*;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    utility::{assert_attr, element_by_id, open_utility_panel},
};

/// Runs the `Landmark` browser assertions inside the Utility fixture panel.
///
/// Verifies the documented role/element mapping for the four fixture roles:
///
/// - `Banner` → `<header>`
/// - `Navigation` → `<nav>`
/// - `Search` → `<div role="search">` fallback (matches
///   `Api::prefers_generic_fallback_element`)
/// - `Region` → `<section>`
///
/// Also asserts that native landmark elements omit redundant `role` attrs and
/// that the localized `aria-label` is present.
///
/// # Errors
///
/// Returns an error when `WebDriver` cannot find the fixture nodes or an
/// assertion fails.
pub async fn run_landmark_flow(
    driver: &WebDriver,
    url: &str,
    adapter: Adapter,
) -> Result<(), Error> {
    open_utility_panel(driver, url).await?;

    let prefix = id_prefix(adapter);

    let banner = element_by_id(driver, &format!("{prefix}-landmark-banner")).await?;

    assert_expected_tag(
        &banner.tag_name().await?,
        "header",
        "Banner landmark must render <header>",
    )?;
    assert_attr(&banner, "data-ars-scope", "landmark").await?;
    assert_attr(&banner, "data-ars-part", "root").await?;
    assert_attr(&banner, "aria-label", "Page banner").await?;
    assert_no_attr(&banner, "role").await?;

    let navigation = element_by_id(driver, &format!("{prefix}-landmark-navigation")).await?;

    assert_expected_tag(
        &navigation.tag_name().await?,
        "nav",
        "Navigation landmark must render <nav>",
    )?;
    assert_no_attr(&navigation, "role").await?;
    assert_attr(&navigation, "aria-label", "Primary navigation").await?;

    let search = element_by_id(driver, &format!("{prefix}-landmark-search")).await?;

    assert_expected_tag(
        &search.tag_name().await?,
        "div",
        "Search landmark must use the explicit <div role='search'> fallback",
    )?;
    assert_attr(&search, "role", "search").await?;
    assert_attr(&search, "aria-label", "Site search").await?;

    let region = element_by_id(driver, &format!("{prefix}-landmark-region")).await?;

    assert_expected_tag(
        &region.tag_name().await?,
        "section",
        "Region landmark must render <section>",
    )?;
    assert_no_attr(&region, "role").await?;
    assert_attr(&region, "aria-label", "Sidebar region").await?;

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
        assert!(assert_expected_tag("HEADER", "header", "tag check").is_ok());

        let err = assert_expected_tag("section", "header", "tag check")
            .expect_err("mismatch should fail");

        assert!(err.to_string().contains("expected header"));
        assert!(err.to_string().contains("\"section\""));
    }
}
