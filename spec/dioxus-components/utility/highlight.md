---
adapter: dioxus
component: highlight
category: utility
source: components/utility/highlight.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Highlight — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Highlight`](../../components/utility/highlight.md) utility to Dioxus 0.7.x.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct HighlightProps {
    pub query: Vec<String>,
    pub text: String,
    #[props(optional, default = true)]
    pub ignore_case: bool,
    #[props(default = MatchStrategy::Contains)]
    pub match_strategy: MatchStrategy,
    #[props(optional)]
    pub locale: Option<Locale>,
}

#[component]
pub fn Highlight(props: HighlightProps) -> Element
```

## 3. Mapping to Core Component Contract

- Props parity: full parity.
- Structure parity: root plus repeated chunk wrappers.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership                               | Attr source                                                     | Notes                                             |
| --------------------- | --------- | ------------------------ | --------------------------------------- | --------------------------------------------------------------- | ------------------------------------------------- |
| `Root`                | required  | wrapper `<span>`         | adapter-owned                           | adapter-owned root attrs                                        | Core anatomy requires a root wrapper.             |
| highlight chunk       | repeated  | `<mark>` or `<span>`     | adapter-owned repeated structural nodes | adapter-owned structural attrs from `highlight_chunks()` output | Repeated structural node, not a core enum `Part`. |

## 5. Attr Merge and Ownership Rules

| Target node                      | Core attrs                                     | Adapter-owned attrs                                                                                  | Consumer attrs                                         | Merge order                                                                                                     | Ownership notes                                   |
| -------------------------------- | ---------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| root text container if present   | root attrs when the utility renders a wrapper  | chunk-boundary attrs plus optional mixed-direction `dir="auto"` repair                               | consumer wrapper attrs if exposed                      | core structural attrs win; documented directionality attrs and `class`/`style` merge additively                 | adapter-owned wrapper when present                |
| matched/unmatched chunk wrappers | chunk attrs derived from the core match result | adapter `data-*` part markers and optional `unicode-bidi: isolate` styles when bidi repair is needed | consumer decoration only if chunk rendering is exposed | match-state attrs and adapter-owned bidi repair win over conflicting decoration that would break text isolation | chunk wrappers are adapter-owned structural nodes |

## 6. Composition / Context Contract

No context contract.

## 7. Prop Sync and Event Mapping

Highlighting is derived from input text and query changes. If the adapter accepts reactive text or query props, updates are immediate and recomputed from props rather than through a long-lived interaction state machine.

| Adapter prop | Mode       | Sync trigger            | Machine event / update path | Visible effect                      | Notes                  |
| ------------ | ---------- | ----------------------- | --------------------------- | ----------------------------------- | ---------------------- |
| `text`       | controlled | prop change after mount | recompute highlight chunks  | updates rendered chunk wrappers     | effect-based recompute |
| `query`      | controlled | prop change after mount | recompute highlight chunks  | updates matched vs unmatched output | effect-based recompute |

## 8. Registration and Cleanup Contract

- No registration lifecycle exists beyond recomputing chunk wrappers on render.
- If chunk identity is keyed for animation, those keys are local render details and do not persist outside the component lifetime.

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

Dioxus can render repeated chunks directly from iterator output. For mixed-direction content, the adapter should set `dir="auto"` on the root wrapper and may apply `unicode-bidi: isolate` on repeated `<mark>` and `<span>` chunk nodes when the highlighted text would otherwise bleed bidi ordering into surrounding content.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Highlight() -> Element {
    let props = highlight::Props::default();
    rsx! {
        span { "data-ars-scope": "highlight", "data-ars-part": "root",
            {highlight::highlight_chunks(&props).into_iter().map(|chunk| {
                if chunk.highlighted {
                    rsx! { mark { "data-ars-part": "highlight-chunk", "{chunk.text}" } }
                } else {
                    rsx! { span { "data-ars-part": "highlight-chunk", "{chunk.text}" } }
                }
            })}
        }
    }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton beyond the canonical sketch is required for this utility.

## 26. Adapter Invariants

- Chunk wrappers must remain explicit structural nodes so matched and unmatched content do not lose part identity.
- Match consolidation rules must remain stable across rerenders so highlighting does not flicker or split unexpectedly.
- Locale-sensitive matching behavior must remain explicit wherever the core contract depends on it.
- Text isolation or bidi-sensitive output rules must be preserved wherever highlighted content can span mixed-direction text.

## 27. Accessibility and SSR Notes

Must preserve `<mark>` semantics and repeated chunk structure.
For fuzzy matching that would otherwise produce excessive tiny wrappers, the adapter should consolidate adjacent chunks when that reduces screen-reader verbosity without changing the user-visible matched ranges.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core rendering parity.

Intentional deviations: none.

## 29. Test Scenarios

- root mapping
- repeated chunk mapping
- highlighted vs non-highlighted chunk rendering
- mixed-direction highlighted content keeps expected root direction and chunk isolation
- fuzzy highlighting with many small chunks documents the chosen consolidation behavior

## 30. Test Oracle Notes

| Behavior                               | Preferred oracle type | Notes                                                                              |
| -------------------------------------- | --------------------- | ---------------------------------------------------------------------------------- |
| structural rendering and part presence | rendered structure    | Verify the documented part mapping rather than incidental wrapper details.         |
| accessibility and state attrs          | DOM attrs             | Assert the normalized attrs emitted by the adapter-owned node.                     |
| bidi handling                          | DOM attrs             | Assert root `dir="auto"` and any documented chunk-level isolation attrs or styles. |
| chunk consolidation policy             | rendered structure    | Verify repeated chunk output matches the documented fuzzy-match verbosity policy.  |

## 31. Implementation Checklist

- [ ] All documented parts and structural nodes are rendered with the correct ownership model.
- [ ] Attr merge precedence matches the documented contract.
- [ ] Prop sync and event normalization follow the documented machine paths.
- [ ] Cleanup releases every resource owned by the component instance.
- [ ] Mixed-direction root and chunk bidi handling is documented and verified when needed.
- [ ] Fuzzy-match chunk consolidation policy is explicit where many tiny wrappers would become noisy.
- [ ] SSR/client boundary behavior matches the documented structure and test oracles.
