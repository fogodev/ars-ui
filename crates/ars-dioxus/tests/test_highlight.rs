//! SSR tests for the Dioxus `Highlight` adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_dioxus::utility::highlight::{Highlight, MatchStrategy};
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    dioxus_ssr::render(&vdom)
}

#[test]
fn highlight_root_emits_dir_auto_and_scope_attrs() {
    fn app() -> Element {
        rsx! {
            Highlight { query: vec!["world".to_string()], text: "Hello world!" }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<span"),
        "Highlight root must be a span: {html}"
    );

    for fragment in [
        r#"data-ars-scope="highlight""#,
        r#"data-ars-part="root""#,
        r#"dir="auto""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn highlight_wraps_matched_text_in_mark() {
    fn app() -> Element {
        rsx! {
            Highlight { query: vec!["world".to_string()], text: "Hello world!" }
        }
    }

    let html = render_app(app);

    assert!(html.contains("<mark"), "expected <mark>: {html}");

    for fragment in [
        r#"data-ars-part="highlight-chunk""#,
        r#"data-ars-highlighted="true""#,
        ">world<",
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn highlight_non_matched_chunks_use_span() {
    fn app() -> Element {
        rsx! {
            Highlight { query: vec!["world".to_string()], text: "Hello world!" }
        }
    }

    let html = render_app(app);

    assert!(html.contains(r#"data-ars-highlighted="false""#));
    assert!(
        html.matches("<span").count() >= 2,
        "expected root span plus at least one unmatched chunk span: {html}"
    );
}

#[test]
fn highlight_empty_query_renders_single_unmatched_chunk() {
    fn app() -> Element {
        rsx! {
            Highlight { query: vec![], text: "Hello" }
        }
    }

    let html = render_app(app);

    assert!(!html.contains("<mark"));
    assert!(html.contains("Hello"));
    assert!(html.contains(r#"data-ars-highlighted="false""#));
}

#[test]
fn highlight_ignore_case_default_matches_uppercase_query() {
    fn app() -> Element {
        rsx! {
            Highlight { query: vec!["WORLD".to_string()], text: "Hello world!" }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains("<mark"),
        "case-insensitive match expected: {html}"
    );
}

#[test]
fn highlight_explicit_case_sensitivity_disables_fold_matching() {
    fn app() -> Element {
        rsx! {
            Highlight {
                query: vec!["WORLD".to_string()],
                text: "Hello world!",
                ignore_case: false,
            }
        }
    }

    let html = render_app(app);

    assert!(
        !html.contains("<mark"),
        "case-sensitive matching must not match different case: {html}"
    );
}

#[test]
fn highlight_starts_with_strategy_only_matches_prefix() {
    fn app() -> Element {
        rsx! {
            Highlight {
                query: vec!["world".to_string()],
                text: "Hello world!",
                match_strategy: MatchStrategy::StartsWith,
            }
        }
    }

    let html = render_app(app);

    assert!(
        !html.contains("<mark"),
        "StartsWith should not match mid-string: {html}"
    );
}

#[test]
fn highlight_starts_with_strategy_matches_leading_run() {
    fn app() -> Element {
        rsx! {
            Highlight {
                query: vec!["Hello".to_string()],
                text: "Hello world!",
                match_strategy: MatchStrategy::StartsWith,
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains("<mark"),
        "leading prefix should match: {html}"
    );
    assert!(html.contains(">Hello<"));
}

#[test]
fn highlight_empty_text_renders_root_only() {
    fn app() -> Element {
        rsx! {
            Highlight { query: vec!["x".to_string()], text: "" }
        }
    }

    let html = render_app(app);

    assert!(html.trim_start().starts_with("<span"));
    assert!(!html.contains("<mark"));
}
