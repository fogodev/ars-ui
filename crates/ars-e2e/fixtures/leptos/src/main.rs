//! Leptos E2E fixture entry point.
//!
//! Each component category lives in its own module under [`categories`]
//! (e.g. `categories::utility`, `categories::navigation`). Add a new
//! category by creating `categories/<name>.rs`, declaring `pub mod <name>;`
//! in `categories/mod.rs`, calling its `register_messages` from
//! `i18n_registries`, and adding a [`CategoryTab`] variant + [`Tab::new`]
//! row to the top-level `<Tabs>` below.

mod categories;

use ars_leptos::{
    ArsProvider,
    navigation::tabs,
    prelude::{Locale, TabKey, Translate},
};
use leptos::{mount::mount_to_body, prelude::*};

use crate::categories::{
    i18n_registries, input::InputPanel, navigation::NavigationPanel, utility::UtilityPanel,
};

fn main() {
    mount_to_body(App);
}

/// Top-level fixture tabs — one variant per component category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey, Translate)]
#[tab_key(ordinal)]
#[translate(fallback = "en-US")]
enum CategoryTab {
    #[translate(en_US = "Input", pt_BR = "Entrada")]
    Input,

    #[translate(en_US = "Navigation", pt_BR = "Navegação")]
    Navigation,

    #[translate(en_US = "Utility", pt_BR = "Utilitários")]
    Utility,
}

#[component]
fn App() -> impl IntoView {
    let locale = RwSignal::new(Locale::parse("en-US").expect("valid fixture locale"));

    view! {
        <ArsProvider locale=locale i18n_registries=i18n_registries()>
            <main class="e2e-shell">
                <h1>"ars-ui Leptos E2E fixture"</h1>
                <div class="locale-controls" aria-label="Fixture locale">
                    <button
                        id="leptos-fixture-locale-en"
                        type="button"
                        on:click=move |_| {
                            locale.set(Locale::parse("en-US").expect("valid fixture locale"));
                        }
                    >
                        "en-US"
                    </button>
                    <button
                        id="leptos-fixture-locale-pt"
                        type="button"
                        on:click=move |_| {
                            locale.set(Locale::parse("pt-BR").expect("valid fixture locale"));
                        }
                    >
                        "pt-BR"
                    </button>
                </div>
                <tabs::Root
                    default_value=CategoryTab::Utility
                    tabs=[
                        tabs::Tab::new(CategoryTab::Input, InputPanel),
                        tabs::Tab::new(CategoryTab::Navigation, NavigationPanel),
                        tabs::Tab::new(CategoryTab::Utility, UtilityPanel),
                    ]
                >
                    <tabs::List<CategoryTab> />
                    <tabs::Panels<CategoryTab> />
                    <tabs::LiveRegion />
                </tabs::Root>
            </main>
        </ArsProvider>
    }
}
