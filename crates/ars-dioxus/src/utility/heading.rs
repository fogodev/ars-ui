//! Dioxus `Heading` adapter.
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
use dioxus::prelude::*;

use crate::{as_child::merge_dioxus_attrs, attr_map_to_dioxus_inline_attrs};

fn inherited_context() -> HeadingContext {
    try_use_context::<HeadingContext>().unwrap_or_default()
}

/// Props for the Dioxus [`Heading`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct HeadingProps {
    /// Optional component instance ID.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Explicit heading level override; when absent, the nearest
    /// [`HeadingContext`] supplies the resolved level.
    #[props(optional)]
    pub level: Option<Level>,

    /// Global HTML attributes (e.g. `class`, `style`, `data-*`, `aria-*`)
    /// forwarded onto the rendered heading root. Tokenized attributes
    /// (`class`, `style`, relationship token lists) concatenate with the
    /// component's own values; ordinary attributes prefer the component's
    /// value on conflict so accessibility semantics stay intact.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Heading content rendered inside the resolved tag.
    pub children: Element,
}

/// Dioxus `Heading` component.
///
/// Renders a semantic `<h1>`–`<h6>` element whose level is resolved from the
/// explicit `level` prop, the nearest [`HeadingContext`] published by
/// [`HeadingLevelProvider`] or [`Section`], or [`Level::One`] when no provider
/// is in scope.
#[component]
pub fn Heading(props: HeadingProps) -> Element {
    let ctx = inherited_context();

    let mut core_props = Props::new();

    if let Some(id) = props.id.as_deref() {
        core_props = core_props.id(id);
    }

    if let Some(level) = props.level {
        core_props = core_props.level(level);
    }

    let api = Api::new(core_props, ctx);

    let resolved = api.resolved_level();

    let mut attrs = api.root_attrs(true);

    if props.id.is_none() {
        attrs.set(HtmlAttr::Id, AttrValue::None);
    }

    let component_attrs = attr_map_to_dioxus_inline_attrs(attrs);

    let attrs = merge_dioxus_attrs(props.attrs, component_attrs);

    let children = props.children;

    match resolved {
        Level::One => rsx! {
            h1 { ..attrs,{children} }
        },
        Level::Two => rsx! {
            h2 { ..attrs,{children} }
        },
        Level::Three => rsx! {
            h3 { ..attrs,{children} }
        },
        Level::Four => rsx! {
            h4 { ..attrs,{children} }
        },
        Level::Five => rsx! {
            h5 { ..attrs,{children} }
        },
        Level::Six => rsx! {
            h6 { ..attrs,{children} }
        },
    }
}

/// Props for the Dioxus [`HeadingLevelProvider`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct HeadingLevelProviderProps {
    /// Starting heading level to publish to descendants.
    pub level: Level,

    /// Descendants that should observe the published [`HeadingContext`].
    pub children: Element,
}

/// Dioxus `HeadingLevelProvider` component.
///
/// Provider-only component that publishes a starting [`HeadingContext`] to its
/// descendants. Renders no DOM of its own.
#[component]
pub fn HeadingLevelProvider(props: HeadingLevelProviderProps) -> Element {
    let provider_props = heading_level_provider::Props::new().level(props.level);

    use_context_provider(|| heading_level_provider::context_for(&provider_props));

    rsx! {
        {props.children}

    }
}

/// Props for the Dioxus [`Section`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct SectionProps {
    /// Descendants that should observe the incremented [`HeadingContext`].
    pub children: Element,
}

/// Dioxus `Section` component.
///
/// Provider-only component that publishes an incremented [`HeadingContext`] to
/// its descendants, clamped at [`Level::Six`]. Renders no DOM of its own.
#[component]
pub fn Section(props: SectionProps) -> Element {
    let parent = inherited_context();

    use_context_provider(|| section::context_for(&parent));

    rsx! {
        {props.children}
    }
}
