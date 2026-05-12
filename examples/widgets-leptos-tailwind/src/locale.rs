use ars_leptos::prelude::{Locale, t};
use leptos::prelude::*;

use crate::text::WidgetsText;

pub(crate) fn parse_locale(tag: &str) -> Locale {
    Locale::parse(tag).expect("example locale should parse")
}

#[component]
pub(crate) fn LocaleSwitcher(locale: RwSignal<Locale>) -> impl IntoView {
    let set_en = move |_| locale.set(parse_locale("en-US"));
    let set_pt = move |_| locale.set(parse_locale("pt-BR"));
    let is_en = move || locale.get().to_bcp47() == "en-US";
    let is_pt = move || locale.get().to_bcp47() == "pt-BR";

    view! {
        <div class="locale-switcher">
            <span class="locale-label">{t(WidgetsText::LocaleLabel)}</span>
            <button
                type="button"
                class=move || if is_en() { "locale-button selected" } else { "locale-button" }
                aria-pressed=move || is_en().to_string()
                on:click=set_en
            >
                {t(WidgetsText::LocaleEnglish)}
            </button>
            <button
                type="button"
                class=move || if is_pt() { "locale-button selected" } else { "locale-button" }
                aria-pressed=move || is_pt().to_string()
                on:click=set_pt
            >
                {t(WidgetsText::LocalePortuguese)}
            </button>
        </div>
    }
}
