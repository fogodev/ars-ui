use ars_dioxus::prelude::{t, Translate};
use dioxus::prelude::*;

#[derive(Clone, Debug, Translate, PartialEq)]
#[translate(fallback = "en-US")]
pub(crate) enum SpecializedText {
    #[translate(
        en_US = "Specialized components - color-picker, file-upload, signature-pad, qr-code, etc. Coming soon.",
        pt_BR = "Componentes especializados - color-picker, file-upload, signature-pad, qr-code etc. Em breve."
    )]
    SpecializedPanel,
}

#[component]
pub(crate) fn SpecializedPanel() -> Element {
    rsx! {
        section { class: "showcase-panel wide empty-category",
            p { {t(SpecializedText::SpecializedPanel)} }
        }
    }
}
