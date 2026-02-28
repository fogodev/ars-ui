---
adapter: dioxus
component: color-swatch-picker
category: specialized
source: components/specialized/color-swatch-picker.md
source_foundation: foundation/09-adapter-dioxus.md
---

# ColorSwatchPicker — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ColorSwatchPicker`](../../components/specialized/color-swatch-picker.md) contract onto Dioxus `0.7.x`. The adapter preserves swatch-listbox semantics, roving focus, optional grid navigation, selection state, and hidden-input submission.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ColorSwatchPickerProps {
    pub colors: Vec<ColorValue>,
    #[props(optional)]
    pub value: Option<Signal<ColorValue>>,
    #[props(optional)]
    pub default_value: Option<ColorValue>,
    #[props(optional)]
    pub layout: Option<SwatchPickerLayout>,
    #[props(optional)]
    pub disabled: Option<bool>,
    #[props(optional)]
    pub readonly: Option<bool>,
}

#[component]
pub fn ColorSwatchPicker(props: ColorSwatchPickerProps) -> Element
```

The adapter renders the listbox root, repeated swatch options, and optional hidden input.

## 3. Mapping to Core Component Contract

- Props parity: full parity with swatch collection, selected value, layout, and disabled/read-only state.
- Part parity: full parity with `Root`, repeated `Item`, and `HiddenInput`.
- Adapter additions: explicit DOM-order roving focus rules and nested `ColorSwatch` composition guidance.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source                         | Notes                                      |
| --------------------- | --------- | ------------------------ | ------------- | ----------------------------------- | ------------------------------------------ |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`                  | `role="listbox"` focus container           |
| `Item`                | repeated  | `<div>`                  | adapter-owned | `api.item_attrs(index)`             | `role="option"` swatch option wrapper      |
| nested swatch         | repeated  | `<ColorSwatch>` output   | adapter-owned | composed from `ColorSwatch` adapter | decorative visual child inside each option |
| `HiddenInput`         | optional  | `<input type="hidden">`  | adapter-owned | `api.hidden_input_attrs()`          | form bridge using selected color           |

## 5. Attr Merge and Ownership Rules

Selection, roving tabindex, and option roles always win on `Item`. Nested `ColorSwatch` output remains non-interactive and must not replace the option semantics.

## 6. Composition / Context Contract

`ColorSwatchPicker` composes the standalone `ColorSwatch` adapter inside each option. No parent context is required, and no child context is published.

## 7. Prop Sync and Event Mapping

Controlled selection sync flows through the writable signal. Pointer or keyboard activation on an item dispatches selection events. Arrow-key behavior follows the configured stack or grid layout and keeps DOM-order focus registration explicit.

## 8. Registration and Cleanup Contract

Register options in DOM order on mount so roving focus and two-dimensional navigation can rely on stable indices. Cleanup removes option registrations on unmount.

## 9. Ref and Node Contract

Each item may need a live node handle only for focus movement; no measurement is required. Root and items remain adapter-owned.

## 10. State Machine Boundary Rules

- machine-owned state: selected value, focused item, disabled/read-only state, and layout-aware navigation.
- adapter-local derived bookkeeping: ordered item registration only.
- forbidden local mirrors: do not store a second selected value or focus index outside the machine.

## 11. Callback Payload Contract

No dedicated callback is required; selection changes are observed through the controlled signal or hidden input.

## 12. Failure and Degradation Rules

If option registration order becomes unavailable, fail fast in debug and degrade to DOM-order keyboard fallback in production.

## 13. Identity and Key Policy

Swatch option identity is the color value plus stable collection order. Reordering the collection changes focus order intentionally.

## 14. SSR and Client Boundary Rules

SSR renders the root, all options, and the hidden input. Focus movement is client-only, but selection state must hydrate without structural changes.

## 15. Performance Constraints

Keep registration incremental. Do not rebuild the entire option collection when only selection changes.

## 16. Implementation Dependencies

| Dependency          | Required?   | Dependency type | Why it must exist first                            | Notes                               |
| ------------------- | ----------- | --------------- | -------------------------------------------------- | ----------------------------------- |
| roving-focus helper | required    | focus helper    | keyboard navigation depends on stable option order | shared with option collections      |
| hidden-input helper | recommended | form helper     | keeps form submission consistent                   | shared with selection-like controls |

## 17. Recommended Implementation Sequence

1. Render root and repeated item shells.
2. Compose `ColorSwatch` inside each option.
3. Add roving focus and layout-aware keyboard navigation.
4. Add hidden-input synchronization.

## 18. Anti-Patterns

- Do not move selection semantics onto the nested swatch.
- Do not let registration order drift from DOM order.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the nested swatch is visual only.
- Consumers may assume keyboard navigation honors the chosen layout.
- Consumers must not assume arbitrary focus order independent of DOM order.

## 20. Platform Support Matrix

| Capability / behavior            | Web          | Desktop       | Mobile        | SSR          | Notes                                                 |
| -------------------------------- | ------------ | ------------- | ------------- | ------------ | ----------------------------------------------------- |
| listbox semantics and selection  | full support | full support  | full support  | full support | structural parity on SSR                              |
| roving focus and grid navigation | full support | full support  | full support  | client-only  | focus movement is runtime-only                        |
| hidden input                     | full support | fallback path | fallback path | full support | host form semantics outside web are adapter-dependent |

## 21. Debug Diagnostics and Production Policy

Missing or duplicate option registration is fail-fast in debug and warning-plus-best-effort focus fallback in production.

## 22. Shared Adapter Helper Notes

Use one ordered-registration helper for options and reuse the standalone `ColorSwatch` adapter for visuals rather than reimplementing swatch rendering.

## 23. Framework-Specific Behavior

Dioxus should keep option registration stable across rerenders and compose nested swatches without introducing extra focusable nodes.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ColorSwatchPickerSketchProps {
    pub colors: Vec<ColorValue>,
}

#[component]
pub fn ColorSwatchPicker(props: ColorSwatchPickerSketchProps) -> Element {
    let machine = use_machine::<color_swatch_picker::Machine>(color_swatch_picker::Props { colors: props.colors, ..Default::default() });
    let root_attrs = machine.derive(|api| api.root_attrs());
    rsx! { div { ..root_attrs.read().clone() } }
}
```

## 25. Reference Implementation Skeleton

- Bind the selected color value.
- Render the listbox root and option shells in stable order.
- Compose `ColorSwatch` inside each option.
- Register option focus targets and clean them up eagerly.

## 26. Adapter Invariants

- `Item` always owns the option semantics.
- Registration order always matches DOM order.
- Nested swatches never become independently interactive.

## 27. Accessibility and SSR Notes

`Root` remains the listbox surface and `Item` remains the option surface even when the visuals are delegated to `ColorSwatch`.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- stack and grid keyboard navigation
- pointer selection updates the shared value
- nested `ColorSwatch` remains non-interactive
- hidden input reflects the selected color

## 30. Test Oracle Notes

| Behavior          | Preferred oracle type | Notes                                                |
| ----------------- | --------------------- | ---------------------------------------------------- |
| listbox semantics | DOM attrs             | assert root role and option attrs                    |
| focus order       | keyboard navigation   | assert DOM-order roving focus                        |
| composition       | rendered structure    | assert nested swatch remains inside each option only |

## 31. Implementation Checklist

- [ ] Option registration stays DOM-ordered.
- [ ] Nested swatches remain decorative.
- [ ] Hidden input tracks the selected color.
