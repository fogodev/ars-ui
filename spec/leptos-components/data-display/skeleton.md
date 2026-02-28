---
adapter: leptos
component: skeleton
category: data-display
source: components/data-display/skeleton.md
source_foundation: foundation/08-adapter-leptos.md
---

# Skeleton — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Skeleton`](../../components/data-display/skeleton.md) contract onto a Leptos 0.8.x component. The adapter keeps the component stateless while making animation ownership, reduced-motion fallback, CSS custom-property wiring, and accessible loading semantics explicit.

## 2. Public Adapter API

```rust
#[component]
pub fn Skeleton(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] count: Option<NonZero<u32>>,
    #[prop(optional)] variant: Option<skeleton::Variant>,
    #[prop(optional)] line_height: Option<String>,
    #[prop(optional)] gap: Option<String>,
    #[prop(optional)] circle_size: Option<String>,
    #[prop(optional)] animated: bool,
    #[prop(optional)] locale: Option<Locale>,
    #[prop(optional)] messages: Option<skeleton::Messages>,
) -> impl IntoView
```

All props are render-time inputs. Animation remains adapter-owned and is never surfaced as a machine state.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props`.
- Part parity: full parity for `Root`, optional `Circle`, and repeated `Item`.
- Traceability note: this spec promotes reduced-motion suppression, localized loading semantics, and custom-property wiring from the agnostic spec.

## 4. Part Mapping

| Core part | Required? | Adapter rendering target | Ownership     | Attr source             | Notes                                        |
| --------- | --------- | ------------------------ | ------------- | ----------------------- | -------------------------------------------- |
| `Root`    | required  | `<div>`                  | adapter-owned | `api.root_attrs()`      | Carries loading semantics and CSS variables. |
| `Circle`  | optional  | `<div>`                  | adapter-owned | `api.circle_attrs()`    | Decorative only.                             |
| `Item`    | repeated  | `<div>`                  | adapter-owned | `api.item_attrs(index)` | One per line placeholder.                    |

## 5. Attr Merge and Ownership Rules

- Core attrs include `role="status"`, `aria-busy`, `aria-label`, `data-ars-variant`, and skeleton custom properties.
- The adapter owns reduced-motion attr repair and final animation classes or styles.
- Consumer styles may decorate line height, gap, or shape, but must not drop loading semantics.
- `Circle` and `Item` remain `aria-hidden="true"` even when consumers decorate them.

## 6. Composition / Context Contract

`Skeleton` is standalone. It may resolve locale and messages from the nearest `ArsProvider`, but it does not publish or require adapter context.

## 7. Prop Sync and Event Mapping

- All props re-render directly.
- `animated=true` is advisory; the adapter suppresses motion when reduced-motion preferences require it.
- There are no user-driven events or machine transitions.

## 8. Registration and Cleanup Contract

No registration is required. Reduced-motion handling should rely on CSS media queries or a shared preference signal, not per-instance DOM listeners.

## 9. Ref and Node Contract

No live ref is required.

## 10. State Machine Boundary Rules

Skeleton has no state machine. Animation state must not be mirrored into mutable runtime state unless a shared reduced-motion service already provides it.

## 11. Callback Payload Contract

No public adapter callbacks are required.

## 12. Failure and Degradation Rules

| Condition                               | Policy             | Notes                                                                       |
| --------------------------------------- | ------------------ | --------------------------------------------------------------------------- |
| reduced motion requested by the user    | fallback path      | Suppress wave and shimmer motion and render static or minimal pulse output. |
| invalid `count` input from wrapper code | fail fast          | The adapter requires at least one item.                                     |
| unsupported animation strategy          | degrade gracefully | Render a static placeholder with preserved sizing.                          |

## 13. Identity and Key Policy

`Item` nodes use stable per-index identities from `0..count`. The number of items must be hydration-stable.

## 14. SSR and Client Boundary Rules

- SSR renders the final structural node count and semantic attrs.
- Reduced-motion CSS may change animation after hydration, but structure must remain stable.
- No browser-only effects are required for the base implementation.

## 15. Performance Constraints

- Prefer CSS-driven animation over per-frame JavaScript work.
- Compute repeated item attrs from index without allocating extra helpers per render.

## 16. Implementation Dependencies

| Dependency            | Required?   | Dependency type  | Why it must exist first                                        | Notes                             |
| --------------------- | ----------- | ---------------- | -------------------------------------------------------------- | --------------------------------- |
| style-strategy helper | recommended | styling contract | Applies custom properties and optional nonce/CSSOM strategies. | Shared across display components. |

## 17. Recommended Implementation Sequence

1. Resolve locale/messages and base props.
2. Derive root, optional circle, and repeated item attrs.
3. Apply reduced-motion policy.
4. Render the stable item list.

## 18. Anti-Patterns

- Do not animate via timers or requestAnimationFrame.
- Do not expose `Circle` or `Item` to assistive technology.
- Do not let `animated` override reduced-motion requirements.

## 19. Consumer Expectations and Guarantees

- Consumers may assume skeleton layout remains stable while content loads.
- Consumers may assume reduced-motion preferences take precedence over animated variants.
- Consumers must not assume animation is always present.

## 20. Platform Support Matrix

| Capability / behavior        | Browser client | SSR            | Notes                                                            |
| ---------------------------- | -------------- | -------------- | ---------------------------------------------------------------- |
| semantic loading placeholder | full support   | full support   | Attr-only semantics are server-safe.                             |
| animated pulse/wave/shimmer  | full support   | SSR-safe empty | Motion starts through CSS after mount or stylesheet application. |
| reduced-motion fallback      | fallback path  | SSR-safe empty | Resolved by media queries or shared preference helpers.          |

## 21. Debug Diagnostics and Production Policy

| Condition                                   | Debug build behavior | Production behavior | Notes                                          |
| ------------------------------------------- | -------------------- | ------------------- | ---------------------------------------------- |
| impossible count or malformed sizing tokens | fail fast            | warn and ignore     | Rendering may fall back to default dimensions. |
| animated wave/shimmer under reduced motion  | debug warning        | degrade gracefully  | Production suppresses the motion.              |

## 22. Shared Adapter Helper Notes

| Helper concept        | Required?   | Responsibility                           | Reused by                  | Notes                                                           |
| --------------------- | ----------- | ---------------------------------------- | -------------------------- | --------------------------------------------------------------- |
| reduced-motion helper | recommended | Centralize animation suppression policy. | animation-heavy components | Prefer shared CSS/media-query strategy over per-instance logic. |

## 23. Framework-Specific Behavior

Leptos 0.8.x can build the repeated item list with a simple range iterator because there is no interactive identity or descendant registration requirement.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Skeleton(count: NonZero<u32>) -> impl IntoView {
    let api = Memo::new(move |_| skeleton::Api::new(skeleton::Props {
        count,
        ..Default::default()
    }));

    view! {
        <div {..attr_map_to_leptos(api.get().root_attrs(), &use_style_strategy(), None).attrs}>
            <For
                each=move || (0..count.get())
                key=|index| *index
                children=move |index| {
                    view! { <div {..attr_map_to_leptos(api.get().item_attrs(index as usize), &use_style_strategy(), None).attrs} /> }
                }
            />
        </div>
    }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton is required; the canonical sketch already captures the stateless adapter path.

## 26. Adapter Invariants

- `Root` always exposes loading semantics.
- Decorative placeholder parts remain hidden from AT.
- Reduced motion always overrides sweeping animation variants.

## 27. Accessibility and SSR Notes

- Use a localized loading label on `Root`.
- Keep structure stable between SSR and hydration even when animation policy changes.
- Prefer static fallback over subtle animation when implementation confidence is low.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit reduced-motion handling.
- Intentional deviations: none beyond adapter-owned motion suppression and style-strategy wiring.

## 29. Test Scenarios

1. Skeleton renders the configured number of placeholder items.
2. Reduced-motion mode suppresses wave and shimmer animation.
3. Root always exposes loading semantics while child placeholders stay hidden.

## 30. Test Oracle Notes

- Preferred oracle for structure: DOM snapshot of `Root`, optional `Circle`, and repeated `Item`.
- Preferred oracle for accessibility: inspect `role`, `aria-busy`, and `aria-hidden` attrs.
- Verification recipe: render with and without reduced motion and confirm only animation styling changes.

## 31. Implementation Checklist

- [ ] `Root` carries localized loading semantics.
- [ ] Reduced motion overrides animated variants.
- [ ] `Circle` and `Item` are decorative only.
- [ ] Structural node count is hydration-stable.
- [ ] Tests cover structure, semantics, and reduced-motion fallback.
