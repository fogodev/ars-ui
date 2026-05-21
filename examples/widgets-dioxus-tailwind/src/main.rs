use ars_dioxus::prelude::{ArsProvider, t};
use dioxus::prelude::*;

mod categories;
mod locale;
mod messages;
mod text;

use crate::{
    categories::CategoryTabs,
    locale::{LocaleSwitcher, parse_locale},
    text::WidgetsText,
};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let locale = use_signal(|| parse_locale("en-US"));

    rsx! {
        ArsProvider { locale, i18n_registries: messages::i18n_registries(),
            main { class: "mx-auto min-h-screen max-w-6xl px-5 py-10 md:px-8",
                p { class: "mb-2 text-xs font-extrabold uppercase tracking-wider text-blue-700",
                    {t(WidgetsText::TailwindStyling)}
                }
                h1 { class: "max-w-3xl text-4xl font-extrabold leading-tight text-slate-950",
                    {t(WidgetsText::DioxusTitle)}
                }
                p { class: "mt-3 max-w-3xl text-base leading-7 text-slate-600",
                    {t(WidgetsText::PageSummary)}
                }
                LocaleSwitcher { locale }
                div { class: "mt-8", CategoryTabs {} }
            }
        }
    }
}
