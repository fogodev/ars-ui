//! Utility-category fixture module.
//!
//! Owns the Button / VisuallyHidden / Separator / ClientOnly /
//! ZIndexAllocator / Heading / Landmark / Highlight / Dismissable /
//! ErrorBoundary showcase panel, the per-category text enum, and the
//! message-registry entry for `dismissable::Messages`.

use ars_dioxus::{
    I18nRegistries, MessageFn, MessagesRegistry,
    prelude::{Orientation, Translate, ValidationError, t},
    utility::{
        button::{self, Button, ButtonAsChild},
        client_only::ClientOnly,
        dismissable,
        error_boundary::{CapturedError, ErrorBoundary},
        field::{self, Field},
        fieldset::{self, Fieldset},
        form::{self, Form},
        heading::{self, Heading, HeadingLevelProvider},
        highlight::Highlight,
        landmark::{self, Landmark},
        separator::{Separator, SeparatorAsChild},
        visually_hidden::{VisuallyHidden, VisuallyHiddenAsChild},
        z_index_allocator::{Context as ZIndexContext, ZIndexAllocatorProvider},
    },
};
use dioxus::prelude::*;

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

    #[translate(
        en_US = "Field and form primitives",
        pt_BR = "Primitivos de campo e formulário"
    )]
    FieldFormHeading,

    #[translate(en_US = "Account details", pt_BR = "Detalhes da conta")]
    AccountDetails,

    #[translate(
        en_US = "Grouped controls inherit the fieldset contract.",
        pt_BR = "Controles agrupados herdam o contrato do fieldset."
    )]
    FieldsetDescription,

    #[translate(en_US = "Email", pt_BR = "E-mail")]
    EmailLabel,

    #[translate(
        en_US = "Use a reachable address.",
        pt_BR = "Use um endereço acessível."
    )]
    EmailDescription,

    #[translate(en_US = "Email is required.", pt_BR = "E-mail é obrigatório.")]
    EmailRequired,

    #[translate(
        en_US = "Include an @ in the email address.",
        pt_BR = "Inclua um @ no endereço de e-mail."
    )]
    EmailMissingAt,

    #[translate(
        en_US = "Enter a domain after @.",
        pt_BR = "Informe um domínio depois de @."
    )]
    EmailIncompleteDomain,

    #[translate(
        en_US = "Account details are incomplete.",
        pt_BR = "Os detalhes da conta estão incompletos."
    )]
    AccountIncomplete,

    #[translate(en_US = "Ready", pt_BR = "Pronto")]
    Ready,
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

#[component]
fn FixtureErrorChild() -> Element {
    Err(CapturedError::from_display("Dioxus fixture child failed").into())
}

#[component]
fn ZIndexProbe(id: &'static str) -> Element {
    let context = try_use_context::<ZIndexContext>().expect("z-index context should be provided");

    let claim = context.allocate_claim();

    rsx! {
        span { id, "data-z": "{claim.value()}", "Allocated" }
    }
}

/// Utility-category showcase panel.
#[component]
pub(crate) fn UtilityPanel(locale_key: String) -> Element {
    let _ = locale_key;
    let dismiss_status = use_signal_sync(|| UtilityText::DismissInitial);
    let dismiss_status_for_dismiss = dismiss_status;

    let dismiss_props = dismissable::Props::new().on_dismiss(move |reason| {
        let mut dismiss_status = dismiss_status_for_dismiss;

        dismiss_status.set(UtilityText::DismissReason {
            reason: format!("{reason:?}"),
        });
    });
    let required_error = t(UtilityText::EmailRequired);
    let missing_at_error = t(UtilityText::EmailMissingAt);
    let incomplete_domain_error = t(UtilityText::EmailIncompleteDomain);

    rsx! {
        div { class: "gallery-grid",
            section { class: "showcase-panel wide", "aria-labelledby": "variants",
                h2 { id: "variants", {t(UtilityText::ButtonVariants)} }
                div { class: "button-row",
                    Button { id: "dioxus-fixture-default", {t(UtilityText::DefaultButton)} }
                    Button {
                        id: "dioxus-fixture-primary",
                        variant: button::Variant::Primary,
                        {t(UtilityText::PrimaryButton)}
                    }
                    Button {
                        id: "dioxus-fixture-secondary",
                        variant: button::Variant::Secondary,
                        {t(UtilityText::SecondaryButton)}
                    }
                    Button {
                        id: "dioxus-fixture-destructive",
                        variant: button::Variant::Destructive,
                        {t(UtilityText::DestructiveButton)}
                    }
                    Button {
                        id: "dioxus-fixture-outline",
                        variant: button::Variant::Outline,
                        {t(UtilityText::OutlineButton)}
                    }
                    Button {
                        id: "dioxus-fixture-ghost",
                        variant: button::Variant::Ghost,
                        {t(UtilityText::GhostButton)}
                    }
                    Button {
                        id: "dioxus-fixture-link",
                        variant: button::Variant::Link,
                        {t(UtilityText::LinkButton)}
                    }
                    Button { id: "dioxus-fixture-sm", size: button::Size::Sm,
                        {t(UtilityText::SmallButton)}
                    }
                    Button { id: "dioxus-fixture-md", size: button::Size::Md,
                        {t(UtilityText::MediumButton)}
                    }
                    Button { id: "dioxus-fixture-lg", size: button::Size::Lg,
                        {t(UtilityText::LargeButton)}
                    }
                    Button { id: "dioxus-fixture-icon", size: button::Size::Icon,
                        {t(UtilityText::IconButton)}
                    }
                    Button { id: "dioxus-fixture-disabled", disabled: true,
                        {t(UtilityText::DisabledButton)}
                    }
                    Button { id: "dioxus-fixture-loading", loading: true,
                        {t(UtilityText::LoadingButton)}
                    }
                    ButtonAsChild {
                        id: "dioxus-fixture-as-child-docs",
                        variant: button::Variant::Link,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(UtilityText::DocsLinkRoot)} }
                        },
                    }
                    ButtonAsChild {
                        id: "dioxus-fixture-as-child-primary",
                        variant: button::Variant::Primary,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(UtilityText::AnchorAsPrimary)} }
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
                        {t(UtilityText::SubmitOverride)}
                    }
                    Button {
                        id: "dioxus-fixture-reset",
                        r#type: button::Type::Reset,
                        {t(UtilityText::Reset)}
                    }
                }
            }
            section {
                class: "showcase-panel wide",
                "aria-labelledby": "utility-primitives",
                h2 { id: "utility-primitives", "Utility primitives" }
                p {
                    VisuallyHidden { id: "dioxus-fixture-visually-hidden-label",
                        {t(UtilityText::VisuallyHiddenLabel)}
                    }
                    "Visible copy with a hidden accessible companion."
                }
                p {
                    VisuallyHidden {
                        id: "dioxus-fixture-focusable-skip",
                        is_focusable: true,
                        a { href: "#variants", {t(UtilityText::FocusableSkipLink)} }
                    }
                }
                VisuallyHiddenAsChild {
                    id: "dioxus-fixture-visually-hidden-as-child",
                    render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                        span { ..slot.attrs,{t(UtilityText::AsChildHiddenLabel)} }
                    },
                }
                Separator { id: "dioxus-fixture-separator-horizontal" }
                div { class: "separator-demo-row",
                    span { "Before" }
                    Separator {
                        id: "dioxus-fixture-separator-vertical",
                        orientation: Orientation::Vertical,
                    }
                    span { "After" }
                }
                SeparatorAsChild {
                    id: "dioxus-fixture-separator-as-child",
                    orientation: Orientation::Vertical,
                    render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                        div { class: "separator-as-child", ..slot.attrs }
                    },
                }
                Separator {
                    id: "dioxus-fixture-separator-decorative",
                    decorative: true,
                }
                div { id: "dioxus-fixture-client-only-host",
                    ClientOnly {
                        fallback: rsx! {
                            span { id: "dioxus-fixture-client-only-fallback", "Loading client content" }
                        },
                        span { id: "dioxus-fixture-client-only-child", "Client content" }
                    }
                }
                section { id: "dioxus-fixture-z-index-host",
                    ZIndexAllocatorProvider {
                        ZIndexProbe { id: "dioxus-fixture-z-index-first" }
                        ZIndexProbe { id: "dioxus-fixture-z-index-second" }
                    }
                }
            }
            section {
                class: "showcase-panel wide",
                "aria-labelledby": "heading-primitive",
                h2 { id: "heading-primitive", "Heading primitive" }
                // The fixture's surrounding section is rendered at h2, so we
                // start the provider at Level::Three and let Section bump to
                // Level::Four. This keeps the page-wide heading hierarchy
                // monotonic (h1 → h2 → h3 → h4) so axe's `heading-order` rule
                // is satisfied. The harness inspects these four headings below
                // to verify explicit-level, provider-inherited, and
                // section-bumped behavior end-to-end in the browser.
                Heading {
                    id: "dioxus-fixture-heading-default",
                    level: heading::Level::Three,
                    {t(UtilityText::HeadingDefault)}
                }
                Heading {
                    id: "dioxus-fixture-heading-level-three",
                    level: heading::Level::Three,
                    {t(UtilityText::HeadingThree)}
                }
                HeadingLevelProvider { level: heading::Level::Three,
                    Heading { id: "dioxus-fixture-heading-provided", {t(UtilityText::HeadingProvider)} }
                    heading::Section {
                        Heading { id: "dioxus-fixture-heading-section", {t(UtilityText::HeadingSection)} }
                    }
                }
            }
            section {
                class: "showcase-panel wide",
                "aria-labelledby": "landmark-primitive",
                h2 { id: "landmark-primitive", "Landmark primitive" }
                Landmark {
                    id: "dioxus-fixture-landmark-banner",
                    role: landmark::Role::Banner,
                    messages: landmark::Messages {
                        label: MessageFn::static_str("Page banner"),
                    },
                    {t(UtilityText::LandmarkBanner)}
                }
                Landmark {
                    id: "dioxus-fixture-landmark-navigation",
                    role: landmark::Role::Navigation,
                    messages: landmark::Messages {
                        label: MessageFn::static_str("Primary navigation"),
                    },
                    {t(UtilityText::LandmarkNavigation)}
                }
                Landmark {
                    id: "dioxus-fixture-landmark-search",
                    role: landmark::Role::Search,
                    messages: landmark::Messages {
                        label: MessageFn::static_str("Site search"),
                    },
                    {t(UtilityText::LandmarkSearch)}
                }
                Landmark {
                    id: "dioxus-fixture-landmark-region",
                    role: landmark::Role::Region,
                    messages: landmark::Messages {
                        label: MessageFn::static_str("Sidebar region"),
                    },
                    {t(UtilityText::LandmarkRegion)}
                }
            }
            section {
                class: "showcase-panel wide",
                "aria-labelledby": "highlight-primitive",
                h2 { id: "highlight-primitive", "Highlight primitive" }
                div { id: "dioxus-fixture-highlight-host",
                    Highlight {
                        query: vec!["highlighted".to_string()],
                        text: "Hello highlighted world!",
                    }
                }
            }
            section { class: "showcase-panel wide", "aria-labelledby": "field-form",
                h2 { id: "field-form", {t(UtilityText::FieldFormHeading)} }
                Form {
                    id: "dioxus-fixture-account-form",
                    action: "/account",
                    validation_errors: std::collections::BTreeMap::from([
                        (
                            "email-required".to_string(),
                            vec![ValidationError::server(required_error.clone())],
                        ),
                    ]),
                    class: "fixture-form",
                    Fieldset {
                        id: "dioxus-fixture-account-fieldset",
                        disabled: true,
                        invalid: true,
                        fieldset::Legend { {t(UtilityText::AccountDetails)} }
                        fieldset::Description { {t(UtilityText::FieldsetDescription)} }
                        fieldset::Content {
                            Field {
                                id: "dioxus-fixture-email-required-field",
                                name: "email-required",
                                required: true,
                                field::Label { {t(UtilityText::EmailLabel)} }
                                field::Description { {t(UtilityText::EmailDescription)} }
                                field::Input { r#type: field::InputType::Email, name: "email-required" }
                                field::ErrorMessage { {t(UtilityText::EmailRequired)} }
                            }
                            Field {
                                id: "dioxus-fixture-email-missing-at-field",
                                name: "email-missing-at",
                                errors: vec![ValidationError::custom("email-missing-at", missing_at_error.clone())],
                                field::Label { {t(UtilityText::EmailLabel)} }
                                field::Description { {t(UtilityText::EmailDescription)} }
                                field::Input {
                                    r#type: field::InputType::Email,
                                    name: "email-missing-at",
                                    value: "admin",
                                }
                                field::ErrorMessage { {t(UtilityText::EmailMissingAt)} }
                            }
                            Field {
                                id: "dioxus-fixture-email-incomplete-domain-field",
                                name: "email-incomplete-domain",
                                errors: vec![
                                    ValidationError::custom(
                                        "email-incomplete-domain",
                                        incomplete_domain_error.clone(),
                                    ),
                                ],
                                field::Label { {t(UtilityText::EmailLabel)} }
                                field::Description { {t(UtilityText::EmailDescription)} }
                                field::Input {
                                    r#type: field::InputType::Email,
                                    name: "email-incomplete-domain",
                                    value: "admin@",
                                }
                                field::ErrorMessage { {t(UtilityText::EmailIncompleteDomain)} }
                            }
                        }
                        fieldset::ErrorMessage { {t(UtilityText::AccountIncomplete)} }
                    }
                    Field {
                        id: "dioxus-fixture-email-valid-field",
                        name: "email-valid",
                        field::Label { {t(UtilityText::EmailLabel)} }
                        field::Description { {t(UtilityText::EmailDescription)} }
                        field::Input {
                            r#type: field::InputType::Email,
                            name: "email-valid",
                            value: "admin@email.com",
                        }
                    }
                }
            }
            section {
                class: "showcase-panel wide",
                "aria-labelledby": "dismissable",
                h2 { id: "dismissable", "Dismissable primitive" }
                dismissable::Region {
                    props: dismiss_props,
                    dismiss_label: t(UtilityText::DismissExampleRegion),
                    div { class: "dismissable-card",
                        h3 { {t(UtilityText::DismissableHeading)} }
                    }
                }
                p { class: "dismissable-status", {t(dismiss_status())} }
            }
            section { class: "showcase-panel wide", "aria-labelledby": "errors",
                h2 { id: "errors", "Error boundary" }
                ErrorBoundary {
                    p { class: "healthy-boundary", "Healthy child rendered" }
                }
                ErrorBoundary { FixtureErrorChild {} }
            }
        }
    }
}
