//! SSR tests for the Dioxus Form adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_dioxus::utility::{field, form};
use ars_forms::validation::Error;
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

#[test]
fn form_renders_root_and_status_region() {
    #[rustfmt::skip]
    fn app() -> Element {
        rsx! {
            form::Root {
                id: "account-form",
                action: "/account",
                role: "search",
                validation_behavior: form::ValidationBehavior::Aria,
                validation_errors: [
                    ("email".to_string(), vec![Error::server("Email is already registered.")]),
                ],
                status_message: "Ready",
                class: "account-form",
                style: "display: grid;",
                input { name: "email" }
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"<form"#,
        r#"id="account-form""#,
        r#"action="/account""#,
        r#"role="search""#,
        r#"novalidate"#,
        r#"data-ars-scope="form""#,
        r#"data-ars-part="root""#,
        r#"data-ars-state="idle""#,
        r#"class="account-form""#,
        r#"style="display: grid;""#,
        r#"name="email""#,
        r#"role="status""#,
        r#"aria-live="polite""#,
        r#"aria-atomic="true""#,
        r#"data-ars-part="status-region""#,
        "Ready",
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert_eq!(
        html.matches(r#"data-ars-part="status-region""#).count(),
        1,
        "Form must render exactly one adapter-owned status region: {html}"
    );
}

#[test]
fn form_status_region_part_accepts_consumer_class_and_style() {
    #[rustfmt::skip]
    fn app() -> Element {
        rsx! {
            form::Root {
                id: "account-form",
                status_message: "Ready",
                input { name: "email" }
                form::StatusRegion {
                    class: "form-status",
                    style: "margin-block-start: 0.5rem;",
                }
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"role="status""#,
        r#"aria-live="polite""#,
        r#"aria-atomic="true""#,
        r#"data-ars-part="status-region""#,
        r#"class="form-status""#,
        r#"style="margin-block-start: 0.5rem;""#,
        "Ready",
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert_eq!(
        html.matches(r#"data-ars-part="status-region""#).count(),
        1,
        "explicit StatusRegion must replace the fallback status region: {html}"
    );
}

#[test]
fn form_status_region_detector_ignores_unrelated_status_region_components() {
    #[component]
    fn StatusRegion() -> Element {
        rsx! {
            div { class: "other-status-region", "Other status" }
        }
    }

    #[rustfmt::skip]
    fn app() -> Element {
        rsx! {
            form::Root {
                id: "account-form",
                status_message: "Ready",
                input { name: "email" }
                StatusRegion {}
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"class="other-status-region""#,
        "Other status",
        r#"role="status""#,
        r#"data-ars-part="status-region""#,
        "Ready",
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert_eq!(
        html.matches(r#"data-ars-part="status-region""#).count(),
        1,
        "unrelated StatusRegion component must not suppress the form fallback: {html}"
    );
}

#[test]
fn form_status_region_wrapper_can_disable_fallback_region() {
    #[component]
    fn WrappedStatusRegion() -> Element {
        rsx! {
            form::StatusRegion { class: "wrapped-status" }
        }
    }

    #[rustfmt::skip]
    fn app() -> Element {
        rsx! {
            form::Root {
                id: "account-form",
                status_message: "Ready",
                has_status_region: true,
                input { name: "email" }
                WrappedStatusRegion {}
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"role="status""#,
        r#"data-ars-part="status-region""#,
        r#"class="wrapped-status""#,
        "Ready",
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert_eq!(
        html.matches(r#"data-ars-part="status-region""#).count(),
        1,
        "wrapped StatusRegion must replace the fallback status region when declared: {html}"
    );
}

#[test]
fn form_validation_errors_drive_matching_field_by_name() {
    #[rustfmt::skip]
    fn app() -> Element {
        rsx! {
            form::Root {
                id: "account-form",
                validation_errors: [
                    ("email".to_string(), vec![Error::server("Correo electronico invalido.")]),
                ],
                field::Root { id: "name-field", name: "name",
                    field::Label { "Name" }
                    field::Input { name: "name" }
                    field::ErrorMessage { "Name error" }
                }
                field::Root { id: "email-field", name: "email",
                    field::Label { "Email" }
                    field::Input { name: "email" }
                    field::ErrorMessage { "Correo electronico invalido." }
                }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"id="name-field""#),
        "missing name field: {html}"
    );
    assert!(
        !html.contains(
            r#"id="name-field" data-ars-scope="field" data-ars-part="root" data-ars-invalid"#
        ),
        "unmatched field should not inherit form validation errors: {html}"
    );

    for fragment in [
        r#"id="email-field""#,
        r#"data-ars-invalid"#,
        r#"id="email-field-input""#,
        r#"aria-invalid="true""#,
        r#"aria-describedby="email-field-error-message""#,
        r#"aria-errormessage="email-field-error-message""#,
        r#"id="email-field-error-message""#,
        r#"role="alert""#,
        "Correo electronico invalido.",
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert!(
        !html.contains("email-field-description"),
        "field input must not reference a missing description element: {html}"
    );
}
