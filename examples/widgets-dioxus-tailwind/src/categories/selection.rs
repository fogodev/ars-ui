use ars_dioxus::prelude::{Translate, t};
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
        section { class: "mt-5 rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
            p { class: "text-sm text-slate-600", {t(SelectionText::SelectionPanel)} }
        }
    }
}
