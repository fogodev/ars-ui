//! SSR tests for the Dioxus Fieldset adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_dioxus::utility::fieldset::{Content, Description, ErrorMessage, Fieldset, Legend};
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
            Fieldset { id: "billing", disabled: true, class: "billing-group",
                Legend { "Billing" }
                Description { "Fields marked required must be completed." }
                Content {
                    input { name: "postal-code" }
                }
                ErrorMessage { "Billing information is incomplete." }
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"<fieldset"#,
        r#"id="billing""#,
        r#"disabled"#,
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
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}
