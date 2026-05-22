//! SSR tests for the Dioxus `Landmark` adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_core::MessageFn;
use ars_dioxus::utility::landmark::{Landmark, Messages, Role};
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    dioxus_ssr::render(&vdom)
}

fn label(text: &'static str) -> Messages {
    Messages {
        label: MessageFn::static_str(text),
    }
}

#[test]
fn landmark_region_renders_section_with_aria_label() {
    fn app() -> Element {
        rsx! {
            Landmark { id: "region", messages: label("Activity"), "content" }
        }
    }

    let html = render_app(app);

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
    fn app() -> Element {
        rsx! {
            Landmark { id: "b", role: Role::Banner, messages: label("Site banner"), "top" }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<header"),
        "expected <header>: {html}"
    );
    assert!(!html.contains(r#"role="banner""#));
    assert!(html.contains(r#"aria-label="Site banner""#));
}

#[test]
fn landmark_navigation_renders_native_nav() {
    fn app() -> Element {
        rsx! {
            Landmark { id: "n", role: Role::Navigation, messages: label("Primary"), "links" }
        }
    }

    let html = render_app(app);

    assert!(html.trim_start().starts_with("<nav"));
    assert!(!html.contains(r#"role="navigation""#));
    assert!(html.contains(r#"aria-label="Primary""#));
}

#[test]
fn landmark_main_renders_native_main() {
    fn app() -> Element {
        rsx! {
            Landmark { id: "m", role: Role::Main, "main" }
        }
    }

    let html = render_app(app);

    assert!(html.trim_start().starts_with("<main"));
    assert!(!html.contains(r#"role="main""#));
}

#[test]
fn landmark_complementary_renders_native_aside() {
    fn app() -> Element {
        rsx! {
            Landmark {
                id: "c",
                role: Role::Complementary,
                messages: label("Related"),
                "side"
            }
        }
    }

    let html = render_app(app);

    assert!(html.trim_start().starts_with("<aside"));
    assert!(!html.contains(r#"role="complementary""#));
}

#[test]
fn landmark_content_info_renders_native_footer() {
    fn app() -> Element {
        rsx! {
            Landmark { id: "f", role: Role::ContentInfo, "footer" }
        }
    }

    let html = render_app(app);

    assert!(html.trim_start().starts_with("<footer"));
    assert!(!html.contains(r#"role="contentinfo""#));
}

#[test]
fn landmark_form_renders_native_form() {
    fn app() -> Element {
        rsx! {
            Landmark {
                id: "form",
                role: Role::Form,
                messages: label("Account settings"),
                "fields"
            }
        }
    }

    let html = render_app(app);

    assert!(html.trim_start().starts_with("<form"));
    assert!(!html.contains(r#"role="form""#));
    assert!(html.contains(r#"aria-label="Account settings""#));
}

#[test]
fn landmark_search_falls_back_to_div_with_explicit_role() {
    fn app() -> Element {
        rsx! {
            Landmark {
                id: "search",
                role: Role::Search,
                messages: label("Site search"),
                "input"
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<div"),
        "Search must use a div fallback: {html}"
    );
    assert!(html.contains(r#"role="search""#));
    assert!(html.contains(r#"aria-label="Site search""#));
}

#[test]
fn landmark_labelledby_takes_precedence_over_aria_label() {
    fn app() -> Element {
        rsx! {
            Landmark {
                id: "r",
                role: Role::Region,
                labelledby_id: "external-label",
                messages: label("ignored"),
                "content"
            }
        }
    }

    let html = render_app(app);

    assert!(html.contains(r#"aria-labelledby="external-label""#));
    assert!(
        !html.contains(r#"aria-label="#),
        "labelledby must suppress aria-label: {html}"
    );
}

#[test]
fn landmark_without_id_does_not_emit_id_attr() {
    fn app() -> Element {
        rsx! {
            Landmark { "content" }
        }
    }

    let html = render_app(app);

    assert!(
        !html.contains("id="),
        "passive Landmark must not emit id=: {html}"
    );
}

#[test]
fn landmark_navigation_without_messages_omits_aria_label() {
    fn app() -> Element {
        rsx! {
            Landmark { id: "n", role: Role::Navigation, "links" }
        }
    }

    let html = render_app(app);

    assert!(
        !html.contains("aria-label"),
        "missing accessible name should omit aria-label entirely: {html}"
    );
}

#[test]
fn landmark_blank_labelledby_is_treated_as_missing() {
    fn app() -> Element {
        rsx! {
            Landmark {
                id: "blank",
                role: Role::Region,
                labelledby_id: "   ",
                messages: label("Activity"),
                "content"
            }
        }
    }

    let html = render_app(app);

    assert!(!html.contains("aria-labelledby"));
    assert!(html.contains(r#"aria-label="Activity""#));
}
