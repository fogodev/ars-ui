//! Leptos `VisuallyHidden` adapter.
//!
//! Renders the framework-agnostic `VisuallyHidden` attribute contract as either
//! an adapter-owned `<span>` root or a consumer-owned root through
//! [`VisuallyHiddenAsChild`].

pub use ars_components::utility::visually_hidden::{Api, Part, Props};
use ars_core::{AttrMap, AttrValue, HtmlAttr};
use leptos::{children::TypedChildren, prelude::*, tachys::view::add_attr::AddAnyAttr};

use crate::{as_child::AsChildAttrs, attr_map_to_leptos_inline_attrs};

fn root_attr_map(id: Option<String>, is_focusable: bool, as_child: bool) -> AttrMap {
    let mut props = Props::new().is_focusable(is_focusable).as_child(as_child);

    if let Some(id) = id.clone() {
        props = props.id(id);
    }

    let mut attrs = Api::new(props).root_attrs();

    if let Some(id) = id {
        attrs.set(HtmlAttr::Id, id);
    }

    attrs
}

fn root_attrs(
    id: Option<String>,
    is_focusable: bool,
    as_child: bool,
) -> Vec<crate::LeptosAttribute> {
    attr_map_to_leptos_inline_attrs(root_attr_map(id, is_focusable, as_child))
}

fn as_child_root_attrs(id: Option<String>, is_focusable: bool) -> Vec<crate::LeptosAttribute> {
    use leptos::tachys::html::attribute::any_attribute::IntoAnyAttribute as _;

    let mut attrs = root_attr_map(id, is_focusable, true);

    if !is_focusable {
        attrs.set(HtmlAttr::Class, AttrValue::None);
    }

    let mut leptos_attrs = attr_map_to_leptos_inline_attrs(attrs);

    if !is_focusable {
        leptos_attrs.push(
            leptos::tachys::html::class::class(("ars-visually-hidden", true)).into_any_attr(),
        );
    }

    leptos_attrs
}

/// Leptos VisuallyHidden component rendered as an adapter-owned `<span>` root.
#[component]
pub fn VisuallyHidden<T>(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,

    /// Whether the hidden content should become visible when focused.
    #[prop(optional)]
    is_focusable: bool,

    /// Hidden content that remains available to assistive technology.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let id = id.map(Oco::into_owned);
    let attrs = root_attrs(id, is_focusable, false);
    let children = children.into_inner();

    view! { <span {..attrs}>{children()}</span> }
}

/// Leptos VisuallyHidden component that forwards root attrs to one child root.
#[component]
pub fn VisuallyHiddenAsChild<T>(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,

    /// Whether the hidden content should become visible when focused.
    #[prop(optional)]
    is_focusable: bool,

    /// Child root that receives the visually-hidden root attrs.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: AddAnyAttr,
    <View<T> as AddAnyAttr>::Output<Vec<crate::LeptosAttribute>>: IntoView,
{
    let id = id.map(Oco::into_owned);

    children.into_inner()()
        .add_any_attr(AsChildAttrs::from(as_child_root_attrs(id, is_focusable)).into_inner())
}
