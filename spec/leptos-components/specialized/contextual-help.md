---
adapter: leptos
component: contextual-help
category: specialized
source: components/specialized/contextual-help.md
source_foundation: foundation/08-adapter-leptos.md
---

# ContextualHelp — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ContextualHelp`](../../components/specialized/contextual-help.md) contract onto Leptos `0.8.x`. The adapter preserves trigger-plus-nonmodal-dialog behavior while explicitly documenting its composition over the popover adapter contract.

## 2. Public Adapter API

```rust
#[slot] pub struct ContextualHelpHeading { children: Children }
#[slot] pub struct ContextualHelpFooter { children: Children }

#[component]
pub fn ContextualHelp(
    #[prop(optional)] variant: ContextualHelpVariant,
    #[prop(optional)] placement: Placement,
    #[prop(optional)] offset: f64,
    #[prop(optional)] cross_offset: f64,
    #[slot] heading: ContextualHelpHeading,
    children: Children,
    #[slot] footer: Option<ContextualHelpFooter>,
) -> impl IntoView
```

The adapter renders the trigger, content, heading, body, optional footer, and dismiss button as one composed surface.

## 3. Mapping to Core Component Contract

- Props parity: full parity with variant, placement, offsets, and slotted heading/body/footer content.
- Part parity: full parity with `Root`, `Trigger`, `Content`, `Heading`, `Body`, `Footer`, and `DismissButton`.
- Adapter additions: explicit composition over a non-modal popover instance.

## 4. Part Mapping

| Core part / structure         | Required?                              | Adapter rendering target    | Ownership     | Attr source                                | Notes                                  |
| ----------------------------- | -------------------------------------- | --------------------------- | ------------- | ------------------------------------------ | -------------------------------------- |
| `Root`                        | required                               | fragment or wrapper `<div>` | adapter-owned | contextual-help attrs plus popover context | root may be structural only            |
| `Trigger`                     | required                               | `<button>`                  | adapter-owned | `api.trigger_attrs()`                      | help or info icon trigger              |
| `Content`                     | required                               | `<div>`                     | adapter-owned | `api.content_attrs()`                      | non-modal dialog content               |
| `Heading` / `Body` / `Footer` | heading/body required, footer optional | `<div>` or heading element  | mixed         | derived attrs plus slotted children        | slotted text content is consumer-owned |
| `DismissButton`               | required                               | `<button>`                  | adapter-owned | `api.dismiss_button_attrs()`               | visually hidden close affordance       |

## 5. Attr Merge and Ownership Rules

Trigger popup semantics, content dialog semantics, and dismiss-button labeling always win. Consumer slots own text content only, not popup attrs or ids.

## 6. Composition / Context Contract

`ContextualHelp` internally composes the popover adapter in non-modal mode. Consumers do not receive direct popover context and must not render help parts outside the root component.

## 7. Prop Sync and Event Mapping

Variant and placement props rebuild the composed popover props. Trigger activation toggles the popover. Escape and dismiss-button activation close the content and restore focus to the trigger.

## 8. Registration and Cleanup Contract

Popover-owned outside-interaction handlers, focus restoration hooks, and positioning resources are registered through the underlying popover instance and must be cleaned up with it.

## 9. Ref and Node Contract

Trigger and content own the live handles required by the composed popover behavior. Consumers do not own these refs through the base surface.

## 10. State Machine Boundary Rules

- machine-owned state: open/closed popover state and focus return behavior via the composed popover machine.
- adapter-local derived bookkeeping: slot presence and consumer children only.
- forbidden local mirrors: do not duplicate open state outside the composed machine.

## 11. Callback Payload Contract

No dedicated callback is required; open and close behavior is embodied in the rendered help surface.

## 12. Failure and Degradation Rules

If positioning helpers are unavailable, degrade gracefully by rendering content adjacent to the trigger while preserving dialog labeling and dismiss behavior.

## 13. Identity and Key Policy

Trigger-content id linkage is instance-scoped and must remain stable for the lifetime of the help surface.

## 14. SSR and Client Boundary Rules

SSR renders the trigger and closed-state structural content branch as required by the composed popover contract. Positioning and outside-interaction handlers are client-only.

## 15. Performance Constraints

Do not re-create the composed popover instance on every render when only slot content changes. Reuse stable ids.

## 16. Implementation Dependencies

| Dependency               | Required? | Dependency type        | Why it must exist first                                          | Notes          |
| ------------------------ | --------- | ---------------------- | ---------------------------------------------------------------- | -------------- |
| popover adapter contract | required  | composition dependency | contextual-help delegates open state, dismissal, and positioning | non-modal only |

## 17. Recommended Implementation Sequence

1. Build the non-modal popover props.
2. Render trigger and composed content.
3. Add heading/body/footer slot plumbing.
4. Verify dismiss and focus-return behavior.

## 18. Anti-Patterns

- Do not fork a second open-state store outside the composed popover machine.
- Do not make the help content modal.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the trigger controls a non-modal dialog.
- Consumers may assume footer content is optional.
- Consumers must not assume arbitrary popover-part access outside this component.

## 20. Platform Support Matrix

| Capability / behavior               | Browser client | SSR          | Notes                                                   |
| ----------------------------------- | -------------- | ------------ | ------------------------------------------------------- |
| trigger and content semantics       | full support   | full support | structural parity through the composed popover contract |
| positioning and outside interaction | full support   | client-only  | runtime popover helpers only                            |

## 21. Debug Diagnostics and Production Policy

Missing composed popover context is fail-fast. Positioning fallback is a debug warning and graceful inline content path.

## 22. Shared Adapter Helper Notes

`ContextualHelp` should reuse the shared popover adapter and any positioning helper taxonomy already documented in the overlay tree.

## 23. Framework-Specific Behavior

Leptos should derive trigger and content attrs reactively from the composed popover machine rather than using non-reactive snapshots for open-state rendering.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn ContextualHelp(heading: ContextualHelpHeading, children: Children) -> impl IntoView {
    view! { <button /> }
}
```

## 25. Reference Implementation Skeleton

- Create one non-modal popover machine.
- Derive trigger, content, and dismiss attrs from that machine.
- Render slot content into the documented structural parts only.

## 26. Adapter Invariants

- The underlying popover stays non-modal.
- The trigger remains the focus-return target.
- Heading, body, and footer content never own popup semantics themselves.

## 27. Accessibility and SSR Notes

The content remains `role="dialog"` and is labeled by the heading slot. The dismiss button remains available for assistive technology even when visually hidden.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- trigger open/close behavior
- Escape and dismiss button close behavior
- heading labels content correctly
- positioning fallback keeps semantics intact

## 30. Test Oracle Notes

| Behavior          | Preferred oracle type | Notes                                               |
| ----------------- | --------------------- | --------------------------------------------------- |
| dialog semantics  | DOM attrs             | assert trigger popup attrs and content dialog attrs |
| focus restoration | keyboard interaction  | assert close returns focus to trigger               |
| slot structure    | rendered structure    | assert heading/body/footer land in documented parts |

## 31. Implementation Checklist

- [ ] Open state is delegated to one non-modal popover machine.
- [ ] Heading labels the content.
- [ ] Dismiss and focus-return behavior are verified.
