//! SSR tests for the Leptos Form adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use std::collections::BTreeMap;

use ars_components::utility::form;
use ars_forms::validation::Error;
use ars_leptos::utility::{
    field::{ErrorMessage, Field, Input, Label},
    form::Form,
};
use leptos::{prelude::*, reactive::owner::Owner};

fn render(view_fn: impl FnOnce() -> String + 'static) -> String {
    let owner = Owner::new();
    let result = owner.with(view_fn);

    drop(owner);

    result
}

#[test]
fn form_renders_root_and_status_region() {
    let html = render(|| {
        view! {
            <Form
                id="account-form"
                action="/account"
                role="search"
                validation_behavior=form::ValidationBehavior::Aria
                validation_errors=BTreeMap::from([
                    ("email".to_string(), vec![Error::server("Email is already registered.")]),
                ])
                class="account-form"
            >
                <input name="email" />
            </Form>
        }
        .to_html()
    });

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
    let html = render(|| {
        view! {
            <Form
                id="account-form"
                validation_errors=BTreeMap::from([
                    ("email".to_string(), vec![Error::server("Correo electronico invalido.")]),
                ])
            >

                <Field id="name-field" name="name">
                    <Label>"Name"</Label>
                    <Input name="name" />
                    <ErrorMessage>"Name error"</ErrorMessage>
                </Field>
                <Field id="email-field" name="email">
                    <Label>"Email"</Label>
                    <Input name="email" />
                    <ErrorMessage>"Correo electronico invalido."</ErrorMessage>
                </Field>
            </Form>
        }
        .to_html()
    });

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
