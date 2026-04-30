---
adapter: leptos
component: collapsible
category: layout
source: components/layout/collapsible.md
source_foundation: foundation/08-adapter-leptos.md
---

# Collapsible — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Collapsible`](../../components/layout/collapsible.md) contract onto Leptos `0.8.x` compound components. The adapter preserves the disclosure state machine, required `Root`, `Trigger`, and `Content` parts, optional `Indicator`, and adapter-owned lazy-mount and unmount-on-exit behavior.

## 2. Public Adapter API

```rust,no_check
pub mod collapsible {
    #[component]
    pub fn Collapsible(
        #[prop(optional, into)] open: Option<Signal<bool>>,
        #[prop(optional)] default_open: bool,
        #[prop(optional, into)] disabled: Signal<bool>,
        #[prop(optional)] lazy_mount: bool,
        #[prop(optional)] unmount_on_exit: bool,
        #[prop(optional)] collapsed_height: Option<String>,
        #[prop(optional)] collapsed_width: Option<String>,
        children: Children,
    ) -> impl IntoView

    #[component]
    pub fn Trigger(children: Children) -> impl IntoView

    #[component]
    pub fn Indicator(children: Children) -> impl IntoView

    #[component]
    pub fn Content(children: Children) -> impl IntoView
}
```

`Collapsible` owns the machine and publishes context. `Trigger`, `Indicator`, and `Content` are required descendants except that `Indicator` remains optional.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props`.
- State parity: full parity with the core `Open` and `Closed` states plus focus-visible bookkeeping.
- Part parity: full parity with `Root`, `Trigger`, `Indicator`, and `Content`.
- Adapter additions: explicit Leptos context contract and presence rules for `lazy_mount` and `unmount_on_exit`.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source             | Notes                                                                                   |
| --------------------- | --------- | ------------------------ | ------------- | ----------------------- | --------------------------------------------------------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`      | Publishes the collapsible context.                                                      |
| `Trigger`             | required  | `<button>` by default    | adapter-owned | `api.trigger_attrs()`   | Sends toggle and focus events.                                                          |
| `Indicator`           | optional  | `<span>`                 | adapter-owned | `api.indicator_attrs()` | Decorative by default and `aria-hidden`.                                                |
| `Content`             | required  | `<div>`                  | adapter-owned | `api.content_attrs()`   | Presence behavior depends on `lazy_mount`, `unmount_on_exit`, and collapsed-size props. |

## 5. Attr Merge and Ownership Rules

| Target node | Core attrs                                                                      | Adapter-owned attrs                        | Consumer attrs                          | Merge order                                                                                            | Ownership notes                          |
| ----------- | ------------------------------------------------------------------------------- | ------------------------------------------ | --------------------------------------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------- |
| `Root`      | `api.root_attrs()` including state and disabled markers                         | none beyond context publication            | wrapper decoration attrs when exposed   | required state attrs win                                                                               | root stays adapter-owned                 |
| `Trigger`   | `api.trigger_attrs()` including `aria-expanded`, `aria-controls`, and labels    | normalized press and focus handlers        | trigger decoration attrs                | core accessibility attrs win; handlers compose adapter before consumer when preventing invalid toggles | trigger remains adapter-owned            |
| `Indicator` | `api.indicator_attrs()`                                                         | none beyond structural wrapper choice      | indicator decoration attrs when exposed | `aria-hidden` must remain                                                                              | indicator is adapter-owned when rendered |
| `Content`   | `api.content_attrs()` including region attrs, `hidden`, and collapsed-size vars | presence gating and any exit-state markers | content decoration attrs                | core region and visibility attrs win                                                                   | content remains adapter-owned            |

## 6. Composition / Context Contract

- `Collapsible` provides required collapsible context containing machine access, ids, and adapter presence policy.
- `Trigger`, `Indicator`, and `Content` consume required context and fail fast when rendered outside `collapsible::Collapsible`.
- No optional external context is required, though locale or message resolution may be supplied through higher-level provider layers before `Root` constructs core props.

## 7. Prop Sync and Event Mapping

| Adapter prop                     | Mode       | Sync trigger                      | Machine event / update path                                    | Visible effect                                              | Notes                                                        |
| -------------------------------- | ---------- | --------------------------------- | -------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------ |
| `open`                           | controlled | signal change after mount         | `SetOpen(bool)`                                                | updates state attrs, trigger labels, and content visibility | do not support controlled/uncontrolled switching after mount |
| `disabled`                       | controlled | signal change after mount         | adapter rebuilds core props or dispatches equivalent sync path | blocks toggle behavior and focusable interaction            | visibility stays derived from current state                  |
| `lazy_mount` / `unmount_on_exit` | controlled | render-time and state transitions | adapter-owned presence policy                                  | decides whether content remains mounted while closed        | not a machine state by itself                                |

| UI event                  | Preconditions                    | Machine event / callback path | Ordering notes                                                 | Notes                                  |
| ------------------------- | -------------------------------- | ----------------------------- | -------------------------------------------------------------- | -------------------------------------- |
| trigger click             | not disabled                     | `Toggle`                      | adapter toggles before wrapper callbacks observe the new state | primary activation path                |
| trigger `Enter` / `Space` | trigger focused and not disabled | `Toggle`                      | must not double-fire with native button click                  | keyboard disclosure path               |
| trigger focus             | trigger receives focus           | `Focus { is_keyboard }`       | focus-visible repair runs before attrs are read                | keeps `data-ars-focus-visible` in sync |
| trigger or content blur   | focus leaves the active region   | `Blur`                        | blur must clear focus-visible bookkeeping before late cleanup  | preserves disclosure focus state       |

## 8. Registration and Cleanup Contract

| Registered entity                            | Registration trigger      | Identity key             | Cleanup trigger              | Cleanup action                                                | Notes                       |
| -------------------------------------------- | ------------------------- | ------------------------ | ---------------------------- | ------------------------------------------------------------- | --------------------------- |
| collapsible context                          | `Root` mount              | instance-derived ids     | `Root` cleanup               | drop provided context naturally with component teardown       | one context per root        |
| presence bookkeeping for content mount state | first content render path | instance-derived root id | state transitions or cleanup | insert or remove content according to adapter presence policy | no global registry required |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule        | Notes                                                                 |
| ------------------ | ------------- | ------------- | ---------------------------------- | ----------------------- | --------------------------------------------------------------------- |
| `Root`             | no            | adapter-owned | always structural, handle optional | no composition required | Structural wrapper only.                                              |
| `Trigger`          | no            | adapter-owned | always structural, handle optional | no composition required | IDs are sufficient for the documented aria linkage.                   |
| `Content`          | no            | adapter-owned | always structural, handle optional | no composition required | Measurement is not required unless a future animation helper opts in. |

## 10. State Machine Boundary Rules

- machine-owned state: open or closed state, disabled flag, focus, focus-visible, ids, and localized messages.
- adapter-local derived bookkeeping: presence decisions for `lazy_mount`, `unmount_on_exit`, and collapsed-size presentation.
- forbidden local mirrors: do not mirror `open` into a separate signal that can diverge from the machine state.
- allowed snapshot-read contexts: render derivation, trigger handlers, and presence gating only.

## 11. Callback Payload Contract

| Callback                                       | Payload source           | Payload shape    | Timing                                          | Cancelable? | Notes                                                          |
| ---------------------------------------------- | ------------------------ | ---------------- | ----------------------------------------------- | ----------- | -------------------------------------------------------------- |
| open change callback when exposed by a wrapper | machine-derived snapshot | `{ open: bool }` | after the machine accepts `Toggle` or `SetOpen` | no          | Wrapper-owned callback only; not a separate core adapter prop. |

## 12. Failure and Degradation Rules

| Condition                                                                        | Policy             | Notes                                                                    |
| -------------------------------------------------------------------------------- | ------------------ | ------------------------------------------------------------------------ |
| `Trigger`, `Indicator`, or `Content` rendered outside `collapsible::Collapsible` | fail fast          | Required context is missing.                                             |
| animation or presence helper unavailable                                         | degrade gracefully | Closed content falls back to immediate hide or unmount behavior.         |
| contradictory wrapper attrs attempt to remove disclosure semantics               | warn and ignore    | `aria-expanded`, `aria-controls`, and region linkage must remain intact. |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                                           | Notes                   |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | --------------------------------------------------------------------------------- | ----------------------- |
| root disclosure instance         | instance-derived | not applicable      | not applicable                           | ids and root or content structure must remain stable for the chosen initial state | one disclosure per root |

## 14. SSR and Client Boundary Rules

- SSR renders `Root` and `Trigger` for the initial state and renders `Content` according to `default_open`, controlled `open`, `lazy_mount`, and `unmount_on_exit`.
- Client-only listeners normalize focus-visible and toggle behavior.
- Hydration must preserve the initial content mount choice made on the server.

## 15. Performance Constraints

- Keep the context object stable except when machine state or versioning requires an update.
- Avoid remounting `Content` unnecessarily when only attrs change.
- Keep presence bookkeeping instance-scoped and effect-light.

## 16. Implementation Dependencies

| Dependency                | Required?   | Dependency type         | Why it must exist first                                                | Notes                                 |
| ------------------------- | ----------- | ----------------------- | ---------------------------------------------------------------------- | ------------------------------------- |
| disclosure context helper | required    | context contract        | `Trigger`, `Indicator`, and `Content` depend on a shared root context. | Core to all compound parts.           |
| presence helper           | recommended | shared helper           | Centralizes `lazy_mount` and `unmount_on_exit` sequencing.             | Shared with overlay content adapters. |
| button semantics helper   | recommended | behavioral prerequisite | Reuses normalized button activation and focus-visible handling.        | Avoids duplicate trigger logic.       |

## 17. Recommended Implementation Sequence

1. Create `Root` and initialize the core machine.
2. Publish required context for descendants.
3. Implement `Trigger` with normalized toggle and focus handlers.
4. Implement `Content` presence gating and collapsed-size attrs.
5. Add optional `Indicator` rendering and wrapper-level callback integration.

## 18. Anti-Patterns

- Do not leave `lazy_mount` and `unmount_on_exit` implied instead of documenting their presence behavior.
- Do not let trigger decoration remove required disclosure attrs.
- Do not mirror the open state in separate local signals.

## 19. Consumer Expectations and Guarantees

- Consumers may assume `Trigger` and `Content` ids stay linked through `aria-controls` and `aria-labelledby`.
- Consumers may assume `Indicator` is decorative unless they explicitly add semantics outside this spec.
- Consumers must not assume closed content remains mounted unless the documented presence props require it.

## 20. Platform Support Matrix

| Capability / behavior                      | Browser client | SSR          | Notes                                                                   |
| ------------------------------------------ | -------------- | ------------ | ----------------------------------------------------------------------- |
| disclosure semantics and presence behavior | full support   | full support | SSR renders the initial disclosure state without client-only observers. |

## 21. Debug Diagnostics and Production Policy

| Condition                                        | Debug build behavior | Production behavior | Notes                                    |
| ------------------------------------------------ | -------------------- | ------------------- | ---------------------------------------- |
| required descendant missing root context         | fail fast            | fail fast           | Compound parts must be nested correctly. |
| wrapper attrs conflict with disclosure semantics | debug warning        | warn and ignore     | Required aria wiring must survive.       |

## 22. Shared Adapter Helper Notes

| Helper concept            | Required?   | Responsibility                                          | Reused by                     | Notes                                             |
| ------------------------- | ----------- | ------------------------------------------------------- | ----------------------------- | ------------------------------------------------- |
| disclosure context helper | required    | Publishes machine access and ids to descendants.        | `toolbar`, `scroll-area`      | A generic compound-context pattern is sufficient. |
| presence helper           | recommended | Handles mount or unmount sequencing for closed content. | overlay content adapters      | Allows graceful fallback when absent.             |
| button semantics helper   | recommended | Normalizes trigger activation and focus-visible logic.  | other trigger-based compounds | Reuses shared utility behavior.                   |

## 23. Framework-Specific Behavior

Leptos can publish the compound context with `provide_context` and derive attrs reactively from the root machine. Presence decisions can remain render-driven unless a richer animation helper is layered on top.

## 24. Canonical Implementation Sketch

```rust
pub mod collapsible {
    #[component]
    pub fn Collapsible(children: Children) -> impl IntoView {
        let machine = use_machine::<collapsible::Machine>(collapsible::Props::default());
        provide_context(machine);
        view! { <div>{children()}</div> }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
pub mod collapsible {
    #[component]
    pub fn Collapsible(#[prop(optional, into)] open: Option<Signal<bool>>, children: Children) -> impl IntoView {
        let machine = use_machine::<collapsible::Machine>(build_props_from_signals(open));
        provide_context(Context::from_machine(machine));
        view! { <div {..machine.derive(|api| api.root_attrs()).get()}>{children()}</div> }
    }

    #[component]
    pub fn Trigger(children: Children) -> impl IntoView {
        let ctx = use_context::<Context>()
            .expect("collapsible::Trigger must be used inside Collapsible");
        view! { <button {..ctx.trigger_attrs()}>{children()}</button> }
    }

    #[component]
    pub fn Content(children: Children) -> impl IntoView {
        let ctx = use_context::<Context>()
            .expect("collapsible::Content must be used inside Collapsible");
        view! { <Show when=move || ctx.should_render_content()><div {..ctx.content_attrs()}>{children()}</div></Show> }
    }
}
```

## 26. Adapter Invariants

- `Root` always owns exactly one disclosure machine and one descendant context.
- `Trigger` always preserves `aria-expanded`, `aria-controls`, and accessible labeling.
- `Content` mount or unmount behavior follows the documented presence props.

## 27. Accessibility and SSR Notes

- The disclosure trigger and content region must keep their aria linkage intact even when content is partially collapsed.
- `hidden` must only be removed for closed content when a partial collapsed-size mode intentionally keeps content visible.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core `Collapsible` contract.
- Intentional deviations: none.
- Traceability note: adapter-owned presence behavior, trigger normalization, and context publication are restated explicitly instead of being left implicit in the agnostic spec.

## 29. Test Scenarios

- Toggle open and closed state and verify trigger and content attrs.
- Verify `lazy_mount` and `unmount_on_exit` behavior for initially closed content.
- Verify collapsed-size props omit `hidden` while preserving region semantics.
- Render a descendant outside `collapsible::Collapsible` and verify failure behavior.

## 30. Test Oracle Notes

- Trigger and content semantics: prefer `DOM attrs`.
- Presence behavior: prefer `rendered structure`.
- Open-state synchronization: prefer `machine state`.
- Missing context failure: prefer `fail fast` behavior.

## 31. Implementation Checklist

- [ ] Publish one required collapsible context from `Root`.
- [ ] Preserve disclosure attrs on `Trigger` and region attrs on `Content`.
- [ ] Implement `lazy_mount` and `unmount_on_exit` as explicit adapter-owned presence rules.
- [ ] Keep controlled `open` sync one-way and do not support switching modes after mount.
- [ ] Fail fast when compound descendants are rendered outside `collapsible::Collapsible`.
