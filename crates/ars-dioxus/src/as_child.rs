//! Dioxus root-reassignment helper for the `as_child` pattern.
//!
//! The slot receives converted Dioxus attributes from a hosting component and
//! passes them to an explicit render callback. The callback owns the root
//! element and must spread the provided attrs onto that root. This avoids
//! arbitrary `VNode` template mutation, which is not a stable Dioxus composition
//! surface.

use std::fmt::{self, Debug};

use dioxus::prelude::*;

use crate::merge_dioxus_attrs;

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
