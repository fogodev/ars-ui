# Foundation Completion Roadmap

This roadmap picks up after the foundation-gap-audit tasks (#31–#41) and defines the remaining work required to complete the foundation layer before any UI component implementation begins.

## Why Complete the Foundation First

The project needs a fully stable foundation before component work starts. Components built on incomplete foundation crates will create merge conflicts when parallel component PRs all touch the same foundation files. By completing interactions, collections, DOM positioning, and i18n first, component authors can work independently against stable APIs.

## Status Summary

### What is built (357 tests passing)

| Crate              | LOC    | Status   | Key surface                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------------ | ------ | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ars-core`         | 4,306  | Solid    | Machine, Service, TransitionPlan, PendingEffect, Bindable, ConnectApi, ComponentPart, AttrMap/AttrValue/UserAttrs, StyleStrategy, Callback, WeakSend, PlatformEffects, Provider (ColorMode), companion CSS                                                                                                                                                                                                                                                                                                                                     |
| `ars-derive`       | 535    | Complete | HasId, ComponentPart proc macros with error tests                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `ars-a11y`         | 5,800+ | 95%      | AriaRole (88 variants), AriaAttribute (55+ variants), ComponentIds, ARIA state helpers (`set_expanded`/`set_selected`/`set_checked`/`set_disabled`/`set_busy`/`set_invalid`), FocusScopeBehavior, FocusStrategy, FocusRing, FocusZone, DomEvent/KeyboardShortcut/Platform, VisuallyHidden, LabelConfig/DescriptionConfig/FieldContext, LiveAnnouncer, Announcements/Messages (14 fields), Touch/Mobile, AriaValidator, FocusZoneTestHarness. Missing: ARIA assertion test helpers (#554), `set_readonly` (#555), public focus selectors (#556) |
| `ars-forms`        | 4,128  | Partial  | field::State/Value/Context/Descriptors/InputAria, validation::Error/Validator/AsyncValidator, form::Context/Data/Mode, hidden_input, form_submit machine. Missing: built-in validators, ValidatorsBuilder, Messages, DebouncedAsyncValidator, Fieldset/Field/Form machines                                                                                                                                                                                                                                                                     |
| `ars-interactions` | 3,107  | Partial  | Press, Hover, Focus, FocusWithin, InteractOutside, Dismissable, compose::merge_attrs, LogicalDirection. Missing: LongPress, Move, DnD, Keyboard types                                                                                                                                                                                                                                                                                                                                                                                          |
| `ars-dom`          | 5,880  | Partial  | FocusScope, focus queries, ScrollLockManager, positioning engine (types + compute_position + overflow + VirtualElement), z-index allocator, scroll_into_view, modality manager. Missing: viewport/visualViewport, containing-block detection, auto_update, portal/inert, overlay stack, media queries, URL sanitization                                                                                                                                                                                                                        |
| `ars-leptos`       | 1,195  | Partial  | use_machine, UseMachineReturn, EphemeralRef, use_id, attr_map_to_leptos, use_style_strategy, AdapterCapabilities. Missing: ArsProvider context (#190), reactive props (#190), controlled value helper (#190), emit/emit_map (#191), event mapping (#191), nonce CSS collector (#191), safe event listeners (#191), LiveAnnouncer context bridge (#513)                                                                                                                                                                                         |
| `ars-dioxus`       | 762    | Partial  | use_machine, UseMachineReturn, EphemeralRef, use_id, attr_map_to_dioxus, use_style_strategy, AdapterCapabilities. Missing: ArsProvider context (#193), reactive props (#193), controlled value helper (#193), emit/emit_map (#194), event mapping (#194), nonce CSS collector (#194), safe event listeners (#194), LiveAnnouncer context bridge (#512), DioxusPlatform (#195), SSR hydration (#196), error boundary (#197)                                                                                                                     |
| `ars-collections`  | 6,221  | 90%      | Key, Node, NodeType, Collection trait, StaticCollection, TreeCollection, CollectionBuilder, selection (Mode/Behavior/Set/State/DisabledBehavior), navigation helpers, typeahead, AsyncCollection/AsyncLoader, Virtualizer/LayoutStrategy/VirtualLayout, FilteredCollection, SortedCollection, CollationSupport (i18n). Missing: MutableListData/MutableTreeData, CollectionChangeAnnouncement/CollectionMessages, OnAction, DnD types                                                                                                          |
| `ars-i18n`         | 1,928  | Partial  | Locale (ICU4X-backed), Direction, Orientation, NumberFormatter, CurrencyCode, BiDi isolation, Weekday, IntlBackend trait (stub), placeholder date/time types                                                                                                                                                                                                                                                                                                                                                                                   |

### Architecture spec (01-architecture.md) completion — 2026-04-10 audit

A full section-by-section audit of `spec/foundation/01-architecture.md` (§1–§10, ~5000 lines) against the implementation found that **~95% of the spec is implemented**. Four gaps remain, tracked as sub-issues of Epic #2:

| Gap                                                          | Issue                                                | Points | Status |
| ------------------------------------------------------------ | ---------------------------------------------------- | ------ | ------ |
| `BindableValue` trait alias + `Bindable<T>: Default`         | [#145](https://github.com/fogodev/ars-ui/issues/145) | 1      | Open   |
| `Orientation` re-export from ars-core                        | [#146](https://github.com/fogodev/ars-ui/issues/146) | 1      | Open   |
| Structured debug logging (§2.9) — `debug` feature, `log` dep | [#147](https://github.com/fogodev/ars-ui/issues/147) | 3      | Open   |
| `WebPlatformEffects` in ars-dom (§2.2.7)                     | [#148](https://github.com/fogodev/ars-ui/issues/148) | 5      | Open   |

Issues #145 and #146 are trivial and unblocked. #147 is self-contained. #148 depends partially on #73, #69, #85 for full method coverage but can land with documented stubs for blocked methods.

### Foundation gap matrix

| Foundation area    | Spec file                    | Spec coverage            | Implementation % | Blocking impact                                                                                                                                                                                                                                                                             |
| ------------------ | ---------------------------- | ------------------------ | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Architecture core  | `01-architecture.md`         | ~5000 lines, 10 sections | 95%              | 4 remaining gaps tracked above; core contract is stable                                                                                                                                                                                                                                     |
| Interactions       | `05-interactions.md`         | ~4000 lines, 12 sections | 60%              | 8 tasks closed; 6 open (#76, #77, #159–#162). Blocks Slider, DnD components, custom keyboard handlers                                                                                                                                                                                       |
| Collections        | `06-collections.md`          | ~5700 lines, 10 sections | 90%              | 4 tasks remain (12 pts): mutable wrappers, announcements, DnD, coverage. Core collection/selection/tree/async/virtual complete.                                                                                                                                                             |
| I18n               | `04-internationalization.md` | ~4000 lines, 16 sections | 25%              | Blocks number/date components, RTL. Locale + NumberFormatter done; 16 tasks remaining (48 pts ICU4X + web-intl parity)                                                                                                                                                                      |
| DOM utilities      | `11-dom-utilities.md`        | ~2800 lines, 10 sections | 50%              | 8 tasks closed; 8 open (#69, #72, #85, #88, #112–#114, #176). Blocks all overlay components                                                                                                                                                                                                 |
| Accessibility      | `03-accessibility.md`        | ~4340 lines, 14 sections | 95%              | Wave 4 complete (13 tasks closed). 3 audit follow-up tasks remain (5 pts): ARIA assertion helpers (#554, 3pts), `set_readonly` (#555, 1pt), public focus selectors (#556, 1pt)                                                                                                              |
| Forms              | `07-forms.md`                | ~4300 lines, 15 sections | 50%              | 3 tasks closed; 8 open (#164–#171, 26 pts). Blocks Field, Fieldset, Form components and validator builder API                                                                                                                                                                               |
| Adapter conversion | `08/09-adapter-*.md` §4/§3   | ~200 lines               | 40%              | AttrMap conversion done (#55/#56). Leptos: ArsProvider (#190), utilities (#191), LiveAnnouncer bridge (#513). Dioxus: ArsProvider (#193), utilities (#194), LiveAnnouncer bridge (#512), DioxusPlatform (#195), SSR hydration (#196), error boundary (#197). Blocks ALL component rendering |

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
  - `ArsContext` exposes a shared `Arc<dyn ModalityContext>` alongside `PlatformEffects`.
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
  - `ModalityManager` holds `Arc<dyn ModalityContext>` plus `FocusRing`.
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

**Note:** Tasks sized at `8` points (#82) must be decomposed into ≤5-point subtasks before pickup. #80 (DateFormatter, 8pts) has been decomposed into #128, #129, #130. #78 (Drag and Drop, 8pts) has been decomposed into #159, #160, #161.

| GitHub                                               | Title                                                                    | Points | Epic | Deps       |
| ---------------------------------------------------- | ------------------------------------------------------------------------ | ------ | ---- | ---------- |
| [#76](https://github.com/fogodev/ars-ui/issues/76)   | Implement LongPress interaction in ars-interactions                      | 3      | #4   | Wave 1     |
| [#77](https://github.com/fogodev/ars-ui/issues/77)   | Implement Move interaction in ars-interactions                           | 3      | #4   | Wave 1     |
| [#159](https://github.com/fogodev/ars-ui/issues/159) | Implement Drag and Drop core types in ars-interactions                   | 2      | #4   | #58        |
| [#160](https://github.com/fogodev/ars-ui/issues/160) | Implement Drag/Drop state machines and use_drag/use_drop                 | 3      | #4   | #159, #76  |
| [#161](https://github.com/fogodev/ars-ui/issues/161) | Implement keyboard DnD protocol and screen reader announcements          | 3      | #4   | #160       |
| [#162](https://github.com/fogodev/ars-ui/issues/162) | Implement Keyboard interaction types in ars-interactions                 | 2      | #4   | —          |
| [#79](https://github.com/fogodev/ars-ui/issues/79)   | Implement NumberFormatter trait with ICU4X backend in ars-i18n           | 5      | #54  | #75        |
| [#128](https://github.com/fogodev/ars-ui/issues/128) | CalendarDate internal type, calendar system extensions, and error types  | 5      | #54  | #75        |
| [#129](https://github.com/fogodev/ars-ui/issues/129) | DateFormatter with ICU4X backend                                         | 3      | #54  | #128       |
| [#130](https://github.com/fogodev/ars-ui/issues/130) | RelativeTimeFormatter with ICU4X backend                                 | 3      | #54  | #75        |
| [#131](https://github.com/fogodev/ars-ui/issues/131) | Plural and ordinal rules with ICU4X backend                              | 3      | #54  | #75        |
| [#132](https://github.com/fogodev/ars-ui/issues/132) | Logical/Physical side and rect layout types                              | 2      | #54  | #75        |
| [#133](https://github.com/fogodev/ars-ui/issues/133) | Locale-aware case transformation with ICU4X backend                      | 2      | #54  | #75        |
| [#134](https://github.com/fogodev/ars-ui/issues/134) | LocaleStack fallback chain                                               | 2      | #54  | #75        |
| [#135](https://github.com/fogodev/ars-ui/issues/135) | MessagesRegistry, I18nRegistries, and resolve_messages                   | 3      | #54  | #134       |
| [#136](https://github.com/fogodev/ars-ui/issues/136) | StringCollator with ICU4X backend                                        | 3      | #54  | #75        |
| [#137](https://github.com/fogodev/ars-ui/issues/137) | Translate trait                                                          | 2      | #54  | #75        |
| [#138](https://github.com/fogodev/ars-ui/issues/138) | IcuProvider full trait + Icu4xProvider production implementation         | 5      | #54  | #128, #131 |
| [#139](https://github.com/fogodev/ars-ui/issues/139) | locale_from_accept_language server utility                               | 2      | #54  | #75        |
| [#140](https://github.com/fogodev/ars-ui/issues/140) | t() function for Leptos and Dioxus adapters                              | 3      | #54  | #137       |
| [#81](https://github.com/fogodev/ars-ui/issues/81)   | Implement AsyncCollection with pagination support in ars-collections     | 5      | #53  | #63        |
| [#82](https://github.com/fogodev/ars-ui/issues/82)   | Implement Virtualizer for large collection rendering                     | 8      | #53  | #63        |
| [#83](https://github.com/fogodev/ars-ui/issues/83)   | Implement TreeCollection in ars-collections                              | 5      | #53  | #63        |
| [#84](https://github.com/fogodev/ars-ui/issues/84)   | Implement FilteredCollection and SortedCollection in ars-collections     | 3      | #53  | #63        |
| [#518](https://github.com/fogodev/ars-ui/issues/518) | Implement CollationSupport and CollatorCache in ars-collections          | 2      | #53  | #136, #84  |
| [#547](https://github.com/fogodev/ars-ui/issues/547) | Implement MutableListData, MutableTreeData, and CollectionChange         | 3      | #53  | #63, #83   |
| [#548](https://github.com/fogodev/ars-ui/issues/548) | Implement CollectionChangeAnnouncement, CollectionMessages, and OnAction | 2      | #53  | #84        |
| [#549](https://github.com/fogodev/ars-ui/issues/549) | Implement DnD collection traits and types                                | 5      | #53  | #159, #64  |
| [#550](https://github.com/fogodev/ars-ui/issues/550) | Improve ars-collections edge-case test coverage                          | 2      | #53  | —          |
| [#85](https://github.com/fogodev/ars-ui/issues/85)   | Implement media query utilities in ars-dom                               | 2      | #6   | —          |
| [#176](https://github.com/fogodev/ars-ui/issues/176) | Implement URL sanitization utilities                                     | 2      | #6   | —          |
| [#150](https://github.com/fogodev/ars-ui/issues/150) | Implement FocusZone for arrow-key navigation in composite widgets        | 5      | #3   | —          |
| [#151](https://github.com/fogodev/ars-ui/issues/151) | Implement DomEvent trait, KeyboardShortcut, and Platform detection       | 3      | #3   | —          |
| [#152](https://github.com/fogodev/ars-ui/issues/152) | Implement VisuallyHidden utilities for screen-reader-only content        | 1      | #3   | —          |
| [#153](https://github.com/fogodev/ars-ui/issues/153) | Implement LabelConfig, DescriptionConfig, and FieldContext               | 3      | #3   | —          |
| [#154](https://github.com/fogodev/ars-ui/issues/154) | Implement AnnouncementMessages and Announcements helpers                 | 2      | #3   | —          |
| [#155](https://github.com/fogodev/ars-ui/issues/155) | Implement Touch and Mobile accessibility utilities                       | 2      | #3   | #151       |
| [#156](https://github.com/fogodev/ars-ui/issues/156) | Implement ARIA Validation testing infrastructure                         | 3      | #3   | —          |
| [#157](https://github.com/fogodev/ars-ui/issues/157) | Implement Keyboard Navigation test helpers                               | 3      | #3   | #150, #151 |
| [#554](https://github.com/fogodev/ars-ui/issues/554) | Implement ARIA assertion test helpers for component testing              | 3      | #3   | —          |
| [#555](https://github.com/fogodev/ars-ui/issues/555) | Add set_readonly helper and DATA_ARS_READONLY constant                   | 1      | #3   | —          |
| [#556](https://github.com/fogodev/ars-ui/issues/556) | Export TABBABLE_SELECTOR and FOCUSABLE_SELECTOR as public constants      | 1      | #6   | —          |
| [#164](https://github.com/fogodev/ars-ui/issues/164) | Add MessageFn From impls for usize and f64 arity closures                | 1      | #5   | —          |
| [#165](https://github.com/fogodev/ars-ui/issues/165) | Implement Messages, Error factory methods, DEFAULT_VALIDATOR_LOCALE      | 3      | #5   | #164       |
| [#166](https://github.com/fogodev/ars-ui/issues/166) | Implement built-in validators and FnValidator                            | 5      | #5   | #165       |
| [#167](https://github.com/fogodev/ars-ui/issues/167) | Implement ChainValidator, ValidatorsBuilder, and Validators alias        | 3      | #5   | #166       |
| [#168](https://github.com/fogodev/ars-ui/issues/168) | Implement AsyncFnValidator, DebouncedAsyncValidator, and TimerHandle     | 3      | #5   | —          |
| [#169](https://github.com/fogodev/ars-ui/issues/169) | Implement Fieldset component machine in ars-forms                        | 3      | #5   | —          |
| [#170](https://github.com/fogodev/ars-ui/issues/170) | Implement Field component machine in ars-forms                           | 3      | #5   | —          |
| [#171](https://github.com/fogodev/ars-ui/issues/171) | Implement Form component machine in ars-forms                            | 5      | #5   | —          |

**Total:** 151 points (132 original + 2 collation + 12 Wave 5-C collections + 5 a11y audit follow-up)

---

### Wave 5: Browser Intl Backends

**Goal:** Complete the `web-intl` i18n backend so WASM client builds can rely on browser `Intl` services behind the same public `ars-i18n` API surface established by the ICU4X tasks. Includes full parity for collation, case transformation, and the `IntlBackend` trait.

**Depends on:** `#75` plus the relevant Wave 4 i18n foundation tasks.

| GitHub                                               | Title                                                                          | Points | Epic | Deps      |
| ---------------------------------------------------- | ------------------------------------------------------------------------------ | ------ | ---- | --------- |
| [#124](https://github.com/fogodev/ars-ui/issues/124) | Implement web-intl NumberFormatter backend in ars-i18n                         | 5      | #54  | #75, #79  |
| [#125](https://github.com/fogodev/ars-ui/issues/125) | Implement web-intl DateFormatter and RelativeTimeFormatter backend in ars-i18n | 5      | #54  | #75, #129 |
| [#126](https://github.com/fogodev/ars-ui/issues/126) | Implement web-intl plural and ordinal rules backend in ars-i18n                | 3      | #54  | #75       |
| [#141](https://github.com/fogodev/ars-ui/issues/141) | web-intl StringCollator backend                                                | 3      | #54  | #136      |
| [#142](https://github.com/fogodev/ars-ui/issues/142) | web-intl case transformation backend                                           | 2      | #54  | #133      |
| [#143](https://github.com/fogodev/ars-ui/issues/143) | WebIntlProvider implementation of IcuProvider                                  | 5      | #54  | #138      |

**Total:** 23 points

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

#### W4-3a: Implement Drag and Drop core types in ars-interactions

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #58
- Spec refs:
  - `spec/foundation/05-interactions.md` §7.2 "Item Types" (L1862–L1912)
  - `spec/foundation/05-interactions.md` §7.3 "Drop Operation" (L1914–L1944)
  - `spec/foundation/05-interactions.md` §7.4 "Drag Source Configuration" (L1947–L1997)
  - `spec/foundation/05-interactions.md` §7.5 "Drop Target Configuration" (L1999–L2096)
  - `spec/foundation/05-interactions.md` §7.5.1 "Dropzone Accept Type Validation" (L2098–L2100)
- Goal: define all data types, configuration structs, event structs, and enums for drag-and-drop.
- Acceptance criteria:
  - `DragItem`, `FileHandle`, `DirectoryHandle`, `DropOperation` (with `as_drop_effect()`).
  - `DragConfig`, `DragStartEvent`, `DragEndEvent`, `DragConfig::with_selection()`.
  - `DropConfig`, `DropIndicatorPosition`, `DropTargetEvent`, `DragItemPreview`, `DragItemKind`, `DropEvent`.
  - MIME type validation logic (case-insensitive, wildcard `image/*`).
- Spec impact: `No spec change required`.

#### W4-3b: Implement Drag/Drop state machines and use_drag/use_drop in ars-interactions

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #159, #76
- Spec refs:
  - `spec/foundation/05-interactions.md` §7.6 "Drag State Machine (Source Side)" (L2106–L2166)
  - `spec/foundation/05-interactions.md` §7.9 "Multi-Item Drag" (L2270–L2288)
  - `spec/foundation/05-interactions.md` §7.10 "Drop Indicators and Positioning" (L2290–L2443)
  - `spec/foundation/05-interactions.md` §7.10.1 "Pointer Capture Error Recovery" (L2387–L2415)
- Goal: implement drag source and drop target state machines, `use_drag`/`use_drop` factory functions, and result types.
- Acceptance criteria:
  - `DragState` enum with full transition set per spec §7.6.
  - Drop target enter/leave counting with `enter_count: i32`.
  - `DragResult` and `DropResult` with snapshot-based `attrs: AttrMap`.
  - `use_drag(config) -> DragResult` and `use_drop(config) -> DropResult`.
  - Data attributes: `data-ars-dragging`, `data-ars-drag-over`, `data-ars-drop-operation`, `data-ars-drop-position`.
- Spec impact: `No spec change required`.

#### W4-3c: Implement keyboard DnD protocol and screen reader announcements in ars-interactions

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #160
- Spec refs:
  - `spec/foundation/05-interactions.md` §7.7 "Keyboard Drag and Drop Protocol" (L2168–L2196)
  - `spec/foundation/05-interactions.md` §7.8 "Screen Reader DnD Announcements" (L2198–L2268)
  - `spec/foundation/04-internationalization.md` §7.1 (MessageFn pattern)
- Goal: implement the keyboard DnD modal protocol and `DragAnnouncements` screen reader integration.
- Acceptance criteria:
  - `KeyboardDragRegistry` and `KeyboardDropTarget` structs.
  - Full keyboard protocol: Enter → start, Tab/Shift+Tab → cycle targets, Enter → drop, Escape → cancel.
  - `DragAnnouncements` with 5 `MessageFn` fields and `Default` impl.
  - Announcement priority dispatch: Assertive for drag_start/drop/cancel, Polite for enter/leave.
  - Per-element announcement closures take precedence over defaults.
- Spec impact: `No spec change required`.

#### W4-keyboard: Implement Keyboard interaction types in ars-interactions

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/05-interactions.md` §11 "Keyboard Interaction" (L2948–L3998)
  - `spec/foundation/05-interactions.md` §11.2 "Configuration" (L2957–L3961)
  - `spec/foundation/05-interactions.md` §11.5 "IME Composition Handling" (L3971–L3991)
- Goal: implement the standalone keyboard interaction types that components with custom key handling consume.
- Files to create/modify: `crates/ars-interactions/src/keyboard.rs` (new), wire into `lib.rs`
- Acceptance criteria:
  - `KeyboardConfig` with `disabled: bool`.
  - `KeyboardEventData` with key, character, code, modifiers, repeat, is_composing.
  - `ArsKeyboardEvent` enum: `KeyDown(KeyboardEventData)`, `KeyUp(KeyboardEventData)`.
  - `pub use ars_core::KeyboardKey;` re-export.
  - Doc comments on `is_composing` explain IME suppression requirement.
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

#### W4-5: ~~Implement DateFormatter and calendar system support in ars-i18n~~ (SUPERSEDED)

**#80 has been closed and decomposed into 16 granular tasks under epic #54:**

- **Wave 4a (replaces #80):** #128 CalendarDate (5pts), #129 DateFormatter (3pts), #130 RelativeTimeFormatter (3pts)
- **Wave 4b:** #131 Plural rules (3pts), #132 Layout geometry (2pts), #133 Case transformation (2pts), #134 LocaleStack (2pts)
- **Wave 4c:** #135 MessagesRegistry (3pts), #136 StringCollator (3pts), #137 Translate trait (2pts)
- **Wave 5a:** #138 IntlBackend+Icu4xBackend (5pts), #139 accept_language (2pts), #140 t() function (3pts)
- **Wave 5b (web-intl parity):** #141 web-intl Collation (3pts), #142 web-intl Case (2pts), #143 WebIntlBackend (5pts)

See epic #54 for the full wave structure, dependency graph, and ICU4X ↔ web-intl parity matrix.

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

#### W5-C1: Implement MutableListData, MutableTreeData, and CollectionChange in ars-collections

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #63, #83
- Spec refs:
  - `spec/foundation/06-collections.md` §1.8.1 "Change Events" (L757)
  - `spec/foundation/06-collections.md` §1.8.2 "MutableListData" (L780)
  - `spec/foundation/06-collections.md` §1.8.3 "MutableTreeData" (L865)
- Goal: implement mutable collection wrappers with change tracking for DOM reconciliation.
- Files to create/modify: `crates/ars-collections/src/mutable.rs` (new), `crates/ars-collections/src/lib.rs`
- Tests to add first:
  - `MutableListData` push/insert/remove/move/replace/clear emit correct `CollectionChange` variants.
  - `drain_changes()` returns pending changes and clears the buffer.
  - `Collection<T>` delegation to inner `StaticCollection`.
  - `MutableTreeData` insert_child/remove/reparent/reorder/replace emit correct changes.
  - `MutableTreeData` `Collection<T>` delegation to inner `TreeCollection`.
- Acceptance criteria:
  - `CollectionChange<K: Clone>` enum (Insert/Remove/Move/Replace/Reset).
  - `MutableListData<T>` wrapping `StaticCollection<T>` with mutation + change tracking.
  - `MutableTreeData<T>` wrapping `TreeCollection<T>` with mutation + change tracking.
  - Both implement `Collection<T>` via delegation.
  - `drain_changes()` via `core::mem::take`.
- Spec impact: `No spec change required`.

#### W5-C2: Implement CollectionChangeAnnouncement, CollectionMessages, and OnAction in ars-collections

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #84
- Spec refs:
  - `spec/foundation/06-collections.md` §1.6.1 "Collection Change Announcements" (L610)
  - `spec/foundation/06-collections.md` §3.1 "OnAction" (L2163)
- Goal: implement accessibility announcement types for collection mutations and the OnAction callback alias.
- Files to create/modify: `crates/ars-collections/src/announcements.rs` (new), `crates/ars-collections/src/selection.rs`, `crates/ars-collections/src/lib.rs`
- Tests to add first:
  - Construct each `CollectionChangeAnnouncement` variant.
  - `CollectionMessages::default()` closures return correct English text.
  - `CollectionMessages` Debug output.
  - `OnAction` type alias compiles on current target.
- Acceptance criteria:
  - `CollectionChangeAnnouncement` enum (6 variants).
  - `CollectionMessages` struct with 6 `MessageFn` closure fields + Default + Debug.
  - `OnAction` cfg-gated type alias (Rc on wasm32, Arc on native).
  - No new crate dependencies (`MessageFn` from `ars_core`, `Locale` from `ars_core`).
- Spec impact: `No spec change required`.

#### W5-C3: Implement DnD collection traits and types in ars-collections

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #159, #64
- Spec refs:
  - `spec/foundation/06-collections.md` §10.2 "Drop Position" (L5319)
  - `spec/foundation/06-collections.md` §10.3 "Collection DnD Events" (L5366)
  - `spec/foundation/06-collections.md` §10.4 "DraggableCollection Trait" (L5432)
  - `spec/foundation/06-collections.md` §10.5 "DroppableCollection Trait" (L5484)
  - `spec/foundation/06-collections.md` §10.9 "I18n — CollectionDndMessages" (L5590)
- Goal: implement collection-level drag-and-drop integration types and traits.
- Files to create/modify: `crates/ars-collections/src/dnd.rs` (new), `crates/ars-collections/src/lib.rs`, `crates/ars-collections/Cargo.toml` (add `ars-interactions` dep)
- New dependency: `ars-interactions = { path = "../ars-interactions", default-features = false }`
- Tests to add first:
  - `DropPosition` Display output and trait derives.
  - `CollectionDropTarget` construction.
  - `CollectionDndEvent` all 5 variants.
  - `DraggableCollection` default methods.
  - `DroppableCollection` default methods.
  - `CollectionDndMessages::default()` English text for all 8 fields.
  - `DndAnnouncements` and `DndAnnouncementData` construction.
- Acceptance criteria:
  - 8 types: DropPosition, CollectionDropTarget, CollectionDndEvent, DraggableCollection, DroppableCollection, CollectionDndMessages, DndAnnouncements, DndAnnouncementData.
  - All re-exported from `lib.rs`.
- Spec impact: Fix §10.4 `DraggableCollection::drag_data` default impl — `DragItem` is an enum, not a struct.

#### W5-C4: Improve ars-collections edge-case test coverage

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/06-collections.md` (general)
- Goal: cover 4 identified edge-case gaps in existing code (99.47% → ~99.7% line coverage).
- Files to modify: `crates/ars-collections/src/selection.rs`, `crates/ars-collections/src/tree_collection.rs`, `crates/ars-collections/src/virtualization.rs` (add tests only)
- Tests to add:
    1. `selection::State::select()` with `Mode::None` — verify state unchanged.
    2. `selection::State::deselect_from_all()` with `Set::Multiple` — verify delegates to deselect.
    3. `TreeCollection::reparent()` circular ancestor guard — verify tree stays valid.
    4. `Virtualizer::visible_range()` grid range inversion — verify empty range returned.
- Acceptance criteria:
  - All 4 new tests pass.
  - `cargo llvm-cov` shows fewer missed lines than current 33.
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

#### W4-11: Implement URL sanitization utilities

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/01-architecture.md` §3.1.1.1 "URL Sanitization"
- Goal: implement URL validation and sanitization to prevent URL injection attacks (`javascript:`, `data:`, `vbscript:` schemes).
- Files to create/modify: core connect-layer URL helper module and exports
- Tests to add first:
  - `is_safe_url()` allows `http://`, `https://`, `mailto:`, `tel:`, relative paths; rejects `javascript:`, `data:`, `vbscript:`.
  - Case-insensitive scheme detection.
  - `sanitize_url()` returns `"#"` for unsafe URLs.
  - `SafeUrl::new()` returns `Ok`/`Err` appropriately.
  - `SafeUrl` and `UnsafeUrlError` implement `Display`.
- Acceptance criteria:
  - `is_safe_url(url: &str) -> bool` validates URL schemes against an allowlist.
  - `sanitize_url(url: &str) -> &str` returns `"#"` for unsafe URLs.
  - `SafeUrl` newtype enforces validation at construction time.
  - `UnsafeUrlError` error type for rejected URLs.
  - Pure Rust, no `web_sys` deps.
- Spec impact: `Canonical ownership belongs in architecture/connect docs, not DOM utilities`.

---

### Wave 5 Task Details

#### W5-1: Implement web-intl NumberFormatter backend in ars-i18n

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #75, #79
- Spec refs:
  - `spec/foundation/04-internationalization.md`
  - `spec/testing/14-ci.md`
- Goal: implement the browser `Intl.NumberFormat` backend behind the existing `ars-i18n::NumberFormatter` public API.
- Files to create/modify:
  - `crates/ars-i18n/Cargo.toml`
  - `crates/ars-i18n/src/lib.rs`
  - `crates/ars-i18n/src/number.rs` or `crates/ars-i18n/src/web_intl/number.rs`
  - `spec/foundation/04-internationalization.md`
  - `spec/testing/14-ci.md` only if checks/spec drift need sync
- Tests to add first:
  - Feature-gated compile coverage proving `ars-i18n` builds with `--no-default-features --features web-intl --target wasm32-unknown-unknown`.
  - Tests for option mapping of decimal, percent, and currency formatting.
  - Tests proving `icu4x` and `web-intl` are mutually exclusive.
  - Tests or smoke coverage for locale separator extraction and parsing behavior under `web-intl`.
- Acceptance criteria:
  - `web-intl` is a real backend, not an empty feature flag.
  - `NumberFormatter` keeps the same public API under both `icu4x` and `web-intl`.
  - Browser builds use `Intl.NumberFormat` for decimal, percent, and currency formatting.
  - `format_percent(0.47, None)` preserves ars-ui fractional semantics and formats as `47%`.
  - `format_currency()` preserves ISO-4217 minor-unit defaults.
  - `decimal_separator()` and `grouping_separator()` work under `web-intl`.
  - `parse()` remains locale-aware for browser-backed formatting.
  - `icu4x` and `web-intl` cannot be enabled together.
  - CI and verification include the wasm `web-intl` cargo check path required by the spec.
- Spec impact: `Likely yes` to remove stale backend-specific public wrapper sketches if the concrete `NumberFormatter` API remains canonical.

#### W5-2: Implement web-intl DateFormatter and RelativeTimeFormatter backend in ars-i18n

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #75, #129
- Spec refs:
  - `spec/foundation/04-internationalization.md`
  - `spec/shared/date-time-types.md`
  - `spec/testing/14-ci.md`
- Goal: implement browser `Intl.DateTimeFormat` and `Intl.RelativeTimeFormat` backends for the existing `ars-i18n` date and relative-time public APIs.
- Files to create/modify:
  - `crates/ars-i18n/Cargo.toml`
  - `crates/ars-i18n/src/lib.rs`
  - `crates/ars-i18n/src/date.rs` and/or `crates/ars-i18n/src/web_intl/date.rs`
  - `spec/foundation/04-internationalization.md`
  - `spec/testing/14-ci.md` only if checks/spec drift need sync
- Tests to add first:
  - Feature-gated compile coverage proving `ars-i18n` builds with `--no-default-features --features web-intl --target wasm32-unknown-unknown`.
  - Tests for representative date formatting under locale-sensitive browser patterns.
  - Tests for relative time formatting via browser `Intl.RelativeTimeFormat`.
  - Tests proving `icu4x` and `web-intl` are mutually exclusive.
- Acceptance criteria:
  - `web-intl` is a real backend for the public date and relative-time formatter surface.
  - Browser builds use `Intl.DateTimeFormat` for date and time formatting where the spec maps cleanly to browser capabilities.
  - Browser builds use `Intl.RelativeTimeFormat` for relative time output.
  - The public `DateFormatter` and related API stay stable across `icu4x` and `web-intl` builds.
  - Unsupported browser gaps are handled explicitly in code and spec rather than silently changing semantics.
  - CI and verification include the wasm `web-intl` cargo check path required by the spec.
- Spec impact: `Likely yes` if browser APIs cannot exactly match every ICU4X-oriented option or calendar behavior currently described.

#### W5-3: Implement web-intl plural and ordinal rules backend in ars-i18n

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #75
- Spec refs:
  - `spec/foundation/04-internationalization.md`
  - `spec/testing/14-ci.md`
- Goal: implement browser `Intl.PluralRules` support for plural and ordinal category selection behind the existing `ars-i18n` contract.
- Files to create/modify:
  - `crates/ars-i18n/Cargo.toml`
  - `crates/ars-i18n/src/lib.rs`
  - `crates/ars-i18n/src/plural.rs` and/or `crates/ars-i18n/src/web_intl/plural.rs`
  - `spec/foundation/04-internationalization.md`
  - `spec/testing/14-ci.md` only if checks/spec drift need sync
- Tests to add first:
  - Feature-gated compile coverage proving `ars-i18n` builds with `--no-default-features --features web-intl --target wasm32-unknown-unknown`.
  - Tests for representative cardinal and ordinal category selection across multiple locales.
  - Tests proving `icu4x` and `web-intl` are mutually exclusive.
- Acceptance criteria:
  - `web-intl` is a real backend for plural and ordinal rules.
  - Browser builds use `Intl.PluralRules` for cardinal and ordinal category selection.
  - The public plural-category API stays stable across `icu4x` and `web-intl` builds.
  - Any browser and ICU naming or behavior mismatches are normalized at the `ars-i18n` API boundary.
  - CI and verification include the wasm `web-intl` cargo check path required by the spec.
- Spec impact: `No` — spec §9.4 already updated with web-intl parity definitions.

#### W5-4: Implement web-intl StringCollator backend in ars-i18n

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #136 (ICU4X StringCollator defines the public API)
- Spec refs:
  - `spec/foundation/04-internationalization.md` §8, §9.4
- Goal: implement browser `Intl.Collator`-backed StringCollator for WASM client builds.
- Acceptance criteria:
  - `StringCollator::new()` uses `Intl.Collator` under `web-intl`.
  - `CollationStrength` maps to Intl.Collator sensitivity: Primary→"base", Secondary→"accent", Tertiary→"case", Quaternary→"variant".
  - `numeric: true` produces natural sort order.
  - Builds with `--no-default-features --features web-intl --target wasm32-unknown-unknown`.
- Spec impact: `No` — spec §8 already updated with web-intl backend definition.

#### W5-5: Implement web-intl case transformation backend in ars-i18n

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #133 (ICU4X case transformation defines the public API)
- Spec refs:
  - `spec/foundation/04-internationalization.md` §6.4, §9.4
- Goal: implement browser `toLocaleUpperCase`/`toLocaleLowerCase`-backed case transformation for WASM client builds.
- Acceptance criteria:
  - `to_uppercase`/`to_lowercase` use `js_sys::JsString` locale methods under `web-intl`.
  - Same public function signatures as ICU4X backend.
  - Turkish dotted-I and German eszett tests pass via browser.
  - Builds with `--no-default-features --features web-intl --target wasm32-unknown-unknown`.
- Spec impact: `No` — spec §6.4 already updated with web-intl dispatch.

#### W5-6: Implement WebIntlBackend for IntlBackend trait in ars-i18n

- Points: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: #138 (IntlBackend full trait definition)
- Spec refs:
  - `spec/foundation/04-internationalization.md` §9.5.4
- Goal: implement browser-backed `IntlBackend` using Intl APIs for WASM client builds, providing full parity with `Icu4xBackend`.
- Acceptance criteria:
  - All 11 `IntlBackend` methods implemented under `web-intl`.
  - Clean-mapping methods (weekday/month labels, digits, hourCycle) produce locale-correct output.
  - Fallback methods (max_months, days_in_month, convert_date) have documented strategies per spec §9.5.4.
  - `default_backend()` returns `WebIntlBackend` under `web-intl`.
  - Builds with `--no-default-features --features web-intl --target wasm32-unknown-unknown`.
- Spec impact: `No` — spec §9.5.4 already added with full `WebIntlBackend` definition.

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
Wave 4 (72 pts)            ┌─── Wave 3 complete
  #76 (long press, 3)      │
  #77 (move, 3)            │
  #162 (keyboard types, 2) │
  #159 (DnD types, 2)      │
    └─→ #160 (DnD SM, 3)  │  depends on #159, #76
         └─→ #161 (kbd DnD, 3)
  #79 (number fmt, 5)      │
  #80 (date fmt, 8*)       │  * decompose before pickup
  #81 (async collection, 5)│
  #82 (virtualizer, 8*)    │  * decompose before pickup
  #83 (tree collection, 5) │
  #84 (filter/sort, 3)     │
  #85 (media query, 2)     │
  #176 (url sanitize, 2)   │
  #150 (focus zone, 5)     │  A11y Wave 4 (all complete)
  #151 (dom event, 3)      │
  #152 (visually hidden, 1)│
  #153 (field context, 3)  │
  #154 (announcements, 2)  │
  #156 (aria validator, 3) │
    ├─→ #155 (touch, 2)    │  depends on #151
    └─→ #157 (kbd test, 3) │  depends on #150, #151
  #554 (aria asserts, 3)   │  A11y Wave 5 (audit follow-up)
  #555 (set_readonly, 1)   │
  #556 (focus selectors, 1)│
                           ▼
Wave 5 (13 pts)            ┌─── Wave 4 i18n tasks available
  #124 (web number, 5)     │
  #125 (web date/rtf, 5)   │
  #126 (web plural, 3)     │
                           ▼
            ┌─── All waves complete
            │
            ▼
    Decompose #24 → Component work begins
```

## Epic Mapping

| Epic                 | Issue | Tasks covered                                                        |
| -------------------- | ----- | -------------------------------------------------------------------- |
| Interactions         | #4    | #57, #58, #59, #60, #61, #65, #76, #77, #90, #159, #160, #161, #162  |
| DOM utilities        | #6    | #66, #67, #68, #69, #72, #74, #85, #88, #112, #113, #114, #115, #176 |
| Leptos adapter       | #8    | #55, #105, #190, #191, #513                                          |
| Dioxus adapter       | #9    | #56, #106, #193, #194, #195, #196, #197, #512                        |
| A11y                 | #3    | #73, #89, #150, #151, #152, #153, #154, #155, #156, #157, #554, #555 |
| DOM utilities (a11y) | #6    | #556                                                                 |
| Collections          | #53   | #62, #63, #64, #70, #71, #81, #82, #83, #84                          |
| I18n                 | #54   | #75, #79, #80, #124, #125, #126                                      |
| First utility slice  | #10   | #104                                                                 |

## Post-Foundation Plan

After all five waves are complete:

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
  - `disable_outside_pointer_events` behavior supported without breaking keyboard dismissal.
    - Dioxus Web/Desktop behavior and cleanup ordering match the spec and adapter contract.
- Spec impact: `No spec change required`.

### Targeted Follow-On: Leptos Adapter Foundation Completion

These cards complete the remaining foundational infrastructure in `ars-leptos` that every
component will depend on. They were identified by an audit of `spec/foundation/08-adapter-leptos.md`
against the 3 original Epic #8 tasks (2026-04-10), plus the adapter-owned live announcer
follow-up discovered while implementing `#73` (2026-04-11).

| GitHub                                               | Title                                                                                     | Points | Epic | Deps      |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------- | ------ | ---- | --------- |
| [#190](https://github.com/fogodev/ars-ui/issues/190) | Implement ArsProvider context, reactive props, and controlled value helper in ars-leptos  | 5      | #8   | —         |
| [#191](https://github.com/fogodev/ars-ui/issues/191) | Implement Leptos adapter utilities — emit, event mapping, nonce collector, safe listeners | 3      | #8   | #190      |
| [#513](https://github.com/fogodev/ars-ui/issues/513) | Implement LiveAnnouncer context bridge in ars-leptos                                      | 2      | #8   | #190, #73 |

#### PF-LA-1: Implement ArsProvider context, reactive props, and controlled value helper in ars-leptos

- Points: `5`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Unit`
- Depends on: none (all core dependencies exist: `Service::set_props()` in ars-core, `ArsContext` in ars-core, `Locale`/`IntlBackend`/`Direction` in ars-i18n)
- Spec refs:
  - `spec/foundation/08-adapter-leptos.md` §13 "ArsProvider Context" (L1721)
  - `spec/foundation/08-adapter-leptos.md` §13.1 `use_locale()` (L1764)
  - `spec/foundation/08-adapter-leptos.md` §13.2 `resolve_locale()` (L1793)
  - `spec/foundation/08-adapter-leptos.md` §13.3 `t()` — Translatable Text Resolver (L1812)
  - `spec/foundation/08-adapter-leptos.md` §3.3 "Reactive Props Variant" (L442)
  - `spec/foundation/08-adapter-leptos.md` §16 "Controlled Value Helper" (L1922)
- Goal: implement the ArsProvider reactive context bridge, complete `use_machine_with_reactive_props` (currently a `todo!()` stub), and add the `use_controlled_prop` DRY helper.
- Files to create/modify: `crates/ars-leptos/src/provider.rs` (new), `crates/ars-leptos/src/controlled.rs` (new), `crates/ars-leptos/src/use_machine.rs`, `crates/ars-leptos/src/lib.rs`, `crates/ars-leptos/src/prelude.rs`
- Tests to add first:
  - Unit tests for `use_locale()` fallback to `en-US` when no `ArsProvider` present.
  - Unit tests for `resolve_locale()` preferring per-instance override over context.
  - Unit tests for `use_controlled_prop()` skipping initial value and dispatching on change.
  - Unit tests for `use_machine_with_reactive_props()` syncing external prop changes to the machine service.
  - Unit tests for `use_machine_inner()` resolving locale and ICU provider from ArsProvider context.
- Acceptance criteria:
  - `ArsContext` struct with reactive signals matching spec §13.
  - `use_locale()`, `use_icu_provider()`, `resolve_locale()`, `resolve_messages()`, `t()` implemented.
  - `use_machine_with_reactive_props` fully implemented (replacing `todo!()` stub).
  - `use_controlled_prop(signal, send, event_fn)` helper with skip-initial and previous-value tracking.
  - `use_machine_inner` reads environment from `ArsProvider` context.
- Spec impact: `No spec change required`.

#### PF-LA-2: Implement Leptos adapter utilities — emit, event mapping, nonce collector, safe listeners

- Points: `3`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Unit`
- Depends on: #190 (`ArsNonceStyle` needs `ArsNonceCssCtx` which is provided alongside `ArsProvider`)
- Spec refs:
  - `spec/foundation/08-adapter-leptos.md` §10 "Event Callbacks Pattern" (L1595)
  - `spec/foundation/08-adapter-leptos.md` §14 "Event Mapping" (L1841)
  - `spec/foundation/08-adapter-leptos.md` §4.5.1 "Nonce CSS Collector" (L830)
  - `spec/foundation/08-adapter-leptos.md` §7.5 "Effect Cleanup and Event Safety" (L1395)
- Goal: implement the remaining small foundational utilities that many components depend on.
- Files to create/modify: `crates/ars-leptos/src/callbacks.rs` (new), `crates/ars-leptos/src/event_mapping.rs` (new), `crates/ars-leptos/src/nonce.rs` (new), `crates/ars-leptos/src/safe_listener.rs` (new), `crates/ars-leptos/src/lib.rs`
- Tests to add first:
  - Unit tests for `emit()` with `Some(callback)` and `None`.
  - Unit tests for `emit_map()` applying transform before dispatch.
  - Unit tests for `leptos_key_to_keyboard_key()` mapping common DOM key strings.
  - Unit tests for `ArsNonceCssCtx` accumulating CSS rules.
  - Unit tests for `use_safe_event_listener` cleanup idempotency and weak-guard pattern.
- Acceptance criteria:
  - `emit()` and `emit_map()` helpers implemented.
  - `leptos_key_to_keyboard_key()` correctly maps DOM key strings via `KeyboardKey::from_key_str()`.
  - `ArsNonceCssCtx`, `ArsNonceStyle` component, and `append_nonce_css()` implemented.
  - `use_safe_event_listener()` with weak-guard, idempotent cleanup, and two-phase lifecycle.
- Spec impact: `No spec change required`.

#### PF-LA-3: Implement LiveAnnouncer context bridge in ars-leptos

- Points: `2`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Unit`
- Depends on: #190, #73
- Spec refs:
  - `spec/foundation/03-accessibility.md` §5.1 "LiveAnnouncer"
  - `spec/foundation/11-dom-utilities.md` announcement helpers
  - `spec/foundation/08-adapter-leptos.md` §13 "ArsProvider Context"
- Goal: make announcement ownership adapter-provided rather than global by publishing a shared `Rc<RefCell<ars_a11y::LiveAnnouncer>>` through Leptos context and wiring adapter announcement delivery to that instance.
- Files to create/modify: `crates/ars-leptos/src/provider.rs` or equivalent adapter context module, any Leptos-side announcement bridge needed to satisfy the spec without globals, related spec text if the cross-crate contract needs clarification
- Tests to add first:
  - Unit test that a provided `LiveAnnouncer` context is discoverable from the Leptos adapter surface.
  - Unit test that missing provider state degrades gracefully without global fallback.
  - Unit test that repeated announcements reuse the same provided announcer instance instead of hidden singleton state.
- Acceptance criteria:
  - The Leptos adapter owns announcer provisioning via context.
  - No process-global or thread-local singleton is used for announcements.
  - Runtime announcement flow uses the shared `ars_a11y::LiveAnnouncer` instance rather than duplicating queue state in `ars-dom`.
  - Any required spec clarification is updated in the same task.
- Spec impact: `Update shared adapter/DOM wording if the announcement contract needs clarification`.

### Targeted Follow-On: Dioxus Adapter Foundation Completion

These tasks close the foundation gaps in the Dioxus adapter (Epic #9). The first three
are symmetric with Leptos #190/#191/#513; the remaining three cover Dioxus-unique sections
(multi-platform, SSR hydration, error boundary). All are sub-issues of Epic #9.

| GitHub                                               | Title                                                                           | Points | Epic | Deps      |
| ---------------------------------------------------- | ------------------------------------------------------------------------------- | ------ | ---- | --------- |
| [#193](https://github.com/fogodev/ars-ui/issues/193) | ArsProvider context, reactive props, controlled value helper in ars-dioxus      | 5      | #9   | —         |
| [#194](https://github.com/fogodev/ars-ui/issues/194) | Dioxus adapter utilities — emit, event mapping, nonce collector, safe listeners | 3      | #9   | #193      |
| [#512](https://github.com/fogodev/ars-ui/issues/512) | Implement LiveAnnouncer context bridge in ars-dioxus                            | 2      | #9   | #193, #73 |
| [#195](https://github.com/fogodev/ars-ui/issues/195) | DioxusPlatform trait, platform implementations, use_platform() hook             | 3      | #9   | #193      |
| [#196](https://github.com/fogodev/ars-ui/issues/196) | SSR Hydration support in ars-dioxus                                             | 3      | #9   | #193      |
| [#197](https://github.com/fogodev/ars-ui/issues/197) | ArsErrorBoundary component in ars-dioxus                                        | 2      | #9   | —         |

#### PF-DA-1: Implement ArsProvider context, reactive props, and controlled value helper in ars-dioxus

- Points: `5`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/09-adapter-dioxus.md` §16 "ArsProvider Context" (L2186)
  - `spec/foundation/09-adapter-dioxus.md` §16.1 `use_locale()` (L2224)
  - `spec/foundation/09-adapter-dioxus.md` §16.2 "Environment Resolution Utilities" (L2248)
  - `spec/foundation/09-adapter-dioxus.md` §2.3 "Reactive Props Sync" (L435)
  - `spec/foundation/09-adapter-dioxus.md` §19 "Controlled Value Helper" (L2448)
- Goal: implement the ArsProvider reactive context bridge, complete `use_machine_with_reactive_props` (currently `todo!()`), and add controlled value helpers.
- Files to create/modify: `crates/ars-dioxus/src/provider.rs` (new), `crates/ars-dioxus/src/controlled.rs` (new), `crates/ars-dioxus/src/use_machine.rs`, `crates/ars-dioxus/src/lib.rs`
- Tests to add first:
  - Unit tests for `use_locale()` fallback to `en-US` when no `ArsProvider` present.
  - Unit tests for `resolve_locale()` preferring per-instance override over context.
  - Unit tests for `use_controlled_prop_sync()` skipping initial value and dispatching on change.
  - Unit tests for `use_sync_props()` syncing external prop changes to the machine service.
  - Unit tests for `use_machine_inner()` resolving locale and ICU provider from ArsProvider context.
- Acceptance criteria:
  - `ArsContext` struct with reactive signals matching spec §16.
  - `use_locale()`, `use_icu_provider()`, `resolve_locale()`, `resolve_messages()` implemented.
  - `use_sync_props` fully implemented (replacing `todo!()` stub) with deadlock-safe `try_write()` fallback.
  - `use_controlled_prop_sync()` and `use_controlled_prop_sync_optional()` helpers with body-level sync.
  - `use_machine_inner` reads environment from `ArsProvider` context.
- Spec impact: `No spec change required`.

#### PF-DA-2: Implement Dioxus adapter utilities — emit, event mapping, nonce collector, safe listeners

- Points: `3`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Unit`
- Depends on: #193 (`ArsNonceStyle` needs `ArsNonceCssCtx` provided alongside `ArsProvider`)
- Spec refs:
  - `spec/foundation/09-adapter-dioxus.md` §19.1 "Event Callback Helper" (L2501)
  - `spec/foundation/09-adapter-dioxus.md` §13.1 "Event Mapping" (L1905)
  - `spec/foundation/09-adapter-dioxus.md` §3.5.1 "Nonce CSS Collector" (L864)
  - `spec/foundation/09-adapter-dioxus.md` §10 "Effect Cleanup and Event Safety" (L1698)
- Goal: implement the remaining small foundational utilities that many components depend on.
- Files to create/modify: `crates/ars-dioxus/src/callbacks.rs` (new), `crates/ars-dioxus/src/event_mapping.rs` (new), `crates/ars-dioxus/src/nonce.rs` (new), `crates/ars-dioxus/src/safe_listener.rs` (new), `crates/ars-dioxus/src/lib.rs`
- Tests to add first:
  - Unit tests for `emit()` with `Some(handler)` and `None`.
  - Unit tests for `emit_map()` applying transform before dispatch.
  - Unit tests for `dioxus_key_to_keyboard_key()` mapping Dioxus `Key` variants.
  - Unit tests for `ArsNonceCssCtx` accumulating CSS rules.
  - Unit tests for `use_safe_event_listener` cleanup idempotency and stale-check guard.
- Acceptance criteria:
  - `emit()` and `emit_map()` helpers implemented.
  - `dioxus_key_to_keyboard_key()` maps Dioxus `Key` variants to `KeyboardKey`.
  - `ArsNonceCssCtx`, `ArsNonceStyle` component, and `append_nonce_css()` wired to provider.
  - `use_safe_event_listener()` (web feature) with `Signal::try_read()` guard, idempotent cleanup, and two-phase lifecycle.
- Spec impact: `No spec change required`.

#### PF-DA-3: Implement LiveAnnouncer context bridge in ars-dioxus

- Points: `2`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Unit`
- Depends on: #193, #73
- Spec refs:
  - `spec/foundation/03-accessibility.md` §5.1 "LiveAnnouncer"
  - `spec/foundation/11-dom-utilities.md` announcement helpers
  - `spec/foundation/09-adapter-dioxus.md` §16 "ArsProvider Context"
- Goal: make announcement ownership adapter-provided rather than global by publishing a shared `Rc<RefCell<ars_a11y::LiveAnnouncer>>` through Dioxus context and wiring adapter announcement delivery to that instance across Dioxus targets.
- Files to create/modify: `crates/ars-dioxus/src/provider.rs` or equivalent adapter context module, any Dioxus-side announcement bridge needed to satisfy the spec without globals, related spec text if the cross-crate contract needs clarification
- Tests to add first:
  - Unit test that a provided `LiveAnnouncer` context is discoverable from the Dioxus adapter surface.
  - Unit test that missing provider state degrades gracefully without global fallback.
  - Unit test that repeated announcements reuse the same provided announcer instance instead of hidden singleton state.
- Acceptance criteria:
  - The Dioxus adapter owns announcer provisioning via context.
  - No process-global or thread-local singleton is used for announcements.
  - Runtime announcement flow uses the shared `ars_a11y::LiveAnnouncer` instance rather than duplicating queue state in `ars-dom`.
  - Any required spec clarification is updated in the same task.
- Spec impact: `Update shared adapter/DOM wording if the announcement contract needs clarification`.

#### PF-DA-4: Implement DioxusPlatform trait, platform implementations, and use_platform() hook

- Points: `3`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Unit`
- Depends on: #193 (`use_platform()` reads `dioxus_platform` from `ArsContext`)
- Spec refs:
  - `spec/foundation/09-adapter-dioxus.md` §6 "Multi-Platform Support" (L1268)
  - `spec/foundation/09-adapter-dioxus.md` §6.1 "Platform Abstraction Trait" (L1270)
  - `spec/foundation/09-adapter-dioxus.md` §6.2 "Platform Hook" (L1501)
  - `spec/foundation/09-adapter-dioxus.md` §6.3 "Platform Support Matrix" (L1534)
- Goal: implement the Dioxus-specific platform abstraction for cross-platform operations.
- Files to create/modify: `crates/ars-dioxus/src/platform.rs` (new), `crates/ars-dioxus/src/lib.rs`, `crates/ars-dioxus/Cargo.toml`
- Tests to add first:
  - Unit tests for `NullPlatform` no-op implementations.
  - Unit tests for `use_platform()` fallback chain.
  - Compile-gate tests for `WebPlatform` and `DesktopPlatform` feature gating.
- Acceptance criteria:
  - `DioxusPlatform` trait with 8 methods (focus, bounding rect, scroll, clipboard, file picker, timestamp, ID, drag data).
  - `WebPlatform` (web feature), `DesktopPlatform` (desktop feature), `NullPlatform` implementations.
  - `use_platform() -> Rc<dyn DioxusPlatform>` with feature-gated fallback.
- Spec impact: `No spec change required`.

#### PF-DA-5: Implement SSR Hydration support in ars-dioxus

- Points: `3`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Unit`
- Depends on: #193 (ArsProvider context and `use_machine` wiring)
- Spec refs:
  - `spec/foundation/09-adapter-dioxus.md` §20 "SSR Hydration Support" (L2567)
  - `spec/foundation/09-adapter-dioxus.md` §20.1 "FocusScope Hydration Handling" (L2569)
  - `spec/foundation/09-adapter-dioxus.md` §20.2 "HydrationSnapshot" (L2679)
  - `spec/foundation/09-adapter-dioxus.md` §19.2 "Hydration-Safe ID Generation" (L2526)
- Goal: implement SSR-to-client hydration support with FocusScope safety and state transfer.
- Files to create/modify: `crates/ars-dioxus/src/hydration.rs` (new), `crates/ars-dioxus/src/id.rs`, `crates/ars-dioxus/src/lib.rs`
- Tests to add first:
  - Unit tests for `HydrationSnapshot<M>` serde round-trip.
  - Unit tests for `setup_focus_scope_hydration_safe()` gating on `data-ars-hydrated`.
  - Unit tests for `use_stable_id()` deterministic format.
- Acceptance criteria:
  - `HydrationSnapshot<M>` with serde support.
  - `setup_focus_scope_hydration_safe()` implementing all 5 spec rules.
  - `use_stable_id(prefix)` with documented hydration-safety caveat.
- Spec impact: `No spec change required`.

#### PF-DA-6: Implement ArsErrorBoundary component in ars-dioxus

- Points: `2`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Unit`
- Depends on: none
- Spec refs:
  - `spec/foundation/09-adapter-dioxus.md` §21 "Error Boundary Pattern" (L2714)
- Goal: implement the `ArsErrorBoundary` wrapper component for graceful error handling.
- Files to create/modify: `crates/ars-dioxus/src/error_boundary.rs` (new), `crates/ars-dioxus/src/lib.rs`, `crates/ars-dioxus/src/prelude.rs`
- Tests to add first:
  - Unit tests for children rendering when no error.
  - Unit tests for fallback with `data-ars-error="true"` and `role="alert"`.
- Acceptance criteria:
  - `ArsErrorBoundary` wrapping Dioxus `ErrorBoundary`.
  - Accessible fallback UI with error message display.
  - Exported in `ars_dioxus::prelude`.
- Spec impact: `No spec change required`.

## Summary

| Wave      | Tasks  | Points  | Unlocks                                                  |
| --------- | ------ | ------- | -------------------------------------------------------- |
| Wave 1    | 7      | 19      | Button, VisuallyHidden, Separator                        |
| Wave 2    | 12     | 43      | Select, Combobox, Menu, Listbox, Dialog, Popover         |
| Wave 3    | 9      | 29      | Tooltip, DatePicker prerequisites, accessibility         |
| Wave 4    | 37     | 113     | All remaining components (Slider, TreeView, Table, etc.) |
| Wave 5-C  | 4      | 12      | Mutable collections, announcements, DnD, coverage        |
| Wave 5    | 6      | 23      | Browser Intl backends for WASM client builds             |
| Post-F IO | 3      | 13      | Dismissable for overlays                                 |
| Post-F LA | 2      | 8       | Leptos adapter foundation completion                     |
| Post-F DA | 5      | 16      | Dioxus adapter foundation completion                     |
| **Total** | **85** | **276** | **Complete foundation for all 112 components**           |
