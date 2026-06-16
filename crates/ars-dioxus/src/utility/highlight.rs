//! Dioxus `Highlight` adapter.
//!
//! Renders the framework-agnostic [`Highlight`](ars_components::utility::highlight)
//! attribute contract as a `<span>` root that wraps alternating
//! highlighted / non-highlighted chunks. Highlighted chunks render as
//! `<mark>`; non-highlighted chunks render as `<span>`. Locale resolves
//! from the optional `locale` prop or the surrounding
//! [`ArsProvider`](crate::ArsProvider) context.

pub use ars_components::utility::highlight::{Api, HighlightChunk, MatchStrategy, Part, Props};
use ars_i18n::Locale;
use dioxus::prelude::*;

use crate::{attr_map_to_dioxus_inline_attrs, merge_dioxus_attrs, use_locale};

/// Props for the Dioxus [`Highlight`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct HighlightProps {
    /// Search queries to highlight. An empty or all-empty input emits a
    /// single non-highlighted chunk covering the full text.
    pub query: Vec<String>,

    /// Source text to split into highlighted / non-highlighted chunks.
    #[props(into)]
    pub text: String,

    /// When `true` (default), matching is case-insensitive via locale-aware
    /// Unicode case folding.
    #[props(default = true)]
    pub ignore_case: bool,

    /// Match strategy. Defaults to [`MatchStrategy::Contains`].
    #[props(default = MatchStrategy::Contains)]
    pub match_strategy: MatchStrategy,

    /// Optional locale override. When absent, the locale is sourced from the
    /// surrounding [`ArsProvider`](crate::ArsProvider) context.
    #[props(optional)]
    pub locale: Option<Locale>,

    /// Global HTML attributes forwarded onto the rendered root `<span>`.
    /// Tokenized attributes (`class`, `style`, relationship token lists)
    /// concatenate with the component's own values; ordinary attributes prefer
    /// the component's value on conflict so the bidi `dir="auto"` and
    /// `data-ars-*` scope/part attrs stay intact.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,
}

/// Dioxus `Highlight` component.
///
/// Splits `text` into chunks according to the configured `query`,
/// `ignore_case`, and `match_strategy`, rendering each chunk as either a
/// `<mark>` (highlighted) or `<span>` (unmatched). The root `<span>`
/// receives `data-ars-scope="highlight"`, `data-ars-part="root"`, and
/// `dir="auto"` from the agnostic core.
#[component]
pub fn Highlight(props: HighlightProps) -> Element {
    let core_props = Props::new()
        .query(props.query)
        .text(props.text)
        .ignore_case(props.ignore_case)
        .match_strategy(props.match_strategy);

    let api = Api::new(core_props);

    let locale = props.locale.unwrap_or_else(|| use_locale().read().clone());

    let component_root_attrs = attr_map_to_dioxus_inline_attrs(api.root_attrs());

    let root_attrs = merge_dioxus_attrs(props.attrs, component_root_attrs);

    let chunks = api.chunks(&locale).into_iter().map(|chunk| {
        let chunk_attrs = attr_map_to_dioxus_inline_attrs(api.chunk_attrs(chunk.highlighted));

        let text = chunk.text.to_owned();

        if chunk.highlighted {
            rsx! {
                mark { ..chunk_attrs,"{text}" }
            }
        } else {
            rsx! {
                span { ..chunk_attrs,"{text}" }
            }
        }
    });

    rsx! {
        span { ..root_attrs,{chunks} }
    }
}
