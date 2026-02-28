---
adapter: leptos
component: button
category: utility
source: components/utility/button.md
source_foundation: foundation/08-adapter-leptos.md
---

# Button — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Button`](../../components/utility/button.md) contract onto a Leptos 0.8.x component. The adapter must preserve all three core parts, native button semantics, loading behavior, and root reassignment when `as_child=true`.

## 2. Public Adapter API

```rust
#[component]
pub fn Button(
    #[prop(optional)] id: Option<String>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] loading: Signal<bool>,
    #[prop(optional)] variant: Option<String>,
    #[prop(optional)] size: Option<String>,
    #[prop(optional)] r#type: Option<button::Type>,
    #[prop(optional)] form: Option<String>,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] value: Option<String>,
    #[prop(optional)] as_child: bool,
    #[prop(optional)] exclude_from_tab_order: bool,
    #[prop(optional)] auto_focus: bool,
    #[prop(optional)] prevent_focus_on_press: bool,
    children: Children,
) -> impl IntoView
```

The adapter surfaces the full core prop set. `disabled` and `loading` are the common reactive inputs; all other props may be plain Leptos values unless a wrapper makes them reactive.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props`, including form overrides, locale/messages, and `as_child`.
- Event parity: `Focus`, `Blur`, `Press`, `Release`, `Click`, `SetLoading`, and `SetDisabled` are all adapter-driven.
- Core machine ownership: `use_machine::<button::Machine>(...)` remains the single source of truth for state and attrs.

## 4. Part Mapping

| Core part / structure | Required?   | Adapter rendering target                                   | Ownership                                                                                                                  | Attr source                                                   | Notes                                                                             |
| --------------------- | ----------- | ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `Root`                | required    | `<button>` by default; consumer child when `as_child=true` | adapter-owned by default; consumer-owned under root reassignment                                                           | `api.root_attrs()`                                            | The core `Root` never disappears; only the rendering target changes.              |
| `LoadingIndicator`    | conditional | `<span>` inside `Root` while `api.is_loading()`            | adapter-owned                                                                                                              | `api.loading_indicator_attrs()`                               | Must stay `aria-hidden="true"`.                                                   |
| `Content`             | required    | `<span>` wrapping the visible label/icon slot              | adapter-owned by default; may be suppressed only if a documented `as_child` strategy merges directly into a consumer child | adapter-owned structural attrs plus `data-ars-part="content"` | The core `Part::Content` exists even though `part_attrs(Part::Content)` is empty. |

## 5. Attr Merge and Ownership Rules

| Target node        | Core attrs                                                                    | Adapter-owned attrs                                                              | Consumer attrs                                                                         | Merge order                                                                                                                                                                                                                            | Ownership notes                                                                                             |
| ------------------ | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `Root`             | `api.root_attrs()` including state, ARIA, `type`, `form`, and tab-order attrs | pointer-modality bookkeeping hooks and any adapter-local `data-ars-part` helpers | hosting-component root attrs; child attrs when `as_child=true`                         | core required state/ARIA attrs win; native attrs required by the core contract win; `class`/`style` merge additively; handlers compose adapter after child for observation and adapter before child when preventing invalid activation | adapter-owned by default; consumer-owned only after root reassignment under `as_child`                      |
| `LoadingIndicator` | `api.loading_indicator_attrs()`                                               | none beyond structural wrapper choice                                            | no direct consumer attrs unless the hosting component exposes a dedicated loading slot | core attrs apply as-is; consumer decoration must not remove `aria-hidden`                                                                                                                                                              | always adapter-owned when rendered                                                                          |
| `Content`          | no core attr map beyond conceptual part identity                              | `data-ars-part="content"` and any wrapper-only attrs                             | consumer children content only                                                         | adapter structural attrs always remain; consumer classes decorate content inside the wrapper rather than replacing it                                                                                                                  | wrapper is adapter-owned unless a documented `as_child` strategy folds the wrapper into the reassigned root |

- `id`, `role`, `tabindex`, `aria-*`, `data-*`, `type`, `name`, `value`, and `form` must preserve the core contract even when consumer attrs are present.
- `class` and `style` are additive unless the hosting component explicitly declares a non-additive escape hatch.
- Under `as_child`, root reassignment changes rendered-node ownership only; it does not allow the consumer child to drop core accessibility or state attrs.

## 6. Composition / Context Contract

`Button` is standalone. When `as_child=true`, the adapter reassigns `Root` to the single consumer child and must document whether `Content` remains a wrapper or is folded into the child structure. No other contexts are required.

## 7. Prop Sync and Event Mapping

Controlled/uncontrolled switching is not supported after mount. `disabled` and `loading` are controlled reactive inputs; all default-only values are read at initialization unless a higher-level wrapper documents additional sync.

| Adapter prop             | Mode                      | Sync trigger                      | Machine event / update path                              | Visible effect                                                            | Notes                                                                              |
| ------------------------ | ------------------------- | --------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `disabled`               | controlled                | signal or prop change after mount | `SetDisabled`                                            | updates focusability, disabled semantics, and blocked activation behavior | sync is immediate and effect-based                                                 |
| `loading`                | controlled                | signal or prop change after mount | `SetLoading`                                             | toggles loading indicator, busy state, and blocked activation behavior    | sync is immediate and effect-based                                                 |
| `prevent_focus_on_press` | controlled                | signal or prop change after mount | adapter reads latest value before pointer press handling | affects whether `pointerdown` prevents focus movement                     | no separate machine event unless the core machine models it directly               |
| `type`                   | non-reactive adapter prop | render time only                  | included in root props passed to the machine             | controls native submit/reset/button behavior                              | post-mount changes should be treated as unsupported unless a wrapper reinitializes |
| `form`                   | non-reactive adapter prop | render time only                  | included in root props passed to the machine             | binds the button to the target form owner                                 | post-mount changes should be treated as unsupported unless a wrapper reinitializes |

| UI event                      | Preconditions                                          | Machine event / callback path              | Ordering notes                                                                                       | Notes                                                         |
| ----------------------------- | ------------------------------------------------------ | ------------------------------------------ | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `pointerdown`                 | root interactive and not core-disabled                 | `Press`; optionally prevent focus          | runs before consumer click callbacks; may short-circuit focus when `prevent_focus_on_press=true`     | establishes pointer modality                                  |
| `pointerup`                   | matching active press                                  | `Release`                                  | must preserve core release ordering relative to blur                                                 | no-op when press was suppressed                               |
| `focus`                       | root receives focus                                    | `Focus { is_keyboard }`                    | computed after pointer-modality normalization                                                        | keyboard focus must remain distinguishable from pointer focus |
| `blur`                        | root loses focus                                       | `Blur`                                     | must occur before any late release cleanup in the same interaction drain                             | preserves focus-visible correctness                           |
| `click`                       | native activation path not blocked by disabled/loading | `Click` and any public activation callback | activation callback fires after normalized machine transition unless explicitly documented otherwise | native `<button>` Enter behavior flows through this path      |
| `keydown` / `keyup` for Space | only when root is not a native `<button>`              | `Press` / `Release`                        | must not duplicate native click synthesis                                                            | native buttons rely on browser behavior instead               |

## 8. Registration and Cleanup Contract

- No compound child registration exists beyond normal ownership of `LoadingIndicator` and `Content`.
- Pointer-modality bookkeeping is local adapter state and must be discarded on cleanup.
- Any temporary prevention state used for `prevent_focus_on_press` must not outlive the active interaction.

| Registered entity      | Registration trigger       | Identity key       | Cleanup trigger                      | Cleanup action                           | Notes                                                |
| ---------------------- | -------------------------- | ------------------ | ------------------------------------ | ---------------------------------------- | ---------------------------------------------------- |
| pointer modality flag  | first interactive render   | component instance | component cleanup                    | discard stored pointer-vs-keyboard state | purely local, no DOM registration                    |
| loading indicator node | `loading=true` render path | component instance | `loading=false` or component cleanup | remove structural loading node           | no stale `aria-busy` or indicator wrapper may remain |

## 9. Ref and Node Contract

| Target part / node | Ref required?                                                                   | Ref owner                                                                   | Node availability                  | Composition rule                                               | Notes                                                                    |
| ------------------ | ------------------------------------------------------------------------------- | --------------------------------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `Root`             | yes for focus management, press normalization, and `as_child` root reassignment | adapter-owned by default; composed with the consumer child under `as_child` | required after mount               | compose adapter ref with the consumer ref when `as_child=true` | The adapter may not rely on IDs alone for focus and activation behavior. |
| `LoadingIndicator` | no                                                                              | adapter-owned                                                               | always structural, handle optional | no composition                                                 | Structural only.                                                         |
| `Content` wrapper  | no                                                                              | adapter-owned unless folded into a documented `as_child` strategy           | always structural, handle optional | no composition unless root reassignment eliminates the wrapper | The content wrapper is not the interaction target.                       |

## 10. State Machine Boundary Rules

- machine-owned state: disabled, loading, pressed, focus-visible, busy semantics, and the emitted root/loading attrs.
- adapter-local derived bookkeeping: pointer-versus-keyboard modality tracking and temporary `prevent_focus_on_press` guard state.
- forbidden local mirrors: do not mirror loading, disabled, or pressed state in local signals that can diverge from machine events.
- allowed snapshot-read contexts: render derivation, pointer and focus event handlers, and cleanup for ephemeral pointer bookkeeping only.

## 11. Callback Payload Contract

| Callback                                              | Payload source             | Payload shape                                                               | Timing                                                  | Cancelable? | Notes                                                           |
| ----------------------------------------------------- | -------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------- | ----------- | --------------------------------------------------------------- |
| activation / click callback when exposed by a wrapper | normalized adapter payload | `{ original_event?: framework event, is_keyboard: bool, is_loading: bool }` | after normalized machine transition to click/activation | no          | Native `<button>` Enter and click semantics must converge here. |
| press-start / press-end callback when exposed         | normalized adapter payload | `{ pointer_type?: string, is_keyboard: bool }`                              | after `Press` / after `Release` respectively            | no          | Must reflect deduplicated native button behavior.               |

## 12. Failure and Degradation Rules

| Condition                                                                | Policy    | Notes                                                      |
| ------------------------------------------------------------------------ | --------- | ---------------------------------------------------------- |
| `as_child` receives zero or multiple children                            | fail fast | Root reassignment requires exactly one consumer child.     |
| root node handle unavailable after mount in an interactive configuration | fail fast | Press/focus normalization depends on a concrete root node. |
| browser-only activation details unavailable during SSR                   | no-op     | SSR renders structure only; no interactive behavior runs.  |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                            | Notes                                                                               |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| root button instance             | instance-derived | not applicable      | not applicable                           | root structure and part identity must stay stable across hydration | `Root`, `LoadingIndicator`, and `Content` identities belong to one button instance. |

## 14. SSR and Client Boundary Rules

- SSR must render the same `Root` / `LoadingIndicator?` / `Content` structure implied by the initial machine state.
- The root node handle is server-safe absent and becomes required after mount for interaction logic.
- Pointer, focus, and keyboard normalization are client-only behaviors.
- `as_child` must not change the server/client child count across hydration.

## 15. Performance Constraints

- Root attrs and loading-indicator attrs must be derived or memoized, not rebuilt eagerly from ad hoc logic every render.
- Do not attach duplicate Space-key handlers to native `<button>` roots.
- Pointer-modality bookkeeping must stay instance-local and must not allocate global listeners.
- Loading-indicator insertion/removal should only follow machine state changes, not independent wrapper bookkeeping.

## 16. Implementation Dependencies

| Dependency | Required? | Dependency type      | Why it must exist first                                            | Notes                                     |
| ---------- | --------- | -------------------- | ------------------------------------------------------------------ | ----------------------------------------- |
| `as-child` | required  | composition contract | Root reassignment depends on the shared child-forwarding contract. | Needed before supporting `as_child=true`. |

## 17. Recommended Implementation Sequence

1. Initialize the button machine and confirm the `Root`, `LoadingIndicator`, and `Content` structure.
2. Establish the root ref strategy, including composed refs under `as_child`.
3. Derive root/loading/content attrs and render the documented structure.
4. Wire controlled sync for `disabled` and `loading`.
5. Normalize pointer, focus, click, and keyboard events.
6. Add callback wiring, SSR guards, and cleanup for pointer-modality bookkeeping.

## 18. Anti-Patterns

- Do not attach Space-key handlers to native `<button>` roots that already synthesize click.
- Do not drop `Content` just because its core attr map is empty.
- Do not use HTML `disabled` to represent loading semantics when the core contract requires `aria-disabled` and `aria-busy`.

## 19. Consumer Expectations and Guarantees

- Consumers may assume documented adapter-owned structural nodes and attrs remain the canonical implementation surface.
- Consumers may assume framework-specific divergence is called out explicitly rather than hidden in generic prose.
- Consumers must not assume unspecified fallback behavior, cleanup ordering, or helper ownership beyond what this adapter spec documents.

## 20. Platform Support Matrix

| Capability / behavior                     | Browser client | SSR          | Notes                                                                                         |
| ----------------------------------------- | -------------- | ------------ | --------------------------------------------------------------------------------------------- |
| documented structural and state semantics | full support   | full support | This utility does not have additional platform variance beyond its existing SSR/client rules. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                          | Debug build behavior | Production behavior | Notes                                                                                   |
| ------------------------------------------------------------------ | -------------------- | ------------------- | --------------------------------------------------------------------------------------- |
| native button receives redundant custom keyboard activation wiring | debug warning        | warn and ignore     | The adapter should surface the mismatch without double-firing activation in production. |
| invalid `as_child` child count                                     | fail fast            | fail fast           | Root reassignment requires exactly one consumer child.                                  |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required?      | Responsibility                                                                          | Reused by      | Notes                                                         |
| ----------------- | -------------- | --------------------------------------------------------------------------------------- | -------------- | ------------------------------------------------------------- |
| attr merge helper | not applicable | No special helper beyond the documented attr derivation and merge contract is required. | not applicable | Use the normal machine attr derivation path for this utility. |

## 23. Framework-Specific Behavior

Leptos 0.8.x allows the adapter to keep root attrs in a memo and spread them into the rendered node. `as_child` requires an adapter-local helper because Leptos cannot arbitrarily mutate an opaque child vnode. Optional parent context, when present in wrappers, should be read via `use_context::<T>()`.

## 24. Canonical Implementation Sketch

```rust
use leptos::prelude::*;

#[component]
pub fn Button(
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] loading: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let props = button::Props {
        disabled: disabled.get(),
        loading: loading.get(),
        ..Default::default()
    };

    let machine = use_machine::<button::Machine>(props);
    let root_attrs = machine.derive(|api| api.root_attrs());
    let loading_attrs = machine.derive(|api| api.loading_indicator_attrs());
    let is_loading = machine.derive(|api| api.is_loading());

    let last_pointer = StoredValue::new(false);

    view! {
        <button
            {..root_attrs.get()}
            on:pointerdown=move |ev| {
                last_pointer.set_value(true);
                machine.send.run(button::Event::Press);
                if machine.with_api_snapshot(|api| api.should_prevent_focus_on_press()) {
                    ev.prevent_default();
                }
            }
            on:pointerup=move |_| machine.send.run(button::Event::Release)
            on:focus=move |_| {
                let is_keyboard = !last_pointer.get_value();
                last_pointer.set_value(false);
                machine.send.run(button::Event::Focus { is_keyboard });
            }
            on:blur=move |_| machine.send.run(button::Event::Blur)
            on:click=move |_| machine.send.run(button::Event::Click)
        >
            {move || {
                if is_loading.get() {
                    view! { <span {..loading_attrs.get()} /> }.into_any()
                } else {
                    ().into_any()
                }
            }}
            <span data-ars-part="content">
                {children()}
            </span>
        </button>
    }
}
```

For native `<button>`, the adapter must deduplicate Space-key handlers as required by the core accessibility contract.

## 25. Reference Implementation Skeleton

```rust
// Pseudo-Rust: keep machine state authoritative and layer helpers around it.
let machine = use_machine::<button::Machine>(props);
let root_ref = create_root_ref();
let root_attrs = derive_root_attrs(machine);
let content_attrs = derive_content_attrs(machine);
let loading_attrs = derive_loading_attrs(machine);

publish_required_contexts_if_any();
attach_root_ref(root_ref);
sync_controlled_props(machine, props.disabled, props.loading);
wire_press_focus_and_click_normalization(root_ref, machine);

render_root_with_optional_as_child(root_attrs, {
    render_loading_indicator_if_needed(loading_attrs);
    render_content_wrapper(content_attrs);
});

on_cleanup(|| release_pointer_or_focus_bookkeeping());
```

## 26. Adapter Invariants

- Native `<button>` rendering must not attach Space-key handlers that duplicate native click synthesis.
- The adapter must preserve the core blur and release ordering so pointer or keyboard cleanup does not race focus updates.
- When `loading=true`, the adapter must block native submit and reset activation without relying on HTML `disabled`.
- Loading state must preserve accessibility exposure through core disabled semantics rather than removing the control from discovery.
- `Root` must remain conceptually present under `as_child`; only the rendering target changes through root reassignment.
- `Content` must remain a documented structural node even when the core content attr map is empty.
- `LoadingIndicator` must stay structurally distinct and `aria-hidden` whenever it is rendered.
- Callbacks must follow normalized press and activation semantics rather than raw DOM event order.

## 27. Accessibility and SSR Notes

- `LoadingIndicator` is decorative and must remain `aria-hidden`.
- `Content` is part of the accessible name unless overridden by `aria-label` or `aria-labelledby`.
- Loading uses `aria-disabled="true"` and `aria-busy="true"` instead of the HTML `disabled` attribute.
- SSR must preserve hydration-safe IDs and initial loading state.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, part, and event parity.

Intentional deviations: none. If an adapter-local `as_child` helper folds `Content` differently, that difference must remain purely structural and not delete the conceptual part.

## 29. Test Scenarios

- `Root`, `LoadingIndicator`, and `Content` all appear in the documented structure
- default native button rendering
- root reassignment under `as_child`
- loading state renders `LoadingIndicator` and preserves tab discoverability
- native button Space-key deduplication
- `prevent_focus_on_press` suppression
- loading submit/reset prevention

## 30. Test Oracle Notes

| Behavior                                | Preferred oracle type | Notes                                                                                          |
| --------------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------- |
| loading and disabled semantics          | DOM attrs             | Assert `aria-disabled`, `aria-busy`, and root state attrs on the actual root node.             |
| press/click normalization               | callback order        | Verify `Press`/`Release`/activation ordering, especially for native button Space-key behavior. |
| loading indicator and content structure | rendered structure    | Assert the presence and identity of `LoadingIndicator` and `Content` separately.               |

## 31. Implementation Checklist

- [ ] Root attrs and ref ownership are wired correctly, including `as_child` composition.
- [ ] `LoadingIndicator` and `Content` are rendered as distinct documented structures.
- [ ] Controlled sync for `disabled` and `loading` is verified.
- [ ] Pointer, focus, click, and keyboard normalization matches the documented callback order.
- [ ] SSR preserves the same root/loading/content structure.
