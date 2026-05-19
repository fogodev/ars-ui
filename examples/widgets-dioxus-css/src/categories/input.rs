use ars_dioxus::prelude::{t, Translate};
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
        section { class: "showcase-panel wide empty-category",
            p { {t(InputText::InputPanel)} }
        }
    }
}
