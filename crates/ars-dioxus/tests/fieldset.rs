//! SSR tests for the Dioxus Fieldset adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_dioxus::utility::fieldset;
use ars_forms::validation::Error;
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

#[test]
fn fieldset_renders_group_anatomy() {
    fn app() -> Element {
        rsx! {
            fieldset::Root {
                id: "billing",
                disabled: true,
                errors: vec![Error::server("Billing information is incomplete.")],
                class: "billing-group",
                fieldset::Legend { "Billing" }
                fieldset::Description { "Fields marked required must be completed." }
                fieldset::Content {
                    input { name: "postal-code" }
                }
                fieldset::ErrorMessage { "Billing information is incomplete." }
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"<fieldset"#,
        r#"id="billing""#,
        r#"disabled"#,
        r#"aria-describedby="billing-description billing-error-message""#,
        r#"data-ars-scope="fieldset""#,
        r#"data-ars-part="root""#,
        r#"class="billing-group""#,
        r#"<legend"#,
        r#"id="billing-legend""#,
        r#"data-ars-part="legend""#,
        r#"id="billing-description""#,
        r#"data-ars-part="description""#,
        r#"data-ars-part="content""#,
        r#"name="postal-code""#,
        r#"id="billing-error-message""#,
        r#"role="alert""#,
        r#"data-ars-part="error-message""#,
        "Billing information is incomplete.",
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert!(
        !html.contains(r#"id="billing-error-message" hidden"#),
        "fieldset error message must be reachable when errors are present: {html}"
    );
}

#[test]
fn fieldset_parts_accept_consumer_class_and_style() {
    fn app() -> Element {
        rsx! {
            fieldset::Root {
                id: "styled-billing",
                errors: vec![Error::server("Required.")],
                fieldset::Legend { class: "legend-class", style: "color: blue;", "Billing" }
                fieldset::Description { class: "description-class", style: "font-size: 12px;",
                    "Fields marked required must be completed."
                }
                fieldset::Content { class: "content-class", style: "display: grid;",
                    input { name: "postal-code" }
                }
                fieldset::ErrorMessage { class: "error-class", style: "color: red;", "Required." }
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"class="legend-class""#,
        r#"style="color: blue;""#,
        r#"class="description-class""#,
        r#"style="font-size: 12px;""#,
        r#"class="content-class""#,
        r#"style="display: grid;""#,
        r#"class="error-class""#,
        r#"style="color: red;""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}
