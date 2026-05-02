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
            p { class: "page-kicker", "CSS styling" }
            h1 { "Dioxus Button Widgets" }
            p { class: "page-summary",
                "A compact gallery for variant, size, loading, disabled, and form behaviors."
            }
            div { class: "gallery-grid",
                section {
                    class: "showcase-panel wide",
                    "aria-labelledby": "variants",
                    div { class: "panel-heading",
                        h2 { id: "variants", "Variants" }
                        p { class: "panel-note", "Hover each button to inspect transitions." }
                    }
                    div { class: "button-row",
                        Button { id: "dioxus-css-default", "Default" }
                        Button {
                            id: "dioxus-css-primary",
                            variant: button::Variant::Primary,
                            "Primary"
                        }
                        Button {
                            id: "dioxus-css-secondary",
                            variant: button::Variant::Secondary,
                            "Secondary"
                        }
                        Button {
                            id: "dioxus-css-destructive",
                            variant: button::Variant::Destructive,
                            "Destructive"
                        }
                        Button {
                            id: "dioxus-css-outline",
                            variant: button::Variant::Outline,
                            "Outline"
                        }
                        Button {
                            id: "dioxus-css-ghost",
                            variant: button::Variant::Ghost,
                            "Ghost"
                        }
                        Button {
                            id: "dioxus-css-link",
                            variant: button::Variant::Link,
                            "Link"
                        }
                    }
                }
                section { class: "showcase-panel", "aria-labelledby": "sizes",
                    div { class: "panel-heading",
                        h2 { id: "sizes", "Sizes" }
                        p { class: "panel-note", "Enum-driven sizing." }
                    }
                    div { class: "button-row",
                        Button { id: "dioxus-css-sm", size: button::Size::Sm, "Small" }
                        Button { id: "dioxus-css-md", size: button::Size::Md, "Medium" }
                        Button { id: "dioxus-css-lg", size: button::Size::Lg, "Large" }
                        Button { id: "dioxus-css-icon", size: button::Size::Icon, "R" }
                    }
                }
                section { class: "showcase-panel", "aria-labelledby": "states",
                    div { class: "panel-heading",
                        h2 { id: "states", "States" }
                        p { class: "panel-note", "Disabled and busy controls." }
                    }
                    div { class: "button-row",
                        Button { id: "dioxus-css-disabled", disabled: true, "Disabled" }
                        Button { id: "dioxus-css-loading", loading: true, "Loading" }
                    }
                }
                section { class: "showcase-panel", "aria-labelledby": "loading",
                    div { class: "panel-heading",
                        h2 { id: "loading", "Loading indicator" }
                        p { class: "panel-note", "Spinner part styling." }
                    }
                    div { class: "button-row",
                        Button {
                            id: "dioxus-css-loading-primary",
                            variant: button::Variant::Primary,
                            loading: true,
                            "Saving"
                        }
                        Button {
                            id: "dioxus-css-loading-destructive",
                            variant: button::Variant::Destructive,
                            loading: true,
                            "Deleting"
                        }
                    }
                }
                section { class: "showcase-panel", "aria-labelledby": "as-child",
                    div { class: "panel-heading",
                        h2 { id: "as-child", "As child" }
                        p { class: "panel-note", "Button attrs on consumer-owned anchors." }
                    }
                    div { class: "button-row",
                        ButtonAsChild {
                            id: "dioxus-css-as-child-docs",
                            variant: button::Variant::Link,
                            render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                                a { href: "#forms", ..slot.attrs, "Docs link root" }
                            },
                        }
                        ButtonAsChild {
                            id: "dioxus-css-as-child-primary",
                            variant: button::Variant::Primary,
                            render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                                a { href: "#variants", ..slot.attrs, "Anchor as primary" }
                            },
                        }
                    }
                }
                section { class: "showcase-panel", "aria-labelledby": "forms",
                    div { class: "panel-heading",
                        h2 { id: "forms", "Forms" }
                        p { class: "panel-note", "Submit/reset and form overrides." }
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
                                "Submit override"
                            }
                            Button {
                                id: "dioxus-css-reset",
                                r#type: button::Type::Reset,
                                "Reset"
                            }
                        }
                    }
                }
                section {
                    class: "showcase-panel wide",
                    "aria-labelledby": "dismissable",
                    div { class: "panel-heading",
                        h2 { id: "dismissable", "Dismissable primitive" }
                        p { class: "panel-note",
                            "Outside pointer/focus, Escape, and hidden dismiss buttons share one primitive."
                        }
                    }
                    dismissable::Region {
                        props: dismiss_props,
                        dismiss_label: "Dismiss example region",
                        div { class: "dismissable-card",
                            h3 { "CSS dismissable region" }
                            p {
                                "This standalone primitive is the behavior layer future overlays will compose."
                            }
                        }
                    }
                    p { class: "dismissable-status", "{dismiss_status}" }
                }
                section { class: "showcase-panel wide", "aria-labelledby": "errors",
                    div { class: "panel-heading",
                        h2 { id: "errors", "Error boundary" }
                        p { class: "panel-note", "Healthy and captured child output." }
                    }
                    div { class: "error-grid",
                        Boundary {
                            p { class: "healthy-boundary",
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
