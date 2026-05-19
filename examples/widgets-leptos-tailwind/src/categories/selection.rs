use ars_leptos::prelude::{t, Translate};
use leptos::prelude::*;

#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum SelectionText {
    #[translate(
        en_US = "Selection components - select, combobox, listbox, menu, tags-input, etc. Coming soon.",
        pt_BR = "Componentes de seleção - select, combobox, listbox, menu, tags-input etc. Em breve."
    )]
    SelectionPanel,
}

#[component]
pub(crate) fn SelectionPanel() -> impl IntoView {
    view! {
        <section class="p-5 mt-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10">
            <p class="text-sm text-slate-600">{t(SelectionText::SelectionPanel)}</p>
        </section>
    }
}
