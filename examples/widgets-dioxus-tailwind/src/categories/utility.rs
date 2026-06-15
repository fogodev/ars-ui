use ars_dioxus::prelude::*;

#[derive(Clone, Debug, Translate, PartialEq)]
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

#[component]
fn ExampleErrorChild() -> Element {
    Err(error_boundary::CapturedError::from_display(t(UtilityText::ExampleChildError)).into())
}

#[component]
fn ZIndexProbe(id: &'static str) -> Element {
    let context = try_use_context::<ZIndexContext>().expect("z-index context should be provided");

    let claim = context.allocate_claim();

    rsx! {
        span {
            id,
            class: "rounded border border-slate-300 px-2 py-1 text-xs",
            "data-z": "{claim.value()}",
            "{claim.value()}"
        }
    }
}

#[component]
pub(crate) fn UtilityPanel() -> Element {
    let dismiss_status = use_signal_sync(|| UtilityText::DismissInitial);

    let dismiss_props = dismissable::Props::new().on_dismiss(move |reason| {
        let mut dismiss_status = dismiss_status;

        dismiss_status.set(UtilityText::DismissReason {
            reason: format!("{reason:?}"),
        });
    });

    rsx! {
        div { class: "mt-5 grid gap-5 lg:grid-cols-2",
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-xl shadow-slate-900/10 lg:col-span-2",
                "aria-labelledby": "variants",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "variants",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::ButtonVariants)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::ButtonVariantsNote)} }
                }
                div { class: "flex flex-wrap gap-3",
                    Button { id: "dioxus-tw-default", {t(UtilityText::DefaultButton)} }
                    Button {
                        id: "dioxus-tw-primary",
                        variant: button::Variant::Primary,
                        {t(UtilityText::PrimaryButton)}
                    }
                    Button {
                        id: "dioxus-tw-secondary",
                        variant: button::Variant::Secondary,
                        {t(UtilityText::SecondaryButton)}
                    }
                    Button {
                        id: "dioxus-tw-destructive",
                        variant: button::Variant::Destructive,
                        {t(UtilityText::DestructiveButton)}
                    }
                    Button {
                        id: "dioxus-tw-outline",
                        variant: button::Variant::Outline,
                        {t(UtilityText::OutlineButton)}
                    }
                    Button {
                        id: "dioxus-tw-ghost",
                        variant: button::Variant::Ghost,
                        {t(UtilityText::GhostButton)}
                    }
                    Button { id: "dioxus-tw-link", variant: button::Variant::Link,
                        {t(UtilityText::LinkButton)}
                    }
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
                "aria-labelledby": "sizes",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "sizes",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::ButtonSizes)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::ButtonSizeTokens)} }
                }
                div { class: "flex flex-wrap gap-3",
                    Button { id: "dioxus-tw-sm", size: button::Size::Sm, {t(UtilityText::SmallButton)} }
                    Button { id: "dioxus-tw-md", size: button::Size::Md,
                        {t(UtilityText::MediumButton)}
                    }
                    Button { id: "dioxus-tw-lg", size: button::Size::Lg, {t(UtilityText::LargeButton)} }
                    Button { id: "dioxus-tw-icon", size: button::Size::Icon,
                        {t(UtilityText::IconButton)}
                    }
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
                "aria-labelledby": "states",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "states",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::ButtonStates)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::ButtonStatesNote)} }
                }
                div { class: "flex flex-wrap gap-3",
                    Button { id: "dioxus-tw-disabled", disabled: true,
                        {t(UtilityText::DisabledButton)}
                    }
                    Button { id: "dioxus-tw-loading", loading: true, {t(UtilityText::LoadingButton)} }
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10 transition hover:-translate-y-0.5 hover:shadow-xl hover:shadow-slate-900/15",
                "aria-labelledby": "as-child",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "as-child",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::AsChild)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::AsChildNote)} }
                }
                div { class: "flex flex-wrap gap-3",
                    ButtonAsChild {
                        id: "dioxus-tw-as-child-docs",
                        variant: button::Variant::Link,
                        class: "group",
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(UtilityText::DocsLinkRoot)} }
                        },
                    }
                    ButtonAsChild {
                        id: "dioxus-tw-as-child-primary",
                        variant: button::Variant::Primary,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(UtilityText::AnchorAsPrimary)} }
                        },
                    }
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
                "aria-labelledby": "forms",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "forms",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::Forms)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::FormsNote)} }
                }
                form { id: "dioxus-tw-example-form",
                    div { class: "flex flex-wrap gap-3",
                        Button {
                            id: "dioxus-tw-submit",
                            r#type: button::Type::Submit,
                            form: "dioxus-tw-example-form",
                            name: "intent",
                            value: "save",
                            form_action: "/submit",
                            form_method: button::FormMethod::Post,
                            form_enc_type: button::FormEncType::UrlEncoded,
                            form_target: button::FormTarget::Self_,
                            form_no_validate: true,
                            {t(UtilityText::SubmitOverride)}
                        }
                        Button {
                            id: "dioxus-tw-reset",
                            r#type: button::Type::Reset,
                            {t(UtilityText::Reset)}
                        }
                    }
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
                "aria-labelledby": "field-form",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "field-form",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::FieldForm)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::FieldFormDescription)} }
                }
                Form {
                    id: "dioxus-tw-field-form-demo",
                    action: "/account",
                    on_submit: move |()| {},
                    class: "grid max-w-md gap-4 [&_fieldset]:grid [&_fieldset]:m-0 [&_fieldset]:gap-4 [&_fieldset]:rounded-lg [&_fieldset]:border [&_fieldset]:border-slate-300 [&_fieldset]:p-4 [&_legend]:px-1.5 [&_legend]:font-bold **:data-[ars-part=content]:grid **:data-[ars-part=content]:gap-3 **:data-[ars-part=description]:text-sm **:data-[ars-part=description]:text-slate-500 **:data-[ars-part=status-region]:text-sm **:data-[ars-part=status-region]:font-semibold **:data-[ars-part=status-region]:text-emerald-700",
                    Fieldset { id: "dioxus-tw-fieldset-demo",
                        fieldset::Legend { {t(UtilityText::AccountDetails)} }
                        fieldset::Description { {t(UtilityText::RequiredFieldsDescription)} }
                        fieldset::Content {
                            Field {
                                id: "dioxus-tw-name-field",
                                required: true,
                                class: "grid gap-2",
                                field::Label { {t(UtilityText::NameLabel)} }
                                field::Input {
                                    class: "rounded-md border border-slate-300 px-3 py-2 text-sm shadow-sm",
                                    name: "name",
                                    placeholder: t(UtilityText::NamePlaceholder),
                                }
                            }
                            Field {
                                id: "dioxus-tw-email-field",
                                name: "email",
                                required: true,
                                class: "grid gap-2",
                                field::Label { {t(UtilityText::EmailLabel)} }
                                field::Description { {t(UtilityText::EmailDescription)} }
                                field::Input {
                                    class: "rounded-md border border-slate-300 px-3 py-2 text-sm shadow-sm",
                                    r#type: field::InputType::Email,
                                    name: "email",
                                    placeholder: t(UtilityText::EmailPlaceholder),
                                }
                            }
                        }
                    }
                    div { class: "flex flex-wrap gap-3",
                        Button {
                            class: "rounded-md bg-black px-4 py-2.5 text-sm font-bold text-white shadow-lg shadow-slate-900/20",
                            r#type: button::Type::Submit,
                            {t(UtilityText::Submit)}
                        }
                        Button {
                            class: "rounded-md bg-slate-200 px-4 py-2.5 text-sm font-bold text-slate-950 shadow-lg shadow-slate-900/10",
                            r#type: button::Type::Reset,
                            {t(UtilityText::Reset)}
                        }
                    }
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
                "aria-labelledby": "visually-hidden",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "visually-hidden",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::VisuallyHidden)}
                    }
                    p { class: "text-sm text-slate-500",
                        {t(UtilityText::VisuallyHiddenDescription)}
                    }
                }
                p { class: "text-sm leading-6 text-slate-600",
                    VisuallyHidden { id: "dioxus-tw-visually-hidden-label",
                        {t(UtilityText::VisuallyHiddenLabel)}
                    }
                    {t(UtilityText::VisuallyHiddenDescription)}
                }
                p { class: "mt-2 text-sm leading-6",
                    VisuallyHidden { id: "dioxus-tw-focusable-skip", is_focusable: true,
                        a {
                            class: "font-semibold text-blue-700 underline",
                            href: "#variants",
                            {t(UtilityText::FocusableSkipLink)}
                        }
                    }
                }
                VisuallyHiddenAsChild {
                    id: "dioxus-tw-visually-hidden-as-child",
                    render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                        span { ..slot.attrs,{t(UtilityText::AsChildHiddenLabel)} }
                    },
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
                "aria-labelledby": "separator",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "separator",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::SeparatorPrimitive)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::SeparatorDescription)} }
                }
                Separator { id: "dioxus-tw-separator-horizontal" }
                div { class: "flex min-h-12 items-stretch gap-3 text-sm text-slate-600",
                    span { {t(UtilityText::HorizontalSeparator)} }
                    Separator {
                        id: "dioxus-tw-separator-vertical",
                        orientation: Orientation::Vertical,
                    }
                    span { {t(UtilityText::VerticalSeparator)} }
                }
                Separator { id: "dioxus-tw-separator-decorative", decorative: true }
                p { class: "text-sm text-slate-500", {t(UtilityText::DecorativeSeparator)} }
                SeparatorAsChild {
                    id: "dioxus-tw-separator-as-child",
                    orientation: Orientation::Vertical,
                    render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                        div { class: "h-8 w-0.5 bg-current text-slate-300", ..slot.attrs }
                    },
                }
                p { class: "text-sm text-slate-500", {t(UtilityText::AsChildSeparator)} }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
                "aria-labelledby": "client-only-z-index",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "client-only-z-index",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::ClientOnly)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::ClientOnlyDescription)} }
                }
                ClientOnly {
                    fallback: rsx! {
                        span { {t(UtilityText::ClientOnlyFallback)} }
                    },
                    span { class: "text-sm text-slate-700", {t(UtilityText::ClientOnlyMounted)} }
                }
                h3 { class: "mt-4 text-sm font-semibold text-slate-900",
                    {t(UtilityText::ZIndexAllocator)}
                }
                p { class: "text-sm text-slate-500", {t(UtilityText::ZIndexAllocatorDescription)} }
                div { class: "mt-2 flex gap-2",
                    ZIndexAllocatorProvider {
                        ZIndexProbe { id: "dioxus-tw-z-index-first" }
                        ZIndexProbe { id: "dioxus-tw-z-index-second" }
                    }
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10 transition hover:-translate-y-0.5 hover:shadow-xl hover:shadow-slate-900/15 lg:col-span-2",
                "aria-labelledby": "dismissable",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "dismissable",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::DismissablePrimitive)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::DismissableNote)} }
                }
                dismissable::Region { props: dismiss_props,
                    div { class: "dismissable-card",
                        h3 { class: "text-sm font-bold text-blue-950",
                            {t(UtilityText::TailwindDismissableRegion)}
                        }
                        p { class: "mt-2 max-w-2xl text-sm leading-6 text-blue-900",
                            {t(UtilityText::DismissableCompositionDescription)}
                        }
                    }
                }
                p { class: "dismissable-status mt-3 rounded-md bg-slate-950 px-3 py-2 text-sm font-medium text-white shadow-sm",
                    {t(dismiss_status())}
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10 lg:col-span-2",
                "aria-labelledby": "errors",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "errors",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::ErrorBoundary)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::ErrorBoundaryNote)} }
                }
                div { class: "grid gap-4 md:grid-cols-2",
                    ErrorBoundary {
                        p { class: "rounded-lg border border-emerald-200 bg-emerald-50 p-4 text-sm font-medium text-emerald-900 shadow-sm",
                            {t(UtilityText::HealthyChild)}
                        }
                    }
                    ErrorBoundary { ExampleErrorChild {} }
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
                "aria-labelledby": "heading-primitive",
                div { class: "mb-4",
                    h2 {
                        id: "heading-primitive",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::HeadingPrimitive)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::HeadingDescription)} }
                }
                div { class: "space-y-2 text-slate-900",
                    Heading {
                        id: "dioxus-tailwind-heading-h1",
                        level: heading::Level::One,
                        class: "text-4xl font-bold",
                        {t(UtilityText::HeadingLevelOne)}
                    }
                    Heading {
                        id: "dioxus-tailwind-heading-h2",
                        level: heading::Level::Two,
                        class: "text-3xl font-bold",
                        {t(UtilityText::HeadingLevelTwo)}
                    }
                    Heading {
                        id: "dioxus-tailwind-heading-h3",
                        level: heading::Level::Three,
                        class: "text-2xl font-bold",
                        {t(UtilityText::HeadingLevelThree)}
                    }
                    Heading {
                        id: "dioxus-tailwind-heading-h4",
                        level: heading::Level::Four,
                        class: "text-xl font-bold",
                        {t(UtilityText::HeadingLevelFour)}
                    }
                    Heading {
                        id: "dioxus-tailwind-heading-h5",
                        level: heading::Level::Five,
                        class: "text-lg font-bold",
                        {t(UtilityText::HeadingLevelFive)}
                    }
                    Heading {
                        id: "dioxus-tailwind-heading-h6",
                        level: heading::Level::Six,
                        class: "text-base font-bold",
                        {t(UtilityText::HeadingLevelSix)}
                    }
                    p { class: "pt-2 text-sm text-slate-500",
                        {t(UtilityText::HeadingNestingDescription)}
                    }
                    HeadingLevelProvider { level: heading::Level::Two,
                        Heading {
                            id: "dioxus-tailwind-heading-provided",
                            class: "text-3xl font-bold",
                            {t(UtilityText::HeadingProvider)}
                        }
                        heading::Section {
                            Heading {
                                id: "dioxus-tailwind-heading-section",
                                class: "text-2xl font-bold",
                                {t(UtilityText::HeadingSection)}
                            }
                        }
                    }
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
                "aria-labelledby": "landmark-primitive",
                div { class: "mb-4",
                    h2 {
                        id: "landmark-primitive",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::LandmarkPrimitive)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::LandmarkDescription)} }
                }
                div { class: "space-y-3 text-slate-900",
                    Landmark {
                        id: "dioxus-tailwind-landmark-banner",
                        role: landmark::Role::Banner,
                        messages: landmark::Messages {
                            label: MessageFn::static_str("Page banner"),
                        },
                        class: "block rounded-md border-2 border-violet-400 bg-violet-50 px-3 py-2",
                        {t(UtilityText::LandmarkBanner)}
                    }
                    Landmark {
                        id: "dioxus-tailwind-landmark-navigation",
                        role: landmark::Role::Navigation,
                        messages: landmark::Messages {
                            label: MessageFn::static_str("Primary navigation"),
                        },
                        class: "block rounded-md border-2 border-blue-400 bg-blue-50 px-3 py-2",
                        {t(UtilityText::LandmarkNavigation)}
                    }
                    Landmark {
                        id: "dioxus-tailwind-landmark-main",
                        role: landmark::Role::Main,
                        messages: landmark::Messages {
                            label: MessageFn::static_str("Main content"),
                        },
                        class: "block rounded-md border-2 border-emerald-400 bg-emerald-50 px-3 py-2",
                        {t(UtilityText::LandmarkMain)}
                    }
                    Landmark {
                        id: "dioxus-tailwind-landmark-complementary",
                        role: landmark::Role::Complementary,
                        messages: landmark::Messages {
                            label: MessageFn::static_str("Related content"),
                        },
                        class: "block rounded-md border-2 border-yellow-400 bg-yellow-50 px-3 py-2",
                        {t(UtilityText::LandmarkComplementary)}
                    }
                    Landmark {
                        id: "dioxus-tailwind-landmark-contentinfo",
                        role: landmark::Role::ContentInfo,
                        messages: landmark::Messages {
                            label: MessageFn::static_str("Page footer"),
                        },
                        class: "block rounded-md border-2 border-slate-400 bg-slate-50 px-3 py-2",
                        {t(UtilityText::LandmarkContentInfo)}
                    }
                    Landmark {
                        id: "dioxus-tailwind-landmark-form",
                        role: landmark::Role::Form,
                        messages: landmark::Messages {
                            label: MessageFn::static_str("Subscribe form"),
                        },
                        class: "block rounded-md border-2 border-teal-400 bg-teal-50 px-3 py-2",
                        {t(UtilityText::LandmarkForm)}
                    }
                    Landmark {
                        id: "dioxus-tailwind-landmark-region",
                        role: landmark::Role::Region,
                        messages: landmark::Messages {
                            label: MessageFn::static_str("Sidebar region"),
                        },
                        class: "block rounded-md border-2 border-rose-400 bg-rose-50 px-3 py-2",
                        {t(UtilityText::LandmarkRegion)}
                    }
                    Landmark {
                        id: "dioxus-tailwind-landmark-search",
                        role: landmark::Role::Search,
                        messages: landmark::Messages {
                            label: MessageFn::static_str("Site search"),
                        },
                        class: "block rounded-md border-2 border-orange-400 bg-orange-50 px-3 py-2",
                        {t(UtilityText::LandmarkSearch)}
                    }
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
                "aria-labelledby": "highlight-primitive",
                div { class: "mb-4",
                    h2 {
                        id: "highlight-primitive",
                        class: "text-base font-bold text-slate-950",
                        {t(UtilityText::HighlightPrimitive)}
                    }
                    p { class: "text-sm text-slate-500", {t(UtilityText::HighlightDescription)} }
                }
                p { class: "text-slate-900",
                    Highlight {
                        query: vec!["highlighted".to_string()],
                        text: "Hello highlighted world!",
                    }
                }
            }
        }
    }
}
