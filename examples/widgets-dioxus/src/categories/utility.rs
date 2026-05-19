use ars_dioxus::{
    prelude::{Orientation, t, Translate},
    utility::{
        button::{self, Button, ButtonAsChild},
        dismissable,
        error_boundary::{Boundary, CapturedError},
        separator::{Separator, SeparatorAsChild},
        visually_hidden::{VisuallyHidden, VisuallyHiddenAsChild},
    },
};
use dioxus::prelude::*;

const SEPARATOR_STYLE: &str = r#"
[data-ars-scope="separator"][data-ars-part="root"] {
    border: 0;
    background: currentColor;
    color: #cbd5e1;
}

[data-ars-scope="separator"][data-ars-part="root"][data-ars-orientation="horizontal"],
[data-ars-scope="separator"][data-ars-part="root"][role="none"] {
    display: block;
    width: 100%;
    height: 1px;
    margin: 1rem 0;
}

[data-ars-scope="separator"][data-ars-part="root"][data-ars-orientation="vertical"] {
    display: inline-block;
    align-self: stretch;
    width: 1px;
    min-height: 2rem;
    margin: 0 0.25rem;
}
"#;

#[derive(Clone, Debug, Translate, PartialEq)]
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

    #[translate(
        en_US = "Plain dismissable region",
        pt_BR = "Região dismissable simples"
    )]
    PlainDismissableRegion,

    #[translate(
        en_US = "The primitive owns outside pointer, outside focus, Escape, and paired dismiss-button behavior.",
        pt_BR = "O primitivo gerencia ponteiro externo, foco externo, Escape e o comportamento do botão de dispensar pareado."
    )]
    DismissableDescription,

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

#[component]
fn ExampleErrorChild() -> Element {
    Err(CapturedError::from_display(t(UtilityText::ExampleChildError)).into())
}

#[component]
pub(crate) fn UtilityPanel() -> Element {
    let dismiss_status = use_signal_sync(|| UtilityText::DismissInitial);
    let dismiss_status_for_dismiss = dismiss_status;
    let dismiss_props = dismissable::Props::new().on_dismiss(move |reason| {
        let mut dismiss_status = dismiss_status_for_dismiss;

        dismiss_status.set(UtilityText::DismissReason {
            reason: format!("{reason:?}"),
        });
    });

    rsx! {
        style { "{SEPARATOR_STYLE}" }
        div { class: "utility-grid",
            section { "aria-labelledby": "variants",
                h3 { id: "variants", {t(UtilityText::ButtonVariants)} }
                div { class: "button-row",
                    Button { id: "dioxus-default", {t(UtilityText::DefaultButton)} }
                    Button {
                        id: "dioxus-primary",
                        variant: button::Variant::Primary,
                        {t(UtilityText::PrimaryButton)}
                    }
                    Button {
                        id: "dioxus-secondary",
                        variant: button::Variant::Secondary,
                        {t(UtilityText::SecondaryButton)}
                    }
                    Button {
                        id: "dioxus-destructive",
                        variant: button::Variant::Destructive,
                        {t(UtilityText::DestructiveButton)}
                    }
                    Button {
                        id: "dioxus-outline",
                        variant: button::Variant::Outline,
                        {t(UtilityText::OutlineButton)}
                    }
                    Button { id: "dioxus-ghost", variant: button::Variant::Ghost,
                        {t(UtilityText::GhostButton)}
                    }
                    Button { id: "dioxus-link", variant: button::Variant::Link,
                        {t(UtilityText::LinkButton)}
                    }
                }
            }
            section { "aria-labelledby": "sizes",
                h3 { id: "sizes", {t(UtilityText::ButtonSizes)} }
                div { class: "button-row",
                    Button { id: "dioxus-sm", size: button::Size::Sm, {t(UtilityText::SmallButton)} }
                    Button { id: "dioxus-md", size: button::Size::Md, {t(UtilityText::MediumButton)} }
                    Button { id: "dioxus-lg", size: button::Size::Lg, {t(UtilityText::LargeButton)} }
                    Button { id: "dioxus-icon", size: button::Size::Icon, {t(UtilityText::IconButton)} }
                }
            }
            section { "aria-labelledby": "states",
                h3 { id: "states", {t(UtilityText::ButtonStates)} }
                div { class: "button-row",
                    Button { id: "dioxus-disabled", disabled: true, {t(UtilityText::DisabledButton)} }
                    Button { id: "dioxus-loading", loading: true, {t(UtilityText::LoadingButton)} }
                }
            }
            section { "aria-labelledby": "as-child",
                h3 { id: "as-child", {t(UtilityText::AsChild)} }
                div { class: "button-row",
                    ButtonAsChild {
                        id: "dioxus-as-child-docs",
                        variant: button::Variant::Link,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(UtilityText::DocsLinkRoot)} }
                        },
                    }
                    ButtonAsChild {
                        id: "dioxus-as-child-primary",
                        variant: button::Variant::Primary,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(UtilityText::AnchorAsPrimary)} }
                        },
                    }
                }
            }
            section { "aria-labelledby": "forms",
                h3 { id: "forms", {t(UtilityText::Forms)} }
                form { id: "dioxus-example-form",
                    div { class: "button-row",
                        Button {
                            id: "dioxus-submit",
                            r#type: button::Type::Submit,
                            form: "dioxus-example-form",
                            name: "intent",
                            value: "save",
                            form_action: "/submit",
                            form_method: button::FormMethod::Post,
                            form_enc_type: button::FormEncType::UrlEncoded,
                            form_target: button::FormTarget::Self_,
                            form_no_validate: true,
                            {t(UtilityText::SubmitOverride)}
                        }
                        Button { id: "dioxus-reset", r#type: button::Type::Reset,
                            {t(UtilityText::Reset)}
                        }
                    }
                }
            }
            section { "aria-labelledby": "visually-hidden",
                h3 { id: "visually-hidden", {t(UtilityText::VisuallyHidden)} }
                p {
                    VisuallyHidden { id: "dioxus-visually-hidden-label", {t(UtilityText::VisuallyHiddenLabel)} }
                    {t(UtilityText::VisuallyHiddenDescription)}
                }
                p {
                    VisuallyHidden { id: "dioxus-focusable-skip", is_focusable: true,
                        a { href: "#variants", {t(UtilityText::FocusableSkipLink)} }
                    }
                }
                VisuallyHiddenAsChild {
                    id: "dioxus-visually-hidden-as-child",
                    render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                        span { ..slot.attrs,{t(UtilityText::AsChildHiddenLabel)} }
                    },
                }
            }
            section { "aria-labelledby": "separator",
                h3 { id: "separator", {t(UtilityText::SeparatorPrimitive)} }
                p { {t(UtilityText::SeparatorDescription)} }
                Separator { id: "dioxus-separator-horizontal" }
                div { style: "display: flex; align-items: stretch; gap: 12px; min-height: 48px;",
                    span { {t(UtilityText::HorizontalSeparator)} }
                    Separator {
                        id: "dioxus-separator-vertical",
                        orientation: Orientation::Vertical,
                    }
                    span { {t(UtilityText::VerticalSeparator)} }
                }
                Separator { id: "dioxus-separator-decorative", decorative: true }
                p { {t(UtilityText::DecorativeSeparator)} }
                SeparatorAsChild {
                    id: "dioxus-separator-as-child",
                    orientation: Orientation::Vertical,
                    render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                        div {
                            style: "width: 2px; min-height: 32px; background: currentColor;",
                            ..slot.attrs,
                        }
                    },
                }
                p { {t(UtilityText::AsChildSeparator)} }
            }
            section { "aria-labelledby": "dismissable",
                h3 { id: "dismissable", {t(UtilityText::DismissablePrimitive)} }
                dismissable::Region { props: dismiss_props,
                    div {
                        h4 { {t(UtilityText::PlainDismissableRegion)} }
                        p { {t(UtilityText::DismissableDescription)} }
                    }
                }
                p { {t(dismiss_status())} }
            }
            section { "aria-labelledby": "errors",
                h3 { id: "errors", {t(UtilityText::ErrorBoundary)} }
                div { class: "button-row",
                    Boundary {
                        p { {t(UtilityText::HealthyChild)} }
                    }
                    Boundary { ExampleErrorChild {} }
                }
            }
        }
    }
}
