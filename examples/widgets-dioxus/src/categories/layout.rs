use ars_dioxus::prelude::{Translate, t};
use dioxus::prelude::*;

#[derive(Clone, Debug, Translate, PartialEq)]
#[translate(fallback = "en-US")]
pub(crate) enum LayoutText {
    #[translate(
        en_US = "Layout components - splitter, scroll-area, carousel, portal, toolbar, etc. Coming soon.",
        pt_BR = "Componentes de layout - splitter, scroll-area, carousel, portal, toolbar etc. Em breve."
    )]
    LayoutPanel,
}

#[component]
pub(crate) fn LayoutPanel() -> Element {
    rsx! {
        section { class: "empty-category",
            p { {t(LayoutText::LayoutPanel)} }
        }
    }
}
