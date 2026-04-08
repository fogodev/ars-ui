# Foundation Completion Roadmap

This roadmap picks up after the foundation-gap-audit tasks (#31–#41) and defines the remaining work required to complete the foundation layer before any UI component implementation begins.

## Why Complete the Foundation First

The project needs a fully stable foundation before component work starts. Components built on incomplete foundation crates will create merge conflicts when parallel component PRs all touch the same foundation files. By completing interactions, collections, DOM positioning, and i18n first, component authors can work independently against stable APIs.

## Status Summary

### What is built (357 tests passing)

| Crate              | LOC   | Status     | Key surface                                                                                                                                                                                                |
| ------------------ | ----- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ars-core`         | 4,306 | Solid      | Machine, Service, TransitionPlan, PendingEffect, Bindable, ConnectApi, ComponentPart, AttrMap/AttrValue/UserAttrs, StyleStrategy, Callback, WeakSend, PlatformEffects, Provider (ColorMode), companion CSS |
| `ars-derive`       | 535   | Complete   | HasId, ComponentPart proc macros with error tests                                                                                                                                                          |
| `ars-a11y`         | 2,271 | Solid      | AriaRole, AriaAttribute, ComponentIds, ARIA state helpers, FocusScopeBehavior, FocusStrategy                                                                                                               |
| `ars-forms`        | 4,128 | Solid      | field::State/Value/Context/Descriptors/InputAria, validation::Error/Validator/AsyncValidator, form::Context/Data/Mode, hidden_input, form_submit machine                                                   |
| `ars-interactions` | 559   | Stubs only | PointerType, PressState, FocusState enums, compose::merge_attrs                                                                                                                                            |
| `ars-dom`          | 1,777 | Partial    | FocusScope, focus queries, ScrollLockManager                                                                                                                                                               |
| `ars-leptos`       | 751   | Partial    | use_machine, UseMachineReturn, EphemeralRef, use_id, AdapterCapabilities                                                                                                                                   |
| `ars-dioxus`       | 762   | Partial    | Same as Leptos adapter                                                                                                                                                                                     |
| `ars-collections`  | 28    | Stub       | Selection\<T\> only                                                                                                                                                                                        |
| `ars-i18n`         | 116   | Stub       | Locale, Direction, Orientation, placeholder date/time types                                                                                                                                                |

### Foundation gap matrix

| Foundation area    | Spec file                    | Spec coverage           | Implementation % | Blocking impact                    |
| ------------------ | ---------------------------- | ----------------------- | ---------------- | ---------------------------------- |
| Interactions       | `05-interactions.md`         | ~600 lines, 12 sections | 5%               | Blocks ALL interactive components  |
| Collections        | `06-collections.md`          | ~400 lines, 6 sections  | 10%              | Blocks all list-based components   |
| I18n               | `04-internationalization.md` | ~500 lines              | 10%              | Blocks number/date components, RTL |
| DOM utilities      | `11-dom-utilities.md`        | ~400 lines, 8 sections  | 30%              | Blocks all overlay components      |
| Accessibility      | `03-accessibility.md`        | ~300 lines              | 60%              | LiveAnnouncer, FocusRing missing   |
| Adapter conversion | `08/09-adapter-*.md` §4/§3   | ~200 lines              | 0%               | Blocks ALL component rendering     |

## Task Waves

### Wave 1: Interaction Core and Adapter Conversion

**Goal:** Enable Button, VisuallyHidden, and Separator — the simplest components.

**Parallelism:** #55, #56, and #61 can all run in parallel. The original `#57` thread-local modality task is superseded by #90. #58, #59, and #60 depend on #90.

| GitHub                                             | Title                                                                                      | Points | Epic | Deps |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------ | ------ | ---- | ---- |
| [#55](https://github.com/fogodev/ars-ui/issues/55) | Implement `attr_map_to_leptos` and `use_style_strategy` in ars-leptos                      | 3      | #8   | —    |
| [#56](https://github.com/fogodev/ars-ui/issues/56) | Implement `attr_map_to_dioxus`, `intern_attr_name`, and `use_style_strategy` in ars-dioxus | 3      | #9   | —    |
| [#57](https://github.com/fogodev/ars-ui/issues/57) | Superseded thread-local modality task in ars-interactions                                  | 2      | #4   | —    |
| [#58](https://github.com/fogodev/ars-ui/issues/58) | Implement Press interaction state machine in ars-interactions                              | 5      | #4   | #90  |
| [#59](https://github.com/fogodev/ars-ui/issues/59) | Implement Hover interaction state machine in ars-interactions                              | 2      | #4   | #90  |
| [#60](https://github.com/fogodev/ars-ui/issues/60) | Implement Focus and FocusWithin interactions in ars-interactions                           | 3      | #4   | #90  |
| [#61](https://github.com/fogodev/ars-ui/issues/61) | Implement LogicalDirection and resolve_arrow_key in ars-interactions                       | 1      | #4   | —    |

**Total:** 19 points

---

### Wave 1 Task Details

#### W1-1: Implement `attr_map_to_leptos` and `use_style_strategy` in ars-leptos

- Points: `3`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/08-adapter-leptos.md` §4 "AttrMap to Leptos Attributes" (L621)
  - `spec/foundation/08-adapter-leptos.md` §4.1 "AttrMap Conversion" (L625)
  - `spec/foundation/08-adapter-leptos.md` §4.3 "Event Listener Options" (L745)
  - `spec/foundation/08-adapter-leptos.md` §4.5 "CSP Style Strategy" (L792)
- Goal: implement the bridge from framework-agnostic `AttrMap` to Leptos-ready attribute tuples, with style strategy awareness.
- Files to create/modify: `crates/ars-leptos/src/attrs.rs` (new), wire into `crates/ars-leptos/src/lib.rs`
- Tests to add first:
  - Unit tests for `attr_map_to_leptos` with `Inline` strategy producing `style` attribute string.
  - Unit tests for `Cssom` strategy returning styles in `cssom_styles` field.
  - Unit tests for `Nonce` strategy generating CSS rule text with `data-ars-style-id` selector.
  - Unit tests for `AttrValue::Bool(true)` mapping to empty string, `Bool(false)` and `None` being filtered out.
  - Unit test for `use_style_strategy()` context hook returning the configured strategy.
- Acceptance criteria:
  - `LeptosAttrResult` struct with `attrs: Vec<(String, String)>`, `cssom_styles: Vec<(CssProperty, String)>`, `nonce_css: String`.
  - `attr_map_to_leptos(map, strategy, element_id) -> LeptosAttrResult` handles all three `StyleStrategy` variants.
  - `apply_styles_cssom(el, styles)` implemented behind `#[cfg(not(feature = "ssr"))]`.
  - `styles_to_nonce_css(id, styles)` generates valid CSS rule text.
  - `use_style_strategy()` Leptos context hook returning `StyleStrategy`.
  - `EventOptions` struct with `passive` and `capture` fields.
- Spec impact: `No spec change required`.

#### W1-2: Implement `attr_map_to_dioxus`, `intern_attr_name`, and `use_style_strategy` in ars-dioxus

- Points: `3`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/09-adapter-dioxus.md` §3 "AttrMap to Dioxus Attributes" (L600)
- Goal: implement the symmetric Dioxus counterpart of W1-1 with attribute name interning.
- Files to create/modify: `crates/ars-dioxus/src/attrs.rs` (new), wire into `crates/ars-dioxus/src/lib.rs`
- Tests to add first:
  - Unit tests symmetric with W1-1 for all three style strategies.
  - Unit tests for `intern_attr_name()` fast path using `static_name()`.
  - Unit tests for `dioxus_attrs!` macro producing correct `Attribute` values.
- Acceptance criteria:
  - `ATTR_NAMES` static `LazyLock<Mutex<HashSet<&'static str>>>` intern pool.
  - `intern_attr_name()` with fast path for `static_name()` variants.
  - `DioxusAttrResult` with `attrs: Vec<Attribute>`, `cssom_styles`, `nonce_css`.
  - `attr_map_to_dioxus(map, strategy, element_id)` handles all three strategies.
  - `use_style_strategy()` Dioxus context hook.
  - `dioxus_attrs!` macro.
- Spec impact: `No spec change required`.

#### W1-3: Superseded thread-local modality task in ars-interactions

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/05-interactions.md` §4.4 "Focus Visible Detection: Shared Modality Tracking" (L1135)
  - `spec/foundation/05-interactions.md` §3.4 "Integration with Press" (L881)
- Goal: historical only. The old thread-local design is superseded by #90 and should not be implemented independently.
- Files to create/modify: none
- Tests to add first: none
- Acceptance criteria:
  - This task remains unimplemented.
  - All modality work is redirected to #90.
- Spec impact: `No separate spec work; see #90`.

#### W1-4: Implement Press interaction state machine in ars-interactions

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #90
- Spec refs:
  - `spec/foundation/05-interactions.md` §2 "Press Interaction" (L115)
  - `spec/foundation/05-interactions.md` §2.2 "Types" (L121)
  - `spec/foundation/05-interactions.md` §2.3 "State Machine" (L393)
  - `spec/foundation/05-interactions.md` §2.5 "Output Props" (L669)
- Goal: implement the full press interaction as a framework-agnostic state machine.
- Files to create/modify: `crates/ars-interactions/src/press.rs` (new), wire into `crates/ars-interactions/src/lib.rs`
- Tests to add first:
  - Unit tests for all state transitions: Idle → Pressing → PressedInside, Idle → Pressing → PressedOutside, PressedInside → PressedOutside (pointer exit), PressedOutside → PressedInside (pointer re-enter), PressedInside → Idle (release = activation), PressedOutside → Idle (release = no activation).
  - Unit tests for disabled config suppressing all transitions.
  - Unit tests for touch scroll cancellation via `scroll_threshold_px`.
  - Unit tests for `current_attrs()` producing `data-ars-pressed` and `data-ars-disabled`.
  - Unit tests for shared modality `global_press_active` integration (set on press start, cleared on press end).
- Acceptance criteria:
  - `PressConfig` with all fields per spec §2.2: `disabled`, `prevent_text_selection`, `allow_press_on_exit`, `scroll_threshold_px`, `on_press_start/end/press/change/up`, `pointer_capture_timeout`, `long_press_cancel_flag`.
  - `PressEvent` with `pointer_type`, `event_type`, `client_x/y`, `modifiers: KeyModifiers`, `is_within_element`, `continue_propagation: Rc<Cell<bool>>`.
  - `PressEventType` enum: `PressStart`, `PressEnd`, `Press`, `PressUp`.
  - `KeyModifiers` struct: `shift`, `ctrl`, `alt`, `meta`.
  - `PressResult` holding current `PressState`, providing `current_attrs() -> AttrMap`.
  - `use_press(config) -> PressResult` factory function.
- Spec impact: `No spec change required`.

#### W1-5: Implement Hover interaction state machine in ars-interactions

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #90
- Spec refs:
  - `spec/foundation/05-interactions.md` §3 "Hover Interaction" (L796)
  - `spec/foundation/05-interactions.md` §3.2 "Types" (L802)
  - `spec/foundation/05-interactions.md` §3.3 "State Machine" (L846)
  - `spec/foundation/05-interactions.md` §3.4 "Integration with Press" (L881)
  - `spec/foundation/05-interactions.md` §3.5 "Output Props" (L946)
- Goal: implement the hover interaction with press integration.
- Files to create/modify: `crates/ars-interactions/src/hover.rs` (new), wire into `crates/ars-interactions/src/lib.rs`
- Tests to add first:
  - Unit tests for NotHovered → Hovered on mouse/pen enter, Hovered → NotHovered on leave.
  - Unit tests for touch and keyboard events being ignored.
  - Unit tests for hover suppression when shared modality `global_press_active` is true.
  - Unit tests for disabled config suppressing transitions.
  - Unit tests for `current_attrs()` producing `data-ars-hovered`.
- Acceptance criteria:
  - `HoverConfig` with `disabled`, `on_hover_start`, `on_hover_end`, `on_hover_change`.
  - `HoverEvent` with `pointer_type` (always Mouse or Pen), `event_type`.
  - `HoverEventType` enum: `HoverStart`, `HoverEnd`.
  - `HoverState` enum: `NotHovered`, `Hovered`; `is_hovered()` method.
  - `HoverResult` with `current_attrs() -> AttrMap` producing `data-ars-hovered`.
  - `use_hover(config) -> HoverResult` factory function.
- Spec impact: `No spec change required`.

#### W1-6: Implement Focus and FocusWithin interactions in ars-interactions

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #90
- Spec refs:
  - `spec/foundation/05-interactions.md` §4 "Focus Interaction" (L995)
  - `spec/foundation/05-interactions.md` §4.2 "Types" (L1011)
  - `spec/foundation/05-interactions.md` §4.3 "State Machine" (L1074)
  - `spec/foundation/05-interactions.md` §4.4 "Focus Visible Detection" (L1135)
  - `spec/foundation/05-interactions.md` §4.5 "Output Props" (L1234)
- Goal: implement focus and focus-within interactions with focus-visible determination.
- Files to create/modify: `crates/ars-interactions/src/focus.rs` (new), update `crates/ars-interactions/src/lib.rs`
- Tests to add first:
  - Unit tests for Unfocused → FocusedByKeyboard when last modality is Keyboard.
  - Unit tests for Unfocused → FocusedByPointer when last modality is Mouse/Touch/Pen.
  - Unit tests for Unfocused → FocusedProgrammatic when no prior modality.
  - Unit tests for `is_focus_visible()` returning true only for keyboard-triggered focus.
  - Unit tests for FocusWithin tracking (child focus propagates to parent container).
  - Unit tests for `current_attrs()` producing `data-ars-focused`, `data-ars-focus-visible`, `data-ars-focus-within`, `data-ars-focus-within-visible`.
- Acceptance criteria:
  - `FocusConfig` with `disabled`, `on_focus`, `on_blur`, `on_focus_visible_change`.
  - `FocusWithinConfig` with `disabled`, `on_focus_within`, `on_blur_within`, `on_focus_within_visible_change`.
  - `FocusEvent` with `event_type`, `pointer_type`.
  - `FocusEventType` enum: `Focus`, `Blur`, `FocusWithin`, `BlurWithin`.
  - Existing `FocusState` extended with `is_focus_visible()` reading `had_pointer_interaction()`.
  - `FocusResult` with `current_attrs() -> AttrMap`.
  - `FocusWithinResult` with `current_attrs() -> AttrMap`.
  - `use_focus(config) -> FocusResult` and `use_focus_within(config) -> FocusWithinResult`.
- Spec impact: `No spec change required`.

#### W1-7: Implement LogicalDirection and resolve_arrow_key in ars-interactions

- Points: `1`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/05-interactions.md` §7.11 "Directional Arrow Key Resolution" (L2415)
- Goal: add RTL-aware arrow key resolution consumed by all keyboard-navigable components.
- Files to create/modify: `crates/ars-interactions/src/direction.rs` (new) or inline in `lib.rs`
- Tests to add first:
  - Unit tests for LTR: ArrowRight → Forward, ArrowLeft → Backward.
  - Unit tests for RTL: ArrowRight → Backward, ArrowLeft → Forward.
  - Unit tests for non-horizontal keys (ArrowUp, ArrowDown) returning `None`.
  - Unit test for `Direction::Auto` triggering debug assertion.
- Acceptance criteria:
  - `LogicalDirection` enum with `Forward`, `Backward`.
  - `resolve_arrow_key(key, direction: Direction) -> Option<LogicalDirection>`.
  - Debug assert on `Direction::Auto` input.
- Spec impact: `No spec change required`.

---

### Wave 2: Collection Infrastructure and Overlay Primitives

**Goal:** Enable Select, Combobox, Menu, Listbox, Dialog, and Popover.

**Depends on:** Wave 1 complete.

**Parallelism:** #62, #65, #66, and #68 can run in parallel. #63 and #64 depend on #62. #67 depends on #66. #112, #113, and #115 depend on #67. #114 depends on #67 and #112. #69 depends on #68.

| GitHub                                               | Title                                                                                       | Points | Epic | Deps      |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------- | ------ | ---- | --------- |
| [#62](https://github.com/fogodev/ars-ui/issues/62)   | Implement Key, NodeType, and Node in ars-collections                                        | 3      | #53  | —         |
| [#63](https://github.com/fogodev/ars-ui/issues/63)   | Implement Collection trait and StaticCollection in ars-collections                          | 5      | #53  | #62       |
| [#64](https://github.com/fogodev/ars-ui/issues/64)   | Implement selection::Mode, Set, and State in ars-collections                                | 5      | #53  | #62       |
| [#65](https://github.com/fogodev/ars-ui/issues/65)   | Implement InteractOutside interaction in ars-interactions                                   | 3      | #4   | Wave 1    |
| [#66](https://github.com/fogodev/ars-ui/issues/66)   | Implement positioning engine types in ars-dom                                               | 3      | #6   | —         |
| [#67](https://github.com/fogodev/ars-ui/issues/67)   | Implement compute_position with flip, shift, and arrow in ars-dom                           | 5      | #6   | #66       |
| [#112](https://github.com/fogodev/ars-ui/issues/112) | Implement viewport measurement and visualViewport support for positioning in ars-dom        | 3      | #6   | #67       |
| [#113](https://github.com/fogodev/ars-ui/issues/113) | Implement containing-block detection and coordinate-space conversion in ars-dom positioning | 5      | #6   | #67       |
| [#114](https://github.com/fogodev/ars-ui/issues/114) | Implement auto_update observer lifecycle for positioned overlays in ars-dom                 | 5      | #6   | #67, #112 |
| [#115](https://github.com/fogodev/ars-ui/issues/115) | Add VirtualElement helper for non-DOM positioning anchors in ars-dom                        | 1      | #6   | #67       |
| [#68](https://github.com/fogodev/ars-ui/issues/68)   | Implement z-index allocator in ars-dom                                                      | 2      | #6   | —         |
| [#69](https://github.com/fogodev/ars-ui/issues/69)   | Implement portal root and background inert utilities in ars-dom                             | 3      | #6   | #68       |

**Total:** 43 points

---

### Wave 2 Task Details

#### W2-1: Implement Key, NodeType, and Node in ars-collections

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/06-collections.md` §1.3 "Key Type" (L99)
  - `spec/foundation/06-collections.md` §1.4 "NodeType" (L214)
  - `spec/foundation/06-collections.md` §1.5 "Node Struct" (L238)
- Goal: implement the core collection data types.
- Files to create/modify: `crates/ars-collections/src/key.rs` (new), `crates/ars-collections/src/node.rs` (new), wire into `crates/ars-collections/src/lib.rs`
- Tests to add first:
  - Unit tests for `Key` ordering: Int variants sort before String variants.
  - Unit tests for `Key::from()` conversions: `&str`, `String`, `u64`, `u32`, `usize`, `i64`.
  - Unit tests for `Key::parse()` attempting int parse before falling back to string.
  - Unit tests for `NodeType` variants: `Item`, `Section`, `Header`, `Separator`.
  - Unit tests for `Node::is_focusable()` (true for Item, false for structural types).
  - Unit tests for `Node::is_structural()`.
- Acceptance criteria:
  - `Key` enum with `String(String)`, `Int(i64)` variants; manual `Ord` (Int < String); `Display`, `From` impls.
  - `Key::str()`, `Key::int()`, `Key::from_database_id()`, `Key::parse()` methods.
  - `NodeType` with `Item`, `Section`, `Header`, `Separator`.
  - `Node<T>` with `key`, `node_type`, `value`, `text_value`, `level`, `has_children`, `is_expanded`, `parent_key`, `index`.
  - Crate remains `no_std` compatible with `alloc`.
- Spec impact: `No spec change required`.

#### W2-2: Implement Collection trait and StaticCollection in ars-collections

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #62
- Spec refs:
  - `spec/foundation/06-collections.md` §1.6 "Collection Trait" (L378)
  - `spec/foundation/06-collections.md` §2.2 "StaticCollection" (L1072)
- Goal: implement the core collection trait and its static (in-memory) implementation.
- Files to create/modify: `crates/ars-collections/src/collection.rs` (new), `crates/ars-collections/src/static_collection.rs` (new)
- Tests to add first:
  - Unit tests for `Collection` trait methods: `iter()`, `get()`, `get_by_key()`, `size()`, `first_key()`, `last_key()`, `key_after()`, `key_before()`.
  - Unit tests for `StaticCollection::from_vec()` auto-generating `Key` from index.
  - Unit tests for `StaticCollection::from_nodes()` with pre-built nodes.
  - Unit tests for key-based navigation: `key_after(last)` returns `None`, `key_before(first)` returns `None`.
- Acceptance criteria:
  - `Collection<T>` trait with all methods from spec.
  - `StaticCollection<T>` implementing `Collection<T>`.
  - Efficient key-based lookup via internal `HashMap<Key, usize>`.
- Spec impact: `No spec change required`.

#### W2-3: Implement selection::Mode, Set, and State in ars-collections

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #62
- Spec refs:
  - `spec/foundation/06-collections.md` §3 "Selection Model" (L1854)
  - `spec/foundation/06-collections.md` §3.1 "Mode and Behavior" (L1856)
  - `spec/foundation/06-collections.md` §3.2 "Set" (L1901)
  - `spec/foundation/06-collections.md` §3.3 "DisabledBehavior" (L1991)
  - `spec/foundation/06-collections.md` §3.4 "State" (L2176)
- Goal: replace the current `Selection<T>` stub with the full selection model.
- Files to create/modify: `crates/ars-collections/src/selection.rs` (new module directory), update `crates/ars-collections/src/lib.rs`
- Tests to add first:
  - Unit tests for `selection::Mode` enum: `None`, `Single`, `Multiple`.
  - Unit tests for `selection::Set` operations: `toggle()`, `select()`, `deselect()`, `select_all()`, `clear()`, `is_selected()`, `contains()`.
  - Unit tests for single-mode enforcing at-most-one selection.
  - Unit tests for `disallow_empty_selection` preventing clear in single mode.
  - Unit tests for `DisabledBehavior::All` vs `DisabledBehavior::Selection`.
  - Unit tests for `selection::State` tracking `selected_keys`, `focused_key`, `anchor_key`.
- Acceptance criteria:
  - `selection::Mode` enum (None, Single, Multiple).
  - `selection::Behavior` struct (mode, disallow_empty_selection).
  - `selection::Set` backed by `BTreeSet<Key>` with full API.
  - `DisabledBehavior` enum (All, Selection).
  - `selection::State` tracking selected keys, focused key, anchor key.
  - Old `Selection<T>` replaced (clean break, since it is unused outside the crate).
- Spec impact: `No spec change required`.

#### W2-4: Implement InteractOutside interaction in ars-interactions

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: Wave 1 complete (#55-#61) (#55-#61)
- Spec refs:
  - `spec/foundation/05-interactions.md` §12 "Interact Outside" (L3958)
- Goal: implement the shared "click outside to close" primitive used by Dialog, Popover, Menu, Select, Combobox.
- Files to create/modify: `crates/ars-interactions/src/interact_outside.rs` (new)
- Tests to add first:
  - Unit tests for `InteractOutsideConfig` defaults.
  - Unit tests for event variant coverage: `PointerOutside`, `FocusOutside`, `EscapeKey`.
  - Unit tests for disabled config suppressing detection.
  - Unit tests for portal-aware detection design using `data-ars-portal-owner`.
- Acceptance criteria:
  - `InteractOutsideConfig` with `disabled`, `detect_focus`.
  - `InteractOutsideStandalone` with `target_id`, `portal_owner_ids`, `on_interact_outside`, `enabled`, `pointer_gracing`.
  - `InteractOutsideEvent` enum with `PointerOutside`, `FocusOutside`, `EscapeKey`.
  - Portal-aware detection design.
- Spec impact: clarify that portal-aware boundary registration uses portal-owner IDs rather than arbitrary element IDs.

#### W2-5: Implement positioning engine types in ars-dom

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/11-dom-utilities.md` §2.2 "Types" (L23)
- Goal: implement the pure-data positioning types consumed by the positioning algorithm.
- Files to create/modify: `crates/ars-dom/src/positioning/mod.rs` (new), `crates/ars-dom/src/positioning/types.rs` (new)
- Tests to add first:
  - Unit tests for `Placement::opposite()` returning the correct opposite for all 21 variants.
  - Unit tests for `Placement::main_axis()` returning correct `Axis`.
  - Unit tests for `Placement::resolve_logical()` with LTR and RTL directions.
  - Unit tests for `Placement::side()` and `Placement::alignment()` decomposition.
- Acceptance criteria:
  - `Side` (Top, Right, Bottom, Left), `Alignment` (Start, Center, End), `Axis` (Horizontal, Vertical).
  - All 21 `Placement` variants including Auto and Logical (Start/End).
  - `Placement` methods: `opposite()`, `main_axis()`, `resolve_logical()`, `side()`, `alignment()`, `side_and_alignment()`, `with_side()`.
  - `PositioningOptions` struct (placement, offset, flip, shift, arrow, boundary).
  - `PositioningResult` struct (x, y, actual_placement, arrow_offset, max size metadata).
- Spec impact: `No spec change required`.

#### W2-6: Implement compute_position with flip, shift, and arrow in ars-dom

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #66
- Spec refs:
  - `spec/foundation/11-dom-utilities.md` §2.3 "Algorithm" (L534)
  - `spec/foundation/11-dom-utilities.md` §2.4 "Core Function Signature" (L736)
  - `spec/foundation/11-dom-utilities.md` §2.5 "Overflow Detection and Helpers" (L850)
  - `spec/foundation/11-dom-utilities.md` §2.7 "Auto Placement" (L977)
- Goal: implement the core positioning algorithm with flip, shift, and arrow middleware.
- Files to create/modify: `crates/ars-dom/src/positioning/compute.rs` (new), `crates/ars-dom/src/positioning/overflow.rs` (new)
- Tests to add first:
  - Unit tests for basic positioning (top, bottom, left, right center placements) with synthetic Rect values.
  - Unit tests for flip: overflow on primary side triggers opposite placement.
  - Unit tests for shift: floating element clamped to boundary with configurable padding.
  - Unit tests for arrow: cross-axis offset computed correctly.
  - Unit tests for auto placement: picks side with most available space.
- Acceptance criteria:
  - `compute_position(anchor, floating, viewport, options) -> PositioningResult`.
  - Flip middleware: detects overflow, tries opposite, falls back to adjacent.
  - Shift middleware: clamps to boundary with configurable padding.
  - Arrow middleware: computes cross-axis offset.
  - Auto placement: tries all sides, picks one with most space.
- Spec impact: `No spec change required`.

#### W2-6b: Implement viewport measurement and visualViewport support for positioning in ars-dom

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Mixed`
- Depends on: #67
- Spec refs:
  - `spec/foundation/11-dom-utilities.md` §2.2.2 "Visual Viewport and Virtual Keyboard" (L468)
  - `spec/foundation/11-dom-utilities.md` §2.3.2 "Step 1: Measure" (L627)
  - `spec/foundation/11-dom-utilities.md` §2.8 "Coordinate System" (L1045)
- Goal: implement viewport measurement helpers and visual viewport integration for DOM-backed positioning.
- Files to create/modify: `crates/ars-dom/src/positioning/viewport.rs` (new), `crates/ars-dom/src/positioning/mod.rs`, `crates/ars-dom/src/lib.rs`, `crates/ars-dom/Cargo.toml`
- Tests to add first:
  - Unit tests for non-web fallback viewport measurements.
  - Web-targeted smoke tests for `window.innerWidth` / `window.innerHeight` fallback behavior.
  - Web-targeted smoke tests for `visualViewport` width, height, and offset handling when available.
  - Tests proving `viewport_rect()` reflects the visual viewport when the browser exposes it.
- Acceptance criteria:
  - Viewport helpers expose the dimensions needed by the positioning spec.
  - `visualViewport` support is used when available and falls back cleanly otherwise.
  - Host builds compile without requiring browser globals.
  - The public helpers are documented as the measurement layer for DOM positioning.
- Spec impact: `No spec change required`.

#### W2-6c: Implement containing-block detection and coordinate-space conversion in ars-dom positioning

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Mixed`
- Depends on: #67
- Spec refs:
  - `spec/foundation/11-dom-utilities.md` §2.3.1 "Step 0: Detect Containing Block" (L539)
  - `spec/foundation/11-dom-utilities.md` §2.8 "Coordinate System" (L1045)
  - `spec/foundation/11-dom-utilities.md` §2.8.1 "CSS Transform Ancestor Detection" (L1074)
  - `spec/foundation/11-dom-utilities.md` §6.6 "CSS Containment Interaction" (L2434)
- Goal: implement DOM coordinate-space detection and conversion for positioned overlays rendered in transformed or contained ancestors.
- Files to create/modify: `crates/ars-dom/src/positioning/dom.rs` (new), `crates/ars-dom/src/positioning/mod.rs`, `crates/ars-dom/src/lib.rs`, `crates/ars-dom/Cargo.toml`
- Tests to add first:
  - Unit tests for containing-block detection from computed style flags.
  - Unit tests for `position: absolute` offset-parent coordinate conversion.
  - Web-targeted smoke tests for transformed ancestors and CSS containment.
  - Tests proving host builds stub or fall back safely when DOM APIs are unavailable.
- Acceptance criteria:
  - The DOM positioning layer can detect containing-block ancestors per spec.
  - Coordinate conversion accounts for transformed and contained ancestors.
  - `absolute` strategy conversion uses the correct offset parent space.
  - Portal-target mismatches are surfaced in the documented way.
- Spec impact: `No spec change required`.

#### W2-6d: Implement auto_update observer lifecycle for positioned overlays in ars-dom

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Mixed`
- Depends on: #67, #112
- Spec refs:
  - `spec/foundation/11-dom-utilities.md` §2.2.1 "ResizeObserver Lifecycle for Positioning" (L446)
  - `spec/foundation/11-dom-utilities.md` §2.2.2 "Visual Viewport and Virtual Keyboard" (L468)
  - `spec/foundation/11-dom-utilities.md` §2.10 "Auto-Update" (L1095)
  - `spec/foundation/11-dom-utilities.md` §4.2 "Scrollable Ancestor Detection" (listener usage at L1696)
- Goal: implement the browser-facing observer and listener lifecycle that keeps positioned overlays updated as layout changes.
- Files to create/modify: `crates/ars-dom/src/positioning/auto_update.rs` (new), `crates/ars-dom/src/positioning/mod.rs`, `crates/ars-dom/src/lib.rs`, `crates/ars-dom/Cargo.toml`
- Tests to add first:
  - Unit tests for non-web/SSR no-op cleanup behavior.
  - Unit tests for cleanup idempotency and teardown sequencing.
  - Web-targeted smoke tests for window resize and ancestor scroll listener wiring.
  - Web-targeted smoke tests for `visualViewport` listener registration and cleanup.
- Acceptance criteria:
  - `auto_update(anchor, floating, update) -> Box<dyn FnOnce()>` is implemented per spec.
  - Resize, scroll, mutation, intersection, and `visualViewport` hooks are wired as specified.
  - Cleanup removes every observer and listener and is safe in host builds.
  - RAF batching remains an adapter responsibility, matching the spec split.
- Spec impact: `No spec change required`.

#### W2-6e: Add VirtualElement helper for non-DOM positioning anchors in ars-dom

- Points: `1`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #67
- Spec refs:
  - `spec/foundation/11-dom-utilities.md` §2.6 "Virtual Elements" (L956)
- Goal: add the `VirtualElement` helper used by the positioning API for non-DOM anchors.
- Files to create/modify: `crates/ars-dom/src/positioning/types.rs`, `crates/ars-dom/src/positioning/mod.rs`, `crates/ars-dom/src/lib.rs`
- Tests to add first:
  - Unit tests proving `VirtualElement` can return a synthetic `Rect` through its callback.
  - Unit tests proving repeated calls may return different rects, matching manual recomputation semantics.
  - Public API tests covering exports from `positioning` and the crate root.
- Acceptance criteria:
  - `VirtualElement` exists exactly as specified for geometry-only anchors.
  - The helper is documented as caller-driven for recomputation.
  - The type is publicly exported through the positioning module and crate root.
- Spec impact: `No spec change required`.

#### W2-7: Implement z-index allocator in ars-dom

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/11-dom-utilities.md` §6 "Z-Index Management" (L2226)
  - `spec/foundation/11-dom-utilities.md` §6.2 "Strategy" (L2232)
  - `spec/foundation/11-dom-utilities.md` §6.3 "Usage Pattern" (L2331)
- Goal: implement the monotonic z-index allocator for stacking overlay elements.
- Files to create/modify: `crates/ars-dom/src/z_index.rs` (new)
- Tests to add first:
  - Unit tests for monotonic allocation: successive `allocate()` calls return increasing values.
  - Unit tests for `release()` behavior.
  - Unit tests for configurable base z-index.
- Acceptance criteria:
  - Thread-local monotonic counter.
  - `allocate() -> u32` returning next z-index.
  - `release(z_index)` for cleanup.
  - Base z-index configurable.
- Spec impact: `No spec change required`.

#### W2-8: Implement portal root and background inert utilities in ars-dom

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #68
- Spec refs:
  - `spec/foundation/11-dom-utilities.md` §7 "Portal Root and Background Inert" (L2430)
- Goal: implement portal container management and background inert utilities for modal overlays.
- Files to create/modify: `crates/ars-dom/src/portal.rs` (new)
- Tests to add first:
  - Unit tests for `get_or_create_portal_root()` idempotency.
  - Unit tests for `set_background_inert()` / `restore_background()` toggle behavior.
  - Unit tests for `data-ars-portal-owner` attribute placement.
- Acceptance criteria:
  - `get_or_create_portal_root()` returns or creates a DOM container for portaled content.
  - `set_background_inert(portal_container)` sets `inert` attribute on siblings.
  - `restore_background()` removes `inert` attributes.
  - Portal container marked with `data-ars-portal-owner` attribute.
- Spec impact: `No spec change required`.

---

### Wave 3: Extended Foundations

**Goal:** Enable Dialog, Tooltip, and DatePicker prerequisites.

**Depends on:** Wave 2 complete.

**Parallelism:** #70, #71, #73, #74, #75, #88, and #90 can run in parallel. #89 depends on #90. #72 depends on #90 and #89.

| GitHub                                             | Title                                                                       | Points | Epic | Deps     |
| -------------------------------------------------- | --------------------------------------------------------------------------- | ------ | ---- | -------- |
| [#70](https://github.com/fogodev/ars-ui/issues/70) | Implement type-ahead / type-select state machine in ars-collections         | 3      | #53  | #63      |
| [#71](https://github.com/fogodev/ars-ui/issues/71) | Implement CollectionBuilder in ars-collections                              | 3      | #53  | #62, #63 |
| [#90](https://github.com/fogodev/ars-ui/issues/90) | Introduce shared ModalityContext in ars-core and migrate modality contracts | 5      | #4   | —        |
| [#89](https://github.com/fogodev/ars-ui/issues/89) | Integrate FocusRing with shared modality event stream in ars-a11y           | 2      | #3   | #90      |
| [#72](https://github.com/fogodev/ars-ui/issues/72) | Implement web ModalityManager listener ownership in ars-dom                 | 2      | #6   | #90, #89 |
| [#88](https://github.com/fogodev/ars-ui/issues/88) | Implement overlay stack registry for nested overlay dismissal in ars-dom    | 3      | #6   | #68, #69 |
| [#73](https://github.com/fogodev/ars-ui/issues/73) | Implement LiveAnnouncer for screen reader announcements                     | 3      | #3   | —        |
| [#74](https://github.com/fogodev/ars-ui/issues/74) | Implement scroll_into_view_if_needed with Safari fallback in ars-dom        | 3      | #6   | —        |
| [#75](https://github.com/fogodev/ars-ui/issues/75) | Replace ars-i18n Locale stub with ICU4X-backed implementation               | 5      | #54  | —        |

**Total:** 29 points

---

### Wave 3 Task Details

#### W3-1: Implement type-ahead / type-select state machine in ars-collections

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #63
- Spec refs:
  - `spec/foundation/06-collections.md` §4 "Type-Ahead / Type-Select" (L2585)
  - `spec/foundation/06-collections.md` §4.1 "State" (L2589)
  - `spec/foundation/06-collections.md` §4.2 "Integration in a Component's Event Handler" (L2801)
- Goal: implement type-ahead state machine for keyboard search in collection components.
- Files to create/modify: `crates/ars-collections/src/type_ahead.rs` (new)
- Tests to add first:
  - Unit tests for character accumulation: typing "a" then "b" produces "ab" search.
  - Unit tests for timeout clearing the search buffer (configurable, default 500ms).
  - Unit tests for `find_match()` finding next matching node by text_value prefix.
  - Unit tests for match cycling: repeated same-character input cycles through matches.
- Acceptance criteria:
  - `TypeAheadState` with search buffer and timeout tracking.
  - `on_keypress(char)` appends to buffer and resets timeout.
  - `find_match(collection, from_key)` finds next matching node.
  - Configurable timeout (default 500ms).
- Spec impact: `No spec change required`.

#### W3-2: Implement CollectionBuilder in ars-collections

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #62, #63
- Spec refs:
  - `spec/foundation/06-collections.md` §2.1 "CollectionBuilder" (L902)
- Goal: implement the builder pattern for declarative collection construction.
- Files to create/modify: `crates/ars-collections/src/builder.rs` (new)
- Tests to add first:
  - Unit tests for flat list construction via `item()` calls.
  - Unit tests for sectioned lists with `section()` and nested `item()` calls.
  - Unit tests for automatic key generation for unnamed items.
  - Unit tests for key uniqueness enforcement (duplicate key rejection).
  - Unit tests for `build()` producing a valid `StaticCollection<T>`.
- Acceptance criteria:
  - `CollectionBuilder<T>` with `item()`, `section()`, `header()`, `separator()` methods.
  - `build() -> StaticCollection<T>`.
  - Automatic key generation for unnamed items.
  - Level tracking for nested sections.
- Spec impact: `No spec change required`.

#### W3-3core: Introduce shared ModalityContext in ars-core and migrate modality contracts

- Points: `5`
- Layer: `Core`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/01-architecture.md` §2.2.7 "PlatformEffects Trait" and §6.4 "ArsProvider"
  - `spec/foundation/05-interactions.md` §3.4 "Integration with Press"
  - `spec/foundation/05-interactions.md` §4.4 "Focus Visible Detection: Shared Modality Tracking"
- Goal: replace the old `ars-interactions` thread-local design with the shared provider-scoped `ModalityContext` contract in `ars-core`.
- Files to create/modify: `crates/ars-core/src/modality.rs` (new), `crates/ars-core/src/lib.rs`, `crates/ars-core/src/provider.rs`, `crates/ars-interactions/src/lib.rs`, `crates/ars-interactions/Cargo.toml`
- Tests to add first:
  - Unit tests for `DefaultModalityContext` startup state and per-instance isolation.
  - Unit tests for keyboard, pointer, and virtual modality updates.
  - Unit tests for `set_global_press_active()` / `is_global_press_active()` semantics.
  - Unit tests for `FocusState::is_focus_visible()` consulting injected modality instead of ambient globals.
- Acceptance criteria:
  - `ars-core` exposes `PointerType`, `KeyModifiers`, `KeyboardKey`, `ModalitySnapshot`, `ModalityContext`, `DefaultModalityContext`, and `NullModalityContext`.
  - Modality state is instance-scoped and provider-friendly, not thread-local.
  - `ArsContext` exposes a shared `Rc<dyn ModalityContext>` alongside `PlatformEffects`.
  - `ars-interactions` re-exports the shared modality types and no longer owns DOM listener installation.
- Spec impact: `Update architecture/interactions/provider specs in the same task`.

#### W3-3a: Integrate FocusRing with shared modality event stream in ars-a11y

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #90
- Spec refs:
  - `spec/foundation/03-accessibility.md` §3.4 "FocusRing" (L1591)
  - `spec/foundation/05-interactions.md` §4.4 "Focus Visible Detection: Shared Modality Tracking" (L1135)
- Goal: align `FocusRing` with the shared modality event stream without making it responsible for platform listener ownership.
- Files to create/modify: `crates/ars-a11y/src/focus.rs`, `crates/ars-a11y/src/lib.rs`
- Tests to add first:
  - Unit tests for `on_pointer_down()` clearing keyboard modality.
  - Unit tests for `on_key_down()` enabling keyboard modality only for navigation keys.
  - Unit tests for ctrl/meta/alt-modified key chords being ignored.
  - Unit tests for `on_virtual_input()` and `apply_focus_attrs()` semantics.
- Acceptance criteria:
  - `FocusRing` consumes the same normalized key/pointer/virtual events as `ModalityContext`.
  - `FocusRing` remains separate from `PlatformEffects` and DOM listener installation.
  - `should_show_focus_ring()` and `apply_focus_attrs()` match the updated spec contract.
- Spec impact: `Keep accessibility spec aligned with the shared-modality architecture`.

#### W3-3b: Implement web ModalityManager listener ownership in ars-dom

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #90, #89
- Spec refs:
  - `spec/foundation/11-dom-utilities.md` §8 "Modality Manager" (L2531)
  - `spec/foundation/05-interactions.md` §4.4 "Focus Visible Detection: Shared Modality Tracking" (L1135)
  - `spec/foundation/03-accessibility.md` §3.4 "FocusRing" (L1591)
- Goal: implement the browser listener layer that keeps `ModalityContext` and `FocusRing` synchronized through a single adapter-facing API.
- Files to create/modify: `crates/ars-dom/src/modality.rs` (new), `crates/ars-dom/src/lib.rs` (wire new module), `crates/ars-dom/Cargo.toml`
- Tests to add first:
  - Unit tests for `on_key_down()` updating both `ModalityContext` and `FocusRing`.
  - Unit tests for `on_pointer_down()` updating both trackers atomically.
  - Unit tests for `on_virtual_input()` semantics.
  - Unit tests for refcounted listener install/remove transitions and host-build safety.
- Acceptance criteria:
  - `ModalityManager` holds `Rc<dyn ModalityContext>` plus `FocusRing`.
  - `on_key_down(key, modifiers)`, `on_pointer_down(pointer_type)`, and `on_virtual_input()` fan out to both consumers.
  - `ensure_listeners()` / `remove_listeners()` own browser listener installation and cleanup.
  - Browser listener lifecycle is refcounted, document-guarded, and no-op outside web DOM environments.
  - `focus_ring(&self) -> &FocusRing`.
  - Adapter-facing API matches the spec and replaces direct tracker updates.
- Spec impact: `No spec change required beyond the shared-modality migration`.

#### W3-3c: Implement overlay stack registry for nested overlay dismissal in ars-dom

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #68, #69
- Spec refs:
  - `spec/foundation/05-interactions.md` §12.8 "Nested Overlay Handling" (L4105)
  - `spec/foundation/11-dom-utilities.md` §6.3 "Usage Pattern" (L2331)
  - `spec/foundation/11-dom-utilities.md` §6.3.1 "Z-Index Ranges and Adapter Scope" (L2355)
- Goal: implement the overlay stack registry used to determine topmost overlay ordering and nested overlay dismissal behavior.
- Files to create/modify: `crates/ars-dom/src/overlay_stack.rs` (new), `crates/ars-dom/src/lib.rs` (wire new module)
- Tests to add first:
  - Unit tests for push/pop overlay stack operations.
  - Unit tests for modal vs non-modal distinction.
  - Unit tests for topmost overlay detection.
  - Unit tests for LIFO close ordering.
  - Unit tests for nested overlay scenarios where child overlays suppress parent dismissal.
- Acceptance criteria:
  - Global overlay stack registration and deregistration API.
  - Modal vs non-modal overlay metadata.
  - Topmost overlay lookup for outside-interaction and Escape-key dismissal.
  - Nested overlay ordering consistent with monotonic z-index allocation.
- Spec impact: `No spec change required`.

#### W3-4: Implement LiveAnnouncer for screen reader announcements

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/03-accessibility.md` §5.1 (live region announcements)
- Goal: implement the shared `LiveAnnouncer` for dynamic screen reader announcements.
- Files to create/modify: `crates/ars-a11y/src/announcer.rs` (new, no_std trait), `crates/ars-dom/src/announcer.rs` (new, DOM implementation)
- Tests to add first:
  - Unit tests for `announce(message, politeness)` with `Polite` and `Assertive` levels.
  - Unit tests for message clearing after configurable timeout.
  - Unit tests for announcement queuing behavior.
- Acceptance criteria:
  - `LiveAnnouncer` trait in ars-a11y (no_std compatible).
  - `announce(message, politeness)` and `announce_assertive(message)` methods.
  - `AriaLive::Polite` and `AriaLive::Assertive` support.
  - DOM implementation in ars-dom creating hidden div with `aria-live` attribute.
- Spec impact: `No spec change required`.

#### W3-5: Implement scroll_into_view_if_needed with Safari fallback in ars-dom

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/11-dom-utilities.md` §4.1 "Scroll Into View" (L1584)
  - `spec/foundation/11-dom-utilities.md` §4.2 "Scrollable Ancestor Detection" (L1681)
- Goal: implement cross-browser scroll-into-view with Safari workaround.
- Files to create/modify: `crates/ars-dom/src/scroll.rs` (extend existing or new file)
- Tests to add first:
  - Unit tests for `supports_scroll_into_view_options()` feature detection logic.
  - Unit tests for `nearest_scrollable_ancestor()` traversal.
  - Unit tests for manual fallback handling vertical and horizontal scrolling.
- Acceptance criteria:
  - `supports_scroll_into_view_options()` feature detection.
  - `scroll_into_view_if_needed(element, options)` with native and fallback paths.
  - `nearest_scrollable_ancestor(element)` utility.
  - Nested scrollable container support.
- Spec impact: `No spec change required`.

#### W3-6: Replace ars-i18n Locale stub with ICU4X-backed implementation

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/04-internationalization.md` §2 (Locale type, ~L60-210)
- Goal: replace the `Locale(String)` stub with an ICU4X-backed implementation.
- Files to create/modify: `crates/ars-i18n/src/locale.rs` (new or rewrite `lib.rs`), update `Cargo.toml` for ICU4X deps
- Tests to add first:
  - Unit tests for `Locale::parse()` accepting valid BCP 47 strings and rejecting invalid ones.
  - Unit tests for `direction()` returning `Rtl` for Arabic (`ar`), Hebrew (`he`), etc.
  - Unit tests for `is_rtl()` convenience method.
  - Unit tests for `to_bcp47()`, `language()`, `script()`, `region()` accessors.
  - Unit tests for `script_or_default()` inference logic.
- Acceptance criteria:
  - `Locale` wraps `icu::locale::Locale`.
  - `Locale::parse()` returns `Result<Self, LocaleParseError>`.
  - `direction()` correctly returns `Rtl` for RTL scripts.
  - `RTL_SCRIPTS` constant matching spec.
  - BiDi isolation utilities (`wrap_bidi_isolate()`).
  - `no_std` + `alloc` compatible with `icu4x` feature flag.
- Spec impact: `No spec change required`.

---

### Wave 4: Remaining Foundation

**Goal:** Complete all foundation primitives so that every component in the catalog can be implemented without touching foundation crates.

**Depends on:** Wave 3 complete.

**Note:** Tasks sized at `8` points (#78, #80, #82) must be decomposed into ≤5-point subtasks before pickup.

| GitHub                                             | Title                                                                | Points | Epic | Deps     |
| -------------------------------------------------- | -------------------------------------------------------------------- | ------ | ---- | -------- |
| [#76](https://github.com/fogodev/ars-ui/issues/76) | Implement LongPress interaction in ars-interactions                  | 3      | #4   | Wave 1   |
| [#77](https://github.com/fogodev/ars-ui/issues/77) | Implement Move interaction in ars-interactions                       | 3      | #4   | Wave 1   |
| [#78](https://github.com/fogodev/ars-ui/issues/78) | Implement Drag and Drop interactions in ars-interactions             | 8      | #4   | #58, #76 |
| [#79](https://github.com/fogodev/ars-ui/issues/79) | Implement NumberFormatter trait with ICU4X backend in ars-i18n       | 5      | #54  | #75      |
| [#80](https://github.com/fogodev/ars-ui/issues/80) | Implement DateFormatter and calendar system support in ars-i18n      | 8      | #54  | #75      |
| [#81](https://github.com/fogodev/ars-ui/issues/81) | Implement AsyncCollection with pagination support in ars-collections | 5      | #53  | #63      |
| [#82](https://github.com/fogodev/ars-ui/issues/82) | Implement Virtualizer for large collection rendering                 | 8      | #53  | #63      |
| [#83](https://github.com/fogodev/ars-ui/issues/83) | Implement TreeCollection in ars-collections                          | 5      | #53  | #63      |
| [#84](https://github.com/fogodev/ars-ui/issues/84) | Implement FilteredCollection and SortedCollection in ars-collections | 3      | #53  | #63      |
| [#85](https://github.com/fogodev/ars-ui/issues/85) | Implement media query utilities in ars-dom                           | 2      | #6   | —        |

**Total:** 50 points

---

### Wave 4 Task Details

#### W4-1: Implement LongPress interaction in ars-interactions

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: Wave 1 complete (#55-#61)
- Spec refs:
  - `spec/foundation/05-interactions.md` §5 "Long Press Interaction" (L1349)
  - `spec/foundation/05-interactions.md` §5.3 "Types" (L1366)
  - `spec/foundation/05-interactions.md` §5.4 "State Machine" (L1442)
  - `spec/foundation/05-interactions.md` §5.5 "Accessibility Integration" (L1494)
- Goal: implement the long-press interaction with timing and accessibility integration.
- Files to create/modify: `crates/ars-interactions/src/long_press.rs` (new)
- Tests to add first:
  - Unit tests for state transitions: Idle → Waiting → LongPressed.
  - Unit tests for timing threshold (default 500ms).
  - Unit tests for cancellation before threshold.
  - Unit tests for accessibility announcements on long-press trigger.
- Acceptance criteria:
  - `LongPressConfig` with `disabled`, `threshold`, `on_long_press_start`, `on_long_press`, `accessibility_description`.
  - `LongPressEvent` with `pointer_type`, `event_type`.
  - State machine: Idle → Waiting → LongPressed with configurable threshold.
  - Cross-interaction cancellation protocol with Press (via `long_press_cancel_flag`).
- Spec impact: `No spec change required`.

#### W4-2: Implement Move interaction in ars-interactions

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: Wave 1 complete (#55-#61)
- Spec refs:
  - `spec/foundation/05-interactions.md` §6 "Move Interaction" (L1567)
  - `spec/foundation/05-interactions.md` §6.2 "Types" (L1575)
  - `spec/foundation/05-interactions.md` §6.3 "State Machine" (L1630)
  - `spec/foundation/05-interactions.md` §6.4 "Keyboard Arrow Key Deltas" (L1672)
  - `spec/foundation/05-interactions.md` §6.6 "Output Props" (L1739)
- Goal: implement the move interaction for slider handles, range selectors, and splitters.
- Files to create/modify: `crates/ars-interactions/src/move_interaction.rs` (new)
- Tests to add first:
  - Unit tests for state transitions: Idle → Moving → Idle.
  - Unit tests for delta computation from pointer events.
  - Unit tests for keyboard arrow key delta conversion.
  - Unit tests for CSS zoom/scale coordinate transformation.
- Acceptance criteria:
  - `MoveConfig` with `disabled`, `on_move_start`, `on_move`, `on_move_end`.
  - `MoveEvent` with `delta_x`, `delta_y`, `pointer_type`.
  - Keyboard arrow key delta conversion per §6.4.
  - CSS zoom/scale coordinate transformation per §6.5.
- Spec impact: `No spec change required`.

#### W4-3: Implement Drag and Drop interactions in ars-interactions

- Points: `8` (must be decomposed before pickup)
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #58, #76
- Spec refs:
  - `spec/foundation/05-interactions.md` §7 "Drag and Drop" (L1807)
  - `spec/foundation/05-interactions.md` §7.2 "Item Types" (L1830)
  - `spec/foundation/05-interactions.md` §7.3 "Drop Operation" (L1883)
  - `spec/foundation/05-interactions.md` §7.4 "Drag Source Configuration" (L1916)
  - `spec/foundation/05-interactions.md` §7.5 "Drop Target Configuration" (L1968)
  - `spec/foundation/05-interactions.md` §7.6 "Drag State Machine" (L2075)
  - `spec/foundation/05-interactions.md` §7.7 "Keyboard Drag and Drop Protocol" (L2137)
  - `spec/foundation/05-interactions.md` §7.8 "Screen Reader DnD Announcements" (L2167)
- Goal: implement full drag-and-drop state machines for both source and target sides.
- Acceptance criteria:
  - `DragItem`, `DropOperation`, `DragConfig`, `DropTargetConfig`, `DragEvent`, `DropEvent`.
  - Drag source state machine: Idle → DragPreview → Dragging → Dropped/Cancelled.
  - Drop target state machine: Idle → DragOver → Dropped.
  - Keyboard drag-and-drop protocol.
  - Screen reader announcements during drag.
- Spec impact: `No spec change required`.

#### W4-4: Implement NumberFormatter trait with ICU4X backend in ars-i18n

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #75
- Spec refs:
  - `spec/foundation/04-internationalization.md` (number formatting sections)
- Goal: implement locale-aware number formatting using ICU4X.
- Files to create/modify: `crates/ars-i18n/src/number.rs` (new)
- Tests to add first:
  - Unit tests for formatting integers and decimals in en-US locale.
  - Unit tests for grouping separators (e.g., "1,234.56" in en-US, "1.234,56" in de-DE).
  - Unit tests for parsing formatted numbers back to numeric values.
- Acceptance criteria:
  - `NumberFormatter` trait with `format()`, `parse()`, `separator()` methods.
  - ICU4X-backed implementation for locale-aware formatting.
  - Support for decimal, percent, and currency formatting modes.
- Spec impact: `No spec change required`.

#### W4-5: Implement DateFormatter and calendar system support in ars-i18n

- Points: `8` (must be decomposed before pickup)
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #75
- Spec refs:
  - `spec/foundation/04-internationalization.md` (date formatting and calendar sections)
  - `spec/shared/date-time-types.md`
- Goal: implement locale-aware date/time formatting with multiple calendar system support.
- Acceptance criteria:
  - `DateFormatter` trait with locale-aware formatting.
  - Calendar system support: Gregorian, Islamic, Hebrew, Japanese, Buddhist.
  - `CalendarDate` backed by ICU4X types instead of placeholder struct.
  - `Time` and `DateRange` backed by real types.
- Spec impact: `No spec change required`.

#### W4-6: Implement AsyncCollection with pagination support in ars-collections

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #63
- Spec refs:
  - `spec/foundation/06-collections.md` §5 "Async Loading" (L2829)
  - `spec/foundation/06-collections.md` §5.1 "AsyncLoadingState" (L2833)
  - `spec/foundation/06-collections.md` §5.2 "AsyncCollection" (L2877)
  - `spec/foundation/06-collections.md` §5.3 "Infinite Scroll with Sentinel" (L3151)
- Goal: implement async collection support for server-driven pagination.
- Files to create/modify: `crates/ars-collections/src/async_collection.rs` (new)
- Acceptance criteria:
  - `AsyncLoadingState` enum (Idle, Loading, LoadingMore, Error, Filtering).
  - `AsyncCollection<T>` implementing `Collection<T>` with pagination.
  - `AsyncLoader` trait and `LoadResult`.
  - Error recovery and cancellation semantics.
- Spec impact: `No spec change required`.

#### W4-7: Implement Virtualizer for large collection rendering

- Points: `8` (must be decomposed before pickup)
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #63
- Spec refs:
  - `spec/foundation/06-collections.md` §6 "Virtualization" (L3199)
  - `spec/foundation/06-collections.md` §6.1 "Layout Strategies" (L3209)
  - `spec/foundation/06-collections.md` §6.3 "Virtualizer Struct" (L3302)
  - `spec/foundation/06-collections.md` §6.4 "Virtualizer Integration API" (L3765)
- Goal: implement virtualized rendering for collections with >1000 items.
- Acceptance criteria:
  - `Virtualizer` struct with layout strategy.
  - `VirtualLayout` trait for fixed-height and variable-height strategies.
  - Visible range calculation based on scroll position.
  - Keyboard navigation with virtualization.
  - Scroll position maintenance.
- Spec impact: `No spec change required`.

#### W4-8: Implement TreeCollection in ars-collections

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #63
- Spec refs:
  - `spec/foundation/06-collections.md` §2.3 "Tree Collection" (L1294)
- Goal: implement hierarchical tree collection for tree-view components.
- Files to create/modify: `crates/ars-collections/src/tree.rs` (new)
- Acceptance criteria:
  - `TreeCollection<T>` implementing `Collection<T>` with parent-child relationships.
  - Expand/collapse operations with optimized subtree handling.
  - Depth-first iteration with level tracking.
  - Parent key navigation.
- Spec impact: `No spec change required`.

#### W4-9: Implement FilteredCollection and SortedCollection in ars-collections

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #63
- Spec refs:
  - `spec/foundation/06-collections.md` (filtering and sorting sections)
- Goal: implement filtered and sorted collection wrappers for table and filtered list use cases.
- Files to create/modify: `crates/ars-collections/src/filtered.rs` (new), `crates/ars-collections/src/sorted.rs` (new)
- Acceptance criteria:
  - `FilteredCollection<T>` wrapping a `Collection<T>` with predicate-based filtering.
  - `SortedCollection<T>` wrapping a `Collection<T>` with comparator-based sorting.
  - Both maintain key stability across filter/sort changes.
- Spec impact: `No spec change required`.

#### W4-10: Implement media query utilities in ars-dom

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/11-dom-utilities.md` (media query sections)
- Goal: implement responsive design utilities for viewport and preference queries.
- Files to create/modify: `crates/ars-dom/src/media_query.rs` (new)
- Acceptance criteria:
  - `prefers_reduced_motion()` query.
  - `prefers_color_scheme()` query.
  - Viewport dimension queries.
  - SSR-safe defaults when no window is available.
- Spec impact: `No spec change required`.

---

## Dependency Graph

```diagram
Wave 1 (19 pts)
  #55 (leptos attrs, 3)  ──────────────────────────────────┐
  #56 (dioxus attrs, 3)  ──────────────────────────────────┤
  #61 (arrow keys, 1)    ──────────────────────────────────┤
  #57 (superseded, 2)                                      │
  #90 (shared modality, 5)  ───────────────────────────────┤
    ├─→ #58 (press, 5)   ──────────────────────────────────┤
    ├─→ #59 (hover, 2)   ──────────────────────────────────┤
    └─→ #60 (focus, 3)   ──────────────────────────────────┤
                                                           ▼
Wave 2 (43 pts)                              ┌─── Wave 1 complete
  #62 (key/node, 3)                          │
    ├─→ #63 (collection trait, 5)            │
    └─→ #64 (selection, 5)                   │
  #65 (interact outside, 3)  ◄───────────────┘
  #66 (positioning types, 3)
    └─→ #67 (positioning algo, 5)
         ├─→ #112 (viewport, 3)
         │    └─→ #114 (auto_update, 5)
         ├─→ #113 (containing block, 5)
         └─→ #115 (virtual element, 1)
  #68 (z-index, 2)
    └─→ #69 (portal/inert, 3)
                            │
                            ▼
Wave 3 (25 pts)            ┌─── Wave 2 complete
  #70 (type-ahead, 3)      │
  #71 (builder, 3)         │
  #89 (focus ring, 3)      │
  #72 (modality mgr, 2)    │
  #88 (overlay stack, 3)   │
  #73 (announcer, 3)       │
  #74 (scroll view, 3)     │
  #75 (ICU4X locale, 5)    │
                           ▼
Wave 4 (50 pts)            ┌─── Wave 3 complete
  #76 (long press, 3)      │
  #77 (move, 3)            │
  #78 (drag/drop, 8*)      │  * decompose before pickup
  #79 (number fmt, 5)      │
  #80 (date fmt, 8*)       │  * decompose before pickup
  #81 (async collection, 5)│
  #82 (virtualizer, 8*)    │  * decompose before pickup
  #83 (tree collection, 5) │
  #84 (filter/sort, 3)     │
  #85 (media query, 2)     │
                           ▼
            ┌─── All waves complete
            │
            ▼
    Decompose #24 → Component work begins
```

## Epic Mapping

| Epic                | Issue | Tasks covered                                                  |
| ------------------- | ----- | -------------------------------------------------------------- |
| Interactions        | #4    | #57, #58, #59, #60, #61, #65, #76, #77, #78, #90               |
| DOM utilities       | #6    | #66, #67, #68, #69, #72, #74, #85, #88, #112, #113, #114, #115 |
| Leptos adapter      | #8    | #55, #105                                                      |
| Dioxus adapter      | #9    | #56, #106                                                      |
| A11y                | #3    | #73, #89                                                       |
| Collections         | #53   | #62, #63, #64, #70, #71, #81, #82, #83, #84                    |
| I18n                | #54   | #75, #79, #80                                                  |
| First utility slice | #10   | #104                                                           |

## Post-Foundation Plan

After all four waves are complete:

1. Close or supersede `#24` ("Break the first utility slice into agent-ready delivery cards").
2. Decompose the first utility slice (Button, VisuallyHidden, Separator, FocusScope, Toggle, Field, Form) into agent-ready component tasks.
3. Component work can proceed in parallel across both adapters without foundation merge conflicts.
4. Each component task references the now-stable foundation APIs by crate path and spec section.

### Targeted Follow-On: Full Outside-Interaction Delivery

These cards intentionally carve out the remaining work needed to turn the foundation-level
`InteractOutside` substrate into a full shared `Dismissable` primitive before the broader
utility-slice decomposition resumes. They are narrowly scoped to the outside-interaction
pipeline and do not reopen the full `#24` planning thread.

| GitHub                                               | Title                                                                         | Points | Epic | Deps                |
| ---------------------------------------------------- | ----------------------------------------------------------------------------- | ------ | ---- | ------------------- |
| [#104](https://github.com/fogodev/ars-ui/issues/104) | Implement Dismissable core props and dismiss-button attrs in ars-interactions | 3      | #10  | #65                 |
| [#105](https://github.com/fogodev/ars-ui/issues/105) | Implement use_dismissable and DismissableRegion in ars-leptos                 | 5      | #8   | #65, #69, #88, #104 |
| [#106](https://github.com/fogodev/ars-ui/issues/106) | Implement use_dismissable and DismissableRegion in ars-dioxus                 | 5      | #9   | #65, #69, #88, #104 |

#### PF-IO-1: Implement Dismissable core props and dismiss-button attrs in ars-interactions

- Points: `3`
- Layer: `Component`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #65
- Spec refs:
  - `spec/components/utility/dismissable.md` §1 "API"
  - `spec/components/utility/dismissable.md` §4 "Internationalization"
  - `spec/components/utility/dismissable.md` §5 "Behavior"
  - `spec/foundation/05-interactions.md` §12 "InteractOutside Interaction"
  - `spec/foundation/04-internationalization.md` §7 "Messages"
- Goal: implement the shared `Dismissable` contract so both adapters consume the same props, messages, parts, and dismiss-button attr helper.
- Files to create/modify: `crates/ars-interactions/src/dismissable.rs` (new), `crates/ars-interactions/src/lib.rs` (wire new module)
- Tests to add first:
  - Unit tests for `dismissable::Props` defaults and debug output.
  - Unit tests for callback-bearing `Props` clone/partial-eq pointer identity semantics.
  - Unit tests for `dismissable::Messages` default close label.
  - Unit tests for `dismissable::dismiss_button_attrs()` producing scope/part data attrs, native button semantics, visually-hidden marker, and localized `aria-label`.
  - Unit tests for `exclude_ids` and `disable_outside_pointer_events` config preservation.
- Acceptance criteria:
  - `dismissable::Props` with `on_interact_outside`, `on_escape_key_down`, `on_dismiss`, `disable_outside_pointer_events`, `exclude_ids`, `messages`, and `locale`.
  - `dismissable::Messages` with default English close label following the shared `MessageFn` pattern.
  - `dismissable::Part` with `Root` and `DismissButton`.
  - `dismiss_button_attrs(&Props) -> AttrMap` matching the spec.
  - No document listeners or framework-specific containment logic in this task.
- Spec impact: `No spec change required`.

#### PF-IO-2: Implement use_dismissable and DismissableRegion in ars-leptos

- Points: `5`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Depends on: #65, #69, #88, #104
- Spec refs:
  - `spec/leptos-components/utility/dismissable.md`
  - `spec/components/utility/dismissable.md`
  - `spec/foundation/05-interactions.md` §12 "InteractOutside Interaction"
  - `spec/testing/10-keyboard-focus.md` §13.2 "InteractOutside Tests"
  - `docs/implementation/adapter-contract.md`
- Goal: implement the Leptos adapter-owned `Dismissable` hook and region wrapper with client-only listeners, portal-aware containment, and topmost-overlay dismissal behavior.
- Files to create/modify: `crates/ars-leptos/src/dismissable.rs` (new), `crates/ars-leptos/src/lib.rs` (wire new module)
- Tests to add first:
  - Adapter tests for outside `pointerdown` calling `on_interact_outside` then `on_dismiss`.
  - Adapter tests for outside `focusin` calling `on_interact_outside` then `on_dismiss`.
  - Adapter tests for Escape dismissal firing `on_escape_key_down` then `on_dismiss`.
  - Adapter tests for `exclude_ids` and additional inside boundaries suppressing dismissal.
  - Adapter tests for portal-aware containment using `data-ars-portal-owner`.
  - Adapter tests for topmost-overlay-only dismissal using the overlay stack.
  - Adapter tests for SSR safety and cleanup removing listeners / pending retries.
  - Adapter tests for both dismiss buttons invoking dismiss behavior.
- Acceptance criteria:
  - `use_dismissable(root_ref, props, inside_boundaries) -> DismissableHandle` implemented per spec.
  - `DismissableRegion` renders both native dismiss buttons around consumer content.
  - Client-only `pointerdown`, `focusin`, and Escape listeners with proper cleanup.
  - Portal-aware containment and overlay-stack-aware topmost dismissal behavior.
  - `disable_outside_pointer_events` behavior supported without breaking keyboard dismissal.
  - Adapter behavior and cleanup ordering match the spec and adapter contract.
- Spec impact: `No spec change required`.

#### PF-IO-3: Implement use_dismissable and DismissableRegion in ars-dioxus

- Points: `5`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Depends on: #65, #69, #88, #104
- Spec refs:
  - `spec/dioxus-components/utility/dismissable.md`
  - `spec/components/utility/dismissable.md`
  - `spec/foundation/05-interactions.md` §12 "InteractOutside Interaction"
  - `spec/testing/10-keyboard-focus.md` §13.2 "InteractOutside Tests"
  - `docs/implementation/adapter-contract.md`
- Goal: implement the Dioxus adapter-owned `Dismissable` hook and region wrapper with client-only listeners, portal-aware containment, and topmost-overlay dismissal behavior across Dioxus targets.
- Files to create/modify: `crates/ars-dioxus/src/dismissable.rs` (new), `crates/ars-dioxus/src/lib.rs` (wire new module)
- Tests to add first:
  - Adapter tests for outside `pointerdown` calling `on_interact_outside` then `on_dismiss`.
  - Adapter tests for outside `focusin` calling `on_interact_outside` then `on_dismiss`.
  - Adapter tests for Escape dismissal firing `on_escape_key_down` then `on_dismiss`.
  - Adapter tests for `exclude_ids` and additional inside boundaries suppressing dismissal.
  - Adapter tests for portal-aware containment using `data-ars-portal-owner`.
  - Adapter tests for topmost-overlay-only dismissal using the overlay stack.
  - Adapter tests for web/Desktop-safe cleanup removing listeners / pending retries.
  - Adapter tests for both dismiss buttons invoking dismiss behavior.
- Acceptance criteria:
  - `use_dismissable(root_id, props, inside_boundaries) -> DismissableHandle` implemented per spec.
  - `DismissableRegion` renders both native dismiss buttons around consumer content.
  - Client-only `pointerdown`, `focusin`, and Escape listeners with proper cleanup.
  - Portal-aware containment and overlay-stack-aware topmost dismissal behavior.
  - `disable_outside_pointer_events` behavior supported without breaking keyboard dismissal.
  - Dioxus Web/Desktop behavior and cleanup ordering match the spec and adapter contract.
- Spec impact: `No spec change required`.

## Summary

| Wave      | Tasks  | Points  | Unlocks                                                  |
| --------- | ------ | ------- | -------------------------------------------------------- |
| Wave 1    | 7      | 19      | Button, VisuallyHidden, Separator                        |
| Wave 2    | 8      | 29      | Select, Combobox, Menu, Listbox, Dialog, Popover         |
| Wave 3    | 8      | 25      | Tooltip, DatePicker prerequisites, accessibility         |
| Wave 4    | 10     | 50      | All remaining components (Slider, TreeView, Table, etc.) |
| **Total** | **33** | **123** | **Complete foundation for all 112 components**           |
