---
adapter: dioxus
component: aspect-ratio
category: layout
source: components/layout/aspect-ratio.md
source_foundation: foundation/09-adapter-dioxus.md
---

# AspectRatio — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`AspectRatio`](../../components/layout/aspect-ratio.md) contract onto a Dioxus `0.7.x` component. The adapter preserves the single `Root` part, ratio-driven intrinsic sizing, and passive non-interactive behavior while defining the Dioxus-facing prop and child contract.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct AspectRatioProps {
    #[props(optional)]
    pub id: Option<String>,
    pub ratio: f64,
    pub children: Element,
}

#[component]
pub fn AspectRatio(props: AspectRatioProps) -> Element
```

The adapter exposes a single structural wrapper and renders children inside that wrapper without adding additional interactive behavior.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props` surface.
- Part parity: full parity with the core `Root` part.
- Adapter additions: explicit Dioxus child contract and ratio validation policy.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source        | Notes                                                                            |
| --------------------- | --------- | ------------------------ | ------------- | ------------------ | -------------------------------------------------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()` | Children render inside the ratio box; consumers style the child to fill the box. |

## 5. Attr Merge and Ownership Rules

| Target node | Core attrs                                                                      | Adapter-owned attrs                                 | Consumer attrs                            | Merge order                                                                                                                        | Ownership notes            |
| ----------- | ------------------------------------------------------------------------------- | --------------------------------------------------- | ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| `Root`      | `api.root_attrs()` including `data-ars-*` attrs and ratio-derived inline styles | no additional required attrs beyond child insertion | root decoration attrs exposed by wrappers | core `data-ars-*` attrs and ratio styles win; additive `class`/`style` decoration may extend but not remove required layout styles | root remains adapter-owned |

- Consumer decoration must not remove `position: relative`, `width: 100%`, or computed `padding-top`.
- The adapter does not add ARIA attrs because the core contract is purely structural.

## 6. Composition / Context Contract

`AspectRatio` is standalone. It provides no context and consumes no required context. Optional environment or direction context may influence higher-level styling, but not the ratio math.

## 7. Prop Sync and Event Mapping

| Adapter prop | Mode       | Sync trigger                | Machine event / update path | Visible effect                   | Notes                            |
| ------------ | ---------- | --------------------------- | --------------------------- | -------------------------------- | -------------------------------- |
| `ratio`      | controlled | prop change during rerender | direct attr recomputation   | updates `padding-top` percentage | no machine or event layer exists |

No adapter events, callbacks, or handler composition are required.

## 8. Registration and Cleanup Contract

No registration, observers, timers, or cleanup paths are required.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule        | Notes                                                         |
| ------------------ | ------------- | ------------- | ---------------------------------- | ----------------------- | ------------------------------------------------------------- |
| `Root`             | no            | adapter-owned | always structural, handle optional | no composition required | The component is passive and does not need imperative access. |

## 10. State Machine Boundary Rules

- machine-owned state: not applicable.
- adapter-local derived bookkeeping: computed padding percentage only.
- forbidden local mirrors: do not mirror `ratio` into stale local state.
- allowed snapshot-read contexts: render-time style derivation only.

## 11. Callback Payload Contract

| Callback | Payload source | Payload shape | Timing         | Cancelable? | Notes                               |
| -------- | -------------- | ------------- | -------------- | ----------- | ----------------------------------- |
| none     | none           | none          | not applicable | no          | `AspectRatio` exposes no callbacks. |

## 12. Failure and Degradation Rules

| Condition                                           | Policy             | Notes                                                                             |
| --------------------------------------------------- | ------------------ | --------------------------------------------------------------------------------- |
| `ratio <= 0.0` or non-finite                        | fail fast          | The core contract requires a positive finite ratio.                               |
| child content does not opt into fill styling        | degrade gracefully | The wrapper still reserves space; sizing of inner content remains consumer-owned. |
| host-only layout information unavailable during SSR | no-op              | The ratio box is encoded entirely as static attrs and styles.                     |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                         | Notes                        |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | --------------------------------------------------------------- | ---------------------------- |
| root wrapper                     | instance-derived | not applicable      | not applicable                           | wrapper identity must stay stable across rerender and hydration | Single structural node only. |

## 14. SSR and Client Boundary Rules

- SSR renders the same single wrapper and computed ratio styles as the client.
- No client-only listeners or refs are required.
- Platform adapters must not insert or remove wrapper nodes around the documented root.

## 15. Performance Constraints

- Compute the padding percentage directly from the latest prop instead of storing redundant local state.
- Do not allocate extra wrapper nodes around the documented `Root`.

## 16. Implementation Dependencies

| Dependency        | Required?   | Dependency type | Why it must exist first                                              | Notes                           |
| ----------------- | ----------- | --------------- | -------------------------------------------------------------------- | ------------------------------- |
| attr merge helper | recommended | shared helper   | Keeps optional wrapper decoration from dropping required root attrs. | A generic helper is sufficient. |

## 17. Recommended Implementation Sequence

1. Validate `ratio`.
2. Build the core attr map and render the single `Root`.
3. Merge any wrapper-level decoration without removing required ratio styles.
4. Render children inside the root.

## 18. Anti-Patterns

- Do not add interactive semantics or callbacks to this passive layout primitive.
- Do not replace the ratio-box technique with measurement-driven host logic.
- Do not remove required ratio styles when merging consumer decoration.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the adapter always renders exactly one structural root.
- Consumers may assume the ratio is enforced with static styles that work during SSR and rerender.
- Consumers must not assume the adapter will auto-style child content to fill the box.

## 20. Platform Support Matrix

| Capability / behavior          | Web          | Desktop      | Mobile       | SSR          | Notes                              |
| ------------------------------ | ------------ | ------------ | ------------ | ------------ | ---------------------------------- |
| ratio box structure and styles | full support | full support | full support | full support | No browser-only APIs are required. |

## 21. Debug Diagnostics and Production Policy

| Condition             | Debug build behavior | Production behavior | Notes                                          |
| --------------------- | -------------------- | ------------------- | ---------------------------------------------- |
| invalid `ratio` value | fail fast            | fail fast           | Invalid ratio breaks the core sizing contract. |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required?   | Responsibility                                                   | Reused by                          | Notes                                       |
| ----------------- | ----------- | ---------------------------------------------------------------- | ---------------------------------- | ------------------------------------------- |
| attr merge helper | recommended | Preserves required root attrs while allowing wrapper decoration. | `center`, `grid`, `stack`, `frame` | No dedicated aspect-ratio helper is needed. |

## 23. Framework-Specific Behavior

Dioxus can derive the root attr map during render with no effect or host handle. `Element` children render directly inside the ratio wrapper.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct AspectRatioProps {
    pub ratio: f64,
    pub children: Element,
}

#[component]
pub fn AspectRatio(props: AspectRatioProps) -> Element {
    let api = aspect_ratio::Api::new(aspect_ratio::Props { ratio: props.ratio, ..Default::default() });
    let root_attrs = api.root_attrs();
    rsx! {
        div {
            ..root_attrs,
            {props.children}
        }
    }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton beyond the canonical sketch is required for this stateless component.

## 26. Adapter Invariants

- The adapter always renders exactly one `Root`.
- Required ratio styles and `data-ars-*` attrs remain present after attr merging.
- The adapter never introduces measurement-driven or interactive behavior.

## 27. Accessibility and SSR Notes

- `AspectRatio` is accessibility-neutral; the child content owns all meaningful semantics.
- Because the ratio is encoded in static styles, SSR and rerender are expected to preserve the same structure.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core `AspectRatio` contract.
- Intentional deviations: none.
- Traceability note: the adapter promotes the agnostic ratio validation and passive-root expectations into explicit Dioxus render rules.

## 29. Test Scenarios

- Render with a valid ratio and verify the root styles encode the expected percentage.
- Update `ratio` and verify the computed `padding-top` changes without replacing the root.
- Pass invalid ratio values and verify failure behavior.

## 30. Test Oracle Notes

- Ratio math: prefer `DOM attrs`.
- Root stability across updates: prefer `rendered structure`.
- Invalid ratio handling: prefer debug assertion or `fail fast` behavior.

## 31. Implementation Checklist

- [ ] Validate `ratio` as positive and finite.
- [ ] Render exactly one `Root`.
- [ ] Preserve required ratio styles and `data-ars-*` attrs during attr merge.
- [ ] Keep the component passive and structure-stable across platforms.
