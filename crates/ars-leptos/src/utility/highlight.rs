//! Leptos `Highlight` adapter.
//!
//! Renders the framework-agnostic [`Highlight`](ars_components::utility::highlight)
//! attribute contract as a `<span>` root that wraps alternating
//! highlighted / non-highlighted chunks. Highlighted chunks render as
//! `<mark>`; non-highlighted chunks render as `<span>`. Locale resolves
//! from the optional `locale` prop or the surrounding
//! [`ArsProvider`](crate::ArsProvider) context.

pub use ars_components::utility::highlight::{Api, HighlightChunk, MatchStrategy, Part, Props};
use ars_i18n::Locale;
use leptos::{either::Either, prelude::*};

use crate::{attr_map_to_leptos_inline_attrs, merge_consumer_class_into, use_locale};

/// Leptos `Highlight` component.
///
/// Splits `text` into chunks according to the configured `query`,
/// `ignore_case`, and `match_strategy`, rendering each chunk as either a
/// `<mark>` (highlighted) or `<span>` (unmatched). The root `<span>`
/// receives `data-ars-scope="highlight"`, `data-ars-part="root"`, and
/// `dir="auto"` from the agnostic core.
#[component]
pub fn Highlight(
    /// Search queries to highlight. Empty (or all-empty) input emits a
    /// single non-highlighted chunk.
    #[prop(optional, into)]
    query: Vec<String>,

    /// Source text to split into highlighted / non-highlighted chunks.
    #[prop(optional, into)]
    text: String,

    /// When `true` (default), matching is case-insensitive via
    /// locale-aware Unicode case folding.
    #[prop(optional)]
    ignore_case: Option<bool>,

    /// Match strategy. Defaults to [`MatchStrategy::Contains`].
    #[prop(optional)]
    match_strategy: Option<MatchStrategy>,

    /// Optional locale override. When absent, the locale is sourced from
    /// the surrounding [`ArsProvider`](crate::ArsProvider) context.
    #[prop(optional)]
    locale: Option<Locale>,

    /// Consumer class tokens appended to the root `<span>`. Merges with
    /// any class the component itself emits so both reach the rendered
    /// element as a single `class` attribute.
    #[prop(optional, into)]
    class: Option<Oco<'static, str>>,
) -> impl IntoView {
    let class = class.map(Oco::into_owned);

    let props = Props::new()
        .query(query)
        .text(text)
        .ignore_case(ignore_case.unwrap_or(true))
        .match_strategy(match_strategy.unwrap_or_default());

    let api = Api::new(props);

    let locale = locale.unwrap_or_else(|| use_locale().get());

    let mut root_attr_map = api.root_attrs();

    merge_consumer_class_into(&mut root_attr_map, class.as_deref());

    let root_attrs = attr_map_to_leptos_inline_attrs(root_attr_map);

    let chunks = api
        .chunks(&locale)
        .into_iter()
        .map(|chunk| {
            let chunk_attrs = attr_map_to_leptos_inline_attrs(api.chunk_attrs(chunk.highlighted));

            let text = chunk.text.to_owned();

            if chunk.highlighted {
                Either::Left(view! { <mark {..chunk_attrs}>{text}</mark> })
            } else {
                Either::Right(view! { <span {..chunk_attrs}>{text}</span> })
            }
        })
        .collect_view();

    view! { <span {..root_attrs}>{chunks}</span> }
}
