use ars_dioxus::prelude::{Translate, t};
use dioxus::prelude::*;

#[derive(Clone, Debug, Translate, PartialEq)]
#[translate(fallback = "en-US")]
pub(crate) enum OverlayText {
    #[translate(
        en_US = "Overlay components - dialog, popover, tooltip, toast, presence, etc. Coming soon.",
        pt_BR = "Componentes de sobreposição - dialog, popover, tooltip, toast, presence etc. Em breve."
    )]
    OverlayPanel,
}

#[component]
pub(crate) fn OverlayPanel() -> Element {
    rsx! {
        section { class: "empty-category",
            p { {t(OverlayText::OverlayPanel)} }
        }
    }
}
