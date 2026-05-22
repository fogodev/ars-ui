//! Leptos Separator adapter.
//!
//! Renders the framework-agnostic `Separator` root attributes onto either an
//! adapter-owned `<hr>` node or a consumer-owned root through
//! [`SeparatorAsChild`].

pub use ars_components::utility::separator::{Api, Part, Props};
use ars_core::HtmlAttr;
pub use ars_i18n::Orientation;
use leptos::{children::TypedChildren, prelude::*, tachys::view::add_attr::AddAnyAttr};

use crate::{as_child::AsChildAttrs, attr_map_to_leptos_inline_attrs, merge_consumer_class_into};

fn root_attrs(
    id: Option<String>,
    orientation: Orientation,
    decorative: bool,
    class: Option<&str>,
) -> Vec<crate::LeptosAttribute> {
    let mut props = Props::new().orientation(orientation).decorative(decorative);

    if let Some(id) = id.clone() {
        props = props.id(id);
    }

    let mut attrs = Api::new(props).root_attrs();

    if let Some(id) = id {
        attrs.set(HtmlAttr::Id, id);
    }

    merge_consumer_class_into(&mut attrs, class);

    attr_map_to_leptos_inline_attrs(attrs)
}

/// Leptos Separator component rendered as a single `<hr>` root.
#[component]
pub fn Separator(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,

    /// Separator orientation.
    #[prop(optional)]
    orientation: Orientation,

    /// Whether the separator is decorative and hidden from the accessibility tree.
    #[prop(optional)]
    decorative: bool,

    /// Consumer class tokens appended to the rendered `<hr>` root.
    #[prop(optional, into)]
    class: Option<Oco<'static, str>>,
) -> impl IntoView {
    let id = id.map(Oco::into_owned);

    let class = class.map(Oco::into_owned);

    let attrs = root_attrs(id, orientation, decorative, class.as_deref());

    view! { <hr {..attrs} /> }
}

/// Leptos Separator component that forwards root attrs to one child root.
#[component]
pub fn SeparatorAsChild<T>(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,

    /// Separator orientation.
    #[prop(optional)]
    orientation: Orientation,

    /// Whether the separator is decorative and hidden from the accessibility tree.
    #[prop(optional)]
    decorative: bool,

    /// Consumer class tokens forwarded onto the child root alongside the
    /// separator's own attrs.
    #[prop(optional, into)]
    class: Option<Oco<'static, str>>,

    /// Child root that receives the separator root attrs.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: AddAnyAttr,
    <View<T> as AddAnyAttr>::Output<Vec<crate::LeptosAttribute>>: IntoView,
{
    let id = id.map(Oco::into_owned);
    let class = class.map(Oco::into_owned);

    children.into_inner()().add_any_attr(
        AsChildAttrs::from(root_attrs(id, orientation, decorative, class.as_deref())).into_inner(),
    )
}
