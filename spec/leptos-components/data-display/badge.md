---
adapter: leptos
component: badge
category: data-display
source: components/data-display/badge.md
source_foundation: foundation/08-adapter-leptos.md
---

# Badge — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Badge`](../../components/data-display/badge.md) contract onto a Leptos 0.8.x component. The adapter preserves the single `Root` part, keeps the host inline by default, and makes dynamic-status semantics, locale count formatting, and decorative hiding explicit.

## 2. Public Adapter API

```rust
#[component]
pub fn Badge(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] variant: Option<badge::Variant>,
    #[prop(optional)] size: Option<badge::Size>,
    #[prop(optional)] content: Option<String>,
    #[prop(optional)] dynamic: bool,
    #[prop(optional)] decorative: bool,
    #[prop(optional)] aria_label: Option<String>,
    #[prop(optional)] locale: Option<Locale>,
    #[prop(optional)] messages: Option<badge::Messages>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView
```

`content` is the primary adapter input. `children` is decoration-only sugar and must not bypass the single-root semantics or locale-label rules.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props`, plus adapter-facing `aria_label` and `decorative` conveniences.
- Part parity: full parity; the adapter renders only `Root`.
- Traceability note: this spec promotes dynamic `role="status"` selection, count-label requirements, and semantic hiding rules that are adapter-owned in the agnostic spec.

## 4. Part Mapping

| Core part | Required? | Adapter rendering target | Ownership     | Attr source        | Notes                                         |
| --------- | --------- | ------------------------ | ------------- | ------------------ | --------------------------------------------- |
| `Root`    | required  | `<span>`                 | adapter-owned | `api.root_attrs()` | The badge remains inline and non-interactive. |

## 5. Attr Merge and Ownership Rules

- Core attrs come from `api.root_attrs()`, including `data-ars-variant`, `data-ars-size`, and dynamic `role` or `aria-live`.
- The adapter owns `aria-label`, `aria-hidden`, and any semantic repair needed for decorative or dynamic badges.
- Consumer `class`, `style`, and test IDs merge additively. Consumer attrs must not drop required `role`, `aria-live`, or data attrs.
- `children` content decorates the inside of `Root`; consumers do not replace the root host element.

## 6. Composition / Context Contract

`Badge` is standalone. It may resolve locale and messages from the nearest `ArsProvider`, but it does not publish or require adapter context.

## 7. Prop Sync and Event Mapping

- `content`, `dynamic`, `decorative`, `variant`, and `size` re-render directly from props.
- Locale and messages re-resolve when their inputs change.
- There are no machine events or adapter-owned UI event mappings for the base badge.

## 8. Registration and Cleanup Contract

No registration is required. The adapter must not allocate auxiliary announcer nodes or observers for `Badge`.

## 9. Ref and Node Contract

No live ref is required. The root node is structural only.

## 10. State Machine Boundary Rules

Badge has no state machine. All state is render-derived from props and resolved locale data.

## 11. Callback Payload Contract

No adapter callback is required for the base component.

## 12. Failure and Degradation Rules

| Condition                                                           | Policy             | Notes                                                                                    |
| ------------------------------------------------------------------- | ------------------ | ---------------------------------------------------------------------------------------- |
| `dynamic=true` without an accessible label for count/status meaning | debug warning      | Rendering still succeeds, but development builds should surface the missing description. |
| `content` and `children` both absent                                | degrade gracefully | Render an empty badge root so layout remains stable.                                     |
| consumer attempts interactive behavior on the root                  | warn and ignore    | Badge semantics remain non-interactive; wrappers should own interactivity.               |

## 13. Identity and Key Policy

The component owns one stable root node. Hydration identity is derived from the component instance and must not change with count updates.

## 14. SSR and Client Boundary Rules

- SSR and hydration render the same `<span>` structure.
- Dynamic status semantics are server-safe because they are attr-only.
- No client-only effects are required.

## 15. Performance Constraints

- Count formatting should be memoized from `content`, locale, and message inputs.
- The adapter must not create live-region helper nodes for every badge instance.

## 16. Implementation Dependencies

| Dependency                        | Required? | Dependency type     | Why it must exist first                            | Notes                                                                                                                            |
| --------------------------------- | --------- | ------------------- | -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `ArsProvider` locale/i18n context | optional  | formatting contract | Resolves localized count text and overflow labels. | Explicit props win; otherwise formatting falls back to the nearest `ArsProvider`, then the documented foundation default locale. |

## 17. Recommended Implementation Sequence

1. Resolve locale and messages.
2. Derive root attrs from the core API.
3. Repair semantics for `dynamic` or `decorative`.
4. Render the root span with formatted content.

## 18. Anti-Patterns

- Do not render the root as `<div>` or `<button>`.
- Do not rely on numeric text alone to communicate meaning.
- Do not use `children` to bypass `aria-hidden` or `role="status"` decisions.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the root remains an inline non-interactive host.
- Consumers may assume dynamic badges announce changes only when the spec says they should.
- Consumers must not assume arbitrary root reassignment or machine-managed interaction state.

## 20. Platform Support Matrix

| Capability / behavior                        | Browser client | SSR          | Notes                                  |
| -------------------------------------------- | -------------- | ------------ | -------------------------------------- |
| inline badge semantics and locale formatting | full support   | full support | No client-only capability is required. |

## 21. Debug Diagnostics and Production Policy

| Condition                                      | Debug build behavior | Production behavior | Notes                                                             |
| ---------------------------------------------- | -------------------- | ------------------- | ----------------------------------------------------------------- |
| dynamic badge missing accessible label context | debug warning        | warn and ignore     | Missing context is an accessibility defect, not a render blocker. |
| consumer passes interactive attrs to root      | debug warning        | warn and ignore     | Interactive wrappers should own click and keyboard behavior.      |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required?   | Responsibility                                  | Reused by       | Notes                           |
| ----------------- | ----------- | ----------------------------------------------- | --------------- | ------------------------------- |
| formatting helper | recommended | Format overflow counts and locale-aware labels. | `stat`, `meter` | Keep it pure and memo-friendly. |

## 23. Framework-Specific Behavior

Leptos 0.8.x can derive the final root attr map in a memo and spread it directly into the `<span>`. Common locale reads should flow through `use_locale()` or `t()` from `08-adapter-leptos.md` §13; raw context access should use `use_context::<ArsContext>()` only when the full environment is actually needed.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Badge(content: Option<String>, dynamic: bool) -> impl IntoView {
    let api = Memo::new(move |_| badge::Api::new(badge::Props {
        content: content.clone(),
        dynamic,
        ..Default::default()
    }));
    let root_attrs = move || api.get().root_attrs();

    view! {
        <span {..attr_map_to_leptos(root_attrs(), &use_style_strategy(), None).attrs}>
            {content.unwrap_or_default()}
        </span>
    }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton is required; the canonical sketch already captures the full adapter shape for this stateless component.

## 26. Adapter Invariants

- The root remains a `<span>`.
- `dynamic=true` preserves `role="status"` and `aria-live="polite"`.
- Decorative badges set `aria-hidden="true"` and do not simultaneously expose a conflicting live-region role.

## 27. Accessibility and SSR Notes

- Prefer decorative hiding for badges that duplicate surrounding text.
- Dynamic counts should supply an accessible label that explains what changed, not just the new number.
- SSR output must not differ from hydrated output for semantic attrs.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter semantics.
- Intentional deviations: the adapter adds an `aria_label` convenience so localized count descriptions are easy to supply without mutating core `Props`.

## 29. Test Scenarios

1. Dynamic badge renders `role="status"` and `aria-live="polite"` when the count changes.
2. Decorative badge hides itself from assistive technology while preserving layout.
3. Locale-formatted overflow text remains stable between SSR and hydration.

## 30. Test Oracle Notes

- Preferred oracle for dynamic semantics: inspect rendered attrs on `Root`.
- Preferred oracle for decorative behavior: accessibility tree snapshot should omit the badge.
- Verification recipe: rerender with a new locale and confirm formatted content updates without changing root identity.

## 31. Implementation Checklist

- [ ] Root is always `<span>`.
- [ ] Dynamic semantics come from adapter-owned attr repair, not ad hoc consumer markup.
- [ ] Decorative hiding and dynamic status are mutually coherent.
- [ ] Locale formatting is memoized and hydration-safe.
- [ ] Tests cover dynamic, decorative, and locale-formatted cases.
