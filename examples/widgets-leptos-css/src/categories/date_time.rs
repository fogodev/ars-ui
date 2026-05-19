use ars_leptos::prelude::{t, Translate};
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
        <section class="showcase-panel wide empty-category">
            <p>{t(DateTimeText::DateTimePanel)}</p>
        </section>
    }
}
