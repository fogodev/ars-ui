//! SSR tests for the Dioxus `Heading` adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_dioxus::utility::heading::{Heading, HeadingLevelProvider, Level, Section};
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    dioxus_ssr::render(&vdom)
}

#[test]
fn heading_renders_h1_by_default() {
    fn app() -> Element {
        rsx! {
            Heading { id: "title", "Hello" }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<h1"),
        "default Heading should render an h1 root: {html}"
    );

    for fragment in [
        r#"id="title""#,
        r#"data-ars-scope="heading""#,
        r#"data-ars-part="root""#,
        "Hello",
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert!(
        !html.contains(r#"role="heading""#),
        "native heading must not emit explicit role: {html}"
    );
    assert!(
        !html.contains("aria-level"),
        "native heading must not emit aria-level: {html}"
    );
}

#[test]
fn heading_explicit_level_overrides_default() {
    fn app() -> Element {
        rsx! {
            Heading { id: "three", level: Level::Three, "Three" }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<h3"),
        "expected h3 root: {html}"
    );
}

#[test]
fn heading_renders_each_level_one_through_six() {
    fn one() -> Element {
        rsx! {
            Heading { level: Level::One, "x" }
        }
    }

    fn two() -> Element {
        rsx! {
            Heading { level: Level::Two, "x" }
        }
    }

    fn three() -> Element {
        rsx! {
            Heading { level: Level::Three, "x" }
        }
    }

    fn four() -> Element {
        rsx! {
            Heading { level: Level::Four, "x" }
        }
    }

    fn five() -> Element {
        rsx! {
            Heading { level: Level::Five, "x" }
        }
    }

    fn six() -> Element {
        rsx! {
            Heading { level: Level::Six, "x" }
        }
    }

    for (renderer, expected) in [
        (one as fn() -> Element, "<h1"),
        (two, "<h2"),
        (three, "<h3"),
        (four, "<h4"),
        (five, "<h5"),
        (six, "<h6"),
    ] {
        let html = render_app(renderer);
        assert!(
            html.trim_start().starts_with(expected),
            "expected {expected}: {html}"
        );
    }
}

#[test]
fn heading_without_id_does_not_emit_id_attr() {
    fn app() -> Element {
        rsx! {
            Heading { "Hello" }
        }
    }

    let html = render_app(app);

    assert!(
        !html.contains("id="),
        "passive Heading must not emit id=: {html}"
    );
}

#[test]
fn heading_level_provider_publishes_starting_level() {
    fn app() -> Element {
        rsx! {
            HeadingLevelProvider { level: Level::Four,
                Heading { id: "auto-four", "Four" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<h4"),
        "HeadingLevelProvider should publish Level::Four context: {html}"
    );
}

#[test]
fn section_increments_inherited_level() {
    fn app() -> Element {
        rsx! {
            Section {
                Heading { id: "auto-two", "Two" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<h2"),
        "Section should bump Level::One to Level::Two: {html}"
    );
}

#[test]
fn nested_sections_clamp_at_level_six() {
    fn app() -> Element {
        rsx! {
            Section {
                Section {
                    Section {
                        Section {
                            Section {
                                Section {
                                    Section {
                                        Heading { id: "deep", "Six" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<h6"),
        "nested Sections beyond Level::Six should clamp at h6: {html}"
    );
}

#[test]
fn heading_explicit_level_overrides_inherited_context() {
    fn app() -> Element {
        rsx! {
            HeadingLevelProvider { level: Level::Four,
                Heading { id: "override", level: Level::Two, "Two" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<h2"),
        "explicit level must override provider-inherited context: {html}"
    );
}

#[test]
fn heading_level_provider_renders_no_dom_of_its_own() {
    fn app() -> Element {
        rsx! {
            HeadingLevelProvider { level: Level::Two,
                span { "child" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<span"),
        "HeadingLevelProvider must be provider-only with no DOM wrapper: {html}"
    );
}

#[test]
fn section_renders_no_dom_of_its_own() {
    fn app() -> Element {
        rsx! {
            Section {
                span { "child" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<span"),
        "Section must be provider-only with no DOM wrapper: {html}"
    );
}
