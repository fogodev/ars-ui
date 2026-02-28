---
adapter: dioxus
component: toggle
category: utility
source: components/utility/toggle.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Toggle — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Toggle`](../../components/utility/toggle.md) machine to Dioxus 0.7.x.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ToggleProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub pressed: Option<bool>,
    #[props(default = false)]
    pub default_pressed: bool,
    #[props(default = false)]
    pub disabled: bool,
    #[props(optional)]
    pub on_change: Option<EventHandler<bool>>,
    pub children: Element,
}

#[component]
pub fn Toggle(props: ToggleProps) -> Element
```

The adapter surfaces the full core prop set: `id`, `pressed`, `default_pressed`, `disabled`, and `on_change`.

## 3. Mapping to Core Component Contract

- Props parity: full parity.
- Event parity: toggle, controlled sync, focus, blur, and disabled sync are adapter-driven.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership                                    | Attr source                               | Notes                         |
| --------------------- | --------- | ------------------------ | -------------------------------------------- | ----------------------------------------- | ----------------------------- |
| `Root`                | required  | native `<button>`        | adapter-owned                                | `api.part_attrs(toggle::Part::Root)`      | Interactive toggle root.      |
| `Indicator`           | optional  | `<span>` inside root     | adapter-owned or consumer-owned substructure | `api.part_attrs(toggle::Part::Indicator)` | Optional conditional content. |

## 5. Attr Merge and Ownership Rules

| Target node        | Core attrs                          | Adapter-owned attrs                  | Consumer attrs                             | Merge order                                                                        | Ownership notes              |
| ------------------ | ----------------------------------- | ------------------------------------ | ------------------------------------------ | ---------------------------------------------------------------------------------- | ---------------------------- |
| `Root`             | root attrs from the core toggle API | optional structural `data-*` helpers | consumer root attrs                        | core disabled, role, and checked-state attrs win; `class`/`style` merge additively | adapter-owned root           |
| optional indicator | core indicator attrs if exposed     | structural indicator wrapper attrs   | consumer indicator content only if exposed | core indicator/state attrs win                                                     | adapter-owned indicator node |

## 6. Composition / Context Contract

No external context required.

## 7. Prop Sync and Event Mapping

Switching between controlled and uncontrolled state is not supported after mount. Default state is init-only; controlled state and disabled state use effect-based sync.

| Adapter prop     | Mode                        | Sync trigger            | Machine event / update path        | Visible effect                                   | Notes          |
| ---------------- | --------------------------- | ----------------------- | ---------------------------------- | ------------------------------------------------ | -------------- |
| checked/on state | controlled                  | prop change after mount | `TurnOn` / `TurnOff` or equivalent | updates pressed/checked semantics                | immediate sync |
| default state    | uncontrolled internal state | initial render only     | initial machine props              | seeds internal state                             | read once      |
| disabled         | controlled                  | prop change after mount | `SetDisabled`                      | blocks activation and updates disabled semantics | immediate sync |

| UI event         | Preconditions                  | Machine event / callback path | Ordering notes                                                                    | Notes                          |
| ---------------- | ------------------------------ | ----------------------------- | --------------------------------------------------------------------------------- | ------------------------------ |
| activation event | root interactive               | `Toggle`                      | callback timing must be after normalized state update unless documented otherwise | root may be native or custom   |
| focus / blur     | focus-visible tracking enabled | normalized focus path         | focus normalization precedes notification-only callbacks                          | keeps modality behavior stable |

## 8. Registration and Cleanup Contract

- No registration lifecycle exists beyond ordinary root disposal unless the adapter exposes an optional indicator with animation bookkeeping.
- If indicator-specific bookkeeping exists, it must be cleaned up on unmount.

## 9. Ref and Node Contract

| Target part / node                         | Ref required?                                                                          | Ref owner                                             | Node availability                  | Composition rule                                                 | Notes                                                                              |
| ------------------------------------------ | -------------------------------------------------------------------------------------- | ----------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| primary rendered node or provider boundary | no when the component is purely structural or provider-only; otherwise adapter-defined | adapter-owned unless part mapping says consumer-owned | always structural, handle optional | composed only when a consumer-owned node receives the core attrs | Use a live node handle only when the component's interaction contract requires it. |

## 10. State Machine Boundary Rules

- machine-owned state: all core interaction, accessibility, and controlled-state values defined by the component machine or derived API.
- adapter-local derived bookkeeping: minimal ephemeral data such as pointer modality, mount status, or observer handles when the core machine does not model them directly.
- forbidden local mirrors: do not fork controlled props, accessibility attrs, or machine-visible state into an unsynchronized local source of truth.
- async reconciliation path: when a wrapper performs optimistic async `on_change` work, the adapter must treat the machine state as the source of truth and revert by sending `Toggle`, `TurnOn`, or `TurnOff` if the async action fails.
- allowed snapshot-read contexts: render-time derivation, event handlers, effects, and cleanup only when reading snapshots does not bypass required machine events.

## 11. Callback Payload Contract

| Callback                                                               | Payload source | Payload shape | Timing         | Cancelable? | Notes                                                                                                                 |
| ---------------------------------------------------------------------- | -------------- | ------------- | -------------- | ----------- | --------------------------------------------------------------------------------------------------------------------- |
| no public adapter-specific callback beyond normalized component events | none           | none          | not applicable | no          | When wrappers expose callbacks, they must preserve the normalized timing documented in `Prop Sync and Event Mapping`. |

## 12. Failure and Degradation Rules

| Condition                                                                               | Policy             | Notes                                                                                                                                       |
| --------------------------------------------------------------------------------------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| unsupported platform capability or missing browser-only API during SSR                  | degrade gracefully | Render structural output and defer behavior until client-only APIs are available.                                                           |
| impossible prop combinations not explicitly supported by the core contract              | fail fast          | Prefer an explicit contract violation over silently inventing behavior.                                                                     |
| optimistic async change fails after the adapter already reflected the new pressed state | degrade gracefully | Revert by sending the compensating machine event and surface any error indication in wrapper-owned UI without leaving `aria-pressed` stale. |

## 13. Identity and Key Policy

| Registered or repeated structure   | Identity source | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                                  | Notes                                                                                             |
| ---------------------------------- | --------------- | ------------------- | ---------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| none beyond the component instance | not applicable  | not applicable      | not applicable                           | stable root structure required where the component renders on the server | Use a component-instance identity only for cleanup bookkeeping when no repeated structure exists. |

## 14. SSR and Client Boundary Rules

- The server must render every structural node required by the part-mapping table unless the component is explicitly provider-only or client-gated.
- Client-only listeners, timers, measurements, and node-handle work must wait until hydration or mount.
- Any node that participates in hydration-sensitive semantics must preserve the same structural identity across server and client render paths.

## 15. Performance Constraints

- Attr maps derived from the machine should be memoized or read through adapter derivation helpers instead of rebuilt eagerly on every render.
- Listener, timer, and observer registration must be stable across rerenders and must not churn unless the governing configuration actually changes.
- Cleanup must release only the resources owned by the current component instance and must avoid repeated quadratic teardown work.

## 16. Implementation Dependencies

| Dependency                                  | Required?   | Dependency type | Why it must exist first                                                                                         | Notes                                                                 |
| ------------------------------------------- | ----------- | --------------- | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| none beyond the documented utility contract | recommended | conceptual      | The component may still rely on shared adapter patterns even when no specific prerequisite utility is required. | State `not applicable` explicitly if there is no stronger dependency. |

## 17. Recommended Implementation Sequence

1. Establish the machine/context boundary and confirm the documented part mapping.
2. Establish any required refs, provider context, or registration surfaces.
3. Derive attrs and render the documented structural nodes.
4. Wire prop sync and normalized event handling.
5. Add SSR/client-only guards, cleanup, and verify the documented test oracles.

## 18. Anti-Patterns

- Do not mirror machine-owned state in unsynchronized local adapter state.
- Do not bypass the documented attr merge order or replace required structural nodes with equivalent-looking wrappers.
- Do not re-register listeners, timers, observers, or registries on every render when the governing configuration has not changed.

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

Dioxus may omit the indicator entirely when the consumer does not render it.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ToggleSketchProps {
    pub children: Element,
}

#[component]
pub fn Toggle(props: ToggleSketchProps) -> Element {
    let machine = use_machine::<toggle::Machine>(toggle::Props::default());
    let root_attrs = machine.derive(|api| api.part_attrs(toggle::Part::Root));
    rsx! { button { ..root_attrs.read().clone(), {props.children} } }
}
```

## 25. Reference Implementation Skeleton

```rust
let machine = use_machine::<toggle::Machine>(props);
let root_attrs = derive_root_attrs(machine);
let indicator_attrs = maybe_derive_indicator_attrs(machine);

render_toggle_root_and_optional_indicator(root_attrs, indicator_attrs);
sync_controlled_and_disabled_props(machine, props);
normalize_activation_focus_and_blur(machine);
if optimistic_wrapper_behavior_exists() {
    rollback_through_machine_events_on_async_failure(machine);
}
```

## 26. Adapter Invariants

- The adapter must document when disabled behavior relies on `aria-disabled` rather than HTML `disabled` and preserve keyboard discoverability accordingly.
- No hidden form participation should be introduced unless the core contract explicitly defines it.
- Indicator or content structure must stay explicit wherever the adapter renders additional visual state nodes.
- Controlled synchronization must define the exact prop-to-machine reconciliation path.
- Optimistic async updates must either roll back through machine events on failure or remain pessimistic; the adapter must not leave `aria-pressed` in an unconfirmed intermediate state.

## 27. Accessibility and SSR Notes

Must preserve `aria-pressed` and no form participation.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- root mapping
- optional indicator mapping
- controlled vs uncontrolled state
- optimistic async update fails and state rolls back to the previous confirmed value

## 30. Test Oracle Notes

| Behavior                               | Preferred oracle type | Notes                                                                                                                         |
| -------------------------------------- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| structural rendering and part presence | rendered structure    | Verify the documented part mapping rather than incidental wrapper details.                                                    |
| accessibility and state attrs          | DOM attrs             | Assert the normalized attrs emitted by the adapter-owned node.                                                                |
| optimistic async rollback              | callback order        | Verify the optimistic transition occurs first and the compensating machine event restores the prior pressed state on failure. |

## 31. Implementation Checklist

- [ ] All documented parts and structural nodes are rendered with the correct ownership model.
- [ ] Attr merge precedence matches the documented contract.
- [ ] Prop sync and event normalization follow the documented machine paths.
- [ ] Async optimistic failure handling either stays pessimistic or rolls back through the documented machine event path.
- [ ] Cleanup releases every resource owned by the component instance.
- [ ] SSR/client boundary behavior matches the documented structure and test oracles.
