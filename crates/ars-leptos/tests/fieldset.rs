//! SSR tests for the Leptos Fieldset adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_forms::validation::Error;
use ars_leptos::utility::fieldset::{Content, Description, ErrorMessage, Fieldset, Legend};
use leptos::{prelude::*, reactive::owner::Owner};

fn render(view_fn: impl FnOnce() -> String + 'static) -> String {
    let owner = Owner::new();
    let result = owner.with(view_fn);

    drop(owner);

    result
}

#[test]
fn fieldset_renders_group_anatomy() {
    let html = render(|| {
        view! {
            <Fieldset
                id="billing"
                disabled=true
                errors=vec![Error::server("Billing information is incomplete.")]
                class="billing-group"
            >
                <Legend>"Billing"</Legend>
                <Description>"Fields marked required must be completed."</Description>
                <Content>
                    <input name="postal-code" />
                </Content>
                <ErrorMessage>"Billing information is incomplete."</ErrorMessage>
            </Fieldset>
        }
        .to_html()
    });

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
