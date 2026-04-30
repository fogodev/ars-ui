---
adapter: leptos
component: editable
category: input
source: components/input/editable.md
source_foundation: foundation/08-adapter-leptos.md
---

# Editable — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Editable`](../../components/input/editable.md) contract onto a Leptos 0.8.x component. The adapter must preserve preview-versus-editing state, trigger wiring, IME-aware editing, and native input form participation while editing.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn Editable(
    #[prop(optional, into)] value: Option<Signal<String>>,
    #[prop(optional)] default_value: String,
    #[prop(optional)] activation_mode: editable::ActivateMode,
    #[prop(optional)] submit_mode: editable::SubmitMode,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] form: Option<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] readonly: Signal<bool>,
    #[prop(optional, into)] invalid: Signal<bool>,
    children: Children,
) -> impl IntoView
```

The adapter also exposes locale or messages and optional callbacks such as committed-value or editing-state changes. `value`, `disabled`, `readonly`, and `invalid` are the default reactive surface.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core editable contract, including activation and submission modes.
- Event parity: `Activate`, `Confirm`, `Cancel`, `InputChange`, `Focus`, `Blur`, `CompositionStart`, and `CompositionEnd` remain machine-owned.
- Core machine ownership: `use_machine::<editable::Machine>(...)` owns preview state, edit buffer state, and trigger visibility.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source                  | Notes                                            |
| --------------------- | --------- | ------------------------ | ------------- | ---------------------------- | ------------------------------------------------ |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`           | owns preview or editing state token              |
| `Label`               | optional  | `<label>`                | adapter-owned | `api.label_attrs()`          | linked to `Input` when present                   |
| `Preview`             | required  | `<span>`                 | adapter-owned | `api.preview_attrs()`        | visible in preview state                         |
| `Input`               | required  | native `<input>`         | adapter-owned | `api.input_attrs()`          | visible in editing state and can submit to forms |
| `EditTrigger`         | optional  | `<button>`               | adapter-owned | `api.edit_trigger_attrs()`   | preview-state trigger                            |
| `SubmitTrigger`       | optional  | `<button>`               | adapter-owned | `api.submit_trigger_attrs()` | editing-state confirm trigger                    |
| `CancelTrigger`       | optional  | `<button>`               | adapter-owned | `api.cancel_trigger_attrs()` | editing-state cancel trigger                     |

## 5. Attr Merge and Ownership Rules

- Root state attrs, input naming, and trigger button semantics always win.
- `class` and `style` merge additively across preview, input, and triggers.
- Consumer handlers may decorate preview or input content, but activation and confirm or cancel sequencing remain adapter-owned.

## 6. Composition / Context Contract

`Editable` is self-contained. It does not expose child contexts, and the preview, input, and trigger parts all belong to one local state machine instance.

## 7. Prop Sync and Event Mapping

| Adapter prop                        | Mode          | Sync trigger           | Machine event / update path                  | Visible effect                            |
| ----------------------------------- | ------------- | ---------------------- | -------------------------------------------- | ----------------------------------------- |
| `value`                             | controlled    | signal change          | `SetValue`                                   | updates committed preview and edit buffer |
| `disabled` / `readonly` / `invalid` | controlled    | signal change          | `SetDisabled` / `SetReadonly` / `SetInvalid` | updates guards and attrs                  |
| preview or trigger activation       | machine-owned | click, focus, keyboard | `Activate`                                   | enters editing state                      |
| confirm or cancel paths             | machine-owned | key or trigger         | `Confirm` / `Cancel`                         | commits or discards edit buffer           |

IME composition suppresses eager confirm behavior while editing.

## 8. Registration and Cleanup Contract

- No descendant registry is required.
- No global listeners are required.
- Any temporary edit-buffer-only helpers are dropped on unmount; there is no persistent hidden state outside the machine.

## 9. Ref and Node Contract

- `Input` owns the live editing ref.
- `Preview` may own a ref for re-focusing after cancel or confirm.
- Trigger refs are optional and may be used for focus restoration only.

## 10. State Machine Boundary Rules

- The machine owns preview or editing state and the transient edit buffer.
- The adapter must not commit or discard the edit buffer outside `Confirm` or `Cancel`.
- Native form submission, when editing, uses the committed input name and form attrs on the active `Input`.

## 11. Callback Payload Contract

- Commit callbacks emit the committed string value after `Confirm`.
- Editing-state callbacks emit the committed preview-versus-editing state after the transition.
- Cancel callbacks should not leak transient buffer text unless the wrapper explicitly documents it.

## 12. Failure and Degradation Rules

| Condition                                                               | Policy             | Notes                                       |
| ----------------------------------------------------------------------- | ------------------ | ------------------------------------------- |
| controlled and uncontrolled value props are mixed after mount           | warn and ignore    | first mode wins                             |
| trigger path is configured but the corresponding trigger part is absent | degrade gracefully | other activation or submit paths still work |
| IME composition events are unavailable                                  | degrade gracefully | editing still works                         |

## 13. Identity and Key Policy

`Root`, `Preview`, `Input`, and any rendered triggers belong to one editable instance. The input node should stay stable during one editing session even if preview or trigger content changes.

## 14. SSR and Client Boundary Rules

- SSR renders either the initial preview or editing structure implied by the initial state.
- Focus restoration and IME handling are client-only.
- Hydration must not flip preview versus editing structure unless the initial machine state differs.

## 15. Performance Constraints

- Keep preview text and edit buffer machine-driven rather than mirrored into parallel local state.
- Avoid unnecessary remounting of the input during one editing session.
- Trigger visibility should derive from machine state, not ad hoc UI flags.

## 16. Implementation Dependencies

| Dependency         | Required?   | Dependency type       | Why it must exist first                                                 |
| ------------------ | ----------- | --------------------- | ----------------------------------------------------------------------- |
| text-entry helpers | required    | shared adapter helper | input editing and IME behavior must stay aligned with other text inputs |
| trigger helpers    | recommended | button semantics      | explicit triggers should reuse consistent press behavior                |

## 17. Recommended Implementation Sequence

1. Initialize the editable machine and derive attrs for preview, input, and triggers.
2. Render `Root`, optional `Label`, `Preview`, `Input`, and any triggers in stable order.
3. Wire controlled prop synchronization and editing input events.
4. Compose activation, confirm, and cancel paths.
5. Finish focus-restoration and diagnostics behavior.

## 18. Anti-Patterns

- Do not commit the edit buffer directly from raw DOM blur without the machine deciding.
- Do not keep preview text and edit buffer in separate unsynchronized adapter state.
- Do not remount the input on every keystroke or visibility change.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the preview and editing states come from one machine-owned source of truth.
- Consumers may assume explicit triggers, when rendered, are adapter-owned and semantically stable.
- Consumers must not assume undocumented hidden inputs or extra wrapper nodes.

## 20. Platform Support Matrix

| Capability / behavior                                                         | Browser client | SSR          | Notes                                       |
| ----------------------------------------------------------------------------- | -------------- | ------------ | ------------------------------------------- |
| documented preview-editing behavior and native input submission while editing | full support   | full support | focus restoration activates after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                                     | Debug build behavior | Production behavior | Notes                            |
| ------------------------------------------------------------- | -------------------- | ------------------- | -------------------------------- |
| controlled/uncontrolled mode switch after mount               | debug warning        | warn and ignore     | preserves current mode           |
| configured trigger path missing a required machine transition | debug warning        | degrade gracefully  | fallback activation path remains |

## 22. Shared Adapter Helper Notes

| Helper concept       | Required?   | Responsibility                                                  | Notes                              |
| -------------------- | ----------- | --------------------------------------------------------------- | ---------------------------------- |
| composition helper   | required    | suppress eager commit or cancel behavior during IME composition | shared with text-entry controls    |
| focus-restore helper | recommended | return focus to preview or trigger after confirm or cancel      | shared with interactive composites |

## 23. Framework-Specific Behavior

Leptos should keep the edit input in a stable `NodeRef`, use local focus restoration after machine transitions, and reuse the same composition-handling path as `TextField`.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<editable::Machine>(props);

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <span {..machine.derive(|api| api.preview_attrs()).get()} />
        <input {..machine.derive(|api| api.input_attrs()).get()} />
    </div>
}
```

## 25. Reference Implementation Skeleton

- Initialize the machine and keep preview-state and edit-buffer ownership entirely inside it.
- Render both preview and input parts according to the documented stateful visibility policy.
- Route activation, confirm, cancel, and composition-aware text entry through machine transitions only.

## 26. Adapter Invariants

- Preview-versus-editing state always lives in the machine.
- The edit buffer is never committed or discarded outside machine transitions.
- The native input remains the only form-submitting node while editing.
- IME composition suppresses eager confirm paths.

## 27. Accessibility and SSR Notes

- Preview content must remain keyboard reachable when activation mode allows keyboard editing.
- Trigger labels must stay localized and reflect their action.
- SSR must preserve the same initial preview-versus-editing structure used by the client.

## 28. Parity Summary and Intentional Deviations

- Matches the core editable contract without intentional divergence.
- Promotes edit-buffer ownership, trigger semantics, and focus-restoration guidance into Leptos-facing rules.

## 29. Test Scenarios

- Preview activation enters editing without losing the committed value.
- Confirm commits the edit buffer and returns to preview with the updated text.
- Cancel discards the edit buffer and restores preview text.
- IME composition suppresses eager confirm behavior until composition ends.

## 30. Test Oracle Notes

- Assert preview-versus-editing transitions from machine-driven state logs.
- Inspect the DOM to confirm only the editing input owns form attrs while editing.
- Verify focus restoration after confirm or cancel with explicit focus assertions.

## 31. Implementation Checklist

- [ ] Keep preview state and edit buffer machine-owned.
- [ ] Render preview, input, and triggers in the documented structural positions.
- [ ] Route activation, confirm, and cancel through machine transitions only.
- [ ] Preserve stable input identity within one editing session.
- [ ] Suppress eager confirm paths during IME composition.
- [ ] Keep focus restoration adapter-owned and explicit.
