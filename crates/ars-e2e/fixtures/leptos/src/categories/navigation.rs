//! Navigation-category fixture module.
//!
//! Owns the nested `<Tabs>` showcase (Overview / Keyboard / Closable /
//! Disabled), the per-category text enum, and the message-registry entry
//! for `tabs::Messages`.

use ars_leptos::{
    I18nRegistries, MessageFn, MessagesRegistry,
    navigation::tabs,
    prelude::{Locale, TabKey, Translate, t},
};
use leptos::prelude::*;

/// Tab keys used inside the Navigation showcase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey, Translate)]
#[tab_key(ordinal)]
#[translate(fallback = "en-US")]
pub(crate) enum NavigationTab {
    #[translate(en_US = "Overview", pt_BR = "Visão geral")]
    Overview,

    #[translate(en_US = "Keyboard", pt_BR = "Teclado")]
    Keyboard,

    #[translate(en_US = "Closable", pt_BR = "Fechável")]
    Closable,

    #[translate(en_US = "Disabled", pt_BR = "Desabilitada")]
    Disabled,
}

/// Localized strings used by the navigation panel.
#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum NavigationText {
    #[translate(
        en_US = "Arrow keys move focus across tabs (loop_focus on by default).",
        pt_BR = "As setas movem o foco entre as abas."
    )]
    KeyboardArrowKeys,

    #[translate(
        en_US = "Home / End jump to the first / last enabled tab.",
        pt_BR = "Home / End pulam para a primeira / última aba habilitada."
    )]
    KeyboardHomeEnd,

    #[translate(
        en_US = "Drag tabs to reorder them, or use Ctrl + Arrow keys.",
        pt_BR = "Arraste abas para reordená-las ou use Ctrl + setas."
    )]
    KeyboardReorder,

    #[translate(
        en_US = "Closable tabs render an extra close button and accept Delete / Backspace to fire CloseTab.",
        pt_BR = "Abas fecháveis renderizam um botão extra de fechar e aceitam Delete / Backspace para disparar CloseTab."
    )]
    ClosablePanel,

    #[translate(
        en_US = "Disabled tabs stay rendered but are skipped by selection, keyboard focus, and drag reorder.",
        pt_BR = "Abas desabilitadas permanecem renderizadas, mas são ignoradas por seleção, foco por teclado e reordenação por arraste."
    )]
    DisabledPanel,
}

/// Registers the navigation category's localized message bundles with the
/// fixture's shared `I18nRegistries`.
pub(crate) fn register_messages(registries: &mut I18nRegistries) {
    registries.register(MessagesRegistry::new(tabs::Messages::default()).register(
        "pt-BR",
        tabs::Messages {
            close_tab_label: MessageFn::new(|label: &str, _locale: &Locale| {
                format!("Fechar {label}")
            }),
            reorder_announce_label: MessageFn::new(
                |label: &str, position: usize, total: usize, _locale: &Locale| {
                    format!("Aba {label} movida para a posição {position} de {total}")
                },
            ),
        },
    ));
}

/// Navigation-category showcase panel.
#[component]
pub(crate) fn NavigationPanel() -> impl IntoView {
    view! {
        <section class="showcase-panel wide">
            <tabs::Root
                default_value=NavigationTab::Overview
                tabs=[
                    tabs::Tab::new(NavigationTab::Overview, || view! { <p>"Tabs fixture overview."</p> }),
                    tabs::Tab::new(
                            NavigationTab::Keyboard,
                            || {
                                view! {
                                    <ul>
                                        <li>{t(NavigationText::KeyboardArrowKeys)}</li>
                                        <li>{t(NavigationText::KeyboardHomeEnd)}</li>
                                        <li>{t(NavigationText::KeyboardReorder)}</li>
                                    </ul>
                                }
                            },
                        )
                        .closable(true),
                    tabs::Tab::new(
                            NavigationTab::Closable,
                            || view! { <p>{t(NavigationText::ClosablePanel)}</p> },
                        )
                        .closable(true),
                    tabs::Tab::new(
                            NavigationTab::Disabled,
                            || view! { <p>{t(NavigationText::DisabledPanel)}</p> },
                        )
                        .disabled(true),
                ]
                reorderable=true
            >
                <tabs::List<NavigationTab> />
                <tabs::Panels<NavigationTab> />
                <tabs::LiveRegion />
            </tabs::Root>
        </section>
    }
}
