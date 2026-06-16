mod categories;
mod locale;
mod messages;
mod text;

use ars_dioxus::prelude::{ArsProvider, t};
use dioxus::prelude::*;

use crate::{
    categories::CategoryTabs,
    locale::{LocaleSwitcher, parse_locale},
    text::WidgetsText,
};

const ARS_BASE_STYLE: &str = include_str!("../public/ars-base.css");
const ARS_INTERACTIONS_STYLE: &str = include_str!("../public/ars-interactions.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let locale = use_signal(|| parse_locale("en-US"));

    rsx! {
        ArsProvider { locale, i18n_registries: messages::i18n_registries(),
            style { "{ARS_BASE_STYLE}" }
            style { "{ARS_INTERACTIONS_STYLE}" }
            main { class: "widgets-page",
                h1 { {t(WidgetsText::DioxusTitle)} }
                p { {t(WidgetsText::PageSummary)} }
                LocaleSwitcher { locale }
                CategoryTabs {}
            }
        }
    }
}
