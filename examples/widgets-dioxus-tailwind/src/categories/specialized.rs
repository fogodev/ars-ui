use ars_dioxus::prelude::{Translate, t};
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
        section { class: "mt-5 rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
            p { class: "text-sm text-slate-600", {t(SpecializedText::SpecializedPanel)} }
        }
    }
}
