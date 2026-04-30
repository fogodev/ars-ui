---
adapter: leptos
component: group
category: utility
source: components/utility/group.md
source_foundation: foundation/08-adapter-leptos.md
---

# Group — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Group`](../../components/utility/group.md) utility to Leptos 0.8.x.

## 2. Public Adapter API

```rust,no_check
#[component] pub fn Group(...) -> impl IntoView
```

## 3. Mapping to Core Component Contract

- Props parity: full parity.
- Structure parity: single root grouping element.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target      | Ownership      | Attr source        | Notes                                                                       |
| --------------------- | --------- | ----------------------------- | -------------- | ------------------ | --------------------------------------------------------------------------- |
| `Root`                | required  | wrapper `<div>`               | adapter-owned  | `api.root_attrs()` | Carries `role="group"` and inherited state attrs.                           |
| children subtree      | required  | consumer children inside root | consumer-owned | none               | May inherit group state through context if a wrapper chooses to provide it. |

## 5. Attr Merge and Ownership Rules

| Target node                         | Core attrs                                               | Adapter-owned attrs                                                          | Consumer attrs       | Merge order                                                                                                                                        | Ownership notes                                                                                                           |
| ----------------------------------- | -------------------------------------------------------- | ---------------------------------------------------------------------------- | -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `Root`                              | `api.root_attrs()`                                       | structural `data-*` markers if needed                                        | consumer root attrs  | core group semantics win; `class`/`style` merge additively                                                                                         | adapter-owned root                                                                                                        |
| children that consume group context | child utility attrs derived from their own core contract | propagated `disabled`, `invalid`, and `read_only` values from `GroupContext` | consumer child props | child-local core attrs merge with propagated group state; `aria-disabled` on the root alone must not be treated as sufficient descendant disabling | descendant utilities remain consumer-owned but must consume the published group context when they support inherited state |

## 6. Composition / Context Contract

The adapter publishes `GroupContext` so descendant utilities can merge inherited `disabled`, `invalid`, and `read_only` state with their own props. `aria-disabled` on the root container does not disable descendants by itself; child adapters that participate in group inheritance must read the context and apply the effective state programmatically.

## 7. Prop Sync and Event Mapping

Group is usually stateless beyond derived props.

| Adapter prop             | Mode                      | Sync trigger     | Machine event / update path | Visible effect             | Notes                       |
| ------------------------ | ------------------------- | ---------------- | --------------------------- | -------------------------- | --------------------------- |
| grouping semantics props | non-reactive adapter prop | render time only | root attr derivation        | updates grouping semantics | no post-mount sync expected |

## 8. Registration and Cleanup Contract

- No registration lifecycle exists beyond normal root disposal unless a wrapper adds dynamic group context bookkeeping.

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

- Consumers may assume published `GroupContext` carries inherited disabled, invalid, and read-only state.
- Consumers may assume root `aria-disabled` is only part of the accessibility contract, not the descendant enforcement mechanism.
- Consumers must not assume root ARIA alone disables descendants without reading group context.

## 20. Platform Support Matrix

| Capability / behavior                     | Browser client | SSR          | Notes                                                                                         |
| ----------------------------------------- | -------------- | ------------ | --------------------------------------------------------------------------------------------- |
| documented structural and state semantics | full support   | full support | This utility does not have additional platform variance beyond its existing SSR/client rules. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                            | Debug build behavior | Production behavior | Notes                                                                               |
| -------------------------------------------------------------------- | -------------------- | ------------------- | ----------------------------------------------------------------------------------- |
| no component-specific diagnostics beyond documented failure policies | not applicable       | not applicable      | Use the `Failure and Degradation Rules` section as the full runtime policy surface. |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required?      | Responsibility                                                                          | Reused by      | Notes                                                         |
| ----------------- | -------------- | --------------------------------------------------------------------------------------- | -------------- | ------------------------------------------------------------- |
| attr merge helper | not applicable | No special helper beyond the documented attr derivation and merge contract is required. | not applicable | Use the normal machine attr derivation path for this utility. |

## 23. Framework-Specific Behavior

Leptos can render the root directly with spread attrs.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Group(children: Children) -> impl IntoView {
    let api = group::Api::new(group::Props::default());
    view! { <div {..api.root_attrs()}>{children()}</div> }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton beyond the canonical sketch is required for this utility.

## 26. Adapter Invariants

- Group semantics must remain structurally explicit even when the rendered node is visually neutral.
- Inherited state exposure rules must remain explicit wherever parent state is surfaced to descendants.
- Root-level `aria-disabled` must not be treated as sufficient descendant disabling; adapter-owned context propagation must carry disabled semantics into participating children.

## 27. Accessibility and SSR Notes

Must preserve group semantics and labeling attrs.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop and root parity.

Intentional deviations: none.

## 29. Test Scenarios

- root mapping
- role and invalid or disabled attrs
- disabled group state propagates to descendants through context rather than root ARIA alone

## 30. Test Oracle Notes

| Behavior                               | Preferred oracle type | Notes                                                                                                       |
| -------------------------------------- | --------------------- | ----------------------------------------------------------------------------------------------------------- |
| structural rendering and part presence | rendered structure    | Verify the documented part mapping rather than incidental wrapper details.                                  |
| accessibility and state attrs          | DOM attrs             | Assert the normalized attrs emitted by the adapter-owned node.                                              |
| descendant disabled propagation        | context registration  | Verify descendants observe propagated group disabled state instead of relying only on root `aria-disabled`. |

## 31. Implementation Checklist

- [ ] All documented parts and structural nodes are rendered with the correct ownership model.
- [ ] Attr merge precedence matches the documented contract.
- [ ] Prop sync and event normalization follow the documented machine paths.
- [ ] Descendant disabled, invalid, and read-only propagation is implemented through `GroupContext`, not only root ARIA attrs.
- [ ] Cleanup releases every resource owned by the component instance.
- [ ] SSR/client boundary behavior matches the documented structure and test oracles.
