use std::{
    fmt::{self, Display},
    sync::Arc,
};

use ars_leptos::{
    ArsProvider, I18nRegistries, MessageFn, MessagesRegistry,
    navigation::tabs::{self, Tab, Tabs},
    prelude::{Locale, Orientation, TabKey, Translate, t},
    utility::{
        button::{self, Button, ButtonAsChild},
        client_only::ClientOnly,
        dismissable,
        error_boundary::Boundary,
        separator::{Separator, SeparatorAsChild},
        visually_hidden::{VisuallyHidden, VisuallyHiddenAsChild},
        z_index_allocator::{Context as ZIndexContext, ZIndexAllocatorProvider},
    },
};
use leptos::{mount::mount_to_body, prelude::*};

fn main() {
    mount_to_body(App);
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
        en_US = "Screen reader only label",
        pt_BR = "Rótulo apenas para leitor de tela"
    )]
    VisuallyHiddenLabel,

    #[translate(
        en_US = "Skip to button variants",
        pt_BR = "Pular para variantes de botão"
    )]
    FocusableSkipLink,

    #[translate(
        en_US = "Hidden label on consumer root",
        pt_BR = "Rótulo oculto na raiz do consumidor"
    )]
    AsChildHiddenLabel,

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

#[derive(Debug)]
struct FixtureError;

impl Display for FixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Leptos fixture child failed")
    }
}

impl std::error::Error for FixtureError {}

fn fixture_error() -> Result<&'static str, FixtureError> {
    Err(FixtureError)
}

#[component]
fn ZIndexProbe(id: &'static str) -> impl IntoView {
    let context = use_context::<ZIndexContext>().expect("z-index context should be provided");

    let claim = context.allocate_claim();

    view! {
        <span id=id data-z=claim.value().to_string()>
            "Allocated"
        </span>
    }
}

#[component]
fn App() -> impl IntoView {
    let locale = RwSignal::new(Locale::parse("en-US").expect("valid fixture locale"));

    view! {
        <ArsProvider locale=locale i18n_registries=i18n_registries()>
            <main class="e2e-shell">
                <h1>"ars-ui Leptos E2E fixture"</h1>
                <Tabs
                    default_value=CategoryTab::Utility
                    tabs=[
                        Tab::new(CategoryTab::Navigation, NavigationPanel),
                        Tab::new(CategoryTab::Utility, UtilityPanel),
                    ]
                />
            </main>
        </ArsProvider>
    }
}

#[component]
fn NavigationPanel() -> impl IntoView {
    view! {
        <section class="showcase-panel wide">
            <Tabs
                default_value=NavigationTab::Overview
                tabs=[
                    Tab::new(NavigationTab::Overview, || view! { <p>"Tabs fixture overview."</p> }),
                    Tab::new(
                            NavigationTab::Keyboard,
                            || {
                                view! {
                                    <ul>
                                        <li>{t(FixtureText::KeyboardArrowKeys)}</li>
                                        <li>{t(FixtureText::KeyboardHomeEnd)}</li>
                                        <li>{t(FixtureText::KeyboardReorder)}</li>
                                    </ul>
                                }
                            },
                        )
                        .closable(true),
                    Tab::new(
                            NavigationTab::Closable,
                            || view! { <p>{t(FixtureText::ClosablePanel)}</p> },
                        )
                        .closable(true),
                    Tab::new(
                            NavigationTab::Disabled,
                            || view! { <p>{t(FixtureText::DisabledPanel)}</p> },
                        )
                        .disabled(true),
                ]
                reorderable=true
            />
        </section>
    }
}

#[component]
fn UtilityPanel() -> impl IntoView {
    let (dismiss_status, set_dismiss_status) = signal(FixtureText::DismissInitial);

    let dismiss_props = dismissable::Props::new().on_dismiss(move |reason| {
        set_dismiss_status.set(FixtureText::DismissReason {
            reason: format!("{reason:?}"),
        });
    });

    view! {
        <div class="gallery-grid">
            <section class="showcase-panel wide" aria-labelledby="variants">
                <h2 id="variants">{t(FixtureText::ButtonVariants)}</h2>
                <div class="button-row">
                    <Button id="leptos-fixture-default">{t(FixtureText::DefaultButton)}</Button>
                    <Button id="leptos-fixture-primary" variant=button::Variant::Primary>
                        {t(FixtureText::PrimaryButton)}
                    </Button>
                    <Button id="leptos-fixture-secondary" variant=button::Variant::Secondary>
                        {t(FixtureText::SecondaryButton)}
                    </Button>
                    <Button id="leptos-fixture-destructive" variant=button::Variant::Destructive>
                        {t(FixtureText::DestructiveButton)}
                    </Button>
                    <Button id="leptos-fixture-outline" variant=button::Variant::Outline>
                        {t(FixtureText::OutlineButton)}
                    </Button>
                    <Button id="leptos-fixture-ghost" variant=button::Variant::Ghost>
                        {t(FixtureText::GhostButton)}
                    </Button>
                    <Button id="leptos-fixture-link" variant=button::Variant::Link>
                        {t(FixtureText::LinkButton)}
                    </Button>
                    <Button id="leptos-fixture-sm" size=button::Size::Sm>
                        {t(FixtureText::SmallButton)}
                    </Button>
                    <Button id="leptos-fixture-md" size=button::Size::Md>
                        {t(FixtureText::MediumButton)}
                    </Button>
                    <Button id="leptos-fixture-lg" size=button::Size::Lg>
                        {t(FixtureText::LargeButton)}
                    </Button>
                    <Button id="leptos-fixture-icon" size=button::Size::Icon>
                        {t(FixtureText::IconButton)}
                    </Button>
                    <Button id="leptos-fixture-disabled" disabled=true>
                        {t(FixtureText::DisabledButton)}
                    </Button>
                    <Button id="leptos-fixture-loading" loading=true>
                        {t(FixtureText::LoadingButton)}
                    </Button>
                    <ButtonAsChild id="leptos-fixture-as-child-docs" variant=button::Variant::Link>
                        <a href="#variants">{t(FixtureText::DocsLinkRoot)}</a>
                    </ButtonAsChild>
                    <ButtonAsChild
                        id="leptos-fixture-as-child-primary"
                        variant=button::Variant::Primary
                    >
                        <a href="#variants">{t(FixtureText::AnchorAsPrimary)}</a>
                    </ButtonAsChild>
                </div>
                <form id="leptos-fixture-example-form">
                    <Button
                        id="leptos-fixture-submit"
                        r#type=button::Type::Submit
                        form="leptos-fixture-example-form"
                        name="intent"
                        value="save"
                        form_action="/submit"
                        form_method=button::FormMethod::Post
                        form_enc_type=button::FormEncType::UrlEncoded
                        form_target=button::FormTarget::Self_
                        form_no_validate=true
                    >
                        {t(FixtureText::SubmitOverride)}
                    </Button>
                    <Button id="leptos-fixture-reset" r#type=button::Type::Reset>
                        {t(FixtureText::Reset)}
                    </Button>
                </form>
            </section>
            <section class="showcase-panel wide" aria-labelledby="utility-primitives">
                <h2 id="utility-primitives">"Utility primitives"</h2>
                <p>
                    <VisuallyHidden id="leptos-fixture-visually-hidden-label">
                        {t(FixtureText::VisuallyHiddenLabel)}
                    </VisuallyHidden>
                    "Visible copy with a hidden accessible companion."
                </p>
                <p>
                    <VisuallyHidden id="leptos-fixture-focusable-skip" is_focusable=true>
                        <a href="#variants">{t(FixtureText::FocusableSkipLink)}</a>
                    </VisuallyHidden>
                </p>
                <VisuallyHiddenAsChild id="leptos-fixture-visually-hidden-as-child">
                    <span>{t(FixtureText::AsChildHiddenLabel)}</span>
                </VisuallyHiddenAsChild>
                <Separator id="leptos-fixture-separator-horizontal" />
                <div class="separator-demo-row">
                    <span>"Before"</span>
                    <Separator
                        id="leptos-fixture-separator-vertical"
                        orientation=Orientation::Vertical
                    />
                    <span>"After"</span>
                </div>
                <SeparatorAsChild
                    id="leptos-fixture-separator-as-child"
                    orientation=Orientation::Vertical
                >
                    <div class="separator-as-child"></div>
                </SeparatorAsChild>
                <Separator id="leptos-fixture-separator-decorative" decorative=true />
                <div id="leptos-fixture-client-only-host">
                    <ClientOnly fallback=|| {
                        view! {
                            <span id="leptos-fixture-client-only-fallback">
                                "Loading client content"
                            </span>
                        }
                    }>
                        <span id="leptos-fixture-client-only-child">"Client content"</span>
                    </ClientOnly>
                </div>
                <section id="leptos-fixture-z-index-host">
                    <ZIndexAllocatorProvider>
                        <ZIndexProbe id="leptos-fixture-z-index-first" />
                        <ZIndexProbe id="leptos-fixture-z-index-second" />
                    </ZIndexAllocatorProvider>
                </section>
            </section>
            <section class="showcase-panel wide" aria-labelledby="dismissable">
                <h2 id="dismissable">"Dismissable primitive"</h2>
                <dismissable::Region props=dismiss_props dismiss_label="Dismiss example region">
                    <div class="dismissable-card">
                        <h3>{t(FixtureText::DismissableHeading)}</h3>
                    </div>
                </dismissable::Region>
                <p class="dismissable-status">{move || t(dismiss_status.get())}</p>
            </section>
            <section class="showcase-panel wide" aria-labelledby="errors">
                <h2 id="errors">"Error boundary"</h2>
                <Boundary>
                    <p class="healthy-boundary">"Healthy child rendered"</p>
                </Boundary>
                <Boundary>{fixture_error()}</Boundary>
            </section>
        </div>
    }
}
