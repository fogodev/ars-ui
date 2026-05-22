//! Utility-category fixture module.
//!
//! Owns the Button / VisuallyHidden / Separator / ClientOnly /
//! ZIndexAllocator / Heading / Landmark / Highlight / Dismissable /
//! ErrorBoundary showcase panel, the per-category text enum, and the
//! message-registry entry for `dismissable::Messages`.

use std::fmt::{self, Display};

use ars_leptos::{
    I18nRegistries, MessageFn, MessagesRegistry,
    prelude::{Orientation, Translate, t},
    utility::{
        button::{self, Button, ButtonAsChild},
        client_only::ClientOnly,
        dismissable,
        error_boundary::Boundary,
        heading::{self, Heading, HeadingLevelProvider, Section},
        highlight::Highlight,
        landmark::{self, Landmark},
        separator::{Separator, SeparatorAsChild},
        visually_hidden::{VisuallyHidden, VisuallyHiddenAsChild},
        z_index_allocator::{Context as ZIndexContext, ZIndexAllocatorProvider},
    },
};
use leptos::prelude::*;

/// Localized strings used by the utility panel.
#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum UtilityText {
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

    #[translate(en_US = "Top-level heading", pt_BR = "Cabeçalho de nível superior")]
    HeadingDefault,

    #[translate(en_US = "Explicit level three", pt_BR = "Nível três explícito")]
    HeadingThree,

    #[translate(
        en_US = "Heading inside provider",
        pt_BR = "Cabeçalho dentro do provider"
    )]
    HeadingProvider,

    #[translate(en_US = "Heading inside section", pt_BR = "Cabeçalho dentro da seção")]
    HeadingSection,

    #[translate(en_US = "Page banner", pt_BR = "Cabeçalho da página")]
    LandmarkBanner,

    #[translate(en_US = "Primary navigation", pt_BR = "Navegação principal")]
    LandmarkNavigation,

    #[translate(en_US = "Site search", pt_BR = "Busca do site")]
    LandmarkSearch,

    #[translate(en_US = "Sidebar region", pt_BR = "Região da barra lateral")]
    LandmarkRegion,
}

/// Registers the utility category's localized message bundles with the
/// fixture's shared `I18nRegistries`.
pub(crate) fn register_messages(registries: &mut I18nRegistries) {
    registries.register(
        MessagesRegistry::new(dismissable::Messages::default()).register(
            "pt-BR",
            dismissable::Messages {
                dismiss_label: MessageFn::static_str("Dispensar"),
            },
        ),
    );
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
fn HeadingProviderDemo() -> impl IntoView {
    // The fixture's surrounding section is rendered at h2, so we start the
    // provider at Level::Three and let Section bump to Level::Four. This keeps
    // the page-wide heading hierarchy monotonic (h1 → h2 → h3 → h4) so axe's
    // `heading-order` rule is satisfied. The harness inspects the four headings
    // below to verify explicit-level, provider-inherited, and section-bumped
    // behavior end-to-end in the browser.
    view! {
        <Heading id="leptos-fixture-heading-default" level=heading::Level::Three>
            {t(UtilityText::HeadingDefault)}
        </Heading>
        <Heading id="leptos-fixture-heading-level-three" level=heading::Level::Three>
            {t(UtilityText::HeadingThree)}
        </Heading>
        <HeadingLevelProvider level=heading::Level::Three>
            <Heading id="leptos-fixture-heading-provided">
                {t(UtilityText::HeadingProvider)}
            </Heading>
            <Section>
                <Heading id="leptos-fixture-heading-section">
                    {t(UtilityText::HeadingSection)}
                </Heading>
            </Section>
        </HeadingLevelProvider>
    }
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

/// Utility-category showcase panel.
#[component]
pub(crate) fn UtilityPanel() -> impl IntoView {
    let (dismiss_status, set_dismiss_status) = signal(UtilityText::DismissInitial);

    let dismiss_props = dismissable::Props::new().on_dismiss(move |reason| {
        set_dismiss_status.set(UtilityText::DismissReason {
            reason: format!("{reason:?}"),
        });
    });

    view! {
        <div class="gallery-grid">
            <section class="showcase-panel wide" aria-labelledby="variants">
                <h2 id="variants">{t(UtilityText::ButtonVariants)}</h2>
                <div class="button-row">
                    <Button id="leptos-fixture-default">{t(UtilityText::DefaultButton)}</Button>
                    <Button id="leptos-fixture-primary" variant=button::Variant::Primary>
                        {t(UtilityText::PrimaryButton)}
                    </Button>
                    <Button id="leptos-fixture-secondary" variant=button::Variant::Secondary>
                        {t(UtilityText::SecondaryButton)}
                    </Button>
                    <Button id="leptos-fixture-destructive" variant=button::Variant::Destructive>
                        {t(UtilityText::DestructiveButton)}
                    </Button>
                    <Button id="leptos-fixture-outline" variant=button::Variant::Outline>
                        {t(UtilityText::OutlineButton)}
                    </Button>
                    <Button id="leptos-fixture-ghost" variant=button::Variant::Ghost>
                        {t(UtilityText::GhostButton)}
                    </Button>
                    <Button id="leptos-fixture-link" variant=button::Variant::Link>
                        {t(UtilityText::LinkButton)}
                    </Button>
                    <Button id="leptos-fixture-sm" size=button::Size::Sm>
                        {t(UtilityText::SmallButton)}
                    </Button>
                    <Button id="leptos-fixture-md" size=button::Size::Md>
                        {t(UtilityText::MediumButton)}
                    </Button>
                    <Button id="leptos-fixture-lg" size=button::Size::Lg>
                        {t(UtilityText::LargeButton)}
                    </Button>
                    <Button id="leptos-fixture-icon" size=button::Size::Icon>
                        {t(UtilityText::IconButton)}
                    </Button>
                    <Button id="leptos-fixture-disabled" disabled=true>
                        {t(UtilityText::DisabledButton)}
                    </Button>
                    <Button id="leptos-fixture-loading" loading=true>
                        {t(UtilityText::LoadingButton)}
                    </Button>
                    <ButtonAsChild id="leptos-fixture-as-child-docs" variant=button::Variant::Link>
                        <a href="#variants">{t(UtilityText::DocsLinkRoot)}</a>
                    </ButtonAsChild>
                    <ButtonAsChild
                        id="leptos-fixture-as-child-primary"
                        variant=button::Variant::Primary
                    >
                        <a href="#variants">{t(UtilityText::AnchorAsPrimary)}</a>
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
                        {t(UtilityText::SubmitOverride)}
                    </Button>
                    <Button id="leptos-fixture-reset" r#type=button::Type::Reset>
                        {t(UtilityText::Reset)}
                    </Button>
                </form>
            </section>
            <section class="showcase-panel wide" aria-labelledby="utility-primitives">
                <h2 id="utility-primitives">"Utility primitives"</h2>
                <p>
                    <VisuallyHidden id="leptos-fixture-visually-hidden-label">
                        {t(UtilityText::VisuallyHiddenLabel)}
                    </VisuallyHidden>
                    "Visible copy with a hidden accessible companion."
                </p>
                <p>
                    <VisuallyHidden id="leptos-fixture-focusable-skip" is_focusable=true>
                        <a href="#variants">{t(UtilityText::FocusableSkipLink)}</a>
                    </VisuallyHidden>
                </p>
                <VisuallyHiddenAsChild id="leptos-fixture-visually-hidden-as-child">
                    <span>{t(UtilityText::AsChildHiddenLabel)}</span>
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
            <section class="showcase-panel wide" aria-labelledby="heading-primitive">
                <h2 id="heading-primitive">"Heading primitive"</h2>
                <HeadingProviderDemo />
            </section>
            <section class="showcase-panel wide" aria-labelledby="landmark-primitive">
                <h2 id="landmark-primitive">"Landmark primitive"</h2>
                <Landmark
                    id="leptos-fixture-landmark-banner"
                    role=landmark::Role::Banner
                    messages=landmark::Messages {
                        label: MessageFn::static_str("Page banner"),
                    }
                >
                    {t(UtilityText::LandmarkBanner)}
                </Landmark>
                <Landmark
                    id="leptos-fixture-landmark-navigation"
                    role=landmark::Role::Navigation
                    messages=landmark::Messages {
                        label: MessageFn::static_str("Primary navigation"),
                    }
                >
                    {t(UtilityText::LandmarkNavigation)}
                </Landmark>
                <Landmark
                    id="leptos-fixture-landmark-search"
                    role=landmark::Role::Search
                    messages=landmark::Messages {
                        label: MessageFn::static_str("Site search"),
                    }
                >
                    {t(UtilityText::LandmarkSearch)}
                </Landmark>
                <Landmark
                    id="leptos-fixture-landmark-region"
                    role=landmark::Role::Region
                    messages=landmark::Messages {
                        label: MessageFn::static_str("Sidebar region"),
                    }
                >
                    {t(UtilityText::LandmarkRegion)}
                </Landmark>
            </section>
            <section class="showcase-panel wide" aria-labelledby="highlight-primitive">
                <h2 id="highlight-primitive">"Highlight primitive"</h2>
                <div id="leptos-fixture-highlight-host">
                    <Highlight
                        query=vec!["highlighted".to_string()]
                        text="Hello highlighted world!".to_string()
                    />
                </div>
            </section>
            <section class="showcase-panel wide" aria-labelledby="dismissable">
                <h2 id="dismissable">"Dismissable primitive"</h2>
                <dismissable::Region props=dismiss_props dismiss_label="Dismiss example region">
                    <div class="dismissable-card">
                        <h3>{t(UtilityText::DismissableHeading)}</h3>
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
