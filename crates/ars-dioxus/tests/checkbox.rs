//! SSR tests for the Dioxus Checkbox adapter.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeMap;

use ars_dioxus::{
    input::checkbox,
    utility::{fieldset, form},
};
use ars_forms::validation::Error;
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

#[test]
fn checkbox_renders_default_anatomy_and_aria() {
    fn app() -> Element {
        rsx! {
            checkbox::Root { id: "accept-terms", name: "terms",
                checkbox::Label { "Accept terms" }
                checkbox::Control { checkbox::Indicator {} }
                checkbox::HiddenInput {}
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"id="accept-terms""#,
        r#"data-ars-scope="checkbox""#,
        r#"data-ars-part="root""#,
        r#"data-ars-state="unchecked""#,
        r#"data-ars-part="label""#,
        r#"data-ars-part="control""#,
        r#"role="checkbox""#,
        r#"aria-checked="false""#,
        r#"tabindex="0""#,
        r#"data-ars-part="indicator""#,
        r#"aria-hidden="true""#,
        r#"data-ars-part="hidden-input""#,
        r#"type="checkbox""#,
        r#"name="terms""#,
        r#"value="on""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert!(
        !html.contains(r#" checked"#),
        "unchecked hidden input must not be checked: {html}"
    );
}

#[test]
fn checkbox_renders_checked_indeterminate_and_form_states() {
    fn checked_app() -> Element {
        rsx! {
            checkbox::Root {
                id: "newsletter",
                checked: checkbox::State::Checked,
                name: "newsletter",
                value: "weekly",
                form: "preferences",
                required: true,
                checkbox::Label { "Newsletter" }
                checkbox::Control { checkbox::Indicator {} }
                checkbox::HiddenInput {}
            }
        }
    }

    let checked_html = render_app(checked_app);

    for fragment in [
        r#"aria-checked="true""#,
        r#"data-ars-state="checked""#,
        r#" checked"#,
        r#"name="newsletter""#,
        r#"value="weekly""#,
        r#"form="preferences""#,
        r#"required"#,
        r#"aria-required="true""#,
    ] {
        assert!(
            checked_html.contains(fragment),
            "missing {fragment}: {checked_html}"
        );
    }

    fn mixed_app() -> Element {
        rsx! {
            checkbox::Root { id: "partial", checked: checkbox::State::Indeterminate,
                checkbox::Label { "Some selected" }
                checkbox::Control { checkbox::Indicator {} }
                checkbox::HiddenInput {}
            }
        }
    }

    let mixed_html = render_app(mixed_app);

    assert!(
        mixed_html.contains(r#"aria-checked="mixed""#),
        "missing mixed aria state: {mixed_html}"
    );
    assert!(
        mixed_html.contains(r#"data-ars-state="indeterminate""#),
        "missing indeterminate state token: {mixed_html}"
    );
    assert!(
        !mixed_html.contains(r#" checked"#),
        "indeterminate hidden input must not submit as checked: {mixed_html}"
    );
}

#[test]
fn checkbox_renders_disabled_readonly_invalid_description_and_user_attrs() {
    fn app() -> Element {
        rsx! {
            checkbox::Root {
                id: "legal",
                disabled: true,
                readonly: true,
                invalid: true,
                has_description: true,
                has_error_message: true,
                class: "app-checkbox",
                style: "display: grid;",
                checkbox::Label { "Legal terms" }
                checkbox::Control { checkbox::Indicator {} }
                checkbox::HiddenInput {}
                checkbox::Description { "Needed for account access" }
                checkbox::ErrorMessage { "You must accept" }
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"class="app-checkbox""#,
        r#"style="display: grid;""#,
        r#"data-ars-disabled"#,
        r#"data-ars-readonly"#,
        r#"data-ars-invalid"#,
        r#"aria-disabled="true""#,
        r#"aria-readonly="true""#,
        r#"aria-invalid="true""#,
        r#"aria-errormessage="legal-error-message""#,
        r#"aria-describedby="legal-description legal-error-message""#,
        r#"data-ars-part="description""#,
        r#"data-ars-part="error-message""#,
        r#"aria-live="polite""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert!(
        !html.contains(r#"for="legal-hidden-input""#),
        "readonly label must not target hidden native checkbox: {html}"
    );
}

#[test]
fn checkbox_compound_parts_link_description_and_error_without_manual_presence_flags() {
    fn app() -> Element {
        rsx! {
            checkbox::Root { id: "legal", invalid: true,
                checkbox::Label { "Legal terms" }
                checkbox::Control { checkbox::Indicator {} }
                checkbox::HiddenInput {}
                checkbox::Description { "Needed for account access" }
                checkbox::ErrorMessage { "You must accept" }
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"aria-errormessage="legal-error-message""#,
        r#"aria-describedby="legal-description legal-error-message""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn checkbox_inherits_fieldset_state() {
    fn app() -> Element {
        rsx! {
            fieldset::Root {
                id: "settings",
                disabled: true,
                readonly: true,
                invalid: true,
                checkbox::Root { id: "legal",
                    checkbox::Label { "Legal terms" }
                    checkbox::Control { checkbox::Indicator {} }
                    checkbox::HiddenInput {}
                }
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"id="legal""#,
        r#"data-ars-disabled"#,
        r#"data-ars-readonly"#,
        r#"data-ars-invalid"#,
        r#"aria-disabled="true""#,
        r#"aria-readonly="true""#,
        r#"aria-invalid="true""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn checkbox_inherits_matching_form_validation_errors_by_name() {
    fn app() -> Element {
        rsx! {
            form::Root {
                id: "preferences",
                validation_errors: BTreeMap::from([
                    ("terms".to_string(), vec![Error::server("Accept terms before continuing.")]),
                ]),
                checkbox::Root { id: "terms", name: "terms", has_error_message: true,
                    checkbox::Label { "Accept terms" }
                    checkbox::Control { checkbox::Indicator {} }
                    checkbox::HiddenInput {}
                    checkbox::ErrorMessage { "Accept terms before continuing." }
                }
                checkbox::Root { id: "newsletter", name: "newsletter",
                    checkbox::Label { "Newsletter" }
                    checkbox::Control { checkbox::Indicator {} }
                    checkbox::HiddenInput {}
                }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"id="terms""#) && html.contains(r#"aria-invalid="true""#),
        "matching checkbox should inherit form validation errors: {html}"
    );
    assert!(
        html.contains(r#"aria-errormessage="terms-error-message""#),
        "matching checkbox error message should be referenced: {html}"
    );
    assert!(
        !html.contains(r#"aria-errormessage="newsletter-error-message""#),
        "unmatched checkbox should not inherit form validation errors: {html}"
    );
}

#[test]
fn checkbox_compound_parts_forward_consumer_attrs() {
    fn app() -> Element {
        rsx! {
            checkbox::Root { id: "styled", class: "root-class", style: "display: grid;",
                checkbox::Label { class: "label-class", style: "color: blue;", "Styled checkbox" }
                checkbox::Control { class: "control-class", style: "border-color: red;",
                    checkbox::Indicator { class: "indicator-class", style: "opacity: 1;" }
                }
                checkbox::HiddenInput { class: "input-class", style: "position: absolute;" }
                checkbox::Description { class: "description-class", style: "font-size: 12px;", "Help text" }
                checkbox::ErrorMessage { class: "error-class", style: "color: red;", "Error text" }
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"class="root-class""#,
        r#"style="display: grid;""#,
        r#"class="label-class""#,
        r#"style="color: blue;""#,
        r#"class="control-class""#,
        r#"style="border-color: red;""#,
        r#"class="indicator-class""#,
        r#"style="opacity: 1;""#,
        r#"class="input-class ars-sr-input""#,
        r#"style="position: absolute;""#,
        r#"class="description-class""#,
        r#"style="font-size: 12px;""#,
        r#"class="error-class""#,
        r#"style="color: red;""#,
        r#"data-ars-part="root""#,
        r#"data-ars-part="label""#,
        r#"data-ars-part="control""#,
        r#"data-ars-part="indicator""#,
        r#"data-ars-part="hidden-input""#,
        r#"data-ars-part="description""#,
        r#"data-ars-part="error-message""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}
