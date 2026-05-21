//! Browser tests for the Dioxus `ArsProvider` adapter.
//!
//! Complements the SSR oracle (`tests/ars_provider.rs`) and the inline
//! `wasm_tests` module of `src/provider.rs`. The inline tests cover
//! mutation-log oracles for context publication and reactivity; this file
//! adds the integration-tier scenarios that require a real DOM mount via
//! `dioxus_web::launch::launch_virtual_dom`:
//!
//! - Provider auto-renders `<style nonce="...">` into the DOM when
//!   `style_strategy` is `StyleStrategy::Nonce(...)`.
//! - `use_platform()` outside any `ArsProvider` falls back to
//!   `default_dioxus_platform()`, which on `wasm32 + feature = "web"` is
//!   `WebPlatform`. The fingerprint is `new_id()` returning a UUID rather
//!   than `NullPlatform`'s `null-id-N` strings.

#![cfg(all(target_arch = "wasm32", feature = "web"))]

use std::{cell::RefCell, rc::Rc};

use ars_core::StyleStrategy;
use ars_dioxus::{ArsContext, ArsProvider, default_dioxus_platform, use_platform};
use dioxus::prelude::*;
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

async fn animation_frame_turn() {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let callback = wasm_bindgen::closure::Closure::once_into_js({
            let resolve = resolve.clone();
            move || {
                drop(resolve.call0(&wasm_bindgen::JsValue::UNDEFINED));
            }
        });

        web_sys::window()
            .expect("window should exist")
            .request_animation_frame(callback.unchecked_ref())
            .expect("requestAnimationFrame should succeed");
    });

    drop(wasm_bindgen_futures::JsFuture::from(promise).await);
}

#[component]
fn NonceTrigger() -> Element {
    let nonce_context = try_use_context::<ars_dioxus::ArsNonceCssCtx>()
        .expect("ArsProvider should publish nonce context");

    ars_dioxus::append_nonce_css(String::from(".probe { color: red; }"));

    rsx! {
        span { "data-testid": "nonce-rules-count", "{nonce_context.rules.read().len()}" }
    }
}

fn nonce_app() -> Element {
    rsx! {
        ArsProvider { style_strategy: StyleStrategy::Nonce(String::from("dom-nonce")), NonceTrigger {} }
    }
}

#[wasm_bindgen_test(async)]
async fn ars_provider_with_nonce_strategy_auto_renders_style_tag_in_dom_on_wasm() {
    let parent = container();

    let dom = VirtualDom::new(nonce_app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let style = parent
        .query_selector("style[nonce='dom-nonce']")
        .expect("selector should be valid")
        .expect("nonce-bearing style tag should be in the DOM");

    let css = style
        .text_content()
        .expect("style tag should have CSS text");

    assert!(
        css.contains(".probe { color: red; }"),
        "appended CSS rule should land in the nonce style tag on wasm: {css}"
    );

    let wrapper = parent
        .query_selector("[dir='ltr']")
        .expect("selector should be valid")
        .expect("provider wrapper should exist");

    assert_eq!(wrapper.get_attribute("dir").as_deref(), Some("ltr"));

    parent.remove();
}

#[derive(Clone, Props)]
struct PlatformIdProbeProps {
    outputs: Rc<RefCell<Vec<String>>>,
}

impl PartialEq for PlatformIdProbeProps {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.outputs, &other.outputs)
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus component props are passed by value"
)]
#[expect(non_snake_case, reason = "Dioxus components use PascalCase names")]
fn PlatformIdProbe(props: PlatformIdProbeProps) -> Element {
    let platform = use_platform();

    props.outputs.borrow_mut().push(platform.new_id());

    rsx! {
        div { "data-testid": "platform-id-probe" }
    }
}

#[wasm_bindgen_test]
fn use_platform_falls_back_to_web_platform_without_provider_on_wasm() {
    let outputs = Rc::new(RefCell::new(Vec::<String>::new()));

    fn app(outputs: Rc<RefCell<Vec<String>>>) -> Element {
        rsx! {
            PlatformIdProbe { outputs }
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&outputs));

    dom.rebuild_in_place();

    let ids = outputs.borrow();

    assert_eq!(ids.len(), 1, "PlatformIdProbe should have run once");

    let id = &ids[0];

    assert!(
        !id.starts_with("null-id-"),
        "on wasm32 + feature=\"web\", use_platform without provider must \
         resolve to WebPlatform (UUID), not NullPlatform: got {id}"
    );

    // `WebPlatform::new_id()` delegates to `crypto.randomUUID()`, which
    // produces strings of the form `xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx`
    // (36 chars with four hyphens). Spot-check the shape so the oracle
    // catches future regressions where the fallback chain reorders.
    assert_eq!(
        id.len(),
        36,
        "WebPlatform::new_id() should return a UUID (got {id})"
    );
    assert_eq!(
        id.chars().filter(|c| *c == '-').count(),
        4,
        "UUID v4 has 4 hyphen separators (got {id})"
    );
}

#[derive(Clone, Props)]
struct ContextPlatformProbeProps {
    outputs: Rc<RefCell<Vec<String>>>,
}

impl PartialEq for ContextPlatformProbeProps {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.outputs, &other.outputs)
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus component props are passed by value"
)]
#[expect(non_snake_case, reason = "Dioxus components use PascalCase names")]
fn ContextPlatformProbe(props: ContextPlatformProbeProps) -> Element {
    let context = try_use_context::<ArsContext>().expect("ArsProvider should publish ArsContext");

    props
        .outputs
        .borrow_mut()
        .push(context.dioxus_platform.new_id());

    rsx! {
        div {}
    }
}

#[wasm_bindgen_test]
fn ars_provider_publishes_default_dioxus_platform_when_prop_absent_on_wasm() {
    let outputs = Rc::new(RefCell::new(Vec::<String>::new()));

    fn app(outputs: Rc<RefCell<Vec<String>>>) -> Element {
        rsx! {
            ArsProvider {
                ContextPlatformProbe { outputs }
            }
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&outputs));

    dom.rebuild_in_place();

    let ids = outputs.borrow();

    assert_eq!(ids.len(), 1, "ContextPlatformProbe should have run once");

    let context_id = &ids[0];

    // Sanity-check by allocating a fresh `default_dioxus_platform()` and
    // confirming both produce IDs in the same family. On wasm32 + web that
    // family is UUID (36 chars, 4 hyphens). This pins the §29 oracle:
    // "use_platform() returns the feature-flag-appropriate impl".
    let direct_id = default_dioxus_platform().new_id();

    assert_eq!(
        context_id.len(),
        direct_id.len(),
        "context-resolved platform and direct default should produce \
         IDs of the same family (context={context_id} direct={direct_id})"
    );
    assert_eq!(
        context_id.chars().filter(|c| *c == '-').count(),
        direct_id.chars().filter(|c| *c == '-').count(),
        "same hyphen count proves the same platform family"
    );
    assert!(
        !context_id.starts_with("null-id-"),
        "feature=\"web\" on wasm32 must publish WebPlatform, not NullPlatform"
    );
}
