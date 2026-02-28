---
adapter: leptos
component: file-trigger
category: input
source: components/input/file-trigger.md
source_foundation: foundation/08-adapter-leptos.md
---

# FileTrigger — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`FileTrigger`](../../components/input/file-trigger.md) contract onto a Leptos 0.8.x component. The adapter owns the hidden native file input, programmatic picker opening, and translation from `change` events into adapter callbacks.

## 2. Public Adapter API

```rust
#[component]
pub fn FileTrigger(
    #[prop(optional)] accept: Vec<String>,
    #[prop(optional)] multiple: bool,
    #[prop(optional)] directory: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] form: Option<String>,
    #[prop(optional)] on_select: Option<Callback<Vec<web_sys::File>>>,
    children: Children,
) -> impl IntoView
```

The adapter forwards the full core prop surface, including capture mode, locale or messages, and ID overrides. `on_select` is adapter-owned and fires with the current selection snapshot after the native input changes.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the stateless core contract plus Leptos callback wiring.
- Event parity: the core API remains stateless; adapter-owned events are trigger press and input `change`.
- Core API ownership: `Api::new(props)` remains the source of truth for root, trigger, and input attrs.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target     | Ownership     | Attr source           | Notes                                       |
| --------------------- | --------- | ---------------------------- | ------------- | --------------------- | ------------------------------------------- |
| `Root`                | required  | `<div>`                      | adapter-owned | `api.root_attrs()`    | wrapper for trigger and input               |
| `Trigger`             | required  | consumer child               | shared        | `api.trigger_attrs()` | trigger is a slot, not a fixed host element |
| `Input`               | required  | hidden `<input type="file">` | adapter-owned | `api.input_attrs()`   | owns native picker and selected files       |

## 5. Attr Merge and Ownership Rules

- Core `accept`, `multiple`, `capture`, `webkitdirectory`, `name`, `form`, `tabindex`, and `aria-hidden` attrs on `Input` always win.
- Trigger `class` and `style` merge additively with consumer decoration, but disabled semantics remain adapter-owned.
- Consumers must not remove the hidden-input visibility or tab-order constraints.

## 6. Composition / Context Contract

`FileTrigger` is trigger-slot based. It does not consume field context by default, and it does not expose compound child contexts. The trigger child must be exactly one interactive element or wrapper that can accept the trigger attrs.

## 7. Prop Sync and Event Mapping

| Adapter prop                                    | Mode      | Sync trigger   | Update path        | Visible effect                      |
| ----------------------------------------------- | --------- | -------------- | ------------------ | ----------------------------------- |
| `disabled`                                      | immediate | prop change    | attr re-derivation | blocks picker opening               |
| `accept` / `multiple` / `directory` / `capture` | immediate | prop change    | attr re-derivation | changes native picker configuration |
| `on_select`                                     | callback  | input `change` | adapter callback   | emits selected file snapshot        |

Trigger press or click maps to `input_ref.click()` only when not disabled. Input `change` maps to callback emission and should preserve the browser's selected file ordering.

## 8. Registration and Cleanup Contract

- No machine or descendant registry exists.
- The adapter owns a live input ref and any imperative click bridge attached to the trigger.
- No global listeners are required; unmount simply drops refs and callbacks.

## 9. Ref and Node Contract

- `Input` owns the only required live node ref.
- The trigger child may forward a ref for host integration, but that ref never replaces the input ref used for picker opening.
- The hidden input must stay mounted while the trigger is mounted.

## 10. State Machine Boundary Rules

- There is no state machine.
- The adapter must not fabricate intermediate file state; the selected files come directly from the native input.
- Clearing selection, if exposed by wrappers, must happen by resetting the input element, not by mutating callback payloads.

## 11. Callback Payload Contract

- `on_select` emits the full `Vec<File>` snapshot from the native input.
- The callback fires after the browser updates `files`, not on trigger press.
- Re-selecting the same file set should follow native browser behavior; wrappers may explicitly clear the input if they need same-file re-selection.

## 12. Failure and Degradation Rules

| Condition                                                    | Policy             | Notes                                      |
| ------------------------------------------------------------ | ------------------ | ------------------------------------------ |
| trigger slot does not resolve to exactly one child           | fail fast          | required for predictable attr spreading    |
| native file picker is unavailable in the current environment | degrade gracefully | trigger stays disabled or no-ops           |
| SSR execution path attempts to open the picker               | no-op              | only structure should render on the server |

## 13. Identity and Key Policy

`Root`, the slotted trigger node, and the hidden `Input` belong to one file-trigger instance. Replacing the `Input` node resets the browser's selection state.

## 14. SSR and Client Boundary Rules

- SSR renders the wrapper, slotted trigger structure, and hidden input attrs only.
- Picker opening and file enumeration are client-only.
- Hydration must preserve the hidden-input node so the trigger-to-input bridge stays valid.

## 15. Performance Constraints

- Keep the trigger-to-input bridge instance-local.
- Do not clone file objects until callback emission actually occurs.
- Avoid re-rendering the slotted trigger solely to mirror transient selection state that belongs to the native input.

## 16. Implementation Dependencies

| Dependency                     | Required?   | Dependency type      | Why it must exist first                                                     |
| ------------------------------ | ----------- | -------------------- | --------------------------------------------------------------------------- |
| `button` or other trigger host | recommended | composition contract | most consumers will spread trigger attrs into an existing pressable control |

## 17. Recommended Implementation Sequence

1. Build the stateless core API and derive root, trigger, and input attrs.
2. Establish the hidden-input ref and the trigger-to-input click bridge.
3. Render the trigger slot and hidden input in stable order.
4. Wire the input `change` handler to `on_select`.
5. Add failure handling for invalid slot composition and unavailable picker environments.

## 18. Anti-Patterns

- Do not make the hidden input focusable.
- Do not treat the trigger slot as adapter-owned DOM when it is consumer-owned.
- Do not synthesize file objects or hide native selection ordering.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the hidden input remains adapter-owned and visually hidden.
- Consumers may assume the trigger slot only opens the picker when not disabled.
- Consumers must not assume drag-and-drop or upload lifecycle behavior; this component only chooses files.

## 20. Platform Support Matrix

| Capability / behavior                    | Browser client | SSR            | Notes                         |
| ---------------------------------------- | -------------- | -------------- | ----------------------------- |
| native picker opening and file selection | full support   | SSR-safe empty | server only renders structure |

## 21. Debug Diagnostics and Production Policy

| Condition                         | Debug build behavior | Production behavior | Notes                                    |
| --------------------------------- | -------------------- | ------------------- | ---------------------------------------- |
| invalid trigger child count       | fail fast            | fail fast           | slot composition is a hard requirement   |
| picker unavailable in environment | debug warning        | degrade gracefully  | component should no-op rather than crash |

## 22. Shared Adapter Helper Notes

| Helper concept         | Required?   | Responsibility                                        | Notes                             |
| ---------------------- | ----------- | ----------------------------------------------------- | --------------------------------- |
| trigger bridge helper  | required    | connect trigger activation to hidden-input `.click()` | shared with upload-style wrappers |
| file extraction helper | recommended | convert `FileList` into stable callback payloads      | keeps selection ordering intact   |

## 23. Framework-Specific Behavior

Leptos should keep the hidden input in a `NodeRef<html::Input>`, attach the click bridge directly on the trigger host, and gate all picker-opening behavior behind a non-SSR path.

## 24. Canonical Implementation Sketch

```rust
let api = file_trigger::Api::new(&props);
let input_ref = NodeRef::<html::Input>::new();

view! {
    <div {..api.root_attrs()}>
        <button
            {..api.trigger_attrs()}
            on:click=move |_| input_ref.get().map(|el| el.click())
        >
            {children()}
        </button>
        <input
            node_ref=input_ref
            {..api.input_attrs()}
            on:change=move |ev| emit_files(ev, on_select)
        />
    </div>
}
```

## 25. Reference Implementation Skeleton

- Build the stateless API from props only.
- Create one hidden-input ref and one trigger bridge closure.
- Render the slotted trigger and hidden input together, then translate native `change` into callback payloads.

## 26. Adapter Invariants

- The hidden input is always the native file-selection owner.
- Trigger activation never fires `on_select` directly; it only opens the picker.
- The trigger slot must accept the required attrs and disabled semantics.
- SSR never attempts to open the picker.

## 27. Accessibility and SSR Notes

- The hidden input keeps its localized `aria-label` even though it is `aria-hidden`, because that label still documents the element for debugging and fallback assistive tooling.
- The trigger child remains the user-visible interactive affordance and must expose the disabled state accessibly.
- Server rendering must not omit the hidden input.

## 28. Parity Summary and Intentional Deviations

- Matches the core stateless file-trigger contract without intentional divergence.
- Promotes slot validation, ref ownership, picker bridging, and callback timing into Leptos-facing rules.

## 29. Test Scenarios

- Clicking the trigger opens the picker only when enabled.
- Changing the native input emits the selected files in browser order.
- `accept`, `multiple`, `directory`, and `capture` all appear on the hidden input as documented.
- SSR renders the hidden input but performs no picker side effects.

## 30. Test Oracle Notes

- Inspect the hidden input DOM attrs directly.
- Mock the input ref click to verify trigger-to-picker bridging.
- Assert callback timing from the native `change` event rather than from trigger clicks.

## 31. Implementation Checklist

- [ ] Render `Root`, one slotted `Trigger`, and one hidden `Input`.
- [ ] Keep the hidden input as the only file-selection owner.
- [ ] Bridge trigger activation to `input.click()` only on the client.
- [ ] Emit `on_select` only from the native input `change` path.
- [ ] Preserve documented `accept`, `multiple`, `directory`, and `capture` attrs.
- [ ] Fail fast on invalid trigger-slot composition.
