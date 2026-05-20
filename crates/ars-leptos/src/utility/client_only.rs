//! Leptos `ClientOnly` adapter.
//!
//! Gates children until the client mount pass while preserving SSR fallback
//! markup and avoiding adapter-owned wrapper nodes.

pub use ars_components::utility::client_only::Props;
use leptos::{
    children::{TypedChildrenFn, ViewFn},
    either::Either,
    prelude::*,
};

/// Leptos logical boundary that renders children only after client mount.
#[component]
pub fn ClientOnly<C>(
    /// Optional fallback rendered during SSR and before client mount.
    #[prop(optional, into)]
    fallback: ViewFn,

    /// Client-only children rendered after mount.
    children: TypedChildrenFn<C>,
) -> impl IntoView
where
    C: IntoView + 'static,
{
    let mounted = RwSignal::new(false);

    #[cfg(not(feature = "ssr"))]
    Effect::new(move |_| mounted.set(true));

    let children = children.into_inner();

    move || {
        if mounted.get() {
            Either::Left(children())
        } else {
            Either::Right(fallback.run())
        }
    }
}
