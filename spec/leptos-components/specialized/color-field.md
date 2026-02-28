---
adapter: leptos
component: color-field
category: specialized
source: components/specialized/color-field.md
source_foundation: foundation/08-adapter-leptos.md
---

# ColorField — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ColorField`](../../components/specialized/color-field.md) contract onto Leptos `0.8.x`. The adapter preserves whole-color text entry, channel-editing mode, parsing and validation feedback, description and error wiring, and hidden-input form submission.

## 2. Public Adapter API

```rust
#[component]
pub fn ColorField(
    #[prop(optional)] value: Option<RwSignal<Option<ColorValue>>>,
    #[prop(optional)] default_value: Option<ColorValue>,
    #[prop(optional)] format: ColorFormat,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] readonly: bool,
    #[prop(optional)] required: bool,
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional, into)] description: Option<String>,
) -> impl IntoView
```

The adapter renders the text input surface directly and owns the optional description, error message, and hidden-input nodes.

## 3. Mapping to Core Component Contract

- Props parity: full parity with bindable optional color value, format, validation state, and field labeling.
- Part parity: full parity with `Root`, `Label`, `Input`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Adapter additions: explicit IME-safe commit behavior and aria-describedby merging.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source                 | Notes                                     |
| --------------------- | --------- | ------------------------ | ------------- | --------------------------- | ----------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`          | owns invalid/focused state markers        |
| `Label`               | optional  | `<label>`                | adapter-owned | `api.label_attrs()`         | render when `label` exists                |
| `Input`               | required  | `<input type="text">`    | adapter-owned | `api.input_attrs()`         | whole-color or channel-entry text surface |
| `Description`         | optional  | `<div>`                  | adapter-owned | `api.description_attrs()`   | contributes to `aria-describedby`         |
| `ErrorMessage`        | optional  | `<div>`                  | adapter-owned | `api.error_message_attrs()` | `role="alert"` when invalid               |
| `HiddenInput`         | optional  | `<input type="hidden">`  | adapter-owned | `api.hidden_input_attrs()`  | submits normalized hex value              |

## 5. Attr Merge and Ownership Rules

Input `aria-invalid`, `aria-required`, labeling, and described-by wiring always win. Consumer decoration must not break the linkage among `Label`, `Description`, `ErrorMessage`, and `Input`.

## 6. Composition / Context Contract

`ColorField` is standalone and also composes through shared color binding with `ColorPicker`. No parent context is required.

## 7. Prop Sync and Event Mapping

Controlled value sync flows through the writable optional color signal. Text input events update draft text; commit events parse into a normalized color or channel update. Focus and blur dispatch the core focus semantics. IME composition must not commit mid-composition.

## 8. Registration and Cleanup Contract

No global listeners are required. Cleanup is ordinary node disposal, but transient composition state must not leak past blur or unmount.

## 9. Ref and Node Contract

No live measurement ref is required. The input node is adapter-owned and may be exposed only through future wrapper APIs, not this base surface.

## 10. State Machine Boundary Rules

- machine-owned state: current value, draft text, invalid state, focus state, and channel-edit mode.
- adapter-local derived bookkeeping: IME composition flag only.
- forbidden local mirrors: do not keep a second parsed color value outside the machine.

## 11. Callback Payload Contract

No dedicated callback is required; value observation happens through the controlled signal or hidden input.

## 12. Failure and Degradation Rules

Parsing failures degrade into the documented invalid state and `ErrorMessage`; they must not panic or silently coerce to a different color.

## 13. Identity and Key Policy

Description and error nodes are instance-scoped. Their ids must remain stable while mounted so `aria-describedby` does not churn.

## 14. SSR and Client Boundary Rules

SSR renders the full field structure, including description, error placeholder when present, and hidden input. No client-only listeners beyond ordinary input and focus events are required.

## 15. Performance Constraints

Do not reparse unchanged text on every render. Recompute described-by ids only when optional description or error parts appear or disappear.

## 16. Implementation Dependencies

| Dependency             | Required?   | Dependency type      | Why it must exist first                        | Notes                           |
| ---------------------- | ----------- | -------------------- | ---------------------------------------------- | ------------------------------- |
| color parser/formatter | required    | helper               | normalizes text entry and hidden-input output  | shared with `color-picker`      |
| field-ids helper       | recommended | accessibility helper | keeps label, description, and error ids stable | shared with field-like controls |

## 17. Recommended Implementation Sequence

1. Render root, input, optional label, and hidden input.
2. Add text sync, parse-on-commit, and IME guards.
3. Add description and error-message linking.
4. Verify standalone and composed shared-signal use.

## 18. Anti-Patterns

- Do not commit partial IME composition text.
- Do not bypass the machine by parsing directly in multiple local handlers.

## 19. Consumer Expectations and Guarantees

- Consumers may assume invalid text remains visible for correction.
- Consumers may assume the hidden input carries normalized color output.
- Consumers must not assume every typed string parses immediately.

## 20. Platform Support Matrix

| Capability / behavior                       | Browser client | SSR            | Notes                        |
| ------------------------------------------- | -------------- | -------------- | ---------------------------- |
| text input, labels, description, and errors | full support   | full support   | no client-only geometry work |
| IME-safe commit behavior                    | full support   | not applicable | runtime input path only      |

## 21. Debug Diagnostics and Production Policy

Duplicate described-by ids or invalid linkage are fail-fast. Parse failures are normal runtime behavior and stay in the documented invalid state.

## 22. Shared Adapter Helper Notes

Use one parser/formatter helper and one field-id helper so `ColorField` stays aligned with `ColorPicker` and form-bound controls.

## 23. Framework-Specific Behavior

Leptos should handle input and change events without creating a second local draft store outside the machine. IME composition can be tracked in a small local flag only.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn ColorField() -> impl IntoView {
    let machine = use_machine::<color_field::Machine>(color_field::Props::default());
    view! { <input {..machine.derive(|api| api.input_attrs()).get()} /> }
}
```

## 25. Reference Implementation Skeleton

- Bind the optional color value.
- Render the field structure with stable ids.
- Parse and commit only through the machine.
- Keep IME composition local and ephemeral.

## 26. Adapter Invariants

- Input parsing and normalization always flow through the machine.
- `aria-describedby` stays synchronized with description and error visibility.
- Invalid text is preserved rather than discarded.

## 27. Accessibility and SSR Notes

`ErrorMessage` is the authoritative invalid-state announcement surface. The input remains the only editable control node.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- whole-color text parsing and normalization
- channel-mode keyboard stepping
- invalid text with error-message linkage
- IME composition does not commit early
- hidden input reflects the normalized value

## 30. Test Oracle Notes

| Behavior        | Preferred oracle type | Notes                                                    |
| --------------- | --------------------- | -------------------------------------------------------- |
| field semantics | DOM attrs             | assert label, required, invalid, and described-by wiring |
| parse behavior  | rendered behavior     | assert valid and invalid commit paths                    |
| IME guard       | event-sequence test   | assert no commit before composition end                  |

## 31. Implementation Checklist

- [ ] Parsing and normalization stay machine-driven.
- [ ] Description and error ids stay stable.
- [ ] IME composition is guarded explicitly.
