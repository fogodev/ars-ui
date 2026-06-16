//! SSR tests for the Dioxus Field adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_dioxus::utility::field;
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
            field::Root {
                id: "email-field",
                required: true,
                disabled: true,
                readonly: true,
                errors: vec![Error::server("Email is required.")],
                class: "account-field",
                field::Label { "Email" }
                field::Description { "Use your work email address." }
                field::Input {
                    r#type: field::InputType::Email,
                    name: "email",
                    placeholder: "name@example.com",
                    class: "account-input",
                }
                field::ErrorMessage { "Email is required." }
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

#[test]
fn field_parts_accept_consumer_class_and_style() {
    fn app() -> Element {
        rsx! {
            field::Root { id: "styled-field", errors: vec![Error::server("Required.")],
                field::Label { class: "label-class", style: "color: blue;", "Email" }
                field::Description { class: "description-class", style: "font-size: 12px;",
                    "Use your work email address."
                }
                field::Input {
                    name: "email",
                    class: "input-class",
                    style: "display: block;",
                }
                field::ErrorMessage { class: "error-class", style: "color: red;", "Required." }
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"class="label-class""#,
        r#"style="color: blue;""#,
        r#"class="input-class""#,
        r#"style="display: block;""#,
        r#"class="description-class""#,
        r#"style="font-size: 12px;""#,
        r#"class="error-class""#,
        r#"style="color: red;""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn field_input_references_description_when_rendered_before_description() {
    fn app() -> Element {
        rsx! {
            field::Root { id: "email-field",
                field::Label { "Email" }
                field::Input { name: "email" }
                field::Description { "Use your work email address." }
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"id="email-field-input""#,
        r#"aria-describedby="email-field-description""#,
        r#"id="email-field-description""#,
        "Use your work email address.",
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}
