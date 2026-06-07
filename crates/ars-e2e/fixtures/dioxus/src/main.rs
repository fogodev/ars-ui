//! Dioxus E2E fixture entry point.
//!
//! Each component category lives in its own module under [`categories`]
//! (e.g. `categories::utility`, `categories::navigation`). Add a new
//! category by creating `categories/<name>.rs`, declaring `pub mod <name>;`
//! in `categories/mod.rs`, calling its `register_messages` from
//! `i18n_registries`, and adding a [`CategoryTab`] variant + [`Tab::new`]
//! row to the top-level `<Tabs>` below.

mod categories;

use ars_dioxus::{
    ArsProvider,
    navigation::tabs::{Tab, Tabs},
    prelude::{Locale, TabKey, Translate},
};
use dioxus::prelude::*;

use crate::categories::{i18n_registries, navigation::NavigationPanel, utility::UtilityPanel};

fn main() {
    dioxus::launch(App);
}

/// Top-level fixture tabs — one variant per component category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey, Translate)]
#[tab_key(ordinal)]
#[translate(fallback = "en-US")]
enum CategoryTab {
    #[translate(en_US = "Navigation", pt_BR = "Navegação")]
    Navigation,

    #[translate(en_US = "Utility", pt_BR = "Utilitários")]
    Utility,
}

#[component]
fn App() -> Element {
    let mut locale = use_signal(|| Locale::parse("en-US").expect("valid fixture locale"));
    let locale_key = locale.read().to_bcp47();

    rsx! {
        ArsProvider { locale, i18n_registries: i18n_registries(),
            main { class: "e2e-shell",
                h1 { "ars-ui Dioxus E2E fixture" }
                div { class: "locale-controls", "aria-label": "Fixture locale",
                    button {
                        id: "dioxus-fixture-locale-en",
                        r#type: "button",
                        onclick: move |_| locale.set(Locale::parse("en-US").expect("valid fixture locale")),
                        "en-US"
                    }
                    button {
                        id: "dioxus-fixture-locale-pt",
                        r#type: "button",
                        onclick: move |_| locale.set(Locale::parse("pt-BR").expect("valid fixture locale")),
                        "pt-BR"
                    }
                }
                Tabs {
                    default_value: CategoryTab::Utility,
                    tabs: [
                        Tab::new(CategoryTab::Navigation, NavigationPanel()),
                        Tab::new(CategoryTab::Utility, rsx! { UtilityPanel { locale_key } }),
                    ],
                }
            }
        }
    }
}
