---
adapter: leptos
component: menu
category: selection
source: components/selection/menu.md
source_foundation: foundation/08-adapter-leptos.md
---

# Menu — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Menu`](../../components/selection/menu.md) contract onto Leptos 0.8.x. The adapter must preserve triggered popup command menu with submenu support and typeahead while making trigger or content composition, keyed command registration, submenu wiring, popup positioning, focus return, and typeahead cleanup explicit at the framework boundary.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn Menu(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] positioning: Option<PositioningOptions>,
    #[prop(optional)] loop_focus: bool,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
    #[prop(optional)] on_action: Option<Callback<Key>>,
    children: Children,
) -> impl IntoView
```

Compound helpers typically include `Trigger`, `Positioner`, `Content`, `Item`, `CheckboxItem`, `RadioItem`, `SubTrigger`, `SubContent`, `Separator`, and `Shortcut` parts.

## 3. Mapping to Core Component Contract

- Props parity: full parity with open-state behavior, loop focus, submenu support, checkbox or radio item state, shortcut display, and positioning.
- Part parity: full parity for trigger, popup, items, submenu parts, and shortcut hints.
- Traceability note: this spec promotes submenu hover bridge behavior, typeahead timeout cleanup, focus return, and positioning responsibilities from the agnostic contract.

## 4. Part Mapping

| Core part / structure | Required?         | Adapter rendering target  | Ownership     | Attr source                | Notes                                             |
| --------------------- | ----------------- | ------------------------- | ------------- | -------------------------- | ------------------------------------------------- |
| Root                  | required          | wrapper element           | adapter-owned | api.root_attrs()           | Owns compound context and open-state scope.       |
| Trigger               | required          | native button             | adapter-owned | api.trigger_attrs()        | Primary entry point for the top-level menu.       |
| Positioner            | required          | positioned wrapper        | adapter-owned | api.positioner_attrs()     | Receives placement output.                        |
| Content               | required          | menu host                 | adapter-owned | api.content_attrs()        | Owns menu semantics and roving focus.             |
| Item                  | repeated          | menuitem host             | adapter-owned | api.item_attrs(key)        | Action, checkbox, radio, or submenu trigger item. |
| SubmenuContent        | optional repeated | nested positioned content | adapter-owned | api.sub_content_attrs(key) | Rendered for submenu items only.                  |
| Shortcut              | optional repeated | decorative text node      | adapter-owned | api.shortcut_attrs(key)    | Purely visual shortcut hint.                      |

## 5. Attr Merge and Ownership Rules

- Core attrs win for menu semantics, `aria-expanded`, checked-state attrs, submenu linkage, and roving tabindex state.
- The adapter owns positioner CSS variables, outside-interaction handling, focus return to the trigger, and purely decorative shortcut rendering.
- Consumers may decorate item content through documented parts, but they must not replace structural trigger, content, or submenu ownership boundaries.

## 6. Composition / Context Contract

The root publishes required menu context to trigger, content, item, and submenu parts. The adapter consumes positioning and dismissal helpers plus environment direction when submenu placement depends on it. Missing root context is a fail-fast structural error for child parts.

## 7. Prop Sync and Event Mapping

| Adapter prop / event | Mode                                    | Sync trigger                                                        | Machine event / update path                            | Notes                                                   |
| -------------------- | --------------------------------------- | ------------------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------- |
| open state           | machine-owned with callback observation | trigger, submenu trigger, escape, outside interaction, or prop sync | `Open` / `Close` / submenu transitions                 | Callbacks observe committed open-state changes.         |
| item activation      | adapter event                           | click, Enter, Space, or pointer hover bridge                        | `SelectItem`, checkbox/radio events, or submenu events | Action callbacks fire after the committed transition.   |
| typeahead            | adapter event                           | printable key plus timestamp                                        | typeahead transition and timeout scheduling            | Shared timeout cleanup is adapter-owned.                |
| submenu hover bridge | adapter effect                          | hover intent or pointer movement                                    | submenu open/close transitions                         | Delay and cleanup rules belong to the adapter contract. |

## 8. Registration and Cleanup Contract

- The adapter owns keyed item registration, submenu timers or hover bridges, positioning handles, and typeahead timeout cleanup.
- Outside-interaction and escape-listener helpers must detach on close or unmount.
- Submenu positioning subscriptions must release when nested content closes.

## 9. Ref and Node Contract

| Target part / node | Ref required?     | Ref owner     | Node availability    | Composition rule                     | Notes                                              |
| ------------------ | ----------------- | ------------- | -------------------- | ------------------------------------ | -------------------------------------------------- |
| Trigger            | yes               | adapter-owned | required after mount | compose only if explicitly forwarded | Focus returns here when the top-level menu closes. |
| Content            | yes               | adapter-owned | required after mount | no composition by default            | Owns roving focus and dismissal boundaries.        |
| Submenu content    | yes when rendered | adapter-owned | client-only          | no composition                       | Needed for nested positioning and focus handoff.   |

## 10. State Machine Boundary Rules

- Machine-owned state: open state, highlighted key, checked item data, submenu open key, and typeahead buffer.
- Adapter-local derived bookkeeping: keyed item registration, hover-bridge timers, positioning subscriptions, and dismissal boundaries.
- Forbidden local mirrors: do not keep an unsynchronized second open flag, highlighted key, or submenu tree.
- Allowed snapshot reads: trigger or item handlers, submenu hover bridges, dismissal callbacks, and timeout cleanup.

## 11. Callback Payload Contract

| Callback         | Payload source           | Payload shape | Timing                                 | Cancelable? | Notes                                                                               |
| ---------------- | ------------------------ | ------------- | -------------------------------------- | ----------- | ----------------------------------------------------------------------------------- |
| `on_open_change` | machine-derived snapshot | `bool`        | after top-level open-state transitions | no          | Nested submenu changes may be observed separately through documented local effects. |
| `on_action`      | machine-derived snapshot | `Key`         | after committed action-item activation | no          | Checkbox or radio toggles do not masquerade as generic action callbacks.            |

## 12. Failure and Degradation Rules

| Condition                                               | Policy             | Notes                                                         |
| ------------------------------------------------------- | ------------------ | ------------------------------------------------------------- |
| missing trigger or content part                         | fail fast          | Top-level menu semantics require both structural nodes.       |
| positioning helper unavailable                          | degrade gracefully | Render menu content inline with documented fallback behavior. |
| submenu hover bridge cannot resolve a live submenu node | warn and ignore    | Keep submenu closed and preserve parent menu integrity.       |

## 13. Identity and Key Policy

- Items and submenu triggers are data-derived by `Key` and registration order must match rendering order.
- Trigger, content, and submenu positioners are instance-derived structural nodes.
- Timeouts, dismissal handles, and positioning resources are instance-derived and cleanup-scoped.

## 14. SSR and Client Boundary Rules

- SSR renders the root, trigger, and any hydration-stable open content shells.
- Positioning, hover bridges, outside-interaction listeners, and timeout work are client-only.
- If open content is SSR-rendered, the same nested submenu structure must hydrate on the client.

## 15. Performance Constraints

- Typeahead and hover bridges must reuse one timer path per menu scope instead of stacking handles.
- Positioning and dismissal helpers must attach only while the relevant menu content is open.
- Nested submenu registration should be incremental rather than rebuilding the whole menu tree on every pointer move.

## 16. Implementation Dependencies

| Dependency         | Required? | Dependency type | Why it must exist first                                                            | Notes                                                     |
| ------------------ | --------- | --------------- | ---------------------------------------------------------------------------------- | --------------------------------------------------------- |
| positioning helper | required  | shared helper   | Menu and submenu placement are adapter-owned.                                      | Shared with `select`, `combobox`, and overlay components. |
| dismissal helper   | required  | shared helper   | Outside interaction and Escape handling must stay cleanup-safe.                    | Shared with popup-based overlay components.               |
| typeahead helper   | required  | shared helper   | Menu typeahead behavior and timeout cleanup must stay aligned across the category. | Shared with `select`, `listbox`, and `menu-bar`.          |

## 17. Recommended Implementation Sequence

1. Initialize machine props, publish root context, and render trigger plus top-level content shell.
2. Render keyed items, checkbox or radio variants, and submenu parts in stable order.
3. Wire action callbacks, typeahead, dismissal, focus return, and submenu hover bridges.
4. Add positioning behavior for top-level and nested content, then verify cleanup ordering.

## 18. Anti-Patterns

- Do not treat checkbox or radio state changes as generic action callbacks.
- Do not leave submenu timers or positioning subscriptions alive after nested content closes.
- Do not rely on browser-default menu behavior for focus return or roving tabindex.

## 19. Consumer Expectations and Guarantees

- Consumers may assume menu open state, highlighted item, and typeahead are machine-owned.
- Consumers may assume submenu and dismissal behavior is explicit adapter logic.
- Consumers must not assume shortcut text implies global shortcut registration; it is purely visual here.

## 20. Platform Support Matrix

| Capability / behavior                     | Browser client | SSR            | Notes                                       |
| ----------------------------------------- | -------------- | -------------- | ------------------------------------------- |
| top-level and nested menu semantics       | full support   | full support   | Structural popup shells are SSR-safe.       |
| positioning, dismissal, and hover bridges | client-only    | SSR-safe empty | Requires mounted nodes and event listeners. |
| typeahead timeout cleanup                 | client-only    | SSR-safe empty | Timeouts exist only after hydration.        |

## 21. Debug Diagnostics and Production Policy

| Condition                                    | Debug build behavior | Production behavior | Notes                                                         |
| -------------------------------------------- | -------------------- | ------------------- | ------------------------------------------------------------- |
| duplicate item keys                          | fail fast            | fail fast           | Stable highlight and submenu behavior depend on unique keys.  |
| submenu bridge cannot resolve nested content | debug warning        | warn and ignore     | Keep the submenu closed and recover on the next valid render. |

## 22. Shared Adapter Helper Notes

| Helper concept     | Required? | Responsibility                                  | Reused by                            | Notes                                               |
| ------------------ | --------- | ----------------------------------------------- | ------------------------------------ | --------------------------------------------------- |
| positioning helper | required  | Place top-level and nested menu content.        | `select`, `context-menu`, `combobox` | Nested content needs independent placement handles. |
| dismissal helper   | required  | Own outside interaction and escape-key cleanup. | `context-menu`, overlay popups       | Attach only while content is open.                  |
| typeahead helper   | required  | Own buffer updates and timeout cleanup.         | `select`, `listbox`, `menu-bar`      | Reuse the shared timeout policy.                    |

## 23. Framework-Specific Behavior

Leptos should keep submenu timers and registration in context-driven state, use effect teardown for cleanup, and derive submenu placement from the current direction before dispatching the placement helper.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Menu(/* props */ children: Children) -> impl IntoView {
    let machine = use_machine::<menu::Machine>(menu::Props { /* ... */ });
    provide_context(Context::from_machine(machine));

    view! {
        <div {..machine.derive(|api| api.root_attrs()).get()}>{children()}</div>
    }
}
```

## 25. Reference Implementation Skeleton

Keep one machine, one keyed item-registration helper, one dismissal helper, one positioning subscription per open content scope, and one shared typeahead timeout path. Action and open-state callbacks always observe the committed post-transition snapshot.

## 26. Adapter Invariants

- Trigger, content, submenu ownership, and dismissal boundaries remain explicit adapter responsibilities.
- Typeahead and submenu hover-bridge timers are cleanup-safe and instance-scoped.
- Action callbacks fire only for committed command activation, not for every state change inside the menu.

## 27. Accessibility and SSR Notes

- Submenu triggers and nested content must expose valid parent-child aria relationships and live ids only when the nested DOM exists.
- Focus return to the top-level trigger must be explicit on close rather than left to browser defaults.
- Shortcut text is decorative and must not be treated as the accessible name of the item.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter ownership of submenu behavior, dismissal, and positioning.
- Intentional deviations: non-web pointer or dismissal handling may use documented fallback paths rather than browser-specific events.

## 29. Test Scenarios

1. Opening and closing the top-level menu updates trigger attrs and focus return correctly.
2. Checkbox, radio, and action items follow distinct callback and state-update paths.
3. Submenu hover bridges and nested positioning open the correct submenu without leaking timers.
4. Typeahead highlights the next matching enabled item and clears timeout state on close or unmount.

## 30. Test Oracle Notes

- Preferred oracle for popup behavior: `DOM attrs` on trigger, content, and items plus `machine state` for committed open and highlight changes.
- Preferred oracle for submenu cleanup: `cleanup side effects` on timers and nested positioning handles.
- Preferred oracle for action timing: `callback order` distinguishing command activation from checkbox or radio updates.

## 31. Implementation Checklist

- [ ] Submenu, dismissal, positioning, and typeahead responsibilities are explicit and cleanup-safe.
- [ ] Action, checkbox, and radio behaviors are distinguished clearly in callbacks and tests.
- [ ] Focus return and aria relationships are covered in invariants and test scenarios.
