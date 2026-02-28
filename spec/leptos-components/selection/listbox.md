---
adapter: leptos
component: listbox
category: selection
source: components/selection/listbox.md
source_foundation: foundation/08-adapter-leptos.md
---

# Listbox — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Listbox`](../../components/selection/listbox.md) contract onto Leptos 0.8.x. The adapter must preserve inline keyed selection surface with typeahead and optional load-more behavior while making keyed item registration, typeahead, selection semantics, description wiring, virtualization-aware focus strategy, and optional sentinel behavior explicit at the framework boundary.

## 2. Public Adapter API

```rust
#[component]
pub fn Listbox(
    #[prop(optional)] id: Option<String>,
    #[prop(optional, into)] value: Option<Signal<selection::Set>>,
    #[prop(optional)] default_value: selection::Set,
    #[prop(optional)] selection_mode: Option<selection::Mode>,
    #[prop(optional)] selection_behavior: Option<selection::Behavior>,
    #[prop(optional)] disabled_keys: Option<BTreeSet<Key>>,
    #[prop(optional)] on_action: Option<Callback<Key>>,
    #[prop(optional)] on_load_more: Option<Callback<()>>,
    children: Children,
) -> impl IntoView
```

Compound helpers typically include `Item`, `ItemGroup`, `Description`, and optional `LoadingSentinel` parts.

## 3. Mapping to Core Component Contract

- Props parity: full parity with inline keyed selection, optional action callbacks, disabled-key policy, and load-more sentinel behavior.
- Part parity: full parity for root, keyed items, description content, and optional load-more sentinel.
- Traceability note: this spec promotes typeahead cleanup, `aria-activedescendant` validity, iOS VoiceOver fallback, load-more behavior, and description wiring from the agnostic contract.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target  | Ownership     | Attr source                  | Notes                                                  |
| --------------------- | --------- | ------------------------- | ------------- | ---------------------------- | ------------------------------------------------------ |
| Root                  | required  | listbox host              | adapter-owned | api.root_attrs()             | Owns selection semantics and top-level focus strategy. |
| Item                  | repeated  | option host               | adapter-owned | api.item_attrs(key)          | One per keyed item.                                    |
| Description           | optional  | descriptive node          | shared        | api.description_attrs()      | Participates in described-by wiring.                   |
| LoadingSentinel       | optional  | adapter-owned status node | adapter-owned | api.loading_sentinel_attrs() | Used when load-more behavior is configured.            |

## 5. Attr Merge and Ownership Rules

- Core attrs win for listbox semantics, selected or disabled option state, and `aria-activedescendant` or roving-tabindex focus strategy output.
- The adapter owns described-by composition, sentinel attrs, and any virtualization-aware focus repair needed when options mount asynchronously.
- Consumers may decorate item content, but they must not replace keyed option hosts or the structural root listbox node.

## 6. Composition / Context Contract

The root publishes required listbox context to item, group, and optional sentinel parts. The adapter consumes environment and optional announcement or virtualization helpers. Missing root context is a fail-fast structural error for child parts.

## 7. Prop Sync and Event Mapping

| Adapter prop / event | Mode           | Sync trigger                          | Machine event / update path                 | Notes                                                                                       |
| -------------------- | -------------- | ------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `value`              | controlled     | signal change after mount             | selection sync event                        | Updates selection state and option attrs.                                                   |
| item activation      | adapter event  | click, Enter, Space, or modifier keys | selection and optional action events        | Modifier-key normalization is adapter-owned.                                                |
| typeahead            | adapter event  | printable key plus timestamp          | typeahead transition and timeout scheduling | Suppressed during IME composition when the component supports text composition input paths. |
| load-more sentinel   | adapter effect | intersection or scroll threshold      | `on_load_more` callback only                | No selection state is mutated by the sentinel itself.                                       |

## 8. Registration and Cleanup Contract

- The adapter owns keyed item registration, typeahead timeout cleanup, and any sentinel observation handles.
- Virtualization-aware focus repair must drop stale pending work when the highlighted item unmounts.
- Announcement or sentinel observers must detach on unmount.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability    | Composition rule                     | Notes                                                             |
| ------------------ | ------------- | ------------- | -------------------- | ------------------------------------ | ----------------------------------------------------------------- |
| Root               | yes           | adapter-owned | required after mount | compose only if explicitly forwarded | Needed for focus entry and active-descendant behavior.            |
| Item hosts         | recommended   | adapter-owned | required after mount | no composition by default            | Needed for roving tabindex or virtualization-aware scroll repair. |
| LoadingSentinel    | no            | adapter-owned | client-only          | no composition                       | Only when load-more behavior is enabled.                          |

## 10. State Machine Boundary Rules

- Machine-owned state: selected set, highlighted key, typeahead buffer, and disabled-item knowledge.
- Adapter-local derived bookkeeping: item registration, focus-strategy fallback, sentinel observation, and optional announcement throttling.
- Forbidden local mirrors: do not keep a second selected set or highlighted key outside the machine.
- Allowed snapshot reads: keyboard handlers, sentinel callbacks, focus repair effects, and timeout cleanup.

## 11. Callback Payload Contract

| Callback       | Payload source             | Payload shape | Timing                                  | Cancelable? | Notes                                                    |
| -------------- | -------------------------- | ------------- | --------------------------------------- | ----------- | -------------------------------------------------------- |
| `on_action`    | machine-derived snapshot   | `Key`         | after committed item activation         | no          | Distinct from selection changes.                         |
| `on_load_more` | normalized adapter payload | `()`          | when the sentinel or threshold triggers | no          | Suppressed while the documented loading state is active. |

## 12. Failure and Degradation Rules

| Condition                                        | Policy             | Notes                                                                |
| ------------------------------------------------ | ------------------ | -------------------------------------------------------------------- |
| duplicate item keys                              | fail fast          | Stable selection and focus behavior depend on unique keys.           |
| sentinel observation unavailable                 | degrade gracefully | Keep the structural sentinel but skip automatic load-more callbacks. |
| missing mounted item for active-descendant focus | degrade gracefully | Fall back to the documented roving or deferred-focus strategy.       |

## 13. Identity and Key Policy

- Items are data-derived by `Key` and registration order must match rendered order.
- Root and any sentinel nodes are instance-derived and must remain hydration-stable.
- Timeout and observation resources are instance-derived and cleanup-scoped.

## 14. SSR and Client Boundary Rules

- SSR renders the root, items, description, and sentinel shell if configured.
- Typeahead timers, sentinel observers, and DOM focus repair are client-only.
- Hydration must preserve item order and whichever focus strategy the server output expects.

## 15. Performance Constraints

- Do not re-register the full item set when only selection changes.
- Keep one typeahead timeout path per instance.
- Sentinel observation should use one shared observer or threshold path per listbox instance.

## 16. Implementation Dependencies

| Dependency               | Required?   | Dependency type | Why it must exist first                                                       | Notes                                                       |
| ------------------------ | ----------- | --------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------- |
| item registration helper | required    | shared helper   | Stable keyed registration drives highlight, selection, and focus repair.      | Shared with `select`, `combobox`, and menu-like components. |
| typeahead helper         | required    | shared helper   | Listbox typeahead behavior and cleanup must stay aligned across the category. | Shared with `select` and `menu`.                            |
| sentinel observer helper | recommended | shared helper   | Optional load-more behavior should not duplicate observer setup logic.        | Shared with scroll or data-display components.              |

## 17. Recommended Implementation Sequence

1. Initialize machine props, publish root context, and render keyed items.
2. Wire selection, action, modifier-key normalization, and typeahead before optional sentinel behavior.
3. Add description wiring, focus-strategy fallback, and virtualization-aware repair.
4. Attach optional load-more observation and verify cleanup on unmount.

## 18. Anti-Patterns

- Do not set `aria-activedescendant` to an empty string or a stale id.
- Do not conflate activation callbacks with every selection change.
- Do not leave stale observer or timeout handles alive after the listbox unmounts.

## 19. Consumer Expectations and Guarantees

- Consumers may assume inline selection and typeahead are machine-owned.
- Consumers may assume load-more behavior is optional and adapter-owned when configured.
- Consumers must not assume browser-default listbox behavior covers modifier-key normalization or focus fallback paths.

## 20. Platform Support Matrix

| Capability / behavior                               | Browser client | SSR            | Notes                               |
| --------------------------------------------------- | -------------- | -------------- | ----------------------------------- |
| keyed selection, typeahead, and described-by wiring | full support   | full support   | Structural semantics are SSR-safe.  |
| focus repair and iOS VoiceOver fallback             | client-only    | SSR-safe empty | Requires mounted option nodes.      |
| load-more sentinel observation                      | client-only    | SSR-safe empty | Observation begins after hydration. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                    | Debug build behavior | Production behavior | Notes                                                         |
| ------------------------------------------------------------ | -------------------- | ------------------- | ------------------------------------------------------------- |
| duplicate item keys                                          | fail fast            | fail fast           | Stable listbox semantics require unique keys.                 |
| active-descendant target missing after virtualization change | debug warning        | degrade gracefully  | Fallback focus repair must recover on the next stable render. |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required?   | Responsibility                                            | Reused by                    | Notes                                                      |
| ------------------------ | ----------- | --------------------------------------------------------- | ---------------------------- | ---------------------------------------------------------- |
| item registration helper | required    | Track keyed option registration and disabled state.       | `select`, `combobox`, `menu` | Rendering order must stay aligned with registration order. |
| typeahead helper         | required    | Own buffer updates and timeout cleanup.                   | `select`, `menu`, `menu-bar` | Reuse the shared timeout policy.                           |
| sentinel observer helper | recommended | Trigger `on_load_more` when the sentinel becomes visible. | `grid-list`, `table`         | Optional behavior only.                                    |

## 23. Framework-Specific Behavior

Leptos should choose between active-descendant and roving-tabindex behavior explicitly at initialization, keep item refs keyed, and route sentinel observation through client-only effects.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Listbox(/* props */ children: Children) -> impl IntoView {
    let machine = use_machine::<listbox::Machine>(listbox::Props { /* ... */ });
    provide_context(Context::from_machine(machine));

    view! {
        <div {..machine.derive(|api| api.root_attrs()).get()}>{children()}</div>
    }
}
```

## 25. Reference Implementation Skeleton

Keep one machine, one keyed item-registration helper, one typeahead timeout path, and one optional sentinel observer. Focus repair reads the committed highlighted key and mounted nodes only after the transition completes.

## 26. Adapter Invariants

- Registration order matches rendered option order.
- `aria-activedescendant` is either valid or absent; it is never stale.
- Sentinel callbacks never mutate selection state directly.

## 27. Accessibility and SSR Notes

- Description and error linkage must reference only rendered nodes.
- If iOS VoiceOver fallback is required, the adapter must document exactly when it switches to direct DOM focus.
- Load-more sentinel output must not interfere with option semantics or tabbable order.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter ownership of focus fallback and optional sentinel observation.
- Intentional deviations: non-web observation may use documented threshold fallbacks instead of browser-only observers.

## 29. Test Scenarios

1. Selection, toggle, and activation behavior follow the documented mode and callback rules.
2. Typeahead finds enabled items and cleans up timeout work on unmount.
3. Virtualization or delayed mounting does not leave stale active-descendant references.
4. Optional load-more observation fires one callback at the documented threshold.

## 30. Test Oracle Notes

- Preferred oracle for inline selection: `machine state` plus option `DOM attrs` for selected and highlighted state.
- Preferred oracle for focus fallback: `rendered structure` and DOM focus checks after virtualization or delayed mounting.
- Preferred oracle for sentinel behavior: `cleanup side effects` and callback assertions around observer teardown.

## 31. Implementation Checklist

- [ ] Keyed registration, focus fallback, and typeahead cleanup are explicit adapter contracts.
- [ ] Sentinel behavior is optional, cleanup-safe, and separate from selection state.
- [ ] A11y guidance covers valid active-descendant ids and fallback focus policy.
