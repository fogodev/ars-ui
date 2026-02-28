---
adapter: leptos
component: clipboard
category: specialized
source: components/specialized/clipboard.md
source_foundation: foundation/08-adapter-leptos.md
---

# Clipboard — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Clipboard`](../../components/specialized/clipboard.md) contract onto Leptos `0.8.x`. The adapter preserves copy-state transitions, success and error feedback, status announcements, and secure-context fallback policy for the browser clipboard APIs.

## 2. Public Adapter API

```rust
#[component]
pub fn Clipboard(
    #[prop(optional)] value: Option<RwSignal<String>>,
    #[prop(optional, into)] default_value: String,
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional)] feedback_duration_ms: u32,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] show_value_text: bool,
) -> impl IntoView
```

The adapter renders the optional `Label` and `ValueText` parts from props rather than separate subcomponents. `Trigger`, `Indicator`, and `Status` are always adapter-owned.

## 3. Mapping to Core Component Contract

- Props parity: full parity with bindable value, feedback duration, disabled state, and accessible labeling.
- Part parity: full parity with `Root`, `Label`, `Trigger`, `Indicator`, `Status`, and `ValueText`.
- Adapter additions: explicit browser permission, fallback, timeout, and live-region wiring rules.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source              | Notes                               |
| --------------------- | --------- | ------------------------ | ------------- | ------------------------ | ----------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`       | Carries state and disabled markers. |
| `Label`               | optional  | `<label>`                | adapter-owned | `api.label_attrs()`      | Render when `label` is present.     |
| `Trigger`             | required  | `<button>`               | adapter-owned | `api.trigger_attrs()`    | Owns the copy action.               |
| `Indicator`           | optional  | `<span>`                 | adapter-owned | `api.indicator_attrs()`  | Decorative icon tied to state.      |
| `Status`              | required  | `<div>`                  | adapter-owned | `api.status_attrs()`     | `role="status"` live region.        |
| `ValueText`           | optional  | `<span>`                 | adapter-owned | `api.value_text_attrs()` | Render when `show_value_text=true`. |

## 5. Attr Merge and Ownership Rules

| Target node           | Core attrs                                  | Adapter-owned attrs          | Consumer attrs        | Merge order                                 | Ownership notes                 |
| --------------------- | ------------------------------------------- | ---------------------------- | --------------------- | ------------------------------------------- | ------------------------------- |
| `Trigger`             | labels, disabled markers, scope, part       | click handler and busy state | decoration attrs only | accessible label and disabled semantics win | trigger stays adapter-owned     |
| `Status`              | `role="status"`, `aria-live`, `aria-atomic` | feedback text content        | none                  | live-region attrs win                       | status is not consumer-owned    |
| `Label` / `ValueText` | textual attrs                               | none                         | decoration only       | core linkage wins                           | text source remains prop-driven |

## 6. Composition / Context Contract

`Clipboard` is context-free. The live region is internal and must not be supplied by consumer context.

## 7. Prop Sync and Event Mapping

| Adapter prop                       | Mode         | Sync trigger            | Machine event / update path | Visible effect                              | Notes                                              |
| ---------------------------------- | ------------ | ----------------------- | --------------------------- | ------------------------------------------- | -------------------------------------------------- |
| `value`                            | controlled   | upstream signal changes | bindable sync               | changes copied text and `ValueText`         | writable only when a controlled signal is provided |
| `default_value`                    | uncontrolled | init only               | initial context             | initial copied text                         | ignored after mount in controlled mode             |
| `disabled`, `feedback_duration_ms` | controlled   | rerender                | prop rebuild                | changes trigger enablement and reset timing | timer behavior follows latest props                |

| UI event                             | Preconditions          | Machine event / callback path | Ordering notes                                               | Notes                                        |
| ------------------------------------ | ---------------------- | ----------------------------- | ------------------------------------------------------------ | -------------------------------------------- |
| trigger click or keyboard activation | not disabled           | `Copy`                        | browser clipboard write must start directly from the gesture | secure-context requirement applies           |
| successful write                     | copy request in flight | `CopySuccess`                 | announce success before scheduling reset timer               | status text updates immediately              |
| failed write                         | copy request in flight | `CopyError(reason)`           | announce failure before scheduling reset timer               | reason stays adapter-visible for diagnostics |

## 8. Registration and Cleanup Contract

| Registered entity    | Registration trigger              | Identity key       | Cleanup trigger                          | Cleanup action | Notes                    |
| -------------------- | --------------------------------- | ------------------ | ---------------------------------------- | -------------- | ------------------------ |
| feedback reset timer | `Copied` or `Error` state entered | component instance | timer fire, new copy attempt, or cleanup | cancel timer   | at most one active timer |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability | Composition rule | Notes                                               |
| ------------------ | ------------- | ------------- | ----------------- | ---------------- | --------------------------------------------------- |
| `Trigger`          | no            | adapter-owned | always structural | no composition   | copy gesture depends on the handler, not a node ref |
| `Status`           | no            | adapter-owned | always structural | no composition   | live-region node stays mounted                      |

## 10. State Machine Boundary Rules

- machine-owned state: `Idle`, `Copying`, `Copied`, and `Error`.
- adapter-local derived bookkeeping: timer handle and runtime clipboard capability probe only.
- forbidden local mirrors: do not store separate visual success or error state outside the machine.

## 11. Callback Payload Contract

No dedicated public callback is required. Consumers observe state through the rendered indicator and status text.

## 12. Failure and Degradation Rules

| Condition                                   | Policy             | Notes                                                      |
| ------------------------------------------- | ------------------ | ---------------------------------------------------------- |
| `navigator.clipboard.writeText` unavailable | degrade gracefully | attempt the documented legacy copy fallback when supported |
| secure-context or permission failure        | degrade gracefully | move to `Error` with structured failure reason             |
| copy operation exceeds timeout window       | degrade gracefully | resolve to `Error(Timeout)` and clear the pending state    |

## 13. Identity and Key Policy

The component owns no repeated descendants. Timer identity is the component instance.

## 14. SSR and Client Boundary Rules

- SSR renders `Root`, `Trigger`, optional text parts, and `Status` in the idle state.
- Clipboard writes and feedback timers are client-only.
- Hydration must not start a copy attempt or timer automatically.

## 15. Performance Constraints

- Keep only one reset timer active.
- Do not probe clipboard capability on every render; cache it per mounted instance if needed.

## 16. Implementation Dependencies

| Dependency             | Required?   | Dependency type      | Why it must exist first                                                 | Notes                                   |
| ---------------------- | ----------- | -------------------- | ----------------------------------------------------------------------- | --------------------------------------- |
| clipboard-write helper | required    | browser helper       | encapsulates secure-context, permission, timeout, and fallback behavior | adapter-owned capability                |
| live-region helper     | recommended | accessibility helper | keeps announcement wording and timing consistent                        | shared with other announcing components |

## 17. Recommended Implementation Sequence

1. Initialize the machine and idle status region.
2. Wire the trigger gesture to the clipboard helper.
3. Add success or error announcements and reset timing.
4. Verify SSR idle behavior and fallback handling.

## 18. Anti-Patterns

- Do not attempt clipboard writes outside a user gesture.
- Do not remove the status live region when the visible indicator is present.
- Do not leave reset timers running across unmount.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the component announces copy success and failure.
- Consumers may assume the visible value text is optional.
- Consumers must not assume clipboard support exists on every runtime.

## 20. Platform Support Matrix

| Capability / behavior                | Browser client | SSR            | Notes                                        |
| ------------------------------------ | -------------- | -------------- | -------------------------------------------- |
| copy state machine and announcements | full support   | SSR-safe empty | SSR renders idle structure only.             |
| async clipboard API                  | full support   | client-only    | requires secure context and user activation. |
| legacy copy fallback                 | fallback path  | not applicable | used only when the async API is unavailable. |

## 21. Debug Diagnostics and Production Policy

| Condition                    | Debug build behavior | Production behavior | Notes                                          |
| ---------------------------- | -------------------- | ------------------- | ---------------------------------------------- |
| clipboard capability missing | debug warning        | degrade gracefully  | keep trigger rendered but surface error on use |
| timer cleanup missed         | fail fast            | fail fast           | stale timers would corrupt feedback state      |

## 22. Shared Adapter Helper Notes

| Helper concept     | Required?   | Responsibility                                  | Reused by             | Notes                    |
| ------------------ | ----------- | ----------------------------------------------- | --------------------- | ------------------------ |
| clipboard helper   | required    | performs async or legacy copy and maps failures | `clipboard` only      | gesture-bound            |
| timer helper       | required    | owns the feedback reset timer                   | timer-backed widgets  | one timer per instance   |
| live-region helper | recommended | normalizes success and failure announcements    | announcing components | keep wording centralized |

## 23. Framework-Specific Behavior

Leptos should keep copy initiation inside the trigger event closure and use effect cleanup only for the feedback timer, not for the clipboard write itself.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Clipboard(value: Option<RwSignal<String>>) -> impl IntoView {
    let machine = use_machine::<clipboard::Machine>(clipboard::Props::default());
    let trigger_attrs = machine.derive(|api| api.trigger_attrs());
    let status_attrs = machine.derive(|api| api.status_attrs());
    view! {
        <div>
            <button {..trigger_attrs.get()} />
            <div {..status_attrs.get()} />
        </div>
    }
}
```

## 25. Reference Implementation Skeleton

- Bind the value through `Bindable<String>`.
- Render the permanent status node.
- Call the clipboard helper from the trigger handler.
- Schedule and cancel the feedback timer from machine transitions.

## 26. Adapter Invariants

- Copy starts only from a user gesture.
- The status live region remains mounted for the component lifetime.
- Reset timers never survive unmount or a new copy attempt.

## 27. Accessibility and SSR Notes

The status node is the authoritative announcement surface. Decorative indicators must stay `aria-hidden`.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- async clipboard success
- permission or secure-context failure
- legacy fallback path
- feedback timer reset after success and error
- SSR and hydration remain idle until user action

## 30. Test Oracle Notes

| Behavior          | Preferred oracle type           | Notes                                        |
| ----------------- | ------------------------------- | -------------------------------------------- |
| status updates    | DOM text plus live-region attrs | assert polite status output                  |
| fallback behavior | mocked browser API              | force async API absence and observe fallback |
| timer cleanup     | cleanup side effects            | assert no stale reset fires after unmount    |

## 31. Implementation Checklist

- [ ] Copy starts from the trigger gesture only.
- [ ] Success and error both update the live region.
- [ ] Timer cleanup is explicit and verified.
