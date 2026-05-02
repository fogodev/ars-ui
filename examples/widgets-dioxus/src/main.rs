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
        main { class: "widgets-page",
            h1 { "Dioxus Button Widgets" }
            p { "A compact gallery for variant, size, loading, disabled, and form behaviors." }
            section { "aria-labelledby": "variants",
                h2 { id: "variants", "Variants" }
                p { "Hover each button to inspect transitions." }
                div { class: "button-row",
                    Button { id: "dioxus-default", "Default" }
                    Button {
                        id: "dioxus-primary",
                        variant: button::Variant::Primary,
                        "Primary"
                    }
                    Button {
                        id: "dioxus-secondary",
                        variant: button::Variant::Secondary,
                        "Secondary"
                    }
                    Button {
                        id: "dioxus-destructive",
                        variant: button::Variant::Destructive,
                        "Destructive"
                    }
                    Button {
                        id: "dioxus-outline",
                        variant: button::Variant::Outline,
                        "Outline"
                    }
                    Button { id: "dioxus-ghost", variant: button::Variant::Ghost, "Ghost" }
                    Button { id: "dioxus-link", variant: button::Variant::Link, "Link" }
                }
            }
            section { "aria-labelledby": "sizes",
                h2 { id: "sizes", "Sizes" }
                p { "Enum-driven sizing." }
                div { class: "button-row",
                    Button { id: "dioxus-sm", size: button::Size::Sm, "Small" }
                    Button { id: "dioxus-md", size: button::Size::Md, "Medium" }
                    Button { id: "dioxus-lg", size: button::Size::Lg, "Large" }
                    Button { id: "dioxus-icon", size: button::Size::Icon, "R" }
                }
            }
            section { "aria-labelledby": "states",
                h2 { id: "states", "States" }
                p { "Disabled and busy controls." }
                div { class: "button-row",
                    Button { id: "dioxus-disabled", disabled: true, "Disabled" }
                    Button { id: "dioxus-loading", loading: true, "Loading" }
                }
            }
            section { "aria-labelledby": "loading",
                h2 { id: "loading", "Loading indicator" }
                p { "Spinner part styling." }
                div { class: "button-row",
                    Button {
                        id: "dioxus-loading-primary",
                        variant: button::Variant::Primary,
                        loading: true,
                        "Saving"
                    }
                    Button {
                        id: "dioxus-loading-destructive",
                        variant: button::Variant::Destructive,
                        loading: true,
                        "Deleting"
                    }
                }
            }
            section { "aria-labelledby": "as-child",
                h2 { id: "as-child", "As child" }
                p { "Button attrs on consumer-owned anchors." }
                div { class: "button-row",
                    ButtonAsChild {
                        id: "dioxus-as-child-docs",
                        variant: button::Variant::Link,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#forms", ..slot.attrs, "Docs link root" }
                        },
                    }
                    ButtonAsChild {
                        id: "dioxus-as-child-primary",
                        variant: button::Variant::Primary,
                        render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                            a { href: "#variants", ..slot.attrs, "Anchor as primary" }
                        },
                    }
                }
            }
            section { "aria-labelledby": "forms",
                h2 { id: "forms", "Forms" }
                p { "Submit/reset and form overrides." }
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
                            "Submit override"
                        }
                        Button { id: "dioxus-reset", r#type: button::Type::Reset, "Reset" }
                    }
                }
            }
            section { "aria-labelledby": "dismissable",
                h2 { id: "dismissable", "Dismissable primitive" }
                p { "Outside pointer/focus, Escape, and hidden dismiss buttons share one primitive." }
                dismissable::Region {
                    props: dismiss_props,
                    dismiss_label: "Dismiss example region",
                    div {
                        h3 { "Plain dismissable region" }
                        p {
                            "The primitive owns outside pointer, outside focus, Escape, and paired dismiss-button behavior."
                        }
                    }
                }
                p { "{dismiss_status}" }
            }
            section { "aria-labelledby": "errors",
                h2 { id: "errors", "Error boundary" }
                p { "Healthy and captured child output." }
                div { class: "button-row",
                    Boundary {
                        p { "Healthy child rendered inside the boundary." }
                    }
                    Boundary { ExampleErrorChild {} }
                }
            }
        }
    }
}
