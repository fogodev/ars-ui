use ars_dioxus::prelude::{Translate, t};
use dioxus::prelude::*;

#[derive(Clone, Debug, Translate, PartialEq)]
#[translate(fallback = "en-US")]
pub(crate) enum InputText {
    #[translate(
        en_US = "Input components - text-field, checkbox, slider, number-input, etc. Coming soon.",
        pt_BR = "Componentes de entrada - campo de texto, checkbox, slider, entrada numérica etc. Em breve."
    )]
    InputPanel,
}

#[component]
pub(crate) fn InputPanel() -> Element {
    rsx! {
        section { class: "mt-5 rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
            p { class: "text-sm text-slate-600", {t(InputText::InputPanel)} }
        }
    }
}
