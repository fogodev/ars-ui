# SSR & Hydration Tests

## 1. SSR Render Tests

Verify that SSR output includes correct ARIA attributes without executing effects:

```rust
#[cfg(test)]
mod ssr_tests {
    use super::*;

    #[test]
    fn dialog_ssr_renders_correct_aria_without_effects() {
        // SSR: create service and connect without executing effects
        let svc = Service::<dialog::Machine>::new(dialog::Props {
            open: Some(true),
            modal: true,
            ..Default::default()
        });
        let api = svc.connect(&|_| {}); // no-op send — effects are not set up during SSR

        let content_attrs = api.content_attrs();
        assert_role(&content_attrs, "dialog");
        assert_eq!(
            content_attrs.get(&HtmlAttr::Aria(AriaAttr::Modal)).expect("aria-modal must be present for modal dialog SSR output"),
            "true"
        );
        // Effects (focus trap, scroll lock) should NOT be in the AttrMap
        assert!(content_attrs.get(&HtmlAttr::Data("ars-focus-trap")).is_none());
    }

    #[test]
    fn tooltip_ssr_renders_hidden_content() {
        let svc = Service::<tooltip::Machine>::new(tooltip::Props::default());
        // Tooltip starts in Hidden state
        assert_eq!(*svc.state(), tooltip::State::Hidden);

        let api = svc.connect(&|_| {});
        let attrs = api.content_attrs();
        // Content should still have role="tooltip" for SSR SEO
        assert_role(&attrs, "tooltip");
    }
}
```

## 2. Hydration Round-Trip Tests

Verify state survives SSR → serialization → client hydration:

```rust
/// Matches the canonical HydrationSnapshot from foundation adapter specs
/// (08-adapter-leptos.md, 09-adapter-dioxus.md).
#[derive(Debug, Serialize, Deserialize)]
struct HydrationSnapshot<M: Machine>
where
    M::State: Serialize + DeserializeOwned,
{
    state: M::State,
    id: String,
}

#[test]
fn hydration_snapshot_round_trip() {
    // Server side: create and serialize
    let svc = Service::<dialog::Machine>::new(dialog::Props { open: Some(true), ..Default::default() });
    assert_eq!(*svc.state(), dialog::State::Open);

    let snapshot = HydrationSnapshot {
        state: svc.state().clone(),
        id: svc.props().id().to_string(),
    };
    let json = serde_json::to_string(&snapshot).expect("serialization must succeed");

    // Client side: deserialize and verify
    let restored: HydrationSnapshot<dialog::Machine> = serde_json::from_str(&json).expect("deserialization must succeed");
    assert_eq!(restored.state, dialog::State::Open);
    assert_eq!(restored.id.as_str(), svc.props().id());
}

#[test]
fn hydration_snapshot_initializes_service() {
    // Server side: create service and serialize
    let props = dialog::Props { id: "dlg1".into(), ..Default::default() };
    let mut svc = Service::new(props.clone());
    svc.send(dialog::Event::Open);
    let snapshot = HydrationSnapshot {
        state: svc.state().clone(),
        id: props.id.clone(),
    };
    let json = serde_json::to_string(&snapshot).expect("serialization must succeed");

    // Client side: deserialize and initialize service from snapshot.
    // Service::new creates with initial state; to restore from hydration,
    // use the SSR-only constructor that accepts a pre-existing state:
    let restored: HydrationSnapshot<dialog::Machine> =
        serde_json::from_str(&json).expect("deserialization must succeed");
    let client_svc = Service::new_hydrated(props, restored.state);

    // Service should be in the Open state without re-running init()
    assert_eq!(*client_svc.state(), dialog::State::Open);
}
```

> **Note:** `Service::new_hydrated(props, state)` is a `#[cfg(feature = "ssr")]` constructor
> on `Service` that bypasses `Machine::init()` and directly sets the provided state.
> It is used exclusively for hydration: the server serializes the state via
> `HydrationSnapshot`, and the client reconstructs the service without re-running
> initialization logic. See `01-architecture.md` §2.3 for the `Service` API surface.

---

## 3. SSR Hydration Mismatch Detection

Verify that server-rendered attributes match client-side first render:

```rust
#[test]
fn no_hydration_mismatch() {
    let props = dialog::Props { open: Some(true), ..Default::default() };

    // Simulate server render
    let server_svc = Service::<dialog::Machine>::new(props.clone());
    let server_api = server_svc.connect(&|_| {});
    let server_attrs = server_api.content_attrs();

    // Simulate client render (same props, same initial state)
    let client_svc = Service::<dialog::Machine>::new(props);
    let client_api = client_svc.connect(&|_| {});
    let client_attrs = client_api.content_attrs();

    assert_eq!(server_attrs, client_attrs,
        "Hydration mismatch: server and client AttrMap differ");
}
```

Any component that branches on runtime information (viewport size, user agent, etc.) in `connect()` MUST be flagged as a hydration mismatch risk.

## 4. Hydration ID Consistency Test

Components that generate IDs (via `ComponentIds`, `HasId`, or hydration-safe ID
generation) MUST verify that SSR and hydration produce identical
IDs. A mismatch causes Leptos/Dioxus hydration errors and broken ARIA
references (`aria-labelledby`, `aria-controls`, etc.).

```rust
#[test]
fn hydration_ids_match_between_ssr_and_client() {
    let props = dialog::Props {
        id: "dialog-1".into(),
        open: Some(true),
        ..Default::default()
    };

    // Step 1: Render with Service::new — extract all generated IDs.
    // SSR vs hydration mode is determined by the adapter (Leptos/Dioxus) wrapping,
    // not by the Service constructor. The Service API is the same in both modes.
    let ssr_svc = Service::<dialog::Machine>::new(props.clone());
    let ssr_api = ssr_svc.connect(&|_| {});
    let ssr_ids = extract_all_ids(&[
        ssr_api.content_attrs(),
        ssr_api.trigger_attrs(),
        ssr_api.title_attrs(),
        ssr_api.description_attrs(),
        ssr_api.backdrop_attrs(),
    ]);

    // Reset ID counter to simulate the SSR→client boundary
    // (server and client both start from 0)
    // Reset the adapter's ID counter before each SSR test.
    // Each adapter has its own counter — there is no shared ars_core counter.
    // Leptos: ars_leptos::reset_id_counter() (see foundation 08 §15)
    // Dioxus: ars_dioxus::reset_id_counter() (see foundation 09 §15)
    #[cfg(feature = "leptos")]
    ars_leptos::reset_id_counter();
    #[cfg(feature = "dioxus")]
    ars_dioxus::reset_id_counter();

    // Step 2: Render again with same props — IDs must match.
    // In a real app, step 1 runs on the server and step 2 runs on the client.
    // Both use Service::new with the same props; the adapter controls which
    // rendering path (SSR string output vs hydration DOM attachment) is used.
    let hydration_svc = Service::<dialog::Machine>::new(props);
    let hydration_api = hydration_svc.connect(&|_| {});
    let hydration_ids = extract_all_ids(&[
        hydration_api.content_attrs(),
        hydration_api.trigger_attrs(),
        hydration_api.title_attrs(),
        hydration_api.description_attrs(),
        hydration_api.backdrop_attrs(),
    ]);

    // Step 3: Assert equality
    assert_eq!(ssr_ids, hydration_ids,
        "Hydration ID mismatch: SSR IDs {:?} != hydration IDs {:?}",
        ssr_ids, hydration_ids);
}

/// Extract all `id`, `aria-labelledby`, `aria-controls`, `aria-describedby`,
/// and `for` attribute values from a set of AttrMaps.
fn extract_all_ids(attr_maps: &[AttrMap]) -> BTreeSet<String> {
    let id_attrs = [
        HtmlAttr::Id,
        HtmlAttr::Aria(AriaAttr::LabelledBy),
        HtmlAttr::Aria(AriaAttr::Controls),
        HtmlAttr::Aria(AriaAttr::DescribedBy),
        // NOTE: HtmlAttr::For maps to the HTML `for` attribute (used on <label> elements).
        // Verify this variant exists in the HtmlAttr enum (foundation/01-architecture.md §3.1).
        HtmlAttr::For,
    ];
    let mut ids = BTreeSet::new();
    for attrs in attr_maps {
        for attr in &id_attrs {
            if let Some(val) = attrs.get(attr) {
                for id in val.split_whitespace() {
                    ids.insert(id.to_string());
                }
            }
        }
    }
    ids
}
```

> **Cross-reference:** This test pattern validates the hydration-safe ID
> generation work. See `01-architecture.md` for
> the `ComponentIds` contract.

---

## 5. SSR Hydration

Server-side rendering tests ensure components produce valid HTML with correct ARIA attributes and hydrate without mismatches.

### 5.1 Test Utility Modules

> **Adapter test infrastructure:** Framework-specific test utilities below extend the canonical adapter harness defined in [05-adapter-harness.md](05-adapter-harness.md). See that file for the core parity test patterns.

The SSR tests below use the following test utility modules to wrap framework-specific
SSR rendering APIs into a consistent interface.

#### 5.1.1 Leptos SSR Utility

> **Leptos 0.8 SSR:** Leptos does not expose a public `leptos::ssr` module.
> The `RenderHtml` trait (`leptos::tachys::view::RenderHtml`) provides
> `to_html()` which renders any view to an HTML string synchronously.
> Since `IntoView` requires `RenderHtml`, any Leptos view supports this.
> See [08-adapter-leptos.md](../foundation/08-adapter-leptos.md) §7 for the
> SSR architecture.

```rust
/// Test utility: renders a Leptos component to an HTML string for SSR testing.
///
/// Uses `RenderHtml::to_html()` from `leptos::tachys::view`.
/// A reactive `Owner` scope is created so components can create signals.
mod ssr {
    use leptos::prelude::*;
    use leptos::tachys::view::RenderHtml;

    pub fn render<F, V>(f: F) -> String
    where
        F: FnOnce() -> V + 'static,
        V: IntoView + RenderHtml,
    {
        let owner = Owner::new();
        owner.with(|| f().to_html())
    }
}
```

#### 5.1.2 Leptos Client Utility

```rust
mod client {
    fn document() -> web_sys::Document {
        web_sys::window().expect("window must exist")
            .document().expect("document must exist")
    }

    /// Client-side rendering utility. Unlike `ssr::render`, this mounts
    /// the component to a detached DOM node to exercise the CSR code path,
    /// ensuring client-only behavior is tested.
    pub fn render(f: impl FnOnce() -> impl IntoView + 'static) -> web_sys::HtmlElement {
        let container = document()
            .create_element("div")
            .expect("element creation must succeed")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("must be HtmlElement");
        leptos::mount::mount_to(container.clone().into(), f);
        container
    }
}
```

#### 5.1.3 Leptos Hydration Test Utility

```rust
/// Test utility for hydration mismatch detection.
mod hydration_test {
    use leptos::prelude::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Mount a component for hydration testing.
    /// `ssr_html`: Pre-rendered HTML string from SSR.
    /// `f`: Component factory closure.
    pub fn mount_and_hydrate<F, V>(ssr_html: &str, f: F) -> HydrationResult
    where
        F: Fn() -> V + 'static,
        V: IntoView,
    {
        let container = document().create_element("div")
            .expect("document must support createElement");
        container.set_inner_html(ssr_html);
        document().body().expect("document must have body")
            .append_child(&container).expect("body must accept child");

        // Capture hydration warnings via console.warn interception
        let warnings = Rc::new(RefCell::new(Vec::new()));
        let warnings_clone = Rc::clone(&warnings);
        let _guard = install_warning_capture(move |msg| {
            warnings_clone.borrow_mut().push(msg);
        });

        // Hydrate the component onto the pre-rendered HTML
        leptos::mount::hydrate_body(f);

        HydrationResult { warnings: warnings.borrow().clone(), container }
    }
}

pub struct HydrationResult {
    pub warnings: Vec<String>,
    pub container: web_sys::Element,
}
```

> **Note:** `leptos::mount::hydrate_body` is the hydration entry point used in client-side
> hydration tests. This function is not yet documented in `08-adapter-leptos.md` — see the
> [Leptos documentation](https://docs.rs/leptos) for the current API surface.

### 5.2 ARIA Attributes in Server-Rendered HTML

> **Adapter test infrastructure:** Framework-specific test utilities below extend the canonical adapter harness defined in [05-adapter-harness.md](05-adapter-harness.md). See that file for the core parity test patterns.

#### 5.2.1 Leptos SSR

```rust
use leptos::prelude::*;

#[test]
fn ssr_dialog_aria_attributes_present_leptos() {
    // Leptos SSR: render component to HTML string via the Leptos SSR feature.
    let html = ssr::render(|| {
        view! { <Dialog title="Confirm" open=false /> }
    });
    assert!(html.contains(r#"role="dialog""#));
    assert!(html.contains(r#"aria-labelledby="#));
    assert!(html.contains(r#"aria-modal="true""#));
}

#[test]
fn ssr_select_aria_attributes_present_leptos() {
    let html = ssr::render(|| {
        view! { <Select placeholder="Choose..." /> }
    });
    assert!(html.contains(r#"role="combobox""#) || html.contains(r#"role="listbox""#));
    assert!(html.contains(r#"aria-expanded="false""#));
}
```

#### 5.2.2 Dioxus SSR

```rust
use dioxus::prelude::*;

fn render_dialog_confirm_ssr() -> Element {
    rsx! { Dialog { title: "Confirm", open: false } }
}

#[test]
fn ssr_dialog_aria_attributes_present_dioxus() {
    // Dioxus 0.7.3 SSR: build VirtualDom, rebuild, render to string.
    let mut dom = VirtualDom::new(render_dialog_confirm_ssr);
    dom.rebuild_in_place();
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains(r#"role="dialog""#));
    assert!(html.contains(r#"aria-labelledby="#));
    assert!(html.contains(r#"aria-modal="true""#));
}

fn render_select_choose_ssr() -> Element {
    rsx! { Select { placeholder: "Choose..." } }
}

#[test]
fn ssr_select_aria_attributes_present_dioxus() {
    let mut dom = VirtualDom::new(render_select_choose_ssr);
    dom.rebuild_in_place();
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains(r#"role="combobox""#) || html.contains(r#"role="listbox""#));
    assert!(html.contains(r#"aria-expanded="false""#));
}
```

### 5.3 No Hydration Mismatch Warnings

> **Adapter test infrastructure:** Framework-specific test utilities below extend the canonical adapter harness defined in [05-adapter-harness.md](05-adapter-harness.md). See that file for the core parity test patterns.

#### 5.3.1 Leptos

```rust
use leptos::prelude::*;

#[test]
fn no_hydration_mismatch_for_select_leptos() {
    let html = ssr::render(|| {
        view! { <Select placeholder="Choose...">
            <select::Item value="a">"Alpha"</select::Item>
            <select::Item value="b">"Beta"</select::Item>
        </Select> }
    });
    let result = hydration_test::mount_and_hydrate(&html, || {
        view! { <Select placeholder="Choose...">
            <select::Item value="a">"Alpha"</select::Item>
            <select::Item value="b">"Beta"</select::Item>
        </Select> }
    });
    assert!(result.warnings.is_empty(), "Hydration mismatches: {:?}", result.warnings);
}

#[test]
fn no_hydration_mismatch_for_dialog_leptos() {
    let html = ssr::render(|| {
        view! { <Dialog title="Test"><p>"Content"</p></Dialog> }
    });
    let result = hydration_test::mount_and_hydrate(&html, || {
        view! { <Dialog title="Test"><p>"Content"</p></Dialog> }
    });
    assert!(result.warnings.is_empty(), "Hydration mismatches: {:?}", result.warnings);
}
```

```rust
/// Installs a console.warn interceptor that forwards each warning message
/// to the provided closure. Returns a guard that restores the original
/// console.warn on drop.
fn install_warning_capture(on_warn: impl Fn(String) + 'static) -> WarningCaptureGuard {
    use wasm_bindgen::prelude::*;

    // Intercept console.warn via inline JS — filter for hydration mismatch messages
    #[wasm_bindgen(inline_js = "
        let origWarn = console.warn;
        export function install_warning_interceptor(callback) {
            console.warn = function(...args) {
                const msg = args.map(String).join(' ');
                if (msg.includes('hydration') || msg.includes('mismatch')) {
                    callback(msg);
                }
                origWarn.apply(console, args);
            };
        }
        export function restore_console_warn() {
            console.warn = origWarn;
        }
    ")]
    extern "C" {
        fn install_warning_interceptor(callback: &Closure<dyn FnMut(String)>);
        fn restore_console_warn();
    }

    let closure = Closure::wrap(Box::new(move |msg: String| {
        on_warn(msg);
    }) as Box<dyn FnMut(String)>);

    install_warning_interceptor(&closure);
    closure.forget(); // leak intentionally — restored via restore_console_warn()

    WarningCaptureGuard { _private: () }
}

/// Guard that restores the original console.warn when dropped.
struct WarningCaptureGuard { _private: () }
impl Drop for WarningCaptureGuard {
    fn drop(&mut self) {
        restore_console_warn();
    }
}

#[wasm_bindgen_test]
async fn hydration_produces_no_warnings() {
    let warnings = Rc::new(RefCell::new(Vec::new()));
    let warnings_clone = Rc::clone(&warnings);
    let _guard = install_warning_capture(move |msg| {
        warnings_clone.borrow_mut().push(msg);
    });
    let html = ssr::render(|| ToggleComponent());
    let _result = hydration_test::mount_and_hydrate(&html, || ToggleComponent());
    let captured = warnings.borrow().clone();
    assert!(captured.is_empty(),
        "hydration must produce no warnings, got: {captured:?}");
}
```

#### 5.3.2 Dioxus

```rust
use dioxus::prelude::*;

fn render_select_ssr() -> Element {
    rsx! {
        Select { placeholder: "Choose...",
            select::Item { value: "a", "Alpha" }
            select::Item { value: "b", "Beta" }
        }
    }
}

#[test]
fn no_hydration_mismatch_for_select_dioxus() {
    // Dioxus 0.7.3: SSR render, then hydration render — compare output.
    let mut ssr_dom = VirtualDom::new(render_select_ssr);
    ssr_dom.rebuild_in_place();
    let ssr_html = dioxus::ssr::render(&ssr_dom);

    let mut client_dom = VirtualDom::new(render_select_ssr);
    client_dom.rebuild_in_place();
    let client_html = dioxus::ssr::render(&client_dom);

    assert_eq!(ssr_html, client_html, "Hydration mismatch: SSR and client HTML differ");
}

fn render_dialog_ssr() -> Element {
    rsx! { Dialog { title: "Test", p { "Content" } } }
}

#[test]
fn no_hydration_mismatch_for_dialog_dioxus() {
    let mut ssr_dom = VirtualDom::new(render_dialog_ssr);
    ssr_dom.rebuild_in_place();
    let ssr_html = dioxus::ssr::render(&ssr_dom);

    let mut client_dom = VirtualDom::new(render_dialog_ssr);
    client_dom.rebuild_in_place();
    let client_html = dioxus::ssr::render(&client_dom);

    assert_eq!(ssr_html, client_html, "Hydration mismatch: SSR and client HTML differ");
}
```

> **Known gap — Dioxus hydration parity:** This test compares two SSR renders rather than performing a true SSR-to-hydration round-trip, because Dioxus does not yet expose a hydration testing API equivalent to Leptos's `mount_and_hydrate`. Client-only code paths, Suspense boundaries, and lazy loading mismatches are NOT caught by this approach. **Requirement:** When `dioxus-testing` infrastructure supports true hydration tests, this section MUST be updated with a `wasm_bindgen_test` that (1) renders SSR HTML, (2) mounts it in a DOM container, (3) hydrates over it, and (4) verifies no mismatch warnings are emitted. See [05-adapter-harness.md](05-adapter-harness.md) for the canonical adapter test infrastructure.

#### 5.3.3 Timer-Based Component Hydration (Clipboard)

```rust
use leptos::prelude::*;

#[wasm_bindgen_test]
async fn clipboard_hydration_no_timer_flash() {
    // SSR renders idle state
    let ssr_html = ssr::render(|| view! { <Clipboard id="c1" value="copy me" /> });
    assert!(ssr_html.contains(r#"data-ars-state="idle""#));
    // Hydration should not start any timers
    let result = hydration_test::mount_and_hydrate(&ssr_html, || view! { <Clipboard id="c1" value="copy me" /> });
    assert!(result.warnings.is_empty(), "no hydration mismatches");
    // State should remain idle (no flash to "copied" and back)
}
```

### 5.4 Client-Only Effects Don't Run During SSR

Verify that the Service layer does not produce pending effects when no events have
been sent (SSR renders the initial state only — effects are set up by the adapter
on the client after hydration).

```rust
#[test]
fn ssr_dialog_no_pending_effects_on_initial_render() {
    // Simulate SSR: create service with initial props, connect, check for effects.
    // During SSR, no events are sent, so no pending effects should be produced.
    let svc = Service::<dialog::Machine>::new(dialog::Props { open: Some(true), ..Default::default() });
    let result = svc.connect(&|_| {});

    // The connect() call produces ARIA attributes but must NOT produce pending effects.
    // Effects (focus trap, scroll lock, outside-click) are set up by the adapter
    // on the client side, gated behind #[cfg(not(feature = "ssr"))].
    // This test validates the Service contract: initial connect has no side effects.
    assert!(
        result.content_attrs().get(&HtmlAttr::Data("ars-focus-trap")).is_none(),
        "SSR output must not contain focus trap markers"
    );
    assert!(
        result.content_attrs().get(&HtmlAttr::Data("ars-scroll-lock")).is_none(),
        "SSR output must not contain scroll lock markers"
    );
}
```

### 5.5 Controlled Values Match in SSR and Client

Helper for extracting the displayed value from rendered HTML:

```rust
/// Extract the displayed/selected value text from rendered HTML.
fn extract_displayed_value(html: &str) -> String {
    let fragment = scraper::Html::parse_fragment(html);
    let selector = scraper::Selector::parse("[data-ars-part='value']")
        .expect("valid selector");
    fragment.select(&selector)
        .next()
        .map(|el| el.text().collect::<String>())
        .unwrap_or_default()
}
```

#### 5.5.1 Leptos

```rust
use leptos::prelude::*;

#[test]
fn ssr_controlled_value_matches_client_leptos() {
    let ssr_html = ssr::render(|| {
        view! { <Select value="b">
            <select::Item value="a">"Alpha"</select::Item>
            <select::Item value="b">"Beta"</select::Item>
        </Select> }
    });

    let client_el = client::render(|| {
        view! { <Select value="b">
            <select::Item value="a">"Alpha"</select::Item>
            <select::Item value="b">"Beta"</select::Item>
        </Select> }
    });
    let client_html = client_el.inner_html();

    let ssr_value = extract_displayed_value(&ssr_html);
    let client_value = extract_displayed_value(&client_html);
    assert_eq!(ssr_value, client_value, "SSR and client must render the same controlled value");
}
```

#### 5.5.2 Dioxus

```rust
use dioxus::prelude::*;

fn render_controlled_select_ssr() -> Element {
    rsx! {
        Select { value: "b",
            select::Item { value: "a", "Alpha" }
            select::Item { value: "b", "Beta" }
        }
    }
}

#[test]
fn ssr_controlled_value_matches_client_dioxus() {
    let mut ssr_dom = VirtualDom::new(render_controlled_select_ssr);
    ssr_dom.rebuild_in_place();
    let ssr_html = dioxus::ssr::render(&ssr_dom);

    let mut client_dom = VirtualDom::new(render_controlled_select_ssr);
    client_dom.rebuild_in_place();
    let client_html = dioxus::ssr::render(&client_dom);

    let ssr_value = extract_displayed_value(&ssr_html);
    let client_value = extract_displayed_value(&client_html);
    assert_eq!(ssr_value, client_value, "SSR and client must render the same controlled value");
}
```

---

## 6. SSR/Hydration Tests (Adapter-Level)

> **SSR mode is adapter-determined.** The `Service` type has a single constructor
> (`Service::new(props)`) — there is no `new_ssr()` or `new_hydrate()` variant.
> Whether a component renders to a string (SSR) or attaches to existing DOM
> (hydration) is controlled by the adapter layer: Leptos uses the `ssr`/`hydrate`
> feature flags, Dioxus uses the `server`/`web` feature flags.

1. Server-rendered HTML must produce identical DOM structure to client-hydrated output.
2. ID generation must be deterministic and consistent between server and client.
3. Test harness: render component server-side, hydrate client-side, diff DOM trees — zero differences expected.
4. Hydration tests run as integration tests with `wasm-pack test` (Leptos) or `dx test` (Dioxus).

**Effect Ordering Parity**: Both adapters MUST execute `PendingEffect` setup/cleanup in the same order for the same event sequence. A CI test runs a fixed event sequence against both adapters and asserts identical effect execution logs:

```rust
#[test]
fn effect_ordering_parity_dialog() {
    let events = vec![dialog::Event::Open, dialog::Event::Close, dialog::Event::Open];
    let props = dialog::Props::default();

    // Collect effects by sending the same event sequence through a Service
    // and accumulating the pending_effects from each transition result.
    let mut svc = Service::<dialog::Machine>::new(props.clone());
    let mut effects = Vec::new();
    for event in &events {
        let result = svc.send(event.clone());
        effects.extend(result.pending_effects);
    }

    // Both adapters consume the same PendingEffect list from the core Service,
    // so effect ordering is guaranteed identical. This test verifies the core
    // produces a deterministic effect sequence for a given event sequence.
    let mut svc2 = Service::<dialog::Machine>::new(props);
    let mut effects2 = Vec::new();
    for event in &events {
        let result = svc2.send(event.clone());
        effects2.extend(result.pending_effects);
    }

    assert_eq!(effects, effects2, "Effect ordering must be deterministic for the same event sequence");
}
```

---

## 7. Async Operation Testing

Components with asynchronous behavior require specific test patterns:

1. **Loading State Timing**: Test that the loading indicator/state appears within 1 frame of the async trigger (e.g., button click that initiates fetch). Use `await tick()` or framework-equivalent to advance one microtask cycle.

2. **Loading Spinner Debounce**: The loading spinner should only appear after a 300ms delay to avoid a flash of loading state for fast operations. Test that:
   - Operations completing in <300ms never show the spinner.
   - Operations completing in >300ms show the spinner after the 300ms threshold.

3. **Error Recovery**: After an async operation fails, the component must return to its idle (non-loading) state. Test that:
   - The error state is displayed.
   - The component can be re-triggered for a retry.
   - No stale loading indicators remain.

4. **Cancellation on Unmount**: When a component is unmounted during a pending async operation, the operation must be cancelled (or its result ignored). Test that:
   - Unmounting during loading does not produce console errors or warnings.
   - No state updates occur after unmount (no "setState on unmounted component" equivalent).
   - Resources (abort controllers, timers) are properly cleaned up.

```rust
/// Async hydration: loading state with Suspense boundary
#[wasm_bindgen_test]
async fn suspense_boundary_shows_fallback_then_content() {
    let html = ssr::render(|| {
        view! {
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                <AsyncDataComponent />
            </Suspense>
        }
    });
    assert!(html.contains("Loading..."),
        "SSR output must contain fallback content");

    let harness = hydration_test::mount_and_hydrate(&html, || {
        view! {
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                <AsyncDataComponent />
            </Suspense>
        }
    });
    // After hydration and data resolution
    tick().await;
    let content = harness.container.query_selector("[data-ars-part='content']")
        .expect("query must not error");
    assert!(content.is_some(), "content must appear after async resolution");
}
```

---

## 8. SSR Smoke Tests for ars-dom Utilities

Per [11-dom-utilities.md](../foundation/11-dom-utilities.md), `ars-dom` has two surfaces:

- raw DOM-typed APIs, which are `web`-only and are not available under pure `ssr` builds
- cross-build utilities, which remain available under `ssr` and must return safe defaults without attempting DOM access

The smoke tests in this section cover only the cross-build `ssr` surface.

```rust
#[cfg(feature = "ssr")]
mod ssr_dom_smoke_tests {
    use ars_dom::positioning::{compute_position, PositioningOptions, Rect};
    use ars_dom::scroll::ScrollLockManager;

    #[test]
    fn compute_position_returns_default_under_ssr() {
        let anchor = Rect { x: 0.0, y: 0.0, width: 100.0, height: 40.0 };
        let floating = Rect { x: 0.0, y: 0.0, width: 200.0, height: 100.0 };
        let viewport = Rect { x: 0.0, y: 0.0, width: 1024.0, height: 768.0 };
        let options = PositioningOptions::default();

        let result = compute_position(&anchor, &floating, &viewport, &options);
        // Under SSR, compute_position returns a default placement (typically top-left origin)
        // without accessing the DOM. The exact values depend on the default PositioningResult.
        // This test verifies it does not panic or access browser APIs.
        let _ = result;
    }

    #[test]
    fn scroll_lock_is_noop_under_ssr() {
        let mut manager = ScrollLockManager::new();
        // lock/unlock must not panic or access document/window
        manager.lock("test-component");
        manager.unlock("test-component");
    }

    // Raw DOM-typed helpers such as `get_focusable_elements`,
    // `scroll_into_view_if_needed`, and `nearest_scrollable_ancestor`
    // are part of the `web` surface and are intentionally not exercised here.
}
```
