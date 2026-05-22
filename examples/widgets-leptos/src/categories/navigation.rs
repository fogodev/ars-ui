use ars_leptos::{
    navigation::tabs::{Tab, Tabs},
    prelude::{TabKey, Translate, t},
};
use leptos::prelude::*;

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

#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum NavigationText {
    #[translate(en_US = "Tabs", pt_BR = "Abas")]
    TabsHeading,

    #[translate(
        en_US = "Live demo of the Tabs adapter - drag tabs to reorder, close the removable tabs, and inspect the disabled state.",
        pt_BR = "Demo ao vivo do adaptador de abas - arraste abas para reordenar, feche as abas removíveis e inspecione o estado desabilitado."
    )]
    TabsDemoSummary,

    #[translate(
        en_US = "Tabs is the first navigation primitive shipped in this gallery. The category tabs above use the same component.",
        pt_BR = "Abas são o primeiro primitivo de navegação nesta galeria. As abas de categoria acima usam o mesmo componente."
    )]
    TabsOverview,

    #[translate(
        en_US = "Arrow keys move focus across tabs (loop_focus on by default).",
        pt_BR = "As setas movem o foco entre as abas (loop_focus fica ativo por padrão)."
    )]
    KeyboardArrowKeys,

    #[translate(
        en_US = "Home / End jump to the first / last enabled tab.",
        pt_BR = "Home / End pulam para a primeira / última aba habilitada."
    )]
    KeyboardHomeEnd,

    #[translate(
        en_US = "In manual activation mode, Enter / Space activates the focused tab.",
        pt_BR = "No modo de ativação manual, Enter / Espaço ativa a aba focada."
    )]
    KeyboardManualActivation,

    #[translate(
        en_US = "Drag tabs to reorder them, or use Ctrl + Arrow keys.",
        pt_BR = "Arraste abas para reordená-las ou use Ctrl + setas."
    )]
    KeyboardReorder,

    #[translate(
        en_US = "This tab is closable, so Delete / Backspace removes it too.",
        pt_BR = "Esta aba é fechável, então Delete / Backspace também a remove."
    )]
    KeyboardClosable,

    #[translate(
        en_US = "Closable tabs render an extra close button and accept Delete / Backspace to fire CloseTab.",
        pt_BR = "Abas fecháveis renderizam um botão extra de fechar e aceitam Delete / Backspace para disparar CloseTab."
    )]
    ClosablePanel,

    #[translate(
        en_US = "Disabled tabs stay in the DOM for layout parity but are skipped by selection, keyboard focus, and drag reorder.",
        pt_BR = "Abas desabilitadas permanecem no DOM para paridade de leiaute, mas são ignoradas por seleção, foco por teclado e reordenação por arraste."
    )]
    DisabledPanel,
}

#[component]
pub(crate) fn NavigationPanel() -> impl IntoView {
    view! {
        <section>
            <h3>{t(NavigationText::TabsHeading)}</h3>
            <p>{t(NavigationText::TabsDemoSummary)}</p>
            <Tabs
                default_value=NavigationTab::Overview
                tabs=[
                    Tab::new(
                        NavigationTab::Overview,
                        || {
                            view! { <p>{t(NavigationText::TabsOverview)}</p> }
                        },
                    ),
                    Tab::new(
                            NavigationTab::Keyboard,
                            || {
                                view! {
                                    <ul>
                                        <li>{t(NavigationText::KeyboardArrowKeys)}</li>
                                        <li>{t(NavigationText::KeyboardHomeEnd)}</li>
                                        <li>{t(NavigationText::KeyboardManualActivation)}</li>
                                        <li>{t(NavigationText::KeyboardReorder)}</li>
                                        <li>{t(NavigationText::KeyboardClosable)}</li>
                                    </ul>
                                }
                            },
                        )
                        .closable(true),
                    Tab::new(
                            NavigationTab::Closable,
                            || {
                                view! { <p>{t(NavigationText::ClosablePanel)}</p> }
                            },
                        )
                        .closable(true),
                    Tab::new(
                            NavigationTab::Disabled,
                            || {
                                view! { <p>{t(NavigationText::DisabledPanel)}</p> }
                            },
                        )
                        .disabled(true),
                ]
                reorderable=true
            />
        </section>
    }
}
