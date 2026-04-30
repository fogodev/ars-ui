---
adapter: leptos
component: textarea
category: input
source: components/input/textarea.md
source_foundation: foundation/08-adapter-leptos.md
---

# Textarea — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Textarea`](../../components/input/textarea.md) contract onto a Leptos 0.8.x component. The adapter must preserve native textarea submission, optional auto-resize behavior, character-count publication, and IME-aware value synchronization.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn Textarea(
    #[prop(optional, into)] value: Option<Signal<String>>,
    #[prop(optional)] default_value: String,
    #[prop(optional)] rows: Option<u32>,
    #[prop(optional)] cols: Option<u32>,
    #[prop(optional)] auto_resize: bool,
    #[prop(optional)] show_character_count: bool,
    #[prop(optional)] input_mode: Option<InputMode>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] readonly: Signal<bool>,
    #[prop(optional, into)] required: Signal<bool>,
    #[prop(optional, into)] invalid: Signal<bool>,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] form: Option<String>,
    children: Children,
) -> impl IntoView
```

The adapter also forwards the shared field props from the core contract, including locale or messages and length constraints. `value`, `disabled`, `readonly`, `required`, and `invalid` are the default post-mount reactive inputs.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core textarea contract, including auto-resize and character-count options.
- Event parity: `Input`, `Focus`, `Blur`, `CompositionStart`, `CompositionEnd`, and controlled `Set*` events remain machine-owned.
- Core machine ownership: `use_machine::<textarea::Machine>(...)` owns value, length-derived state, and any auto-resize signals promoted from the core spec.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source                   | Notes                   |
| --------------------- | --------- | ------------------------ | ------------- | ----------------------------- | ----------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`            | structural wrapper      |
| `Label`               | required  | `<label>`                | adapter-owned | `api.label_attrs()`           | linked to `Textarea`    |
| `Textarea`            | required  | native `<textarea>`      | adapter-owned | `api.textarea_attrs()`        | direct form participant |
| `CharacterCount`      | optional  | `<div>`                  | adapter-owned | `api.character_count_attrs()` | optional live status    |
| `Description`         | optional  | `<div>`                  | adapter-owned | `api.description_attrs()`     | described-by content    |
| `ErrorMessage`        | optional  | `<div>`                  | adapter-owned | `api.error_message_attrs()`   | invalid-only content    |

## 5. Attr Merge and Ownership Rules

- Native `rows`, `cols`, `name`, `form`, length constraints, `required`, `readonly`, and ARIA attrs on `Textarea` always win.
- `class` and `style` merge additively on `Root` and `Textarea`.
- Consumer handlers may decorate the textarea, but auto-resize and value synchronization remain adapter-owned.

## 6. Composition / Context Contract

`Textarea` consumes field, fieldset, and optional form context. Character-count content is adapter-owned even when consumers provide surrounding status text.

## 7. Prop Sync and Event Mapping

| Adapter prop            | Mode                       | Sync trigger    | Machine event / update path   | Visible effect                          |
| ----------------------- | -------------------------- | --------------- | ----------------------------- | --------------------------------------- |
| `value`                 | controlled                 | signal change   | `SetValue`                    | updates native textarea value and count |
| `disabled` / `readonly` | controlled                 | signal change   | `SetDisabled` / `SetReadonly` | updates attrs and guards                |
| `required` / `invalid`  | controlled                 | signal change   | `SetRequired` / `SetInvalid`  | updates ARIA and validation wiring      |
| native `input`          | uncontrolled or controlled | DOM input event | `InputChange`                 | updates machine value                   |

When `auto_resize=true`, post-input measurement must happen after the machine accepts the new value. IME composition suppresses eager callback side effects in the same way as `TextField`.

## 8. Registration and Cleanup Contract

- No descendant registry is required.
- Auto-resize, when enabled, owns only local measurement work for this textarea node.
- No global listeners should survive unmount.

## 9. Ref and Node Contract

- `Textarea` owns the live node ref.
- Auto-resize uses that ref for measurement; it must not create a second authoritative value source.
- Character-count rendering never replaces `Textarea` as the focus target.

## 10. State Machine Boundary Rules

- The machine owns the committed text value and any length-based status.
- The adapter may measure height after accepted updates, but it must not compute alternate text state outside the machine.
- Native textarea submission remains direct; there is no hidden-input bridge.

## 11. Callback Payload Contract

- Value-change callbacks emit the committed string value.
- Character-count publication reflects the committed value, not transient pre-machine DOM edits.
- Focus callbacks may expose `is_keyboard` for focus-visible behavior.

## 12. Failure and Degradation Rules

| Condition                                                     | Policy             | Notes                                     |
| ------------------------------------------------------------- | ------------------ | ----------------------------------------- |
| controlled and uncontrolled value props are mixed after mount | warn and ignore    | first mode wins                           |
| auto-resize measurement is unavailable                        | degrade gracefully | textarea remains manually sizable per CSS |
| IME composition events are unavailable                        | degrade gracefully | raw input still works                     |

## 13. Identity and Key Policy

`Root`, `Textarea`, and optional `CharacterCount` belong to one textarea instance. Hydration must preserve the native textarea node.

## 14. SSR and Client Boundary Rules

- SSR renders the same part structure and initial textarea attrs as the client.
- Auto-resize and IME handling are client-only behaviors layered over stable markup.
- The native textarea remains the direct form participant in both SSR and hydrated modes.

## 15. Performance Constraints

- Auto-resize should measure only when value, rows, or width materially changes.
- Do not mirror text value into extra local signals outside the machine.
- Character-count rendering should derive from machine state, not from DOM reads.

## 16. Implementation Dependencies

| Dependency         | Required?   | Dependency type      | Why it must exist first                                |
| ------------------ | ----------- | -------------------- | ------------------------------------------------------ |
| `field`            | required    | composition contract | label, description, and error wiring must stay uniform |
| measurement helper | recommended | adapter helper       | auto-resize should reuse stable measurement logic      |

## 17. Recommended Implementation Sequence

1. Initialize the machine with merged props and contexts.
2. Render `Root`, `Label`, native `Textarea`, and optional status parts in stable order.
3. Wire controlled prop synchronization and native input or composition events.
4. Add optional auto-resize measurement and optional character-count rendering.
5. Finish diagnostics and SSR checks.

## 18. Anti-Patterns

- Do not replace the native textarea with a contenteditable element.
- Do not compute character count from stale DOM reads when machine state already owns the value.
- Do not run auto-resize logic during SSR.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the native textarea remains the form-submitting node.
- Consumers may assume `CharacterCount` tracks machine-committed value.
- Consumers must not assume undocumented wrapper nodes around the textarea.

## 20. Platform Support Matrix

| Capability / behavior                                             | Browser client | SSR          | Notes                                       |
| ----------------------------------------------------------------- | -------------- | ------------ | ------------------------------------------- |
| documented textarea semantics, auto-resize, and native submission | full support   | full support | measurement behavior starts after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                              |
| ----------------------------------------------- | -------------------- | ------------------- | ---------------------------------- |
| controlled/uncontrolled mode switch after mount | debug warning        | warn and ignore     | preserves current mode             |
| invalid auto-resize configuration               | debug warning        | degrade gracefully  | textarea still renders and submits |

## 22. Shared Adapter Helper Notes

| Helper concept     | Required?   | Responsibility                                                 | Notes                                |
| ------------------ | ----------- | -------------------------------------------------------------- | ------------------------------------ |
| field merge helper | required    | merge IDs, described-by, disabled, readonly, and invalid state | shared by text-entry controls        |
| measurement helper | recommended | recompute height for auto-resize after committed updates       | shared with other measuring controls |

## 23. Framework-Specific Behavior

Leptos should use a `NodeRef<html::Textarea>` for auto-resize measurement, keep controlled prop watchers narrow, and gate all measurement work behind a client-only path.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<textarea::Machine>(props);
let textarea_ref = NodeRef::<html::Textarea>::new();

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <label {..machine.derive(|api| api.label_attrs()).get()}>{children()}</label>
        <textarea
            node_ref=textarea_ref
            {..machine.derive(|api| api.textarea_attrs()).get()}
            on:input=move |ev| send_input(ev, &machine)
        />
    </div>
}
```

## 25. Reference Implementation Skeleton

- Merge field context and explicit props before machine initialization.
- Initialize the machine and register only the controlled watchers that exist.
- Render the native textarea first, then layer optional character-count and client-only auto-resize behavior on top.

## 26. Adapter Invariants

- `Textarea` always remains native and directly form-participating.
- Auto-resize never becomes the source of truth for value.
- Character count follows committed machine value.
- IME composition suppresses eager side effects.

## 27. Accessibility and SSR Notes

- Description-first, error-second ordering is mandatory for `aria-describedby`.
- Character-count live behavior must not double-announce alongside the error message.
- SSR must preserve the same native textarea attrs used by the client.

## 28. Parity Summary and Intentional Deviations

- Matches the core textarea contract without intentional divergence.
- Promotes auto-resize ownership, character-count publication, and native submission rules into Leptos-facing guidance.

## 29. Test Scenarios

- Controlled value updates synchronize with the native textarea and character count.
- Auto-resize updates height after committed input changes without affecting SSR markup.
- Invalid state updates described-by ordering and native constraint attrs correctly.
- IME composition suppresses eager callbacks and resumes normal updates on composition end.

## 30. Test Oracle Notes

- Inspect the native textarea DOM attrs and described-by ordering.
- Compare client height changes only after hydration when auto-resize is enabled.
- Assert character-count output from machine state rather than from raw DOM length reads.

## 31. Implementation Checklist

- [ ] Keep `Textarea` native and directly form-participating.
- [ ] Render optional `CharacterCount` only in the documented structural position.
- [ ] Route controlled value and native input changes through machine events.
- [ ] Gate auto-resize measurement behind client-only logic.
- [ ] Preserve description and error-message ordering on `aria-describedby`.
- [ ] Keep character-count publication machine-driven.
