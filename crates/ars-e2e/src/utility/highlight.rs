//! Browser E2E harness for the `Highlight` component.

use thirtyfour::prelude::*;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    utility::{assert_attr, element_by_id, open_utility_panel},
};

/// Runs the `Highlight` browser assertions inside the Utility fixture panel.
///
/// Verifies that:
///
/// - the root `<span>` carries `data-ars-scope="highlight"`,
///   `data-ars-part="root"`, and `dir="auto"`;
/// - matched text is wrapped in `<mark data-ars-highlighted="true">`;
/// - unmatched text is wrapped in `<span data-ars-highlighted="false">`.
///
/// # Errors
///
/// Returns an error when `WebDriver` cannot find the fixture nodes or an
/// assertion fails.
pub async fn run_highlight_flow(
    driver: &WebDriver,
    url: &str,
    adapter: Adapter,
) -> Result<(), Error> {
    open_utility_panel(driver, url).await?;

    let host = element_by_id(driver, &format!("{}-highlight-host", id_prefix(adapter))).await?;

    let root = host
        .find(By::Css("span[data-ars-scope='highlight']"))
        .await?;

    assert_attr(&root, "data-ars-part", "root").await?;
    assert_attr(&root, "dir", "auto").await?;

    let mark = root
        .find(By::Css("mark[data-ars-highlighted='true']"))
        .await?;

    assert_attr(&mark, "data-ars-part", "highlight-chunk").await?;

    let mark_text = mark.text().await?;

    if mark_text != "highlighted" {
        return Err(Error::Assertion(format!(
            "highlighted chunk must contain 'highlighted', got {mark_text:?}"
        )));
    }

    let unmatched = root
        .find(By::Css("span[data-ars-highlighted='false']"))
        .await?;

    assert_attr(&unmatched, "data-ars-part", "highlight-chunk").await?;

    Ok(())
}

const fn id_prefix(adapter: Adapter) -> &'static str {
    match adapter {
        Adapter::Leptos => "leptos-fixture",
        Adapter::Dioxus => "dioxus-fixture",
    }
}
