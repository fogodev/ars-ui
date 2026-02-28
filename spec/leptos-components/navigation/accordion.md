---
adapter: leptos
component: accordion
category: navigation
source: components/navigation/accordion.md
source_foundation: foundation/08-adapter-leptos.md
---

# Accordion — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Accordion`](../../components/navigation/accordion.md) contract onto Leptos 0.8.x. The adapter preserves repeated item registration in DOM order, trigger-focused keyboard navigation, single-vs-multiple expansion rules, heading wrapper ownership, and scroll-position preservation during programmatic focus moves.

## 2. Public Adapter API

```rust
#[component]
pub fn Accordion(
    #[prop(optional, into)] value: Option<Signal<BTreeSet<Key>>>,
    #[prop(optional)] default_value: BTreeSet<Key>,
    #[prop(optional)] multiple: bool,
    #[prop(optional)] collapsible: bool,
    #[prop(optional)] orientation: Orientation,
    #[prop(optional)] dir: Direction,
    #[prop(optional)] heading_level: u8,
    #[prop(optional)] disabled_items: BTreeSet<Key>,
    children: Children,
) -> impl IntoView
```

The adapter owns repeated item registration, trigger refs, heading wrappers, and content-region linkage.

## 3. Mapping to Core Component Contract

- Props parity: full parity with open item set, `multiple`, `collapsible`, direction, orientation, and per-item disabled state.
- State parity: full parity with the core open-set and focused-trigger model.
- Part parity: full parity with `Root`, repeated `Item`, `ItemHeader`, `ItemTrigger`, `ItemIndicator`, and `ItemContent`.
- Adapter additions: explicit Leptos context publication, trigger registration, and scroll-preservation policy.

## 4. Part Mapping

| Core part / structure | Required?         | Adapter rendering target | Ownership     | Attr source                                           | Notes                                 |
| --------------------- | ----------------- | ------------------------ | ------------- | ----------------------------------------------------- | ------------------------------------- |
| `Root`                | required          | `<div>`                  | adapter-owned | `api.root_attrs()`                                    | owns ordered trigger registry         |
| `Item`                | repeated          | `<div>`                  | adapter-owned | `api.item_attrs(key)`                                 | keyed by item key                     |
| `ItemHeader`          | repeated          | heading wrapper          | adapter-owned | `api.item_header_attrs(key)`                          | semantic heading shell around trigger |
| `ItemTrigger`         | repeated          | `<button>`               | adapter-owned | `api.item_trigger_attrs(key, content_id)`             | focusable roving target               |
| `ItemIndicator`       | optional repeated | `<span>`                 | adapter-owned | `api.item_indicator_attrs(key)`                       | decorative only                       |
| `ItemContent`         | repeated          | `<div>`                  | adapter-owned | `api.item_content_attrs(key, content_id, trigger_id)` | `role="region"` when rendered         |

## 5. Attr Merge and Ownership Rules

| Target node   | Core attrs                                         | Adapter-owned attrs                               | Consumer attrs                         | Merge order                                | Ownership notes               |
| ------------- | -------------------------------------------------- | ------------------------------------------------- | -------------------------------------- | ------------------------------------------ | ----------------------------- |
| `Root`        | orientation and scope attrs                        | registration handlers and context publication     | wrapper decoration only                | required structure attrs win               | registry belongs to root      |
| `ItemTrigger` | expanded, controls, disabled, focus-visible attrs  | normalized click, keydown, and focus handlers     | decoration attrs and trailing handlers | required keyboard and ARIA attrs win       | trigger stays semantic owner  |
| `ItemContent` | region linkage, hidden state, open or closed attrs | presence decision if later wrappers add animation | decoration attrs                       | `aria-labelledby` and hidden semantics win | content remains adapter-owned |

## 6. Composition / Context Contract

`Accordion` publishes required root context containing machine access, ordered trigger registration, and focus helpers. All repeated item parts consume that context and fail fast when rendered outside the root boundary.

## 7. Prop Sync and Event Mapping

| Adapter prop                                                      | Mode       | Sync trigger              | Machine event / update path | Visible effect                                           | Notes                                            |
| ----------------------------------------------------------------- | ---------- | ------------------------- | --------------------------- | -------------------------------------------------------- | ------------------------------------------------ |
| `value`                                                           | controlled | signal change after mount | open-set sync event         | updates expanded triggers and visible content            | no controlled/uncontrolled switching after mount |
| `multiple`, `collapsible`, `orientation`, `dir`, `disabled_items` | controlled | rerender with new props   | core prop rebuild           | updates guards, focus navigation, and disabled semantics | registry identity remains stable                 |

| UI event                     | Preconditions                 | Machine event / callback path                       | Ordering notes                                                   | Notes                              |
| ---------------------------- | ----------------------------- | --------------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------- |
| trigger click or Enter/Space | target item not disabled      | `ToggleItem(key)`                                   | guard logic runs before trailing handlers                        | native button activation preferred |
| arrow-key navigation         | more than one enabled trigger | `FocusNext`, `FocusPrev`, `FocusFirst`, `FocusLast` | focus move uses registered order plus scroll-preservation policy | RTL swaps horizontal siblings only |
| trigger focus                | trigger receives focus        | `Focus { item, is_keyboard }`                       | focus-visible state settles before attrs are read                | focus target stays on trigger only |

## 8. Registration and Cleanup Contract

- Each item trigger registers on mount in current DOM order.
- Cleanup removes that trigger from the registry and recomputes sibling order.
- No stale registry entry may survive unmount or key changes.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner                                           | Node availability    | Composition rule                                        | Notes                                            |
| ------------------ | ------------- | --------------------------------------------------- | -------------------- | ------------------------------------------------------- | ------------------------------------------------ |
| each `ItemTrigger` | yes           | shared between adapter and consumer decoration only | required after mount | compose adapter `NodeRef` with any exposed consumer ref | programmatic focus depends on live trigger nodes |
| `ItemContent`      | no            | adapter-owned                                       | always structural    | no composition                                          | ids are sufficient for linkage                   |

## 10. State Machine Boundary Rules

- Open-set rules, focus-visible state, focused trigger key, and single-vs-multiple guards remain core-owned.
- Trigger registry, heading wrapper rendering, and scroll-position preservation remain adapter-owned.
- The adapter must not keep a second open-set source of truth.

## 11. Callback Payload Contract

| Callback              | Payload source           | Payload shape                   | Timing                 | Cancelable? | Notes              |
| --------------------- | ------------------------ | ------------------------------- | ---------------------- | ----------- | ------------------ |
| value-change callback | machine-derived snapshot | `{ open_items: BTreeSet<Key> }` | after committed toggle | no          | wrapper-owned only |

## 12. Failure and Degradation Rules

| Condition                                                                              | Policy             | Notes                                                         |
| -------------------------------------------------------------------------------------- | ------------------ | ------------------------------------------------------------- |
| missing root context for a repeated part                                               | fail fast          | compound structure violation                                  |
| all remaining open items would collapse while `collapsible=false` and `multiple=false` | warn and ignore    | keep last open item                                           |
| trigger node unavailable during focus move                                             | degrade gracefully | skip programmatic focus rather than corrupting registry state |

## 13. Identity and Key Policy

Item identity is the item key from the core contract. Registry order must match rendered DOM order for those keys on both server and client.

## 14. SSR and Client Boundary Rules

- SSR renders the same open items, heading wrappers, and content regions implied by the initial value.
- Live trigger refs and scroll-preservation logic are client-only.
- Server and client must preserve trigger order for hydration-safe keyboard navigation.

## 15. Performance Constraints

- Keep ordered registration incremental instead of rebuilding every trigger on each render.
- Avoid mirroring open state into per-item signals.
- Run scroll-preservation only on adapter-driven focus moves, not on every focus event.

## 16. Implementation Dependencies

| Dependency                    | Required?   | Dependency type     | Why it must exist first                                       | Notes                                   |
| ----------------------------- | ----------- | ------------------- | ------------------------------------------------------------- | --------------------------------------- |
| ordered registration helper   | required    | registration helper | trigger navigation depends on stable DOM-order registration   | shared with `tabs`                      |
| disclosure or presence helper | recommended | behavioral helper   | content visibility can reuse established open-region patterns | keep accordion-specific guards explicit |

## 17. Recommended Implementation Sequence

1. Initialize the core accordion machine.
2. Publish root context and ordered trigger registration.
3. Render repeated items, headers, triggers, indicators, and content regions.
4. Add keyboard focus movement and scroll-preservation logic.
5. Add final diagnostics for context misuse and invalid controlled transitions.

## 18. Anti-Patterns

- Do not focus content regions during roving navigation.
- Do not drop heading wrappers when the public API still exposes `heading_level`.
- Do not bypass the `collapsible=false` single-open guard in click paths.

## 19. Consumer Expectations and Guarantees

- Consumers may assume trigger order defines keyboard navigation order.
- Consumers may assume `ItemTrigger` remains the only focusable composite target by default.
- Consumers must not assume content mounts can reorder independently of item keys.

## 20. Platform Support Matrix

| Capability / behavior                                     | Browser client | SSR          | Notes                            |
| --------------------------------------------------------- | -------------- | ------------ | -------------------------------- |
| open-set semantics, heading wrappers, and content linkage | full support   | full support | baseline accordion behavior      |
| trigger registry and programmatic focus                   | full support   | client-only  | refs are runtime-only            |
| scroll-preservation during focus moves                    | full support   | client-only  | no browser API use on the server |

## 21. Debug Diagnostics and Production Policy

| Condition                           | Debug build behavior | Production behavior | Notes                       |
| ----------------------------------- | -------------------- | ------------------- | --------------------------- |
| repeated part rendered outside root | fail fast            | fail fast           | context contract violation  |
| unstable key or registry order      | debug warning        | warn and ignore     | preserves last stable order |

## 22. Shared Adapter Helper Notes

| Helper concept              | Required?   | Responsibility                                          | Reused by                       | Notes                                        |
| --------------------------- | ----------- | ------------------------------------------------------- | ------------------------------- | -------------------------------------------- |
| ordered registration helper | required    | tracks trigger order and cleanup                        | `tabs`, `navigation-menu`       | DOM-order source of truth                    |
| scroll-preservation helper  | recommended | preserves viewport position around adapter-driven focus | sibling-based roving composites | keep browser hash-scroll exceptions explicit |

## 23. Framework-Specific Behavior

Leptos should publish the accordion context with `provide_context`, compose trigger `NodeRef`s with any wrapper refs, and use watched `Signal<T>` input only for `value` when it is truly controlled after mount.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<accordion::Machine>(props);
provide_context(Context::from_machine(machine));

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        {children()}
    </div>
}
```

## 25. Reference Implementation Skeleton

- Initialize the machine from the committed props.
- Register each trigger in DOM order on mount and unregister on cleanup.
- Render heading wrappers and trigger-content linkage from the machine ids.
- Route click and keyboard focus through the same guard-protected core transitions.

## 26. Adapter Invariants

- Trigger registry order always matches DOM order.
- `ItemTrigger` remains the only roving focus target.
- `ItemContent` linkage always points back to its trigger id.
- Single-open non-collapsible mode never collapses the last open item.

## 27. Accessibility and SSR Notes

- `ItemContent` must keep `aria-labelledby` linkage to the trigger.
- Horizontal accordion navigation swaps left/right only in RTL sibling navigation, not vertical mode.
- SSR must preserve heading structure and initial open regions exactly.

## 28. Parity Summary and Intentional Deviations

- Matches the core accordion contract without intentional adapter divergence.
- Promotes heading wrappers, ordered trigger registration, scroll-preservation, and single-vs-multiple guards into explicit Leptos-facing guidance.

## 29. Test Scenarios

- single-open non-collapsible accordion
- multiple-open accordion with stable item order
- disabled item skipped by sibling navigation
- horizontal RTL trigger navigation
- scroll-preserved programmatic focus move between triggers

## 30. Test Oracle Notes

- Inspect DOM attrs for `aria-expanded`, `aria-controls`, and content-region linkage.
- Verify programmatic focus uses trigger order, not rendered child order of wrappers.
- Use browser tests to confirm scroll position is preserved when focus moves within the viewport.

## 31. Implementation Checklist

- [ ] Register triggers in DOM order and clean them up on unmount.
- [ ] Keep `ItemTrigger` as the only roving focus target.
- [ ] Preserve heading wrappers and trigger-content linkage.
- [ ] Enforce single-open non-collapsible guards in all toggle paths.
- [ ] Apply scroll-preservation only to adapter-driven focus moves.
