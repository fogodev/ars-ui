mod categories;
mod locale;
mod messages;
mod text;

use ars_leptos::prelude::{ArsProvider, t};
use leptos::{mount::mount_to_body, prelude::*};

use crate::{
    categories::CategoryTabs,
    locale::{LocaleSwitcher, parse_locale},
    text::WidgetsText,
};

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let locale = RwSignal::new(parse_locale("en-US"));

    view! {
        <ArsProvider locale i18n_registries=messages::i18n_registries()>
            <main class="py-10 px-5 mx-auto max-w-6xl min-h-screen md:px-8">
                <p class="mb-2 text-xs font-extrabold tracking-wider text-blue-700 uppercase">
                    {t(WidgetsText::TailwindStyling)}
                </p>
                <h1 class="max-w-3xl text-4xl font-extrabold leading-tight text-slate-950">
                    {t(WidgetsText::LeptosTitle)}
                </h1>
                <p class="mt-3 max-w-3xl text-base leading-7 text-slate-600">
                    {t(WidgetsText::PageSummary)}
                </p>
                <LocaleSwitcher locale />
                <div class="mt-8">
                    <CategoryTabs />
                </div>
            </main>
        </ArsProvider>
    }
}
