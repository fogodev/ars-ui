use ars_leptos::prelude::{t, Translate};
use leptos::prelude::*;

#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum InputText {
    #[translate(
        en_US = "Input components - text-field, checkbox, slider, number-input, etc. Coming soon.",
        pt_BR = "Componentes de entrada - campo de texto, checkbox, slider, entrada numérica etc. Em breve."
    )]
    InputPanel,
}

#[component]
pub(crate) fn InputPanel() -> impl IntoView {
    view! {
        <section class="empty-category">
            <p>{t(InputText::InputPanel)}</p>
        </section>
    }
}
