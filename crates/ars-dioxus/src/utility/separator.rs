//! Dioxus Separator adapter.
//!
//! Renders the framework-agnostic `Separator` root attributes onto either an
//! adapter-owned `<hr>` node or a consumer-owned root through
//! [`SeparatorAsChild`].

pub use ars_components::utility::separator::{Api, Part, Props};
use ars_core::HtmlAttr;
pub use ars_i18n::Orientation;
use dioxus::prelude::*;

use crate::{as_child::AsChildRenderProps, attr_map_to_dioxus_inline_attrs, merge_dioxus_attrs};

fn root_attrs(id: Option<&str>, orientation: Orientation, decorative: bool) -> Vec<Attribute> {
    let mut props = Props::new().orientation(orientation).decorative(decorative);

    if let Some(id) = id {
        props = props.id(id);
    }

    let mut attrs = Api::new(props).root_attrs();

    if let Some(id) = id {
        attrs.set(HtmlAttr::Id, id);
    }

    attr_map_to_dioxus_inline_attrs(attrs)
}

/// Props for the Dioxus [`Separator`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct SeparatorProps {
    /// Optional component instance ID.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Separator orientation.
    #[props(default = Orientation::Horizontal)]
    pub orientation: Orientation,

    /// Whether the separator is decorative and hidden from the accessibility tree.
    #[props(default = false)]
    pub decorative: bool,

    /// Global HTML attributes forwarded onto the rendered `<hr>` root.
    /// Tokenized attributes (`class`, `style`, relationship token lists)
    /// concatenate with the component's own values; ordinary attributes prefer
    /// the component's value on conflict so accessibility semantics stay
    /// intact.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,
}

/// Props for the Dioxus [`SeparatorAsChild`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct SeparatorAsChildProps {
    /// Optional component instance ID.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Separator orientation.
    #[props(default = Orientation::Horizontal)]
    pub orientation: Orientation,

    /// Whether the separator is decorative and hidden from the accessibility tree.
    #[props(default = false)]
    pub decorative: bool,

    /// Render callback that owns the child root and spreads `Separator` attrs.
    pub render: Callback<AsChildRenderProps, Element>,
}

/// Dioxus Separator component rendered as a single `<hr>` root.
#[component]
pub fn Separator(props: SeparatorProps) -> Element {
    let component_attrs = root_attrs(props.id.as_deref(), props.orientation, props.decorative);

    let attrs = merge_dioxus_attrs(props.attrs, component_attrs);

    rsx! {
        hr { ..attrs }
    }
}

/// Dioxus Separator component that forwards root attrs to one child root.
#[component]
pub fn SeparatorAsChild(props: SeparatorAsChildProps) -> Element {
    let attrs = root_attrs(props.id.as_deref(), props.orientation, props.decorative);

    props.render.call(AsChildRenderProps { attrs })
}
