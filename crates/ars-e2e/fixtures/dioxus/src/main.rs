use std::sync::Arc;

use ars_dioxus::{
    ArsProvider, I18nRegistries, MessageFn, MessagesRegistry,
    navigation::tabs::{self, Tab, Tabs},
    prelude::{Locale, TabKey, Translate, t},
    utility::{
        button::{self, Button, ButtonAsChild},
        dismissable,
        error_boundary::{Boundary, CapturedError},
    },
};
use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey, Translate)]
#[tab_key(ordinal)]
#[translate(fallback = "en-US")]
enum CategoryTab {
    #[translate(en_US = "Navigation", pt_BR = "Navegação")]
    Navigation,

    #[translate(en_US = "Utility", pt_BR = "Utilitários")]
    Utility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey, Translate)]
#[tab_key(ordinal)]
#[translate(fallback = "en-US")]
enum NavigationTab {
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
enum FixtureText {
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

    #[translate(en_US = "Button variants", pt_BR = "Variantes de botão")]
    ButtonVariants,

    #[translate(en_US = "Default", pt_BR = "Padrão")]
    DefaultButton,

    #[translate(en_US = "Primary", pt_BR = "Primário")]
    PrimaryButton,

    #[translate(en_US = "Secondary", pt_BR = "Secundário")]
    SecondaryButton,

    #[translate(en_US = "Destructive", pt_BR = "Destrutivo")]
    DestructiveButton,

    #[translate(en_US = "Outline", pt_BR = "Contorno")]
    OutlineButton,

    #[translate(en_US = "Ghost", pt_BR = "Fantasma")]
    GhostButton,

    #[translate(en_US = "Link", pt_BR = "Link")]
    LinkButton,

    #[translate(en_US = "Small", pt_BR = "Pequeno")]
    SmallButton,

    #[translate(en_US = "Medium", pt_BR = "Médio")]
    MediumButton,

    #[translate(en_US = "Large", pt_BR = "Grande")]
    LargeButton,

    #[translate(en_US = "R", pt_BR = "R")]
    IconButton,

    #[translate(en_US = "Disabled", pt_BR = "Desabilitado")]
    DisabledButton,

    #[translate(en_US = "Loading", pt_BR = "Carregando")]
    LoadingButton,

    #[translate(en_US = "Docs link root", pt_BR = "Link de docs como raiz")]
    DocsLinkRoot,

    #[translate(en_US = "Anchor as primary", pt_BR = "Âncora como primário")]
    AnchorAsPrimary,

    #[translate(en_US = "Submit override", pt_BR = "Sobrescrever envio")]
    SubmitOverride,

    #[translate(en_US = "Reset", pt_BR = "Redefinir")]
    Reset,

    #[translate(
        en_US = "Dismiss example region",
        pt_BR = "Dispensar região de exemplo"
    )]
    DismissExampleRegion,

    #[translate(
        en_US = "Inside dismissable content",
        pt_BR = "Conteúdo dispensável interno"
    )]
    DismissableHeading,

    #[translate(
        en_US = "Click outside the region, press Escape, or activate a dismiss button.",
        pt_BR = "Clique fora da região, pressione Escape ou ative um botão de dispensar."
    )]
    DismissInitial,

    #[translate(
        en_US = "Last dismiss reason: {reason}",
        pt_BR = "Último motivo de dispensa: {reason}"
    )]
    DismissReason { reason: String },
}

fn i18n_registries() -> Arc<I18nRegistries> {
    let mut registries = I18nRegistries::new();

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

    registries.register(
        MessagesRegistry::new(dismissable::Messages::default()).register(
            "pt-BR",
            dismissable::Messages {
                dismiss_label: MessageFn::static_str("Dispensar"),
            },
        ),
    );

    Arc::new(registries)
}

#[component]
fn FixtureErrorChild() -> Element {
    Err(CapturedError::from_display("Dioxus fixture child failed").into())
}

#[component]
fn App() -> Element {
    let locale = use_signal(|| Locale::parse("en-US").expect("valid fixture locale"));

    rsx! {
        ArsProvider { locale, i18n_registries: i18n_registries(),
            main { class: "e2e-shell",
                h1 { "ars-ui Dioxus E2E fixture" }
                Tabs {
                    default_value: CategoryTab::Utility,
                    tabs: [
                        Tab::new(CategoryTab::Navigation, NavigationPanel()),
                        Tab::new(CategoryTab::Utility, UtilityPanel()),
                    ],
                }
            }
        }
    }
}

#[component]
fn NavigationPanel() -> Element {
    rsx! {
        section { class: "showcase-panel wide",
            Tabs {
                default_value: NavigationTab::Overview,
                tabs: [
                    Tab::new(NavigationTab::Overview, rsx! {
                        p { "Tabs fixture overview." }
                    }),
                    Tab::new(NavigationTab::Keyboard, rsx! {
                        ul {
                            li { {t(FixtureText::KeyboardArrowKeys)} }
                            li { {t(FixtureText::KeyboardHomeEnd)} }
                            li { {t(FixtureText::KeyboardReorder)} }
                        }
                    }).closable(true),
                    Tab::new(NavigationTab::Closable, rsx! {
                        p { {t(FixtureText::ClosablePanel)} }
                    }).closable(true),
                    Tab::new(NavigationTab::Disabled, rsx! {
                        p { {t(FixtureText::DisabledPanel)} }
                    }).disabled(true),
                ],
                reorderable: true,
            }
        }
    }
}

#[component]
fn UtilityPanel() -> Element {
    let mut dismiss_status = use_signal_sync(|| FixtureText::DismissInitial);

    let dismiss_props = dismissable::Props::new().on_dismiss(move |reason| {
        dismiss_status.set(FixtureText::DismissReason {
            reason: format!("{reason:?}"),
        });
    });

    rsx! {
        div { class: "gallery-grid",
            section { class: "showcase-panel wide", "aria-labelledby": "variants",
                h2 { id: "variants", {t(FixtureText::ButtonVariants)} }
                div { class: "button-row",
                    Button { id: "dioxus-fixture-default", {t(FixtureText::DefaultButton)} }
                    Button {
                        id: "dioxus-fixture-primary",
                        variant: button::Variant::Primary,
                        {t(FixtureText::PrimaryButton)}
                    }
                    Button {
                        id: "dioxus-fixture-secondary",
                        variant: button::Variant::Secondary,
                        {t(FixtureText::SecondaryButton)}
                    }
                    Button {
                        id: "dioxus-fixture-destructive",
                        variant: button::Variant::Destructive,
                        {t(FixtureText::DestructiveButton)}
                    }
                    Button {
                        id: "dioxus-fixture-outline",
                        variant: button::Variant::Outline,
                        {t(FixtureText::OutlineButton)}
                    }
                    Button {
                        id: "dioxus-fixture-ghost",
                        variant: button::Variant::Ghost,
                        {t(FixtureText::GhostButton)}
                    }
                    Button {
                        id: "dioxus-fixture-link",
                        variant: button::Variant::Link,
                        {t(FixtureText::LinkButton)}
                    }
                    Button { id: "dioxus-fixture-sm", size: button::Size::Sm,
                        {t(FixtureText::SmallButton)}
                    }
                    Button { id: "dioxus-fixture-md", size: button::Size::Md,
                        {t(FixtureText::MediumButton)}
                    }
                    Button { id: "dioxus-fixture-lg", size: button::Size::Lg,
                        {t(FixtureText::LargeButton)}
                    }
                    Button { id: "dioxus-fixture-icon", size: button::Size::Icon,
                        {t(FixtureText::IconButton)}
                    }
                    Button { id: "dioxus-fixture-disabled", disabled: true,
                        {t(FixtureText::DisabledButton)}
                    }
                    Button { id: "dioxus-fixture-loading", loading: true,
                        {t(FixtureText::LoadingButton)}
                    }
                    ButtonAsChild {
                        id: "dioxus-fixture-as-child-docs",
                        variant: button::Variant::Link,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(FixtureText::DocsLinkRoot)} }
                        },
                    }
                    ButtonAsChild {
                        id: "dioxus-fixture-as-child-primary",
                        variant: button::Variant::Primary,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(FixtureText::AnchorAsPrimary)} }
                        },
                    }
                }
                form { id: "dioxus-fixture-example-form",
                    Button {
                        id: "dioxus-fixture-submit",
                        r#type: button::Type::Submit,
                        form: "dioxus-fixture-example-form",
                        name: "intent",
                        value: "save",
                        form_action: "/submit",
                        form_method: button::FormMethod::Post,
                        form_enc_type: button::FormEncType::UrlEncoded,
                        form_target: button::FormTarget::Self_,
                        form_no_validate: true,
                        {t(FixtureText::SubmitOverride)}
                    }
                    Button {
                        id: "dioxus-fixture-reset",
                        r#type: button::Type::Reset,
                        {t(FixtureText::Reset)}
                    }
                }
            }
            section {
                class: "showcase-panel wide",
                "aria-labelledby": "dismissable",
                h2 { id: "dismissable", "Dismissable primitive" }
                dismissable::Region {
                    props: dismiss_props,
                    dismiss_label: t(FixtureText::DismissExampleRegion),
                    div { class: "dismissable-card",
                        h3 { {t(FixtureText::DismissableHeading)} }
                    }
                }
                p { class: "dismissable-status", {t(dismiss_status())} }
            }
            section { class: "showcase-panel wide", "aria-labelledby": "errors",
                h2 { id: "errors", "Error boundary" }
                Boundary {
                    p { class: "healthy-boundary", "Healthy child rendered" }
                }
                Boundary { FixtureErrorChild {} }
            }
        }
    }
}
