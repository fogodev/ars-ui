use ars_leptos::prelude::{Translate, t};
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
        <section class="empty-category">
            <p>{t(SelectionText::SelectionPanel)}</p>
        </section>
    }
}
