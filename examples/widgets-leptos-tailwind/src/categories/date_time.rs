use ars_leptos::prelude::{Translate, t};
use leptos::prelude::*;

#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum DateTimeText {
    #[translate(
        en_US = "Date and time components - date-field, time-field, calendar, date-picker, etc. Coming soon.",
        pt_BR = "Componentes de data e hora - date-field, time-field, calendar, date-picker etc. Em breve."
    )]
    DateTimePanel,
}

#[component]
pub(crate) fn DateTimePanel() -> impl IntoView {
    view! {
        <section class="p-5 mt-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10">
            <p class="text-sm text-slate-600">{t(DateTimeText::DateTimePanel)}</p>
        </section>
    }
}
