# Adapter Test Harness

> **Two-tier testing model:** This file covers **adapter parity tests**, which use raw
> framework testing utilities (`mount_to_body`, `VirtualDom::new`, `query_selector`)
> to verify that both adapters produce identical output. These tests intentionally stay
> close to the framework's native patterns.
>
> **Component behavior tests** (specs 02, 06, 08, 10, 11, 12) use the `TestHarness`
> abstraction defined in `15-test-harness.md`, which provides higher-level methods
> (`harness.open()`, `harness.press_key(...)`) for readable component-level assertions.
> Both approaches coexist — parity tests stay close to the metal, behavior tests use
> the harness.
>
> When those behavior specs use `render(...)` or `mount_with_locale(...)`, the
> helpers come from the active adapter harness crate. The core
> `ars-test-harness` crate only provides `render_with_backend(...)` and
> `render_with_locale_and_backend(...)`.

## 1. Leptos

```rust
#[cfg(test)]
mod leptos_tests {
    use leptos::prelude::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    /// Helper: returns the global `Document` via `web_sys`.
    fn document() -> web_sys::Document {
        web_sys::window().expect("window must exist").document().expect("document must exist")
    }

    #[wasm_bindgen_test]
    fn button_renders_with_correct_attrs() {
        mount_to_body(|| {
            view! { <Button id="test-btn" variant="primary".to_string()>"Click"</Button> }
        });

        let btn = document()
            .query_selector("[data-ars-scope='button']")
            .expect("query must not error")
            .expect("element must exist");

        assert_eq!(btn.get_attribute("role").as_deref(), Some("button"));
        assert_eq!(btn.get_attribute("type").as_deref(), Some("button"));
        assert_eq!(btn.get_attribute("data-ars-variant").as_deref(), Some("primary"));
        assert_eq!(btn.get_attribute("data-ars-state").as_deref(), Some("idle"));
    }

    #[wasm_bindgen_test]
    async fn dialog_open_close_cycle() {
        let (open, set_open) = signal(false);
        mount_to_body(move || {
            view! {
                <Dialog open=open on_open_change=move |v| set_open.set(v)>
                    <DialogTrigger>"Open"</DialogTrigger>
                    <DialogContent>"Content"</DialogContent>
                </Dialog>
            }
        });

        // Click trigger to open.
        let trigger = document().query_selector("[data-ars-part='trigger']").expect("query must not error").expect("element must exist");
        trigger.dyn_ref::<web_sys::HtmlElement>().expect("element must be HtmlElement").click();

        // Reactivity flush per foundation 08 section 15. Import path: leptos::task::tick
        tick().await;
```

> **Leptos 0.8 reactivity flush:** Use `leptos::task::tick().await`. The adapter spec
> ([08-adapter-leptos.md](../foundation/08-adapter-leptos.md)) documents the canonical pattern.

```rust

        let content = document().query_selector("[data-ars-part='content']").expect("query must not error").expect("element must exist");
        assert_eq!(content.get_attribute("role").as_deref(), Some("dialog"));
        assert_eq!(content.get_attribute("aria-modal").as_deref(), Some("true"));
    }
}
```

## 2. Dioxus

> **Dioxus 0.7.3 API:** The examples below use Dioxus 0.7.3 patterns.
> SSR rendering uses `VirtualDom::new` + `rebuild_in_place()` + `dioxus::ssr::render()`.
> Interactive tests use `VirtualDom` with SSR assertions.
>
> **Dioxus 0.7.3 SSR:** The SSR module is `dioxus::ssr`.

```rust
#[cfg(test)]
mod dioxus_tests {
    use dioxus::prelude::*;

    #[test]
    fn toggle_ssr_renders_initial_state() {
        // Dioxus 0.7.3: build a VirtualDom, rebuild, then render to string
        let mut dom = VirtualDom::new(|| rsx! {
            Toggle { id: "t1", default_pressed: false }
        });
        dom.rebuild_in_place();
        // Path per foundation 09 section 15. See dioxus_ssr crate re-export.
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains(r#"aria-pressed="false""#));
    }
```

> **Known gap — Dioxus interactive parity:** Dioxus tests currently exercise the `Service` layer directly because `VirtualDom` does not expose DOM event simulation APIs. This means Dioxus parity tests do NOT verify DOM event handler wiring. When `dioxus-testing` infrastructure becomes available, full DOM event simulation tests (`wasm_bindgen_test` with real mount + click + query) MUST be added to achieve true parity with Leptos interactive tests. Until then, interactive parity is verified only at the state machine level, not at the DOM integration level.
> See [09-adapter-dioxus.md](../foundation/09-adapter-dioxus.md) §10 for `use_safe_event_listener` — a cleanup pattern for DOM listeners in the adapter, not an event simulation API. Dioxus does not provide built-in event simulation or testing utilities (verified docs.rs/dioxus 0.7.x). The interactive testing parity gap remains open until a `dioxus-testing` crate or equivalent emerges.

```rust
    #[test]
    fn toggle_on_off_via_machine() {
        // Dioxus 0.7.3: interactive testing uses the Service layer directly
        // since VirtualDom does not expose click/event simulation APIs.
        let props = toggle::Props { id: "t1".into(), default_pressed: false, ..Default::default() };
        let mut svc = Service::<toggle::Machine>::new(props);
        assert_eq!(*svc.state(), toggle::State::Off);

        svc.send(toggle::Event::Toggle);
        assert_eq!(*svc.state(), toggle::State::On);

        // Verify SSR output after state change
        let mut dom = VirtualDom::new(|| rsx! {
            Toggle { id: "t1", default_pressed: true }
        });
        dom.rebuild_in_place();
        // Path per foundation 09 section 15. See dioxus_ssr crate re-export.
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains(r#"aria-pressed="true""#));
    }
}
```

## 3. What to test per adapter

- **Mount**: component renders with correct initial attributes.
- **Interact**: click/keyboard events trigger correct state transitions.
- **Assert DOM**: attributes, text content, and structure match the machine's `connect()` output.
- **Cleanup**: effects are cleaned up on unmount (no memory leaks, no dangling timers).

---

## 4. Adapter Test Parity

Every component MUST have equivalent test coverage across all supported adapters (Leptos, Dioxus).

### 4.1 Per-Adapter Component Test Checklist

For each component, both adapters must cover:

- [ ] Mount/unmount lifecycle
- [ ] Controlled and uncontrolled modes
- [ ] Keyboard navigation
- [ ] ARIA attribute output (snapshot)
- [ ] State machine transitions (all events)
- [ ] Focus management
- [ ] Form participation (hidden input)

```rust
/// SSR rendering helpers for parity tests.
/// Leptos 0.8 does not have a `leptos::ssr` module. Instead, `RenderHtml::to_html()`
/// from `leptos::tachys::view` renders any view to an HTML string synchronously.
fn leptos_render<F, V>(f: F) -> String
where
    F: FnOnce() -> V + 'static,
    V: IntoView + RenderHtml,
{
    use leptos::tachys::view::RenderHtml;
    let owner = Owner::new();
    owner.with(|| f().to_html())
}

fn dioxus_render(f: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(f);
    dom.rebuild_in_place();
    // Path per foundation 09 section 15. See dioxus_ssr crate re-export.
    dioxus::ssr::render(&dom)
}

/// Form participation parity: Checkbox hidden input
fn checkbox_form_participation_parity() {
    let leptos_html = leptos_render(|| Checkbox::new(checkbox::Props {
        name: "agree".into(),
        checked: true,
        ..Default::default()
    }));
    let dioxus_html = dioxus_render(|| Checkbox::new(checkbox::Props {
        name: "agree".into(),
        checked: true,
        ..Default::default()
    }));
    // Both must render a hidden input for form participation
    assert!(leptos_html.contains(r#"<input type="hidden" name="agree" value="on""#),
        "Leptos Checkbox must render hidden input");
    assert!(dioxus_html.contains(r#"<input type="hidden" name="agree" value="on""#),
        "Dioxus Checkbox must render hidden input");
}
```

### 4.2 Shared Test Matrix Format

> **Canonical definition:** This `InteractionTestCase` type is for DOM-level interaction parity testing (click, press, focus). For machine-level event parity, see `ParityTestCase` in section 6.1.

```rust
// tests/shared/select_scenarios.rs
pub fn select_scenarios() -> Vec<InteractionTestCase> {
    vec![
        InteractionTestCase {
            name: "open_with_click",
            component: ComponentType::Select,
            steps: vec![TestStep::Click("[data-ars-part='trigger']"), TestStep::AssertOpen],
            expected_attrs: HashMap::from([("aria-expanded", "true")]),
        },
        InteractionTestCase {
            name: "close_with_escape",
            component: ComponentType::Select,
            steps: vec![
                TestStep::Click("[data-ars-part='trigger']"),
                TestStep::Press(KeyboardKey::Escape),
                TestStep::AssertClosed,
            ],
            expected_attrs: HashMap::from([("aria-expanded", "false")]),
        },
        // ...
    ]
}
```

Each adapter crate imports the shared scenarios and runs them through its `render_and_collect_attrs` / `run_events_and_get_state` functions (see Adapter Parity Tests below). This ensures both adapters exercise the exact same behavioral matrix.

---

## 5. Adapter Parity

Every headless test MUST pass identically on both the Leptos and Dioxus adapters. Adapter-specific tests are ONLY for integration/rendering concerns (e.g., hydration, framework-specific lifecycle).

### 5.1 Requirement

- All unit tests (`transition()`, `connect()`, state machine logic) are framework-agnostic by design.
- Integration tests that mount components must run on **both** adapters and produce identical results.
- Adapter-only tests cover rendering integration: mount/unmount lifecycle, reactive signal propagation, SSR output.

### 5.2 Test Harness Pattern

Parity tests are driven by shared `ParityTestCase` structs (see below). Each adapter
crate provides a `render_and_collect_attrs` / `run_events_and_get_state` function
that accepts a `ParityTestCase` and returns adapter-independent output. There is no
abstract `TestAdapter` trait — both adapters consume the same `ParityTestCase` data
and their outputs are compared directly.

```rust
// Usage: define shared test cases, run through both adapter harnesses

fn select_opens_on_click_cases() -> Vec<ParityTestCase> {
    vec![ParityTestCase {
        name: "Select opens on click",
        component: ComponentType::Select,
        props: Props::from(select::Props::default()),
        events: vec![Event::from(select::Event::Open)],
        expected_attrs: HashMap::from([
            ("aria-expanded", "true"),
        ]),
    }]
}

#[test]
fn parity_select_opens_on_click() {
    for case in &select_opens_on_click_cases() {
        let leptos_attrs = ars_leptos::test_harness::render_and_collect_attrs(case);
        let dioxus_attrs = ars_dioxus::test_harness::render_and_collect_attrs(case);
        assert_eq!(leptos_attrs, dioxus_attrs,
            "Parity mismatch for '{}': leptos={:?}, dioxus={:?}",
            case.name, leptos_attrs, dioxus_attrs);
    }
}
```

---

## 6. Adapter Parity Tests

Each component must have a cross-adapter parity test that verifies:

1. **Identical ARIA attribute output** between Leptos and Dioxus adapters for the same machine state.
2. **Identical event-to-state-change mapping**.
3. **Identical keyboard interaction sequences** produce identical state transitions.

Parity tests use a shared test definition (machine state + expected ARIA attributes) with adapter-specific test harnesses.

### 6.1 `ParityTestCase` Framework

All parity tests are driven by a shared `ParityTestCase` struct that captures the
component configuration, user interactions, and expected output. Both the Leptos and
Dioxus adapter test harnesses consume the same `ParityTestCase` instances, ensuring
that any divergence is detected automatically.

```rust
/// `ParityTestCase` checks static ARIA attribute output at a point in time.
/// This differs from `InteractionTestCase` (§4.2), which tests interactive DOM
/// event sequences and multi-step state transitions.

/// A single cross-adapter parity test definition.
/// Both adapter test runners consume the same Vec<ParityTestCase>.
#[derive(Clone, Debug)]
pub struct ParityTestCase {
    /// Human-readable test name for diagnostics.
    pub name: &'static str,
    /// The component under test.
    pub component: ComponentType,
    /// Props to initialize the component with.
    pub props: Props,
    /// Sequence of events to send before checking output.
    pub events: Vec<Event>,
    /// Expected ARIA and HTML attributes on the root element after all events.
    pub expected_attrs: HashMap<&'static str, &'static str>,
}

/// Adapter-agnostic component type tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentType {
    Checkbox,
    RadioGroup,
    Select,
    Dialog,
    SearchInput,
    Tabs,
    // ... one variant per component
}
```

### 6.2 ARIA Attribute Parity

The same component with the same props must produce identical ARIA attributes in
both Leptos and Dioxus adapters. This is the primary parity guarantee.

```rust
#[test]
fn aria_parity_search_input() {
    let case = ParityTestCase {
        name: "SearchInput focused with value",
        component: ComponentType::SearchInput,
        props: Props::from(search_input::Props {
            id: "search-1".into(),
            value: Some("hello".into()),
            ..Default::default()
        }),
        events: vec![Event::from(search_input::Event::Focus { is_keyboard: true })],
        expected_attrs: HashMap::from([
            ("role", "search"),
            ("data-ars-scope", "search-input"),
            ("data-ars-focus-visible", ""),
        ]),
    };

    let leptos_attrs = ars_leptos::test_harness::render_and_collect_attrs(&case);
    let dioxus_attrs = ars_dioxus::test_harness::render_and_collect_attrs(&case);

    assert_eq!(
        leptos_attrs, dioxus_attrs,
        "ARIA attribute mismatch for '{}': leptos={:?}, dioxus={:?}",
        case.name, leptos_attrs, dioxus_attrs,
    );

    // Also verify against the expected attrs from the test case
    for (key, expected) in &case.expected_attrs {
        assert_eq!(
            leptos_attrs.get(*key).map(|v| v.as_str()),
            Some(*expected),
            "expected attr '{}' = '{}' for '{}'",
            key, expected, case.name,
        );
    }
}
```

### 6.2.1 FilterMode::Custom Cfg-Gate Compilation Test

```rust
/// Verify FilterMode::Custom compiles and works with shared `Arc` ownership.
#[test]
fn filter_mode_custom_compiles_on_both_targets() {
    let filter = FilterMode::Custom(Arc::new(|input: &str, label: &str| {
        label.to_lowercase().contains(&input.to_lowercase())
    }));

    match &filter {
        FilterMode::Custom(f) => assert!(f("app", "Apple"),
            "Custom filter should match case-insensitively"),
        _ => panic!("expected Custom variant"),
    }
}
```

### 6.3 Event Mapping Parity

The same user interaction must produce the same state machine events in both
adapters. This verifies that DOM event handlers in each adapter translate browser
events to machine events identically.

```rust
#[test]
fn event_mapping_parity_checkbox() {
    let cases = vec![
        ParityTestCase {
            name: "Checkbox toggle via click",
            component: ComponentType::Checkbox,
            props: Props::from(checkbox::Props::default()),
            events: vec![Event::from(checkbox::Event::Toggle)],
            expected_attrs: HashMap::from([
                ("role", "checkbox"),
                ("aria-checked", "true"),
            ]),
        },
        ParityTestCase {
            name: "Checkbox toggle via Space key",
            component: ComponentType::Checkbox,
            props: Props::from(checkbox::Props::default()),
            events: vec![Event::from(checkbox::Event::Toggle)],
            expected_attrs: HashMap::from([
                ("role", "checkbox"),
                ("aria-checked", "true"),
            ]),
        },
    ];

    for case in &cases {
        let leptos_state = ars_leptos::test_harness::run_events_and_get_state(case);
        let dioxus_state = ars_dioxus::test_harness::run_events_and_get_state(case);
        assert_eq!(
            leptos_state, dioxus_state,
            "state divergence for '{}': leptos={:?}, dioxus={:?}",
            case.name, leptos_state, dioxus_state,
        );
    }
}
```

### 6.4 DOM Structure Parity

Both adapters must render the same anatomy parts in the same order. This test
extracts the `data-ars-part` attributes from the rendered DOM and compares the
sequences.

Each adapter's `collect_anatomy_parts` queries all elements with `[data-ars-part]`
in document order and returns their part names:

```rust
/// Queries all elements with `[data-ars-part]` in document order
/// and returns their part names.
fn collect_anatomy_parts(root: &Element) -> Vec<String> {
    root.query_selector_all("[data-ars-part]")
        .expect("query must succeed")
        .iter()
        .map(|el| el.get_attribute("data-ars-part").expect("data-ars-part must exist on selector-matched element"))
        .collect()
}
```

```rust
#[test]
fn dom_structure_parity_dialog() {
    let case = ParityTestCase {
        name: "Dialog open",
        component: ComponentType::Dialog,
        props: Props::from(dialog::Props {
            open: Some(true),
            ..Default::default()
        }),
        events: vec![],
        expected_attrs: HashMap::from([
            ("role", "dialog"),
            ("aria-modal", "true"),
        ]),
    };

    let leptos_parts = ars_leptos::test_harness::collect_anatomy_parts(&case);
    let dioxus_parts = ars_dioxus::test_harness::collect_anatomy_parts(&case);

    assert_eq!(
        leptos_parts, dioxus_parts,
        "anatomy part order mismatch for '{}': leptos={:?}, dioxus={:?}",
        case.name, leptos_parts, dioxus_parts,
    );
}
```

### 6.5 CI Enforcement

The parity test suite MUST run on every PR. Any divergence between adapters fails
the build. CI configuration:

- Test count per component per adapter must be tracked.
- CI fails if any adapter has fewer tests than the reference adapter for a given component.
- Shared test matrix: `tests/shared/` defines scenario descriptions; adapter-specific test files implement them.

- **Job name**: `adapter-parity`
- **Trigger**: Every pull request and push to `main`.
- **Scope**: All components with both Leptos and Dioxus adapter implementations.
- **Failure mode**: Any `assert_eq!` failure between adapter outputs fails the job.
- **Coverage requirement**: Every component must have at least one `ParityTestCase`
  covering default props, one interactive state, and one disabled/readonly state.

```yaml
# .github/workflows/ci.yml (excerpt)
adapter-parity:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test --package ars-test-parity --lib -- --test-threads=1
```

---

## 7. DOM Utility Parity Tests

Components that rely on `ars-dom` utilities (positioning, scroll locking, z-index allocation) from [11-dom-utilities.md](../foundation/11-dom-utilities.md) may exhibit behavioral differences between Leptos and Dioxus adapters at the DOM level. These parity tests verify that both adapters produce equivalent DOM-level outcomes.

### 7.1 Positioning Parity

> **CI note:** Positioning parity cannot be verified in a single binary because Leptos
> and Dioxus adapters are mutually exclusive feature flags. CI runs this test twice --
> once with `--features leptos` and once with `--features dioxus` -- and a post-step
> compares the serialized positioning output from both runs.

```rust
/// Verify that an overlay component produces correct positioning output.
/// This test is compiled once per adapter via feature flags. CI runs it under
/// both `--features leptos` and `--features dioxus` and compares results.
#[wasm_bindgen_test]
async fn positioning_output_popover() {
    let harness = render(popover::Machine::new(popover::Props {
        placement: Placement::Start,
        ..Default::default()
    })).await;

    harness.open().await;

    let content = harness.query("[data-ars-part='content']")
        .expect("content part");
    let pos = content.bounding_rect();

    // Serialized to a file for cross-adapter comparison by CI post-step
    let output = serde_json::json!({
        "component": "popover",
        "placement": "start",
        "top": pos.top,
        "left": pos.left,
        "width": pos.width,
        "height": pos.height,
    });
    std::fs::write(
        format!("target/parity/positioning-popover-{}.json", env!("ARS_ADAPTER")),
        serde_json::to_string_pretty(&output).expect("JSON serialization must succeed"),
    ).expect("must write parity output");
}
```

### 7.2 Scroll Lock Parity

```rust
/// Verify that modal overlays (Dialog, Drawer) activate/deactivate scroll
/// lock consistently in both adapters.
fn scroll_lock_parity_test() {
    // 1. Mount Dialog in both adapters
    // 2. Open dialog
    // 3. Assert document.body overflow is 'hidden' in both
    // 4. Close dialog
    // 5. Assert document.body overflow is restored in both
}
```

### 7.3 Z-Index Allocation Parity

```rust
/// Verify that stacked overlays receive z-index values in the same order
/// from both adapters.
fn z_index_stacking_parity_test() {
    // 1. Open Dialog A in both adapters
    // 2. Open nested Popover B inside Dialog A
    // 3. Read z-index of A and B in both adapters
    // 4. Assert B.z_index > A.z_index in both
    // 5. Assert z-index values match between adapters
}
```

### 7.4 RTL Positioning Parity

`ars-dom` depends on `ars-i18n` for RTL-aware positioning (foundation 11). `Placement::resolve_logical` converts logical placements (`Start`, `End`) to physical (`Left`, `Right`) based on text direction.

```rust
#[wasm_bindgen_test]
async fn placement_start_resolves_to_left_in_ltr() {
    let harness = mount_with_locale(
        popover::Machine::new(popover::Props { placement: Placement::Start, ..Default::default() }),
        locale!("en"),
    ).await;
    harness.open().await;
    let pos = harness.query("[data-ars-part='content']").expect("popover content").bounding_rect();
    let trigger = harness.query("[data-ars-part='trigger']").expect("popover trigger").bounding_rect();
    assert!(pos.right <= trigger.left, "Start placement in LTR should position content to the left of trigger");
}

#[wasm_bindgen_test]
async fn placement_start_resolves_to_right_in_rtl() {
    let harness = mount_with_locale(
        popover::Machine::new(popover::Props { placement: Placement::Start, ..Default::default() }),
        locale!("ar"),
    ).await;
    harness.open().await;
    let pos = harness.query("[data-ars-part='content']").expect("popover content").bounding_rect();
    let trigger = harness.query("[data-ars-part='trigger']").expect("popover trigger").bounding_rect();
    assert!(pos.left >= trigger.right, "Start placement in RTL should position content to the right of trigger");
}
```
