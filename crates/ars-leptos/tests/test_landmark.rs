//! SSR tests for the Leptos `Landmark` adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_core::MessageFn;
use ars_leptos::utility::landmark::{Landmark, Messages, Role};
use leptos::{prelude::*, reactive::owner::Owner};

fn render(view_fn: impl FnOnce() -> String + 'static) -> String {
    let owner = Owner::new();
    let result = owner.with(view_fn);

    drop(owner);

    result
}

fn messages(label: &'static str) -> Messages {
    Messages {
        label: MessageFn::static_str(label),
    }
}

#[test]
fn landmark_region_renders_section_with_aria_label() {
    let html = render(|| {
        view! {
            <Landmark id="region" messages=messages("Activity")>
                "content"
            </Landmark>
        }
        .to_html()
    });

    assert!(
        html.trim_start().starts_with("<section"),
        "default Landmark should render a section root: {html}"
    );

    for fragment in [
        r#"id="region""#,
        r#"data-ars-scope="landmark""#,
        r#"data-ars-part="root""#,
        r#"aria-label="Activity""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert!(
        !html.contains(r#"role="region""#),
        "native section landmark must not emit explicit role: {html}"
    );
}

#[test]
fn landmark_banner_renders_native_header() {
    let html = render(|| {
        view! {
            <Landmark id="b" role=Role::Banner messages=messages("Site banner")>
                "top"
            </Landmark>
        }
        .to_html()
    });

    assert!(
        html.trim_start().starts_with("<header"),
        "expected <header>: {html}"
    );
    assert!(!html.contains(r#"role="banner""#));
    assert!(html.contains(r#"aria-label="Site banner""#));
}

#[test]
fn landmark_navigation_renders_native_nav() {
    let html = render(|| {
        view! {
            <Landmark id="n" role=Role::Navigation messages=messages("Primary navigation")>
                "links"
            </Landmark>
        }
        .to_html()
    });

    assert!(html.trim_start().starts_with("<nav"));
    assert!(!html.contains(r#"role="navigation""#));
    assert!(html.contains(r#"aria-label="Primary navigation""#));
}

#[test]
fn landmark_main_renders_native_main() {
    let html = render(|| {
        view! {
            <Landmark id="m" role=Role::Main>
                "main"
            </Landmark>
        }
        .to_html()
    });

    assert!(html.trim_start().starts_with("<main"));
    assert!(!html.contains(r#"role="main""#));
}

#[test]
fn landmark_complementary_renders_native_aside() {
    let html = render(|| {
        view! {
            <Landmark id="c" role=Role::Complementary messages=messages("Related")>
                "side"
            </Landmark>
        }
        .to_html()
    });

    assert!(html.trim_start().starts_with("<aside"));
    assert!(!html.contains(r#"role="complementary""#));
}

#[test]
fn landmark_content_info_renders_native_footer() {
    let html = render(|| {
        view! {
            <Landmark id="f" role=Role::ContentInfo>
                "footer"
            </Landmark>
        }
        .to_html()
    });

    assert!(html.trim_start().starts_with("<footer"));
    assert!(!html.contains(r#"role="contentinfo""#));
}

#[test]
fn landmark_form_renders_native_form() {
    let html = render(|| {
        view! {
            <Landmark id="form" role=Role::Form messages=messages("Account settings")>
                "fields"
            </Landmark>
        }
        .to_html()
    });

    assert!(html.trim_start().starts_with("<form"));
    assert!(!html.contains(r#"role="form""#));
    assert!(html.contains(r#"aria-label="Account settings""#));
}

#[test]
fn landmark_search_falls_back_to_div_with_explicit_role() {
    let html = render(|| {
        view! {
            <Landmark id="search" role=Role::Search messages=messages("Site search")>
                "input"
            </Landmark>
        }
        .to_html()
    });

    assert!(
        html.trim_start().starts_with("<div"),
        "Search must use a div fallback: {html}"
    );
    assert!(html.contains(r#"role="search""#));
    assert!(html.contains(r#"aria-label="Site search""#));
}

#[test]
fn landmark_labelledby_takes_precedence_over_aria_label() {
    let html = render(|| {
        view! {
            <Landmark
                id="r"
                role=Role::Region
                labelledby_id="external-label"
                messages=messages("ignored")
            >
                "content"
            </Landmark>
        }
        .to_html()
    });

    assert!(html.contains(r#"aria-labelledby="external-label""#));
    assert!(
        !html.contains(r#"aria-label="#),
        "labelledby must suppress aria-label: {html}"
    );
}

#[test]
fn landmark_without_id_does_not_emit_id_attr() {
    let html = render(|| view! { <Landmark>"content"</Landmark> }.to_html());

    assert!(
        !html.contains("id="),
        "passive Landmark must not emit id=: {html}"
    );
}

#[test]
fn landmark_navigation_without_messages_omits_aria_label() {
    let html = render(|| {
        view! {
            <Landmark id="n" role=Role::Navigation>
                "links"
            </Landmark>
        }
        .to_html()
    });

    assert!(
        !html.contains("aria-label"),
        "missing accessible name should omit aria-label entirely: {html}"
    );
}

#[test]
fn landmark_blank_labelledby_is_treated_as_missing() {
    let html = render(|| {
        view! {
            <Landmark
                id="blank"
                role=Role::Region
                labelledby_id="   "
                messages=messages("Activity")
            >
                "content"
            </Landmark>
        }
        .to_html()
    });

    // Blank labelledby is filtered, so aria-labelledby is suppressed and
    // aria-label falls through.
    assert!(!html.contains("aria-labelledby"));
    assert!(html.contains(r#"aria-label="Activity""#));
}
