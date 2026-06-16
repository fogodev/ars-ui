use std::fmt::{self, Display};

use ars_leptos::prelude::*;

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

    #[translate(en_US = "Submit", pt_BR = "Enviar")]
    Submit,

    #[translate(en_US = "Field and form", pt_BR = "Campo e formulário")]
    FieldForm,

    #[translate(
        en_US = "React Aria-style form fields with labels, helper text, and validation.",
        pt_BR = "Campos de formulário no estilo React Aria com rótulos, texto de ajuda e validação."
    )]
    FieldFormDescription,

    #[translate(en_US = "Account details", pt_BR = "Detalhes da conta")]
    AccountDetails,

    #[translate(
        en_US = "Required fields are announced from their labels and descriptions.",
        pt_BR = "Campos obrigatórios são anunciados a partir de seus rótulos e descrições."
    )]
    RequiredFieldsDescription,

    #[translate(en_US = "Name", pt_BR = "Nome")]
    NameLabel,

    #[translate(en_US = "Enter your full name", pt_BR = "Digite seu nome completo")]
    NamePlaceholder,

    #[translate(en_US = "Email", pt_BR = "E-mail")]
    EmailLabel,

    #[translate(
        en_US = "Use a reachable address.",
        pt_BR = "Use um endereço acessível."
    )]
    EmailDescription,

    #[translate(en_US = "Enter your email", pt_BR = "Digite seu e-mail")]
    EmailPlaceholder,

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

    #[translate(en_US = "Separator", pt_BR = "Separador")]
    SeparatorPrimitive,

    #[translate(
        en_US = "Semantic, vertical, and decorative separators share the same root part.",
        pt_BR = "Separadores semânticos, verticais e decorativos compartilham a mesma parte raiz."
    )]
    SeparatorDescription,

    #[translate(
        en_US = "Horizontal section break",
        pt_BR = "Quebra horizontal de seção"
    )]
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

    #[translate(en_US = "Client-only content", pt_BR = "Conteúdo somente no cliente")]
    ClientOnly,

    #[translate(
        en_US = "Fallback content is replaced after client mount.",
        pt_BR = "O conteúdo fallback é substituído após a montagem no cliente."
    )]
    ClientOnlyDescription,

    #[translate(
        en_US = "Client content mounted",
        pt_BR = "Conteúdo do cliente montado"
    )]
    ClientOnlyMounted,

    #[translate(
        en_US = "Loading client content",
        pt_BR = "Carregando conteúdo do cliente"
    )]
    ClientOnlyFallback,

    #[translate(en_US = "Z-index allocator", pt_BR = "Alocador de z-index")]
    ZIndexAllocator,

    #[translate(
        en_US = "Provider-scoped claims allocate deterministic stacking layers.",
        pt_BR = "Claims no escopo do provider alocam camadas determinísticas."
    )]
    ZIndexAllocatorDescription,

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

    #[translate(en_US = "Heading primitive", pt_BR = "Primitivo de cabeçalho")]
    HeadingPrimitive,

    #[translate(
        en_US = "Heading resolves its level from explicit props or the nearest HeadingContext.",
        pt_BR = "O Heading resolve seu nível a partir de props explícitas ou do HeadingContext mais próximo."
    )]
    HeadingDescription,

    #[translate(en_US = "Level one (h1)", pt_BR = "Nível um (h1)")]
    HeadingLevelOne,

    #[translate(en_US = "Level two (h2)", pt_BR = "Nível dois (h2)")]
    HeadingLevelTwo,

    #[translate(en_US = "Level three (h3)", pt_BR = "Nível três (h3)")]
    HeadingLevelThree,

    #[translate(en_US = "Level four (h4)", pt_BR = "Nível quatro (h4)")]
    HeadingLevelFour,

    #[translate(en_US = "Level five (h5)", pt_BR = "Nível cinco (h5)")]
    HeadingLevelFive,

    #[translate(en_US = "Level six (h6)", pt_BR = "Nível seis (h6)")]
    HeadingLevelSix,

    #[translate(
        en_US = "HeadingLevelProvider sets the starting level. Section bumps the nested level by one.",
        pt_BR = "HeadingLevelProvider define o nível inicial. Section incrementa o nível aninhado em um."
    )]
    HeadingNestingDescription,

    #[translate(
        en_US = "Inside HeadingLevelProvider level=Two (h2)",
        pt_BR = "Dentro de HeadingLevelProvider level=Two (h2)"
    )]
    HeadingProvider,

    #[translate(
        en_US = "Inside Section, bumped to h3",
        pt_BR = "Dentro de Section, promovido para h3"
    )]
    HeadingSection,

    #[translate(en_US = "Landmark primitive", pt_BR = "Primitivo de marco")]
    LandmarkPrimitive,

    #[translate(
        en_US = "Landmark picks the right native element for each WAI-ARIA role.",
        pt_BR = "O Landmark escolhe o elemento nativo certo para cada papel WAI-ARIA."
    )]
    LandmarkDescription,

    #[translate(
        en_US = "Banner (renders as <header>)",
        pt_BR = "Banner (renderiza como <header>)"
    )]
    LandmarkBanner,

    #[translate(
        en_US = "Navigation (renders as <nav>)",
        pt_BR = "Navegação (renderiza como <nav>)"
    )]
    LandmarkNavigation,

    #[translate(
        en_US = "Main (renders as <main>)",
        pt_BR = "Main (renderiza como <main>)"
    )]
    LandmarkMain,

    #[translate(
        en_US = "Complementary (renders as <aside>)",
        pt_BR = "Complementary (renderiza como <aside>)"
    )]
    LandmarkComplementary,

    #[translate(
        en_US = "ContentInfo (renders as <footer>)",
        pt_BR = "ContentInfo (renderiza como <footer>)"
    )]
    LandmarkContentInfo,

    #[translate(
        en_US = "Form (renders as <form>)",
        pt_BR = "Form (renderiza como <form>)"
    )]
    LandmarkForm,

    #[translate(
        en_US = "Region (renders as <section>)",
        pt_BR = "Region (renderiza como <section>)"
    )]
    LandmarkRegion,

    #[translate(
        en_US = "Search (fallback: <div role=\"search\">)",
        pt_BR = "Search (fallback: <div role=\"search\">)"
    )]
    LandmarkSearch,

    #[translate(en_US = "Highlight primitive", pt_BR = "Primitivo de destaque")]
    HighlightPrimitive,

    #[translate(
        en_US = "Highlight splits text into matched/unmatched chunks with locale-aware case folding.",
        pt_BR = "O Highlight divide o texto em trechos correspondentes/não correspondentes com dobramento de caso sensível à localidade."
    )]
    HighlightDescription,
}

struct ExampleError(TextProp);

impl fmt::Debug for ExampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ExampleError").finish_non_exhaustive()
    }
}

impl Display for ExampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.get())
    }
}

impl std::error::Error for ExampleError {}

fn example_error(message: impl Into<TextProp>) -> Result<&'static str, ExampleError> {
    Err(ExampleError(message.into()))
}

#[component]
fn ZIndexProbe(id: &'static str) -> impl IntoView {
    let context = use_context::<ZIndexContext>().expect("z-index context should be provided");

    let claim = context.allocate_claim();

    view! {
        <span
            id=id
            class="py-1 px-2 text-xs rounded border border-slate-300"
            data-z=claim.value().to_string()
        >
            {claim.value().to_string()}
        </span>
    }
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
                aria-labelledby="field-form"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="field-form" class="text-base font-bold text-slate-950">
                        {t(UtilityText::FieldForm)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::FieldFormDescription)}</p>
                </div>
                <Form
                    id="leptos-tw-field-form-demo"
                    action="/account"
                    on_submit=Callback::new(|()| ())
                    class="grid gap-4 max-w-md [&_fieldset]:grid [&_fieldset]:gap-4 [&_fieldset]:m-0 [&_fieldset]:p-4 [&_fieldset]:rounded-lg [&_fieldset]:border [&_fieldset]:border-slate-300 [&_legend]:px-1.5 [&_legend]:font-bold **:data-[ars-part=content]:grid **:data-[ars-part=content]:gap-3 **:data-[ars-part=description]:text-sm **:data-[ars-part=description]:text-slate-500 **:data-[ars-part=status-region]:text-sm **:data-[ars-part=status-region]:font-semibold **:data-[ars-part=status-region]:text-emerald-700"
                >
                    <Fieldset id="leptos-tw-fieldset-demo">
                        <fieldset::Legend>{t(UtilityText::AccountDetails)}</fieldset::Legend>
                        <fieldset::Description>
                            {t(UtilityText::RequiredFieldsDescription)}
                        </fieldset::Description>
                        <fieldset::Content>
                            <Field id="leptos-tw-name-field" required=true class="grid gap-2">
                                <field::Label>{t(UtilityText::NameLabel)}</field::Label>
                                <field::Input
                                    class="py-2 px-3 text-sm rounded-md border shadow-sm border-slate-300"
                                    name="name"
                                    placeholder=t(UtilityText::NamePlaceholder)
                                />
                            </Field>
                            <Field
                                id="leptos-tw-email-field"
                                name="email"
                                required=true
                                class="grid gap-2"
                            >
                                <field::Label>{t(UtilityText::EmailLabel)}</field::Label>
                                <field::Description>
                                    {t(UtilityText::EmailDescription)}
                                </field::Description>
                                <field::Input
                                    class="py-2 px-3 text-sm rounded-md border shadow-sm border-slate-300"
                                    r#type=field::InputType::Email
                                    name="email"
                                    placeholder=t(UtilityText::EmailPlaceholder)
                                />
                            </Field>
                        </fieldset::Content>
                    </Fieldset>
                    <div class="flex flex-wrap gap-3">
                        <Button r#type=button::Type::Submit>{t(UtilityText::Submit)}</Button>
                        <Button r#type=button::Type::Reset variant=button::Variant::Secondary>
                            {t(UtilityText::Reset)}
                        </Button>
                    </div>
                </Form>
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
                class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="client-only-z-index"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="client-only-z-index" class="text-base font-bold text-slate-950">
                        {t(UtilityText::ClientOnly)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::ClientOnlyDescription)}</p>
                </div>
                <ClientOnly fallback=|| view! { <span>{t(UtilityText::ClientOnlyFallback)}</span> }>
                    <span class="text-sm text-slate-700">{t(UtilityText::ClientOnlyMounted)}</span>
                </ClientOnly>
                <h3 class="mt-4 text-sm font-semibold text-slate-900">
                    {t(UtilityText::ZIndexAllocator)}
                </h3>
                <p class="text-sm text-slate-500">{t(UtilityText::ZIndexAllocatorDescription)}</p>
                <div class="flex gap-2 mt-2">
                    <ZIndexAllocatorProvider>
                        <ZIndexProbe id="leptos-tw-z-index-first" />
                        <ZIndexProbe id="leptos-tw-z-index-second" />
                    </ZIndexAllocatorProvider>
                </div>
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
                    <ErrorBoundary>
                        <p class="p-4 text-sm font-medium text-emerald-900 bg-emerald-50 rounded-lg border border-emerald-200 shadow-sm">
                            {t(UtilityText::HealthyChild)}
                        </p>
                    </ErrorBoundary>
                    <ErrorBoundary>{example_error(error_message)}</ErrorBoundary>
                </div>
            </section>
            <section
                class="p-6 bg-white rounded-2xl border shadow-sm border-slate-200"
                aria-labelledby="heading-primitive"
            >
                <div class="mb-4">
                    <h2 id="heading-primitive" class="text-base font-bold text-slate-950">
                        {t(UtilityText::HeadingPrimitive)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::HeadingDescription)}</p>
                </div>
                <div class="space-y-2 text-slate-900">
                    <Heading
                        id="leptos-tailwind-heading-h1"
                        level=heading::Level::One
                        class="text-4xl font-bold"
                    >
                        {t(UtilityText::HeadingLevelOne)}
                    </Heading>
                    <Heading
                        id="leptos-tailwind-heading-h2"
                        level=heading::Level::Two
                        class="text-3xl font-bold"
                    >
                        {t(UtilityText::HeadingLevelTwo)}
                    </Heading>
                    <Heading
                        id="leptos-tailwind-heading-h3"
                        level=heading::Level::Three
                        class="text-2xl font-bold"
                    >
                        {t(UtilityText::HeadingLevelThree)}
                    </Heading>
                    <Heading
                        id="leptos-tailwind-heading-h4"
                        level=heading::Level::Four
                        class="text-xl font-bold"
                    >
                        {t(UtilityText::HeadingLevelFour)}
                    </Heading>
                    <Heading
                        id="leptos-tailwind-heading-h5"
                        level=heading::Level::Five
                        class="text-lg font-bold"
                    >
                        {t(UtilityText::HeadingLevelFive)}
                    </Heading>
                    <Heading
                        id="leptos-tailwind-heading-h6"
                        level=heading::Level::Six
                        class="text-base font-bold"
                    >
                        {t(UtilityText::HeadingLevelSix)}
                    </Heading>
                    <p class="pt-2 text-sm text-slate-500">
                        {t(UtilityText::HeadingNestingDescription)}
                    </p>
                    <HeadingLevelProvider level=heading::Level::Two>
                        <Heading id="leptos-tailwind-heading-provided" class="text-3xl font-bold">
                            {t(UtilityText::HeadingProvider)}
                        </Heading>
                        <heading::Section>
                            <Heading
                                id="leptos-tailwind-heading-section"
                                class="text-2xl font-bold"
                            >
                                {t(UtilityText::HeadingSection)}
                            </Heading>
                        </heading::Section>
                    </HeadingLevelProvider>
                </div>
            </section>
            <section
                class="p-6 bg-white rounded-2xl border shadow-sm border-slate-200"
                aria-labelledby="landmark-primitive"
            >
                <div class="mb-4">
                    <h2 id="landmark-primitive" class="text-base font-bold text-slate-950">
                        {t(UtilityText::LandmarkPrimitive)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::LandmarkDescription)}</p>
                </div>
                <div class="space-y-3 text-slate-900">
                    <Landmark
                        id="leptos-tailwind-landmark-banner"
                        role=landmark::Role::Banner
                        messages=landmark::Messages {
                            label: MessageFn::static_str("Page banner"),
                        }
                        class="block py-2 px-3 bg-violet-50 rounded-md border-2 border-violet-400"
                    >
                        {t(UtilityText::LandmarkBanner)}
                    </Landmark>
                    <Landmark
                        id="leptos-tailwind-landmark-navigation"
                        role=landmark::Role::Navigation
                        messages=landmark::Messages {
                            label: MessageFn::static_str("Primary navigation"),
                        }
                        class="block py-2 px-3 bg-blue-50 rounded-md border-2 border-blue-400"
                    >
                        {t(UtilityText::LandmarkNavigation)}
                    </Landmark>
                    <Landmark
                        id="leptos-tailwind-landmark-main"
                        role=landmark::Role::Main
                        messages=landmark::Messages {
                            label: MessageFn::static_str("Main content"),
                        }
                        class="block py-2 px-3 bg-emerald-50 rounded-md border-2 border-emerald-400"
                    >
                        {t(UtilityText::LandmarkMain)}
                    </Landmark>
                    <Landmark
                        id="leptos-tailwind-landmark-complementary"
                        role=landmark::Role::Complementary
                        messages=landmark::Messages {
                            label: MessageFn::static_str("Related content"),
                        }
                        class="block py-2 px-3 bg-yellow-50 rounded-md border-2 border-yellow-400"
                    >
                        {t(UtilityText::LandmarkComplementary)}
                    </Landmark>
                    <Landmark
                        id="leptos-tailwind-landmark-contentinfo"
                        role=landmark::Role::ContentInfo
                        messages=landmark::Messages {
                            label: MessageFn::static_str("Page footer"),
                        }
                        class="block py-2 px-3 rounded-md border-2 border-slate-400 bg-slate-50"
                    >
                        {t(UtilityText::LandmarkContentInfo)}
                    </Landmark>
                    <Landmark
                        id="leptos-tailwind-landmark-form"
                        role=landmark::Role::Form
                        messages=landmark::Messages {
                            label: MessageFn::static_str("Subscribe form"),
                        }
                        class="block py-2 px-3 bg-teal-50 rounded-md border-2 border-teal-400"
                    >
                        {t(UtilityText::LandmarkForm)}
                    </Landmark>
                    <Landmark
                        id="leptos-tailwind-landmark-region"
                        role=landmark::Role::Region
                        messages=landmark::Messages {
                            label: MessageFn::static_str("Sidebar region"),
                        }
                        class="block py-2 px-3 bg-rose-50 rounded-md border-2 border-rose-400"
                    >
                        {t(UtilityText::LandmarkRegion)}
                    </Landmark>
                    <Landmark
                        id="leptos-tailwind-landmark-search"
                        role=landmark::Role::Search
                        messages=landmark::Messages {
                            label: MessageFn::static_str("Site search"),
                        }
                        class="block py-2 px-3 bg-orange-50 rounded-md border-2 border-orange-400"
                    >
                        {t(UtilityText::LandmarkSearch)}
                    </Landmark>
                </div>
            </section>
            <section
                class="p-6 bg-white rounded-2xl border shadow-sm border-slate-200"
                aria-labelledby="highlight-primitive"
            >
                <div class="mb-4">
                    <h2 id="highlight-primitive" class="text-base font-bold text-slate-950">
                        {t(UtilityText::HighlightPrimitive)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(UtilityText::HighlightDescription)}</p>
                </div>
                <p class="text-slate-900">
                    <Highlight
                        query=vec!["highlighted".to_string()]
                        text="Hello highlighted world!".to_string()
                    />
                </p>
            </section>
        </div>
    }
}
