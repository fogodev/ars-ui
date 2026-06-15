//! Dioxus `Landmark` adapter.
//!
//! Renders the framework-agnostic [`Landmark`](ars_components::utility::landmark)
//! attribute contract as a semantic landmark element selected from the
//! configured [`Role`], falling back to `<div role="…">` when no native
//! landmark element applies (currently only [`Role::Search`]).

pub use ars_components::utility::landmark::{Api, Messages, Part, Props, Role};
use ars_core::{AttrValue, Env, HtmlAttr, Locale};
use dioxus::prelude::*;

use crate::{attr_map_to_dioxus_inline_attrs, merge_dioxus_attrs, use_messages_and_locale};

/// Props for the Dioxus [`Landmark`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct LandmarkProps {
    /// Optional component instance ID.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Semantic landmark role. Defaults to [`Role::Region`].
    #[props(default = Role::Region)]
    pub role: Role,

    /// Optional ID of an external element that labels this landmark. When set
    /// and non-empty, `aria-labelledby` takes precedence over the localized
    /// `aria-label`.
    #[props(optional, into)]
    pub labelledby_id: Option<String>,

    /// Localized messages bundle. When omitted, the adapter resolves messages
    /// from the [`ArsProvider`](crate::ArsProvider) message registry, falling
    /// back to [`Messages::default`] (empty label).
    #[props(optional)]
    pub messages: Option<Messages>,

    /// Locale override for resolving the `aria-label`. When absent, the
    /// surrounding [`ArsProvider`](crate::ArsProvider) locale is used.
    #[props(optional)]
    pub locale: Option<Locale>,

    /// Global HTML attributes forwarded onto the rendered landmark root.
    /// Tokenized attributes (`class`, `style`, relationship token lists)
    /// concatenate with the component's own values; ordinary attributes prefer
    /// the component's value on conflict so accessibility semantics stay
    /// intact.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Children rendered inside the landmark element.
    pub children: Element,
}

/// Dioxus `Landmark` component.
///
/// Renders a semantic landmark element (`<header>`, `<nav>`, `<main>`,
/// `<aside>`, `<footer>`, `<form>`, `<section>`) for the requested [`Role`],
/// or an explicit `<div role="search">` fallback when
/// [`Api::prefers_generic_fallback_element`] returns `true`. Resolves the
/// localized `aria-label` from `messages` (or the provider's message
/// registry when omitted) using the optional `locale` override or the
/// surrounding [`ArsProvider`](crate::ArsProvider) locale.
#[component]
pub fn Landmark(props: LandmarkProps) -> Element {
    let mut core_props = Props::new().role(props.role);

    if let Some(id) = props.id.as_deref() {
        core_props = core_props.id(id);
    }

    if let Some(labelledby) = props.labelledby_id.as_deref() {
        core_props = core_props.labelledby_id(labelledby);
    }

    let (resolved_messages, resolved_locale) =
        use_messages_and_locale::<Messages>(props.messages, props.locale);

    let env = Env {
        locale: resolved_locale,
        ..Env::default()
    };

    let api = Api::new(core_props, &env, &resolved_messages);

    let is_native = !api.prefers_generic_fallback_element();

    let mut attrs = api.root_attrs(is_native);

    if props.id.is_none() {
        attrs.set(HtmlAttr::Id, AttrValue::None);
    }

    let component_attrs = attr_map_to_dioxus_inline_attrs(attrs);

    let attrs = merge_dioxus_attrs(props.attrs, component_attrs);

    let children = props.children;

    if !is_native {
        return rsx! {
            div { ..attrs,{children} }
        };
    }

    match props.role {
        Role::Banner => rsx! {
            header { ..attrs,{children} }
        },

        Role::Navigation => rsx! {
            nav { ..attrs,{children} }
        },

        Role::Main => rsx! {
            main { ..attrs,{children} }
        },

        Role::Complementary => rsx! {
            aside { ..attrs,{children} }
        },

        Role::ContentInfo => rsx! {
            footer { ..attrs,{children} }
        },

        Role::Form => rsx! {
            form { ..attrs,{children} }
        },

        Role::Region => rsx! {
            section { ..attrs,{children} }
        },

        Role::Search => {
            unreachable!("Role::Search is handled by Api::prefers_generic_fallback_element()")
        }
    }
}
