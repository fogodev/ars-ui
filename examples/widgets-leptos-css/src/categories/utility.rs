use std::fmt::{self, Display};

use ars_leptos::{
    prelude::{Orientation, t, Translate},
    utility::{
        button::{self, Button, ButtonAsChild},
        dismissable,
        error_boundary::Boundary,
        separator::{Separator, SeparatorAsChild},
        visually_hidden::{VisuallyHidden, VisuallyHiddenAsChild},
    },
};
use leptos::prelude::*;

#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum UtilityText {
    #[translate(en_US = "Button variants", pt_BR = "Variantes de botão")]
    ButtonVariants,

    #[translate(
        en_US = "Hover each button to inspect transitions.",
        pt_BR = "Passe o mouse em cada botão para inspecionar as transições."
    )]
    ButtonVariantsNote,

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

    #[translate(en_US = "Button sizes", pt_BR = "Tamanhos de botão")]
    ButtonSizes,

    #[translate(en_US = "Small", pt_BR = "Pequeno")]
    SmallButton,

    #[translate(en_US = "Medium", pt_BR = "Médio")]
    MediumButton,

    #[translate(en_US = "Large", pt_BR = "Grande")]
    LargeButton,

    #[translate(en_US = "R", pt_BR = "R")]
    IconButton,

    #[translate(en_US = "Button states", pt_BR = "Estados de botão")]
    ButtonStates,

    #[translate(en_US = "Disabled", pt_BR = "Desabilitado")]
    DisabledButton,

    #[translate(en_US = "Loading", pt_BR = "Carregando")]
    LoadingButton,

    #[translate(en_US = "As child", pt_BR = "Como filho")]
    AsChild,

    #[translate(en_US = "Docs link root", pt_BR = "Link de docs como raiz")]
    DocsLinkRoot,

    #[translate(en_US = "Anchor as primary", pt_BR = "Âncora como primário")]
    AnchorAsPrimary,

    #[translate(en_US = "Forms", pt_BR = "Formulários")]
    Forms,

    #[translate(en_US = "Submit override", pt_BR = "Sobrescrever envio")]
    SubmitOverride,

    #[translate(en_US = "Reset", pt_BR = "Redefinir")]
    Reset,

    #[translate(en_US = "Visually hidden", pt_BR = "Visualmente oculto")]
    VisuallyHidden,

    #[translate(
        en_US = "Screen-reader text stays in the DOM while the visual layout remains quiet.",
        pt_BR = "O texto para leitores de tela permanece no DOM enquanto o leiaute visual fica limpo."
    )]
    VisuallyHiddenDescription,

    #[translate(
        en_US = "Screen reader only label",
        pt_BR = "Rótulo apenas para leitor de tela"
    )]
    VisuallyHiddenLabel,

    #[translate(en_US = "Skip to button variants", pt_BR = "Pular para variantes de botão")]
    FocusableSkipLink,

    #[translate(
        en_US = "Hidden label on consumer root",
        pt_BR = "Rótulo oculto na raiz do consumidor"
    )]
    AsChildHiddenLabel,

    #[translate(en_US = "Separator", pt_BR = "Separador")]
    SeparatorPrimitive,

    #[translate(
        en_US = "Semantic, vertical, and decorative separators share the same root part.",
        pt_BR = "Separadores semânticos, verticais e decorativos compartilham a mesma parte raiz."
    )]
    SeparatorDescription,

    #[translate(en_US = "Horizontal section break", pt_BR = "Quebra horizontal de seção")]
    HorizontalSeparator,

    #[translate(en_US = "Vertical divider", pt_BR = "Divisor vertical")]
    VerticalSeparator,

    #[translate(en_US = "Decorative divider", pt_BR = "Divisor decorativo")]
    DecorativeSeparator,

    #[translate(
        en_US = "Consumer-owned divider keeps separator semantics",
        pt_BR = "O divisor da raiz do consumidor preserva a semântica de separador"
    )]
    AsChildSeparator,

    #[translate(en_US = "Dismissable primitive", pt_BR = "Primitivo dismissable")]
    DismissablePrimitive,

    #[translate(en_US = "Enum-driven sizing.", pt_BR = "Tamanhos orientados por enum.")]
    ButtonSizesNote,

    #[translate(
        en_US = "Disabled and busy controls.",
        pt_BR = "Controles desabilitados e ocupados."
    )]
    ButtonStatesNote,

    #[translate(
        en_US = "Button attrs on consumer-owned anchors.",
        pt_BR = "Atributos de botão em âncoras controladas pelo consumidor."
    )]
    AsChildNote,

    #[translate(
        en_US = "Submit/reset and form overrides.",
        pt_BR = "Envio, redefinição e sobrescritas de formulário."
    )]
    FormsNote,

    #[translate(
        en_US = "Outside pointer/focus, Escape, and hidden dismiss buttons share one primitive.",
        pt_BR = "Ponteiro/foco externo, Escape e botões ocultos de dispensar compartilham um primitivo."
    )]
    DismissableNote,

    #[translate(en_US = "CSS dismissable region", pt_BR = "Região dismissable em CSS")]
    CssDismissableRegion,

    #[translate(
        en_US = "This standalone primitive is the behavior layer future overlays will compose.",
        pt_BR = "Este primitivo independente e a camada de comportamento que futuras sobreposições vão compor."
    )]
    DismissableCompositionDescription,

    #[translate(
        en_US = "Healthy and captured child output.",
        pt_BR = "Saída de filho saudável e capturada."
    )]
    ErrorBoundaryNote,

    #[translate(
        en_US = "Click outside the region, press Escape, or tab to a hidden dismiss button.",
        pt_BR = "Clique fora da região, pressione Escape ou use Tab até um botão oculto de dispensar."
    )]
    DismissInitial,

    #[translate(
        en_US = "Last dismiss reason: {reason}",
        pt_BR = "Último motivo de dispensa: {reason}"
    )]
    DismissReason { reason: String },

    #[translate(
        en_US = "Example child failed while rendering.",
        pt_BR = "O filho de exemplo falhou durante a renderização."
    )]
    ExampleChildError,

    #[translate(en_US = "Error boundary", pt_BR = "Limite de erro")]
    ErrorBoundary,

    #[translate(
        en_US = "Healthy child rendered inside the boundary.",
        pt_BR = "Filho saudável renderizado dentro do limite."
    )]
    HealthyChild,
}

#[derive(Debug)]
struct ExampleError(Signal<String>);

impl Display for ExampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.get())
    }
}

impl std::error::Error for ExampleError {}

fn example_error(message: Signal<String>) -> Result<&'static str, ExampleError> {
    Err(ExampleError(message))
}

#[component]
pub(crate) fn UtilityPanel() -> impl IntoView {
    let (dismiss_status, set_dismiss_status) = signal(UtilityText::DismissInitial);
    let dismiss_props = dismissable::Props::new().on_dismiss(move |reason| {
        set_dismiss_status.set(UtilityText::DismissReason {
            reason: format!("{reason:?}"),
        });
    });
    let error_message = t(UtilityText::ExampleChildError);

    view! {
        <div class="gallery-grid">
            <section class="showcase-panel wide" aria-labelledby="variants">
                <div class="panel-heading">
                    <h2 id="variants">{t(UtilityText::ButtonVariants)}</h2>
                    <p class="panel-note">{t(UtilityText::ButtonVariantsNote)}</p>
                </div>
                <div class="button-row">
                    <Button id="leptos-css-default">{t(UtilityText::DefaultButton)}</Button>
                    <Button id="leptos-css-primary" variant=button::Variant::Primary>
                        {t(UtilityText::PrimaryButton)}
                    </Button>
                    <Button id="leptos-css-secondary" variant=button::Variant::Secondary>
                        {t(UtilityText::SecondaryButton)}
                    </Button>
                    <Button id="leptos-css-destructive" variant=button::Variant::Destructive>
                        {t(UtilityText::DestructiveButton)}
                    </Button>
                    <Button id="leptos-css-outline" variant=button::Variant::Outline>
                        {t(UtilityText::OutlineButton)}
                    </Button>
                    <Button id="leptos-css-ghost" variant=button::Variant::Ghost>
                        {t(UtilityText::GhostButton)}
                    </Button>
                    <Button id="leptos-css-link" variant=button::Variant::Link>
                        {t(UtilityText::LinkButton)}
                    </Button>
                </div>
            </section>
            <section class="showcase-panel" aria-labelledby="sizes">
                <div class="panel-heading">
                    <h2 id="sizes">{t(UtilityText::ButtonSizes)}</h2>
                    <p class="panel-note">{t(UtilityText::ButtonSizesNote)}</p>
                </div>
                <div class="button-row">
                    <Button id="leptos-css-sm" size=button::Size::Sm>
                        {t(UtilityText::SmallButton)}
                    </Button>
                    <Button id="leptos-css-md" size=button::Size::Md>
                        {t(UtilityText::MediumButton)}
                    </Button>
                    <Button id="leptos-css-lg" size=button::Size::Lg>
                        {t(UtilityText::LargeButton)}
                    </Button>
                    <Button id="leptos-css-icon" size=button::Size::Icon>
                        {t(UtilityText::IconButton)}
                    </Button>
                </div>
            </section>
            <section class="showcase-panel" aria-labelledby="states">
                <div class="panel-heading">
                    <h2 id="states">{t(UtilityText::ButtonStates)}</h2>
                    <p class="panel-note">{t(UtilityText::ButtonStatesNote)}</p>
                </div>
                <div class="button-row">
                    <Button id="leptos-css-disabled" disabled=true>
                        {t(UtilityText::DisabledButton)}
                    </Button>
                    <Button id="leptos-css-loading" loading=true>
                        {t(UtilityText::LoadingButton)}
                    </Button>
                </div>
            </section>
            <section class="showcase-panel" aria-labelledby="as-child">
                <div class="panel-heading">
                    <h2 id="as-child">{t(UtilityText::AsChild)}</h2>
                    <p class="panel-note">{t(UtilityText::AsChildNote)}</p>
                </div>
                <div class="button-row">
                    <ButtonAsChild id="leptos-css-as-child-docs" variant=button::Variant::Link>
                        <a href="#variants">{t(UtilityText::DocsLinkRoot)}</a>
                    </ButtonAsChild>
                    <ButtonAsChild
                        id="leptos-css-as-child-primary"
                        variant=button::Variant::Primary
                    >
                        <a href="#variants">{t(UtilityText::AnchorAsPrimary)}</a>
                    </ButtonAsChild>
                </div>
            </section>
            <section class="showcase-panel" aria-labelledby="forms">
                <div class="panel-heading">
                    <h2 id="forms">{t(UtilityText::Forms)}</h2>
                    <p class="panel-note">{t(UtilityText::FormsNote)}</p>
                </div>
                <form id="leptos-css-example-form">
                    <div class="button-row">
                        <Button
                            id="leptos-css-submit"
                            r#type=button::Type::Submit
                            form="leptos-css-example-form"
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
                        <Button id="leptos-css-reset" r#type=button::Type::Reset>
                            {t(UtilityText::Reset)}
                        </Button>
                    </div>
                </form>
            </section>
            <section class="showcase-panel" aria-labelledby="visually-hidden">
                <div class="panel-heading">
                    <h2 id="visually-hidden">{t(UtilityText::VisuallyHidden)}</h2>
                    <p class="panel-note">{t(UtilityText::VisuallyHiddenDescription)}</p>
                </div>
                <p>
                    <VisuallyHidden id="leptos-css-visually-hidden-label">
                        {t(UtilityText::VisuallyHiddenLabel)}
                    </VisuallyHidden>
                    {t(UtilityText::VisuallyHiddenDescription)}
                </p>
                <p>
                    <VisuallyHidden id="leptos-css-focusable-skip" is_focusable=true>
                        <a href="#variants">{t(UtilityText::FocusableSkipLink)}</a>
                    </VisuallyHidden>
                </p>
                <VisuallyHiddenAsChild id="leptos-css-visually-hidden-as-child">
                    <span>{t(UtilityText::AsChildHiddenLabel)}</span>
                </VisuallyHiddenAsChild>
            </section>
            <section class="showcase-panel" aria-labelledby="separator">
                <div class="panel-heading">
                    <h2 id="separator">{t(UtilityText::SeparatorPrimitive)}</h2>
                    <p class="panel-note">{t(UtilityText::SeparatorDescription)}</p>
                </div>
                <Separator id="leptos-css-separator-horizontal" />
                <div class="separator-demo-row">
                    <span>{t(UtilityText::HorizontalSeparator)}</span>
                    <Separator
                        id="leptos-css-separator-vertical"
                        orientation=Orientation::Vertical
                    />
                    <span>{t(UtilityText::VerticalSeparator)}</span>
                </div>
                <Separator id="leptos-css-separator-decorative" decorative=true />
                <p class="panel-note">{t(UtilityText::DecorativeSeparator)}</p>
                <SeparatorAsChild
                    id="leptos-css-separator-as-child"
                    orientation=Orientation::Vertical
                >
                    <div class="separator-as-child"></div>
                </SeparatorAsChild>
                <p class="panel-note">{t(UtilityText::AsChildSeparator)}</p>
            </section>
            <section class="showcase-panel wide" aria-labelledby="dismissable">
                <div class="panel-heading">
                    <h2 id="dismissable">{t(UtilityText::DismissablePrimitive)}</h2>
                    <p class="panel-note">{t(UtilityText::DismissableNote)}</p>
                </div>
                <dismissable::Region props=dismiss_props>
                    <div class="dismissable-card">
                        <h3>{t(UtilityText::CssDismissableRegion)}</h3>
                        <p>{t(UtilityText::DismissableCompositionDescription)}</p>
                    </div>
                </dismissable::Region>
                <p class="dismissable-status">{t(dismiss_status)}</p>
            </section>
            <section class="showcase-panel wide" aria-labelledby="errors">
                <div class="panel-heading">
                    <h2 id="errors">{t(UtilityText::ErrorBoundary)}</h2>
                    <p class="panel-note">{t(UtilityText::ErrorBoundaryNote)}</p>
                </div>
                <div class="error-grid">
                    <Boundary>
                        <p class="healthy-boundary">{t(UtilityText::HealthyChild)}</p>
                    </Boundary>
                    <Boundary>{example_error(error_message)}</Boundary>
                </div>
            </section>
        </div>
    }
}
