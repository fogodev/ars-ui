use ars_dioxus::{
    navigation::tabs::{Tab, Tabs},
    prelude::t,
    utility::{
        button::{self, Button, ButtonAsChild},
        dismissable,
        error_boundary::{Boundary, CapturedError},
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
        section { class: "mt-5 rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
            p { class: "text-sm text-slate-600", {t(text)} }
        }
    }
}

#[component]
pub(crate) fn NavigationPanel() -> Element {
    rsx! {
        section { class: "mt-5 rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
            div { class: "mb-4",
                h2 { class: "text-base font-bold text-slate-950", {t(WidgetsText::TabsHeading)} }
                p { class: "mt-1 text-sm text-slate-500", {t(WidgetsText::TabsDemoSummary)} }
            }
            Tabs {
                default_value: NavigationTab::Overview,
                tabs: [
                    Tab::new(NavigationTab::Overview, rsx! {
                        p { class: "text-sm leading-6 text-slate-600", {t(WidgetsText::TabsOverview)} }
                    }),
                    Tab::new(NavigationTab::Keyboard, rsx! {
                        ul { class: "list-inside list-disc text-sm leading-6 text-slate-600",
                            li { {t(WidgetsText::KeyboardArrowKeys)} }
                            li { {t(WidgetsText::KeyboardHomeEnd)} }
                            li { {t(WidgetsText::KeyboardManualActivation)} }
                            li { {t(WidgetsText::KeyboardReorder)} }
                            li { {t(WidgetsText::KeyboardClosable)} }
                        }
                    }).closable(true),
                    Tab::new(NavigationTab::Closable, rsx! {
                        p { class: "text-sm leading-6 text-slate-600", {t(WidgetsText::ClosablePanel)} }
                    }).closable(true),
                    Tab::new(NavigationTab::Disabled, rsx! {
                        p { class: "text-sm leading-6 text-slate-600", {t(WidgetsText::DisabledPanel)} }
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
        div { class: "mt-5 grid gap-5 lg:grid-cols-2",
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-xl shadow-slate-900/10 lg:col-span-2",
                "aria-labelledby": "variants",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "variants",
                        class: "text-base font-bold text-slate-950",
                        {t(WidgetsText::ButtonVariants)}
                    }
                    p { class: "text-sm text-slate-500", {t(WidgetsText::ButtonVariantsNote)} }
                }
                div { class: "flex flex-wrap gap-3",
                    Button { id: "dioxus-tw-default", {t(WidgetsText::DefaultButton)} }
                    Button {
                        id: "dioxus-tw-primary",
                        variant: button::Variant::Primary,
                        {t(WidgetsText::PrimaryButton)}
                    }
                    Button {
                        id: "dioxus-tw-secondary",
                        variant: button::Variant::Secondary,
                        {t(WidgetsText::SecondaryButton)}
                    }
                    Button {
                        id: "dioxus-tw-destructive",
                        variant: button::Variant::Destructive,
                        {t(WidgetsText::DestructiveButton)}
                    }
                    Button {
                        id: "dioxus-tw-outline",
                        variant: button::Variant::Outline,
                        {t(WidgetsText::OutlineButton)}
                    }
                    Button {
                        id: "dioxus-tw-ghost",
                        variant: button::Variant::Ghost,
                        {t(WidgetsText::GhostButton)}
                    }
                    Button { id: "dioxus-tw-link", variant: button::Variant::Link,
                        {t(WidgetsText::LinkButton)}
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
                        {t(WidgetsText::ButtonSizes)}
                    }
                    p { class: "text-sm text-slate-500", {t(WidgetsText::ButtonSizeTokens)} }
                }
                div { class: "flex flex-wrap gap-3",
                    Button { id: "dioxus-tw-sm", size: button::Size::Sm, {t(WidgetsText::SmallButton)} }
                    Button { id: "dioxus-tw-md", size: button::Size::Md,
                        {t(WidgetsText::MediumButton)}
                    }
                    Button { id: "dioxus-tw-lg", size: button::Size::Lg, {t(WidgetsText::LargeButton)} }
                    Button { id: "dioxus-tw-icon", size: button::Size::Icon,
                        {t(WidgetsText::IconButton)}
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
                        {t(WidgetsText::ButtonStates)}
                    }
                    p { class: "text-sm text-slate-500", {t(WidgetsText::ButtonStatesNote)} }
                }
                div { class: "flex flex-wrap gap-3",
                    Button { id: "dioxus-tw-disabled", disabled: true,
                        {t(WidgetsText::DisabledButton)}
                    }
                    Button { id: "dioxus-tw-loading", loading: true, {t(WidgetsText::LoadingButton)} }
                }
            }
            section {
                class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10 transition hover:-translate-y-0.5 hover:shadow-xl hover:shadow-slate-900/15",
                "aria-labelledby": "as-child",
                div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                    h2 {
                        id: "as-child",
                        class: "text-base font-bold text-slate-950",
                        {t(WidgetsText::AsChild)}
                    }
                    p { class: "text-sm text-slate-500", {t(WidgetsText::AsChildNote)} }
                }
                div { class: "flex flex-wrap gap-3",
                    ButtonAsChild {
                        id: "dioxus-tw-as-child-docs",
                        variant: button::Variant::Link,
                        class: "group",
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(WidgetsText::DocsLinkRoot)} }
                        },
                    }
                    ButtonAsChild {
                        id: "dioxus-tw-as-child-primary",
                        variant: button::Variant::Primary,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, {t(WidgetsText::AnchorAsPrimary)} }
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
                        {t(WidgetsText::Forms)}
                    }
                    p { class: "text-sm text-slate-500", {t(WidgetsText::FormsNote)} }
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
                            {t(WidgetsText::SubmitOverride)}
                        }
                        Button {
                            id: "dioxus-tw-reset",
                            r#type: button::Type::Reset,
                            {t(WidgetsText::Reset)}
                        }
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
                        {t(WidgetsText::DismissablePrimitive)}
                    }
                    p { class: "text-sm text-slate-500", {t(WidgetsText::DismissableNote)} }
                }
                dismissable::Region { props: dismiss_props,
                    div { class: "dismissable-card",
                        h3 { class: "text-sm font-bold text-blue-950",
                            {t(WidgetsText::TailwindDismissableRegion)}
                        }
                        p { class: "mt-2 max-w-2xl text-sm leading-6 text-blue-900",
                            {t(WidgetsText::DismissableCompositionDescription)}
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
                        {t(WidgetsText::ErrorBoundary)}
                    }
                    p { class: "text-sm text-slate-500", {t(WidgetsText::ErrorBoundaryNote)} }
                }
                div { class: "grid gap-4 md:grid-cols-2",
                    Boundary {
                        p { class: "rounded-lg border border-emerald-200 bg-emerald-50 p-4 text-sm font-medium text-emerald-900 shadow-sm",
                            {t(WidgetsText::HealthyChild)}
                        }
                    }
                    Boundary { ExampleErrorChild {} }
                }
            }
        }
    }
}
