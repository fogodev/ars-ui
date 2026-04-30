---
adapter: leptos
component: combobox
category: selection
source: components/selection/combobox.md
source_foundation: foundation/08-adapter-leptos.md
---

# Combobox — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Combobox`](../../components/selection/combobox.md) contract onto Leptos 0.8.x. The adapter must preserve editable text input paired with adapter-owned popup listbox selection while making input ownership, popup and positioner composition, filtered selection, live announcements, and custom-value policy explicit at the framework boundary.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn Combobox(
    #[prop(optional)] id: Option<String>,
    #[prop(optional, into)] input_value: Option<Signal<String>>,
    #[prop(optional, into)] value: Option<Signal<selection::Set>>,
    #[prop(optional)] default_input_value: String,
    #[prop(optional)] default_value: selection::Set,
    #[prop(optional)] filter_mode: Option<combobox::FilterMode>,
    #[prop(optional)] open_on_focus: bool,
    #[prop(optional)] open_on_click: bool,
    #[prop(optional)] allow_custom_value: bool,
    #[prop(optional)] positioning: Option<PositioningOptions>,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
    #[prop(optional)] on_selection_change: Option<Callback<selection::Set>>,
    children: Children,
) -> impl IntoView
```

Compound helpers typically include `Trigger`, `Positioner`, `Content`, `Item`, `EmptyState`, and optional `Description` or `LiveRegion` parts.

## 3. Mapping to Core Component Contract

- Props parity: full parity with core input, selection, filtering, and popup behavior including `allow_custom_value`, highlight defaults, and open-state observation.
- Part parity: full parity for input, trigger, popup, options, and description content; the adapter also owns the live-region strategy required by the core accessibility notes.
- Traceability note: this spec promotes IME handling, count announcements, popup positioning, `aria-activedescendant` fallback, and option registration from the agnostic contract.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target                   | Ownership     | Attr source             | Notes                                                              |
| --------------------- | --------- | ------------------------------------------ | ------------- | ----------------------- | ------------------------------------------------------------------ |
| Root                  | required  | wrapper element                            | adapter-owned | api.root_attrs()        | Owns scope attrs and compound context.                             |
| Input                 | required  | native text input                          | adapter-owned | api.input_attrs()       | Retains focus while the popup is open.                             |
| Trigger               | optional  | native button                              | adapter-owned | api.trigger_attrs()     | Opens or closes the popup without stealing ownership of the input. |
| Positioner            | required  | adapter-owned positioned wrapper           | adapter-owned | api.positioner_attrs()  | Receives placement output and sizing vars.                         |
| Content               | required  | listbox host                               | adapter-owned | api.content_attrs()     | Renders the option list and empty state.                           |
| Item                  | repeated  | option host                                | adapter-owned | api.item_attrs(key)     | One per filtered item.                                             |
| LiveRegion            | optional  | status node                                | adapter-owned | adapter-local attrs     | Announces result counts and active option text when documented.    |
| Description           | optional  | adapter-owned or composed descriptive node | shared        | api.description_attrs() | Participates in described-by wiring.                               |

## 5. Attr Merge and Ownership Rules

- Core attrs win for combobox semantics, `aria-controls`, `aria-expanded`, `aria-activedescendant`, and option selection state.
- The adapter owns `aria-describedby` composition, popup placement variables, and any extra hidden structural nodes required for empty state or live announcements.
- Consumers may add visual classes and wrappers through documented parts, but they must not replace the input host or popup ownership boundaries.

## 6. Composition / Context Contract

The root publishes required combobox context to trigger, content, and item parts. The adapter consumes environment, positioning, field, and optional live-region helpers. Missing root context is a fail-fast structural error for every child part.

## 7. Prop Sync and Event Mapping

| Adapter prop / event | Mode                                    | Sync trigger                         | Machine event / update path                     | Notes                                                             |
| -------------------- | --------------------------------------- | ------------------------------------ | ----------------------------------------------- | ----------------------------------------------------------------- |
| `input_value`        | controlled                              | signal change after mount            | `SetInputValue` or equivalent sync path         | Updates the input text and filtered option set.                   |
| `value`              | controlled                              | signal change after mount            | selection sync event                            | Updates selected item state without losing input focus ownership. |
| open state           | machine-owned with callback observation | focus, click, keyboard, or prop sync | `Open` / `Close`                                | Callback fires after transition completion.                       |
| item activation      | adapter event                           | Enter, click, or pointer selection   | `SelectItem`                                    | When `allow_custom_value=false`, only matching items commit.      |
| IME composition      | adapter event                           | compositionstart / compositionend    | suppresses filtering commits and Enter handling | Intermediate composition text must not commit a value.            |

## 8. Registration and Cleanup Contract

- The adapter owns popup descendant registration, positioner subscriptions, and any result-announcement timer.
- Item registration must be removed as items unmount or as filtering swaps the visible option set.
- No hidden input bridge is created unless the surrounding field contract explicitly requires one for form participation.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability    | Composition rule                                             | Notes                                                     |
| ------------------ | ------------- | ------------- | -------------------- | ------------------------------------------------------------ | --------------------------------------------------------- |
| Input              | yes           | adapter-owned | required after mount | compose with forwarded refs only through a documented helper | Focus remains on the input in the active-descendant path. |
| Positioner         | yes           | adapter-owned | client-only          | no consumer composition                                      | Receives positioning updates.                             |
| Content            | yes           | adapter-owned | required after mount | no composition                                               | Owns the listbox DOM and virtualization hooks.            |

## 10. State Machine Boundary Rules

- Machine-owned state: input text, open state, highlighted key, and selected set.
- Adapter-local derived bookkeeping: item registration, popup positioning handles, live-region throttling, and iOS VoiceOver focus strategy fallback.
- Forbidden local mirrors: do not keep an unsynchronized second open flag, highlighted key, or input buffer.
- Allowed snapshot reads: input handlers, popup open or close handlers, positioning callbacks, and announcement effects.

## 11. Callback Payload Contract

| Callback              | Payload source           | Payload shape    | Timing                           | Cancelable? | Notes                                                                     |
| --------------------- | ------------------------ | ---------------- | -------------------------------- | ----------- | ------------------------------------------------------------------------- |
| `on_open_change`      | machine-derived snapshot | `bool`           | after open-state transitions     | no          | Observes the committed popup state only.                                  |
| `on_selection_change` | machine-derived snapshot | `selection::Set` | after committed option selection | no          | Custom-value acceptance still routes through the committed machine state. |

## 12. Failure and Degradation Rules

| Condition                                        | Policy             | Notes                                                                          |
| ------------------------------------------------ | ------------------ | ------------------------------------------------------------------------------ |
| missing item registry or popup content when open | fail fast          | The component cannot satisfy combobox semantics without a live option surface. |
| positioning engine unavailable                   | degrade gracefully | Render the popup inline with documented fallback semantics.                    |
| controlled/uncontrolled mode switch after mount  | warn and ignore    | Keep the first resolved mode for input and selection separately.               |

## 13. Identity and Key Policy

- Item identity is data-derived by `Key` and must not be rewritten by filtering or virtualization.
- Input, content, and positioner nodes are instance-derived and must remain hydration-stable.
- Announcement and positioning resources are instance-derived and must be released on close or unmount.

## 14. SSR and Client Boundary Rules

- SSR renders the root, input, and any open popup structure that is documented as hydration-stable.
- Positioning, live announcements, and DOM focus repair are client-only.
- If the popup is SSR-rendered open, the same structure and item order must hydrate on the client.

## 15. Performance Constraints

- Filtering should update the visible option set incrementally rather than rebuilding registration from scratch when unnecessary.
- Positioning work must only run while the popup is open.
- Announcement timers should coalesce repeated keystrokes into one documented emission path.

## 16. Implementation Dependencies

| Dependency               | Required? | Dependency type | Why it must exist first                                                    | Notes                                                      |
| ------------------------ | --------- | --------------- | -------------------------------------------------------------------------- | ---------------------------------------------------------- |
| positioning helper       | required  | shared helper   | Popup placement and sizing are adapter-owned responsibilities.             | Shared with popup-based selection and overlay components.  |
| item registration helper | required  | shared helper   | Keyed option registration drives highlight, selection, and virtualization. | Shared with `select`, `listbox`, and menu-like components. |
| IME helper               | required  | shared helper   | Text-entry composition rules must stay aligned across the category.        | Shared with `autocomplete` and `tags-input`.               |

## 17. Recommended Implementation Sequence

1. Initialize machine props and publish compound context.
2. Render input, trigger, popup positioner, content, and keyed option parts in stable order.
3. Wire controlled input and selection sync before attaching positioning and announcement helpers.
4. Add iOS VoiceOver fallback, live-region behavior, and cleanup ordering checks.

## 18. Anti-Patterns

- Do not move DOM focus into the popup in the default `aria-activedescendant` path.
- Do not treat free-form input as committed selection when `allow_custom_value=false`.
- Do not leave stale option registrations behind after filtering or virtualization changes.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the combobox input remains the primary focus target while the popup is open.
- Consumers may assume popup open-state callbacks observe committed machine transitions only.
- Consumers must not assume positioning or announcement behavior is inherited implicitly from another component; it is restated here.

## 20. Platform Support Matrix

| Capability / behavior                   | Browser client | SSR            | Notes                                                                             |
| --------------------------------------- | -------------- | -------------- | --------------------------------------------------------------------------------- |
| input, popup, and keyed option behavior | full support   | full support   | Structural output is SSR-safe; interactive popup behavior begins after hydration. |
| positioning and focus repair            | client-only    | SSR-safe empty | Requires mounted DOM nodes.                                                       |
| live result announcements               | client-only    | SSR-safe empty | Announcements are intentionally post-hydration only.                              |

## 21. Debug Diagnostics and Production Policy

| Condition                                 | Debug build behavior | Production behavior | Notes                                             |
| ----------------------------------------- | -------------------- | ------------------- | ------------------------------------------------- |
| popup content missing while open          | fail fast            | fail fast           | Combobox semantics require a live option surface. |
| stale option registration after filtering | debug warning        | warn and ignore     | Cleanup must converge on the next render.         |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required?   | Responsibility                                       | Reused by                        | Notes                                    |
| ------------------------ | ----------- | ---------------------------------------------------- | -------------------------------- | ---------------------------------------- |
| positioning helper       | required    | Apply placement and sizing output to the positioner. | `select`, `menu`, `context-menu` | Reuse the shared popup positioning path. |
| item registration helper | required    | Track keyed options for highlight and selection.     | `select`, `listbox`, `menu`      | Must stay cleanup-safe under filtering.  |
| announcement helper      | recommended | Throttle count and active-option announcements.      | `autocomplete`, `listbox`        | Use one shared live-region policy.       |

## 23. Framework-Specific Behavior

Leptos should keep popup and item registration in reactive context, drive DOM focus repair through `NodeRef` or effect-local handles, and avoid ad hoc shadow signals for highlighted keys.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Combobox(/* props */ children: Children) -> impl IntoView {
    let machine = use_machine::<combobox::Machine>(combobox::Props { /* ... */ });
    provide_context(Context::from_machine(machine));

    view! {
        <div {..machine.derive(|api| api.root_attrs()).get()}>
            <input {..machine.derive(|api| api.input_attrs()).get()} />
            <div {..machine.derive(|api| api.positioner_attrs()).get()}>
                <div {..machine.derive(|api| api.content_attrs()).get()}>{children()}</div>
            </div>
        </div>
    }
}
```

## 25. Reference Implementation Skeleton

Keep one machine, one keyed option-registration helper, one positioning subscription, and one optional announcement helper. Input, highlight, and selection always flow through machine events before the adapter reads the committed snapshot.

## 26. Adapter Invariants

- The input remains adapter-owned and focus-stable while the popup is open.
- Popup content, positioner ownership, and item registration are explicit adapter responsibilities.
- Announcement and custom-value policies are driven by committed machine state rather than transient DOM reads.

## 27. Accessibility and SSR Notes

- When `aria-activedescendant` is used, it must always reference a live option id or be omitted entirely.
- If iOS VoiceOver fallback is required, the adapter must document when it shifts from active-descendant semantics to direct DOM focus on options.
- Description and error-message linkage must include only nodes that are actually rendered.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter ownership of popup composition, positioning, and live announcements.
- Intentional deviations: non-web positioning or focus repair may use documented fallback paths rather than browser-specific APIs.

## 29. Test Scenarios

1. Typing filters options while keeping input focus stable.
2. Opening the popup on focus or click respects the documented prop policy.
3. IME composition suppresses premature selection and blur commits.
4. Result announcements and active-option semantics do not double-announce.

## 30. Test Oracle Notes

- Preferred oracle for popup semantics: `DOM attrs` on input, content, and option nodes plus `machine state` for committed open and selection changes.
- Preferred oracle for positioning and cleanup: `cleanup side effects` showing subscriptions attach only while open.
- Preferred oracle for announcement behavior: `callback order` and `rendered structure` of the live-region node.

## 31. Implementation Checklist

- [ ] Popup ownership, item registration, and positioning are explicit adapter contracts.
- [ ] IME handling, custom-value policy, and callback timing are machine-driven.
- [ ] Announcement and iOS VoiceOver fallback guidance is captured in invariants, tests, and checklist coverage.
