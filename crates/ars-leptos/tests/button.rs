//! SSR tests for the Leptos Button adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_components::utility::button;
use ars_leptos::{
    prelude::SafeUrl,
    utility::button::{Button, ButtonAsChild},
};
use leptos::prelude::*;

fn render_default_button() -> String {
    view! {
        <Button id="primary-action" variant=button::Variant::Primary>
            "Save"
        </Button>
    }
    .to_html()
}

#[test]
fn button_renders_default_root_and_content_attrs() {
    let html = render_default_button();

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
    let html = view! {
        <Button id="loading-action" loading=true>
            "Save"
        </Button>
    }
    .to_html();

    assert!(
        html.contains(r#"aria-busy="true""#),
        "missing busy state: {html}"
    );
    assert!(
        html.contains(r#"aria-disabled="true""#),
        "missing loading-disabled state: {html}"
    );
    assert!(
        html.contains(r#"data-ars-part="loading-indicator""#),
        "missing loading indicator: {html}"
    );
    assert!(
        html.contains(r#"role="status""#),
        "missing status role: {html}"
    );
    assert!(
        html.contains(r#"aria-live="polite""#),
        "missing live region: {html}"
    );
    assert!(
        html.contains(r#"aria-label="Loading""#),
        "missing loading label: {html}"
    );
    assert!(
        html.contains(r#"data-ars-loading="true""#),
        "missing loading data attr: {html}"
    );
}

#[test]
fn button_renders_disabled_and_form_override_attrs() {
    let action = SafeUrl::from_static("/submit");

    let html = view! {
        <Button
            id="submit-action"
            disabled=true
            r#type=button::Type::Submit
            form="account-form"
            name="intent"
            value="save"
            form_action=action
            form_method=button::FormMethod::Post
            form_enc_type=button::FormEncType::MultipartFormData
            form_target=button::FormTarget::Self_
            form_no_validate=true
        >
            "Submit"
        </Button>
    }
    .to_html();

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
    let html = view! {
        <Button
            id="customized-action"
            class="app-button"
            style="min-width: 12rem;"
            aria_label="Save account"
            aria_labelledby="save-label"
        >
            "Save"
        </Button>
    }
    .to_html();

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
    let action = SafeUrl::from_static("/submit");

    let html = view! {
        <Button
            id="ergonomic-action"
            form="ergonomic-form"
            name="intent"
            value="save"
            form_action=action
            class="app-button"
            style="min-width: 8rem;"
            aria_label="Save account"
            aria_labelledby="save-label"
        >
            "Save"
        </Button>
    }
    .to_html();

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
    let html = view! {
        <ButtonAsChild id="docs-link" variant=button::Variant::Link>
            <a href="/docs">"Docs"</a>
        </ButtonAsChild>
    }
    .to_html();

    assert!(
        html.trim_start().starts_with("<a"),
        "as-child should render the child root directly: {html}"
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
    let html = view! {
        <ButtonAsChild id="docs-link">
            <a href="/docs">"Docs"</a>
        </ButtonAsChild>
    }
    .to_html();

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
    let first = render_default_button();
    let second = render_default_button();

    assert_eq!(
        first, second,
        "SSR output must be byte-identical across renders"
    );
}
