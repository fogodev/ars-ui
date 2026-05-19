use ars_leptos::{ArsProvider, prelude::t};
use leptos::{mount::mount_to_body, prelude::*};

mod categories;
mod locale;
mod messages;
mod text;

use crate::{categories::CategoryTabs, locale::{LocaleSwitcher, parse_locale}, text::WidgetsText};

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let locale = RwSignal::new(parse_locale("en-US"));

    view! {
        <ArsProvider locale=locale i18n_registries=messages::i18n_registries()>
            <main class="widgets-page">
                <p class="page-kicker">{t(WidgetsText::CssStyling)}</p>
                <h1>{t(WidgetsText::LeptosTitle)}</h1>
                <p class="page-summary">{t(WidgetsText::PageSummary)}</p>
                <LocaleSwitcher locale />
                <CategoryTabs />
            </main>
        </ArsProvider>
    }
}
