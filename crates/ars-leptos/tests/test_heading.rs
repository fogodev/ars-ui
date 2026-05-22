//! SSR tests for the Leptos `Heading` adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_leptos::utility::heading::{Heading, HeadingLevelProvider, Level, Section};
use leptos::{prelude::*, reactive::owner::Owner};

fn render(view_fn: impl FnOnce() -> String + 'static) -> String {
    let owner = Owner::new();
    let result = owner.with(view_fn);

    drop(owner);

    result
}

#[test]
fn heading_renders_h1_by_default() {
    let html = view! { <Heading id="title">"Hello"</Heading> }.to_html();

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
        "native heading element must not emit explicit role: {html}"
    );

    assert!(
        !html.contains("aria-level"),
        "native heading element must not emit aria-level: {html}"
    );
}

#[test]
fn heading_explicit_level_overrides_default() {
    let html = view! {
        <Heading id="three" level=Level::Three>
            "Three"
        </Heading>
    }
    .to_html();

    assert!(
        html.trim_start().starts_with("<h3"),
        "expected h3 root: {html}"
    );
    assert!(html.contains(r#"id="three""#));
}

#[test]
fn heading_renders_each_level_one_through_six() {
    let cases = [
        (Level::One, "<h1"),
        (Level::Two, "<h2"),
        (Level::Three, "<h3"),
        (Level::Four, "<h4"),
        (Level::Five, "<h5"),
        (Level::Six, "<h6"),
    ];

    for (level, expected_tag) in cases {
        let html = view! { <Heading level=level>"Level"</Heading> }.to_html();

        assert!(
            html.trim_start().starts_with(expected_tag),
            "expected {expected_tag} for {level:?}: {html}"
        );
    }
}

#[test]
fn heading_without_id_does_not_emit_id_attr() {
    let html = view! { <Heading>"Hello"</Heading> }.to_html();

    assert!(
        !html.contains("id="),
        "passive Heading must not emit id=: {html}"
    );
}

#[test]
fn heading_level_provider_publishes_starting_level() {
    let html = render(|| {
        view! {
            <HeadingLevelProvider level=Level::Four>
                <Heading id="auto-four">"Four"</Heading>
            </HeadingLevelProvider>
        }
        .to_html()
    });

    assert!(
        html.trim_start().starts_with("<h4"),
        "HeadingLevelProvider should publish Level::Four context: {html}"
    );
}

#[test]
fn section_increments_inherited_level() {
    let html = render(|| {
        view! {
            <Section>
                <Heading id="auto-two">"Two"</Heading>
            </Section>
        }
        .to_html()
    });

    assert!(
        html.trim_start().starts_with("<h2"),
        "Section should bump Level::One to Level::Two: {html}"
    );
}

#[test]
fn nested_sections_clamp_at_level_six() {
    let html = render(|| {
        view! {
            <Section>
                <Section>
                    <Section>
                        <Section>
                            <Section>
                                <Section>
                                    <Section>
                                        <Heading id="deep">"Six"</Heading>
                                    </Section>
                                </Section>
                            </Section>
                        </Section>
                    </Section>
                </Section>
            </Section>
        }
        .to_html()
    });

    assert!(
        html.trim_start().starts_with("<h6"),
        "nested Sections beyond Level::Six should clamp at h6: {html}"
    );
}

#[test]
fn heading_explicit_level_overrides_inherited_context() {
    let html = render(|| {
        view! {
            <HeadingLevelProvider level=Level::Four>
                <Heading id="override" level=Level::Two>
                    "Two"
                </Heading>
            </HeadingLevelProvider>
        }
        .to_html()
    });

    assert!(
        html.trim_start().starts_with("<h2"),
        "explicit level must override provider-inherited context: {html}"
    );
}

#[test]
fn heading_level_provider_renders_no_dom_of_its_own() {
    let html = render(|| {
        view! {
            <HeadingLevelProvider level=Level::Two>
                <span>"child"</span>
            </HeadingLevelProvider>
        }
        .to_html()
    });

    assert!(
        html.trim_start().starts_with("<span"),
        "HeadingLevelProvider must be provider-only with no DOM wrapper: {html}"
    );
}

#[test]
fn section_renders_no_dom_of_its_own() {
    let html = render(|| {
        view! {
            <Section>
                <span>"child"</span>
            </Section>
        }
        .to_html()
    });

    assert!(
        html.trim_start().starts_with("<span"),
        "Section must be provider-only with no DOM wrapper: {html}"
    );
}
