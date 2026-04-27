# Test Harness

## 1. Purpose and Scope

The `TestHarness` is the unified testing API for ars-ui adapter tests. It wraps a rendered component inside an isolated DOM container and provides methods for querying elements, simulating user interactions, and inspecting state -- all without framework-specific boilerplate.

### 1.1 Testing Tiers

1. **Service-level tests** -- pure Rust, no DOM. Exercise `Service::send()` / `transition()` / `connect()` directly. Do not use `TestHarness` (see [02-integration-tests.md](02-integration-tests.md)).
2. **Adapter-level tests (WASM DOM)** -- mount a real component into a WASM DOM and assert rendered output. Use `TestHarness` as their primary API. This is the primary adapter testing surface and covers all browser-runtime behavior (listeners, focus, pointer/keyboard interaction, ARIA queries, animation timing, …).
3. **Adapter-level tests (non-web Dioxus runtime)** -- mount a Dioxus component on a bare `VirtualDom` (no `dioxus-web`, no real WRY/webview window) to validate the `cfg(not(feature = "web"))` graceful-degrade path adapter components follow on Desktop, mobile, and SSR builds. Use [`ars_test_harness_dioxus::desktop::DesktopHarness`](#54-non-web-dioxus-backend) as the entry point. Tests in this tier express expectations through callbacks captured by the fixture component (no DOM querying). They run as plain `#[test]` on native targets, alongside the workspace's other native unit and integration tests.

> **Relationship to the three-tier model:** [00-overview.md section 1.1](00-overview.md#11-testing-tiers) defines a three-tier model (Unit, Integration, Adapter). The test harness applies to **tier 3 (Adapter)** only. Tier 1 (Unit) tests use `Machine::transition` directly; tier 2 (Integration) tests use `Service` directly. The harness abstracts the adapter-level mount/interact/assert cycle for tier 3, which now covers both the WASM-DOM sub-tier (item 2) and the non-web Dioxus sub-tier (item 3).

`TestHarness` renders a component through the adapter, then exposes a framework-agnostic DOM-query and interaction API. Test code looks the same regardless of whether the Leptos or Dioxus backend is active. The non-web Dioxus harness deliberately exposes a smaller, callback-oriented surface — it does not query DOM nodes, it only drives the runtime and observes fixture-side state.

### 1.2 Crate Structure

```filetree
ars-test-harness/          # Framework-agnostic harness API
  src/lib.rs               # TestHarness, render_with_backend(), core methods
  src/backend.rs           # HarnessBackend trait
  src/element.rs           # ElementHandle wrapper
  src/item.rs              # ItemHandle wrapper
  src/types.rs             # KeyboardKey, Point, Rect

ars-test-harness-leptos/   # Leptos backend + adapter-owned render() wrappers
ars-test-harness-dioxus/   # Dioxus backend + adapter-owned render() wrappers
  src/lib.rs               # wasm-only DioxusHarnessBackend + render() wrappers
  src/desktop.rs           # non-web Dioxus runtime (DesktopHarness)
ars-test-ext-{component}/  # Per-component extension crates (optional)
```

`ars-test-harness` stays backend-agnostic. Each adapter crate owns the
zero-argument `render(...)` and `mount_with_locale(...)` wrappers that delegate
to the core constructors with that adapter's concrete backend instance.

`ars-test-harness-dioxus::desktop` is gated on
`cfg(not(target_arch = "wasm32"))` and lives alongside the wasm-only
`DioxusHarnessBackend` in the same crate; native test runs see only the
desktop module, wasm-pack runs see only the wasm-tier API.

---

## 2. Core Harness

### 2.1 TestHarness Struct

```rust
/// Marker trait for components that can be mounted via the test harness.
/// Each adapter crate implements this for its component types.
///
/// See [foundation/01-architecture.md](../foundation/01-architecture.md) for
/// the Machine trait that components implement at the core level.
pub trait Component: 'static {
    /// The machine type this component wraps.
    type Machine: Machine;
}

/// Type-erased wrapper around `Service<M>` for framework-agnostic test access.
/// Allows the test harness to interact with any component's service without
/// knowing the concrete Machine type.
pub trait AnyService {
    /// Get the current state as a debug string.
    fn state_debug(&self) -> String;
    /// Get the root attributes for snapshot comparison.
    fn root_attrs(&self) -> AttrMap;
    /// Get attributes for a named part.
    fn part_attrs(&self, part: &str) -> AttrMap;
    /// Send an event by name (for generic test patterns).
    fn send_named(&mut self, event_name: &str);
    /// Send a concrete boxed event through the erased service boundary.
    fn send_boxed(&mut self, event: Box<dyn Any>);
}

// NOTE: M::Event: FromStr is required for AnyService::send_named().
// Components MUST implement FromStr for their Event type to be used with AnyService.
// M::State: Debug is already required by the Machine trait, so the bound below
// is redundant but kept explicit for documentation clarity.
impl<M: Machine> AnyService for Service<M>
where
    M::Event: std::str::FromStr,
{
    fn state_debug(&self) -> String {
        format!("{:?}", self.state())
    }
    /// Returns the root element's attributes by calling `connect()` with a no-op send closure.
    /// Event handler closures in the returned AttrMap are inert — use `Service::send()` directly
    /// for event dispatch in tests.
    fn root_attrs(&self) -> AttrMap {
        let api = M::connect(self.state(), self.context(), self.props(), &|_| {});
        api.part_attrs(<M::Api<'_> as ConnectApi>::Part::ROOT)
    }
    /// Returns attrs for a part looked up by name string.
    ///
    /// **Data-carrying Part limitation:** For data-carrying Part variants (e.g.,
    /// `Part::Item { id }`, `Part::Tab { index }`), `all()` yields instances with
    /// `Default::default()` for each field. ARIA attributes that depend on real
    /// data (such as `aria-controls` referencing a specific panel ID) will have
    /// default values (e.g., empty string or 0). For tests that need real data,
    /// use `part_attrs_typed` instead.
    fn part_attrs(&self, part: &str) -> AttrMap {
        let api = M::connect(self.state(), self.context(), self.props(), &|_| {});
        // Find the part variant matching the name string.
        // all() returns default-valued instances for data-carrying variants.
        for p in <M::Api<'_> as ConnectApi>::Part::all() {
            if p.name() == part {
                return api.part_attrs(p);
            }
        }
        panic!("part_attrs: no part named '{}' found for {}", part, std::any::type_name::<M>());
    }
    fn send_named(&mut self, event_name: &str) {
        // Parse the event name into the concrete event type.
        // Machine::Event must implement FromStr for name-based dispatch.
        let event = event_name.parse::<M::Event>()
            .unwrap_or_else(|_| panic!(
                "Unknown event name for {}: {}", std::any::type_name::<M>(), event_name
            ));
        let _ = self.send(event);
    }
    fn send_boxed(&mut self, event: Box<dyn Any>) {
        let event = event.downcast::<M::Event>()
            .unwrap_or_else(|_| panic!(
                "boxed event type mismatch for {}; expected {}",
                std::any::type_name::<M>(),
                std::any::type_name::<M::Event>(),
            ));
        let _ = self.send(*event);
    }
}

pub trait ServiceHarnessExt<M: Machine> {
    /// Returns attrs for a specific Part variant with real data.
    /// Use this instead of `part_attrs(&str)` when testing data-carrying Part
    /// variants that need actual IDs for correct ARIA attributes.
    fn part_attrs_typed<'a>(&'a self, part: <M::Api<'a> as ConnectApi>::Part) -> AttrMap;
}

impl<M: Machine> ServiceHarnessExt<M> for Service<M> {
    fn part_attrs_typed<'a>(&'a self, part: <M::Api<'a> as ConnectApi>::Part) -> AttrMap {
        let api = M::connect(self.state(), self.context(), self.props(), &|_| {});
        api.part_attrs(part)
    }
}

/// The primary test API. Wraps a rendered component in an isolated DOM container.
///
/// Created via adapter-owned `render()` wrappers backed by `render_with_backend()`.
/// Each harness gets its own `<div>` appended to
/// `<body>`, removed on `Drop`. Tests never share DOM state.
pub struct TestHarness {
    container: web_sys::HtmlElement,
    service: RefCell<Box<dyn AnyService>>,
    backend: Box<dyn HarnessBackend>,
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        self.container.remove();
    }
}
```

### 2.2 Core Render Entry Point

```rust
/// Mount a component into an isolated DOM container using an explicit backend.
///
/// Adapter crates expose zero-argument `render(...)` wrappers that delegate to
/// this function with their concrete backend implementation.
pub async fn render_with_backend<C: Component, B: HarnessBackend>(
    component: C,
    backend: B,
) -> TestHarness {
    let container = create_isolated_container();
    // create_isolated_container() appends a <div data-ars-test-container> to <body>
    let service = backend.mount(&container, Box::new(component)).await;
    backend.flush().await;
    TestHarness { container, service, backend }
}
```

### 2.3 Locale-Aware Mounting

```rust
/// Mount a component with a specific locale context and explicit backend.
/// Delegates locale-wrapping to the `HarnessBackend` implementation,
/// which uses framework-specific environment provider components.
/// Used by i18n tests ([08-i18n-testing.md](08-i18n-testing.md)) to verify
/// RTL layout, locale-dependent formatting, and IME behavior.
pub async fn render_with_locale_and_backend<C: Component, B: HarnessBackend>(
    component: C,
    locale: ars_i18n::Locale,
    backend: B,
) -> TestHarness {
    let container = create_isolated_container();
    let service = backend.mount_with_locale(&container, Box::new(component), locale).await;
    backend.flush().await;
    TestHarness { container, service, backend }
}
```

### 2.4 DOM Query Methods

```rust
impl TestHarness {
    /// Query a single element by CSS selector within this container.
    pub fn query_selector(&self, selector: &str) -> Option<ElementHandle>;

    /// Query all matching elements in document order.
    pub fn query_selector_all(&self, selector: &str) -> Vec<ElementHandle>;

    /// Like `query_selector` but panics if no element matches.
    pub fn query(&self, selector: &str) -> ElementHandle;

    /// Return the focused element if it is inside this container.
    pub fn focused_element(&self) -> Option<ElementHandle>;
}
```

#### 2.4.1 ElementHandle

`ElementHandle` wraps a DOM element inside the test container and provides attribute access, text content, layout geometry, and focus queries.

```rust
/// Wrapper around a DOM element for test assertions.
pub struct ElementHandle {
    element: web_sys::Element,
}

impl ElementHandle {
    /// Read an attribute value.
    pub fn attr(&self, name: &str) -> Option<String>;
    /// Get the text content of the element.
    pub fn text_content(&self) -> String;
    /// Get the inner HTML of the element.
    pub fn inner_html(&self) -> String;
    /// Get the bounding client rect (position and size).
    pub fn bounding_rect(&self) -> Rect;
    /// Get computed styles as a map.
    pub fn computed_styles(&self) -> HashMap<String, String>;
    /// True if this element currently has focus.
    pub fn is_focused(&self) -> bool;
}
```

### 2.5 Part Attribute Shortcuts

Methods that query by `data-ars-part` or read specific part attributes -- the most common patterns across test specs.

```rust
impl TestHarness {
    /// Attribute from `[data-ars-part='trigger']`.
    pub fn trigger_attr(&self, attr: &str) -> Option<String>;

    /// Attribute from `[data-ars-part='input']`.
    pub fn input_attr(&self, attr: &str) -> Option<String>;

    /// Attribute from `[data-ars-part='control']`.
    pub fn control_attr(&self, attr: &str) -> Option<String>;

    /// `data-ars-*` attribute from the component root (`[data-ars-scope]`).
    pub fn data_attr(&self, name: &str) -> Option<String>;

    /// Attribute from an arbitrary selector.
    pub fn attr(&self, selector: &str, attr: &str) -> Option<String>;

    /// Attribute from `[data-ars-part='button']` (for Button-specific tests).
    pub fn button_attr(&self, attr: &str) -> Option<String>;
}
```

### 2.6 User Interaction Methods

All interaction methods are `async` -- they dispatch a DOM event, then call `backend.flush()` to process the resulting reactivity cycle before returning.

```rust
impl TestHarness {
    /// Click the component root (`[data-ars-scope]`).
    pub async fn click(&self);

    /// Click the element matching `selector`.
    pub async fn click_selector(&self, selector: &str);

    /// Type text into the focused input. Dispatches `input` events.
    pub async fn type_text(&self, text: &str);

    /// Hover (pointerenter + mouseover) on the element at `selector`.
    pub async fn hover(&self, selector: &str);

    /// Hover the trigger part element.
    pub async fn hover_trigger(&self);

    /// Blur the currently focused element.
    pub async fn blur(&self);

    /// Focus the element matching `selector`.
    pub async fn focus(&self, selector: &str);

    /// Set a value on value-bearing components (TextField, Slider, Progress).
    pub async fn set_value<V: Into<JsValue>>(&self, value: V);
}
```

### 2.7 Keyboard Input

```rust
impl TestHarness {
    /// Simulate a keydown on the focused element.
    pub async fn press_key(&self, key: KeyboardKey);

    /// Alias for `press_key`.
    pub async fn press(&self, key: KeyboardKey);

    /// Full key sequence: keydown, keypress, keyup.
    pub async fn key_sequence(&self, key: KeyboardKey);
}

pub enum KeyboardKey {
    Enter, Space, Escape, Tab,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Home, End, PageUp, PageDown,
    Backspace, Delete,
    Char(char),
}
```

### 2.8 Touch and Pointer Input

```rust
impl TestHarness {
    /// Start a touch at the given point.
    pub async fn touch_start(&self, point: Point);
    /// Move the active touch.
    pub async fn touch_move(&self, point: Point);
    /// End the active touch.
    pub async fn touch_end(&self);
    /// Start a touch on the trigger element's center.
    pub async fn touch_start_on_trigger(&self);
    /// Move the first touch in a multi-touch gesture.
    pub async fn touch_move_first(&self, point: Point);

    /// Dispatch pointerdown at (x, y) within the container.
    pub async fn pointer_down_at(&self, x: f64, y: f64);
    /// Dispatch pointermove to (x, y).
    pub async fn pointer_move_to(&self, x: f64, y: f64);
    /// Dispatch pointerup.
    pub async fn pointer_up(&self);
}

#[derive(Clone, Copy, Debug)]
pub struct Point { pub x: f64, pub y: f64 }

pub fn point(x: impl Into<f64>, y: impl Into<f64>) -> Point { /* ... */ }
```

### 2.9 IME/Composition Input

```rust
impl TestHarness {
    /// Begin an IME composition. Dispatches `compositionstart` + `compositionupdate`.
    pub async fn ime_compose(&self, text: &str);

    /// Commit the composition. Dispatches `compositionend` + `input`.
    pub async fn ime_commit(&self);
}
```

### 2.10 Focus Management

```rust
impl TestHarness {
    /// True if any element inside this container has focus.
    pub fn is_focused(&self) -> bool;

    /// Return focusable elements in Tab order.
    pub fn get_tab_order(&self) -> Vec<ElementHandle>;

    /// Return `[data-ars-part]` elements sorted by visual (bounding rect) position.
    pub fn get_visual_order(&self) -> Vec<ElementHandle>;
}
```

### 2.11 Time and Animation Control

```rust
impl TestHarness {
    /// Advance simulated time by `duration`. Fires pending timer callbacks
    /// whose delay has elapsed. Uses intercepted setTimeout/setInterval --
    /// no real wall-clock time passes.
    pub async fn advance_time(&self, duration: Duration);

    /// Dispatch `animationend` on the component root. Components using
    /// Presence (Dialog, Toast, Popover) wait for this before completing
    /// mount/unmount transitions.
    pub async fn fire_animation_end(&self);
}
```

### 2.12 Lifecycle Methods

```rust
impl TestHarness {
    /// True if the component root (`[data-ars-scope]`) is present in the DOM.
    pub fn is_mounted(&self) -> bool;

    /// Send a machine event directly to the underlying Service through erased dispatch.
    pub async fn send<E: Any>(&self, event: E);

    /// Current machine state as a debug string.
    pub fn state(&self) -> String;
}
```

### 2.13 State Inspection

```rust
impl TestHarness {
    /// True if the component is open/expanded (checks `data-ars-state` and `aria-expanded`).
    pub fn is_open(&self) -> bool;

    /// All `data-ars-*` attributes on the root as a sorted map (for insta snapshots).
    pub fn snapshot_attrs(&self) -> BTreeMap<String, String>;

    /// All `data-ars-part` values in document order (for anatomy snapshots).
    pub fn snapshot_parts(&self) -> Vec<String>;

    /// True if `<body>` has `overflow: hidden` (scroll-lock active).
    pub fn body_has_scroll_lock(&self) -> bool;

    /// Read a computed style from `<body>`.
    pub fn body_style(&self, property: &str) -> String;

    /// Set a style on `<body>` (for scroll-lock restoration tests).
    pub fn set_body_style(&self, property: &str, value: &str);

    /// Record values emitted during a closure (for testing intermediate states).
    pub fn record_values<F, T>(&self, f: F) -> Vec<T> where F: FnOnce() -> Vec<T>;
}
```

### 2.14 Convenience Methods

Additional utility methods used across multiple test spec files.

```rust
impl TestHarness {
    /// Emulate a CSS media feature for the duration of the test.
    /// Used for prefers-reduced-motion, prefers-color-scheme, etc.
    pub async fn emulate_media(&self, feature: &str, value: &str);

    /// Alias for `backend.flush()` -- process all pending reactivity updates.
    pub async fn tick(&self);

    /// Shorthand for `query_selector(&format!("[data-ars-part='{}']", part_name))`.
    pub fn query_part(&self, part_name: &str) -> Option<ElementHandle>;

    /// Explicit reactivity flush. Equivalent to `tick()`.
    pub async fn flush(&self);
}
```

---

## 3. Component Extension Pattern

The core `TestHarness` provides generic DOM queries and interactions. Component-specific convenience methods are added via extension traits.

### 3.1 Extension Trait Design

- Trait name: `{Component}HarnessExt`
- Defined in the component's test module or a dedicated `ars-test-ext-{component}` crate
- Implemented on `TestHarness`
- Methods delegate to core harness query/interaction methods

### 3.2 Naming Convention

| Component  | Extension Trait        | Key Methods                                                    |
| ---------- | ---------------------- | -------------------------------------------------------------- |
| Dialog     | `DialogHarnessExt`     | `open`, `close`, `open_dialog_with_id`, `close_dialog_with_id` |
| Select     | `SelectHarnessExt`     | `open`, `close`, `select_item`, `highlighted_item`             |
| Combobox   | `ComboboxHarnessExt`   | `open`, `type_text`, `highlighted_item`, `option_count`        |
| Tabs       | `TabsHarnessExt`       | `select_tab`, `tab`, `panel`, `tab_count`, `selected_index`    |
| Accordion  | `AccordionHarnessExt`  | `item`, `add_item`, `remove_item`, `item_count`                |
| Slider     | `SliderHarnessExt`     | `value`, `drag_thumb_to`, `drag_thumb`                         |
| Calendar   | `CalendarHarnessExt`   | `navigate_to_date`, `selected_date`                            |
| Menu       | `MenuHarnessExt`       | `open`, `open_menu`, `highlighted_item`                        |
| Toast      | `ToastHarnessExt`      | `add_toast`                                                    |
| Presence   | `PresenceHarnessExt`   | `set_present`                                                  |
| Checkbox   | `CheckboxHarnessExt`   | `is_checked`                                                   |
| Tooltip    | `TooltipHarnessExt`    | `tooltip_rect`                                                 |
| HoverCard  | `HoverCardHarnessExt`  | `card_rect`                                                    |
| Popover    | `PopoverHarnessExt`    | `popover_rect`, `set_anchor_position`                          |
| Listbox    | `ListboxHarnessExt`    | `dom_item_count`, `scroll_to_item`, `is_item_visible`          |
| FileUpload | `FileUploadHarnessExt` | `drop_files`, `select_files`                                   |

### 3.3 Example: DialogHarnessExt

```rust
pub trait DialogHarnessExt {
    async fn open(&self);
    async fn close(&self);
    async fn open_dialog_with_id(&self, id: &str);
    async fn close_dialog_with_id(&self, id: &str);
    async fn open_dialog(&self);
}

impl DialogHarnessExt for TestHarness {
    async fn open(&self) {
        self.click_selector("[data-ars-part='trigger']").await;
    }

    async fn close(&self) {
        self.press_key(KeyboardKey::Escape).await;
    }

    async fn open_dialog_with_id(&self, id: &str) {
        self.click_selector(&format!("#{id} [data-ars-part='trigger']")).await;
    }

    async fn close_dialog_with_id(&self, id: &str) {
        self.focus(&format!("#{id} [role='dialog']")).await;
        self.press_key(KeyboardKey::Escape).await;
    }

    async fn open_dialog(&self) { self.open().await; }
}
```

### 3.4 Example: SliderHarnessExt

```rust
pub trait SliderHarnessExt {
    /// Read the current slider value from `aria-valuenow`.
    fn value(&self) -> f64;
    /// Drag the thumb to a specific value position.
    async fn drag_thumb_to(&self, value: f64);
    /// Drag between two points with `steps` intermediate moves.
    async fn drag_thumb(&self, from: Point, to: Point, steps: u32);
}

impl SliderHarnessExt for TestHarness {
    fn value(&self) -> f64 {
        self.query("[role='slider']")
            .attr("aria-valuenow").expect("missing aria-valuenow")
            .parse().expect("aria-valuenow not a number")
    }

    async fn drag_thumb_to(&self, value: f64) {
        let thumb = self.query("[role='slider']");
        let rect = thumb.bounding_rect();
        let min: f64 = thumb.attr("aria-valuemin").expect("aria-valuemin must be present on slider thumb").parse().expect("aria-valuemin must be a valid number");
        let max: f64 = thumb.attr("aria-valuemax").expect("aria-valuemax must be present on slider thumb").parse().expect("aria-valuemax must be a valid number");
        let target_x = rect.x + ((value - min) / (max - min)) * rect.width;
        self.pointer_down_at(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0).await;
        self.pointer_move_to(target_x, rect.y + rect.height / 2.0).await;
        self.pointer_up().await;
    }

    async fn drag_thumb(&self, from: Point, to: Point, steps: u32) {
        self.pointer_down_at(from.x, from.y).await;
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            self.pointer_move_to(
                from.x + (to.x - from.x) * t,
                from.y + (to.y - from.y) * t,
            ).await;
        }
        self.pointer_up().await;
    }
}
```

### 3.5 ItemHandle for Collection Components

Components with repeated items (Accordion, Tabs, Listbox, Menu) return `ItemHandle` wrappers that scope queries to a specific item.

```rust
pub struct ItemHandle<'a> {
    harness: &'a TestHarness,
    element: ElementHandle,
}

impl<'a> ItemHandle<'a> {
    pub fn query_selector(&self, selector: &str) -> Option<ElementHandle>;
    pub fn trigger_attr(&self, attr: &str) -> Option<String>;
    pub fn trigger(&self) -> ElementHandle;
    pub async fn click_trigger(&self);
    pub fn text_content(&self) -> String;
    pub fn is_focused(&self) -> bool;
    pub async fn focus(&self);
    pub fn attr(&self, name: &str) -> Option<String>;
}

impl TestHarness {
    /// Get an item handle by index. Queries `[data-ars-part='item']` elements.
    pub fn item(&self, index: usize) -> ItemHandle<'_>;
}
```

---

## 4. Async Model

### 4.1 WASM Test Runtime

All adapter tests run as `#[wasm_bindgen_test]` inside a real browser (headless Chrome in CI).

```rust
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn checkbox_toggles() {
    // `render(...)` is imported from the active adapter harness crate.
    let harness = render(Checkbox::new(false)).await;
    assert_eq!(harness.control_attr("aria-checked"), Some("false".into()));
    harness.click().await;
    assert_eq!(harness.control_attr("aria-checked"), Some("true".into()));
}
```

### 4.2 Reactivity Flush

Every interaction method calls `backend.flush()` after dispatching DOM events. This ensures the framework's reactivity system has processed the event before the test reads DOM state. Equivalent to Leptos `tick().await` or the Dioxus backend's owned post-update browser task boundary. Test authors never call flush manually.

### 4.3 Timer Simulation

`advance_time(duration)` uses a `FakeTimer` that intercepts `setTimeout`/`setInterval`. No real wall-clock time passes -- callbacks fire deterministically. See [12-advanced.md](12-advanced.md) section 1.1 for the `FakeTimer` design.

```rust
#[wasm_bindgen_test]
async fn toast_auto_dismiss() {
    // `render(...)` is imported from the active adapter harness crate.
    let harness = render(Toast::new().auto_dismiss(Duration::from_secs(5))).await;
    harness.send(toast::Event::Show).await;
    assert!(harness.is_mounted());
    harness.advance_time(Duration::from_secs(5)).await;
    harness.fire_animation_end().await;
    assert!(!harness.is_mounted());
}
```

---

## 5. Framework Backend

### 5.1 HarnessBackend Trait

```rust
/// Abstracts framework-specific rendering and reactivity.
pub trait HarnessBackend: 'static {
    /// Mount a component into the container. Returns a type-erased service handle.
    fn mount(
        &self,
        container: &web_sys::HtmlElement,
        component: Box<dyn Any>,
    ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>>;

    /// Mount a component into the container with an explicit locale/provider wrapper.
    /// Backends own the framework-specific `ArsProvider` construction.
    fn mount_with_locale(
        &self,
        container: &web_sys::HtmlElement,
        component: Box<dyn Any>,
        locale: ars_i18n::Locale,
    ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>>;

    /// Flush the reactivity system. After this returns, all DOM updates are applied.
    fn flush(&self) -> Pin<Box<dyn Future<Output = ()>>>;

    /// Advance simulated time. Fires pending timer callbacks with delay <= `duration`.
    fn advance_time(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()>>>;
}
```

### 5.2 Leptos Backend

Uses `leptos::mount::mount_to` to render. `mount_with_locale()` wraps the
component in `<ArsProvider locale>` using Leptos `view!`. `flush()` calls
`leptos::tick().await`.

### 5.3 Dioxus Backend

Uses the public `dioxus-web` launch path with a web renderer rooted at the
isolated harness container. `mount_with_locale()` wraps the component in
`ArsProvider { locale, ... }` via `rsx!`. Because the public launch API consumes
the `VirtualDom`, `flush()` cannot await `wait_for_work()` directly; instead it
waits for a backend-owned browser task boundary plus trailing microtask turn.
Shared-harness DOM tests rely on that backend-owned flush behavior rather than
ad hoc timer shims in test code.

### 5.4 Non-Web Dioxus Backend

Adapter components that target Dioxus also compile for Desktop (`dioxus-desktop`),
mobile (`dioxus-mobile`), and SSR (`dioxus/server`). On those platforms the
`web` feature is disabled, the browser-only listener install path is gated
out via `cfg(feature = "web")`, and the component degrades to its
structural surface (rendered tree, returned handles, callback wiring)
without document/window listeners. The non-web Dioxus tier exists to
exercise that graceful-degrade path.

The entrypoint is `ars_test_harness_dioxus::desktop::DesktopHarness`. It
wraps a `dioxus_core::VirtualDom` directly — no `dioxus-web`, no real
WRY/webview window — because the cfg branch under test is identical
regardless of which renderer would normally drive the runtime. Real
Desktop launching adds GUI dependencies (xvfb, GTK, webkitgtk in CI),
serialised event loops, and window-lifecycle flake without any
additional coverage delta over a `VirtualDom`-only fixture.

API contract:

```rust
/// Headless `VirtualDom` wrapper for non-web Dioxus component tests.
pub struct DesktopHarness { /* ... */ }

impl DesktopHarness {
    /// Mounts a no-prop component fn item and runs the initial rebuild.
    #[must_use]
    pub fn launch(component: fn() -> Element) -> Self;

    /// Mounts a component with custom root props and runs the initial rebuild.
    /// `P: Clone + 'static` mirrors `VirtualDom::new_with_props`.
    #[must_use]
    pub fn launch_with_props<P, M>(
        component: impl ComponentFunction<P, M>,
        props: P,
    ) -> Self
    where
        P: Clone + 'static,
        M: 'static;

    /// Mounts a closure-rendered subtree wrapped in `ars_dioxus::ArsProvider`
    /// with the supplied `Locale`. Mirrors the wasm tier's
    /// `HarnessBackend::mount_with_locale` contract — when a non-web component
    /// test needs to exercise locale-sensitive output (for example the
    /// dismissable region's `dismiss_label`), this entrypoint installs the
    /// provider context before rebuilding so `use_locale` and `use_messages`
    /// resolve to the requested locale.
    #[must_use]
    pub fn launch_with_locale<F>(builder: F, locale: Locale) -> Self
    where
        F: Fn() -> Element + 'static;

    /// Drains pending Dioxus work — queued events, dirty scopes, and effects —
    /// until the runtime is idle. Mirrors the wasm-tier
    /// `HarnessBackend::flush` contract.
    ///
    /// `process_events` alone only converts the event queue into dirty marks —
    /// it does **not** re-render dirty scopes. To make sure signal writes
    /// triggered by callbacks under test are visible to subsequent assertions,
    /// the implementation loops `process_events` + `render_immediate` until
    /// `render_immediate_to_vec` reports zero edits (i.e. the runtime is
    /// quiescent), with a hard ceiling on iterations to surface re-render
    /// loops as a panic instead of a hang.
    pub fn flush(&mut self);
}
```

`DesktopHarness` deliberately does not implement `HarnessBackend`. The
backend trait's signatures take `&web_sys::HtmlElement` containers and
return `AnyService` futures wired into the wasm test runtime — both
contracts are tied to the browser. The non-web tier instead exposes a
small, synchronous surface that test authors drive directly: mount,
flush, drop. Test code expresses expectations through fixture-side
recorders (`Arc<Mutex<Vec<…>>>`, `Rc<RefCell<…>>`, `Arc<AtomicUsize>`)
captured by callbacks the fixture passes into the component's `Props`.

```rust
#[test]
fn region_mounts_on_desktop_without_panic() {
    let state = build_state();
    let _harness = DesktopHarness::launch_with_props(fixture, state.clone());

    let id = state.handle_slot
        .borrow()
        .as_ref()
        .expect("fixture must populate the handle slot during the initial rebuild")
        .overlay_id
        .peek()
        .clone();

    assert!(!id.is_empty());
}
```

Use this tier whenever a component spec calls for "validation on the
target runtime rather than only in a browser harness" — for example
[`spec/dioxus-components/utility/dismissable.md`](../dioxus-components/utility/dismissable.md)
§29-§31. Future overlay components that follow the same `cfg(feature =
"web")` graceful-degrade pattern (Dialog, Popover, Tooltip, …) reuse the
same harness.

---

## 6. Mock Infrastructure

### 6.1 File and Directory Mocks

Used by FileUpload and DropZone tests (see [12-advanced.md](12-advanced.md) section 1.3).

```rust
/// Create a mock `web_sys::File` for testing file upload/drop interactions.
pub fn mock_file(name: &str, content: &str, mime_type: &str) -> web_sys::File;

/// Create a mock `DataTransfer` with the given files for simulating drop events.
pub fn mock_data_transfer(files: &[web_sys::File]) -> web_sys::DataTransfer;
```

### 6.2 Clipboard Mocking

```rust
/// Install a mock clipboard that records `writeText` calls.
pub fn mock_clipboard() -> MockClipboard;

pub struct MockClipboard { /* ... */ }

impl MockClipboard {
    pub fn last_written_text(&self) -> Option<String>;
    pub fn set_read_text(&self, value: &str);
    pub fn deny_permission(&self);
}
```

### 6.3 Viewport and Layout Mocking

```rust
impl TestHarness {
    /// Set container dimensions for positioning tests.
    pub fn set_viewport(&self, width: f64, height: f64);

    /// Set anchor position for popover/tooltip placement tests.
    pub fn set_anchor_position(&self, rect: Rect);

    /// Scroll the container by a delta and flush reactive updates.
    pub async fn scroll_container_by(&self, dx: i32, dy: i32);

    /// Scroll the window to a position.
    pub fn scroll_to(&self, x: i32, y: i32);

    /// Read the current window scroll Y offset.
    pub fn scroll_y(&self) -> i32;
}

#[derive(Clone, Copy, Debug)]
pub struct Rect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }

impl Rect {
    pub fn right(&self) -> f64;
    pub fn bottom(&self) -> f64;
    pub fn left(&self) -> f64;
    pub fn top(&self) -> f64;
}
```

---

## 7. Snapshot Integration

The harness integrates with `insta` for snapshot testing (see [03-snapshot-tests.md](03-snapshot-tests.md)).

```rust
impl TestHarness {
    /// All `data-ars-*` attributes from the component root as a sorted map.
    pub fn snapshot_attrs(&self) -> BTreeMap<String, String>;

    /// All `data-ars-part` values in document order (anatomy verification).
    pub fn snapshot_parts(&self) -> Vec<String>;
}
```

Usage:

```rust
// `render(...)` is imported from the active adapter harness crate.
let harness = render(Dialog::new().open(true)).await;
assert_snapshot!("dialog_open", harness.snapshot_attrs());
```

---

## 8. Core Method Reference

### 8.1 Query Methods

| Method               | Signature                                  | Used In                |
| -------------------- | ------------------------------------------ | ---------------------- |
| `query_selector`     | `fn(&self, &str) -> Option<ElementHandle>` | 02, 06, 08, 10, 11, 12 |
| `query_selector_all` | `fn(&self, &str) -> Vec<ElementHandle>`    | 06, 08                 |
| `query`              | `fn(&self, &str) -> ElementHandle`         | 02, 06, 08             |
| `query_part`         | `fn(&self, &str) -> Option<ElementHandle>` | 02, 06                 |
| `focused_element`    | `fn(&self) -> Option<ElementHandle>`       | 02, 10                 |

### 8.2 Attribute Shortcuts

| Method         | Signature                                 | Used In    |
| -------------- | ----------------------------------------- | ---------- |
| `trigger_attr` | `fn(&self, &str) -> Option<String>`       | 06, 08, 11 |
| `input_attr`   | `fn(&self, &str) -> Option<String>`       | 06, 11     |
| `control_attr` | `fn(&self, &str) -> Option<String>`       | 06         |
| `data_attr`    | `fn(&self, &str) -> Option<String>`       | 10         |
| `attr`         | `fn(&self, &str, &str) -> Option<String>` | 02         |
| `button_attr`  | `fn(&self, &str) -> Option<String>`       | 02         |

### 8.3 Interaction Methods

| Method           | Signature               | Used In        |
| ---------------- | ----------------------- | -------------- |
| `click`          | `async fn(&self)`       | 06, 10, 12     |
| `click_selector` | `async fn(&self, &str)` | 02             |
| `type_text`      | `async fn(&self, &str)` | 02, 08, 10, 12 |
| `hover`          | `async fn(&self, &str)` | 08             |
| `hover_trigger`  | `async fn(&self)`       | 06, 08, 10, 12 |
| `blur`           | `async fn(&self)`       | 11             |
| `focus`          | `async fn(&self, &str)` | 06, 08         |
| `set_value`      | `async fn(&self, V)`    | 02, 11         |

### 8.4 Keyboard Methods

| Method         | Signature                      | Used In        |
| -------------- | ------------------------------ | -------------- |
| `press_key`    | `async fn(&self, KeyboardKey)` | 02, 08, 10, 12 |
| `press`        | `async fn(&self, KeyboardKey)` | 06, 08         |
| `key_sequence` | `async fn(&self, KeyboardKey)` | 10             |

### 8.5 Touch and Pointer Methods

| Method                   | Signature                   | Used In |
| ------------------------ | --------------------------- | ------- |
| `touch_start`            | `async fn(&self, Point)`    | 12      |
| `touch_move`             | `async fn(&self, Point)`    | 12      |
| `touch_end`              | `async fn(&self)`           | 12      |
| `touch_start_on_trigger` | `async fn(&self)`           | 12      |
| `touch_move_first`       | `async fn(&self, Point)`    | 12      |
| `pointer_down_at`        | `async fn(&self, f64, f64)` | 12      |
| `pointer_move_to`        | `async fn(&self, f64, f64)` | 12      |
| `pointer_up`             | `async fn(&self)`           | 12      |

### 8.6 IME Methods

| Method        | Signature               | Used In |
| ------------- | ----------------------- | ------- |
| `ime_compose` | `async fn(&self, &str)` | 08      |
| `ime_commit`  | `async fn(&self)`       | 08      |

### 8.7 Focus Management

| Method             | Signature                         | Used In |
| ------------------ | --------------------------------- | ------- |
| `is_focused`       | `fn(&self) -> bool`               | 10      |
| `get_tab_order`    | `fn(&self) -> Vec<ElementHandle>` | 08      |
| `get_visual_order` | `fn(&self) -> Vec<ElementHandle>` | 08      |

### 8.8 Time and Animation

| Method               | Signature                   | Used In    |
| -------------------- | --------------------------- | ---------- |
| `advance_time`       | `async fn(&self, Duration)` | 02, 10, 12 |
| `fire_animation_end` | `async fn(&self)`           | 10         |

### 8.9 Lifecycle and State

| Method                 | Signature                               | Used In |
| ---------------------- | --------------------------------------- | ------- |
| `is_mounted`           | `fn(&self) -> bool`                     | 10, 12  |
| `send`                 | `async fn(&self, E)`                    | 10      |
| `state`                | `fn(&self) -> String`                   | 10      |
| `is_open`              | `fn(&self) -> bool`                     | 02, 10  |
| `snapshot_attrs`       | `fn(&self) -> BTreeMap<String, String>` | 10      |
| `snapshot_parts`       | `fn(&self) -> Vec<String>`              | 03      |
| `body_has_scroll_lock` | `fn(&self) -> bool`                     | 02      |
| `body_style`           | `fn(&self, &str) -> String`             | 02      |
| `set_body_style`       | `fn(&self, &str, &str)`                 | 02      |
| `record_values`        | `fn(&self, F) -> Vec<T>`                | 10      |

### 8.10 Viewport and Layout

| Method                | Signature                   | Used In |
| --------------------- | --------------------------- | ------- |
| `set_viewport`        | `fn(&self, f64, f64)`       | 02      |
| `set_anchor_position` | `fn(&self, Rect)`           | 02      |
| `scroll_container_by` | `async fn(&self, i32, i32)` | 02      |
| `scroll_to`           | `fn(&self, i32, i32)`       | 02      |
| `scroll_y`            | `fn(&self) -> i32`          | 02      |

### 8.11 Convenience Methods

| Method          | Signature                     | Used In    |
| --------------- | ----------------------------- | ---------- |
| `emulate_media` | `async fn(&self, &str, &str)` | 06, 12     |
| `tick`          | `async fn(&self)`             | 02, 06, 11 |
| `flush`         | `async fn(&self)`             | 02         |

### 8.12 ElementHandle Methods

| Method            | Signature                              | Used In        |
| ----------------- | -------------------------------------- | -------------- |
| `attr`            | `fn(&self, &str) -> Option<String>`    | 02, 06, 08, 12 |
| `text_content`    | `fn(&self) -> String`                  | 02, 06         |
| `inner_html`      | `fn(&self) -> String`                  | 03             |
| `bounding_rect`   | `fn(&self) -> Rect`                    | 12             |
| `computed_styles` | `fn(&self) -> HashMap<String, String>` | 06             |
| `is_focused`      | `fn(&self) -> bool`                    | 02, 10         |
