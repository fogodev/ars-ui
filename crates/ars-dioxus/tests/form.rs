//! SSR tests for the Dioxus Form adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_components::utility::form;
use ars_dioxus::utility::{
    field::{ErrorMessage, Field, Input, Label},
    form::Form,
};
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
            Form {
                id: "account-form",
                action: "/account",
                role: "search",
                validation_behavior: form::ValidationBehavior::Aria,
                validation_errors: [
                    ("email".to_string(), vec![Error::server("Email is already registered.")]),
                ],
                class: "account-form",
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
        r#"name="email""#,
        r#"role="status""#,
        r#"aria-live="polite""#,
        r#"aria-atomic="true""#,
        r#"data-ars-part="status-region""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn form_validation_errors_drive_matching_field_by_name() {
    #[rustfmt::skip]
    fn app() -> Element {
        rsx! {
            Form {
                id: "account-form",
                validation_errors: [
                    ("email".to_string(), vec![Error::server("Correo electronico invalido.")]),
                ],
                Field { id: "name-field", name: "name",
                    Label { "Name" }
                    Input { name: "name" }
                    ErrorMessage { "Name error" }
                }
                Field { id: "email-field", name: "email",
                    Label { "Email" }
                    Input { name: "email" }
                    ErrorMessage { "Correo electronico invalido." }
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
        r#"aria-errormessage="email-field-error-message""#,
        r#"id="email-field-error-message""#,
        r#"role="alert""#,
        "Correo electronico invalido.",
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}
