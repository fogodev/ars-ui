---
adapter: leptos
component: menu-bar
category: selection
source: components/selection/menu-bar.md
source_foundation: foundation/08-adapter-leptos.md
---

# MenuBar — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`MenuBar`](../../components/selection/menu-bar.md) contract onto Leptos 0.8.x. The adapter must preserve persistent top-level menu strip that opens nested menu content while making top-level roving focus, active-menu switching, nested menu content composition, directional keyboard handling, and cleanup-safe focus return explicit at the framework boundary.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn MenuBar(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] orientation: Option<Orientation>,
    #[prop(optional)] loop_focus: bool,
    #[prop(optional)] locale: Option<Locale>,
    children: Children,
) -> impl IntoView
```

Compound helpers typically include `Item`, `MenuPositioner`, `MenuContent`, `MenuItem`, `MenuCheckboxItem`, `MenuRadioItem`, and `Shortcut` parts.

## 3. Mapping to Core Component Contract

- Props parity: full parity with top-level orientation, focus looping, locale-sensitive shortcut labels, and nested menu activation.
- Part parity: full parity for the menubar root, top-level items, active nested menu content, and shortcut hints.
- Traceability note: this spec promotes top-level roving focus, active-menu switching, directional arrow-key handling, and nested-menu cleanup from the agnostic contract.

## 4. Part Mapping

| Core part / structure | Required?                      | Adapter rendering target | Ownership     | Attr source                    | Notes                                         |
| --------------------- | ------------------------------ | ------------------------ | ------------- | ------------------------------ | --------------------------------------------- |
| Root                  | required                       | menubar host             | adapter-owned | api.root_attrs()               | Owns the top-level roving focus scope.        |
| Item                  | repeated                       | top-level menuitem host  | adapter-owned | api.item_attrs(key)            | One per top-level menu trigger.               |
| MenuPositioner        | required when a menu is active | positioned wrapper       | adapter-owned | api.menu_positioner_attrs(key) | Hosts the active nested menu.                 |
| MenuContent           | required when a menu is active | nested menu host         | adapter-owned | api.menu_content_attrs(key)    | Rendered for the active top-level menu only.  |
| Shortcut              | optional repeated              | decorative text node     | adapter-owned | api.shortcut_attrs(key)        | Purely visual shortcut hint for nested items. |

## 5. Attr Merge and Ownership Rules

- Core attrs win for menubar semantics, active item state, and nested menu accessibility relationships.
- The adapter owns directional keyboard normalization, top-level focus movement, nested menu positioners, and decorative shortcut rendering.
- Consumers may decorate top-level labels or nested item content, but they must not replace the structural menubar root or active-menu ownership boundary.

## 6. Composition / Context Contract

The root publishes required menubar context to top-level items and active nested-menu parts. The adapter consumes direction, positioning, and dismissal helpers. Missing root context is a fail-fast structural error for child parts.

## 7. Prop Sync and Event Mapping

| Adapter prop / event  | Mode                     | Sync trigger                                            | Machine event / update path                                        | Notes                                                                         |
| --------------------- | ------------------------ | ------------------------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| top-level active menu | machine-owned            | arrow keys, Enter, click, or pointer activation         | `ActivateMenu`, `MoveToNextMenu`, `MoveToPrevMenu`, or `CloseMenu` | The adapter owns top-level directional key normalization.                     |
| nested menu action    | adapter event            | click, Enter, Escape, or outside interaction            | nested menu transitions                                            | Nested menu behavior follows the shared menu contract inside the active menu. |
| shortcut labels       | adapter-owned decoration | locale or platform change                               | no machine mutation                                                | Shortcut text remains purely visual.                                          |
| typeahead             | adapter event            | printable key plus timestamp inside active menu content | shared menu-typeahead path                                         | Top-level strip does not duplicate nested-menu typeahead logic.               |

## 8. Registration and Cleanup Contract

- The adapter owns active-menu positioning handles, dismissal helpers, nested-menu registration, and any shared typeahead timeout work.
- Closing or switching the active menu must release the previous nested-menu resources before opening the next one.
- Top-level focus-return bookkeeping must be cleared on unmount.

## 9. Ref and Node Contract

| Target part / node | Ref required?     | Ref owner     | Node availability    | Composition rule                     | Notes                                                            |
| ------------------ | ----------------- | ------------- | -------------------- | ------------------------------------ | ---------------------------------------------------------------- |
| Root               | yes               | adapter-owned | required after mount | compose only if explicitly forwarded | Needed for roving-focus entry and active-item focus repair.      |
| Top-level items    | recommended       | adapter-owned | required after mount | no composition by default            | Needed for directional focus movement and active-menu switching. |
| Menu content       | yes when rendered | adapter-owned | client-only          | no composition                       | Needed for nested dismissal and focus handoff.                   |

## 10. State Machine Boundary Rules

- Machine-owned state: active top-level menu, focused top-level item, and nested-menu state.
- Adapter-local derived bookkeeping: top-level item registration, directional key normalization, nested positioning handles, and focus-return helpers.
- Forbidden local mirrors: do not keep a second active-menu flag or focused item outside the machine.
- Allowed snapshot reads: top-level key handlers, nested dismissal callbacks, positioning updates, and cleanup.

## 11. Callback Payload Contract

| Callback                     | Payload source | Payload shape | Timing         | Cancelable? | Notes                                                               |
| ---------------------------- | -------------- | ------------- | -------------- | ----------- | ------------------------------------------------------------------- |
| no public top-level callback | none           | none          | not applicable | no          | Nested action callbacks belong to the active menu content contract. |

## 12. Failure and Degradation Rules

| Condition                                                    | Policy             | Notes                                                                                 |
| ------------------------------------------------------------ | ------------------ | ------------------------------------------------------------------------------------- |
| active top-level key missing from registered items           | fail fast          | Menubar navigation depends on a live keyed top-level item.                            |
| nested menu resources not released during active-menu switch | warn and ignore    | The next switch should re-establish a clean active-menu scope.                        |
| positioning helper unavailable                               | degrade gracefully | Render nested content inline under the active item with documented fallback behavior. |

## 13. Identity and Key Policy

- Top-level items are data-derived by `Key` and registration order must match rendered order.
- Root and active nested-menu positioner are instance-derived structural nodes.
- Directional key-normalization and positioning resources are instance-derived and cleanup-scoped.

## 14. SSR and Client Boundary Rules

- SSR renders the top-level menubar strip and any hydration-stable active-menu shell.
- Directional focus movement, nested positioning, and dismissal helpers are client-only.
- Hydration must preserve top-level item order and the active menu identity if the server rendered an open menu.

## 15. Performance Constraints

- Switching the active menu should reuse top-level registration instead of rebuilding the menubar tree.
- Only one nested menu scope should be positioned and observed at a time.
- Shortcut-label decoration should be memoized or derived from stable locale or platform inputs only.

## 16. Implementation Dependencies

| Dependency                    | Required? | Dependency type  | Why it must exist first                                                                    | Notes                                               |
| ----------------------------- | --------- | ---------------- | ------------------------------------------------------------------------------------------ | --------------------------------------------------- |
| top-level registration helper | required  | shared helper    | Roving focus and active-menu switching depend on stable keyed top-level item registration. | Specific to menubar-like composites.                |
| positioning helper            | required  | shared helper    | Active nested menus still require popup placement.                                         | Shared with `menu`, `select`, and overlay popups.   |
| direction helper              | required  | context contract | Horizontal keyboard behavior must respect direction and orientation.                       | Shared with `segment-group` and list-like controls. |

## 17. Recommended Implementation Sequence

1. Initialize machine props, publish root context, and render top-level items.
2. Wire directional focus movement, active-menu switching, and top-level activation behavior.
3. Render the active nested menu positioner and content using the shared menu contract.
4. Add dismissal, focus return, shortcut decoration, and cleanup checks for active-menu switching.

## 18. Anti-Patterns

- Do not treat the menubar as a generic menu trigger; it owns persistent top-level roving focus.
- Do not keep more than one active nested menu positioned or observed at the same time.
- Do not swap logical menubar left/right behavior without consulting direction and orientation.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the top-level strip remains mounted and focusable even when no nested menu is open.
- Consumers may assume active-menu switching and directional key handling are explicit adapter responsibilities.
- Consumers must not assume shortcut text implies real accelerator registration; it is decorative only.

## 20. Platform Support Matrix

| Capability / behavior                                 | Browser client | SSR            | Notes                                       |
| ----------------------------------------------------- | -------------- | -------------- | ------------------------------------------- |
| top-level menubar semantics and active-menu switching | full support   | full support   | Structural strip rendering is SSR-safe.     |
| directional focus movement and nested positioning     | client-only    | SSR-safe empty | Requires mounted nodes and event listeners. |
| shortcut-label decoration                             | full support   | full support   | Purely visual and locale-sensitive.         |

## 21. Debug Diagnostics and Production Policy

| Condition                                           | Debug build behavior | Production behavior | Notes                                                         |
| --------------------------------------------------- | -------------------- | ------------------- | ------------------------------------------------------------- |
| active menu key missing from top-level registration | fail fast            | fail fast           | Active-menu behavior depends on stable keyed top-level items. |
| nested menu resources leaked during menu switch     | debug warning        | warn and ignore     | Cleanup must re-converge on the next switch or close.         |

## 22. Shared Adapter Helper Notes

| Helper concept                | Required? | Responsibility                                                          | Reused by                        | Notes                                         |
| ----------------------------- | --------- | ----------------------------------------------------------------------- | -------------------------------- | --------------------------------------------- |
| top-level registration helper | required  | Track keyed top-level items for roving focus and active-menu switching. | `tabs`, `segment-group`          | Keep order aligned with rendering.            |
| positioning helper            | required  | Place the active nested menu relative to the active top-level item.     | `menu`, `select`, `context-menu` | Only one active nested menu at a time.        |
| direction helper              | required  | Normalize horizontal key behavior from direction and orientation.       | `segment-group`, `listbox`       | Consume environment direction when available. |

## 23. Framework-Specific Behavior

Leptos should keep top-level item registration keyed, derive directional behavior from current direction before dispatching transitions, and tear down nested-menu resources before switching to the next active menu.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn MenuBar(/* props */ children: Children) -> impl IntoView {
    let machine = use_machine::<menu_bar::Machine>(menu_bar::Props { /* ... */ });
    provide_context(Context::from_machine(machine));

    view! { <div {..machine.derive(|api| api.root_attrs()).get()}>{children()}</div> }
}
```

## 25. Reference Implementation Skeleton

Keep one machine, one top-level registration helper, one active nested-menu scope, and one direction-normalization path. Menu switching always cleans up the previous active scope before positioning the next one.

## 26. Adapter Invariants

- Top-level roving focus and active-menu identity remain machine-owned.
- Only one nested menu scope is active and positioned at a time.
- Shortcut text remains decorative and separate from behavior ownership.

## 27. Accessibility and SSR Notes

- The top-level strip keeps menubar semantics even when no nested menu is open.
- Nested menus must keep valid parent-child aria relationships and focus handoff when active.
- Direction-aware key handling must remain consistent with the documented horizontal menubar behavior.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter ownership of top-level roving focus, active-menu switching, and nested positioning.
- Intentional deviations: non-web nested positioning may use documented fallback placement rather than browser-specific coordinates.

## 29. Test Scenarios

1. Arrow keys move focus across top-level items and open the correct active menu.
2. Switching between top-level menus tears down the previous nested-menu scope before opening the next one.
3. Shortcut text stays decorative while nested action-item behavior follows the shared menu contract.
4. Closing the active menu returns focus to the correct top-level item.

## 30. Test Oracle Notes

- Preferred oracle for top-level behavior: `DOM attrs` on the menubar root and top-level items plus `machine state` for the active menu key.
- Preferred oracle for cleanup: `cleanup side effects` showing nested-menu resources are released during menu switches.
- Preferred oracle for direction behavior: `callback order` and DOM focus assertions for left/right or previous/next menu movement.

## 31. Implementation Checklist

- [ ] Top-level registration, active-menu switching, and nested positioning are explicit adapter contracts.
- [ ] Only one nested menu scope remains active at a time and cleanup ordering is documented.
- [ ] Direction behavior, focus return, and decorative shortcut text are covered in invariants and tests.
