---
adapter: leptos
component: pin-input
category: input
source: components/input/pin-input.md
source_foundation: foundation/08-adapter-leptos.md
---

# PinInput — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`PinInput`](../../components/input/pin-input.md) contract onto a Leptos 0.8.x component. The adapter must preserve repeated per-cell inputs, roving focus, OTP-friendly attrs, and hidden-input form submission of the combined code.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn PinInput(
    #[prop(optional, into)] value: Option<Signal<Vec<String>>>,
    #[prop(optional)] default_value: Vec<String>,
    #[prop(optional)] length: usize,
    #[prop(optional)] mode: pin_input::InputMode,
    #[prop(optional)] otp: bool,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] form: Option<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] invalid: Signal<bool>,
    children: Children,
) -> impl IntoView
```

The adapter also forwards locale or messages and shared field props from the core contract. `value`, `disabled`, and `invalid` are the normal reactive surface; repeated cell count is structural and should not change casually after mount.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core pin-input contract, including per-cell mode, OTP behavior, and form naming.
- Event parity: `InputChar`, `DeleteChar`, focus navigation, `Clear`, `CompositionStart`, and `CompositionEnd` remain machine-owned.
- Core machine ownership: `use_machine::<pin_input::Machine>(...)` owns the per-cell value vector, focus index, and completed-state semantics.

## 4. Part Mapping

| Core part / structure  | Required? | Adapter rendering target | Ownership     | Attr source                 | Notes                                   |
| ---------------------- | --------- | ------------------------ | ------------- | --------------------------- | --------------------------------------- |
| `Root`                 | required  | `<div>`                  | adapter-owned | `api.root_attrs()`          | owns `role="group"` semantics           |
| `Label`                | required  | `<label>`                | adapter-owned | `api.label_attrs()`         | group label                             |
| `Input { cell_index }` | repeated  | native `<input>`         | adapter-owned | `api.input_attrs(index)`    | one input per cell with roving tabindex |
| `HiddenInput`          | required  | `<input type="hidden">`  | adapter-owned | `api.hidden_input_attrs()`  | submits combined value                  |
| `Description`          | optional  | `<div>`                  | adapter-owned | `api.description_attrs()`   | described-by content                    |
| `ErrorMessage`         | optional  | `<div>`                  | adapter-owned | `api.error_message_attrs()` | invalid-only content                    |

## 5. Attr Merge and Ownership Rules

- Per-cell `maxlength`, `inputmode`, `autocomplete`, `tabindex`, and ARIA attrs always win.
- `class` and `style` merge additively on `Root` and each cell input.
- Consumer decoration may style cells, but it must not break stable cell index identity or hidden-input ownership.

## 6. Composition / Context Contract

`PinInput` is compound but self-contained. All cells, the combined hidden input, and focus navigation belong to one local machine instance.

## 7. Prop Sync and Event Mapping

| Adapter prop               | Mode          | Sync trigger                 | Machine event / update path  | Visible effect                                       |
| -------------------------- | ------------- | ---------------------------- | ---------------------------- | ---------------------------------------------------- |
| `value`                    | controlled    | signal change                | `SetValue`                   | updates all cells, hidden input, and completed state |
| `disabled` / `invalid`     | controlled    | signal change                | `SetDisabled` / `SetInvalid` | updates guards and ARIA attrs                        |
| per-cell input or deletion | machine-owned | native input, keydown, paste | `InputChar` / `DeleteChar`   | updates vector and moves focus                       |

IME composition applies at the cell level. OTP autocomplete and paste fan-out remain adapter-owned translation layers around the machine events.

## 8. Registration and Cleanup Contract

- Repeated cell refs must be kept in stable index order.
- No global listeners are required.
- Paste fan-out and focus-move helpers are local to the instance and must be dropped on unmount.

## 9. Ref and Node Contract

- Each cell input owns a live ref indexed by cell position.
- The hidden input does not participate in focus navigation.
- Focus repair after input or deletion always targets a cell input, never the hidden input.

## 10. State Machine Boundary Rules

- The machine owns the per-cell value vector, focused cell, and completed state.
- The adapter may translate paste text into per-cell events, but it must not commit combined string state outside the machine.
- Hidden-input value is always the machine-owned combined string projection.

## 11. Callback Payload Contract

- Value-change callbacks emit the committed `Vec<String>` cell vector or a documented combined-string derivative if a wrapper exposes one.
- Completion callbacks fire only when the machine reaches completed state.
- Focus-change callbacks, if exposed, must reflect the machine-chosen cell index.

## 12. Failure and Degradation Rules

| Condition                                                            | Policy             | Notes                                        |
| -------------------------------------------------------------------- | ------------------ | -------------------------------------------- |
| controlled and uncontrolled value props are mixed after mount        | warn and ignore    | first mode wins                              |
| `length` changes after mount in a way that invalidates cell identity | fail fast          | repeated structure identity must stay stable |
| OTP autocomplete or clipboard APIs are unavailable                   | degrade gracefully | manual cell entry still works                |

## 13. Identity and Key Policy

`Root` owns one machine instance and each cell input is keyed by `cell_index`. Cell count must remain stable for the lifetime of the instance unless the component is remounted intentionally.

## 14. SSR and Client Boundary Rules

- SSR renders the same number of cell inputs and the same hidden input structure as the client.
- Client-only paste fan-out, focus movement, and OTP autofill behavior activate after hydration.
- Cell ordering must remain stable between server and client.

## 15. Performance Constraints

- Keep cell refs in index order without rebuilding the whole ref array on every keystroke.
- Avoid duplicating the combined string outside the machine except when deriving the hidden-input value.
- Paste fan-out should dispatch only the necessary machine events for the received text.

## 16. Implementation Dependencies

| Dependency          | Required? | Dependency type     | Why it must exist first                           |
| ------------------- | --------- | ------------------- | ------------------------------------------------- |
| hidden-input helper | required  | submission contract | combined code must submit natively                |
| roving-focus helper | required  | adapter helper      | repeated cell focus movement must stay consistent |

## 17. Recommended Implementation Sequence

1. Initialize the machine and derive root, label, per-cell, and hidden-input attrs.
2. Allocate stable cell refs by index.
3. Render repeated cell inputs and the hidden input in documented order.
4. Wire controlled prop synchronization, input or deletion events, paste fan-out, and focus movement.
5. Finish OTP, invalid-state, and completion behavior.

## 18. Anti-Patterns

- Do not treat the hidden input as a visible or focusable control.
- Do not derive cell identity from DOM order alone when `cell_index` already exists.
- Do not bypass the machine with direct combined-string mutation.

## 19. Consumer Expectations and Guarantees

- Consumers may assume cell order and focus movement are index-stable.
- Consumers may assume the hidden input always submits the combined code.
- Consumers must not assume undocumented cell-wrapper nodes or dynamic cell-count changes without remounting.

## 20. Platform Support Matrix

| Capability / behavior                                                                   | Browser client | SSR          | Notes                                              |
| --------------------------------------------------------------------------------------- | -------------- | ------------ | -------------------------------------------------- |
| documented cell input semantics, OTP attrs, focus movement, and hidden-input submission | full support   | full support | clipboard and OTP helpers activate after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                             |
| ----------------------------------------------- | -------------------- | ------------------- | --------------------------------- |
| controlled/uncontrolled mode switch after mount | debug warning        | warn and ignore     | preserves current mode            |
| cell-count identity violated after mount        | fail fast            | fail fast           | protects focus and value indexing |

## 22. Shared Adapter Helper Notes

| Helper concept      | Required? | Responsibility                                  | Notes                                       |
| ------------------- | --------- | ----------------------------------------------- | ------------------------------------------- |
| roving-focus helper | required  | move focus between cells based on machine state | shared with other repeated-input composites |
| hidden-input helper | required  | derive combined string submission value         | shared with other synthetic-form bridges    |

## 23. Framework-Specific Behavior

Leptos should keep cell refs in a stable indexed collection, use native `input` and `keydown` events for per-cell updates, and gate clipboard or OTP integration behind client-only code paths.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<pin_input::Machine>(props);

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <For each=cell_indexes key=|i| *i let:index>
            <input {..machine.derive(move |api| api.input_attrs(index)).get()} />
        </For>
        <input {..machine.derive(|api| api.hidden_input_attrs()).get()} />
    </div>
}
```

## 25. Reference Implementation Skeleton

- Initialize the machine and allocate stable index-keyed refs for every cell.
- Route cell typing, deletion, paste fan-out, and focus movement through machine events only.
- Keep the combined hidden-input submission value derived from the same machine-owned vector.

## 26. Adapter Invariants

- Cell order and identity are always index-stable.
- The machine owns the per-cell vector and completed state.
- Hidden-input submission always reflects the machine-owned combined string.
- Focus movement always targets cell inputs only.

## 27. Accessibility and SSR Notes

- Each cell label must remain localized and index-aware.
- Group-level description-first, error-second ordering is mandatory for `aria-describedby`.
- SSR must preserve the same cell count and ordering used by the client.

## 28. Parity Summary and Intentional Deviations

- Matches the core pin-input contract without intentional divergence.
- Promotes cell identity, paste fan-out, OTP attrs, and hidden-input submission rules into Leptos-facing guidance.

## 29. Test Scenarios

- Typing across cells advances focus and updates the hidden combined value.
- Deletion moves focus backward and clears the correct cell.
- Paste fan-out distributes characters across cells in order.
- Controlled value updates synchronize all cells and completion state without identity drift.

## 30. Test Oracle Notes

- Inspect per-cell input attrs and hidden-input combined value in the DOM.
- Use focus assertions on cell refs to verify forward and backward movement.
- Verify combined-code callbacks from machine-driven logs rather than from raw DOM concatenation.

## 31. Implementation Checklist

- [ ] Keep repeated cell identity stable by `cell_index`.
- [ ] Keep the hidden input adapter-owned and non-focusable.
- [ ] Route cell input, deletion, paste, and focus movement through machine events.
- [ ] Preserve OTP attrs and localized per-cell labeling.
- [ ] Keep combined submission value derived from machine state only.
- [ ] Preserve group-level described-by ordering and invalid semantics.
