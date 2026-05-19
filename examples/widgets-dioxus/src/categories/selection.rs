use ars_dioxus::prelude::{t, Translate};
use dioxus::prelude::*;

#[derive(Clone, Debug, Translate, PartialEq)]
#[translate(fallback = "en-US")]
pub(crate) enum SelectionText {
    #[translate(
        en_US = "Selection components - select, combobox, listbox, menu, tags-input, etc. Coming soon.",
        pt_BR = "Componentes de seleção - select, combobox, listbox, menu, tags-input etc. Em breve."
    )]
    SelectionPanel,
}

#[component]
pub(crate) fn SelectionPanel() -> Element {
    rsx! {
        section { class: "empty-category",
            p { {t(SelectionText::SelectionPanel)} }
        }
    }
}
