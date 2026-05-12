//! Shared axe-core accessibility checks for browser E2E harnesses.

use std::{env, fs, path::PathBuf, process::Command};

use serde_json::Value;
use thirtyfour::prelude::*;

use crate::Error;

const AXE_CORE_VERSION: &str = "4.11.4";

pub(crate) async fn run_axe(driver: &WebDriver) -> Result<(), Error> {
    let axe_source = axe_source()?;
    driver.execute(&axe_source, Vec::<Value>::new()).await?;

    let result = driver
        .execute_async(
            r#"
            const done = arguments[0];
            axe.run(document, {
                rules: {
                    "color-contrast": { enabled: false },
                    "link-in-text-block": { enabled: false }
                }
            }).then((results) => done(results)).catch((error) => done({ error: String(error) }));
            "#,
            Vec::<Value>::new(),
        )
        .await?;

    if let Some(error) = result.json().get("error").and_then(Value::as_str) {
        return Err(Error::Assertion(format!("axe-core failed: {error}")));
    }

    let violations = result
        .json()
        .get("violations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if violations.is_empty() {
        return Ok(());
    }

    Err(Error::Assertion(format!(
        "axe-core violations: {}",
        format_axe_violations(&violations)
    )))
}

fn axe_source() -> Result<String, Error> {
    let path = axe_cache_path()?;

    if path.exists() {
        return fs::read_to_string(&path).map_err(|error| {
            Error::Command(format!("failed to read {}: {error}", path.display()))
        });
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            Error::Command(format!("failed to create {}: {error}", parent.display()))
        })?;
    }

    let url = format!("https://cdn.jsdelivr.net/npm/axe-core@{AXE_CORE_VERSION}/axe.min.js");

    let status = Command::new("curl")
        .arg("--fail")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--output")
        .arg(&path)
        .arg(url)
        .status()
        .map_err(|error| Error::Command(format!("failed to spawn curl for axe-core: {error}")))?;

    if !status.success() {
        return Err(Error::Command(format!(
            "failed to download axe-core {AXE_CORE_VERSION}: curl exited with {status}"
        )));
    }

    fs::read_to_string(&path)
        .map_err(|error| Error::Command(format!("failed to read {}: {error}", path.display())))
}

fn axe_cache_path() -> Result<PathBuf, Error> {
    let cwd = env::current_dir()
        .map_err(|error| Error::Command(format!("failed to read current directory: {error}")))?;

    Ok(cwd
        .join("target")
        .join("ars-e2e")
        .join(format!("axe-core-{AXE_CORE_VERSION}.min.js")))
}

fn format_axe_violations(violations: &[Value]) -> String {
    violations
        .iter()
        .map(|violation| {
            let id = violation
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>");

            let impact = violation
                .get("impact")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>");

            let help = violation
                .get("help")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>");

            let nodes = violation
                .get("nodes")
                .and_then(Value::as_array)
                .map(|nodes| {
                    nodes
                        .iter()
                        .filter_map(|node| {
                            node.get("target")
                                .and_then(Value::as_array)
                                .map(|target| Value::Array(target.clone()).to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();

            format!("[{id}] impact={impact} help={help} nodes={nodes}")
        })
        .collect::<Vec<_>>()
        .join("; ")
}
