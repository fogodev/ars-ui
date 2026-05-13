use ars_dioxus::prelude::{Locale, t};
use dioxus::prelude::*;

use crate::text::WidgetsText;

pub(crate) fn parse_locale(tag: &str) -> Locale {
    Locale::parse(tag).expect("example locale should parse")
}

#[component]
pub(crate) fn LocaleSwitcher(locale: Signal<Locale>) -> Element {
    let current = locale();
    let is_en = current.to_bcp47() == "en-US";
    let is_pt = current.to_bcp47() == "pt-BR";
    let mut en_locale = locale;
    let mut pt_locale = locale;

    rsx! {
        div { class: "locale-switcher",
            span { class: "locale-label", {t(WidgetsText::LocaleLabel)} }
            button {
                r#type: "button",
                class: if is_en { "locale-button selected" } else { "locale-button" },
                "aria-pressed": "{is_en}",
                onclick: move |_| en_locale.set(parse_locale("en-US")),
                {t(WidgetsText::LocaleEnglish)}
            }
            button {
                r#type: "button",
                class: if is_pt { "locale-button selected" } else { "locale-button" },
                "aria-pressed": "{is_pt}",
                onclick: move |_| pt_locale.set(parse_locale("pt-BR")),
                {t(WidgetsText::LocalePortuguese)}
            }
        }
    }
}
