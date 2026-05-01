//! Dioxus root-reassignment helper for the `as_child` pattern.
//!
//! The slot receives converted Dioxus attributes from a hosting component and
//! passes them to an explicit render callback. The callback owns the root
//! element and must spread the provided attrs onto that root. This avoids
//! arbitrary `VNode` template mutation, which is not a stable Dioxus composition
//! surface.

use std::fmt::{self, Debug};

use dioxus::{dioxus_core::AttributeValue, prelude::*};

/// Props passed to the Dioxus `as_child` render callback.
#[derive(Clone, Debug, PartialEq)]
pub struct AsChildRenderProps {
    /// Dioxus attributes produced by the hosting component, ready to spread
    /// with `..attrs` onto the callback root.
    pub attrs: Vec<Attribute>,
}

impl AsChildRenderProps {
    /// Merges child-root attrs with the slot attrs before the callback spreads
    /// them onto the root element.
    ///
    /// Use this when the callback root contributes attrs that would otherwise
    /// duplicate slot attrs after `..attrs` spreading. `class`, `style`, and
    /// tokenized relationship attrs are concatenated with child values first;
    /// for other duplicate attrs the slot value wins to preserve component
    /// semantics.
    #[must_use]
    pub fn merged_attrs(&self, child_attrs: Vec<Attribute>) -> Vec<Attribute> {
        merge_dioxus_attrs(child_attrs, self.attrs.clone())
    }
}

/// Props for [`AsChildSlot`].
#[derive(Props, Clone, PartialEq)]
pub struct AsChildSlotProps {
    /// Converted Dioxus attributes produced by the hosting component.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Render callback that owns the single root element and spreads the
    /// provided attrs onto it.
    pub render: Callback<AsChildRenderProps, Element>,
}

impl Debug for AsChildSlotProps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsChildSlotProps")
            .field("attrs", &self.attrs)
            .field("render", &"<callback>")
            .finish()
    }
}

/// Renders the consumer-owned root returned by [`AsChildSlotProps::render`]
/// without introducing an adapter wrapper node.
#[component]
pub fn AsChildSlot(props: AsChildSlotProps) -> Element {
    props.render.call(AsChildRenderProps { attrs: props.attrs })
}

/// Merges component attrs onto child attrs before they are spread onto a
/// Dioxus callback root.
///
/// This native-attribute helper mirrors the core [`AsChildMerge`] behavior for
/// attrs that are still visible as Dioxus [`Attribute`] values. Component attrs
/// win for ordinary duplicate attributes, while `class`, `style`, and
/// relationship token lists concatenate with child values first.
///
/// [`AsChildMerge`]: ars_components::utility::as_child::AsChildMerge
#[must_use]
pub fn merge_dioxus_attrs(
    child_attrs: Vec<Attribute>,
    component_attrs: Vec<Attribute>,
) -> Vec<Attribute> {
    let mut merged = child_attrs;

    for component_attr in component_attrs {
        merge_or_replace_attr(&mut merged, component_attr);
    }

    merged
}

fn merge_or_replace_attr(attrs: &mut Vec<Attribute>, component_attr: Attribute) {
    let Some(index) = attrs.iter().position(|attr| {
        attr.name == component_attr.name && attr.namespace == component_attr.namespace
    }) else {
        attrs.push(component_attr);

        return;
    };

    let child_attr = &mut attrs[index];

    if let Some(value) = merge_attribute_values(
        component_attr.name,
        &child_attr.value,
        &component_attr.value,
    ) {
        child_attr.value = value;
        child_attr.volatile |= component_attr.volatile;

        return;
    }

    attrs[index] = component_attr;
}

fn merge_attribute_values(
    name: &str,
    child: &AttributeValue,
    component: &AttributeValue,
) -> Option<AttributeValue> {
    let AttributeValue::Text(child) = child else {
        return None;
    };

    let AttributeValue::Text(component) = component else {
        return None;
    };

    match name {
        "class" | "aria-labelledby" | "aria-describedby" | "aria-controls" | "aria-owns"
        | "aria-flowto" | "aria-details" | "rel" => {
            Some(AttributeValue::Text(merge_tokens(child, component, " ")))
        }

        "style" => Some(AttributeValue::Text(merge_style_text(child, component))),

        _ => None,
    }
}

fn merge_tokens(child: &str, component: &str, separator: &str) -> String {
    if child.is_empty() {
        return String::from(component);
    }

    if component.is_empty() {
        return String::from(child);
    }

    let mut tokens = child.split_whitespace().collect::<Vec<_>>();

    for token in component.split_whitespace() {
        if !tokens.contains(&token) {
            tokens.push(token);
        }
    }

    tokens.join(separator)
}

fn merge_style_text(child: &str, component: &str) -> String {
    match (child.trim_end_matches(';').trim(), component.trim()) {
        ("", component) => String::from(component),
        (child, "") => String::from(child),
        (child, component) => format!("{child}; {component}"),
    }
}
