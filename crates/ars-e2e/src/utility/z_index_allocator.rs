//! Browser E2E harness for the `ZIndexAllocator` component.

use serde_json::Value;
use thirtyfour::prelude::*;

pub use crate::fixtures::Adapter;
use crate::{
    Error,
    utility::{assert_attr, element_by_id, open_utility_panel},
};

/// Runs the `ZIndexAllocator` browser assertions inside the Utility fixture panel.
///
/// # Errors
///
/// Returns an error when `WebDriver` cannot find the fixture nodes or an
/// assertion fails.
pub async fn run_z_index_allocator_flow(
    driver: &WebDriver,
    url: &str,
    adapter: Adapter,
) -> Result<(), Error> {
    open_utility_panel(driver, url).await?;

    let prefix = id_prefix(adapter);

    let first = element_by_id(driver, &format!("{prefix}-z-index-first")).await?;
    let second = element_by_id(driver, &format!("{prefix}-z-index-second")).await?;

    assert_attr(&first, "data-z", "1000").await?;
    assert_attr(&second, "data-z", "1001").await?;

    let direct_probe_count = driver
        .execute(
            r#"
            const host = document.getElementById(arguments[0]);
            if (!host) return -1;
            return Array.from(host.children)
                .filter((child) => child.id === arguments[1] || child.id === arguments[2])
                .length;
            "#,
            vec![
                Value::String(format!("{prefix}-z-index-host")),
                Value::String(format!("{prefix}-z-index-first")),
                Value::String(format!("{prefix}-z-index-second")),
            ],
        )
        .await?;

    if direct_probe_count.json().as_i64() == Some(2) {
        Ok(())
    } else {
        Err(Error::Assertion(
            "ZIndexAllocatorProvider should not insert a wrapper around probes".to_string(),
        ))
    }
}

const fn id_prefix(adapter: Adapter) -> &'static str {
    match adapter {
        Adapter::Leptos => "leptos-fixture",
        Adapter::Dioxus => "dioxus-fixture",
    }
}
