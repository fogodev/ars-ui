//! SSR tests for the Leptos Form adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use std::collections::BTreeMap;

use ars_components::utility::form as core_form;
use ars_forms::validation::Error;
use ars_leptos::utility::{field, form};
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
            <form::Root
                id="account-form"
                action="/account"
                role="search"
                validation_behavior=core_form::ValidationBehavior::Aria
                validation_errors=BTreeMap::from([
                    ("email".to_string(), vec![Error::server("Email is already registered.")]),
                ])
                status_message="Ready"
                class="account-form"
                style="display: grid;"
            >
                <input name="email" />
            </form::Root>
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
    let html = render(|| {
        view! {
            <form::Root id="account-form" status_message="Ready">
                <input name="email" />
                <form::StatusRegion class="form-status" style="margin-block-start: 0.5rem;" />
            </form::Root>
        }
        .to_html()
    });

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
    fn StatusRegion(children: Children) -> impl IntoView {
        view! { <div class="other-status-region">{children()}</div> }
    }

    let html = render(|| {
        view! {
            <form::Root id="account-form" status_message="Ready">
                <input name="email" />
                <StatusRegion>"Other status"</StatusRegion>
            </form::Root>
        }
        .to_html()
    });

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
fn form_validation_errors_drive_matching_field_by_name() {
    let html = render(|| {
        view! {
            <form::Root
                id="account-form"
                validation_errors=BTreeMap::from([
                    ("email".to_string(), vec![Error::server("Correo electronico invalido.")]),
                ])
            >

                <field::Root id="name-field" name="name">
                    <field::Label>"Name"</field::Label>
                    <field::Input name="name" />
                    <field::ErrorMessage>"Name error"</field::ErrorMessage>
                </field::Root>
                <field::Root id="email-field" name="email">
                    <field::Label>"Email"</field::Label>
                    <field::Input name="email" />
                    <field::ErrorMessage>"Correo electronico invalido."</field::ErrorMessage>
                </field::Root>
            </form::Root>
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
