//! SSR tests for the Dioxus Button adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_components::utility::button;
use ars_dioxus::utility::button::{Button, ButtonAsChild};
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

fn default_app() -> Element {
    rsx! {
        Button { id: "primary-action", variant: button::Variant::Primary, "Save" }
    }
}

#[test]
fn button_renders_default_root_and_content_attrs() {
    let html = render_app(default_app);

    assert!(html.contains("<button"), "missing button root: {html}");
    assert!(
        html.contains(r#"id="primary-action""#),
        "missing explicit id: {html}"
    );
    assert!(
        html.contains(r#"data-ars-scope="button""#),
        "missing button scope: {html}"
    );
    assert!(
        html.contains(r#"data-ars-part="root""#),
        "missing root part: {html}"
    );
    assert!(
        html.contains(r#"type="button""#),
        "missing native button type: {html}"
    );
    assert!(
        html.contains(r#"data-ars-variant="primary""#),
        "missing typed variant token: {html}"
    );
    assert!(
        html.contains(r#"data-ars-size="md""#),
        "missing default size token: {html}"
    );
    assert!(
        html.contains(r#"data-ars-part="content""#),
        "missing content part wrapper: {html}"
    );
    assert!(
        !html.contains(r#"data-ars-part="loading-indicator""#),
        "idle button should not render loading indicator: {html}"
    );
}

#[test]
fn button_renders_loading_indicator_and_busy_attrs() {
    fn app() -> Element {
        rsx! {
            Button { id: "loading-action", loading: true, "Save" }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"aria-busy="true""#,
        r#"aria-disabled="true""#,
        r#"data-ars-part="loading-indicator""#,
        r#"role="status""#,
        r#"aria-live="polite""#,
        r#"aria-label="Loading""#,
        r#"data-ars-loading="true""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn button_renders_disabled_and_form_override_attrs() {
    fn app() -> Element {
        rsx! {
            Button {
                id: "submit-action",
                disabled: true,
                r#type: button::Type::Submit,
                form: "account-form",
                name: "intent",
                value: "save",
                form_action: "/submit",
                form_method: button::FormMethod::Post,
                form_enc_type: button::FormEncType::MultipartFormData,
                form_target: button::FormTarget::Self_,
                form_no_validate: true,
                "Submit"
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"disabled"#,
        r#"aria-disabled="true""#,
        r#"data-ars-disabled"#,
        r#"type="submit""#,
        r#"form="account-form""#,
        r#"name="intent""#,
        r#"value="save""#,
        r#"formaction="/submit""#,
        r#"formmethod="post""#,
        r#"formenctype="multipart/form-data""#,
        r#"formtarget="_self""#,
        r#"formnovalidate"#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn button_renders_user_facing_root_attrs() {
    fn app() -> Element {
        rsx! {
            Button {
                id: "customized-action",
                class: "app-button",
                style: "min-width: 12rem;",
                aria_label: "Save account",
                aria_labelledby: "save-label",
                "Save"
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"class="app-button""#,
        r#"style="min-width: 12rem;""#,
        r#"aria-label="Save account""#,
        r#"aria-labelledby="save-label""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn button_accepts_ergonomic_into_string_props() {
    fn app() -> Element {
        rsx! {
            Button {
                id: "ergonomic-action",
                form: "ergonomic-form",
                name: "intent",
                value: "save",
                form_action: "/submit",
                class: "app-button",
                style: "min-width: 8rem;",
                aria_label: "Save account",
                aria_labelledby: "save-label",
                "Save"
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"id="ergonomic-action""#,
        r#"form="ergonomic-form""#,
        r#"name="intent""#,
        r#"value="save""#,
        r#"formaction="/submit""#,
        r#"class="app-button""#,
        r#"style="min-width: 8rem;""#,
        r#"aria-label="Save account""#,
        r#"aria-labelledby="save-label""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn button_as_child_forwards_root_attrs_without_wrapper() {
    fn app() -> Element {
        rsx! {
            ButtonAsChild {
                id: "docs-link",
                variant: button::Variant::Link,
                render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                    a { href: "/docs", ..slot.attrs, "Docs" }
                },
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<a"),
        "as-child should render the callback root directly: {html}"
    );
    assert!(html.contains(r#"href="/docs""#), "missing href: {html}");
    assert!(
        html.contains(r#"data-ars-scope="button""#),
        "missing forwarded scope: {html}"
    );
    assert!(
        html.contains(r#"data-ars-part="root""#),
        "missing forwarded root part: {html}"
    );
    assert!(
        html.contains(r#"data-ars-variant="link""#),
        "missing forwarded variant: {html}"
    );
    assert!(
        !html.contains("<button"),
        "unexpected wrapper button: {html}"
    );
    assert!(
        !html.contains(r#"type="button""#),
        "as-child must not apply native button type attrs to arbitrary children: {html}"
    );
}

#[test]
fn button_as_child_filters_native_button_attrs() {
    fn app() -> Element {
        rsx! {
            ButtonAsChild {
                id: "docs-link",
                render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                    a { href: "/docs", ..slot.attrs, "Docs" }
                },
            }
        }
    }

    let html = render_app(app);

    for fragment in [
        r#"type="submit""#,
        r#"form="external-form""#,
        r#"name="intent""#,
        r#"value="save""#,
        r#"formaction="/submit""#,
        r#"formmethod="post""#,
        r#"formenctype="application/x-www-form-urlencoded""#,
        r#"formtarget="_self""#,
        r#"formnovalidate"#,
    ] {
        assert!(
            !html.contains(fragment),
            "as-child non-native attrs should be filtered ({fragment}): {html}"
        );
    }
}

#[test]
fn button_ssr_markup_is_deterministic() {
    let first = render_app(default_app);
    let second = render_app(default_app);

    assert_eq!(
        first, second,
        "SSR output must be byte-identical across renders"
    );
}
