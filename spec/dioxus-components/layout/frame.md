---
adapter: dioxus
component: frame
category: layout
source: components/layout/frame.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Frame — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Frame`](../../components/layout/frame.md) contract onto a Dioxus `0.7.x` component. The adapter preserves the core `Root` and `Iframe` parts, sandbox and permissions attrs, and optional aspect-ratio boxing while defining the Dioxus-facing API and validation rules.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct FrameProps {
    #[props(optional)]
    pub id: Option<String>,
    pub src: String,
    pub title: String,
    #[props(optional)]
    pub sandbox: Option<String>,
    #[props(optional)]
    pub allow: Option<String>,
    pub loading: LoadingStrategy,
    #[props(optional)]
    pub aspect_ratio: Option<f64>,
    #[props(optional)]
    pub width: Option<String>,
    #[props(optional)]
    pub height: Option<String>,
}

#[component]
pub fn Frame(props: FrameProps) -> Element
```

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props`.
- Part parity: full parity with the core `Root` and `Iframe` parts.
- Adapter additions: explicit Dioxus validation rules for required `src` and `title`, plus ratio-wrapper rendering behavior.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source          | Notes                                                               |
| --------------------- | --------- | ------------------------ | ------------- | -------------------- | ------------------------------------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`   | The root may become a ratio wrapper when `aspect_ratio` is present. |
| `Iframe`              | required  | `<iframe>`               | adapter-owned | `api.iframe_attrs()` | Always rendered inside `Root`.                                      |

## 5. Attr Merge and Ownership Rules

| Target node | Core attrs                                                                                   | Adapter-owned attrs          | Consumer attrs                                           | Merge order                                           | Ownership notes              |
| ----------- | -------------------------------------------------------------------------------------------- | ---------------------------- | -------------------------------------------------------- | ----------------------------------------------------- | ---------------------------- |
| `Root`      | `api.root_attrs()` including optional ratio-box styles                                       | no additional required attrs | wrapper decoration attrs when exposed                    | core `data-ars-*` attrs and required ratio styles win | root remains adapter-owned   |
| `Iframe`    | `api.iframe_attrs()` including `src`, `title`, `sandbox`, `allow`, `loading`, and size attrs | no additional required attrs | no direct consumer ownership in the core adapter surface | required iframe attrs always win                      | iframe remains adapter-owned |

## 6. Composition / Context Contract

`Frame` is standalone. It provides no context and consumes no required context.

## 7. Prop Sync and Event Mapping

| Adapter prop | Mode       | Sync trigger            | Machine event / update path | Visible effect                               | Notes                            |
| ------------ | ---------- | ----------------------- | --------------------------- | -------------------------------------------- | -------------------------------- |
| all props    | controlled | rerender with new props | direct attr recomputation   | updates iframe attrs or ratio wrapper styles | no machine or event layer exists |

## 8. Registration and Cleanup Contract

No adapter-owned registration, observers, or timers are required. The host platform owns iframe navigation and loading lifecycle after attrs are applied.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule        | Notes                                                            |
| ------------------ | ------------- | ------------- | ---------------------------------- | ----------------------- | ---------------------------------------------------------------- |
| `Root`             | no            | adapter-owned | always structural, handle optional | no composition required | Structural wrapper only.                                         |
| `Iframe`           | no            | adapter-owned | always structural, handle optional | no composition required | Imperative access is not part of the documented adapter surface. |

## 10. State Machine Boundary Rules

- machine-owned state: not applicable.
- adapter-local derived bookkeeping: optional ratio-box calculation only.
- forbidden local mirrors: do not mirror iframe props into stale local state.
- allowed snapshot-read contexts: render-time attr derivation only.

## 11. Callback Payload Contract

| Callback | Payload source | Payload shape | Timing         | Cancelable? | Notes                                                 |
| -------- | -------------- | ------------- | -------------- | ----------- | ----------------------------------------------------- |
| none     | none           | none          | not applicable | no          | `Frame` exposes no callbacks in this adapter surface. |

## 12. Failure and Degradation Rules

| Condition                         | Policy             | Notes                                                         |
| --------------------------------- | ------------------ | ------------------------------------------------------------- |
| empty `src` or `title`            | fail fast          | The core contract requires both.                              |
| invalid `aspect_ratio` value      | fail fast          | Ratio boxing requires a positive finite ratio.                |
| host blocks the embedded resource | degrade gracefully | The adapter still renders the documented structure and attrs. |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                          | Notes                                     |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | ---------------------------------------------------------------- | ----------------------------------------- |
| root and iframe pair             | instance-derived | not applicable      | not applicable                           | root and iframe order must remain stable across rerender and SSR | The pair belongs to one `Frame` instance. |

## 14. SSR and Client Boundary Rules

- SSR renders both `Root` and `Iframe` with the same attrs expected on the client.
- No client-only listeners or effects are required.
- The optional ratio wrapper must not appear only on one side of SSR or client render.

## 15. Performance Constraints

- Derive ratio wrapper styles from props directly without local mirrors.
- Avoid extra wrappers beyond the documented `Root` and `Iframe`.

## 16. Implementation Dependencies

| Dependency        | Required?   | Dependency type | Why it must exist first                                               | Notes                |
| ----------------- | ----------- | --------------- | --------------------------------------------------------------------- | -------------------- |
| attr merge helper | recommended | shared helper   | Preserves required root attrs if wrapper-level decoration is exposed. | Generic helper only. |

## 17. Recommended Implementation Sequence

1. Validate `src`, `title`, and any optional ratio.
2. Build root and iframe attr maps from the core API.
3. Render the root wrapper and inner iframe.
4. Merge any wrapper decoration without dropping required attrs.

## 18. Anti-Patterns

- Do not let consumer decoration override required iframe attrs like `src` or `title`.
- Do not omit the root wrapper when `aspect_ratio` is present.
- Do not replace static ratio boxing with measurement-driven host logic.

## 19. Consumer Expectations and Guarantees

- Consumers may assume `Frame` always renders a `Root` containing one `Iframe`.
- Consumers may assume required sandbox, title, loading, and size attrs reflect the core props exactly.
- Consumers must not assume the adapter can bypass sandbox or cross-origin host restrictions.

## 20. Platform Support Matrix

| Capability / behavior                   | Web          | Desktop       | Mobile        | SSR          | Notes                                                                                                    |
| --------------------------------------- | ------------ | ------------- | ------------- | ------------ | -------------------------------------------------------------------------------------------------------- |
| iframe attrs and optional ratio wrapper | full support | fallback path | fallback path | full support | Non-web targets may need host-native embed fallbacks, but the documented structure remains the contract. |

## 21. Debug Diagnostics and Production Policy

| Condition                | Debug build behavior | Production behavior | Notes                                   |
| ------------------------ | -------------------- | ------------------- | --------------------------------------- |
| missing `src` or `title` | fail fast            | fail fast           | Required for a valid embed contract.    |
| invalid `aspect_ratio`   | fail fast            | fail fast           | Ratio wrapper cannot be derived safely. |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required?   | Responsibility                                                    | Reused by                                 | Notes                                        |
| ----------------- | ----------- | ----------------------------------------------------------------- | ----------------------------------------- | -------------------------------------------- |
| attr merge helper | recommended | Preserves required root attrs during optional wrapper decoration. | `aspect-ratio`, `center`, `grid`, `stack` | No frame-specific shared helper is required. |

## 23. Framework-Specific Behavior

Dioxus web renders the iframe structure directly. Non-web targets may need a host-native embed fallback, but the adapter still owns the documented root/iframe contract and validation rules.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct FrameProps {
    pub src: String,
    pub title: String,
}

#[component]
pub fn Frame(props: FrameProps) -> Element {
    let api = frame::Api::new(frame::Props { src: props.src, title: props.title, ..Default::default() });
    let root_attrs = api.root_attrs();
    let iframe_attrs = api.iframe_attrs();
    rsx! {
        div {
            ..root_attrs,
            iframe { ..iframe_attrs }
        }
    }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton beyond the canonical sketch is required for this stateless component.

## 26. Adapter Invariants

- The adapter always renders `Root` followed by `Iframe`.
- Required iframe attrs survive any wrapper-level attr merge.
- Ratio-box rendering remains static and structure-stable across supported targets.

## 27. Accessibility and SSR Notes

- `title` is required because the iframe's accessible name depends on it.
- SSR and client render must preserve the same root/iframe structure and ratio-wrapper choice.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core `Frame` contract on web; documented host fallbacks elsewhere.
- Intentional deviations: non-web targets may need host-native embed fallbacks.
- Traceability note: adapter validation and ratio-wrapper ownership are promoted into explicit Dioxus rules.

## 29. Test Scenarios

- Render a basic iframe and verify `src`, `title`, `loading`, and sizing attrs.
- Render with `aspect_ratio` and verify ratio wrapper styles plus iframe structure.
- Pass invalid ratio or missing required props and verify failure behavior.
- Verify non-web fallback policy does not change validation rules.

## 30. Test Oracle Notes

- Root and iframe attrs: prefer `DOM attrs`.
- Structural wrapper choice: prefer `rendered structure`.
- Invalid props: prefer `fail fast` behavior.
- Host fallback policy: prefer `rendered structure`.

## 31. Implementation Checklist

- [ ] Validate `src`, `title`, and `aspect_ratio`.
- [ ] Render exactly one `Root` and one `Iframe`.
- [ ] Preserve required iframe attrs during attr merge.
- [ ] Keep ratio-wrapper rendering static and structure-stable across supported targets.
