---
adapter: dioxus
component: z-index-allocator
category: utility
source: components/utility/z-index-allocator.md
source_foundation: foundation/09-adapter-dioxus.md
---

# ZIndexAllocator — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ZIndexAllocator`](../../components/utility/z-index-allocator.md) context contract to Dioxus 0.7.x.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ZIndexAllocatorProviderProps {
    pub children: Element,
}

#[component]
pub fn ZIndexAllocatorProvider(props: ZIndexAllocatorProviderProps) -> Element
```

## 3. Mapping to Core Component Contract

- Context parity: full parity with allocator creation, allocation, release, and reset behavior.
- Structure parity: provider-only, no rendered DOM of its own.

## 4. Part Mapping

| Core part / structure       | Required?     | Adapter rendering target | Ownership      | Attr source | Notes                                    |
| --------------------------- | ------------- | ------------------------ | -------------- | ----------- | ---------------------------------------- |
| allocator provider boundary | provider-only | no DOM output            | provider-only  | none        | Publishes allocator context only.        |
| children subtree            | required      | consumer children        | consumer-owned | none        | Descendants may allocate z-index values. |

## 5. Attr Merge and Ownership Rules

| Target node                 | Core attrs             | Adapter-owned attrs               | Consumer attrs                                                | Merge order    | Ownership notes       |
| --------------------------- | ---------------------- | --------------------------------- | ------------------------------------------------------------- | -------------- | --------------------- |
| allocator provider boundary | no rendered part attrs | provider-owned allocation context | none on a DOM node because there is no rendered provider node | not applicable | provider-only utility |

## 6. Composition / Context Contract

Publish allocator context with `use_context_provider`. Optional consumers use `try_use_context::<z_index_allocator::Context>()`.

## 7. Prop Sync and Event Mapping

Allocator behavior is mostly provider-internal. If allocator options are configurable, they update provider context rather than DOM nodes.

| Adapter prop           | Mode       | Sync trigger            | Machine event / update path    | Visible effect                               | Notes                                      |
| ---------------------- | ---------- | ----------------------- | ------------------------------ | -------------------------------------------- | ------------------------------------------ |
| allocator base/options | controlled | prop change after mount | update allocator context state | descendants allocate from the updated policy | effect-based context update when supported |

## 8. Registration and Cleanup Contract

- Descendant claims register with the allocator provider and must release on cleanup.
- Allocation order must remain deterministic.
- Provider cleanup releases all outstanding claims.

| Registered entity        | Registration trigger          | Identity key      | Cleanup trigger                      | Cleanup action                                      | Notes                           |
| ------------------------ | ----------------------------- | ----------------- | ------------------------------------ | --------------------------------------------------- | ------------------------------- |
| z-index claim            | descendant allocation request | claim instance    | claimant cleanup or explicit release | release claim back to allocator                     | no leaked stacking reservations |
| allocator provider state | provider mount                | provider instance | provider cleanup                     | clear outstanding claims and allocation bookkeeping | provider-owned lifecycle only   |

## 9. Ref and Node Contract

| Target part / node          | Ref required? | Ref owner      | Node availability  | Composition rule | Notes                                         |
| --------------------------- | ------------- | -------------- | ------------------ | ---------------- | --------------------------------------------- |
| allocator provider boundary | no            | not applicable | server-safe absent | not applicable   | Provider-only utility with no owned DOM node. |

## 10. State Machine Boundary Rules

- machine-owned state: allocation order, current outstanding claims, and any reset behavior defined by the allocator context.
- adapter-local derived bookkeeping: provider-scoped claim bookkeeping handles only.
- forbidden local mirrors: do not keep separate unsynchronized z-index counters in claimants.
- allowed snapshot-read contexts: provider updates, claimant allocation/release hooks, and cleanup.

## 11. Callback Payload Contract

| Callback           | Payload source | Payload shape | Timing         | Cancelable? | Notes                                                                  |
| ------------------ | -------------- | ------------- | -------------- | ----------- | ---------------------------------------------------------------------- |
| no public callback | none           | none          | not applicable | no          | Allocation is observed through returned context values, not callbacks. |

## 12. Failure and Degradation Rules

| Condition                                                   | Policy                                  | Notes                                                                                                  |
| ----------------------------------------------------------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| claimant releases an unknown or already-released allocation | ignore in core; adapters may debug-warn | Core release is idempotent; adapter diagnostics may surface misuse when provider context is available. |
| provider absent where a claimant expects allocation context | fail fast                               | Claimants require the allocator contract to allocate correctly.                                        |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                               | Notes                                              |
| -------------------------------- | --------------- | ------------------- | ---------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------- |
| allocation claims                | composite       | yes                 | not applicable                           | provider lifetime must remain stable; claims are client/runtime-owned | Identity is claim instance plus provider instance. |

## 14. SSR and Client Boundary Rules

- The provider may render no DOM while still publishing allocator context.
- Claim allocation/release may occur in SSR-safe logic only if the contract explicitly supports it; otherwise allocation is effectively client/runtime-owned.
- Provider lifetime, not DOM structure, is the SSR boundary.

## 15. Performance Constraints

- Allocation and release should update provider state incrementally.
- Claim cleanup must avoid scanning unrelated claims when a direct claim identity is available.
- Do not recreate allocator context unnecessarily on unrelated rerenders.

## 16. Implementation Dependencies

| Dependency                   | Required?   | Dependency type     | Why it must exist first                                                       | Notes                                                                  |
| ---------------------------- | ----------- | ------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| overlay or layered consumers | recommended | downstream consumer | The allocator exists to support layered utilities that need ordered stacking. | Dependency is about implementation order in the broader design system. |

## 17. Recommended Implementation Sequence

1. Establish the provider-owned allocation context.
2. Implement claim allocation and explicit release behavior.
3. Wire claimant cleanup to automatic release.
4. Verify provider cleanup resets outstanding claims and test oracles.

## 18. Anti-Patterns

- Do not leak allocation claims across claimant cleanup.
- Do not recreate allocator context on unrelated rerenders.

## 19. Consumer Expectations and Guarantees

- Consumers may assume allocation claims are unique per provider lifetime and released on documented cleanup paths.
- Consumers may assume provider context, not DOM structure, defines the allocator scope.
- Consumers must not assume released or unknown claims can be re-used without going back through the allocator contract.

## 20. Platform Support Matrix

| Capability / behavior        | Web          | Desktop      | Mobile       | SSR            | Notes                                                                |
| ---------------------------- | ------------ | ------------ | ------------ | -------------- | -------------------------------------------------------------------- |
| provider context publication | full support | full support | full support | SSR-safe empty | Allocation scope is context-only.                                    |
| claim allocation and release | full support | full support | full support | fallback path  | SSR-safe paths may skip runtime allocation until a live host exists. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                            | Debug build behavior | Production behavior | Notes                                                                               |
| -------------------------------------------------------------------- | -------------------- | ------------------- | ----------------------------------------------------------------------------------- |
| no component-specific diagnostics beyond documented failure policies | not applicable       | not applicable      | Use the `Failure and Degradation Rules` section as the full runtime policy surface. |

## 22. Shared Adapter Helper Notes

| Helper concept                     | Required?   | Responsibility                                                              | Reused by                                      | Notes                                                                    |
| ---------------------------------- | ----------- | --------------------------------------------------------------------------- | ---------------------------------------------- | ------------------------------------------------------------------------ |
| registry helper for repeated items | required    | Track allocation claims, release ordering, and unknown-release diagnostics. | `z-index-allocator`, layered overlay consumers | Provider lifetime defines the allocator scope.                           |
| platform capability helper         | recommended | Avoid assuming runtime-owned stacking behavior during SSR-safe paths.       | `z-index-allocator`, `ars-provider`            | Mainly relevant when allocation is deferred until a live runtime exists. |

## 23. Framework-Specific Behavior

Dioxus hook cleanup may be used by descendants to release values.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ZIndexAllocatorProviderSketchProps {
    pub children: Element,
}

#[component]
pub fn ZIndexAllocatorProvider(props: ZIndexAllocatorProviderSketchProps) -> Element {
    use_context_provider(|| z_index_allocator::Context::new());
    rsx! { {props.children} }
}
```

## 25. Reference Implementation Skeleton

```rust
let allocator = create_allocator_context(props);
let claims = create_claim_registry_helper();

publish_allocator_context(allocator, claims);
render_children_without_extra_structure();
validate_claim_and_release_order_through_helper(claims);

on_cleanup(|| claims.release_all_owned_claims());
```

## 26. Adapter Invariants

- This utility is provider-only and must not claim rendered parts where the core contract defines none.
- Allocation order must remain deterministic and monotonic wherever the core contract requires ordered stacking.
- Release and cleanup behavior must be explicit so allocations are not leaked across unmounts.

## 27. Accessibility and SSR Notes

No rendered semantics.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core context parity.

Intentional deviations: none.

## 29. Test Scenarios

- no rendered parts
- context publication
- allocation and release lifecycle

## 30. Test Oracle Notes

| Behavior                      | Preferred oracle type | Notes                                                                       |
| ----------------------------- | --------------------- | --------------------------------------------------------------------------- |
| claim publication and release | context registration  | Assert claim allocation is visible through allocator context.               |
| provider/claim cleanup        | cleanup side effects  | Verify claims are released and allocator state resets correctly on cleanup. |

## 31. Implementation Checklist

- [ ] Claims allocate and release according to the documented provider contract.
- [ ] Provider cleanup resets outstanding claim state.
- [ ] Context-registration and cleanup-side-effect test oracles are covered.
