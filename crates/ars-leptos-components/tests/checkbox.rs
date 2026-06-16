//! SSR tests for styled Leptos Checkbox components.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_leptos_components::input::checkbox::{css, tailwind};
use leptos::{children::ViewFn, prelude::*, reactive::owner::Owner};

fn render(view_fn: impl FnOnce() -> String + 'static) -> String {
    let owner = Owner::new();
    let result = owner.with(view_fn);

    drop(owner);

    result
}

#[test]
fn css_checkbox_renders_anatomy_and_styles() {
    assert!(css::STYLES.contains(".ars-checkbox__control"));
    assert!(css::STYLES.contains("[data-ars-scope=\"checkbox\"]"));

    let html = render(|| {
        view! {
            <css::Checkbox
                id="accept-terms"
                name="terms"
                class="consumer"
                description=ViewFn::from(|| view! { "Required for signup" })
                error_message=ViewFn::from(|| view! { "Accept before continuing" })
                invalid=true
            >
                "Accept terms"
            </css::Checkbox>
        }
        .to_html()
    });

    for fragment in [
        r#"class="ars-checkbox consumer""#,
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
    let html = render(|| {
        view! {
            <tailwind::Checkbox
                id="newsletter"
                checked=Signal::derive(|| tailwind::State::Checked)
                name="newsletter"
                class="consumer"
            >
                "Newsletter"
            </tailwind::Checkbox>
        }
        .to_html()
    });

    for fragment in [
        r#"class="group my-2"#,
        r#"consumer"#,
        r#"aria-checked="true""#,
        r#"checked="""#,
        r#"data-ars-state="checked""#,
        r#"data-ars-part="label""#,
        r#"data-ars-part="control""#,
        r#"data-ars-part="indicator""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}
