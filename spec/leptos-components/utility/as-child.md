---
adapter: leptos
component: as-child
category: utility
source: components/utility/as-child.md
source_foundation: foundation/08-adapter-leptos.md
---

# AsChild — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`AsChild`](../../components/utility/as-child.md) pattern to Leptos 0.8.x. It documents root reassignment and attr or handler merging for components that expose `as_child`.

## 2. Public Adapter API

```rust
#[component]
pub fn AsChildSlot(
    attrs: AttrMap,
    children: Children,
) -> impl IntoView
```

`as_child: bool` is surfaced by the hosting component, not by a standalone core component.

## 3. Mapping to Core Component Contract

- Pattern parity: the adapter must preserve root reassignment semantics.
- Attr parity: component attrs merge onto the consumer child.
- Event parity: component handlers and child handlers must both be preserved.

## 4. Part Mapping

| Core part / structure      | Required?                     | Adapter rendering target      | Ownership        | Attr source                | Notes                                                             |
| -------------------------- | ----------------------------- | ----------------------------- | ---------------- | -------------------------- | ----------------------------------------------------------------- |
| reassigned root            | required when `as_child=true` | single consumer child element | consumer-owned   | merged component `AttrMap` | The conceptual root remains; only the rendering target changes.   |
| suppressed default wrapper | conditional                   | no DOM output                 | adapter behavior | none                       | Must be documented whenever default wrapper rendering is skipped. |

## 5. Attr Merge and Ownership Rules

| Target node           | Core attrs                                                                    | Adapter-owned attrs                                                           | Consumer attrs                                  | Merge order                                                                                                                                                                                                                                   | Ownership notes                                                                                 |
| --------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| reassigned root child | forwarded component `AttrMap` including ARIA, role, state, and event handlers | any adapter-local structural `data-*` markers needed by the hosting component | the child element's original attrs and handlers | core required ARIA/state attrs win when conflict would break the contract; child `class`/`style` merge additively; handler order is adapter normalization first for guard logic, then child handler, then adapter notification-only callbacks | rendered node is consumer-owned, but semantic root ownership remains with the hosting component |

- Exactly one child element is allowed.
- No wrapper node may be introduced in `as_child` mode.
- If the child already has `role`, `tabindex`, or `aria-*`, the merge result must preserve required core semantics instead of blindly preferring the child value.
- If a child handler calls `prevent_default()`, later notification-only handlers may observe that state but must not re-enable a blocked action.

## 6. Composition / Context Contract

Exactly one child element is required. Context behavior of the hosting component does not change under root reassignment.

## 7. Prop Sync and Event Mapping

`AsChild` itself has no independent machine state. The hosting component remains responsible for prop sync. This adapter slot only normalizes forwarded attrs and handlers.

| Adapter prop      | Mode                      | Sync trigger                         | Machine event / update path                | Visible effect                                       | Notes                                       |
| ----------------- | ------------------------- | ------------------------------------ | ------------------------------------------ | ---------------------------------------------------- | ------------------------------------------- |
| forwarded `attrs` | non-reactive adapter prop | each render of the hosting component | forwarded directly to the reassigned child | child receives the hosting component's root contract | no independent controlled/uncontrolled mode |

| UI event             | Preconditions                   | Machine event / callback path           | Ordering notes                                                                                                                                                   | Notes                                                                            |
| -------------------- | ------------------------------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| child event handlers | exactly one valid child element | composed hosting-component handler path | guard logic runs before child-only behavior when the hosting component needs to prevent invalid activation; notification-only callbacks run after child handlers | actual machine events are defined by the hosting component, not by `AsChildSlot` |

## 8. Registration and Cleanup Contract

- No registration lifecycle exists beyond validating that exactly one child is present.
- Child-count validation happens at render time.
- Cleanup is ordinary vnode disposal; no persistent listeners or timers belong to the slot helper itself.

| Registered entity       | Registration trigger | Identity key               | Cleanup trigger                  | Cleanup action                    | Notes                                                                    |
| ----------------------- | -------------------- | -------------------------- | -------------------------------- | --------------------------------- | ------------------------------------------------------------------------ |
| child validation result | each render          | hosting component instance | next render or component cleanup | discard previous validation state | multiple children and zero-child cases are immediate contract violations |

## 9. Ref and Node Contract

| Target part / node                         | Ref required?                                                                          | Ref owner                                             | Node availability                  | Composition rule                                                 | Notes                                                                              |
| ------------------------------------------ | -------------------------------------------------------------------------------------- | ----------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| primary rendered node or provider boundary | no when the component is purely structural or provider-only; otherwise adapter-defined | adapter-owned unless part mapping says consumer-owned | always structural, handle optional | composed only when a consumer-owned node receives the core attrs | Use a live node handle only when the component's interaction contract requires it. |

## 10. State Machine Boundary Rules

- machine-owned state: all core interaction, accessibility, and controlled-state values defined by the component machine or derived API.
- adapter-local derived bookkeeping: minimal ephemeral data such as pointer modality, mount status, or observer handles when the core machine does not model them directly.
- forbidden local mirrors: do not fork controlled props, accessibility attrs, or machine-visible state into an unsynchronized local source of truth.
- allowed snapshot-read contexts: render-time derivation, event handlers, effects, and cleanup only when reading snapshots does not bypass required machine events.

## 11. Callback Payload Contract

| Callback                                                               | Payload source | Payload shape | Timing         | Cancelable? | Notes                                                                                                                 |
| ---------------------------------------------------------------------- | -------------- | ------------- | -------------- | ----------- | --------------------------------------------------------------------------------------------------------------------- |
| no public adapter-specific callback beyond normalized component events | none           | none          | not applicable | no          | When wrappers expose callbacks, they must preserve the normalized timing documented in `Prop Sync and Event Mapping`. |

## 12. Failure and Degradation Rules

| Condition                                                                  | Policy             | Notes                                                                             |
| -------------------------------------------------------------------------- | ------------------ | --------------------------------------------------------------------------------- |
| unsupported platform capability or missing browser-only API during SSR     | degrade gracefully | Render structural output and defer behavior until client-only APIs are available. |
| impossible prop combinations not explicitly supported by the core contract | fail fast          | Prefer an explicit contract violation over silently inventing behavior.           |

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

| Capability / behavior                     | Browser client | SSR          | Notes                                                                                         |
| ----------------------------------------- | -------------- | ------------ | --------------------------------------------------------------------------------------------- |
| documented structural and state semantics | full support   | full support | This utility does not have additional platform variance beyond its existing SSR/client rules. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                                           | Debug build behavior | Production behavior | Notes                                                                                       |
| ----------------------------------------------------------------------------------- | -------------------- | ------------------- | ------------------------------------------------------------------------------------------- |
| anchor-like child receives button semantics without equivalent Space-key activation | debug warning        | no diagnostic       | Diagnostic only; merged semantics remain unchanged until the host fixes the child behavior. |
| zero or multiple children in root reassignment mode                                 | fail fast            | fail fast           | `as_child` requires exactly one concrete child element.                                     |

## 22. Shared Adapter Helper Notes

| Helper concept                | Required?   | Responsibility                                                                      | Reused by                                                             | Notes                                                                         |
| ----------------------------- | ----------- | ----------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `as_child` vnode merge helper | required    | Clone or rebuild exactly one child with merged attrs, handlers, and guard ordering. | `as-child`, `button`, `visually-hidden`, any polymorphic root utility | This helper owns root reassignment without deleting the conceptual root part. |
| debug-warning helper          | recommended | Emit semantic-mismatch diagnostics in debug builds only.                            | `as-child`, `button`, `download-trigger`, `action-group`              | Warnings must never silently alter runtime semantics.                         |

## 23. Framework-Specific Behavior

Leptos needs an adapter-local helper because arbitrary vnode mutation is not built in. In debug builds, the adapter should emit a warning when `as_child` forwards button semantics such as `role="button"` onto an anchor-like child that still lacks Space-key activation semantics, because the merged result may look valid while remaining keyboard-incomplete.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn AsChildSlot(attrs: AttrMap, children: Children) -> impl IntoView {
    // Adapter-local helper: render exactly one child with merged attrs.
    view! { <>{children()}</> }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton beyond the canonical sketch is required for this utility.

## 26. Adapter Invariants

- Exactly one consumer child is required whenever root reassignment is used.
- Root reassignment must not delete the conceptual root part; it only changes ownership of the rendered node.
- Handler composition order must be explicit so consumer handlers and adapter handlers do not race unpredictably.
- Role, ARIA, and state attr merge rules must remain explicit when attrs are forwarded onto the child.

## 27. Accessibility and SSR Notes

Semantic correctness of the final root element remains the responsibility of the hosting component and consumer.
When `as_child` merges button semantics onto an anchor-like child, the adapter should warn in debug builds if the resulting element still relies on link-native keyboard behavior and therefore does not activate on Space.

## 28. Parity Summary and Intentional Deviations

Parity summary: full pattern parity.

Intentional deviations: none.

## 29. Test Scenarios

- root reassignment
- suppressed wrapper documentation
- merged handler preservation
- debug warning for anchor-like children that receive button semantics without equivalent Space-key activation

## 30. Test Oracle Notes

| Behavior                               | Preferred oracle type | Notes                                                                                                           |
| -------------------------------------- | --------------------- | --------------------------------------------------------------------------------------------------------------- |
| structural rendering and part presence | rendered structure    | Verify the documented part mapping rather than incidental wrapper details.                                      |
| accessibility and state attrs          | DOM attrs             | Assert the normalized attrs emitted by the adapter-owned node.                                                  |
| semantic-mismatch warning              | cleanup side effects  | Verify debug-only warning or equivalent diagnostic fires for anchor-like children with merged button semantics. |

## 31. Implementation Checklist

- [ ] All documented parts and structural nodes are rendered with the correct ownership model.
- [ ] Attr merge precedence matches the documented contract.
- [ ] Prop sync and event normalization follow the documented machine paths.
- [ ] Cleanup releases every resource owned by the component instance.
- [ ] Debug-only semantic-mismatch warnings are documented for anchor-like children that receive button semantics through `as_child`.
- [ ] SSR/client boundary behavior matches the documented structure and test oracles.
