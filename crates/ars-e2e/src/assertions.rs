//! Shared browser assertions for E2E harnesses.

use serde_json::{Value, json};
use thirtyfour::{
    cdp::domains::emulation::{MediaFeature, MediaType, SetEmulatedMedia},
    prelude::*,
};

use crate::Error;

pub(crate) async fn assert_active_role(driver: &WebDriver, expected: &str) -> Result<(), Error> {
    let active = driver.active_element().await?;

    let role = active.attr("role").await?;

    if role.as_deref() == Some(expected) {
        Ok(())
    } else {
        let tag = active
            .tag_name()
            .await
            .unwrap_or_else(|_| "<unknown>".into());

        let text = active.text().await.unwrap_or_else(|_| "<unknown>".into());

        let html = active
            .attr("outerHTML")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "<unknown>".into());

        Err(Error::Assertion(format!(
            "expected active element role {expected:?}, got {role:?}; active tag={tag:?} text={text:?} html={html:?}"
        )))
    }
}

pub(crate) async fn assert_active_text_contains(
    driver: &WebDriver,
    expected: &str,
) -> Result<(), Error> {
    let active = driver.active_element().await?;

    let text = active.text().await?;

    if text.contains(expected) {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "expected active element text to contain {expected:?}, got {text:?}"
        )))
    }
}

pub(crate) async fn assert_focus_indicator_visible(driver: &WebDriver) -> Result<(), Error> {
    let active = driver.active_element().await?;

    let has_focus_visible = active.attr("data-ars-focus-visible").await?.is_some();

    let styles = computed_focus_styles(&active).await?;

    if has_focus_visible && styles.has_visible_focus_indicator() {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "focused element must expose data-ars-focus-visible and a visible focus indicator; data attr={has_focus_visible}, styles={styles:?}"
        )))
    }
}

pub(crate) async fn assert_forced_colors_focus_indicator_visible(
    driver: &WebDriver,
) -> Result<(), Error> {
    driver
        .cdp()
        .send(SetEmulatedMedia {
            media: Some(MediaType::Screen),
            features: Some(vec![MediaFeature {
                name: "forced-colors".to_string(),
                value: "active".to_string(),
            }]),
        })
        .await?;

    let styles = computed_focus_styles(&driver.active_element().await?).await?;

    driver
        .cdp()
        .send(SetEmulatedMedia {
            media: Some(MediaType::Screen),
            features: Some(Vec::new()),
        })
        .await?;

    if styles.has_visible_focus_indicator() {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "focused element must keep a visible outline or border in forced-colors mode; styles={styles:?}"
        )))
    }
}

pub(crate) async fn assert_selected_indicator_visible(
    element: &WebElement,
    label: &str,
) -> Result<(), Error> {
    let result = element
        .handle()
        .execute(
            r#"
            const el = arguments[0];
            const styles = getComputedStyle(el);
            return {
                backgroundColor: styles.backgroundColor,
                color: styles.color,
                borderColor: styles.borderColor,
                boxShadow: styles.boxShadow
            };
            "#,
            vec![element.to_json()?],
        )
        .await?;

    let value = result.json();

    let background = value
        .get("backgroundColor")
        .and_then(Value::as_str)
        .unwrap_or("");

    let border = value
        .get("borderColor")
        .and_then(Value::as_str)
        .unwrap_or("");

    let shadow = value.get("boxShadow").and_then(Value::as_str).unwrap_or("");

    if !is_transparent(background) || !is_transparent(border) || !shadow_is_none(shadow) {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "selected element {label:?} must have a visible selected-state indicator; computed={value}"
        )))
    }
}

pub(crate) async fn assert_accessibility_tree_has_role_and_name(
    driver: &WebDriver,
    role: &str,
    name: &str,
) -> Result<(), Error> {
    let tree = driver
        .cdp()
        .send_raw("Accessibility.getFullAXTree", json!({}))
        .await?;

    if ax_tree_has_role_and_name(&tree, role, name) {
        Ok(())
    } else {
        Err(Error::Assertion(format!(
            "accessibility tree must include a {role:?} named {name:?}"
        )))
    }
}

async fn computed_focus_styles(element: &WebElement) -> Result<FocusStyles, Error> {
    let result = element
        .handle()
        .execute(
            r#"
            const el = arguments[0];
            const styles = getComputedStyle(el);
            return {
                outlineStyle: styles.outlineStyle,
                outlineWidth: styles.outlineWidth,
                borderTopStyle: styles.borderTopStyle,
                borderTopWidth: styles.borderTopWidth,
                boxShadow: styles.boxShadow
            };
            "#,
            vec![element.to_json()?],
        )
        .await?;

    let value = result.json();

    Ok(FocusStyles {
        outline_style: string_prop(value, "outlineStyle"),
        outline_width: string_prop(value, "outlineWidth"),
        border_top_style: string_prop(value, "borderTopStyle"),
        border_top_width: string_prop(value, "borderTopWidth"),
        box_shadow: string_prop(value, "boxShadow"),
    })
}

#[derive(Debug)]
struct FocusStyles {
    outline_style: String,
    outline_width: String,
    border_top_style: String,
    border_top_width: String,
    box_shadow: String,
}

impl FocusStyles {
    fn has_visible_focus_indicator(&self) -> bool {
        (self.outline_style != "none" && !is_zero_width(&self.outline_width))
            || (self.border_top_style != "none" && !is_zero_width(&self.border_top_width))
            || !shadow_is_none(&self.box_shadow)
    }
}

fn ax_tree_has_role_and_name(tree: &Value, role: &str, name: &str) -> bool {
    tree.get("nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|node| {
            node_property_value(node, "role").as_deref() == Some(role)
                && node_property_value(node, "name").is_some_and(|value| value.contains(name))
        })
}

fn node_property_value(node: &Value, property: &str) -> Option<String> {
    node.get(property)
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn string_prop(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn is_zero_width(value: &str) -> bool {
    matches!(value.trim(), "" | "0px" | "0")
}

fn is_transparent(value: &str) -> bool {
    matches!(value.trim(), "" | "transparent" | "rgba(0, 0, 0, 0)")
}

fn shadow_is_none(value: &str) -> bool {
    matches!(value.trim(), "" | "none")
}
