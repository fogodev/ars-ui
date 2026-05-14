//! Dioxus `VisuallyHidden` adapter.
//!
//! Renders the framework-agnostic `VisuallyHidden` attribute contract as either
//! an adapter-owned `<span>` root or a consumer-owned root through
//! [`VisuallyHiddenAsChild`].

pub use ars_components::utility::visually_hidden::{Api, Part, Props};
use ars_core::HtmlAttr;
use dioxus::prelude::*;

use crate::{as_child::AsChildRenderProps, attr_map_to_dioxus_inline_attrs};

fn root_attrs(id: Option<&str>, is_focusable: bool, as_child: bool) -> Vec<Attribute> {
    let mut props = Props::new().is_focusable(is_focusable).as_child(as_child);

    if let Some(id) = id {
        props = props.id(id);
    }

    let mut attrs = Api::new(props).root_attrs();

    if let Some(id) = id {
        attrs.set(HtmlAttr::Id, id);
    }

    attr_map_to_dioxus_inline_attrs(attrs)
}

/// Props for the Dioxus [`VisuallyHidden`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct VisuallyHiddenProps {
    /// Optional component instance ID.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Whether the hidden content should become visible when focused.
    #[props(default = false)]
    pub is_focusable: bool,

    /// Hidden content that remains available to assistive technology.
    pub children: Element,
}

/// Props for the Dioxus [`VisuallyHiddenAsChild`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct VisuallyHiddenAsChildProps {
    /// Optional component instance ID.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Whether the hidden content should become visible when focused.
    #[props(default = false)]
    pub is_focusable: bool,

    /// Render callback that owns the child root and spreads `VisuallyHidden` attrs.
    pub render: Callback<AsChildRenderProps, Element>,
}

/// Dioxus `VisuallyHidden` component rendered as an adapter-owned `<span>` root.
#[component]
pub fn VisuallyHidden(props: VisuallyHiddenProps) -> Element {
    let attrs = root_attrs(props.id.as_deref(), props.is_focusable, false);
    let children = props.children;

    rsx! {
        span { ..attrs,{children} }
    }
}

/// Dioxus `VisuallyHidden` component that forwards root attrs to one child root.
#[component]
pub fn VisuallyHiddenAsChild(props: VisuallyHiddenAsChildProps) -> Element {
    let attrs = root_attrs(props.id.as_deref(), props.is_focusable, true);

    props.render.call(AsChildRenderProps { attrs })
}
