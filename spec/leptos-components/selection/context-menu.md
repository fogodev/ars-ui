---
adapter: leptos
component: context-menu
category: selection
source: components/selection/context-menu.md
source_foundation: foundation/08-adapter-leptos.md
---

# ContextMenu — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ContextMenu`](../../components/selection/context-menu.md) contract onto Leptos 0.8.x. The adapter must preserve pointer-anchored command menu opened from a target region while making target-region ownership, pointer-positioned popup behavior, focus return to the target, and menu action semantics without submenu support explicit at the framework boundary.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn ContextMenu(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] loop_focus: bool,
    #[prop(optional)] close_on_action: bool,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
    #[prop(optional)] on_action: Option<Callback<Key>>,
    children: Children,
) -> impl IntoView
```

Compound helpers typically include `Target`, `Positioner`, `Content`, `Item`, `CheckboxItem`, `RadioItem`, and `Separator` parts.

## 3. Mapping to Core Component Contract

- Props parity: full parity with disabled-key behavior, close-on-action policy, open-state observation, and action callbacks.
- Part parity: full parity for target region, point-positioned popup, and keyed action items.
- Traceability note: this spec promotes pointer-coordinate anchoring, keyboard-open fallback positioning, focus return to the target, and the intentional omission of submenus from the agnostic contract.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target                | Ownership     | Attr source            | Notes                                                               |
| --------------------- | --------- | --------------------------------------- | ------------- | ---------------------- | ------------------------------------------------------------------- |
| Root                  | required  | wrapper element                         | adapter-owned | api.root_attrs()       | Owns context-menu scope and open-state policy.                      |
| Target                | required  | consumer or adapter-owned target region | shared        | api.target_attrs()     | Owns the `contextmenu` interaction entry point.                     |
| Positioner            | required  | point-positioned wrapper                | adapter-owned | api.positioner_attrs() | Anchored to pointer coordinates or keyboard-open fallback geometry. |
| Content               | required  | menu host                               | adapter-owned | api.content_attrs()    | Owns command semantics and roving focus.                            |
| Item                  | repeated  | menuitem host                           | adapter-owned | api.item_attrs(key)    | Action, checkbox, or radio item.                                    |

## 5. Attr Merge and Ownership Rules

- Core attrs win for menu semantics, target-region accessibility state, and checked-item attrs.
- The adapter owns point-positioning output, target-region event normalization, and focus return to the target after close.
- Consumers may decorate target content, but they must not replace the structural target interaction boundary or assume submenu parts exist.

## 6. Composition / Context Contract

The root publishes required context to target, content, and item parts. The adapter consumes dismissal and point-positioning helpers. Missing root context is a fail-fast structural error for child parts.

## 7. Prop Sync and Event Mapping

| Adapter prop / event | Mode                                    | Sync trigger                                                                     | Machine event / update path                 | Notes                                                                |
| -------------------- | --------------------------------------- | -------------------------------------------------------------------------------- | ------------------------------------------- | -------------------------------------------------------------------- |
| open state           | machine-owned with callback observation | native `contextmenu`, keyboard fallback, outside interaction, or item activation | `ContextOpen` / `Close`                     | Callbacks observe committed open-state changes.                      |
| target interaction   | adapter event                           | pointer coordinates or Shift+F10 / Context Menu key                              | open transition plus anchor-point update    | Pointer and keyboard paths normalize into one anchor-point contract. |
| item activation      | adapter event                           | click, Enter, or Space                                                           | command or checkbox/radio transitions       | Submenus are intentionally unsupported.                              |
| typeahead            | adapter event                           | printable key plus timestamp                                                     | typeahead transition and timeout scheduling | Uses the shared menu timeout policy.                                 |

## 8. Registration and Cleanup Contract

- The adapter owns target-region event listeners, point-positioning handles, dismissal helpers, and typeahead timeout cleanup.
- Any stale pointer-coordinate state must be cleared on close.
- Listeners attached to the target region must detach on unmount.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner           | Node availability    | Composition rule                         | Notes                                             |
| ------------------ | ------------- | ------------------- | -------------------- | ---------------------------------------- | ------------------------------------------------- |
| Target             | yes           | shared with adapter | required after mount | compose only through a documented helper | Focus returns here when the menu closes.          |
| Content            | yes           | adapter-owned       | required after mount | no composition by default                | Owns roving focus and dismissal boundaries.       |
| Positioner         | yes           | adapter-owned       | client-only          | no composition                           | Anchored to pointer or keyboard-open coordinates. |

## 10. State Machine Boundary Rules

- Machine-owned state: open state, highlighted key, checked-item data, and typeahead buffer.
- Adapter-local derived bookkeeping: anchor-point coordinates, dismissal boundaries, positioning subscriptions, and focus-return target handling.
- Forbidden local mirrors: do not keep a second open flag or highlighted key outside the machine.
- Allowed snapshot reads: target event handlers, content dismissal callbacks, action handlers, and timeout cleanup.

## 11. Callback Payload Contract

| Callback         | Payload source           | Payload shape | Timing                                 | Cancelable? | Notes                                                                    |
| ---------------- | ------------------------ | ------------- | -------------------------------------- | ----------- | ------------------------------------------------------------------------ |
| `on_open_change` | machine-derived snapshot | `bool`        | after open-state transitions           | no          | Observes the committed open state only.                                  |
| `on_action`      | machine-derived snapshot | `Key`         | after committed action-item activation | no          | Checkbox and radio updates remain distinct from generic command actions. |

## 12. Failure and Degradation Rules

| Condition                            | Policy             | Notes                                                                                   |
| ------------------------------------ | ------------------ | --------------------------------------------------------------------------------------- |
| missing target part                  | fail fast          | ContextMenu requires a target region to anchor focus return and keyboard-open behavior. |
| point-positioning helper unavailable | degrade gracefully | Render content near the target bounds with documented fallback placement.               |
| submenu part requested               | warn and ignore    | ContextMenu intentionally does not support submenus.                                    |

## 13. Identity and Key Policy

- Items are data-derived by `Key` and registration order must match rendering order.
- Target, content, and point-positioner nodes are instance-derived structural nodes.
- Anchor-point and timeout resources are instance-derived and cleared on close or unmount.

## 14. SSR and Client Boundary Rules

- SSR renders the target region and any closed-menu structural shell only.
- Point-positioning, dismissal, target listeners, and typeahead timers are client-only.
- Hydration must preserve target identity so focus return remains valid.

## 15. Performance Constraints

- Target-region listeners should attach once per instance rather than on every render.
- Point-positioning work should run only while content is open.
- Typeahead timeout handling should reuse the shared menu timeout path.

## 16. Implementation Dependencies

| Dependency               | Required? | Dependency type | Why it must exist first                                             | Notes                                                   |
| ------------------------ | --------- | --------------- | ------------------------------------------------------------------- | ------------------------------------------------------- |
| point-positioning helper | required  | shared helper   | ContextMenu must anchor to pointer or keyboard-derived coordinates. | Shared with other point-positioned popups when present. |
| dismissal helper         | required  | shared helper   | Outside interaction and escape handling remain adapter-owned.       | Shared with `menu` and overlay popups.                  |
| typeahead helper         | required  | shared helper   | Menu-like typeahead cleanup stays aligned across the category.      | Shared with `menu`, `listbox`, and `menu-bar`.          |

## 17. Recommended Implementation Sequence

1. Initialize machine props, publish root context, and render the target region.
2. Wire native `contextmenu` and keyboard-open paths into one anchor-point contract.
3. Render point-positioned content and keyed items, then add dismissal, focus return, and typeahead behavior.
4. Verify close-on-action behavior, coordinate cleanup, and unmount cleanup ordering.

## 18. Anti-Patterns

- Do not silently add submenu support; the spec intentionally omits it.
- Do not return focus to a generic trigger button; focus returns to the target region.
- Do not keep stale pointer coordinates after the menu closes.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the target region owns right-click and keyboard-open entry behavior.
- Consumers may assume point positioning and focus return are explicit adapter responsibilities.
- Consumers must not assume submenu helpers exist in ContextMenu.

## 20. Platform Support Matrix

| Capability / behavior                             | Browser client | SSR            | Notes                                                  |
| ------------------------------------------------- | -------------- | -------------- | ------------------------------------------------------ |
| target-region semantics and command-item behavior | full support   | full support   | The closed target shell is SSR-safe.                   |
| point positioning and target listeners            | client-only    | SSR-safe empty | Requires mounted nodes and pointer or keyboard events. |
| typeahead timeout cleanup                         | client-only    | SSR-safe empty | Timeouts exist only after hydration.                   |

## 21. Debug Diagnostics and Production Policy

| Condition                                          | Debug build behavior | Production behavior | Notes                                     |
| -------------------------------------------------- | -------------------- | ------------------- | ----------------------------------------- |
| submenu composition requested                      | debug warning        | warn and ignore     | ContextMenu intentionally omits submenus. |
| target region missing a live node for focus return | fail fast            | fail fast           | The close path depends on it.             |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required? | Responsibility                                                      | Reused by                     | Notes                                                             |
| ------------------------ | --------- | ------------------------------------------------------------------- | ----------------------------- | ----------------------------------------------------------------- |
| point-positioning helper | required  | Translate pointer or keyboard-open geometry into positioner output. | `menu`, overlay popups`       | Keyboard-open uses target bounds rather than pointer coordinates. |
| dismissal helper         | required  | Own outside interaction and escape cleanup.                         | `menu`, overlay popups`       | Attach only while content is open.                                |
| typeahead helper         | required  | Own buffer updates and timeout cleanup.                             | `menu`, `menu-bar`, `listbox` | Reuse the shared timeout policy.                                  |

## 23. Framework-Specific Behavior

Leptos should normalize native `contextmenu`, Shift+F10, and Context Menu key events into one anchor-point path, and keep target listeners in explicit cleanup-bound effects.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn ContextMenu(/* props */ children: Children) -> impl IntoView {
    let machine = use_machine::<context_menu::Machine>(context_menu::Props { /* ... */ });
    provide_context(Context::from_machine(machine));

    view! { <div {..machine.derive(|api| api.root_attrs()).get()}>{children()}</div> }
}
```

## 25. Reference Implementation Skeleton

Keep one machine, one target-event normalization path, one point-positioning handle, one dismissal helper, and one shared typeahead timeout path. Focus return always resolves against the committed target node after close.

## 26. Adapter Invariants

- The target region is required and remains the focus-return anchor for close behavior.
- Point-positioning state is adapter-owned and cleared on close.
- ContextMenu never exposes submenu behavior.

## 27. Accessibility and SSR Notes

- Keyboard-open behavior must position the menu relative to the target in a predictable way for non-pointer users.
- Focus return to the target is explicit and should not depend on browser defaults.
- Because submenus are unsupported, aria relationships should never imply nested menu ownership.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter ownership of point positioning, target event normalization, and focus return.
- Intentional deviations: submenus remain intentionally unsupported, matching the core spec.

## 29. Test Scenarios

1. Right-click opens the menu at the pointer position and returns focus to the target on close.
2. Shift+F10 and Context Menu key open the menu at the documented keyboard fallback position.
3. Action, checkbox, and radio item activation follow the documented callback rules.
4. Submenu composition requests are ignored with the documented diagnostic behavior.

## 30. Test Oracle Notes

- Preferred oracle for positioning: `DOM attrs` plus rendered positioner structure after pointer and keyboard-open paths.
- Preferred oracle for focus return: DOM focus checks and `callback order` after closing the menu.
- Preferred oracle for intentional submenu omission: `rendered structure` and diagnostics behavior when submenu parts are attempted.

## 31. Implementation Checklist

- [ ] Target-region ownership, point positioning, and focus return are explicit adapter contracts.
- [ ] Submenu omission is documented in behavior, failure, parity, and test sections.
- [ ] Typeahead and dismissal cleanup stay instance-scoped and cleanup-safe.
