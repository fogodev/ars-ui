//! SSR tests for the Dioxus Field adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_dioxus::utility::field::{Description, ErrorMessage, Field, Input, InputType, Label};
use ars_forms::validation::Error;
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

#[test]
fn field_renders_root_label_input_and_messages() {
    fn app() -> Element {
        rsx! {
            Field {
                id: "email-field",
                required: true,
                disabled: true,
                readonly: true,
                errors: vec![Error::server("Email is required.")],
                class: "account-field",
                Label { "Email" }
                Description { "Use your work email address." }
                Input {
                    r#type: InputType::Email,
                    name: "email",
                    placeholder: "name@example.com",
                    class: "account-input",
                }
                ErrorMessage { "Email is required." }
            }
        }
    }

    let html = render_app(app);

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
