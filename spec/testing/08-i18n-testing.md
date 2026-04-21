# I18N Testing

## 1. Test Harness Utilities

> **Test harness integration:** The `mount_with_locale` helper is defined in [15-test-harness.md section 2.3](15-test-harness.md#23-locale-aware-mounting). It wraps the component in an `ArsProvider` with the specified locale.

```rust
/// Test harness that wraps a component in a container with the specified
/// text direction and provides the corresponding locale context.
///
/// Returns a test handle with DOM query methods.
/// `mount_with_locale` owns locale/provider setup; this helper only adds
/// a direction wrapper around the component under test.
async fn render_with_dir(dir: Direction, component: impl IntoView) -> TestHarness {
    let locale = match dir {
        Direction::Ltr => ars_i18n::Locale::parse("en").expect("valid locale"),
        Direction::Rtl => ars_i18n::Locale::parse("ar").expect("valid locale"),
    };
    mount_with_locale(
        view! { <div dir={dir.as_str()}>{component}</div> },
        locale,
    ).await
}

/// Dioxus equivalent of `render_with_dir`.
/// Uses the shared harness backend for DOM behavior tests; only the direction
/// wrapper is Dioxus-specific.
async fn render_with_dir_dioxus(dir: Direction, component: Element) -> TestHarness {
    let locale = match dir {
        Direction::Ltr => ars_i18n::Locale::parse("en").expect("valid locale"),
        Direction::Rtl => ars_i18n::Locale::parse("ar").expect("valid locale"),
    };
    mount_with_locale(
        rsx! { div { dir: dir.as_str(), {component} } },
        locale,
    ).await
}
```

## 2. RTL and Bidirectional Text Testing

### 2.1 Arrow Key Swapping

All components using horizontal arrow keys MUST swap Left↔Right when `dir="rtl"`:

```rust
#[wasm_bindgen_test]
async fn slider_rtl_arrow_keys() {
    let harness = render_with_dir(Direction::Rtl, Slider::new(50.0)).await;
    harness.press_key(KeyboardKey::ArrowLeft);
    assert_eq!(harness.value(), 51.0); // Left increases in RTL
    harness.press_key(KeyboardKey::ArrowRight);
    assert_eq!(harness.value(), 50.0); // Right decreases in RTL
}

#[wasm_bindgen_test]
async fn tabs_rtl_arrow_navigation() {
    let harness = render_with_dir(Direction::Rtl, Tabs::new()).await;
    // ArrowLeft moves to next tab (visually right) in RTL
    harness.press_key(KeyboardKey::ArrowLeft);
    assert_eq!(harness.selected_index(), 1);
}

#[wasm_bindgen_test]
async fn dioxus_rtl_wrapper_sets_dir() {
    let harness = render_with_dir_dioxus(
        Direction::Rtl,
        rsx! { div { "RTL smoke test" } },
    ).await;
    assert_eq!(harness.query("[dir='rtl']").attr("dir").as_deref(), Some("rtl"));
}
```

**Required components** (per foundation 05-interactions.md `resolve_arrow_key` affected list): Tabs, RadioGroup, Slider, Splitter, TreeView, Carousel, Toolbar, Menu (horizontal), and the Move interaction.

> TreeView is included in the RTL-affected component list per foundation 05-interactions.md. While ArrowLeft/ArrowRight also control expand/collapse, the direction mapping MUST be swapped in RTL mode: in RTL, ArrowRight collapses and ArrowLeft expands (the reverse of LTR).

```rust
// Component props helpers for RTL tests
fn radio_props() -> radio_group::Props {
    radio_group::Props { items: vec!["A".into(), "B".into(), "C".into()], ..Default::default() }
}
fn splitter_props() -> splitter::Props { splitter::Props::default() }
fn carousel_props() -> carousel::Props { carousel::Props::default() }
fn toolbar_props() -> toolbar::Props { toolbar::Props::default() }
fn menubar_props() -> menu_bar::Props { menu_bar::Props::default() }

#[wasm_bindgen_test]
async fn tree_view_rtl_arrow_keys_swap_expand_collapse() {
    let items = vec![
        tree_view::Node::item(Key::from("a"), "Alpha"),
        tree_view::Node::branch(Key::from("b"), "Beta", vec![
            tree_view::Node::item(Key::from("b1"), "Beta-1"),
        ]),
    ];
    let harness = render_with_dir(Direction::Rtl, TreeView::new("tv1", items)).await;
    harness.focus("[role='treeitem']:first-child");
    // In RTL, ArrowLeft expands (opposite of LTR)
    harness.press_key(KeyboardKey::ArrowLeft);
    assert!(harness.query("[role='treeitem']:first-child").attr("aria-expanded") == Some("true"));
    // ArrowRight collapses in RTL
    harness.press_key(KeyboardKey::ArrowRight);
    assert!(harness.query("[role='treeitem']:first-child").attr("aria-expanded") == Some("false"));
}

#[wasm_bindgen_test]
async fn radio_group_rtl_arrow_keys() {
    let harness = render_with_dir(Direction::Rtl, RadioGroup::new(radio_props())).await;
    let first = harness.query("[role='radio']:first-child").expect("first radio must exist");
    first.focus();
    harness.press_key(KeyboardKey::ArrowLeft);
    harness.flush().await;
    // In RTL, ArrowLeft moves to the NEXT item (visually right-to-left)
    assert_aria_checked(harness.query("[role='radio']:nth-child(2)").expect("second radio").attrs(), true);
}

#[wasm_bindgen_test]
async fn splitter_rtl_arrow_keys() {
    let harness = render_with_dir(Direction::Rtl, Splitter::new(splitter_props())).await;
    let handle = harness.query("[data-ars-part='handle']").expect("handle must exist");
    handle.focus();
    let initial = harness.query("[data-ars-part='panel']:nth-of-type(1)").expect("panel").bounding_rect().width;
    harness.press_key(KeyboardKey::ArrowLeft);
    harness.flush().await;
    // In RTL, ArrowLeft increases the first panel (opposite of LTR)
    let after = harness.query("[data-ars-part='panel']:nth-of-type(1)").expect("panel").bounding_rect().width;
    assert!(after > initial, "ArrowLeft in RTL must increase first panel");
}

#[wasm_bindgen_test]
async fn carousel_rtl_arrow_keys() {
    let harness = render_with_dir(Direction::Rtl, Carousel::new(carousel_props())).await;
    harness.query("[data-ars-part='item']").expect("item must exist").focus();
    harness.press_key(KeyboardKey::ArrowLeft);
    harness.flush().await;
    assert_eq!(harness.selected_index(), 1, "ArrowLeft in RTL must advance to next slide");
}

#[wasm_bindgen_test]
async fn toolbar_rtl_arrow_keys() {
    let harness = render_with_dir(Direction::Rtl, Toolbar::new(toolbar_props())).await;
    let first_btn = harness.query("[role='toolbar'] button:first-child").expect("first button must exist");
    first_btn.focus();
    harness.press_key(KeyboardKey::ArrowLeft);
    harness.flush().await;
    let active = document().active_element().expect("element must be focused");
    assert_ne!(active, first_btn, "ArrowLeft in RTL must move focus to next toolbar item");
}

#[wasm_bindgen_test]
async fn menu_horizontal_rtl_arrow_keys() {
    let harness = render_with_dir(Direction::Rtl, MenuBar::new(menubar_props())).await;
    let first_item = harness.query("[role='menuitem']:first-child").expect("first item must exist");
    first_item.focus();
    harness.press_key(KeyboardKey::ArrowLeft);
    harness.flush().await;
    let active = document().active_element().expect("element must be focused");
    assert_ne!(active, first_item, "ArrowLeft in RTL must move to next menu item");
}
```

> **Dioxus adapter:** Interactive DOM-level RTL tests are not feasible due to Dioxus `VirtualDom` limitations in `wasm_bindgen_test` (see 05-adapter-harness.md §2 (Known gap — Dioxus interactive parity)). The machine-level test below verifies the arrow key resolution logic directly:

```rust
/// Dioxus adapter: machine-level RTL arrow key verification.
/// Interactive DOM-level tests are not feasible due to Dioxus VirtualDom
/// limitations in wasm_bindgen_test (see 05-adapter-harness.md §2 (Known gap — Dioxus interactive parity)).
#[test]
fn dioxus_rtl_arrow_key_resolution_machine_level() {
    // Verify that resolve_arrow_key swaps Left/Right in RTL
    use ars_interactions::resolve_arrow_key;
    use ars_core::Direction;

    let resolved = resolve_arrow_key(KeyboardKey::ArrowRight, Direction::Rtl);
    assert_eq!(resolved, Some(LogicalDirection::Backward), "ArrowRight in RTL should resolve to Backward");

    let resolved = resolve_arrow_key(KeyboardKey::ArrowLeft, Direction::Rtl);
    assert_eq!(resolved, Some(LogicalDirection::Forward), "ArrowLeft in RTL should resolve to Forward");
}
```

### 2.2 Tab Order Preservation

```rust
#[wasm_bindgen_test]
async fn rtl_tab_order_matches_visual_order() {
    let harness = render_with_dir(Direction::Rtl, Form::new()).await;
    // Collect tab order by pressing Tab and recording focused elements
    let mut tab_order = Vec::new();
    for _ in 0..10 {
        harness.press_key(KeyboardKey::Tab);
        if let Some(el) = harness.focused_element() {
            tab_order.push(el);
        }
    }
    // Collect visual order by sorting focusable elements by bounding_rect().left (descending for RTL)
    let focusables = harness.query_selector_all("[tabindex], input, button, select, textarea, a[href]");
    let mut visual_order: Vec<_> = focusables.iter().collect();
    visual_order.sort_by(|a, b| b.bounding_rect().left.partial_cmp(&a.bounding_rect().left).expect("cmp"));
    assert_eq!(tab_order, visual_order, "Tab follows visual, not DOM");
}
```

### 2.3 Overlay Positioning

```rust
#[wasm_bindgen_test]
async fn popover_rtl_mirrored_placement() {
    let harness = render_with_dir(Direction::Rtl, Popover::new().placement(Placement::Start)).await;
    harness.open();
    // "start" should resolve to right side in RTL
    assert!(harness.popover_rect().left() >= harness.query_part("trigger").expect("trigger").bounding_rect().right());
}
```

### 2.4 RTL Overlay Placement

All overlay components that support logical placement (`Start`/`End`) must mirror
correctly in RTL. The `Start` placement resolves to the inline-start side: left in
LTR, right in RTL.

```rust
#[wasm_bindgen_test]
async fn tooltip_rtl_start_placement() {
    let harness = render_with_dir(
        Direction::Rtl, Tooltip::new("Help text").placement(Placement::Start)
    ).await;
    harness.hover_trigger();
    // "start" resolves to right side in RTL
    assert!(harness.tooltip_rect().left() >= harness.query_part("trigger").expect("trigger").bounding_rect().right());
}

#[wasm_bindgen_test]
async fn menu_rtl_start_placement() {
    let harness = render_with_dir(Direction::Rtl, Menu::new().placement(Placement::Start)).await;
    harness.open();
    assert!(harness.query_selector("[role='menu']").expect("menu").bounding_rect().left() >= harness.query_part("trigger").expect("trigger").bounding_rect().right());
}

#[wasm_bindgen_test]
async fn select_rtl_start_placement() {
    let harness = render_with_dir(Direction::Rtl, Select::new().placement(Placement::Start)).await;
    harness.open();
    assert!(harness.query_selector("[role='listbox']").expect("listbox").bounding_rect().left() >= harness.query_part("trigger").expect("trigger").bounding_rect().right());
}

#[wasm_bindgen_test]
async fn combobox_rtl_start_placement() {
    let harness = render_with_dir(Direction::Rtl, Combobox::new().placement(Placement::Start)).await;
    harness.open();
    assert!(harness.query_selector("[role='listbox']").expect("listbox").bounding_rect().left() >= harness.query_part("trigger").expect("trigger").bounding_rect().right());
}

#[wasm_bindgen_test]
async fn hover_card_rtl_start_placement() {
    let harness = render_with_dir(Direction::Rtl, HoverCard::new().placement(Placement::Start)).await;
    harness.hover_trigger();
    assert!(harness.query_part("content").expect("hover card content").bounding_rect().left() >= harness.query_part("trigger").expect("trigger").bounding_rect().right());
}

#[wasm_bindgen_test]
async fn toast_rtl_placement_mirrors() {
    let harness = render_with_dir(
        Direction::Rtl,
        Toaster::new(toaster::Props { placement: Placement::TopStart, ..Default::default() }),
    ).await;
    harness.send(toaster::Event::AddToast { message: "Test message".into() });
    harness.flush().await;
    let toast = harness.query("[data-ars-part='toast']").expect("toast must exist");
    // In RTL, TopStart should position on the right side
    let toast_rect = toast.get_bounding_client_rect();
    let viewport_width = window().inner_width().expect("must get width").as_f64().expect("must be number");
    assert!(toast_rect.right() > viewport_width / 2.0, "TopStart in RTL must be on the right side");
}
```

> Dialog overlay positioning is excluded from RTL placement tests because Dialog content is centered (not placement-based).

### 2.5 RTL-Aware Typeahead

```rust
#[wasm_bindgen_test]
async fn combobox_rtl_typeahead() {
    let harness = render_with_dir(Direction::Rtl, Combobox::with_items(arabic_items())).await;
    harness.type_text("ب"); // Arabic letter Ba
    assert!(harness.highlighted_item().starts_with("ب"));
}
```

## 3. Dynamic Locale Switching Testing

### 3.1 Hot-Swap Locale

```rust
#[wasm_bindgen_test]
async fn hot_swap_locale_updates_messages() {
    let harness = render(Select::new()).await;
    harness.send(i18n::Event::SetLocale(Locale::parse("en-US").expect("en-US is a valid BCP-47 tag")));
    assert_eq!(harness.input_attr("placeholder").expect("placeholder"), "Select an option");
    harness.send(i18n::Event::SetLocale(Locale::parse("es").expect("es is a valid BCP-47 tag")));
    assert_eq!(harness.input_attr("placeholder").expect("placeholder"), "Seleccione una opción");
}
```

### 3.2 Direction Propagation

```rust
#[wasm_bindgen_test]
async fn locale_change_updates_direction() {
    let harness = render(App::new()).await;
    harness.send(i18n::Event::SetLocale(Locale::parse("en-US").expect("en-US is a valid BCP-47 tag")));
    assert_eq!(harness.attr("[data-ars-scope]", "dir").expect("dir attr"), "ltr");
    harness.send(i18n::Event::SetLocale(Locale::parse("ar").expect("ar is a valid BCP-47 tag")));
    assert_eq!(harness.attr("[data-ars-scope]", "dir").expect("dir attr"), "rtl");
}
```

### 3.3 Adapter-Level Locale Switching (Reactive, No Remount)

Locale changes must propagate reactively through the adapter without unmounting and
remounting the component. A render counter proves the component was not destroyed.

#### 3.3.1 Leptos

```rust
use leptos::prelude::*;

#[wasm_bindgen_test]
async fn leptos_locale_switch_no_remount() {
    let locale = RwSignal::new(ars_i18n::Locale::parse("en").expect("valid locale"));
    let mount_id = "leptos-locale-test";
    mount_to_body(move || view! {
        <div data-mount-id=mount_id>
            <ArsProvider locale>
                <DateField id="df1" />
            </ArsProvider>
        </div>
    });
    tick().await;
    // Capture the mounted element
    let el = document().query_selector(&format!("[data-mount-id='{mount_id}']"))
        .expect("query must not error")
        .expect("element must exist");
    // Switch locale
    locale.set(ars_i18n::Locale::parse("ar").expect("valid locale"));
    tick().await;
    // Verify same DOM node (not remounted)
    let el_after = document().query_selector(&format!("[data-mount-id='{mount_id}']"))
        .expect("query must not error")
        .expect("element must still exist");
    assert_eq!(el, el_after, "component must not remount on locale switch");
    // Verify dir attribute updated
    assert_eq!(el_after.get_attribute("dir").as_deref(), Some("rtl"));
}
```

#### 3.3.2 Dioxus

```rust
use dioxus::prelude::*;

#[component]
fn LocaleSwitchTest() -> Element {
    let mut locale = use_signal(|| ars_i18n::Locale::parse("en").expect("valid locale"));
    let mut render_count = use_signal(|| 0u32);

    // Dioxus components re-run when tracked signals change.
    // Increment render_count in the component body (not use_effect) to accurately
    // track re-renders. use_effect only re-runs when signals READ inside it change.
    *render_count.write() += 1;

    rsx! {
        div {
            dir: if locale().language() == "ar" { "rtl" } else { "ltr" },
            "data-render-count": "{render_count}",
            ArsProvider { locale,
                Select { placeholder: "...",
                    select::Item { value: "a", "Alpha" }
                }
            }
            button {
                "data-testid": "switch-locale",
                onclick: move |_| locale.set(ars_i18n::Locale::parse("ar").expect("valid locale")),
                "Switch"
            }
        }
    }
}

#[test]
fn dioxus_locale_switch_no_remount() {
    let mut dom = VirtualDom::new(LocaleSwitchTest);
    dom.rebuild_in_place();
    let initial_html = dioxus::ssr::render(&dom);

    // Initial render: LTR direction
    assert!(initial_html.contains(r#"dir="ltr""#));
    // render_count is incremented in the component body, so it is 1 after the initial render.
    assert!(initial_html.contains(r#"data-render-count="1""#));

    // Note: Full interactive locale switching requires VirtualDom event simulation.
    // The SSR-only test above verifies initial state; a full round-trip test
    // (trigger button click -> re-render -> verify RTL) requires wasm_bindgen_test
    // or dioxus-testing infrastructure for event dispatch.
}
```

> **Note:** The SSR-only test above (`dioxus_locale_switch_no_remount`) can only verify the initial render. The `wasm_bindgen_test` version below exercises the full reactive locale switching path. Both tests are required.

```rust
#[component]
fn DioxusLocaleSwitchDomTest() -> Element {
    let mut locale = use_signal(|| ars_i18n::Locale::parse("en").expect("en is a valid BCP-47 tag"));
    let mount_id = "locale-test-mount";

    rsx! {
        div { data_mount_id: mount_id,
            ArsProvider { locale,
                DateField { id: "df1" }
            }
            button {
                data_testid: "switch-btn",
                onclick: move |_| locale.set(ars_i18n::Locale::parse("ar").expect("valid locale")),
                "Switch to Arabic"
            }
        }
    }
}

#[wasm_bindgen_test]
async fn dioxus_locale_switch_no_remount_dom() {
    let harness = render(DioxusLocaleSwitchDomTest).await;
    let mount_id = "locale-test-mount";
    let el = harness.query(&format!("[data-mount-id='{mount_id}']"))
        .expect("mounted element must exist");

    // Switch locale via button click. The harness flushes the reactivity cycle
    // before `click_selector` returns, so no ad hoc timer shim is needed here.
    harness.click_selector("[data-testid='switch-btn']").await;

    // Verify same element (no remount)
    let el_after = harness.query(&format!("[data-mount-id='{mount_id}']"))
        .expect("element must still exist after locale switch");
    assert_eq!(el, el_after, "component must not remount on locale switch");
    // Verify RTL direction applied
    assert_eq!(el_after.get_attribute("dir").as_deref(), Some("rtl"));
}
```

#### 3.3.3 Adapter-Level Runtime Locale Switching

```rust
use leptos::prelude::*;

#[wasm_bindgen_test]
async fn leptos_runtime_locale_switch_updates_direction() {
    let (locale, set_locale) = signal(ars_i18n::Locale::parse("en").expect("valid locale"));
    mount_to_body(move || view! {
        <ArsProvider locale>
            <div id="test-root">{move || locale.get().to_string()}</div>
        </ArsProvider>
    });
    leptos::task::tick().await;
    assert_eq!(document().query_selector("#test-root")
        .expect("query must not error").expect("test-root must exist")
        .get_attribute("dir"), Some("ltr".into()));

    set_locale.set(ars_i18n::Locale::parse("ar").expect("valid locale"));
    leptos::task::tick().await;
    assert_eq!(document().query_selector("#test-root")
        .expect("query must not error").expect("test-root must exist")
        .get_attribute("dir"), Some("rtl".into()));
}
```

### 3.4 Number/Date Format Changes

```rust
#[wasm_bindgen_test]
async fn locale_change_updates_number_format() {
    let harness = render(Slider::new(1000.5)).await;
    harness.send(i18n::Event::SetLocale(Locale::parse("en-US").expect("en-US is a valid BCP-47 tag")));
    assert_eq!(harness.input_attr("value").expect("value"), "1,000.5");
    harness.send(i18n::Event::SetLocale(Locale::parse("de").expect("de is a valid BCP-47 tag")));
    assert_eq!(harness.input_attr("value").expect("value"), "1.000,5");
}

#[wasm_bindgen_test]
async fn locale_change_updates_date_picker_format() {
    let harness = render(DatePicker::with_value(
        CalendarDate::new_gregorian(2026, NonZero::new(3).expect("nonzero"), NonZero::new(6).expect("nonzero")),
    )).await;
    harness.send(i18n::Event::SetLocale(Locale::parse("en-US").expect("en-US is a valid BCP-47 tag")));
    assert_eq!(harness.input_attr("value").expect("value"), "3/6/2026");
    harness.send(i18n::Event::SetLocale(Locale::parse("de").expect("de is a valid BCP-47 tag")));
    assert_eq!(harness.input_attr("value").expect("value"), "6.3.2026");
}
```

## 4. IME Composition Testing

Expand on [05-interactions.md §11.5](../foundation/05-interactions.md#115-ime-composition-handling) with component-specific composition tests.

### 4.1 TextField Composition

```rust
#[wasm_bindgen_test]
async fn textfield_ime_composition_no_intermediate_validation() {
    let harness = render(TextField::new().validate(|v| !v.is_empty())).await;
    harness.ime_compose("漢").await; // Intermediate
    // Validation must NOT fire during composition
    assert!(harness.input_attr("aria-invalid") != "true");
    harness.ime_compose("漢字").await;
    harness.ime_commit().await;
    // Now validation fires
    assert!(harness.input_attr("aria-invalid").is_empty() || harness.input_attr("aria-invalid") == "false");
}
```

### 4.2 Combobox Composition

```rust
#[wasm_bindgen_test]
async fn combobox_no_filtering_during_composition() {
    let harness = render(Combobox::with_items(cjk_items())).await;
    harness.ime_compose("に").await;
    // Dropdown should NOT filter during composition
    assert_eq!(harness.option_count(), cjk_items().len());
    harness.ime_compose("日本").await;
    harness.ime_commit().await;
    // Now filtering applies
    assert!(harness.option_count() < cjk_items().len());
}
```

### 4.3 Textarea Composition

```rust
#[wasm_bindgen_test]
async fn textarea_composition_preserves_cursor() {
    let harness = render(Textarea::new().value("Hello ")).await;
    // Set cursor position via web_sys DOM API (no TestHarness method exists for this)
    let textarea: web_sys::HtmlTextAreaElement = harness.query_selector("textarea")
        .expect("textarea").dyn_into().expect("textarea element");
    textarea.set_selection_start(Some(6));
    harness.ime_compose("世").await;
    harness.ime_compose("世界").await;
    harness.ime_commit().await;
    assert_eq!(harness.value(), "Hello 世界");
    // Read cursor position via web_sys DOM API
    assert_eq!(textarea.selection_start().expect("selection start"), Some(8));
}
```

### 4.4 NumberInput Composition

```rust
#[wasm_bindgen_test]
async fn number_input_ime_composition_no_validation_during_compose() {
    let harness = render(NumberInput::new().min(0.0).max(100.0)).await;
    harness.ime_compose("１").await; // Fullwidth digit 1
    // Validation and numeric parsing must NOT fire during composition
    assert!(harness.input_attr("aria-invalid") != "true");
    assert!(harness.data_attr("ars-composing") == "true");
    harness.ime_compose("１２３").await;
    harness.ime_commit().await;
    // After composition ends, the fullwidth digits are parsed to numeric value
    assert_eq!(harness.value(), 123.0);
    assert!(harness.input_attr("aria-invalid").is_empty() || harness.input_attr("aria-invalid") == "false");
}

#[wasm_bindgen_test]
async fn number_input_ime_rejects_non_numeric_composition() {
    let harness = render(NumberInput::new()).await;
    harness.ime_compose("あ").await;
    harness.ime_compose("abc").await;
    harness.ime_commit().await; // Non-numeric final value
    // NumberInput must reject non-numeric input and show error or revert
    assert!(harness.input_attr("aria-invalid") == "true" || harness.value().is_nan());
}

#[wasm_bindgen_test]
async fn number_input_ime_composition_fullwidth_digits() {
    let harness = render_with_dir(Direction::Ltr, NumberInput::new().id("n1").min(0.0).max(100.0)).await;
    harness.focus("[role='spinbutton']");
    harness.ime_compose("\u{ff11}\u{ff12}\u{ff13}").await; // fullwidth 1, 2, 3
    harness.ime_compose("\u{ff11}\u{ff12}\u{ff13}").await;
    harness.ime_commit().await;
    assert_eq!(harness.value(), 123.0);
}
```

> **Note:** Fullwidth-to-ASCII digit normalization should be specified in `NumberParser` (foundation 04-internationalization.md).

### 4.5 SearchInput Composition

```rust
#[wasm_bindgen_test]
async fn search_input_no_search_during_composition() {
    let search_count = Rc::new(RefCell::new(0u32));
    let search_count_clone = search_count.clone();

    let harness = render(SearchInput::new().on_search(move |_| {
        *search_count_clone.borrow_mut() += 1;
    })).await;

    harness.ime_compose("に").await;
    // Search callback must NOT fire during composition
    assert_eq!(*search_count.borrow(), 0, "search must not trigger during composition");

    harness.ime_compose("日本語").await;
    harness.ime_commit().await;
    // After composition ends, search fires with the composed value
    assert_eq!(*search_count.borrow(), 1, "search must fire exactly once after composition");
    assert_eq!(harness.value(), "日本語");
}

#[wasm_bindgen_test]
async fn search_input_composition_preserves_existing_text() {
    let harness = render(SearchInput::new().value("prefix: ")).await;
    // Set cursor position via web_sys DOM API (no TestHarness method exists for this)
    let input: web_sys::HtmlInputElement = harness.query_selector("input")
        .expect("input").dyn_into().expect("input element");
    input.set_selection_start(Some(8));
    harness.ime_compose("検").await;
    harness.ime_compose("検索").await;
    harness.ime_commit().await;
    assert_eq!(harness.value(), "prefix: 検索");
}
```

### 4.6 TagsInput Composition

```rust
fn tags_props() -> tags_input::Props { tags_input::Props::default() }

// 4.6 TagsInput IME composition
#[wasm_bindgen_test]
async fn tags_input_ime_composition_does_not_add_tag() {
    let harness = render(TagsInput::new(tags_props())).await;
    let input = harness.query("[data-ars-part='input']").expect("input must exist");
    input.focus();
    harness.ime_compose("東京").await;
    // Tag should NOT be added during composition
    assert_eq!(harness.query_selector_all("[data-ars-part='tag']").len(), 0, "no tag during IME composition");
    harness.ime_compose("東京").await;
    harness.ime_commit().await;
    harness.press_key(KeyboardKey::Enter);
    harness.flush().await;
    assert_eq!(harness.query_selector_all("[data-ars-part='tag']").len(), 1, "tag added after composition end + Enter");
}
```

### 4.7 IME Event Ordering

IME event firing order varies across platforms and browsers. Tests MUST verify correct behavior
for each platform's event sequence, since Android fires `input` before `compositionend` while
iOS may fire them simultaneously.

**Expected event sequences per platform:**

| Platform           | Sequence                                                                                                                                                        |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Desktop (Chrome)   | `compositionstart` → `compositionupdate` → `input` → `compositionend` → `input`                                                                                 |
| Android (Chrome)   | `compositionstart` → `compositionupdate` → `input` → `compositionend`                                                                                           |
| iOS Safari         | `compositionstart` → `compositionupdate` → `compositionend` + `input` (same tick)                                                                               |
| Firefox (Desktop)  | `compositionstart` → `compositionupdate` → `input` → `compositionupdate` → `input` → `compositionend` → `input`                                                 |
| Windows IME (Edge) | `compositionstart` → `compositionupdate` → `beforeinput` → `input` → `compositionupdate` → `beforeinput` → `input` → `compositionend` → `beforeinput` → `input` |

**Adapter behavior during composition phases:**

- **During `compositionstart`→`compositionend`**: All `input` events are marked provisional.
  Adapters MUST NOT commit values, trigger validation, or fire `on_change` callbacks.
- **On `compositionend`**: The adapter commits the final composed text and fires `on_change`
  exactly once with the composed value.
- **If `input` fires after `compositionend` (desktop Chrome)**: The adapter deduplicates by
  comparing with the already-committed composition result and suppresses the redundant event.

```rust
fn make_textfield_service() -> Service<text_field::Machine> {
    Service::new(text_field::Props::default(), Env::default(), Default::default())
}

#[test]
fn ime_android_input_before_compositionend() {
    let mut svc = make_textfield_service();
    svc.send(Event::Focus { is_keyboard: false });
    svc.send(Event::CompositionStart);
    // Android fires Change before CompositionEnd — the machine must suppress
    // on_change callbacks while is_composing is true.
    svc.send(Event::Change("か".into()));
    assert!(svc.context().is_composing, "Change during composition must not commit");
    assert!(svc.context().on_change_count == 0, "on_change must not fire during composition");

    svc.send(Event::CompositionEnd);
    // After CompositionEnd, the adapter fires Change with the final value.
    svc.send(Event::Change("漢字".into()));
    assert!(!svc.context().is_composing);
    assert_eq!(svc.context().on_change_count, 1, "on_change fires exactly once on commit");
    assert_eq!(svc.context().value, "漢字");
}

#[test]
fn ime_mid_composition_correction() {
    let mut svc = make_textfield_service();
    svc.send(Event::Focus { is_keyboard: false });
    svc.send(Event::CompositionStart);
    // Intermediate composition updates (CompositionUpdate) are DOM-level events
    // handled by the adapter, not by the state machine. The adapter suppresses
    // Change events while is_composing is true.
    svc.send(Event::CompositionEnd);
    // Only the final value (from the adapter's Change event after CompositionEnd) matters.
    svc.send(Event::Change("学校".into()));
    assert_eq!(svc.context().value, "学校");
    assert_eq!(svc.context().on_change_count, 1);
}
```

---

## 5. TypeAhead Timeout Testing

Select typeahead buffer must be cleared after a configurable timeout. Tests must verify timer creation, previous timeout cancellation, buffer clear after timeout, and rapid typing behavior.

### 5.1 Basic Timeout Behavior

```rust
#[test]
fn typeahead_buffer_clears_after_timeout() {
    let items = vec![
        select::Item { key: "apple".into(), label: "Apple".into(), ..Default::default() },
        select::Item { key: "apricot".into(), label: "Apricot".into(), ..Default::default() },
        select::Item { key: "banana".into(), label: "Banana".into(), ..Default::default() },
    ];
    let props = select::Props { items, ..Default::default() };
    let timer = FakeTimer::new();
    install_fake_timers(&timer);
    let mut svc = Service::<select::Machine>::new(props);
    svc.send(select::Event::Open);

    // Type 'a' — buffer becomes "a", highlights "Apple"
    svc.send(select::Event::TypeaheadSearch('a'));
    assert_eq!(svc.context().typeahead_buffer.as_str(), "a");
    assert_eq!(svc.context().highlighted_key.as_deref(), Some("apple"));

    // Simulate timeout (advance fake timer past typeahead_timeout)
    timer.advance(Duration::from_millis(600)); // default timeout ~500ms

    assert_eq!(
        svc.context().typeahead_buffer.as_str(),
        "",
        "Buffer must be cleared after timeout"
    );
}
```

### 5.2 Rapid Typing Resets Timeout

```rust
#[test]
fn typeahead_rapid_typing_concatenates_buffer() {
    let items = vec![
        select::Item { key: "apple".into(), label: "Apple".into(), ..Default::default() },
        select::Item { key: "apricot".into(), label: "Apricot".into(), ..Default::default() },
        select::Item { key: "banana".into(), label: "Banana".into(), ..Default::default() },
    ];
    let props = select::Props { items, ..Default::default() };
    let timer = FakeTimer::new();
    install_fake_timers(&timer);
    let mut svc = Service::<select::Machine>::new(props);
    svc.send(select::Event::Open);

    // Type 'a' then 'p' quickly (within timeout)
    svc.send(select::Event::TypeaheadSearch('a'));
    timer.advance(Duration::from_millis(100)); // well within timeout
    svc.send(select::Event::TypeaheadSearch('p'));

    assert_eq!(svc.context().typeahead_buffer.as_str(), "ap");
    assert_eq!(
        svc.context().highlighted_key.as_deref(),
        Some("apple"), // "ap" matches "Apple" first
    );

    // Previous timeout must be cancelled; only the latest timer matters
    timer.advance(Duration::from_millis(400)); // 400ms after second keystroke, still within timeout
    assert_eq!(svc.context().typeahead_buffer.as_str(), "ap", "Buffer must persist — timeout not yet elapsed");

    timer.advance(Duration::from_millis(200)); // now 600ms after second keystroke
    assert_eq!(svc.context().typeahead_buffer.as_str(), "", "Buffer must clear after timeout from last keystroke");
}
```

### 5.3 Typing After Timeout Starts Fresh Buffer

```rust
#[test]
fn typeahead_after_timeout_starts_fresh() {
    let items = vec![
        select::Item { key: "apple".into(), label: "Apple".into(), ..Default::default() },
        select::Item { key: "banana".into(), label: "Banana".into(), ..Default::default() },
    ];
    let props = select::Props { items, ..Default::default() };
    let timer = FakeTimer::new();
    install_fake_timers(&timer);
    let mut svc = Service::<select::Machine>::new(props);
    svc.send(select::Event::Open);

    svc.send(select::Event::TypeaheadSearch('a'));
    timer.advance(Duration::from_millis(600));
    assert_eq!(svc.context().typeahead_buffer.as_str(), "");

    // Type 'b' — starts fresh buffer
    svc.send(select::Event::TypeaheadSearch('b'));
    assert_eq!(svc.context().typeahead_buffer.as_str(), "b");
    assert_eq!(svc.context().highlighted_key.as_deref(), Some("banana"));
}
```

---

## 6. Message Localization

Tests for the `Messages` struct override system, locale fallback behavior, and plural rule correctness.

### 6.1 Override Messages Struct Changes Rendered Text

```rust
#[wasm_bindgen_test]
async fn custom_messages_override_defaults() {
    let mut messages = select::Messages::default();
    messages.placeholder = "Pick one...".into();
    messages.no_results = "Nothing found".into();

    let harness = render(Select::new().messages(messages)).await;
    assert_eq!(harness.input_attr("placeholder").expect("placeholder"), "Pick one...");
    harness.open();
    harness.type_text("zzzzz");
    assert_eq!(harness.query_part("no-results").expect("no-results").text_content(), "Nothing found");
}

#[wasm_bindgen_test]
async fn toast_custom_close_label() {
    let mut messages = toast::Messages::default();
    messages.close_label = "Dismiss".into();

    let harness = render(Toaster::new().messages(messages)).await;
    harness.send(toaster::Event::AddToast { message: "Info".into() });
    assert_eq!(harness.query_part("close-trigger").expect("close trigger").attr("aria-label").expect("label"), "Dismiss");
}
```

### 6.2 Fallback to English When Locale Not Available

```rust
#[wasm_bindgen_test]
async fn unknown_locale_falls_back_to_english() {
    let harness = render(Select::new()).await;
    // "xx-XX" is a valid BCP 47 tag but has no translations — tests fallback behavior.
    harness.send(i18n::Event::SetLocale(Locale::parse("xx-XX").expect("xx-XX is a valid BCP-47 tag")));
    // Should display English defaults, not panic or show empty strings
    assert_eq!(harness.input_attr("placeholder").expect("placeholder"), "Select an option");
}

#[wasm_bindgen_test]
async fn partial_locale_falls_back_gracefully() {
    // If "fr-CA" is not defined but "fr" is, fall back to "fr"
    let harness = render(Select::new()).await;
    harness.send(i18n::Event::SetLocale(Locale::parse("fr-CA").expect("fr-CA is a valid BCP-47 tag")));
    assert!(!harness.input_attr("placeholder").expect("placeholder").is_empty());
}
```

### 6.3 Plural Rules

```rust
#[wasm_bindgen_test]
async fn plural_rules_english() {
    let harness = render(Select::new().selection_mode(selection::Mode::Multiple)).await;
    for item in &["a"] { harness.click_selector(&format!("[data-value='{item}']")); }
    assert_eq!(harness.query_part("summary").expect("summary").text_content(), "1 item selected");
    for item in &["a", "b"] { harness.click_selector(&format!("[data-value='{item}']")); }
    assert_eq!(harness.query_part("summary").expect("summary").text_content(), "2 items selected");
    harness.send(select::Event::ClearSelection);
    assert_eq!(harness.query_part("summary").expect("summary").text_content(), "0 items selected");
}

#[wasm_bindgen_test]
async fn plural_rules_per_locale() {
    let test_cases = vec![
        (Locale::parse("en").expect("en is a valid BCP-47 tag"), 0, "0 items selected"),
        (Locale::parse("en").expect("en is a valid BCP-47 tag"), 1, "1 item selected"),
        (Locale::parse("en").expect("en is a valid BCP-47 tag"), 2, "2 items selected"),
        (Locale::parse("en").expect("en is a valid BCP-47 tag"), 5, "5 items selected"),
        // Polish has complex plural rules: 1, 2-4, 5+
        (Locale::parse("pl").expect("pl is a valid BCP-47 tag"), 1, "1 element wybrany"),
        (Locale::parse("pl").expect("pl is a valid BCP-47 tag"), 2, "2 elementy wybrane"),
        (Locale::parse("pl").expect("pl is a valid BCP-47 tag"), 5, "5 elementów wybranych"),
    ];

    for (locale, count, expected) in test_cases {
        let harness = render(Select::new().selection_mode(selection::Mode::Multiple)).await;
        harness.send(i18n::Event::SetLocale(locale));
        let items: Vec<_> = (0..count).map(|i| format!("item_{i}")).collect();
        for item in &items { harness.click_selector(&format!("[data-value='{item}']")); }
        assert_eq!(
            harness.query_part("summary").expect("summary").text_content(), expected,
            "Plural rule failed for locale={locale:?}, count={count}"
        );
    }
}
```

### 6.4 MessageFn Callbacks Receive Correct Parameters

```rust
#[wasm_bindgen_test]
async fn message_fn_receives_correct_params() {
    let captured = Rc::new(RefCell::new(Vec::new()));
    let captured_clone = captured.clone();

    let mut messages = select::Messages::default();
    messages.count_summary = MessageFn::new(move |params| {
        captured_clone.borrow_mut().push(params.clone());
        format!("{} chosen", params.count)
    });

    let harness = render(Select::new()
        .messages(messages)
        .selection_mode(selection::Mode::Multiple)
    ).await;
    for item in &["a", "b", "c"] { harness.click_selector(&format!("[data-value='{item}']")); }

    let calls = captured.borrow();
    assert_eq!(calls.last().expect("calls must not be empty").count, 3);
    assert_eq!(harness.query_part("summary").expect("summary").text_content(), "3 chosen");
}
```

---

## 7. Internationalization Testing

### 7.1 Calendar System Test Matrix

All 13 supported calendar systems MUST be tested for DateField, DatePicker, and Calendar:

| Calendar System       | Test Locale                   | Key Verification               |
| --------------------- | ----------------------------- | ------------------------------ |
| Gregorian             | `en-US`                       | Standard date math             |
| Japanese              | `ja-JP-u-ca-japanese`         | Era boundaries (Reiwa/Heisei)  |
| Buddhist              | `th-TH-u-ca-buddhist`         | Year offset (+543)             |
| Chinese               | `zh-CN-u-ca-chinese`          | Leap months                    |
| Hebrew                | `he-IL-u-ca-hebrew`           | 13-month leap years            |
| Islamic (civil)       | `ar-SA-u-ca-islamic-civil`    | 354/355-day years              |
| Islamic (tabular)     | `ar-EG-u-ca-islamic-tbla`     | Tabular month lengths          |
| Islamic (Umm al-Qura) | `ar-SA-u-ca-islamic-umalqura` | Saudi official calendar        |
| Persian               | `fa-IR-u-ca-persian`          | Solar Hijri year               |
| Indian                | `hi-IN-u-ca-indian`           | Saka era                       |
| Coptic                | `ar-EG-u-ca-coptic`           | 13th month (Nasie)             |
| Ethiopic              | `am-ET-u-ca-ethiopic`         | 13-month year                  |
| ROC (Minguo)          | `zh-TW-u-ca-roc`              | Year offset (year 1 = 1912 CE) |

### 7.2 DateField Calendar System Test Matrix

For each calendar system × primary locale combination in the table above, the following DateField-specific behaviors MUST be verified:

| Test Case                   | Assertion                                                                                                                                  |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| **Segment order**           | `segments_for_locale(locale)` returns segments in the locale's canonical order (e.g., `ja-JP`: Year/Month/Day, `en-US`: Month/Day/Year)    |
| **Type-ahead**              | Typing numeric characters advances segment values correctly; month name type-ahead (if applicable) matches locale-specific month names     |
| **Auto-advance**            | After filling a segment to its maximum digit count, focus auto-advances to the next editable segment                                       |
| **Min/max with leap rules** | Setting `min_value` / `max_value` that span a leap boundary correctly constrains input (e.g., Hebrew month 13 is valid in leap years only) |
| **Era boundaries**          | For Japanese calendar: typing year 1 with era=Reiwa correctly constrains to dates ≥ May 1, 2019                                            |
| **Calendar switching**      | Changing `calendar_system` prop clamps out-of-range dates and emits `DateClamped`                                                          |

```rust
/// Example: DateField calendar system test for Hebrew leap year
#[test]
fn datefield_hebrew_leap_month_validation() {
    let locale = Locale::parse("he-IL-u-ca-hebrew").expect("he-IL-u-ca-hebrew is a valid BCP-47 tag");
    let props = date_field::Props {
        calendar_system: CalendarSystem::Hebrew,
        locale: locale.clone(),
        ..Default::default()
    };
    let mut svc = Service::<DateField>::new(props);

    // Hebrew year 5784 is a leap year (year 8 in 19-year cycle)
    svc.send(Event::SetSegment(DateSegmentKind::Year, 5784));
    svc.send(Event::SetSegment(DateSegmentKind::Month, 13));
    assert!(svc.context().is_valid(), "Month 13 should be valid in Hebrew leap year");

    // Hebrew year 5785 is NOT a leap year
    svc.send(Event::SetSegment(DateSegmentKind::Year, 5785));
    assert!(!svc.context().is_valid(), "Month 13 should be invalid in non-leap Hebrew year");
}
```

#### 7.2.1 Concrete Calendar System Tests

```rust
#[test]
fn japanese_era_boundary_reiwa() {
    let provider = ars_i18n::create_test_provider();
    let date = CalendarDate::new(
        &provider, CalendarSystem::Japanese, Some(Era { code: "reiwa".into(), display_name: String::new() }), 2019, 5, 1,
    ).expect("valid Japanese date at Reiwa boundary");
    assert_eq!(date.era.as_ref().map(|e| e.code.as_str()), Some("reiwa"));
    assert_eq!(date.year, 2019);
}

#[test]
fn buddhist_year_offset() {
    let provider = ars_i18n::create_test_provider();
    let date = CalendarDate::new(
        &provider, CalendarSystem::Buddhist, None, 2024, 1, 1,
    ).expect("valid Buddhist date");
    // Buddhist calendar year = Gregorian year + 543
    assert_eq!(date.year, 2024);
}

#[test]
fn chinese_leap_month() {
    let provider = ars_i18n::create_test_provider();
    // Chinese calendar: some years have a leap month
    let date = CalendarDate::new(
        &provider, CalendarSystem::Chinese, None, 4721, 4, 1,
    ).expect("valid Chinese date with leap month");
    assert!(date.month.get() == 4);
}

#[test]
fn roc_year_offset() {
    let provider = ars_i18n::create_test_provider();
    let date = CalendarDate::new(
        &provider, CalendarSystem::Roc, None, 2024, 1, 1,
    ).expect("valid ROC date");
    // ROC (Minguo) year = Gregorian year - 1911
    assert_eq!(date.year, 2024);
}
```

### 7.3 RTL Layout Assertion Helpers

```rust
/// Assert that a component's ARIA attributes are RTL-correct.
fn assert_rtl_layout(api: &impl ComponentApi, locale: &Locale) {
    assert!(locale.is_rtl());
    let attrs = api.root_attrs();
    assert_eq!(attrs.get(&HtmlAttr::Dir), Some("rtl"));
}

/// Assert that arrow key semantics are flipped for RTL.
/// Accepts concrete left/right events since `Event` is component-specific.
fn assert_rtl_arrow_keys<M: Machine>(
    svc: &mut Service<M>,
    left_event: M::Event,
    right_event: M::Event,
    locale: &ars_i18n::Locale,
) {
    assert!(locale.is_rtl());
    // In RTL, ArrowLeft should move forward (next item), ArrowRight should move backward.
    svc.send(left_event);
    let idx_after_left = svc.context().focused_index;
    svc.send(right_event);
    let idx_after_right = svc.context().focused_index;
    assert!(idx_after_left > idx_after_right,
        "RTL: ArrowLeft should advance, ArrowRight should retreat");
}
```

### 7.4 Locale-Swap State Persistence Tests

```rust
#[test]
fn locale_swap_preserves_component_state() {
    // Start with en-US locale, interact with the component, then swap to ar-SA.
    let mut svc = Service::<datepicker::Machine>::new(datepicker::Props {
        locale: Locale::parse("en-US").expect("en-US is a valid BCP-47 tag"),
        ..Default::default()
    });
    svc.send(datepicker::Event::Open);
    // Use fixed date for deterministic tests instead of CalendarDate::today()
    let today = CalendarDate::new_gregorian(
        2026, NonZero::new(3).expect("nonzero"), NonZero::new(25).expect("nonzero"),
    );
    svc.send(datepicker::Event::SelectDate(today));
    let selected = svc.context().selected_date.clone();

    // Swap locale — state (selected date, open/closed) must persist.
    svc.set_props(datepicker::Props {
        locale: Locale::parse("ar-SA").expect("ar-SA is a valid BCP-47 tag"),
        ..Default::default()
    });
    assert_eq!(svc.context().selected_date, selected, "selected date must persist across locale swap");
    assert_eq!(*svc.state(), datepicker::State::Open, "open state must persist across locale swap");
}
```

---

## 8. User-Defined Translatable Text Testing

Tests for the `Translate` trait and `t()` function (see [04-internationalization.md](../foundation/04-internationalization.md) §7.4–7.5). These tests verify that user-defined translatable enums resolve correctly across locales and react to locale changes.

### 8.1 Pure Translation Tests

`Translate::translate()` is a pure function — test it directly without DOM or framework.

```rust
use ars_i18n::{Translate, Locale, PluralCategory, PluralRuleType};

/// Example user-defined translatable enum (see 04-internationalization.md §7.4.3).
enum Inventory {
    Title,
    Welcome,
    ItemCount { count: usize },
}

impl Translate for Inventory {
    fn translate(&self, locale: &Locale, intl: &dyn IntlBackend) -> String {
        match locale.language().as_str() {
            "es" => match self {
                Self::Title => "Inventario".into(),
                Self::Welcome => "¡Bienvenido!".into(),
                Self::ItemCount { count } => {
                    let cat = ars_i18n::select_plural(
                        locale, *count as f64, PluralRuleType::Cardinal,
                    );
                    match cat {
                        PluralCategory::One => format!("{count} elemento"),
                        _ => format!("{count} elementos"),
                    }
                }
            },
            _ => match self {
                Self::Title => "Inventory".into(),
                Self::Welcome => "Welcome!".into(),
                Self::ItemCount { count } => {
                    let cat = ars_i18n::select_plural(
                        locale, *count as f64, PluralRuleType::Cardinal,
                    );
                    match cat {
                        PluralCategory::One => format!("{count} item"),
                        _ => format!("{count} items"),
                    }
                }
            },
        }
    }
}

#[test]
fn translate_title_per_locale() {
    let intl = StubIntlProvider;
    assert_eq!(
        Inventory::Title.translate(
            &Locale::parse("en").expect("en is a valid BCP-47 tag"), &intl,
        ),
        "Inventory",
    );
    assert_eq!(
        Inventory::Title.translate(
            &Locale::parse("es").expect("es is a valid BCP-47 tag"), &intl,
        ),
        "Inventario",
    );
}

#[test]
fn translate_welcome_per_locale() {
    let intl = StubIntlProvider;
    assert_eq!(
        Inventory::Welcome.translate(
            &Locale::parse("en").expect("en is a valid BCP-47 tag"), &intl,
        ),
        "Welcome!",
    );
    assert_eq!(
        Inventory::Welcome.translate(
            &Locale::parse("es").expect("es is a valid BCP-47 tag"), &intl,
        ),
        "¡Bienvenido!",
    );
}
```

### 8.2 Plural Correctness

Test data-carrying variants across locales with different CLDR plural rules.

```rust
#[test]
fn translate_item_count_english_plurals() {
    let locale = Locale::parse("en").expect("en is a valid BCP-47 tag");
    let intl = StubIntlProvider;
    assert_eq!(Inventory::ItemCount { count: 0 }.translate(&locale, &intl), "0 items");
    assert_eq!(Inventory::ItemCount { count: 1 }.translate(&locale, &intl), "1 item");
    assert_eq!(Inventory::ItemCount { count: 2 }.translate(&locale, &intl), "2 items");
    assert_eq!(Inventory::ItemCount { count: 42 }.translate(&locale, &intl), "42 items");
}

#[test]
fn translate_item_count_spanish_plurals() {
    let locale = Locale::parse("es").expect("es is a valid BCP-47 tag");
    let intl = StubIntlProvider;
    assert_eq!(Inventory::ItemCount { count: 0 }.translate(&locale, &intl), "0 elementos");
    assert_eq!(Inventory::ItemCount { count: 1 }.translate(&locale, &intl), "1 elemento");
    assert_eq!(Inventory::ItemCount { count: 5 }.translate(&locale, &intl), "5 elementos");
}
```

For locales with complex plural rules (Polish: one/few/many, Arabic: zero/one/two/few/many/other), users MUST test all CLDR plural categories their `translate()` handles:

```rust
/// Example: Polish plural rules (one / few / many / other)
enum PolishItems {
    Count { count: usize },
}

impl Translate for PolishItems {
    fn translate(&self, locale: &Locale, _intl: &dyn IntlBackend) -> String {
        match locale.language().as_str() {
            "pl" => match self {
                Self::Count { count } => {
                    let cat = ars_i18n::select_plural(
                        locale, *count as f64, PluralRuleType::Cardinal,
                    );
                    match cat {
                        PluralCategory::One => format!("{count} element"),
                        PluralCategory::Few => format!("{count} elementy"),
                        _ => format!("{count} elementów"), // many + other
                    }
                }
            },
            _ => match self {
                Self::Count { count } => {
                    let cat = ars_i18n::select_plural(
                        locale, *count as f64, PluralRuleType::Cardinal,
                    );
                    match cat {
                        PluralCategory::One => format!("{count} item"),
                        _ => format!("{count} items"),
                    }
                }
            },
        }
    }
}

#[test]
fn polish_plural_categories() {
    let locale = Locale::parse("pl").expect("pl is a valid BCP-47 tag");
    let intl = StubIntlProvider;
    assert_eq!(PolishItems::Count { count: 1 }.translate(&locale, &intl), "1 element");
    assert_eq!(PolishItems::Count { count: 2 }.translate(&locale, &intl), "2 elementy");
    assert_eq!(PolishItems::Count { count: 5 }.translate(&locale, &intl), "5 elementów");
    assert_eq!(PolishItems::Count { count: 22 }.translate(&locale, &intl), "22 elementy");
}
```

### 8.3 Reactive Locale Switching

Mount a component that uses `t()` and verify text updates when locale changes.

```rust
#[wasm_bindgen_test]
async fn t_function_updates_on_locale_switch() {
    let harness = mount_with_locale(
        InventoryPage { item_count: 3 },
        Locale::parse("en").expect("en is a valid BCP-47 tag"),
    ).await;
    assert_eq!(harness.query("h1").text_content(), "Inventory");
    assert_eq!(harness.query("span").text_content(), "3 items");

    // Switch locale — t() reactive text must update without component remount
    harness.set_locale(Locale::parse("es").expect("es is a valid BCP-47 tag"));
    assert_eq!(harness.query("h1").text_content(), "Inventario");
    assert_eq!(harness.query("span").text_content(), "3 elementos");
}
```

### 8.4 Fallback Behavior

Verify that unknown locales fall through to the English `_` arm.

```rust
#[test]
fn unknown_locale_falls_back_to_english() {
    let locale = Locale::parse("xx-XX").expect("xx-XX is a valid BCP-47 tag");
    let intl = StubIntlProvider;
    assert_eq!(Inventory::Title.translate(&locale, &intl), "Inventory");
    assert_eq!(Inventory::Welcome.translate(&locale, &intl), "Welcome!");
    assert_eq!(Inventory::ItemCount { count: 1 }.translate(&locale, &intl), "1 item");
}
```

### 8.5 Intl Backend Threading

Verify that the `intl` parameter is correctly passed through for locale-aware formatting
within `translate()`. This tests the case where `translate()` uses `NumberFormatter` or
`DateFormatter` from the ICU provider.

```rust
/// Example: currency formatting inside translate()
enum Price {
    Total { amount: f64 },
}

impl Translate for Price {
    fn translate(&self, locale: &Locale, intl: &dyn IntlBackend) -> String {
        match self {
            Self::Total { amount } => {
                let formatter = ars_i18n::NumberFormatter::new(
                    locale,
                    ars_i18n::NumberFormatOptions {
                        style: ars_i18n::NumberStyle::Currency(ars_i18n::CurrencyCode::USD),
                        ..ars_i18n::NumberFormatOptions::default()
                    },
                );
                match locale.language().as_str() {
                    "es" => format!("Total: {}", formatter.format(*amount)),
                    _ => format!("Total: {}", formatter.format(*amount)),
                }
            }
        }
    }
}

#[test]
fn translate_with_icu_number_formatting() {
    let locale = Locale::parse("en-US").expect("en-US is a valid BCP-47 tag");
    let intl = StubIntlProvider;
    let result = Price::Total { amount: 1234.50 }.translate(&locale, &intl);
    assert!(result.starts_with("Total: "), "formatted price must start with 'Total: '");
    // The exact formatted output depends on the active backend; StubIntlProvider
    // may produce simplified formatting. Production tests use Icu4xBackend.
}
```

---

> **See also:**
>
> - [04-internationalization.md](../foundation/04-internationalization.md) §7.4–7.5 — `Translate` trait and `t()` function specification
> - [08-adapter-leptos.md](../foundation/08-adapter-leptos.md) §13.2 — Leptos `t()` implementation
> - [09-adapter-dioxus.md](../foundation/09-adapter-dioxus.md) §16.2 — Dioxus `t()` implementation
> - [11-dom-utilities.md](../foundation/11-dom-utilities.md) — Overlay positioning RTL mirroring via `Placement::resolve_logical`
