---
adapter: dioxus
component: swap
category: utility
source: components/utility/swap.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Swap — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Swap`](../../components/utility/swap.md) machine to a Dioxus 0.7.x content-toggle utility. The adapter must preserve `Root`, `OnContent`, and `OffContent` as explicit rendered structures.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct SwapProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub checked: Option<Signal<bool>>,
    #[props(default = false)]
    pub default_checked: bool,
    #[props(default = false)]
    pub disabled: bool,
    #[props(optional)]
    pub label: Option<String>,
    pub on_content: Element,
    pub off_content: Element,
}

#[component]
pub fn Swap(props: SwapProps) -> Element
```

The adapter surfaces the full core prop set, including animation, locale/messages, and `on_change`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Swap` props.
- Event parity: toggle, controlled sync, disabled sync, focus, and blur are adapter-driven.
- Structure parity: `OnContent` and `OffContent` are distinct mapped parts, not implied content blobs.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target            | Ownership                  | Attr source               | Notes                                                                    |
| --------------------- | --------- | ----------------------------------- | -------------------------- | ------------------------- | ------------------------------------------------------------------------ |
| `Root`                | required  | `<button>` or `<div role="button">` | adapter-owned              | `api.root_attrs()`        | Interactive container for the swap.                                      |
| `OnContent`           | required  | `<span>` inside `Root`              | adapter-owned slot wrapper | `api.on_content_attrs()`  | May stay mounted or presence-controlled depending on animation strategy. |
| `OffContent`          | required  | `<span>` inside `Root`              | adapter-owned slot wrapper | `api.off_content_attrs()` | May stay mounted or presence-controlled depending on animation strategy. |

## 5. Attr Merge and Ownership Rules

| Target node  | Core attrs                | Adapter-owned attrs                                         | Consumer attrs                    | Merge order                                                                          | Ownership notes            |
| ------------ | ------------------------- | ----------------------------------------------------------- | --------------------------------- | ------------------------------------------------------------------------------------ | -------------------------- |
| `Root`       | `api.root_attrs()`        | structural `data-*`, animation coordination attrs if needed | consumer root attrs               | core role, pressed/checked, and disabled attrs win; `class`/`style` merge additively | adapter-owned root         |
| `OnContent`  | `api.on_content_attrs()`  | presence or animation wrapper attrs                         | consumer on-content slot content  | core state attrs win; consumer content decorates inside the wrapper                  | adapter-owned slot wrapper |
| `OffContent` | `api.off_content_attrs()` | presence or animation wrapper attrs                         | consumer off-content slot content | core state attrs win; consumer content decorates inside the wrapper                  | adapter-owned slot wrapper |

- Consumers must not collapse `OnContent` and `OffContent` into a single unnamed node.
- Visibility helpers may be adapter-owned, but they must not replace the core ownership of the two content parts.

## 6. Composition / Context Contract

No external context is required. The named content slots are the adapter-level representation of the two core content parts.

## 7. Prop Sync and Event Mapping

Switching between controlled and uncontrolled checked state is not supported after mount. `default_checked` is init-only. `checked` and `disabled` use effect-based sync.

| Adapter prop      | Mode                        | Sync trigger            | Machine event / update path | Visible effect                                        | Notes          |
| ----------------- | --------------------------- | ----------------------- | --------------------------- | ----------------------------------------------------- | -------------- |
| `checked`         | controlled                  | prop change after mount | `SetOn` / `SetOff`          | updates visible content and pressed/checked semantics | immediate sync |
| `default_checked` | uncontrolled internal state | initial render only     | initial machine props       | seeds internal state                                  | read once      |
| `disabled`        | controlled                  | prop change after mount | `SetDisabled`               | blocks interaction and updates disabled semantics     | immediate sync |

| UI event                       | Preconditions      | Machine event / callback path        | Ordering notes                                                           | Notes                                                  |
| ------------------------------ | ------------------ | ------------------------------------ | ------------------------------------------------------------------------ | ------------------------------------------------------ |
| click or normalized activation | root interactive   | `Toggle` then public change callback | callback fires after state update unless explicitly documented otherwise | native and custom button strategies must converge here |
| focus / blur                   | root focus changes | focus-visible normalization path     | focus normalization precedes notification-only callbacks                 | keeps visible-focus behavior stable                    |

## 8. Registration and Cleanup Contract

- No repeated child registration exists beyond any optional animation or presence bookkeeping.
- If a presence system is used, it must register per-content wrapper state and clean it up before parent teardown.
- Without animation bookkeeping, registration lifecycle is limited to normal node disposal.

| Registered entity                 | Registration trigger                           | Identity key                             | Cleanup trigger                           | Cleanup action                             | Notes                                                        |
| --------------------------------- | ---------------------------------------------- | ---------------------------------------- | ----------------------------------------- | ------------------------------------------ | ------------------------------------------------------------ |
| presence or animation bookkeeping | first animated mount when animation is enabled | swap instance plus content part identity | animation completion or component cleanup | clear presence state and pending callbacks | `OnContent` and `OffContent` identities must remain distinct |

## 9. Ref and Node Contract

| Target part / node                  | Ref required?                                                          | Ref owner     | Node availability                                                       | Composition rule                           | Notes                                                   |
| ----------------------------------- | ---------------------------------------------------------------------- | ------------- | ----------------------------------------------------------------------- | ------------------------------------------ | ------------------------------------------------------- |
| `Root`                              | yes when focus normalization or animation coordination needs it        | adapter-owned | required after mount                                                    | compose only when wrappers expose the root | Needed for interactive and optional animation behavior. |
| `OnContent` / `OffContent` wrappers | yes only when presence/animation strategy measures or coordinates them | adapter-owned | client-only when measured; otherwise always structural, handle optional | no consumer composition by default         | Content wrappers remain adapter-owned structural nodes. |

## 10. State Machine Boundary Rules

- machine-owned state: checked/on-off state, disabled state, and canonical content-part identity.
- adapter-local derived bookkeeping: optional animation/presence handles and root ref.
- forbidden local mirrors: do not keep an unsynchronized local checked flag separate from the machine.
- allowed snapshot-read contexts: render derivation, activation handlers, animation coordination, and cleanup.

## 11. Callback Payload Contract

| Callback                     | Payload source           | Payload shape       | Timing                                                                                 | Cancelable? | Notes                                                |
| ---------------------------- | ------------------------ | ------------------- | -------------------------------------------------------------------------------------- | ----------- | ---------------------------------------------------- |
| change callback when exposed | machine-derived snapshot | `{ checked: bool }` | after normalized state transition; after animation only if the spec explicitly says so | no          | Timing must match the documented animation strategy. |

## 12. Failure and Degradation Rules

| Condition                                                                           | Policy    | Notes                                                                |
| ----------------------------------------------------------------------------------- | --------- | -------------------------------------------------------------------- |
| missing `OnContent` or `OffContent` structure in a configuration that requires both | fail fast | The adapter must preserve both conceptual content parts.             |
| animation handles unavailable on the server                                         | no-op     | Keep structural content output and defer animation work until mount. |

## 13. Identity and Key Policy

| Registered or repeated structure    | Identity source | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                | Notes                                                                        |
| ----------------------------------- | --------------- | ------------------- | ---------------------------------------- | ------------------------------------------------------ | ---------------------------------------------------------------------------- |
| `OnContent` / `OffContent` wrappers | data-derived    | not applicable      | yes relative to documented wrapper order | initial checked structure must remain hydration-stable | The two content parts are distinct identities, not interchangeable wrappers. |
| animation/presence bookkeeping      | composite       | not applicable      | not applicable                           | client-only when measured                              | Identity is swap instance plus content part.                                 |

## 14. SSR and Client Boundary Rules

- SSR must preserve the initial checked-state structure, including distinct `OnContent` and `OffContent` wrappers as documented.
- Root and content-wrapper refs are server-safe absent and required after mount only when animation or focus logic needs them.
- Animation/presence work is client-only.

## 15. Performance Constraints

- Presence bookkeeping must only update when checked state changes, not on unrelated rerenders.
- Do not rebuild both content wrappers from unrelated wrapper state when the machine state is unchanged.
- Animation cleanup must release only the callbacks/resources owned by the current content transition.

## 16. Implementation Dependencies

| Dependency | Required?   | Dependency type         | Why it must exist first                                                                    | Notes                                                      |
| ---------- | ----------- | ----------------------- | ------------------------------------------------------------------------------------------ | ---------------------------------------------------------- |
| `button`   | recommended | behavioral prerequisite | Button-like root behavior provides a normalized activation model when swap is interactive. | Useful when the root is rendered as a button-like control. |

## 17. Recommended Implementation Sequence

1. Initialize the swap machine and render `Root`, `OnContent`, and `OffContent`.
2. Wire checked/disabled sync.
3. Normalize activation and change callbacks.
4. Add animation or presence bookkeeping only after structural identity is stable.
5. Verify cleanup and hydration structure.

## 18. Anti-Patterns

- Do not collapse `OnContent` and `OffContent` into a single unnamed wrapper.
- Do not let animation bookkeeping become the source of truth for checked state.
- Do not change the initial checked structure between SSR and hydration.

## 19. Consumer Expectations and Guarantees

- Consumers may assume documented adapter-owned structural nodes and attrs remain the canonical implementation surface.
- Consumers may assume framework-specific divergence is called out explicitly rather than hidden in generic prose.
- Consumers must not assume unspecified fallback behavior, cleanup ordering, or helper ownership beyond what this adapter spec documents.

## 20. Platform Support Matrix

| Capability / behavior                     | Web          | Desktop      | Mobile       | SSR          | Notes                                                                                                |
| ----------------------------------------- | ------------ | ------------ | ------------ | ------------ | ---------------------------------------------------------------------------------------------------- |
| documented structural and state semantics | full support | full support | full support | full support | This utility does not have additional platform variance beyond its existing framework and SSR rules. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                            | Debug build behavior | Production behavior | Notes                                                                               |
| -------------------------------------------------------------------- | -------------------- | ------------------- | ----------------------------------------------------------------------------------- |
| no component-specific diagnostics beyond documented failure policies | not applicable       | not applicable      | Use the `Failure and Degradation Rules` section as the full runtime policy surface. |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required?      | Responsibility                                                                          | Reused by      | Notes                                                         |
| ----------------- | -------------- | --------------------------------------------------------------------------------------- | -------------- | ------------------------------------------------------------- |
| attr merge helper | not applicable | No special helper beyond the documented attr derivation and merge contract is required. | not applicable | Use the normal machine attr derivation path for this utility. |

## 23. Framework-Specific Behavior

Dioxus may implement animation by keeping both parts mounted and toggling visibility, or by composing with Presence while still preserving the documented `OnContent` and `OffContent` wrappers.

## 24. Canonical Implementation Sketch

```rust
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SwapSketchProps {
    pub on_content: Element,
    pub off_content: Element,
}

#[component]
pub fn Swap(props: SwapSketchProps) -> Element {
    let machine = use_machine::<swap::Machine>(swap::Props::default());
    let root_attrs = machine.derive(|api| api.root_attrs());
    let on_attrs = machine.derive(|api| api.on_content_attrs());
    let off_attrs = machine.derive(|api| api.off_content_attrs());

    rsx! {
        button {
            ..root_attrs.read().clone(),
            onclick: move |_| machine.send.call(swap::Event::Toggle),
            span { ..on_attrs.read().clone(), {props.on_content} }
            span { ..off_attrs.read().clone(), {props.off_content} }
        }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
let machine = use_machine::<swap::Machine>(props);
let root_ref = create_optional_root_ref();
let presence = create_presence_bookkeeping_helper();

render_root_with_on_and_off_content(root_ref);
sync_checked_state(machine, props);
wire_activation_and_callback_timing(machine);
track_presence_or_animation_state(presence, machine);

on_cleanup(|| presence.release_before_machine_disposal());
```

## 26. Adapter Invariants

- `OnContent` and `OffContent` must remain distinct conceptual parts even if a transition strategy shares wrappers.
- Visibility and animation strategies must not erase part identity or collapse both content parts into one unnamed node.
- Controlled checked-state synchronization must define how external updates reconcile with the machine state.
- Presence or animation cleanup must finish before parent teardown invalidates callbacks or refs.
- Callbacks must state whether they fire on state change or after transition completion and the adapter must preserve that timing.

## 27. Accessibility and SSR Notes

- `Root` must preserve `role="button"` and `aria-pressed`.
- Both content wrappers must remain distinguishable in the rendered structure even if one is hidden or unmounted.
- SSR must preserve the initial checked-state structure used for hydration.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, part, and event parity.

Intentional deviations: animation presence strategies may differ, but they must not erase the conceptual `OnContent` and `OffContent` parts.

## 29. Test Scenarios

- explicit `Root`, `OnContent`, and `OffContent` mapping
- controlled vs uncontrolled checked state
- always-mounted vs presence-controlled content strategy
- focus-visible handling
- `on_change` callback ordering

## 30. Test Oracle Notes

| Behavior                  | Preferred oracle type | Notes                                                                    |
| ------------------------- | --------------------- | ------------------------------------------------------------------------ |
| on/off content identity   | rendered structure    | Assert separate wrappers for `OnContent` and `OffContent`.               |
| checked-state transitions | callback order        | Verify change-callback timing relative to state or animation completion. |
| initial SSR/CSR structure | hydration structure   | Assert the same checked structure survives hydration.                    |

## 31. Implementation Checklist

- [ ] `OnContent` and `OffContent` remain distinct structural parts.
- [ ] Checked sync and activation behavior are verified.
- [ ] Animation/presence bookkeeping does not replace machine-owned state.
- [ ] Hydration structure and callback timing are covered by tests.
