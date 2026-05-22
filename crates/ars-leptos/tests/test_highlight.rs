//! SSR tests for the Leptos `Highlight` adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr", feature = "icu4x"))]

use ars_leptos::utility::highlight::{Highlight, MatchStrategy};
use leptos::{prelude::*, reactive::owner::Owner};

fn render(view_fn: impl FnOnce() -> String + 'static) -> String {
    let owner = Owner::new();
    let result = owner.with(view_fn);
    drop(owner);
    result
}

#[test]
fn highlight_root_emits_dir_auto_and_scope_attrs() {
    let html = render(|| {
        view! { <Highlight query=vec!["world".into()] text="Hello world!".to_string() /> }.to_html()
    });

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
    let html = render(|| {
        view! { <Highlight query=vec!["world".into()] text="Hello world!".to_string() /> }.to_html()
    });

    assert!(
        html.contains("<mark"),
        "expected <mark> wrapper for matched text: {html}"
    );

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
    let html = render(|| {
        view! { <Highlight query=vec!["world".into()] text="Hello world!".to_string() /> }.to_html()
    });

    assert!(html.contains(r#"data-ars-highlighted="false""#));
    assert!(
        html.matches("<span").count() >= 2,
        "expected root span plus at least one unmatched chunk span: {html}"
    );
}

#[test]
fn highlight_empty_query_renders_single_unmatched_chunk() {
    let html = render(|| {
        let empty: Vec<String> = Vec::new();

        view! { <Highlight query=empty text="Hello".to_string() /> }.to_html()
    });

    assert!(!html.contains("<mark"));
    assert!(html.contains("Hello"));
    assert!(html.contains(r#"data-ars-highlighted="false""#));
}

#[test]
fn highlight_ignore_case_default_matches_uppercase_query() {
    let html = render(|| {
        view! { <Highlight query=vec!["WORLD".into()] text="Hello world!".to_string() /> }.to_html()
    });

    assert!(
        html.contains("<mark"),
        "case-insensitive match expected by default: {html}"
    );
}

#[test]
fn highlight_explicit_case_sensitivity_disables_fold_matching() {
    let html = render(|| {
        view! { <Highlight query=vec!["WORLD".into()] text="Hello world!".to_string() ignore_case=false /> }
        .to_html()
    });

    assert!(
        !html.contains("<mark"),
        "case-sensitive matching must not match different case: {html}"
    );
}

#[test]
fn highlight_starts_with_strategy_only_matches_prefix() {
    let html = render(|| {
        view! {
            <Highlight
                query=vec!["world".into()]
                text="Hello world!".to_string()
                match_strategy=MatchStrategy::StartsWith
            />
        }
        .to_html()
    });

    assert!(
        !html.contains("<mark"),
        "StartsWith should not match mid-string: {html}"
    );
}

#[test]
fn highlight_starts_with_strategy_matches_leading_run() {
    let html = render(|| {
        view! {
            <Highlight
                query=vec!["Hello".into()]
                text="Hello world!".to_string()
                match_strategy=MatchStrategy::StartsWith
            />
        }
        .to_html()
    });

    assert!(
        html.contains("<mark"),
        "leading prefix should match: {html}"
    );
    assert!(html.contains(">Hello<"));
}

#[test]
fn highlight_empty_text_renders_root_only() {
    let html =
        render(|| view! { <Highlight query=vec!["x".into()] text="".to_string() /> }.to_html());

    // Empty text produces no chunks per the core contract.
    assert!(html.trim_start().starts_with("<span"));
    assert!(!html.contains("<mark"));
}
