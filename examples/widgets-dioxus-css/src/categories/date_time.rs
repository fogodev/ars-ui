use ars_dioxus::prelude::{Translate, t};
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
        section { class: "showcase-panel wide empty-category",
            p { {t(DateTimeText::DateTimePanel)} }
        }
    }
}
