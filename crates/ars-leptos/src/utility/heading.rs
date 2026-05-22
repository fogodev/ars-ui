//! Leptos `Heading` adapter.
//!
//! Renders the framework-agnostic [`Heading`](ars_components::utility::heading)
//! attribute contract as a semantic `<h1>`–`<h6>` element whose level resolves
//! from explicit props or the inherited [`HeadingContext`]. Ships alongside
//! [`HeadingLevelProvider`] and [`Section`] provider components so descendants
//! receive an updated `HeadingContext` without prop drilling.

pub use ars_components::utility::heading::{
    Api, HeadingContext, Level, Part, Props, heading_level_provider, section,
};
use ars_core::{AttrValue, HtmlAttr};
use leptos::{children::TypedChildren, either::EitherOf6, prelude::*};

use crate::{attr_map_to_leptos_inline_attrs, merge_consumer_class_into};

/// Reads the inherited heading level context, defaulting to [`HeadingContext::new`]
/// (level one) when no provider is in scope.
fn inherited_context() -> HeadingContext {
    use_context::<HeadingContext>().unwrap_or_default()
}

/// Leptos `Heading` component.
///
/// Renders a semantic `<h1>`–`<h6>` element whose level is resolved from the
/// explicit `level` prop, the nearest [`HeadingContext`] published by
/// [`HeadingLevelProvider`] or [`Section`], or [`Level::One`] when no provider
/// is in scope.
#[component]
pub fn Heading<T>(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,

    /// Explicit heading level override; when absent, the nearest
    /// [`HeadingContext`] supplies the resolved level.
    #[prop(optional)]
    level: Option<Level>,

    /// Consumer class tokens appended to the heading root. Use this for
    /// Tailwind utility chains or any other CSS class list — the tokens
    /// merge with whatever class the component itself emits so both reach
    /// the rendered `<h*>` as a single `class` attribute.
    #[prop(optional, into)]
    class: Option<Oco<'static, str>>,

    /// Heading content rendered inside the resolved tag.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let id = id.map(Oco::into_owned);
    let class = class.map(Oco::into_owned);

    let ctx = inherited_context();

    let mut props = Props::new();

    if let Some(id) = id.as_deref() {
        props = props.id(id);
    }

    if let Some(level) = level {
        props = props.level(level);
    }

    let api = Api::new(props, ctx);

    let resolved = api.resolved_level();

    let mut attrs = api.root_attrs(true);

    if id.is_none() {
        attrs.set(HtmlAttr::Id, AttrValue::None);
    }

    merge_consumer_class_into(&mut attrs, class.as_deref());

    let attrs = attr_map_to_leptos_inline_attrs(attrs);

    let children = children.into_inner();

    let body = children();

    match resolved {
        Level::One => EitherOf6::A(view! { <h1 {..attrs}>{body}</h1> }),
        Level::Two => EitherOf6::B(view! { <h2 {..attrs}>{body}</h2> }),
        Level::Three => EitherOf6::C(view! { <h3 {..attrs}>{body}</h3> }),
        Level::Four => EitherOf6::D(view! { <h4 {..attrs}>{body}</h4> }),
        Level::Five => EitherOf6::E(view! { <h5 {..attrs}>{body}</h5> }),
        Level::Six => EitherOf6::F(view! { <h6 {..attrs}>{body}</h6> }),
    }
}

/// Leptos `HeadingLevelProvider` component.
///
/// Provider-only component that publishes a starting [`HeadingContext`] to its
/// descendants. Renders no DOM of its own.
#[component]
pub fn HeadingLevelProvider<T>(
    /// Starting heading level to publish to descendants.
    level: Level,

    /// Descendants that should observe the published [`HeadingContext`].
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let provider_props = heading_level_provider::Props::new().level(level);

    provide_context(heading_level_provider::context_for(&provider_props));

    children.into_inner()()
}

/// Leptos `Section` component.
///
/// Provider-only component that publishes an incremented [`HeadingContext`] to
/// its descendants, clamped at [`Level::Six`]. Renders no DOM of its own.
#[component]
pub fn Section<T>(
    /// Descendants that should observe the incremented [`HeadingContext`].
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let parent = inherited_context();

    provide_context(section::context_for(&parent));

    children.into_inner()()
}
