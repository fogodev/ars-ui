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

const LANDMARK_BANNER_STYLE: &str = "display: block; border: 2px solid #a78bfa; background: #f5f3ff; padding: 0.5rem 0.75rem; margin: 0.4rem 0; border-radius: 0.375rem;";
const LANDMARK_NAVIGATION_STYLE: &str = "display: block; border: 2px solid #60a5fa; background: #eff6ff; padding: 0.5rem 0.75rem; margin: 0.4rem 0; border-radius: 0.375rem;";
const LANDMARK_MAIN_STYLE: &str = "display: block; border: 2px solid #4ade80; background: #f0fdf4; padding: 0.5rem 0.75rem; margin: 0.4rem 0; border-radius: 0.375rem;";
const LANDMARK_COMPLEMENTARY_STYLE: &str = "display: block; border: 2px solid #facc15; background: #fefce8; padding: 0.5rem 0.75rem; margin: 0.4rem 0; border-radius: 0.375rem;";
const LANDMARK_CONTENTINFO_STYLE: &str = "display: block; border: 2px solid #94a3b8; background: #f8fafc; padding: 0.5rem 0.75rem; margin: 0.4rem 0; border-radius: 0.375rem;";
const LANDMARK_FORM_STYLE: &str = "display: block; border: 2px solid #2dd4bf; background: #f0fdfa; padding: 0.5rem 0.75rem; margin: 0.4rem 0; border-radius: 0.375rem;";
const LANDMARK_REGION_STYLE: &str = "display: block; border: 2px solid #f472b6; background: #fdf2f8; padding: 0.5rem 0.75rem; margin: 0.4rem 0; border-radius: 0.375rem;";
const LANDMARK_SEARCH_STYLE: &str = "display: block; border: 2px solid #fb923c; background: #fff7ed; padding: 0.5rem 0.75rem; margin: 0.4rem 0; border-radius: 0.375rem;";

#[component]
fn ZIndexProbe(id: &'static str) -> impl IntoView {
    let context = use_context::<ZIndexContext>().expect("z-index context should be provided");

    let claim = context.allocate_claim();

    view! {
        <span id=id class="z-index-chip" data-z=claim.value().to_string()>
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
            <section class="showcase-panel" aria-labelledby="field-form">
                <div class="panel-heading">
                    <h2 id="field-form">{t(UtilityText::FieldForm)}</h2>
                    <p class="panel-note">{t(UtilityText::FieldFormDescription)}</p>
                </div>
                <Form
                    id="leptos-css-field-form-demo"
                    action="/account"
                    class="field-form-demo"
                    on_submit=Callback::new(|()| ())
                >
                    <Fieldset id="leptos-css-fieldset-demo">
                        <fieldset::Legend>{t(UtilityText::AccountDetails)}</fieldset::Legend>
                        <fieldset::Description>
                            {t(UtilityText::RequiredFieldsDescription)}
                        </fieldset::Description>
                        <fieldset::Content>
                            <Field
                                id="leptos-css-name-field"
                                required=true
                                class="field-form-field"
                            >
                                <field::Label>{t(UtilityText::NameLabel)}</field::Label>
                                <field::Input
                                    class="field-form-input"
                                    name="name"
                                    placeholder=t(UtilityText::NamePlaceholder)
                                />
                            </Field>
                            <Field
                                id="leptos-css-email-field"
                                name="email"
                                required=true
                                class="field-form-field"
                            >
                                <field::Label>{t(UtilityText::EmailLabel)}</field::Label>
                                <field::Description>
                                    {t(UtilityText::EmailDescription)}
                                </field::Description>
                                <field::Input
                                    class="field-form-input"
                                    r#type=field::InputType::Email
                                    name="email"
                                    placeholder=t(UtilityText::EmailPlaceholder)
                                />
                            </Field>
                        </fieldset::Content>
                    </Fieldset>
                    <div class="button-row">
                        <Button r#type=button::Type::Submit>{t(UtilityText::Submit)}</Button>
                        <Button r#type=button::Type::Reset variant=button::Variant::Secondary>
                            {t(UtilityText::Reset)}
                        </Button>
                    </div>
                </Form>
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
            <section class="showcase-panel" aria-labelledby="client-only-z-index">
                <div class="panel-heading">
                    <h2 id="client-only-z-index">{t(UtilityText::ClientOnly)}</h2>
                    <p class="panel-note">{t(UtilityText::ClientOnlyDescription)}</p>
                </div>
                <ClientOnly fallback=|| view! { <span>{t(UtilityText::ClientOnlyFallback)}</span> }>
                    <span>{t(UtilityText::ClientOnlyMounted)}</span>
                </ClientOnly>
                <div class="panel-heading">
                    <h3>{t(UtilityText::ZIndexAllocator)}</h3>
                    <p class="panel-note">{t(UtilityText::ZIndexAllocatorDescription)}</p>
                </div>
                <ZIndexAllocatorProvider>
                    <ZIndexProbe id="leptos-css-z-index-first" />
                    <ZIndexProbe id="leptos-css-z-index-second" />
                </ZIndexAllocatorProvider>
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
                    <ErrorBoundary>
                        <p class="healthy-boundary">{t(UtilityText::HealthyChild)}</p>
                    </ErrorBoundary>
                    <ErrorBoundary>{example_error(error_message)}</ErrorBoundary>
                </div>
            </section>
            <section class="showcase-panel wide" aria-labelledby="heading-primitive">
                <div class="panel-heading">
                    <h2 id="heading-primitive">{t(UtilityText::HeadingPrimitive)}</h2>
                    <p class="panel-note">{t(UtilityText::HeadingDescription)}</p>
                </div>
                <Heading
                    id="leptos-css-heading-h1"
                    level=heading::Level::One
                    attr:style="font-size: 2.25rem; font-weight: 700; line-height: 1.2; margin: 0.4rem 0;"
                >
                    {t(UtilityText::HeadingLevelOne)}
                </Heading>
                <Heading
                    id="leptos-css-heading-h2"
                    level=heading::Level::Two
                    attr:style="font-size: 1.875rem; font-weight: 700; line-height: 1.25; margin: 0.4rem 0;"
                >
                    {t(UtilityText::HeadingLevelTwo)}
                </Heading>
                <Heading
                    id="leptos-css-heading-h3"
                    level=heading::Level::Three
                    attr:style="font-size: 1.5rem; font-weight: 700; line-height: 1.3; margin: 0.4rem 0;"
                >
                    {t(UtilityText::HeadingLevelThree)}
                </Heading>
                <Heading
                    id="leptos-css-heading-h4"
                    level=heading::Level::Four
                    attr:style="font-size: 1.25rem; font-weight: 700; line-height: 1.35; margin: 0.4rem 0;"
                >
                    {t(UtilityText::HeadingLevelFour)}
                </Heading>
                <Heading
                    id="leptos-css-heading-h5"
                    level=heading::Level::Five
                    attr:style="font-size: 1.125rem; font-weight: 700; line-height: 1.4; margin: 0.4rem 0;"
                >
                    {t(UtilityText::HeadingLevelFive)}
                </Heading>
                <Heading
                    id="leptos-css-heading-h6"
                    level=heading::Level::Six
                    attr:style="font-size: 1rem; font-weight: 700; line-height: 1.5; margin: 0.4rem 0;"
                >
                    {t(UtilityText::HeadingLevelSix)}
                </Heading>
                <p class="panel-note">{t(UtilityText::HeadingNestingDescription)}</p>
                <HeadingLevelProvider level=heading::Level::Two>
                    <Heading
                        id="leptos-css-heading-provided"
                        attr:style="font-size: 1.875rem; font-weight: 700; line-height: 1.25; margin: 0.4rem 0;"
                    >
                        {t(UtilityText::HeadingProvider)}
                    </Heading>
                    <heading::Section>
                        <Heading
                            id="leptos-css-heading-section"
                            attr:style="font-size: 1.5rem; font-weight: 700; line-height: 1.3; margin: 0.4rem 0;"
                        >
                            {t(UtilityText::HeadingSection)}
                        </Heading>
                    </heading::Section>
                </HeadingLevelProvider>
            </section>
            <section class="showcase-panel wide" aria-labelledby="landmark-primitive">
                <div class="panel-heading">
                    <h2 id="landmark-primitive">{t(UtilityText::LandmarkPrimitive)}</h2>
                    <p class="panel-note">{t(UtilityText::LandmarkDescription)}</p>
                </div>
                <Landmark
                    id="leptos-css-landmark-banner"
                    role=landmark::Role::Banner
                    messages=landmark::Messages {
                        label: MessageFn::static_str("Page banner"),
                    }
                    attr:style=LANDMARK_BANNER_STYLE
                >
                    {t(UtilityText::LandmarkBanner)}
                </Landmark>
                <Landmark
                    id="leptos-css-landmark-navigation"
                    role=landmark::Role::Navigation
                    messages=landmark::Messages {
                        label: MessageFn::static_str("Primary navigation"),
                    }
                    attr:style=LANDMARK_NAVIGATION_STYLE
                >
                    {t(UtilityText::LandmarkNavigation)}
                </Landmark>
                <Landmark
                    id="leptos-css-landmark-main"
                    role=landmark::Role::Main
                    messages=landmark::Messages {
                        label: MessageFn::static_str("Main content"),
                    }
                    attr:style=LANDMARK_MAIN_STYLE
                >
                    {t(UtilityText::LandmarkMain)}
                </Landmark>
                <Landmark
                    id="leptos-css-landmark-complementary"
                    role=landmark::Role::Complementary
                    messages=landmark::Messages {
                        label: MessageFn::static_str("Related content"),
                    }
                    attr:style=LANDMARK_COMPLEMENTARY_STYLE
                >
                    {t(UtilityText::LandmarkComplementary)}
                </Landmark>
                <Landmark
                    id="leptos-css-landmark-contentinfo"
                    role=landmark::Role::ContentInfo
                    messages=landmark::Messages {
                        label: MessageFn::static_str("Page footer"),
                    }
                    attr:style=LANDMARK_CONTENTINFO_STYLE
                >
                    {t(UtilityText::LandmarkContentInfo)}
                </Landmark>
                <Landmark
                    id="leptos-css-landmark-form"
                    role=landmark::Role::Form
                    messages=landmark::Messages {
                        label: MessageFn::static_str("Subscribe form"),
                    }
                    attr:style=LANDMARK_FORM_STYLE
                >
                    {t(UtilityText::LandmarkForm)}
                </Landmark>
                <Landmark
                    id="leptos-css-landmark-region"
                    role=landmark::Role::Region
                    messages=landmark::Messages {
                        label: MessageFn::static_str("Sidebar region"),
                    }
                    attr:style=LANDMARK_REGION_STYLE
                >
                    {t(UtilityText::LandmarkRegion)}
                </Landmark>
                <Landmark
                    id="leptos-css-landmark-search"
                    role=landmark::Role::Search
                    messages=landmark::Messages {
                        label: MessageFn::static_str("Site search"),
                    }
                    attr:style=LANDMARK_SEARCH_STYLE
                >
                    {t(UtilityText::LandmarkSearch)}
                </Landmark>
            </section>
            <section class="showcase-panel wide" aria-labelledby="highlight-primitive">
                <div class="panel-heading">
                    <h2 id="highlight-primitive">{t(UtilityText::HighlightPrimitive)}</h2>
                    <p class="panel-note">{t(UtilityText::HighlightDescription)}</p>
                </div>
                <Highlight
                    query=vec!["highlighted".to_string()]
                    text="Hello highlighted world!".to_string()
                />
            </section>
        </div>
    }
}
