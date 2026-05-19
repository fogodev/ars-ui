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

    #[translate(en_US = "sm, md, lg, icon", pt_BR = "sm, md, lg, ícone")]
    ButtonSizeTokens,

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

    #[translate(
        en_US = "Tailwind dismissable region",
        pt_BR = "Região dismissable em Tailwind"
    )]
    TailwindDismissableRegion,

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
        <div class="grid gap-5 mt-5 lg:grid-cols-2">
            <section
                class="p-5 rounded-lg border shadow-xl lg:col-span-2 border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="variants"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="variants" class="text-base font-bold text-slate-950">
                        {t(UtilityText::ButtonVariants)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::ButtonVariantsNote)}</p>
                </div>
                <div class="flex flex-wrap gap-3">
                    <Button id="leptos-tw-default">{t(UtilityText::DefaultButton)}</Button>
                    <Button id="leptos-tw-primary" variant=button::Variant::Primary>
                        {t(UtilityText::PrimaryButton)}
                    </Button>
                    <Button id="leptos-tw-secondary" variant=button::Variant::Secondary>
                        {t(UtilityText::SecondaryButton)}
                    </Button>
                    <Button id="leptos-tw-destructive" variant=button::Variant::Destructive>
                        {t(UtilityText::DestructiveButton)}
                    </Button>
                    <Button id="leptos-tw-outline" variant=button::Variant::Outline>
                        {t(UtilityText::OutlineButton)}
                    </Button>
                    <Button id="leptos-tw-ghost" variant=button::Variant::Ghost>
                        {t(UtilityText::GhostButton)}
                    </Button>
                    <Button id="leptos-tw-link" variant=button::Variant::Link>
                        {t(UtilityText::LinkButton)}
                    </Button>
                </div>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="sizes"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="sizes" class="text-base font-bold text-slate-950">
                        {t(UtilityText::ButtonSizes)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::ButtonSizeTokens)}</p>
                </div>
                <div class="flex flex-wrap gap-3">
                    <Button id="leptos-tw-sm" size=button::Size::Sm>
                        {t(UtilityText::SmallButton)}
                    </Button>
                    <Button id="leptos-tw-md" size=button::Size::Md>
                        {t(UtilityText::MediumButton)}
                    </Button>
                    <Button id="leptos-tw-lg" size=button::Size::Lg>
                        {t(UtilityText::LargeButton)}
                    </Button>
                    <Button id="leptos-tw-icon" size=button::Size::Icon>
                        {t(UtilityText::IconButton)}
                    </Button>
                </div>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="states"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="states" class="text-base font-bold text-slate-950">
                        {t(UtilityText::ButtonStates)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::ButtonStatesNote)}</p>
                </div>
                <div class="flex flex-wrap gap-3">
                    <Button id="leptos-tw-disabled" disabled=true>
                        {t(UtilityText::DisabledButton)}
                    </Button>
                    <Button id="leptos-tw-loading" loading=true>
                        {t(UtilityText::LoadingButton)}
                    </Button>
                </div>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="as-child"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="as-child" class="text-base font-bold text-slate-950">
                        {t(UtilityText::AsChild)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::AsChildNote)}</p>
                </div>
                <div class="flex flex-wrap gap-3">
                    <ButtonAsChild
                        id="leptos-tw-as-child-docs"
                        variant=button::Variant::Link
                        class="group"
                    >
                        <a href="#variants">{t(UtilityText::DocsLinkRoot)}</a>
                    </ButtonAsChild>
                    <ButtonAsChild id="leptos-tw-as-child-primary" variant=button::Variant::Primary>
                        <a href="#variants">{t(UtilityText::AnchorAsPrimary)}</a>
                    </ButtonAsChild>
                </div>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="forms"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="forms" class="text-base font-bold text-slate-950">
                        {t(UtilityText::Forms)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::FormsNote)}</p>
                </div>
                <form id="leptos-tw-example-form">
                    <div class="flex flex-wrap gap-3">
                        <Button
                            id="leptos-tw-submit"
                            r#type=button::Type::Submit
                            form="leptos-tw-example-form"
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
                        <Button id="leptos-tw-reset" r#type=button::Type::Reset>
                            {t(UtilityText::Reset)}
                        </Button>
                    </div>
                </form>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="visually-hidden"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="visually-hidden" class="text-base font-bold text-slate-950">
                        {t(UtilityText::VisuallyHidden)}
                    </h2>
                    <p class="text-sm text-slate-500">
                        {t(UtilityText::VisuallyHiddenDescription)}
                    </p>
                </div>
                <p class="text-sm leading-6 text-slate-600">
                    <VisuallyHidden id="leptos-tw-visually-hidden-label">
                        {t(UtilityText::VisuallyHiddenLabel)}
                    </VisuallyHidden>
                    {t(UtilityText::VisuallyHiddenDescription)}
                </p>
                <p class="mt-2 text-sm leading-6">
                    <VisuallyHidden id="leptos-tw-focusable-skip" is_focusable=true>
                        <a class="font-semibold text-blue-700 underline" href="#variants">
                            {t(UtilityText::FocusableSkipLink)}
                        </a>
                    </VisuallyHidden>
                </p>
                <VisuallyHiddenAsChild id="leptos-tw-visually-hidden-as-child">
                    <span>{t(UtilityText::AsChildHiddenLabel)}</span>
                </VisuallyHiddenAsChild>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="separator"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="separator" class="text-base font-bold text-slate-950">
                        {t(UtilityText::SeparatorPrimitive)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::SeparatorDescription)}</p>
                </div>
                <Separator id="leptos-tw-separator-horizontal" />
                <div class="flex gap-3 items-stretch text-sm min-h-12 text-slate-600">
                    <span>{t(UtilityText::HorizontalSeparator)}</span>
                    <Separator
                        id="leptos-tw-separator-vertical"
                        orientation=Orientation::Vertical
                    />
                    <span>{t(UtilityText::VerticalSeparator)}</span>
                </div>
                <Separator id="leptos-tw-separator-decorative" decorative=true />
                <p class="text-sm text-slate-500">{t(UtilityText::DecorativeSeparator)}</p>
                <SeparatorAsChild
                    id="leptos-tw-separator-as-child"
                    orientation=Orientation::Vertical
                >
                    <div class="w-0.5 h-8 bg-current text-slate-300"></div>
                </SeparatorAsChild>
                <p class="text-sm text-slate-500">{t(UtilityText::AsChildSeparator)}</p>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg lg:col-span-2 border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="dismissable"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="dismissable" class="text-base font-bold text-slate-950">
                        {t(UtilityText::DismissablePrimitive)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::DismissableNote)}</p>
                </div>
                <dismissable::Region props=dismiss_props>
                    <div>
                        <h4 class="text-sm font-bold text-blue-950">
                            {t(UtilityText::TailwindDismissableRegion)}
                        </h4>
                        <p class="mt-2 max-w-2xl text-sm leading-6 text-blue-900">
                            {t(UtilityText::DismissableCompositionDescription)}
                        </p>
                    </div>
                </dismissable::Region>
                <p class="py-2 px-3 mt-3 text-sm font-medium text-white rounded-md shadow-sm bg-slate-950">
                    {t(dismiss_status)}
                </p>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg lg:col-span-2 border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="errors"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="errors" class="text-base font-bold text-slate-950">
                        {t(UtilityText::ErrorBoundary)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::ErrorBoundaryNote)}</p>
                </div>
                <div class="grid gap-4 md:grid-cols-2">
                    <Boundary>
                        <p class="p-4 text-sm font-medium text-emerald-900 bg-emerald-50 rounded-lg border border-emerald-200 shadow-sm">
                            {t(UtilityText::HealthyChild)}
                        </p>
                    </Boundary>
                    <Boundary>{example_error(error_message)}</Boundary>
                </div>
            </section>
        </div>
    }
}
