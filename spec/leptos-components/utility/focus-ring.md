---
adapter: leptos
component: focus-ring
category: utility
source: components/utility/focus-ring.md
source_foundation: foundation/08-adapter-leptos.md
---

# FocusRing — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`FocusRing`](../../components/utility/focus-ring.md) utility to Leptos 0.8.x.

## 2. Public Adapter API

```rust,no_check
#[component] pub fn FocusRing(...) -> impl IntoView
```

The adapter surfaces the full core prop set: `id`, `within`, `focus_class`, `focus_visible_class`, and `is_text_input`.

## 3. Mapping to Core Component Contract

- Props parity: full parity.
- Structure parity: single root part only.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target  | Ownership                               | Attr source        | Notes                           |
| --------------------- | --------- | ------------------------- | --------------------------------------- | ------------------ | ------------------------------- |
| `Root`                | required  | wrapped focusable element | adapter-owned or consumer child wrapper | `api.root_attrs()` | Adds focus-visible state attrs. |

## 5. Attr Merge and Ownership Rules

| Target node    | Core attrs                         | Adapter-owned attrs                           | Consumer attrs      | Merge order                                                        | Ownership notes    |
| -------------- | ---------------------------------- | --------------------------------------------- | ------------------- | ------------------------------------------------------------------ | ------------------ |
| ring host/root | focus-ring attrs from the core API | modality-tracking classes or `data-*` markers | consumer root attrs | core focus-visible semantics win; `class`/`style` merge additively | adapter-owned root |

## 6. Composition / Context Contract

No required external context.

## 7. Prop Sync and Event Mapping

Focus-ring state is derived from focus-visible and modality tracking.

| Adapter prop               | Mode                 | Sync trigger           | Machine event / update path   | Visible effect          | Notes                                      |
| -------------------------- | -------------------- | ---------------------- | ----------------------------- | ----------------------- | ------------------------------------------ |
| focus-visible source props | derived from context | modality/focus updates | focus-visible derivation path | shows or hides the ring | may be shared with a focus-visible utility |

## 8. Registration and Cleanup Contract

- No per-instance registration lifecycle exists unless the adapter uses shared modality listeners.
- Any shared listener strategy must clean up per-instance subscriptions on unmount.

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

| Dependency                               | Required?   | Dependency type | Why it must exist first                                                    | Notes                                     |
| ---------------------------------------- | ----------- | --------------- | -------------------------------------------------------------------------- | ----------------------------------------- |
| modality or focus-visible infrastructure | recommended | shared helper   | Focus-ring behavior depends on a consistent source of focus-visible state. | May be shared with other focus utilities. |

## 17. Recommended Implementation Sequence

1. Render the focus-ring host structure.
2. Connect focus-visible or modality derivation.
3. Apply ring visibility attrs/classes.
4. Verify that styling-state behavior matches the documented oracles.

## 18. Anti-Patterns

- Do not install per-instance global listeners when a shared modality source exists.
- Do not suppress genuine focus-visible semantics in favor of purely stylistic heuristics.

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

Leptos can derive focus-visible attrs reactively.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn FocusRing(children: Children) -> impl IntoView {
    let api = focus_ring::Api::new(focus_ring::Context { within: false, focus_visible: false }, focus_ring::Props::default());
    view! { <span {..api.root_attrs()}>{children()}</span> }
}
```

## 25. Reference Implementation Skeleton

```rust,no_check
let machine = derive_focus_ring_state(props);
let root_ref = create_optional_root_ref();
let modality = resolve_focus_visible_source();

render_focus_host(root_ref, derive_focus_ring_attrs(machine, modality));
attach_client_only_focus_visible_bookkeeping_if_needed(root_ref, modality);

on_cleanup(|| release_instance_local_focus_ring_bookkeeping());
```

## 26. Adapter Invariants

- Modality tracking must not suppress genuine focus semantics or hide the ring for keyboard users who need it.
- Text-input and keyboard modality exceptions must be documented explicitly.
- Ring visibility must follow normalized focus-visible behavior rather than raw focus alone.

## 27. Accessibility and SSR Notes

No semantic role changes; only styling state is reflected.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop and part parity.

Intentional deviations: none.

## 29. Test Scenarios

- direct focus-visible state
- focus-within mode
- text input override

## 30. Test Oracle Notes

| Behavior                               | Preferred oracle type | Notes                                                                      |
| -------------------------------------- | --------------------- | -------------------------------------------------------------------------- |
| structural rendering and part presence | rendered structure    | Verify the documented part mapping rather than incidental wrapper details. |
| accessibility and state attrs          | DOM attrs             | Assert the normalized attrs emitted by the adapter-owned node.             |

## 31. Implementation Checklist

- [ ] Focus-visible state maps to the documented attrs or classes.
- [ ] Listener or modality bookkeeping does not churn across rerenders.
- [ ] DOM-attr or styling-state test oracles are covered.
