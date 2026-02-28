---
adapter: dioxus
component: landmark
category: utility
source: components/utility/landmark.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Landmark — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Landmark`](../../components/utility/landmark.md) utility to Dioxus 0.7.x.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct LandmarkProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(default = Role::Region)]
    pub role: Role,
    #[props(optional)]
    pub labelledby_id: Option<String>,
    #[props(optional)]
    pub locale: Option<Locale>,
    pub children: Element,
}

#[component]
pub fn Landmark(props: LandmarkProps) -> Element
```

## 3. Mapping to Core Component Contract

- Props parity: full parity.
- Structure parity: single root node with semantic element choice or fallback role.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target                      | Ownership     | Attr source        | Notes                                                 |
| --------------------- | --------- | --------------------------------------------- | ------------- | ------------------ | ----------------------------------------------------- |
| `Root`                | required  | semantic landmark element or fallback `<div>` | adapter-owned | `api.root_attrs()` | Element selection depends on role and fallback rules. |

## 5. Attr Merge and Ownership Rules

| Target node   | Core attrs                       | Adapter-owned attrs                                     | Consumer attrs      | Merge order                                                                           | Ownership notes    |
| ------------- | -------------------------------- | ------------------------------------------------------- | ------------------- | ------------------------------------------------------------------------------------- | ------------------ |
| landmark root | landmark attrs from the core API | fallback role attrs when no native landmark tag is used | consumer root attrs | core semantic element choice and labeling attrs win; `class`/`style` merge additively | adapter-owned root |

## 6. Composition / Context Contract

No context contract.

## 7. Prop Sync and Event Mapping

Landmark semantics are render-time concerns.

| Adapter prop                | Mode                      | Sync trigger     | Machine event / update path | Visible effect                                   | Notes                       |
| --------------------------- | ------------------------- | ---------------- | --------------------------- | ------------------------------------------------ | --------------------------- |
| landmark type / label props | non-reactive adapter prop | render time only | root attr derivation        | determines semantic element or role and labeling | no post-mount sync expected |

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

Dioxus chooses the root element dynamically according to the role mapping. When the requested landmark has no native semantic element, the adapter should render an explicit `<div role="...">` fallback rather than a vague generic wrapper with implied semantics.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct LandmarkSketchProps {
    pub children: Element,
}

#[component]
pub fn Landmark(props: LandmarkSketchProps) -> Element {
    let api = landmark::Api::new(&landmark::Props::default());
    rsx! { section { ..api.root_attrs(), {props.children} } }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton beyond the canonical sketch is required for this utility.

## 26. Adapter Invariants

- Semantic landmark elements should be preferred over generic nodes with roles whenever the target landmark has a native element.
- When no native landmark element applies, the fallback node must be an explicit `<div role="...">` that preserves the requested landmark semantics.
- `aria-labelledby` precedence over `aria-label` must remain explicit and stable.

## 27. Accessibility and SSR Notes

Must preserve `aria-labelledby` precedence and semantic landmark behavior.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- root mapping
- semantic element vs fallback role
- labeling precedence
- unsupported or native-less landmark role falls back to explicit `<div role="...">`

## 30. Test Oracle Notes

| Behavior                               | Preferred oracle type | Notes                                                                                          |
| -------------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------- |
| structural rendering and part presence | rendered structure    | Verify the documented part mapping rather than incidental wrapper details.                     |
| accessibility and state attrs          | DOM attrs             | Assert the normalized attrs emitted by the adapter-owned node.                                 |
| native-less fallback rendering         | rendered structure    | Assert the adapter renders an explicit `<div>` fallback instead of an implicit wrapper choice. |

## 31. Implementation Checklist

- [ ] All documented parts and structural nodes are rendered with the correct ownership model.
- [ ] Attr merge precedence matches the documented contract.
- [ ] Prop sync and event normalization follow the documented machine paths.
- [ ] Cleanup releases every resource owned by the component instance.
- [ ] Semantic-element preference and explicit `<div role="...">` fallback behavior are verified.
- [ ] SSR/client boundary behavior matches the documented structure and test oracles.
