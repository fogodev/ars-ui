//! SSR tests for the Leptos Field adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_forms::validation::Error;
use ars_leptos::utility::field::{Description, ErrorMessage, Field, Input, InputType, Label};
use leptos::{prelude::*, reactive::owner::Owner};

fn render(view_fn: impl FnOnce() -> String + 'static) -> String {
    let owner = Owner::new();
    let result = owner.with(view_fn);

    drop(owner);

    result
}

#[test]
fn field_renders_root_label_input_and_messages() {
    let html = render(|| {
        view! {
            <Field
                id="email-field"
                required=true
                disabled=true
                readonly=true
                errors=vec![Error::server("Email is required.")]
                class="account-field"
            >
                <Label>"Email"</Label>
                <Input
                    r#type=InputType::Email
                    name="email"
                    placeholder="name@example.com"
                    class="account-input"
                />
                <Description>"Use your work email address."</Description>
                <ErrorMessage>"Email is required."</ErrorMessage>
            </Field>
        }
        .to_html()
    });

    for fragment in [
        r#"<div"#,
        r#"id="email-field""#,
        r#"data-ars-scope="field""#,
        r#"data-ars-part="root""#,
        r#"data-ars-invalid"#,
        r#"class="account-field""#,
        r#"<label"#,
        r#"id="email-field-label""#,
        r#"for="email-field-input""#,
        r#"<input"#,
        r#"id="email-field-input""#,
        r#"data-ars-part="input""#,
        r#"aria-labelledby="email-field-label""#,
        r#"aria-describedby="email-field-description email-field-error-message""#,
        r#"aria-required="true""#,
        r#"required"#,
        r#"aria-invalid="true""#,
        r#"aria-errormessage="email-field-error-message""#,
        r#"disabled"#,
        r#"readonly"#,
        r#"type="email""#,
        r#"name="email""#,
        r#"placeholder="name@example.com""#,
        r#"class="account-input""#,
        r#"id="email-field-description""#,
        r#"data-ars-part="description""#,
        r#"id="email-field-error-message""#,
        r#"role="alert""#,
        r#"data-ars-part="error-message""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}
