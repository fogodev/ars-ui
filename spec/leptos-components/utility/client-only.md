---
adapter: leptos
component: client-only
category: utility
source: components/utility/client-only.md
source_foundation: foundation/08-adapter-leptos.md
---

# ClientOnly — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ClientOnly`](../../components/utility/client-only.md) contract to Leptos 0.8.x SSR and hydration behavior.

## 2. Public Adapter API

```rust
#[component]
pub fn ClientOnly(
    #[prop(optional)] fallback: Option<ChildrenFn>,
    children: ChildrenFn,
) -> impl IntoView
```

## 3. Mapping to Core Component Contract

- Props parity: `fallback` maps directly.
- Behavior parity: fallback or nothing during SSR and pre-mount client render; children after mount.

## 4. Part Mapping

| Core part / structure | Required?     | Adapter rendering target  | Ownership        | Attr source | Notes                                               |
| --------------------- | ------------- | ------------------------- | ---------------- | ----------- | --------------------------------------------------- |
| client-only boundary  | provider-only | no DOM output of its own  | adapter behavior | none        | Logical render gate only.                           |
| fallback subtree      | conditional   | consumer fallback content | consumer-owned   | none        | Rendered during SSR and before mount when provided. |
| children subtree      | conditional   | consumer children         | consumer-owned   | none        | Rendered after client mount.                        |

## 5. Attr Merge and Ownership Rules

| Target node                      | Core attrs                                                 | Adapter-owned attrs           | Consumer attrs                      | Merge order                                            | Ownership notes                                                |
| -------------------------------- | ---------------------------------------------------------- | ----------------------------- | ----------------------------------- | ------------------------------------------------------ | -------------------------------------------------------------- |
| fallback/client content boundary | no required rendered attrs when the utility is wrapperless | mount-gating bookkeeping only | consumer fallback or client content | no DOM attr merge beyond documented fallback structure | no phantom wrapper unless the adapter explicitly documents one |

## 6. Composition / Context Contract

No context contract.

## 7. Prop Sync and Event Mapping

Client-only gating is driven by mount state.

| Adapter prop           | Mode                        | Sync trigger                              | Machine event / update path | Visible effect                        | Notes                     |
| ---------------------- | --------------------------- | ----------------------------------------- | --------------------------- | ------------------------------------- | ------------------------- |
| fallback content       | non-reactive adapter prop   | server render and pre-mount client render | direct fallback render path | shows fallback before client mount    | must be hydration-safe    |
| mounted client content | uncontrolled internal state | client mount                              | client-only activation path | replaces fallback with client content | no server-side activation |

## 8. Registration and Cleanup Contract

- No registration lifecycle exists beyond client mount gating.
- Cleanup is ordinary component disposal once client content is active.

## 9. Ref and Node Contract

| Target part / node                   | Ref required?                                | Ref owner      | Node availability                                                                       | Composition rule            | Notes                                              |
| ------------------------------------ | -------------------------------------------- | -------------- | --------------------------------------------------------------------------------------- | --------------------------- | -------------------------------------------------- |
| fallback or mounted subtree boundary | no unless a wrapper exposes the subtree root | consumer-owned | server-safe absent for mounted content; always structural, handle optional for fallback | no implicit ref composition | This utility does not introduce a phantom wrapper. |

## 10. State Machine Boundary Rules

- machine-owned state: mounted/not-mounted render gating when modeled by the adapter utility.
- adapter-local derived bookkeeping: client-mounted flag only.
- forbidden local mirrors: do not keep parallel fallback-vs-client visibility state outside the documented mount gate.
- allowed snapshot-read contexts: render and mount effects only.

## 11. Callback Payload Contract

| Callback           | Payload source | Payload shape | Timing         | Cancelable? | Notes                                              |
| ------------------ | -------------- | ------------- | -------------- | ----------- | -------------------------------------------------- |
| no public callback | none           | none          | not applicable | no          | The utility is a render gate, not an event source. |

## 12. Failure and Degradation Rules

| Condition                                    | Policy             | Notes                                                            |
| -------------------------------------------- | ------------------ | ---------------------------------------------------------------- |
| client-only content requested during SSR     | degrade gracefully | Render fallback or nothing according to the documented contract. |
| missing fallback when SSR output is required | no-op              | Render nothing rather than inventing a wrapper.                  |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                          | Notes                                       |
| -------------------------------- | --------------- | ------------------- | ---------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------- |
| fallback subtree                 | not applicable  | not applicable      | not applicable                           | fallback structure must remain hydration-stable                  | There is no repeated registration identity. |
| mounted client subtree           | not applicable  | not applicable      | not applicable                           | mounted subtree must replace fallback in the documented position | Client subtree identity is consumer-owned.  |

## 14. SSR and Client Boundary Rules

- Fallback structure, when present, must be rendered during SSR and pre-mount client render.
- Mounted client content must wait until hydration/mount.
- The utility must not introduce a server/client structural mismatch by changing wrapper structure.

## 15. Performance Constraints

- Mount gating should be a single instance-local flag and must not trigger unnecessary subtree churn beyond the documented fallback-to-client swap.
- Do not allocate listeners or timers for this utility unless a wrapper explicitly requires them.

## 16. Implementation Dependencies

| Dependency                      | Required?   | Dependency type | Why it must exist first                           | Notes                                                    |
| ------------------------------- | ----------- | --------------- | ------------------------------------------------- | -------------------------------------------------------- |
| none beyond client-mount gating | recommended | conceptual      | The utility is primarily a render-gating pattern. | State `not applicable` beyond the mount-gating contract. |

## 17. Recommended Implementation Sequence

1. Define the fallback/pre-mount render path.
2. Establish the client-mount gate.
3. Swap to client content after mount without changing wrapper structure.
4. Verify hydration-safe structure and test oracles.

## 18. Anti-Patterns

- Do not introduce phantom wrapper nodes.
- Do not create server/client structural mismatches during the fallback-to-client swap.

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

Leptos can use SSR cfg and a mounted signal while keeping the same initial markup for hydration.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn ClientOnly(
    #[prop(optional)] fallback: Option<ChildrenFn>,
    children: ChildrenFn,
) -> impl IntoView {
    let mounted = RwSignal::new(false);
    Effect::new(move |_| mounted.set(true));
    move || if mounted.get() { children() } else { fallback.as_ref().map(|f| f()).into_view() }
}
```

## 25. Reference Implementation Skeleton

```rust
let gate = create_client_mount_gate(props);

render_fallback_or_nothing_for_ssr(gate);
on_client_mount(|| gate.show_client_subtree());
ensure_hydration_sensitive_structure_matches_documented_contract();

on_cleanup(|| drop_client_only_mount_bookkeeping());
```

## 26. Adapter Invariants

- The adapter must not introduce a phantom wrapper node unless the core contract explicitly requires one.
- Fallback structure must remain hydration-safe and deterministic.
- Client-only content must not create a server or client structural mismatch during hydration.

## 27. Accessibility and SSR Notes

Fallback content should be accessible if it conveys loading or placeholder state.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core behavior parity.

Intentional deviations: none.

## 29. Test Scenarios

- no rendered wrapper node
- fallback before mount
- children after mount

## 30. Test Oracle Notes

| Behavior                    | Preferred oracle type | Notes                                                       |
| --------------------------- | --------------------- | ----------------------------------------------------------- |
| fallback vs mounted content | rendered structure    | Assert the documented structure before and after mount.     |
| SSR-safe swapping           | hydration structure   | Verify the fallback path does not cause hydration mismatch. |

## 31. Implementation Checklist

- [ ] Fallback and client content use the documented render gate.
- [ ] No phantom wrapper appears.
- [ ] Hydration structure remains stable.
