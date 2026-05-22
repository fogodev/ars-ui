//! Leptos `Landmark` adapter.
//!
//! Renders the framework-agnostic [`Landmark`](ars_components::utility::landmark)
//! attribute contract as a semantic landmark element selected from the
//! configured [`Role`], falling back to `<div role="…">` when no native
//! landmark element applies (currently only [`Role::Search`]).

pub use ars_components::utility::landmark::{Api, Messages, Part, Props, Role};
use ars_core::{AttrValue, Env, HtmlAttr, Locale};
use leptos::{
    children::TypedChildren,
    either::{Either, EitherOf7},
    prelude::*,
};

use crate::{
    attr_map_to_leptos_inline_attrs, merge_consumer_class_into, resolve_locale, use_messages,
};

/// Leptos `Landmark` component.
///
/// Renders a semantic landmark element (`<header>`, `<nav>`, `<main>`,
/// `<aside>`, `<footer>`, `<form>`, `<section>`) for the requested
/// [`Role`], or an explicit `<div role="search">` fallback when
/// [`Api::prefers_generic_fallback_element`] returns `true`. Resolves the
/// localized `aria-label` from `messages` (or the provider's message
/// registry when omitted) using the optional `locale` override or the
/// surrounding [`ArsProvider`](crate::ArsProvider) locale.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Leptos #[component] requires props to be passed by value."
)]
pub fn Landmark<T>(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,

    /// Semantic landmark role. Defaults to [`Role::Region`].
    #[prop(optional)]
    role: Option<Role>,

    /// Optional ID of an external element that labels this landmark. When
    /// set and non-empty, `aria-labelledby` takes precedence over the
    /// localized `aria-label`.
    #[prop(optional, into)]
    labelledby_id: Option<Oco<'static, str>>,

    /// Localized messages bundle. When omitted, the adapter resolves
    /// messages from the [`ArsProvider`](crate::ArsProvider) message
    /// registry, falling back to [`Messages::default`] (empty label).
    #[prop(optional)]
    messages: Option<Messages>,

    /// Locale override for resolving the `aria-label`. When absent, the
    /// surrounding [`ArsProvider`](crate::ArsProvider) locale is used.
    #[prop(optional)]
    locale: Option<Locale>,

    /// Consumer class tokens appended to the landmark root. Merges with
    /// whatever class the component itself emits so both reach the rendered
    /// element as a single `class` attribute.
    #[prop(optional, into)]
    class: Option<Oco<'static, str>>,

    /// Children rendered inside the landmark element.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let id = id.map(Oco::into_owned);

    let labelledby_id = labelledby_id.map(Oco::into_owned);

    let class = class.map(Oco::into_owned);

    let resolved_role = role.unwrap_or(Role::Region);

    let mut props = Props::new().role(resolved_role);

    if let Some(id) = id.as_deref() {
        props = props.id(id);
    }

    if let Some(labelledby) = labelledby_id.as_deref() {
        props = props.labelledby_id(labelledby);
    }

    let resolved_locale = resolve_locale(locale.as_ref());
    let resolved_messages = use_messages::<Messages>(messages.as_ref(), Some(&resolved_locale));

    let env = Env {
        locale: resolved_locale,
        ..Env::default()
    };

    let api = Api::new(props, &env, &resolved_messages);

    let is_native = !api.prefers_generic_fallback_element();

    let mut attrs = api.root_attrs(is_native);

    if id.is_none() {
        attrs.set(HtmlAttr::Id, AttrValue::None);
    }

    merge_consumer_class_into(&mut attrs, class.as_deref());

    let attrs = attr_map_to_leptos_inline_attrs(attrs);

    let children = children.into_inner();

    let body = children();

    if is_native {
        Either::Right(match resolved_role {
            Role::Banner => EitherOf7::A(view! { <header {..attrs}>{body}</header> }),
            Role::Navigation => EitherOf7::B(view! { <nav {..attrs}>{body}</nav> }),
            Role::Main => EitherOf7::C(view! { <main {..attrs}>{body}</main> }),
            Role::Complementary => EitherOf7::D(view! { <aside {..attrs}>{body}</aside> }),
            Role::ContentInfo => EitherOf7::E(view! { <footer {..attrs}>{body}</footer> }),
            Role::Form => EitherOf7::F(view! { <form {..attrs}>{body}</form> }),
            Role::Region => EitherOf7::G(view! { <section {..attrs}>{body}</section> }),
            Role::Search => {
                unreachable!("Role::Search is handled by Api::prefers_generic_fallback_element()")
            }
        })
    } else {
        Either::Left(view! { <div {..attrs}>{body}</div> })
    }
}
