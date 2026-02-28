---
adapter: dioxus
component: stat
category: data-display
source: components/data-display/stat.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Stat — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Stat`](../../components/data-display/stat.md) contract onto a Dioxus 0.7.x component. The adapter keeps the component stateless while making semantic grouping, loading announcements, trend-label repair, and RTL icon handling explicit across Dioxus renderers.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct StatProps {
    #[props(optional)]
    pub id: Option<String>,
    pub label: String,
    pub value: String,
    #[props(optional)]
    pub change: Option<f64>,
    #[props(optional)]
    pub trend: Option<stat::Trend>,
    #[props(optional)]
    pub help_text: Option<String>,
    #[props(default = false)]
    pub loading: bool,
    #[props(optional)]
    pub locale: Option<Locale>,
    #[props(optional)]
    pub messages: Option<stat::Messages>,
}

#[component]
pub fn Stat(props: StatProps) -> Element
```

`value` is passed pre-formatted by the caller. The adapter owns the accessible grouping, derived trend labeling, and loading semantics.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core stat surface.
- Part parity: full parity for `Root`, `Label`, `Value`, optional `Change`, optional `TrendIndicator`, and optional `HelpText`.
- Traceability note: this spec promotes `role="group"`, loading live-region guidance, trend accessible labeling, and RTL icon repair from the agnostic spec.

## 4. Part Mapping

| Core part        | Required? | Adapter rendering target  | Ownership     | Attr source                   | Notes                                                                            |
| ---------------- | --------- | ------------------------- | ------------- | ----------------------------- | -------------------------------------------------------------------------------- |
| `Root`           | required  | `<div>` or `<dl>` wrapper | adapter-owned | `api.root_attrs()`            | The adapter may choose a description-list structure while preserving root attrs. |
| `Label`          | required  | `<dt>` or `<span>`        | adapter-owned | `api.label_attrs()`           | Associates with `Value`.                                                         |
| `Value`          | required  | `<dd>` or `<span>`        | adapter-owned | `api.value_attrs()`           | Carries the primary formatted text.                                              |
| `Change`         | optional  | `<span>`                  | adapter-owned | `api.change_attrs()`          | Must expose descriptive `aria-label`.                                            |
| `TrendIndicator` | optional  | `<span>` or icon          | adapter-owned | `api.trend_indicator_attrs()` | Decorative only.                                                                 |
| `HelpText`       | optional  | `<p>`                     | adapter-owned | `api.help_text_attrs()`       | Supplemental description.                                                        |

## 5. Attr Merge and Ownership Rules

- Core attrs include `role="group"`, `aria-label`, `aria-busy`, `data-ars-loading`, and `data-ars-trend`.
- The adapter owns derived root labeling, change `aria-label`, and decorative hiding for the trend icon.
- Consumer `class` and `style` merge additively but must not remove grouping or loading semantics.
- If a description-list host is used, the adapter owns the semantic structure and part wrappers.

## 6. Composition / Context Contract

`Stat` is standalone. It may resolve locale and messages from the nearest `ArsProvider`, but it does not publish adapter context to descendants.

## 7. Prop Sync and Event Mapping

- `label`, `value`, `change`, `trend`, and `loading` are render-derived.
- The adapter derives trend direction from `change` when `trend` is absent.
- There are no user-facing events; loading announcements are adapter-owned consequences of prop changes.

## 8. Registration and Cleanup Contract

- No descendant registration is required.
- If the implementation uses a shared live announcer for loading completion, cleanup is limited to releasing the announcer handle.

## 9. Ref and Node Contract

No live refs are required.

## 10. State Machine Boundary Rules

Stat has no state machine. Trend derivation and loading labeling are pure computations and must not drift from props.

## 11. Callback Payload Contract

No public adapter callbacks are required.

## 12. Failure and Degradation Rules

| Condition                                                | Policy             | Notes                                                            |
| -------------------------------------------------------- | ------------------ | ---------------------------------------------------------------- |
| missing `label` or `value`                               | fail fast          | Root grouping semantics depend on both.                          |
| `change` supplied without localized change label support | degrade gracefully | Render visible change text and a basic descriptive `aria-label`. |
| loading state without a completion announcer helper      | warn and ignore    | `aria-busy` remains the minimum accessible fallback.             |

## 13. Identity and Key Policy

The root and its optional trend subtree remain stable across loading and change updates. Optional parts may mount or unmount, but their relative order must stay fixed.

## 14. SSR and Client Boundary Rules

- SSR renders the same grouping structure and trend parts implied by initial props.
- Loading completion announcements are client-only.
- RTL direction may change visual icon mirroring after locale resolution, but structure remains stable.

## 15. Performance Constraints

- Derived trend and root label strings should be memoized.
- The adapter should not allocate icon or announcement helpers when there is no `change` and no `loading`.

## 16. Implementation Dependencies

| Dependency               | Required?   | Dependency type      | Why it must exist first                              | Notes                                        |
| ------------------------ | ----------- | -------------------- | ---------------------------------------------------- | -------------------------------------------- |
| formatting helper        | recommended | i18n contract        | Produces percent and directional labels.             | Shared with `meter` and `progress`.          |
| live announcement helper | optional    | accessibility helper | Announces loading completion or significant updates. | Only used when the product surface needs it. |

## 17. Recommended Implementation Sequence

1. Resolve locale/messages.
2. Derive trend direction and accessible change label.
3. Render grouping structure with root semantics.
4. Add optional loading or change announcement plumbing.

## 18. Anti-Patterns

- Do not rely on arrow icons or color alone to communicate direction.
- Do not split label and value into unrelated semantic islands.
- Do not expose the trend icon to assistive technology.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the component announces as one coherent metric.
- Consumers may assume visible trend symbols are backed by descriptive text.
- Consumers must not assume the adapter formats arbitrary raw values; the primary `value` is pre-formatted by the caller.

## 20. Platform Support Matrix

| Capability / behavior               | Web          | Desktop      | Mobile       | SSR            | Notes                                     |
| ----------------------------------- | ------------ | ------------ | ------------ | -------------- | ----------------------------------------- |
| grouped label/value/trend semantics | full support | full support | full support | full support   | Attr-only semantics are server-safe.      |
| loading completion announcement     | client-only  | client-only  | client-only  | SSR-safe empty | Optional helper behavior after mount.     |
| RTL trend icon mirroring            | full support | full support | full support | full support   | May be handled by CSS or mirrored assets. |

## 21. Debug Diagnostics and Production Policy

| Condition                                        | Debug build behavior | Production behavior | Notes                                         |
| ------------------------------------------------ | -------------------- | ------------------- | --------------------------------------------- |
| missing `label` or `value`                       | fail fast            | fail fast           | The stat contract is incomplete without them. |
| change text missing descriptive accessible label | debug warning        | warn and ignore     | Production still renders the visible delta.   |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required?   | Responsibility                                     | Reused by        | Notes                                       |
| ----------------- | ----------- | -------------------------------------------------- | ---------------- | ------------------------------------------- |
| formatting helper | recommended | Build trend prefixes and accessible change labels. | `meter`, `badge` | Keep locale-sensitive behavior centralized. |

## 23. Framework-Specific Behavior

Dioxus 0.7.x should memoize derived root and change attrs so unrelated parent re-renders do not churn the optional trend subtree.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct StatSketchProps {
    pub label: String,
    pub value: String,
    #[props(optional)]
    pub change: Option<f64>,
}

#[component]
pub fn Stat(props: StatSketchProps) -> Element {
    let api = use_memo(move || stat::Api::new(stat::Props {
        label: Some(props.label.clone()),
        value: Some(props.value.clone()),
        change: props.change,
        ..Default::default()
    }));
    let strategy = use_style_strategy();

    rsx! {
        div {
            ..attr_map_to_dioxus(api().root_attrs(), &strategy, None).attrs,
            span { ..attr_map_to_dioxus(api().label_attrs(), &strategy, None).attrs, {props.label.clone()} }
            span { ..attr_map_to_dioxus(api().value_attrs(), &strategy, None).attrs, {props.value.clone()} }
        }
    }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton is required beyond the canonical sketch for this stateless component, but description-list rendering must preserve the same part identities and semantics.

## 26. Adapter Invariants

- `Root` exposes one cohesive accessible label/value announcement.
- Trend meaning never relies on iconography alone.
- Loading semantics and trend semantics do not conflict on the root.

## 27. Accessibility and SSR Notes

- Prefer `<dl>` semantics when the surrounding layout benefits from native label-value association.
- Keep the trend indicator decorative and let `Change` carry the semantic description.
- SSR and hydration must agree on the initial loading and trend subtree.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit grouping and trend-label semantics.
- Intentional deviations: none beyond adapter-owned root grouping and optional loading-announcement helper usage.

## 29. Test Scenarios

1. Root announces label and value as a single metric.
2. Trend icon is hidden from AT while change text exposes a descriptive label.
3. Loading state applies `aria-busy` and clears it without changing structural identity.

## 30. Test Oracle Notes

- Preferred oracle for semantics: inspect root and change attrs in the accessibility tree.
- Preferred oracle for RTL behavior: snapshot mirrored icon styling or mirrored asset selection.
- Verification recipe: rerender through trend changes and ensure only the optional trend subtree changes.

## 31. Implementation Checklist

- [ ] Root groups label and value coherently.
- [ ] Trend accessible labeling is explicit.
- [ ] Trend icon remains decorative.
- [ ] Loading semantics do not require client-only structure changes.
- [ ] Tests cover grouping, trend semantics, and loading state.
