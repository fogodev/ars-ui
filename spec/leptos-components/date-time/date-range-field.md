---
adapter: leptos
component: date-range-field
category: date-time
source: components/date-time/date-range-field.md
source_foundation: foundation/08-adapter-leptos.md
---

# DateRangeField — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`DateRangeField`](../../components/date-time/date-range-field.md) contract onto a Leptos 0.8.x component. The adapter adds the twin-field composition contract, separator rendering, split-or-single hidden-input behavior, and Leptos-specific cleanup rules.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn DateRangeField(
    #[prop(optional)] id: Option<String>,
    #[prop(optional, into)] value: Signal<Option<DateRange>>,
    #[prop(optional)] default_value: Option<DateRange>,
    #[prop(optional)] min: Option<CalendarDate>,
    #[prop(optional)] max: Option<CalendarDate>,
    #[prop(optional)] locale: Option<Locale>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] readonly: bool,
    #[prop(optional)] required: bool,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] start_name: Option<String>,
    #[prop(optional)] end_name: Option<String>,
    #[prop(optional)] force_leading_zeros: bool,
    #[prop(optional)] on_value_change: Option<Callback<Option<DateRange>>>,
) -> impl IntoView
```

The adapter owns the shared root, label, separator, error, and hidden-input bridges, and delegates start and end segment bundles to two internal `DateField` adapters.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core range-field contract, including single or split form names.
- Part parity: full parity with `Root`, `Label`, `StartField`, `Separator`, `EndField`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Known adapter deviations: none beyond explicit Leptos composition and cleanup rules.

## 4. Part Mapping

| Core part / structure                        | Required?   | Adapter rendering target                  | Ownership                 | Attr source                                         | Notes                                                               |
| -------------------------------------------- | ----------- | ----------------------------------------- | ------------------------- | --------------------------------------------------- | ------------------------------------------------------------------- |
| `Root` and `Label`                           | required    | wrapper + shared label                    | adapter-owned             | range-field API attrs                               | Root is the shared accessible group.                                |
| `StartField` / `EndField`                    | required    | two internal `DateField` adapters         | adapter-owned composition | child `DateField` props derived from parent machine | Child fields remain machine-owned.                                  |
| `Separator`                                  | required    | `<span aria-hidden="true">`               | adapter-owned             | `api.separator_attrs()`                             | Text comes from localized messages.                                 |
| `Description`, `ErrorMessage`, hidden inputs | conditional | prose blocks + one or three hidden inputs | adapter-owned             | API attrs                                           | Hidden-input strategy depends on `name` vs `start_name`/`end_name`. |

## 5. Attr Merge and Ownership Rules

| Target node             | Core attrs                                | Adapter-owned attrs               | Consumer attrs                | Merge order               | Ownership notes                       |
| ----------------------- | ----------------------------------------- | --------------------------------- | ----------------------------- | ------------------------- | ------------------------------------- |
| `Root`                  | role, labelled-by, described-by, required | state markers and wrapper classes | wrapper decoration only       | core group semantics win  | Root remains adapter-owned.           |
| child `DateField` roots | child core attrs                          | derived min/max and aria labels   | child-wrapper decoration only | child field semantics win | Parent does not replace child groups. |
| hidden inputs           | name(s) and ISO value(s)                  | none                              | none                          | adapter controls fully    | Parent owns form bridge strategy.     |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data plus optional utility `field` / `form` contexts.
- Provided contexts: none.
- Missing required context behavior: `ArsProvider` locale/ICU context follows the foundation policy in `08-adapter-leptos.md` §13; utility contexts are optional.
- Composition rule: the adapter derives start-field and end-field props from the parent machine, including cross-bound min/max constraints and localized start/end labels.

## 7. Prop Sync and Event Mapping

- Controlled range sync updates the parent machine, which then derives child `DateField` props.
- Child field changes normalize onto parent range updates; the adapter must not maintain a parallel child-only source of truth.
- Separator text is render-time only and not interactive.
- Hidden-input bridging mirrors the committed normalized range only, either as one interval string or two ISO date strings.

## 8. Registration and Cleanup Contract

| Registered entity                  | Registration trigger           | Identity key     | Cleanup trigger           | Cleanup action              | Notes                                     |
| ---------------------------------- | ------------------------------ | ---------------- | ------------------------- | --------------------------- | ----------------------------------------- |
| child field bridge                 | child mount                    | composite        | child rerender or cleanup | drop stale callback bridge  | Prevent start/end cross-talk.             |
| hidden input bridge(s)             | render with form names present | composite        | name removal or cleanup   | remove hidden input node(s) | Support both single and split submission. |
| shared error / description linkage | root render                    | instance-derived | rerender or cleanup       | rebuild described-by list   | Keep ids stable.                          |

## 9. Ref and Node Contract

| Target part / node       | Ref required? | Ref owner           | Node availability                  | Composition rule                   | Notes                               |
| ------------------------ | ------------- | ------------------- | ---------------------------------- | ---------------------------------- | ----------------------------------- |
| `Root`                   | yes           | adapter-owned       | required after mount               | may compose with utility field ref | Used for group-level focus checks.  |
| child `DateField` groups | yes           | child adapter-owned | required after mount               | child adapter composes refs        | Parent should not steal child refs. |
| separator                | no            | adapter-owned       | always structural, handle optional | no composition                     | Visual only.                        |
| hidden input(s)          | no            | adapter-owned       | always structural, handle optional | no composition                     | Structural only.                    |

## 10. State Machine Boundary Rules

- Machine-owned state: normalized range, active field, invalid-range semantics, and localized separator text.
- Adapter-local derived bookkeeping: callback bridge handles and hidden-input strategy.
- Forbidden local mirrors: start value, end value, or active field outside the parent machine.
- Allowed snapshot-read contexts: render, child-prop derivation, and form-bridge serialization.

## 11. Callback Payload Contract

| Callback                                     | Payload source             | Payload shape                                         | Timing                               | Cancelable? | Notes                         |
| -------------------------------------------- | -------------------------- | ----------------------------------------------------- | ------------------------------------ | ----------- | ----------------------------- |
| `on_value_change`                            | machine-derived snapshot   | `Option<DateRange>`                                   | after a committed child-field update | no          | Emits normalized range state. |
| child-field bridge callbacks                 | normalized adapter payload | `{ field: ActiveField, value: Option<CalendarDate> }` | after child commit                   | no          | Internal bridge only.         |
| form-reset diagnostics callback when exposed | none                       | `()`                                                  | after reset handling                 | no          | Diagnostics only.             |

## 12. Failure and Degradation Rules

- Both `name` and `start_name`/`end_name` supplied: `warn and ignore` the split names and keep the single bridge authoritative.
- Missing `ArsProvider` locale/ICU context: `fail fast` unless the documented foundation fallback applies.
- Missing child field bridge on rerender: `no-op` for that frame and recover on next render.
- Impossible range bounds (`min > max`): `warn and ignore` invalid bound input.
- SSR-only absence of browser focus APIs: `SSR-safe empty`.

## 13. Identity and Key Policy

- Child field instances use `composite` identity from base id plus `start` / `end`.
- Hidden input bridge uses `composite` identity from submission strategy and field name.
- Range normalization uses `data-derived` identity from `CalendarDate` endpoints.
- Server error keys are `not applicable`.

## 14. SSR and Client Boundary Rules

- Server render emits stable root, child field structure, separator, error, and hidden-input nodes.
- Child focus movement and ref access remain client-only.
- Hydration must preserve start-before-end ordering and separator placement.
- Hidden-input values may render on the server because they derive from committed range state.

## 15. Performance Constraints

- Reuse child field instances and update only derived props.
- Avoid rebuilding hidden-input strategy when names are unchanged.
- Keep bridge callbacks instance-scoped.
- Do not replace the root group when one endpoint changes.

## 16. Implementation Dependencies

| Dependency                       | Required?   | Dependency type      | Why it matters                                                                                |
| -------------------------------- | ----------- | -------------------- | --------------------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context | required    | context contract     | Child field ordering, separator text, range formatting, and inherited direction depend on it. |
| `DateField` adapter              | required    | composition contract | Start and end fields should reuse the existing segmented field contract.                      |
| utility `field` / `form`         | required    | composition contract | Shared label, invalid, description, and reset semantics.                                      |
| hidden-input helper              | required    | shared helper        | Single and split ISO submission need one consistent implementation.                           |
| range-formatting helper          | recommended | shared helper        | Range descriptions and separator text should stay consistent.                                 |

## 17. Recommended Implementation Sequence

1. Render the shared root and label.
2. Compose start and end `DateField` children from parent-derived props.
3. Add separator, description, and error blocks.
4. Add single and split hidden-input bridges.
5. Finalize parent-child callback bridging and utility integration.

## 18. Anti-Patterns

- Do not maintain separate uncontrolled state inside the child fields.
- Do not render both single and split hidden-input strategies at the same time.
- Do not let start/end ordering drift from the parent machine’s normalized range.

## 19. Consumer Expectations and Guarantees

- Consumers may assume both child fields stay synchronized with the parent range value.
- Consumers may assume hidden-input submission uses committed ISO dates only.
- Consumers must not assume the visual separator is semantically interactive.

## 20. Platform Support Matrix

| Capability / behavior                    | Browser client | SSR            | Notes                                        |
| ---------------------------------------- | -------------- | -------------- | -------------------------------------------- |
| twin-field rendering and synchronization | full support   | full support   | Structure and committed values are SSR-safe. |
| hidden-input form bridge                 | full support   | full support   | Works with single or split names.            |
| child focus traversal                    | full support   | SSR-safe empty | Requires mounted child refs.                 |
| shared description / error wiring        | full support   | full support   | Stable ids across hydration.                 |

## 21. Debug Diagnostics and Production Policy

| Condition                                | Debug build behavior | Production behavior | Notes                                            |
| ---------------------------------------- | -------------------- | ------------------- | ------------------------------------------------ |
| conflicting hidden-input naming strategy | debug warning        | warn and ignore     | Prefer single `name`.                            |
| invalid bounds                           | debug warning        | warn and ignore     | Keep machine-clamped range behavior.             |
| missing `ArsProvider` locale/ICU context | fail fast            | degrade gracefully  | Only through the documented foundation fallback. |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required? | Responsibility                                             | Reused by                               | Notes                     |
| ------------------------ | --------- | ---------------------------------------------------------- | --------------------------------------- | ------------------------- |
| child field bridge       | yes       | Feed start/end field commits into the parent range machine | `date-range-field`, `date-range-picker` | Internal only.            |
| hidden-input helper      | yes       | Serialize one interval or two endpoint inputs              | range field and range picker adapters   | Shared concept.           |
| range description helper | no        | Build accessible range labels                              | range field and range picker adapters   | Optional wrapper surface. |

## 23. Framework-Specific Behavior

- Leptos child callback bridges should dispatch parent machine updates rather than mutate parent signals directly.
- Child field props should be derived lazily from the parent snapshot to avoid needless rerenders.
- Shared root focus checks should treat both child fields as inside the same group.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<date_range_field::Machine>(props);

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <DateField ..machine.with_api_snapshot(|api| api.start_field_props()) />
        <span {..machine.derive(|api| api.separator_attrs()).get()}>{move || machine.with_api_snapshot(|api| api.separator_text())}</span>
        <DateField ..machine.with_api_snapshot(|api| api.end_field_props()) />
    </div>
}
```

## 25. Reference Implementation Skeleton

```rust,no_check
let start_props = machine.with_api_snapshot(|api| api.start_field_props());
let end_props = machine.with_api_snapshot(|api| api.end_field_props());

view! {
    <DateField ..start_props />
    <DateField ..end_props />
    <Show when=move || single_name_present()><input {..hidden_input_attrs()} /></Show>
}
```

## 26. Adapter Invariants

- Parent machine state remains the only source of truth for the range.
- Single and split hidden-input strategies are mutually exclusive.
- Child field constraints always reflect the current opposing endpoint when present.

## 27. Accessibility and SSR Notes

- Root group-level label and described-by wiring remain stable across hydration.
- The separator is always `aria-hidden`.
- SSR may emit hidden-input values because they derive from committed range state.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core range-field contract.
- Intentional deviations: none at the contract level.
- Traceability note: adapter-owned concerns promoted explicitly here include child-field composition, hidden-input strategy, and parent-child cleanup.

## 29. Test Scenarios

1. Start and end child fields receive correct localized labels and bounds.
2. Child commits update the normalized parent range.
3. Single hidden-input strategy emits an ISO interval string.
4. Split hidden-input strategy emits two ISO endpoint values.
5. Description and error wiring stays coherent across both child fields.

## 30. Test Oracle Notes

- `DOM attrs`: assert root group attrs, separator `aria-hidden`, and hidden-input values.
- `machine state`: verify parent range after start and end edits.
- `context registration`: verify child bridges do not leak across rerenders.
- `cleanup side effects`: verify stale bridge callbacks are cleared on unmount.
- Cheap recipe: set `start_name` and `end_name`, commit both endpoints, and assert two hidden inputs with ISO dates.

## 31. Implementation Checklist

- [ ] Parent machine remains the only range source of truth.
- [ ] Start and end child props derive from the parent snapshot.
- [ ] Single and split hidden-input strategies are mutually exclusive.
- [ ] Separator stays structural and `aria-hidden`.
- [ ] Utility `field` / `form` integration remains additive.
