//! SSR tests for styled Dioxus Checkbox components.

#![cfg(not(target_arch = "wasm32"))]

use ars_dioxus_components::input::checkbox::{css, tailwind};
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

#[test]
fn css_checkbox_renders_anatomy_and_styles() {
    assert!(css::STYLES.contains(".ars-checkbox__control"));
    assert!(css::STYLES.contains("[data-ars-scope=\"checkbox\"]"));

    fn app() -> Element {
        rsx! {
            css::Checkbox {
                id: "accept-terms",
                name: "terms",
                class: "consumer",
                invalid: true,
                description: Some(rsx! { "Required for signup" }),
                error_message: Some(rsx! { "Accept before continuing" }),
                "Accept terms"
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"class="consumer ars-checkbox""#,
        r#"class="ars-checkbox__label""#,
        r#"class="ars-checkbox__control""#,
        r#"class="ars-checkbox__indicator""#,
        r#"class="ars-checkbox__description""#,
        r#"class="ars-checkbox__error-message""#,
        r#"data-ars-part="root""#,
        r#"data-ars-part="hidden-input""#,
        r#"aria-errormessage="accept-terms-error-message""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn tailwind_checkbox_renders_anatomy_and_root_customization() {
    fn app() -> Element {
        rsx! {
            tailwind::Checkbox {
                id: "newsletter",
                checked: tailwind::State::Checked,
                name: "newsletter",
                class: "consumer",
                "Newsletter"
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"group my-2"#,
        r#"consumer"#,
        r#"aria-checked="true""#,
        r#" checked"#,
        r#"data-ars-state="checked""#,
        r#"data-ars-part="label""#,
        r#"data-ars-part="control""#,
        r#"data-ars-part="indicator""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}
