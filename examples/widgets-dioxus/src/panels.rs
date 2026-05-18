use ars_dioxus::{
    navigation::tabs::{Tab, Tabs},
    prelude::{Orientation, t},
    utility::{
        button::{self, Button, ButtonAsChild},
        dismissable,
        error_boundary::{Boundary, CapturedError},
        separator::{Separator, SeparatorAsChild},
        visually_hidden::{VisuallyHidden, VisuallyHiddenAsChild},
    },
};
use dioxus::prelude::*;

use crate::text::{NavigationTab, WidgetsText};

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

#[component]
fn ExampleErrorChild() -> Element {
    Err(CapturedError::from_display(t(WidgetsText::ExampleChildError)).into())
}

#[component]
pub(crate) fn EmptyCategoryPanel(text: WidgetsText) -> Element {
    rsx! {
        section { class: "empty-category",
            p { {t(text)} }
        }
    }
}

#[component]
pub(crate) fn NavigationPanel() -> Element {
    rsx! {
        section {
            h3 { {t(WidgetsText::TabsHeading)} }
            p { {t(WidgetsText::TabsDemoSummary)} }
            Tabs {
                default_value: NavigationTab::Overview,
                tabs: [
                    Tab::new(NavigationTab::Overview, rsx! {
                        p { {t(WidgetsText::TabsOverview)} }
                    }),
                    Tab::new(NavigationTab::Keyboard, rsx! {
                        ul {
                            li { {t(WidgetsText::KeyboardArrowKeys)} }
                            li { {t(WidgetsText::KeyboardHomeEnd)} }
                            li { {t(WidgetsText::KeyboardManualActivation)} }
                            li { {t(WidgetsText::KeyboardReorder)} }
                            li { {t(WidgetsText::KeyboardClosable)} }
                        }
                    }).closable(true),
                    Tab::new(NavigationTab::Closable, rsx! {
                        p { {t(WidgetsText::ClosablePanel)} }
                    }).closable(true),
                    Tab::new(NavigationTab::Disabled, rsx! {
                        p { {t(WidgetsText::DisabledPanel)} }
                    }).disabled(true),
                ],
                reorderable: true,
            }
        }
    }
}

#[component]
pub(crate) fn UtilityPanel() -> Element {
    let dismiss_status = use_signal_sync(|| WidgetsText::DismissInitial);
    let dismiss_status_for_dismiss = dismiss_status;
    let dismiss_props = dismissable::Props::new().on_dismiss(move |reason| {
        let mut dismiss_status = dismiss_status_for_dismiss;

        dismiss_status.set(WidgetsText::DismissReason {
            reason: format!("{reason:?}"),
        });
    });

    rsx! {
        style { "{SEPARATOR_STYLE}" }
        div { class: "utility-grid",
            section { "aria-labelledby": "variants",
                h3 { id: "variants", {t(WidgetsText::ButtonVariants)} }
                div { class: "button-row",
                    Button { id: "dioxus-default", {t(WidgetsText::DefaultButton)} }
                    Button {
                        id: "dioxus-primary",
                        variant: button::Variant::Primary,
                        {t(WidgetsText::PrimaryButton)}
                    }
                    Button {
                        id: "dioxus-secondary",
                        variant: button::Variant::Secondary,
                        {t(WidgetsText::SecondaryButton)}
                    }
                    Button {
                        id: "dioxus-destructive",
                        variant: button::Variant::Destructive,
                        {t(WidgetsText::DestructiveButton)}
                    }
                    Button {
                        id: "dioxus-outline",
                        variant: button::Variant::Outline,
                        {t(WidgetsText::OutlineButton)}
                    }
                    Button { id: "dioxus-ghost", variant: button::Variant::Ghost,
                        {t(WidgetsText::GhostButton)}
                    }
                    Button { id: "dioxus-link", variant: button::Variant::Link,
                        {t(WidgetsText::LinkButton)}
                    }
                }
            }
            section { "aria-labelledby": "sizes",
                h3 { id: "sizes", {t(WidgetsText::ButtonSizes)} }
                div { class: "button-row",
                    Button { id: "dioxus-sm", size: button::Size::Sm, {t(WidgetsText::SmallButton)} }
                    Button { id: "dioxus-md", size: button::Size::Md, {t(WidgetsText::MediumButton)} }
                    Button { id: "dioxus-lg", size: button::Size::Lg, {t(WidgetsText::LargeButton)} }
                    Button { id: "dioxus-icon", size: button::Size::Icon, {t(WidgetsText::IconButton)} }
                }
            }
            section { "aria-labelledby": "states",
                h3 { id: "states", {t(WidgetsText::ButtonStates)} }
                div { class: "button-row",
                    Button { id: "dioxus-disabled", disabled: true, {t(WidgetsText::DisabledButton)} }
                    Button { id: "dioxus-loading", loading: true, {t(WidgetsText::LoadingButton)} }
                }
            }
            section { "aria-labelledby": "as-child",
                h3 { id: "as-child", {t(WidgetsText::AsChild)} }
                div { class: "button-row",
                    ButtonAsChild {
                        id: "dioxus-as-child-docs",
                        variant: button::Variant::Link,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(WidgetsText::DocsLinkRoot)} }
                        },
                    }
                    ButtonAsChild {
                        id: "dioxus-as-child-primary",
                        variant: button::Variant::Primary,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(WidgetsText::AnchorAsPrimary)} }
                        },
                    }
                }
            }
            section { "aria-labelledby": "forms",
                h3 { id: "forms", {t(WidgetsText::Forms)} }
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
                            {t(WidgetsText::SubmitOverride)}
                        }
                        Button { id: "dioxus-reset", r#type: button::Type::Reset,
                            {t(WidgetsText::Reset)}
                        }
                    }
                }
            }
            section { "aria-labelledby": "visually-hidden",
                h3 { id: "visually-hidden", {t(WidgetsText::VisuallyHidden)} }
                p {
                    VisuallyHidden { id: "dioxus-visually-hidden-label", {t(WidgetsText::VisuallyHiddenLabel)} }
                    {t(WidgetsText::VisuallyHiddenDescription)}
                }
                p {
                    VisuallyHidden { id: "dioxus-focusable-skip", is_focusable: true,
                        a { href: "#variants", {t(WidgetsText::FocusableSkipLink)} }
                    }
                }
                VisuallyHiddenAsChild {
                    id: "dioxus-visually-hidden-as-child",
                    render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                        span { ..slot.attrs,{t(WidgetsText::AsChildHiddenLabel)} }
                    },
                }
            }
            section { "aria-labelledby": "separator",
                h3 { id: "separator", {t(WidgetsText::SeparatorPrimitive)} }
                p { {t(WidgetsText::SeparatorDescription)} }
                Separator { id: "dioxus-separator-horizontal" }
                div { style: "display: flex; align-items: stretch; gap: 12px; min-height: 48px;",
                    span { {t(WidgetsText::HorizontalSeparator)} }
                    Separator {
                        id: "dioxus-separator-vertical",
                        orientation: Orientation::Vertical,
                    }
                    span { {t(WidgetsText::VerticalSeparator)} }
                }
                Separator { id: "dioxus-separator-decorative", decorative: true }
                p { {t(WidgetsText::DecorativeSeparator)} }
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
                p { {t(WidgetsText::AsChildSeparator)} }
            }
            section { "aria-labelledby": "dismissable",
                h3 { id: "dismissable", {t(WidgetsText::DismissablePrimitive)} }
                dismissable::Region { props: dismiss_props,
                    div {
                        h4 { {t(WidgetsText::PlainDismissableRegion)} }
                        p { {t(WidgetsText::DismissableDescription)} }
                    }
                }
                p { {t(dismiss_status())} }
            }
            section { "aria-labelledby": "errors",
                h3 { id: "errors", {t(WidgetsText::ErrorBoundary)} }
                div { class: "button-row",
                    Boundary {
                        p { {t(WidgetsText::HealthyChild)} }
                    }
                    Boundary { ExampleErrorChild {} }
                }
            }
        }
    }
}
