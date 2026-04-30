//! Error boundary wrapper.
//!
//! [`Boundary`] wraps a subtree in Dioxus's [`ErrorBoundary`] and renders the
//! canonical accessible fallback (`<div role="alert">` with a localized
//! heading and `<ul>`/`<li>` error list) defined in
//! `spec/components/utility/error-boundary.md`. It composes around the
//! framework primitive: locale resolution, message bundle resolution, and
//! conversion of Dioxus's single `Option<CapturedError>` into the canonical
//! `<ul>`-of-`<li>` shape so the rendered DOM matches the Leptos adapter
//! byte-for-byte.
//!
//! See `spec/components/utility/error-boundary.md` and
//! `spec/dioxus-components/utility/error-boundary.md` for the full
//! specification.

pub use ars_components::utility::error_boundary::{Api, Messages, Part};
use ars_i18n::Locale;
pub use dioxus::CapturedError;
use dioxus::prelude::*;

use crate::{
    attrs::attr_map_to_dioxus_inline_attrs,
    provider::{resolve_locale, use_messages},
};

// ────────────────────────────────────────────────────────────────────
// FallbackHandler
// ────────────────────────────────────────────────────────────────────

/// Adapter-side fallback handler.
///
/// A thin alias over [`Callback<ErrorContext, Element>`] so consumers can
/// pass a closure directly to [`BoundaryProps::fallback`] without spelling
/// out the generic parameters.
pub type FallbackHandler = Callback<ErrorContext, Element>;

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Props for [`Boundary`].
#[derive(Props, Clone, Debug, PartialEq)]
pub struct BoundaryProps {
    /// Subtree wrapped by the boundary.
    pub children: Element,

    /// Optional override for the entire fallback closure.
    ///
    /// When `None`, the wrapper renders the canonical accessible default
    /// markup defined in `spec/components/utility/error-boundary.md`. When
    /// `Some`, the closure receives Dioxus's [`ErrorContext`] and is
    /// responsible for rendering its own UI; none of the canonical
    /// `data-ars-*` attributes are emitted in that branch.
    #[props(optional)]
    pub fallback: Option<FallbackHandler>,

    /// Optional telemetry hook fired with each captured error.
    ///
    /// Fired before the fallback renders. Consumers can forward the
    /// [`CapturedError`] to monitoring services (Sentry, Datadog, …).
    #[props(optional)]
    pub on_error: Option<EventHandler<CapturedError>>,

    /// Override the default [`Messages`] bundle.
    ///
    /// When `None`, the wrapper resolves the bundle from `ArsProvider`'s
    /// `i18n_registries` for the active locale, falling back to
    /// [`Messages::default`] (English `"A component encountered an error."`).
    #[props(optional)]
    pub messages: Option<Messages>,
}

// ────────────────────────────────────────────────────────────────────
// Boundary
// ────────────────────────────────────────────────────────────────────

/// Wrapper around Dioxus's [`ErrorBoundary`] that renders an accessible
/// fallback when a descendant component returns `Err`.
///
/// The fallback is a `<div role="alert" data-ars-error="true">` containing
/// a localized heading paragraph and a `<ul>` of `<li>` error entries —
/// matching the Leptos adapter byte-for-byte. Optional props expose a
/// custom fallback override, an `on_error` telemetry hook, and a
/// `messages` bundle override.
///
/// See `spec/dioxus-components/utility/error-boundary.md` for the full
/// adapter contract.
#[component]
pub fn Boundary(props: BoundaryProps) -> Element {
    let BoundaryProps {
        children,
        fallback,
        on_error,
        messages,
    } = props;

    let resolved_locale = resolve_locale(None);

    let resolved_messages = use_messages(messages.as_ref(), Some(&resolved_locale));

    let heading = (resolved_messages.message)(&resolved_locale);

    rsx! {
        ErrorBoundary {
            handle_error: move |ctx: ErrorContext| {
                if let (Some(error), Some(handler)) = (ctx.error(), on_error.as_ref()) {
                    handler.call(error);
                }
                if let Some(custom) = fallback.as_ref() {
                    custom.call(ctx)
                } else {
                    render_default_fallback(&ctx, &heading)
                }
            },
            {children}
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// default_fallback
// ────────────────────────────────────────────────────────────────────

/// Renders the canonical accessible fallback markup using English defaults.
///
/// Use this when consuming Dioxus's [`ErrorBoundary`] directly without
/// the [`Boundary`] wrapper:
///
/// ```ignore
/// rsx! {
///     ErrorBoundary {
///         handle_error: ars_dioxus::error_boundary::default_fallback,
///         ChildComponent {}
///     }
/// }
/// ```
///
/// For localized headings, use [`Boundary`] which resolves [`Messages`]
/// from the surrounding `ArsProvider`. This standalone function intentionally
/// does **not** read any reactive context (it cannot, since it is not a
/// component) and always falls back to English.
///
/// # Errors
///
/// Returns the same `Result<VNode, RenderError>` shape as any Dioxus
/// component. The default markup is statically valid; an `Err` would only
/// arise from internal Dioxus `VNode` construction.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus `ErrorBoundary::handle_error` requires a `Fn(ErrorContext) -> Element` \
              signature, so the parameter must be owned even though we only borrow it."
)]
pub fn default_fallback(ctx: ErrorContext) -> Element {
    let messages = Messages::default();

    let locale = en_us_locale();

    let heading = (messages.message)(&locale);

    render_default_fallback(&ctx, &heading)
}

fn en_us_locale() -> Locale {
    Locale::parse("en-US").expect("'en-US' is always a valid BCP-47 locale")
}

fn render_default_fallback(ctx: &ErrorContext, heading: &str) -> Element {
    let error = ctx.error();
    let api = Api::new(usize::from(error.is_some()));

    let root_attrs = attr_map_to_dioxus_inline_attrs(api.root_attrs());
    let message_attrs = attr_map_to_dioxus_inline_attrs(api.message_attrs());
    let list_attrs = attr_map_to_dioxus_inline_attrs(api.list_attrs());

    rsx! {
        div {..root_attrs,
            p { ..message_attrs,"{heading}" }
            ul {..list_attrs,
                if let Some(e) = error {
                    li { ..attr_map_to_dioxus_inline_attrs(api.item_attrs()),"{e}" }
                }
            }
        }
    }
}
