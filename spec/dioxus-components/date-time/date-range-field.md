---
adapter: dioxus
component: date-range-field
category: date-time
source: components/date-time/date-range-field.md
source_foundation: foundation/09-adapter-dioxus.md
---

# DateRangeField — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`DateRangeField`](../../components/date-time/date-range-field.md) contract onto a Dioxus 0.7.x component. The adapter adds twin-field composition, separator rendering, web-only hidden-input strategy, and Dioxus-specific cleanup rules.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct DateRangeFieldProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<Option<DateRange>>>,
    #[props(optional)]
    pub default_value: Option<DateRange>,
    #[props(optional)]
    pub min: Option<CalendarDate>,
    #[props(optional)]
    pub max: Option<CalendarDate>,
    #[props(optional)]
    pub locale: Option<Locale>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub readonly: bool,
    #[props(default = false)]
    pub required: bool,
    #[props(optional)]
    pub name: Option<String>,
    #[props(optional)]
    pub start_name: Option<String>,
    #[props(optional)]
    pub end_name: Option<String>,
    #[props(default = false)]
    pub force_leading_zeros: bool,
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Option<DateRange>>>,
}

#[component]
pub fn DateRangeField(props: DateRangeFieldProps) -> Element
```

The adapter owns the shared root, label, separator, error, and web-only hidden-input bridges, and composes two internal `DateField` adapters.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core range-field contract.
- Part parity: full parity with `Root`, `Label`, `StartField`, `Separator`, `EndField`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Known adapter deviations: hidden-input submission is web-only.

## 4. Part Mapping

| Core part / structure         | Required?   | Adapter rendering target            | Ownership                 | Attr source             | Notes                                   |
| ----------------------------- | ----------- | ----------------------------------- | ------------------------- | ----------------------- | --------------------------------------- |
| `Root` and `Label`            | required    | wrapper + shared label              | adapter-owned             | range-field API attrs   | Root is the shared accessible group.    |
| `StartField` / `EndField`     | required    | two internal `DateField` adapters   | adapter-owned composition | derived child props     | Child fields remain machine-owned.      |
| `Separator`                   | required    | host text node wrapper              | adapter-owned             | `api.separator_attrs()` | Always structural and non-interactive.  |
| `Description`, `ErrorMessage` | conditional | prose nodes                         | adapter-owned             | corresponding API attrs | Host equivalents may replace DOM attrs. |
| hidden input(s)               | conditional | web-only one or three hidden inputs | adapter-owned             | API attrs               | Omitted on desktop/mobile.              |

## 5. Attr Merge and Ownership Rules

| Target node       | Core attrs                                 | Adapter-owned attrs             | Consumer attrs        | Merge order                    | Ownership notes                            |
| ----------------- | ------------------------------------------ | ------------------------------- | --------------------- | ------------------------------ | ------------------------------------------ |
| `Root`            | group role, labels, described-by, required | state markers                   | decoration only       | core semantic attrs win on web | Non-web hosts may map semantics logically. |
| child field roots | child core attrs                           | derived min/max and aria labels | child decoration only | child semantics win            | Parent does not replace child groups.      |
| hidden inputs     | name(s) and ISO value(s)                   | none                            | none                  | adapter controls fully         | Web only.                                  |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data.
- Optional consumed contexts: utility `field` / `form` on web and logical equivalents elsewhere.
- Missing required context behavior: `ArsProvider` locale/ICU context follows the foundation policy in `09-adapter-dioxus.md` §16.
- Composition rule: the adapter derives start/end child props from the parent machine, including cross-bound constraints and localized labels.

## 7. Prop Sync and Event Mapping

- Controlled range sync updates the parent machine, which derives child `DateField` props.
- Child field changes normalize onto parent range updates.
- Separator text is render-time only.
- Hidden-input bridging mirrors the committed normalized range only on web, either as one interval string or two ISO date strings.

## 8. Registration and Cleanup Contract

| Registered entity           | Registration trigger  | Identity key     | Cleanup trigger           | Cleanup action              | Notes                                     |
| --------------------------- | --------------------- | ---------------- | ------------------------- | --------------------------- | ----------------------------------------- |
| child field bridge          | child mount           | composite        | child rerender or cleanup | drop stale callback bridge  | Prevent start/end cross-talk.             |
| hidden input bridge(s)      | web render with names | composite        | name change or cleanup    | remove hidden input node(s) | Support both single and split submission. |
| shared described-by linkage | root render           | instance-derived | rerender or cleanup       | rebuild described-by list   | Keep ids stable.                          |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner           | Node availability                  | Composition rule                      | Notes                               |
| ------------------ | ------------- | ------------------- | ---------------------------------- | ------------------------------------- | ----------------------------------- |
| `Root`             | yes           | adapter-owned       | required after mount               | may compose with utility field ref    | Used for group-level focus checks.  |
| child field groups | yes           | child adapter-owned | required after mount               | child adapters compose their own refs | Parent should not steal child refs. |
| separator          | no            | adapter-owned       | always structural, handle optional | no composition                        | Visual only.                        |
| hidden input(s)    | no            | adapter-owned       | always structural, handle optional | no composition                        | Web only.                           |

## 10. State Machine Boundary Rules

- Machine-owned state: normalized range, active field, and invalid-range semantics.
- Adapter-local derived bookkeeping: callback bridge handles and web form-bridge strategy.
- Forbidden local mirrors: start value, end value, or active field outside the parent machine.
- Allowed snapshot-read contexts: render, child-prop derivation, and web serialization.

## 11. Callback Payload Contract

| Callback                                     | Payload source             | Payload shape                                         | Timing                             | Cancelable? | Notes                         |
| -------------------------------------------- | -------------------------- | ----------------------------------------------------- | ---------------------------------- | ----------- | ----------------------------- |
| `on_value_change`                            | machine-derived snapshot   | `Option<DateRange>`                                   | after committed child-field update | no          | Emits normalized range state. |
| child-field bridge callbacks                 | normalized adapter payload | `{ field: ActiveField, value: Option<CalendarDate> }` | after child commit                 | no          | Internal bridge only.         |
| form-reset diagnostics callback when exposed | none                       | `()`                                                  | after reset handling               | no          | Diagnostics only.             |

## 12. Failure and Degradation Rules

- Both `name` and split names supplied: `warn and ignore` split names when single `name` is present.
- Missing `ArsProvider` locale/ICU context: `fail fast` unless the documented foundation fallback applies.
- Missing child bridge on rerender: `no-op`.
- Impossible range bounds: `warn and ignore`.
- Hosts without browser form APIs: `not applicable` for hidden-input behavior.

## 13. Identity and Key Policy

- Child field instances use `composite` identity from base id plus `start` / `end`.
- Hidden-input bridge uses `composite` identity from submission strategy and field name on web and `not applicable` elsewhere.
- Range normalization uses `data-derived` identity from endpoint dates.
- Server error keys are `not applicable`.

## 14. SSR and Client Boundary Rules

- Web SSR emits stable root, child field structure, separator, error, and hidden-input nodes.
- Child focus movement and refs remain client-only.
- Desktop/mobile hosts do not participate in SSR.
- Hydration must preserve start-before-end ordering and separator placement on web.

## 15. Performance Constraints

- Reuse child field instances and update only derived props.
- Avoid rebuilding hidden-input strategy when names are unchanged.
- Keep bridge callbacks instance-scoped.
- Do not replace the root group when one endpoint changes.

## 16. Implementation Dependencies

| Dependency                       | Required?   | Dependency type      | Why it matters                                                                                |
| -------------------------------- | ----------- | -------------------- | --------------------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context | required    | context contract     | Child field ordering, separator text, range formatting, and inherited direction depend on it. |
| `DateField` adapter              | required    | composition contract | Start and end fields should reuse the segmented contract.                                     |
| utility `field` / `form`         | required    | composition contract | Shared label, invalid, description, and reset semantics.                                      |
| hidden-input helper              | required    | shared helper        | Web submission strategy must stay aligned with `DateRangePicker`.                             |
| range-formatting helper          | recommended | shared helper        | Range descriptions and separator text should stay consistent.                                 |

## 17. Recommended Implementation Sequence

1. Render the shared root and label.
2. Compose start and end child `DateField` adapters.
3. Add separator, description, and error.
4. Add web-only single and split hidden-input bridges.
5. Finalize parent-child callback bridging and host fallback notes.

## 18. Anti-Patterns

- Do not maintain separate uncontrolled state inside child fields.
- Do not render both single and split hidden-input strategies simultaneously.
- Do not let start/end ordering drift from the parent machine’s normalized range.

## 19. Consumer Expectations and Guarantees

- Consumers may assume both child fields stay synchronized with the parent range value.
- Consumers may assume web hidden-input submission uses committed ISO dates only.
- Consumers must not assume desktop/mobile hosts expose browser-native form semantics.

## 20. Platform Support Matrix

| Capability / behavior                    | Web          | Desktop        | Mobile         | SSR            | Notes                                      |
| ---------------------------------------- | ------------ | -------------- | -------------- | -------------- | ------------------------------------------ |
| twin-field rendering and synchronization | full support | full support   | full support   | full support   | Structure and callbacks are host-neutral.  |
| hidden-input form bridge                 | full support | not applicable | not applicable | full support   | Browser forms exist only on web.           |
| child focus traversal                    | full support | full support   | fallback path  | SSR-safe empty | Mobile may rely on host focus affordances. |
| shared description / error wiring        | full support | full support   | full support   | full support   | Stable ids on web SSR.                     |

## 21. Debug Diagnostics and Production Policy

| Condition                                | Debug build behavior | Production behavior | Notes                                        |
| ---------------------------------------- | -------------------- | ------------------- | -------------------------------------------- |
| conflicting hidden-input naming strategy | debug warning        | warn and ignore     | Prefer single `name`.                        |
| invalid bounds                           | debug warning        | warn and ignore     | Keep machine-clamped range behavior.         |
| missing `ArsProvider` locale/ICU context | fail fast            | degrade gracefully  | Only via the documented foundation fallback. |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required? | Responsibility                                       | Reused by                             | Notes                     |
| ------------------------ | --------- | ---------------------------------------------------- | ------------------------------------- | ------------------------- |
| child field bridge       | yes       | Feed start/end child commits into the parent machine | range field and range picker adapters | Internal only.            |
| hidden-input helper      | yes       | Serialize one interval or two endpoint inputs on web | range field and range picker adapters | Shared concept.           |
| range description helper | no        | Build accessible range labels                        | range field and range picker adapters | Optional wrapper surface. |

## 23. Framework-Specific Behavior

- Dioxus child callback bridges should dispatch parent machine updates instead of patching parent signals directly.
- Child field props should be derived lazily from the parent snapshot.
- Desktop/mobile hosts may use logical group semantics when browser aria attrs are unavailable.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<date_range_field::Machine>(props);

rsx! {
    div { ..root_attrs,
        DateField { ..start_props }
        span { ..separator_attrs, {separator_text} }
        DateField { ..end_props }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
let start_props = machine.with_api_snapshot(|api| api.start_field_props());
let end_props = machine.with_api_snapshot(|api| api.end_field_props());
rsx! {
    DateField { ..start_props }
    DateField { ..end_props }
}
```

## 26. Adapter Invariants

- Parent machine state remains the only range source of truth.
- Web single and split hidden-input strategies are mutually exclusive.
- Child field constraints always reflect the current opposing endpoint.

## 27. Accessibility and SSR Notes

- Root group-level label and described-by wiring remain stable across hydration on web.
- Separator is always `aria-hidden` or host equivalent.
- Web SSR may emit hidden-input values because they derive from committed range state.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core range-field contract on web and logical parity on desktop/mobile.
- Intentional deviations: hidden-input form participation is web-only.
- Traceability note: adapter-owned concerns promoted here include child-field composition, hidden-input policy, and parent-child cleanup.

## 29. Test Scenarios

1. Start and end child fields receive correct labels and bounds.
2. Child commits update the normalized parent range.
3. Web single hidden-input strategy emits an ISO interval string.
4. Web split hidden-input strategy emits two ISO endpoint values.
5. Description and error wiring stays coherent across both child fields.

## 30. Test Oracle Notes

- `DOM attrs`: assert root group, separator hidden state, and hidden-input values on web.
- `machine state`: verify parent range after start and end edits across hosts.
- `context registration`: verify child bridges do not leak across rerenders.
- `cleanup side effects`: verify stale bridge callbacks clear on unmount.
- Cheap recipe: set split names on web, commit both endpoints, and assert two hidden inputs with ISO dates.

## 31. Implementation Checklist

- [ ] Parent machine remains the only range source of truth.
- [ ] Start and end child props derive from the parent snapshot.
- [ ] Web single and split hidden-input strategies are mutually exclusive.
- [ ] Separator stays structural and non-interactive.
- [ ] Utility `field` / `form` integration remains additive.
