//! Browser tests for the Leptos `ArsProvider` adapter.
//!
//! Complements the SSR oracle (`tests/ars_provider.rs`) and the inline wasm
//! tests in `src/provider.rs::wasm_tests`. These tests cover the spec §28
//! scenarios that require a real DOM:
//!
//! - All-props configuration round-trip observable via context.
//! - Nonce auto-render attaches a `<style nonce="...">` element under the
//!   provider boundary.
//! - `use_style_strategy()` falls back to `StyleStrategy::Inline` outside of
//!   any provider.

#![cfg(target_arch = "wasm32")]

use ars_core::{ColorMode, StyleStrategy};
use ars_i18n::{Direction, Locale};
use ars_leptos::{
    ArsContext, ArsNonceCssCtx, ArsProvider, append_nonce_css, use_direction, use_style_strategy,
};
use leptos::{mount::mount_to, prelude::*};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

fn document() -> web_sys::Document {
    web_sys::window()
        .and_then(|window| window.document())
        .expect("document should exist")
}

fn container() -> web_sys::HtmlElement {
    let element = document()
        .create_element("div")
        .expect("container should be created");

    document()
        .body()
        .expect("body should exist")
        .append_child(&element)
        .expect("container should attach");

    element
        .dyn_into::<web_sys::HtmlElement>()
        .expect("container should be an HtmlElement")
}

#[leptos::component]
fn ConfigProbe() -> impl IntoView {
    let context = use_context::<ArsContext>().expect("ArsProvider should publish ArsContext");
    let style_strategy_label = format!("{:?}", use_style_strategy());

    // Dogfood `use_direction()` (spec §22 required helper) at the
    // integration tier — the inline tests cover the unit-level fallback
    // contract; this verifies the hook resolves correctly when read from
    // inside a real `ArsProvider`-mounted subtree.
    let direction = use_direction();
    let locale = context.locale;
    let color_mode = context.color_mode;
    let disabled = context.disabled;
    let read_only = context.read_only;
    let id_prefix = context.id_prefix;
    let portal_container_id = context.portal_container_id;
    let root_node_id = context.root_node_id;

    view! {
        <div data-testid="config-probe">
            <span data-testid="locale">{move || locale.get().to_bcp47()}</span>
            <span data-testid="direction">{move || direction.get().as_html_attr()}</span>
            <span data-testid="color-mode">{move || format!("{:?}", color_mode.get())}</span>
            <span data-testid="disabled">{move || disabled.get().to_string()}</span>
            <span data-testid="read-only">{move || read_only.get().to_string()}</span>
            <span data-testid="id-prefix">{move || id_prefix.get().unwrap_or_default()}</span>
            <span data-testid="portal-container-id">
                {move || portal_container_id.get().unwrap_or_default()}
            </span>
            <span data-testid="root-node-id">{move || root_node_id.get().unwrap_or_default()}</span>
            <span data-testid="style-strategy">{style_strategy_label}</span>
        </div>
    }
}

#[leptos::component]
fn NonceProbe() -> impl IntoView {
    let nonce_context =
        use_context::<ArsNonceCssCtx>().expect("ArsProvider should publish nonce context");

    append_nonce_css(String::from(".probe { color: red; }"));

    view! {
        <span data-testid="nonce-rules-count">
            {move || nonce_context.rules.get().len().to_string()}
        </span>
    }
}

#[leptos::component]
fn FallbackProbe() -> impl IntoView {
    let strategy_label = format!("{:?}", use_style_strategy());

    view! { <span data-testid="fallback-style-strategy">{strategy_label}</span> }
}

fn text_for(container: &web_sys::HtmlElement, selector: &str) -> String {
    container
        .query_selector(selector)
        .expect("selector should be valid")
        .unwrap_or_else(|| panic!("{selector} should exist"))
        .text_content()
        .expect("element should have text content")
}

#[cfg(feature = "csr")]
#[wasm_bindgen_test]
async fn ars_provider_publishes_all_configured_fields_to_descendants_on_wasm() {
    let container = container();

    let mount_handle = mount_to(container.clone(), move || {
        view! {
            <ArsProvider
                locale=Signal::stored(Locale::parse("es-ES").expect("locale should parse"))
                direction=Signal::stored(Direction::Rtl)
                color_mode=Signal::stored(ColorMode::Dark)
                disabled=Signal::stored(true)
                read_only=Signal::stored(true)
                id_prefix=String::from("app-prefix")
                portal_container_id=String::from("portal-root")
                root_node_id=String::from("focus-root")
                style_strategy=StyleStrategy::Cssom
            >
                <ConfigProbe />
            </ArsProvider>
        }
    });

    leptos::task::tick().await;

    assert_eq!(text_for(&container, "[data-testid='locale']"), "es-ES");
    assert_eq!(text_for(&container, "[data-testid='direction']"), "rtl");
    assert_eq!(text_for(&container, "[data-testid='color-mode']"), "Dark");
    assert_eq!(text_for(&container, "[data-testid='disabled']"), "true");
    assert_eq!(text_for(&container, "[data-testid='read-only']"), "true");
    assert_eq!(
        text_for(&container, "[data-testid='id-prefix']"),
        "app-prefix"
    );
    assert_eq!(
        text_for(&container, "[data-testid='portal-container-id']"),
        "portal-root"
    );
    assert_eq!(
        text_for(&container, "[data-testid='root-node-id']"),
        "focus-root"
    );
    assert_eq!(
        text_for(&container, "[data-testid='style-strategy']"),
        "Cssom"
    );

    let wrapper = container
        .query_selector("[dir='rtl']")
        .expect("selector should be valid")
        .expect("RTL wrapper should exist");

    assert_eq!(wrapper.get_attribute("dir").as_deref(), Some("rtl"));

    drop(mount_handle);

    container.remove();
}

#[cfg(feature = "csr")]
#[wasm_bindgen_test]
async fn ars_provider_with_nonce_strategy_auto_renders_style_tag_in_dom_on_wasm() {
    let container = container();

    let mount_handle = mount_to(container.clone(), move || {
        view! {
            <ArsProvider style_strategy=StyleStrategy::Nonce(String::from("dom-nonce"))>
                <NonceProbe />
            </ArsProvider>
        }
    });

    leptos::task::tick().await;

    assert_eq!(
        text_for(&container, "[data-testid='nonce-rules-count']"),
        "1",
        "probe should have appended exactly one rule"
    );

    let style = container
        .query_selector("style[nonce='dom-nonce']")
        .expect("selector should be valid")
        .expect("nonce-bearing style tag should be in the DOM");

    let css = style
        .text_content()
        .expect("style tag should have CSS text");

    assert!(
        css.contains(".probe { color: red; }"),
        "appended CSS rules should land in the nonce style tag: {css}"
    );

    drop(mount_handle);

    container.remove();
}

#[cfg(feature = "csr")]
#[wasm_bindgen_test]
async fn use_style_strategy_falls_back_to_inline_without_provider_on_wasm() {
    let container = container();

    let mount_handle = mount_to(container.clone(), move || {
        view! { <FallbackProbe /> }
    });

    leptos::task::tick().await;

    assert_eq!(
        text_for(&container, "[data-testid='fallback-style-strategy']"),
        "Inline",
        "use_style_strategy must default to Inline without ArsProvider in scope"
    );

    drop(mount_handle);

    container.remove();
}
