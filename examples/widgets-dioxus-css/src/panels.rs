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

#[component]
fn ExampleErrorChild() -> Element {
    Err(CapturedError::from_display(t(WidgetsText::ExampleChildError)).into())
}

#[component]
pub(crate) fn EmptyCategoryPanel(text: WidgetsText) -> Element {
    rsx! {
        section { class: "showcase-panel wide empty-category",
            p { {t(text)} }
        }
    }
}

#[component]
pub(crate) fn NavigationPanel() -> Element {
    rsx! {
        section { class: "showcase-panel wide",
            div { class: "panel-heading",
                h2 { {t(WidgetsText::TabsHeading)} }
                p { class: "panel-note", {t(WidgetsText::TabsDemoSummary)} }
            }
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
        div { class: "gallery-grid",
            section { class: "showcase-panel wide", "aria-labelledby": "variants",
                div { class: "panel-heading",
                    h2 { id: "variants", {t(WidgetsText::ButtonVariants)} }
                    p { class: "panel-note", {t(WidgetsText::ButtonVariantsNote)} }
                }
                div { class: "button-row",
                    Button { id: "dioxus-css-default", {t(WidgetsText::DefaultButton)} }
                    Button {
                        id: "dioxus-css-primary",
                        variant: button::Variant::Primary,
                        {t(WidgetsText::PrimaryButton)}
                    }
                    Button {
                        id: "dioxus-css-secondary",
                        variant: button::Variant::Secondary,
                        {t(WidgetsText::SecondaryButton)}
                    }
                    Button {
                        id: "dioxus-css-destructive",
                        variant: button::Variant::Destructive,
                        {t(WidgetsText::DestructiveButton)}
                    }
                    Button {
                        id: "dioxus-css-outline",
                        variant: button::Variant::Outline,
                        {t(WidgetsText::OutlineButton)}
                    }
                    Button {
                        id: "dioxus-css-ghost",
                        variant: button::Variant::Ghost,
                        {t(WidgetsText::GhostButton)}
                    }
                    Button { id: "dioxus-css-link", variant: button::Variant::Link,
                        {t(WidgetsText::LinkButton)}
                    }
                }
            }
            section { class: "showcase-panel", "aria-labelledby": "sizes",
                div { class: "panel-heading",
                    h2 { id: "sizes", {t(WidgetsText::ButtonSizes)} }
                    p { class: "panel-note", {t(WidgetsText::ButtonSizesNote)} }
                }
                div { class: "button-row",
                    Button { id: "dioxus-css-sm", size: button::Size::Sm,
                        {t(WidgetsText::SmallButton)}
                    }
                    Button { id: "dioxus-css-md", size: button::Size::Md,
                        {t(WidgetsText::MediumButton)}
                    }
                    Button { id: "dioxus-css-lg", size: button::Size::Lg,
                        {t(WidgetsText::LargeButton)}
                    }
                    Button { id: "dioxus-css-icon", size: button::Size::Icon,
                        {t(WidgetsText::IconButton)}
                    }
                }
            }
            section { class: "showcase-panel", "aria-labelledby": "states",
                div { class: "panel-heading",
                    h2 { id: "states", {t(WidgetsText::ButtonStates)} }
                    p { class: "panel-note", {t(WidgetsText::ButtonStatesNote)} }
                }
                div { class: "button-row",
                    Button { id: "dioxus-css-disabled", disabled: true,
                        {t(WidgetsText::DisabledButton)}
                    }
                    Button { id: "dioxus-css-loading", loading: true, {t(WidgetsText::LoadingButton)} }
                }
            }
            section { class: "showcase-panel", "aria-labelledby": "as-child",
                div { class: "panel-heading",
                    h2 { id: "as-child", {t(WidgetsText::AsChild)} }
                    p { class: "panel-note", {t(WidgetsText::AsChildNote)} }
                }
                div { class: "button-row",
                    ButtonAsChild {
                        id: "dioxus-css-as-child-docs",
                        variant: button::Variant::Link,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(WidgetsText::DocsLinkRoot)} }
                        },
                    }
                    ButtonAsChild {
                        id: "dioxus-css-as-child-primary",
                        variant: button::Variant::Primary,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(WidgetsText::AnchorAsPrimary)} }
                        },
                    }
                }
            }
            section { class: "showcase-panel", "aria-labelledby": "forms",
                div { class: "panel-heading",
                    h2 { id: "forms", {t(WidgetsText::Forms)} }
                    p { class: "panel-note", {t(WidgetsText::FormsNote)} }
                }
                form { id: "dioxus-css-example-form",
                    div { class: "button-row",
                        Button {
                            id: "dioxus-css-submit",
                            r#type: button::Type::Submit,
                            form: "dioxus-css-example-form",
                            name: "intent",
                            value: "save",
                            form_action: "/submit",
                            form_method: button::FormMethod::Post,
                            form_enc_type: button::FormEncType::UrlEncoded,
                            form_target: button::FormTarget::Self_,
                            form_no_validate: true,
                            {t(WidgetsText::SubmitOverride)}
                        }
                        Button {
                            id: "dioxus-css-reset",
                            r#type: button::Type::Reset,
                            {t(WidgetsText::Reset)}
                        }
                    }
                }
            }
            section { class: "showcase-panel", "aria-labelledby": "visually-hidden",
                div { class: "panel-heading",
                    h2 { id: "visually-hidden", {t(WidgetsText::VisuallyHidden)} }
                    p { class: "panel-note", {t(WidgetsText::VisuallyHiddenDescription)} }
                }
                p {
                    VisuallyHidden { id: "dioxus-css-visually-hidden-label",
                        {t(WidgetsText::VisuallyHiddenLabel)}
                    }
                    {t(WidgetsText::VisuallyHiddenDescription)}
                }
                p {
                    VisuallyHidden { id: "dioxus-css-focusable-skip", is_focusable: true,
                        a { href: "#variants", {t(WidgetsText::FocusableSkipLink)} }
                    }
                }
                VisuallyHiddenAsChild {
                    id: "dioxus-css-visually-hidden-as-child",
                    render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                        span { ..slot.attrs,{t(WidgetsText::AsChildHiddenLabel)} }
                    },
                }
            }
            section { class: "showcase-panel", "aria-labelledby": "separator",
                div { class: "panel-heading",
                    h2 { id: "separator", {t(WidgetsText::SeparatorPrimitive)} }
                    p { class: "panel-note", {t(WidgetsText::SeparatorDescription)} }
                }
                Separator { id: "dioxus-css-separator-horizontal" }
                div { class: "separator-demo-row",
                    span { {t(WidgetsText::HorizontalSeparator)} }
                    Separator {
                        id: "dioxus-css-separator-vertical",
                        orientation: Orientation::Vertical,
                    }
                    span { {t(WidgetsText::VerticalSeparator)} }
                }
                Separator { id: "dioxus-css-separator-decorative", decorative: true }
                p { class: "panel-note", {t(WidgetsText::DecorativeSeparator)} }
                SeparatorAsChild {
                    id: "dioxus-css-separator-as-child",
                    orientation: Orientation::Vertical,
                    render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                        div { class: "separator-as-child", ..slot.attrs }
                    },
                }
                p { class: "panel-note", {t(WidgetsText::AsChildSeparator)} }
            }
            section {
                class: "showcase-panel wide",
                "aria-labelledby": "dismissable",
                div { class: "panel-heading",
                    h2 { id: "dismissable", {t(WidgetsText::DismissablePrimitive)} }
                    p { class: "panel-note", {t(WidgetsText::DismissableNote)} }
                }
                dismissable::Region { props: dismiss_props,
                    div { class: "dismissable-card",
                        h3 { {t(WidgetsText::CssDismissableRegion)} }
                        p { {t(WidgetsText::DismissableCompositionDescription)} }
                    }
                }
                p { class: "dismissable-status", {t(dismiss_status())} }
            }
            section { class: "showcase-panel wide", "aria-labelledby": "errors",
                div { class: "panel-heading",
                    h2 { id: "errors", {t(WidgetsText::ErrorBoundary)} }
                    p { class: "panel-note", {t(WidgetsText::ErrorBoundaryNote)} }
                }
                div { class: "error-grid",
                    Boundary {
                        p { class: "healthy-boundary", {t(WidgetsText::HealthyChild)} }
                    }
                    Boundary { ExampleErrorChild {} }
                }
            }
        }
    }
}
