mod categories;
mod locale;
mod messages;
mod text;

use ars_dioxus::{ArsProvider, prelude::t};
use dioxus::prelude::*;

use crate::{categories::CategoryTabs, locale::{LocaleSwitcher, parse_locale}, text::WidgetsText};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let locale = use_signal(|| parse_locale("en-US"));

    rsx! {
        ArsProvider { locale, i18n_registries: messages::i18n_registries(),
            main { class: "widgets-page",
                h1 { {t(WidgetsText::DioxusTitle)} }
                p { {t(WidgetsText::PageSummary)} }
                LocaleSwitcher { locale }
                CategoryTabs {}
            }
        }
    }
}
