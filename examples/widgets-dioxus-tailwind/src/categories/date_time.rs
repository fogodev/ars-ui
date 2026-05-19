use ars_dioxus::prelude::{t, Translate};
use dioxus::prelude::*;

#[derive(Clone, Debug, Translate, PartialEq)]
#[translate(fallback = "en-US")]
pub(crate) enum DateTimeText {
    #[translate(
        en_US = "Date and time components - date-field, time-field, calendar, date-picker, etc. Coming soon.",
        pt_BR = "Componentes de data e hora - date-field, time-field, calendar, date-picker etc. Em breve."
    )]
    DateTimePanel,
}

#[component]
pub(crate) fn DateTimePanel() -> Element {
    rsx! {
        section { class: "mt-5 rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
            p { class: "text-sm text-slate-600", {t(DateTimeText::DateTimePanel)} }
        }
    }
}
