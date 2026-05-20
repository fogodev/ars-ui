//! Dioxus `ClientOnly` adapter.
//!
//! Gates children until effects run on the client while preserving SSR fallback
//! markup and avoiding adapter-owned wrapper nodes.

pub use ars_components::utility::client_only::Props;
use dioxus::prelude::*;

/// Props for the Dioxus [`ClientOnly`] component.
#[derive(dioxus::prelude::Props, Clone, PartialEq, Debug)]
pub struct ClientOnlyProps {
    /// Optional fallback rendered during SSR and before client mount.
    #[props(optional)]
    pub fallback: Option<Element>,

    /// Client-only children rendered after mount.
    pub children: Element,
}

/// Dioxus logical boundary that renders children only after client mount.
#[component]
pub fn ClientOnly(props: ClientOnlyProps) -> Element {
    let mut mounted = use_signal(|| false);

    use_effect(move || mounted.set(true));

    if mounted() {
        rsx! {
            {props.children}
        }
    } else {
        rsx! {
            {props.fallback}
        }
    }
}
