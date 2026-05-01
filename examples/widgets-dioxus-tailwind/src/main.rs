use ars_dioxus::utility::{
    button::{self, Button, ButtonAsChild},
    dismissable,
    error_boundary::{Boundary, CapturedError},
};
use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn ExampleErrorChild() -> Element {
    Err(CapturedError::from_display("Example child failed while rendering.").into())
}

#[component]
fn App() -> Element {
    let dismiss_status = use_signal_sync(|| {
        String::from("Click outside the region, press Escape, or tab to a hidden dismiss button.")
    });
    let dismiss_status_for_dismiss = dismiss_status;
    let dismiss_props = dismissable::Props::new().on_dismiss(move |reason| {
        let mut dismiss_status = dismiss_status_for_dismiss;

        dismiss_status.set(format!("Last dismiss reason: {reason:?}"));
    });

    rsx! {
        main { class: "mx-auto min-h-screen max-w-6xl px-5 py-10 md:px-8",
            p { class: "mb-2 text-xs font-extrabold uppercase tracking-wider text-blue-700",
                "Tailwind styling"
            }
            h1 { class: "max-w-3xl text-4xl font-extrabold leading-tight text-slate-950",
                "Dioxus Button Widgets"
            }
            p { class: "mt-3 max-w-3xl text-base leading-7 text-slate-600",
                "A compact gallery for variant, size, loading, disabled, and form behaviors."
            }
            div { class: "mt-8 grid gap-5 lg:grid-cols-2",
                section {
                    class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-xl shadow-slate-900/10 lg:col-span-2",
                    "aria-labelledby": "variants",
                    div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                        h2 {
                            id: "variants",
                            class: "text-base font-bold text-slate-950",
                            "Variants"
                        }
                        p { class: "text-sm text-slate-500",
                            "Hover each button to inspect transitions."
                        }
                    }
                    div { class: "flex flex-wrap gap-3",
                        Button { id: "dioxus-tw-default", "Default" }
                        Button {
                            id: "dioxus-tw-primary",
                            variant: button::Variant::Primary,
                            "Primary"
                        }
                        Button {
                            id: "dioxus-tw-secondary",
                            variant: button::Variant::Secondary,
                            "Secondary"
                        }
                        Button {
                            id: "dioxus-tw-destructive",
                            variant: button::Variant::Destructive,
                            "Destructive"
                        }
                        Button {
                            id: "dioxus-tw-outline",
                            variant: button::Variant::Outline,
                            "Outline"
                        }
                        Button {
                            id: "dioxus-tw-ghost",
                            variant: button::Variant::Ghost,
                            "Ghost"
                        }
                        Button {
                            id: "dioxus-tw-link",
                            variant: button::Variant::Link,
                            "Link"
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
                            "Sizes"
                        }
                        p { class: "text-sm text-slate-500", "sm, md, lg, icon" }
                    }
                    div { class: "flex flex-wrap gap-3",
                        Button { id: "dioxus-tw-sm", size: button::Size::Sm, "Small" }
                        Button { id: "dioxus-tw-md", size: button::Size::Md, "Medium" }
                        Button { id: "dioxus-tw-lg", size: button::Size::Lg, "Large" }
                        Button { id: "dioxus-tw-icon", size: button::Size::Icon, "R" }
                    }
                }
                section {
                    class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
                    "aria-labelledby": "states",
                    div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                        h2 {
                            id: "states",
                            class: "text-base font-bold text-slate-950",
                            "States"
                        }
                        p { class: "text-sm text-slate-500", "Disabled and busy controls." }
                    }
                    div { class: "flex flex-wrap gap-3",
                        Button { id: "dioxus-tw-disabled", disabled: true, "Disabled" }
                        Button { id: "dioxus-tw-loading", loading: true, "Loading" }
                    }
                }
                section {
                    class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
                    "aria-labelledby": "loading",
                    div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                        h2 {
                            id: "loading",
                            class: "text-base font-bold text-slate-950",
                            "Loading indicator"
                        }
                        p { class: "text-sm text-slate-500", "Spinner part styling." }
                    }
                    div { class: "flex flex-wrap gap-3",
                        Button {
                            id: "dioxus-tw-loading-primary",
                            variant: button::Variant::Primary,
                            loading: true,
                            "Saving"
                        }
                        Button {
                            id: "dioxus-tw-loading-destructive",
                            variant: button::Variant::Destructive,
                            loading: true,
                            "Deleting"
                        }
                    }
                }
                section {
                    class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10 transition hover:-translate-y-0.5 hover:shadow-xl hover:shadow-slate-900/15",
                    "aria-labelledby": "as-child",
                    div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                        h2 {
                            id: "as-child",
                            class: "text-base font-bold text-slate-950",
                            "As child"
                        }
                        p { class: "text-sm text-slate-500",
                            "Button attrs on consumer-owned anchors."
                        }
                    }
                    div { class: "flex flex-wrap gap-3",
                        ButtonAsChild {
                            id: "dioxus-tw-as-child-docs",
                            variant: button::Variant::Link,
                            class: "group",
                            render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                                a { href: "#forms", ..slot.attrs, "Docs link root" }
                            },
                        }
                        ButtonAsChild {
                            id: "dioxus-tw-as-child-primary",
                            variant: button::Variant::Primary,
                            render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                                a { href: "#variants", ..slot.attrs, "Anchor as primary" }
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
                            "Forms"
                        }
                        p { class: "text-sm text-slate-500", "Submit/reset and form overrides." }
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
                                "Submit override"
                            }
                            Button {
                                id: "dioxus-tw-reset",
                                r#type: button::Type::Reset,
                                "Reset"
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
                            "Dismissable primitive"
                        }
                        p { class: "text-sm text-slate-500",
                            "Outside pointer/focus, Escape, and hidden dismiss buttons share one primitive."
                        }
                    }
                    dismissable::Region {
                        props: dismiss_props,
                        dismiss_label: "Dismiss example region",
                        div {
                            h3 { class: "text-sm font-bold text-blue-950",
                                "Tailwind dismissable region"
                            }
                            p { class: "mt-2 max-w-2xl text-sm leading-6 text-blue-900",
                                "This standalone primitive is the behavior layer future overlays will compose."
                            }
                        }
                    }
                    p { class: "mt-3 rounded-md bg-slate-950 px-3 py-2 text-sm font-medium text-white shadow-sm",
                        "{dismiss_status}"
                    }
                }
                section {
                    class: "rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10 lg:col-span-2",
                    "aria-labelledby": "errors",
                    div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                        h2 {
                            id: "errors",
                            class: "text-base font-bold text-slate-950",
                            "Error boundary"
                        }
                        p { class: "text-sm text-slate-500", "Healthy and captured child output." }
                    }
                    div { class: "grid gap-4 md:grid-cols-2",
                        Boundary {
                            p { class: "rounded-lg border border-emerald-200 bg-emerald-50 p-4 text-sm font-medium text-emerald-900 shadow-sm",
                                "Healthy child rendered inside the boundary."
                            }
                        }
                        Boundary { ExampleErrorChild {} }
                    }
                }
            }
        }
    }
}
