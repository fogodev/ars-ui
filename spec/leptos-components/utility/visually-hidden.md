---
adapter: leptos
component: visually-hidden
category: utility
source: components/utility/visually-hidden.md
source_foundation: foundation/08-adapter-leptos.md
---

# VisuallyHidden — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`VisuallyHidden`](../../components/utility/visually-hidden.md) utility to Leptos 0.8.x.

## 2. Public Adapter API

```rust,no_check
#[component] pub fn VisuallyHidden(...) -> impl IntoView
```

The adapter surfaces the full core prop set: `id`, `as_child`, and `is_focusable`.

## 3. Mapping to Core Component Contract

- Props parity: full parity.
- Structure parity: root wrapper by default, reassigned root under `as_child`.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target                              | Ownership                                                        | Attr source        | Notes                                         |
| --------------------- | --------- | ----------------------------------------------------- | ---------------------------------------------------------------- | ------------------ | --------------------------------------------- |
| `Root`                | required  | default `<span>`; consumer child when `as_child=true` | adapter-owned by default; consumer-owned under root reassignment | `api.root_attrs()` | The conceptual root remains under `as_child`. |

## 5. Attr Merge and Ownership Rules

| Target node | Core attrs                 | Adapter-owned attrs                  | Consumer attrs      | Merge order                                                                                                                   | Ownership notes                                           |
| ----------- | -------------------------- | ------------------------------------ | ------------------- | ----------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `Root`      | core visually-hidden attrs | hidden-style and focus-reveal styles | consumer root attrs | core hidden styles and accessibility-preserving attrs win; `class`/`style` merge additively without removing hidden semantics | adapter-owned by default; consumer-owned under `as_child` |

- Under `as_child`, root reassignment does not allow the consumer node to drop the visually-hidden contract.
- Focus-reveal styles, when supported, are adapter-owned semantics and must survive attr merging.

## 6. Composition / Context Contract

When `as_child=true`, follow the `AsChild` root-reassignment rules.

## 7. Prop Sync and Event Mapping

This utility has no long-lived machine sync beyond any optional focusable-reveal configuration.

| Adapter prop             | Mode                      | Sync trigger     | Machine event / update path      | Visible effect                                             | Notes                                                                                    |
| ------------------------ | ------------------------- | ---------------- | -------------------------------- | ---------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| focusable reveal options | non-reactive adapter prop | render time only | included in root attr derivation | determines whether hidden content becomes visible on focus | post-mount changes should be treated as unsupported unless re-rendered through a wrapper |

## 8. Registration and Cleanup Contract

- No registration lifecycle exists beyond normal node disposal.
- If focus-reveal bookkeeping exists, it is local to the root node and ends at component cleanup.

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

| Dependency | Required?   | Dependency type      | Why it must exist first                                                        | Notes                                               |
| ---------- | ----------- | -------------------- | ------------------------------------------------------------------------------ | --------------------------------------------------- |
| `as-child` | recommended | composition contract | Root reassignment must preserve hidden semantics when polymorphism is allowed. | Relevant only when the utility supports `as_child`. |

## 17. Recommended Implementation Sequence

1. Render the hidden root structure.
2. Apply hidden and focus-reveal semantics.
3. Add `as_child` root reassignment if supported.
4. Verify structure and accessibility attrs through the documented test oracles.

## 18. Anti-Patterns

- Do not replace visually hidden semantics with `display: none`.
- Do not let `as_child` remove the hidden accessibility contract.

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

Leptos may use an adapter-local slot helper for `as_child`.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn VisuallyHidden(children: Children) -> impl IntoView {
    let api = visually_hidden::Api::new(visually_hidden::Props::default());
    view! { <span {..api.root_attrs()}>{children()}</span> }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton beyond the canonical sketch is required for this utility.

## 26. Adapter Invariants

- Hidden content must remain in the accessibility tree and must not be collapsed into `display: none`.
- Focusable reveal behavior must be explicit wherever the utility exposes content on keyboard focus.
- `as_child` must remain root reassignment rather than structural node loss.

## 27. Accessibility and SSR Notes

Must preserve accessibility-tree visibility and focusable reveal behavior.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- default root mapping
- root reassignment under `as_child`
- focusable reveal mode

## 30. Test Oracle Notes

| Behavior                               | Preferred oracle type | Notes                                                                      |
| -------------------------------------- | --------------------- | -------------------------------------------------------------------------- |
| structural rendering and part presence | rendered structure    | Verify the documented part mapping rather than incidental wrapper details. |
| accessibility and state attrs          | DOM attrs             | Assert the normalized attrs emitted by the adapter-owned node.             |

## 31. Implementation Checklist

- [ ] Hidden semantics are applied without removing content from the accessibility tree.
- [ ] `as_child` preserves the documented root contract when supported.
- [ ] Rendered structure and DOM-attr oracles are covered.
