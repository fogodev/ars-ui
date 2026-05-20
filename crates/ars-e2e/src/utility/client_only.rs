//! Browser E2E harness for the `ClientOnly` component.

use serde_json::Value;
use thirtyfour::prelude::*;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    utility::{element_by_id, open_utility_panel},
};

/// Runs the `ClientOnly` browser assertions inside the Utility fixture panel.
///
/// # Errors
///
/// Returns an error when `WebDriver` cannot find the fixture nodes or an
/// assertion fails.
pub async fn run_client_only_flow(
    driver: &WebDriver,
    url: &str,
    adapter: Adapter,
) -> Result<(), Error> {
    open_utility_panel(driver, url).await?;

    let prefix = id_prefix(adapter);

    let child = element_by_id(driver, &format!("{prefix}-client-only-child")).await?;

    if child.text().await? != "Client content" {
        return Err(Error::Assertion(
            "ClientOnly child should be visible after mount".to_string(),
        ));
    }

    let fallback_absent = driver
        .execute(
            "return document.getElementById(arguments[0]) === null;",
            vec![Value::String(format!("{prefix}-client-only-fallback"))],
        )
        .await?;

    if fallback_absent.json().as_bool() == Some(true) {
        Ok(())
    } else {
        Err(Error::Assertion(
            "ClientOnly fallback should be removed after mount".to_string(),
        ))
    }
}

const fn id_prefix(adapter: Adapter) -> &'static str {
    match adapter {
        Adapter::Leptos => "leptos-fixture",
        Adapter::Dioxus => "dioxus-fixture",
    }
}
