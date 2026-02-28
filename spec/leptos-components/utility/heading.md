---
adapter: leptos
component: heading
category: utility
source: components/utility/heading.md
source_foundation: foundation/08-adapter-leptos.md
---

# Heading — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Heading`](../../components/utility/heading.md) utility to Leptos 0.8.x.

## 2. Public Adapter API

```rust
#[component] pub fn Heading(...) -> impl IntoView
```

## 3. Mapping to Core Component Contract

- Props parity: full parity.
- Structure parity: single root heading node plus optional provider-only heading level context in wrappers.

## 4. Part Mapping

| Core part / structure  | Required?     | Adapter rendering target                | Ownership     | Attr source        | Notes                                                |
| ---------------------- | ------------- | --------------------------------------- | ------------- | ------------------ | ---------------------------------------------------- |
| `Root`                 | required      | semantic `<h1>`-`<h6>` or fallback node | adapter-owned | `api.root_attrs()` | Root element choice depends on computed level.       |
| heading level provider | provider-only | no DOM output                           | provider-only | none               | Optional structural context used by section helpers. |

## 5. Attr Merge and Ownership Rules

| Target node  | Core attrs                      | Adapter-owned attrs                               | Consumer attrs      | Merge order                                                                    | Ownership notes    |
| ------------ | ------------------------------- | ------------------------------------------------- | ------------------- | ------------------------------------------------------------------------------ | ------------------ |
| heading root | heading attrs from the core API | fallback role attrs when non-native tags are used | consumer root attrs | core level, role, and labeling semantics win; `class`/`style` merge additively | adapter-owned root |

## 6. Composition / Context Contract

Heading-level providers or section helpers must publish `HeadingContext` for nested headings. `Section`-style helpers increment the inherited level and provide the incremented value through framework context so descendant headings at arbitrary depth resolve their level without prop drilling.

## 7. Prop Sync and Event Mapping

Heading level and label semantics are typically init-only render concerns.

| Adapter prop      | Mode                      | Sync trigger     | Machine event / update path | Visible effect               | Notes                       |
| ----------------- | ------------------------- | ---------------- | --------------------------- | ---------------------------- | --------------------------- |
| level / tag props | non-reactive adapter prop | render time only | heading attr derivation     | determines heading semantics | no post-mount sync expected |

## 8. Registration and Cleanup Contract

- No registration lifecycle exists beyond normal node disposal.

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

| Condition                                                            | Debug build behavior | Production behavior | Notes                                                                               |
| -------------------------------------------------------------------- | -------------------- | ------------------- | ----------------------------------------------------------------------------------- |
| no component-specific diagnostics beyond documented failure policies | not applicable       | not applicable      | Use the `Failure and Degradation Rules` section as the full runtime policy surface. |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required?      | Responsibility                                                                          | Reused by      | Notes                                                         |
| ----------------- | -------------- | --------------------------------------------------------------------------------------- | -------------- | ------------------------------------------------------------- |
| attr merge helper | not applicable | No special helper beyond the documented attr derivation and merge contract is required. | not applicable | Use the normal machine attr derivation path for this utility. |

## 23. Framework-Specific Behavior

Leptos may select the semantic tag dynamically or use a fallback generic node with heading role. When heading text can mix document directions, the rendered heading node should set `dir="auto"` so the browser resolves reading order from the content instead of inheriting a misleading page-level direction. Heading level providers and `Section` helpers must publish the incremented `HeadingContext` with `provide_context(...)` rather than relying on prop drilling.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Heading(children: Children) -> impl IntoView {
    let api = heading::Api::new(&heading::Props::default());
    view! { <h2 {..api.root_attrs()}>{children()}</h2> }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton beyond the canonical sketch is required for this utility.

## 26. Adapter Invariants

- Heading-level resolution must be deterministic and must not drift between server and client render paths.
- Fallback role behavior must be explicit whenever the adapter does not render a native heading element.
- Nested heading-level inheritance must flow through published `HeadingContext`, not through manual prop threading.

## 27. Accessibility and SSR Notes

Must preserve heading level semantics or `aria-level` when using a fallback node.
Mixed-direction heading content should preserve readable ordering; `dir="auto"` on the rendered heading node is the recommended adapter repair when text direction may differ from the surrounding page.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- root mapping
- semantic heading vs fallback role
- nested heading-level context
- mixed-direction heading content preserves expected text direction
- `Section` or heading-level providers publish incremented context for descendants

## 30. Test Oracle Notes

| Behavior                               | Preferred oracle type | Notes                                                                               |
| -------------------------------------- | --------------------- | ----------------------------------------------------------------------------------- |
| structural rendering and part presence | rendered structure    | Verify the documented part mapping rather than incidental wrapper details.          |
| accessibility and state attrs          | DOM attrs             | Assert the normalized attrs emitted by the adapter-owned node.                      |
| mixed-direction heading handling       | DOM attrs             | Assert `dir="auto"` when the adapter enables the documented mixed-direction repair. |
| heading-level context publication      | context registration  | Verify descendant headings observe the incremented `HeadingContext`.                |

## 31. Implementation Checklist

- [ ] All documented parts and structural nodes are rendered with the correct ownership model.
- [ ] Attr merge precedence matches the documented contract.
- [ ] Prop sync and event normalization follow the documented machine paths.
- [ ] Cleanup releases every resource owned by the component instance.
- [ ] Heading level providers publish incremented `HeadingContext` instead of relying on prop drilling.
- [ ] Mixed-direction heading handling is documented and verified when text direction may differ from the page.
- [ ] SSR/client boundary behavior matches the documented structure and test oracles.
