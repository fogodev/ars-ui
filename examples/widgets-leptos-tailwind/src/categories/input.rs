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
        <section class="p-5 mt-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10">
            <p class="text-sm text-slate-600">{t(InputText::InputPanel)}</p>
        </section>
    }
}
