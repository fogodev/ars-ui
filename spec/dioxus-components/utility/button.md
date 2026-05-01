---
adapter: dioxus
component: button
category: utility
source: components/utility/button.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Button — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Button`](../../components/utility/button.md) contract onto a Dioxus 0.7.x component. The adapter must preserve all three core parts, native button semantics, loading behavior, and root reassignment when `as_child=true`.

## 2. Public Adapter API

```rust,no_check
pub use ars_components::utility::button::{FormEncType, FormMethod, FormTarget, Size, Type, Variant};
pub use ars_core::{SafeUrl, UnsafeUrlError};
pub use ars_interactions::{PressEvent, PressEventType};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormAction(Option<SafeUrl>);

impl From<SafeUrl> for FormAction { /* ... */ }
impl From<&'static str> for FormAction { /* ... */ }

#[derive(Props, Clone, PartialEq)]
pub struct ButtonProps {
    #[props(optional, into)]
    pub id: Option<String>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub loading: bool,
    #[props(optional, into)]
    pub variant: Option<button::Variant>,
    #[props(optional, into)]
    pub size: Option<button::Size>,
    #[props(optional, into)]
    pub r#type: Option<button::Type>,
    #[props(optional, into)]
    pub form: Option<String>,
    #[props(optional, into)]
    pub name: Option<String>,
    #[props(optional, into)]
    pub value: Option<String>,
    #[props(default, into)]
    pub form_action: FormAction,
    #[props(optional, into)]
    pub form_method: Option<button::FormMethod>,
    #[props(optional, into)]
    pub form_enc_type: Option<button::FormEncType>,
    #[props(optional, into)]
    pub form_target: Option<button::FormTarget>,
    #[props(default = false)]
    pub form_no_validate: bool,
    #[props(default = false)]
    pub exclude_from_tab_order: bool,
    #[props(default = false)]
    pub auto_focus: bool,
    #[props(default = false)]
    pub prevent_focus_on_press: bool,
    #[props(optional, into)]
    pub class: Option<String>,
    #[props(optional, into)]
    pub style: Option<String>,
    #[props(optional, into)]
    pub aria_label: Option<String>,
    #[props(optional, into)]
    pub aria_labelledby: Option<String>,
    #[props(optional)]
    pub on_press_start: Option<Callback<PressEvent>>,
    #[props(optional)]
    pub on_press_end: Option<Callback<PressEvent>>,
    #[props(optional)]
    pub on_press: Option<Callback<PressEvent>>,
    #[props(optional)]
    pub on_press_change: Option<Callback<bool>>,
    #[props(optional)]
    pub on_press_up: Option<Callback<PressEvent>>,
    pub children: Element,
}

#[component]
pub fn Button(props: ButtonProps) -> Element

#[derive(Props, Clone, PartialEq)]
pub struct ButtonAsChildProps {
    #[props(optional, into)]
    pub id: Option<String>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub loading: bool,
    #[props(optional, into)]
    pub variant: Option<button::Variant>,
    #[props(optional, into)]
    pub size: Option<button::Size>,
    #[props(default = false)]
    pub exclude_from_tab_order: bool,
    #[props(optional, into)]
    pub class: Option<String>,
    #[props(optional, into)]
    pub style: Option<String>,
    #[props(optional, into)]
    pub aria_label: Option<String>,
    #[props(optional, into)]
    pub aria_labelledby: Option<String>,
    pub render: Callback<AsChildRenderProps, Element>,
}

#[component]
pub fn ButtonAsChild(props: ButtonAsChildProps) -> Element
```

The native `Button` surfaces the full core prop set. `ButtonAsChild` exposes state, visual, tab-order, and consumer root attrs only; native button/form attrs belong on a callback-owned child root when needed. Plain props are preferred; controlled reactivity is introduced only when a wrapper needs it.

## 3. Mapping to Core Component Contract

- Props parity: native `Button` has full parity with the core `Props`, including the typed `Variant`/`Size` surface and full native form overrides. `ButtonAsChild` forwards only the root attrs that are legal for arbitrary callback roots.
- Event parity: native `Button` drives `Focus`, `Blur`, `Press`, `Release`, `Click`, `SetLoading`, and `SetDisabled`. `ButtonAsChild` is attr-forwarding only until the shared as-child contract supports handler forwarding.
- Core machine ownership: `use_machine::<button::Machine>(...)` remains the single source of truth for state and attrs.

## 4. Part Mapping

| Core part / structure | Required?   | Adapter rendering target                                    | Ownership                                                                    | Attr source                     | Notes                                                                |
| --------------------- | ----------- | ----------------------------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------- | -------------------------------------------------------------------- |
| `Root`                | required    | `<button>` by default; consumer child when `as_child=true`  | adapter-owned by default; consumer-owned under root reassignment             | `api.root_attrs()`              | The core `Root` never disappears; only the rendering target changes. |
| `LoadingIndicator`    | conditional | `<span>` inside `Root` while `button::Api::is_loading(api)` | adapter-owned                                                                | `api.loading_indicator_attrs()` | Uses the core status/live attrs while loading.                       |
| `Content`             | required    | `<span>` wrapping the visible label/icon slot               | adapter-owned by default; suppressed under `ButtonAsChild` root reassignment | `api.content_attrs()`           | The native `Button` renders the core content attrs directly.         |

## 5. Attr Merge and Ownership Rules

| Target node        | Core attrs                                                                    | Adapter-owned attrs                                                              | Consumer attrs                                                                                  | Merge order                                                                                                                                                                                                      | Ownership notes                                                                                             |
| ------------------ | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `Root`             | `api.root_attrs()` including state, ARIA, `type`, `form`, and tab-order attrs | pointer-modality bookkeeping hooks and any adapter-local `data-ars-part` helpers | exposed `class`, `style`, `aria_label`, and `aria_labelledby`; child attrs when `as_child=true` | core required state/ARIA attrs win; native attrs required by the native root contract win; `class`/`style` merge additively; handlers compose adapter before public callbacks when preventing invalid activation | adapter-owned by default; consumer-owned only after root reassignment under `as_child`                      |
| `LoadingIndicator` | `api.loading_indicator_attrs()`                                               | none beyond structural wrapper choice                                            | no direct consumer attrs unless the hosting component exposes a dedicated loading slot          | core attrs apply as-is; consumer decoration must not remove status/live attrs while loading                                                                                                                      | always adapter-owned when rendered                                                                          |
| `Content`          | no core attr map beyond conceptual part identity                              | `data-ars-part="content"` and any wrapper-only attrs                             | consumer children content only                                                                  | adapter structural attrs always remain; consumer classes decorate content inside the wrapper rather than replacing it                                                                                            | wrapper is adapter-owned unless a documented `as_child` strategy folds the wrapper into the reassigned root |

- Native `Button` preserves `id`, `role`, `tabindex`, `aria-*`, `data-*`, `type`, `name`, `value`, and `form` even when consumer attrs are present.
- `ButtonAsChild` preserves `id`, `role`, `tabindex`, `aria-*`, and `data-*`, but filters native-only button/form attrs before forwarding to arbitrary callback roots.
- `class` and `style` are additive unless the hosting component explicitly declares a non-additive escape hatch.
- Under `as_child`, root reassignment changes rendered-node ownership only; it does not allow the consumer child to drop core accessibility or state attrs.

## 6. Composition / Context Contract

`Button` is standalone. `ButtonAsChild` reassigns `Root` through an explicit render callback that receives `AsChildRenderProps`; the callback-owned root must spread the provided root attrs. This path forwards the root attr map without adding a wrapper node. The native `Button` keeps the `Content` wrapper, while `ButtonAsChild` treats the callback root as the root and does not render the adapter-owned content wrapper. No other contexts are required.

## 7. Prop Sync and Event Mapping

Controlled/uncontrolled switching is not supported after mount. `disabled` and `loading` are controlled reactive inputs; all default-only values are read at initialization unless a higher-level wrapper documents additional sync.

| Adapter prop              | Mode                      | Sync trigger                      | Machine event / update path                                       | Visible effect                                                            | Notes                                                                              |
| ------------------------- | ------------------------- | --------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `disabled`                | controlled                | signal or prop change after mount | `SetDisabled`                                                     | updates focusability, disabled semantics, and blocked activation behavior | sync is immediate and effect-based                                                 |
| `loading`                 | controlled                | signal or prop change after mount | `SetLoading`                                                      | toggles loading indicator, busy state, and blocked activation behavior    | sync is immediate and effect-based                                                 |
| `prevent_focus_on_press`  | adapter prop              | prop change after mount           | adapter reads current machine props before pointer press handling | affects whether `pointerdown` prevents focus movement                     | no separate machine event unless the core machine models it directly               |
| `type`                    | non-reactive adapter prop | render time only                  | included in root props passed to the machine                      | controls native submit/reset/button behavior                              | post-mount changes should be treated as unsupported unless a wrapper reinitializes |
| `form` and form overrides | non-reactive adapter prop | render time only                  | included in root props passed to the machine                      | binds the button to the target form owner and native form override attrs  | `form_action` accepts `SafeUrl` or a static string through `FormAction`; DOM output remains sanitized by the core contract |

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

| Target part / node | Ref required?                                                     | Ref owner                                                         | Node availability                      | Composition rule                                               | Notes                                                                           |
| ------------------ | ----------------------------------------------------------------- | ----------------------------------------------------------------- | -------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `Root`             | no public ref requirement unless an imperative focus API is added | adapter-owned by default; consumer-owned under `as_child`         | event target supplied by the framework | no composed ref contract in the first Button adapter           | Declarative event handlers are sufficient for the current press/focus behavior. |
| `LoadingIndicator` | no                                                                | adapter-owned                                                     | always structural, handle optional     | no composition                                                 | Structural only.                                                                |
| `Content` wrapper  | no                                                                | adapter-owned unless folded into a documented `as_child` strategy | always structural, handle optional     | no composition unless root reassignment eliminates the wrapper | The content wrapper is not the interaction target.                              |

## 10. State Machine Boundary Rules

- machine-owned state: disabled, loading, pressed, focus-visible, busy semantics, and the emitted root/loading attrs.
- adapter-local derived bookkeeping: pointer-versus-keyboard modality tracking and temporary `prevent_focus_on_press` guard state.
- forbidden local mirrors: do not mirror loading, disabled, or pressed state in local signals that can diverge from machine events.
- allowed snapshot-read contexts: render derivation, pointer and focus event handlers, and cleanup for ephemeral pointer bookkeeping only.

## 11. Callback Payload Contract

| Callback                                          | Payload source             | Payload shape                  | Timing                                       | Cancelable? | Notes                                                      |
| ------------------------------------------------- | -------------------------- | ------------------------------ | -------------------------------------------- | ----------- | ---------------------------------------------------------- |
| `on_press_start` / `on_press_end` / `on_press_up` | normalized adapter payload | `ars_interactions::PressEvent` | after `Press` / after `Release` respectively | no          | Must reflect deduplicated native button behavior.          |
| `on_press`                                        | normalized adapter payload | `ars_interactions::PressEvent` | after normalized click/activation handling   | no          | Native `<button>` Enter and click semantics converge here. |
| `on_press_change`                                 | normalized adapter payload | `bool`                         | when pressed state enters or exits           | no          | Mirrors the normalized press lifecycle.                    |

## 12. Failure and Degradation Rules

| Condition                                              | Policy    | Notes                                                     |
| ------------------------------------------------------ | --------- | --------------------------------------------------------- |
| `as_child` receives zero or multiple children          | fail fast | Root reassignment requires exactly one consumer child.    |
| browser-only activation details unavailable during SSR | no-op     | SSR renders structure only; no interactive behavior runs. |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                            | Notes                                                                               |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| root button instance             | instance-derived | not applicable      | not applicable                           | root structure and part identity must stay stable across hydration | `Root`, `LoadingIndicator`, and `Content` identities belong to one button instance. |

## 14. SSR and Client Boundary Rules

- SSR must render the same `Root` / `LoadingIndicator?` / `Content` structure implied by the initial machine state.
- No root node handle is required by the first Button adapter contract.
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
2. Establish root attr derivation, including native-attr filtering under `as_child`.
3. Derive root/loading/content attrs and render the documented structure.
4. Wire controlled sync for `disabled` and `loading`.
5. Normalize pointer, focus, click, and keyboard events.
6. Add press callback wiring, SSR guards, and cleanup for pointer-modality bookkeeping.

## 18. Anti-Patterns

- Do not attach Space-key handlers to native `<button>` roots that already synthesize click.
- Do not drop `Content` just because its core attr map is empty.
- Do not use HTML `disabled` to represent loading semantics when the core contract requires `aria-disabled` and `aria-busy`.

## 19. Consumer Expectations and Guarantees

- Consumers may assume documented adapter-owned structural nodes and attrs remain the canonical implementation surface.
- Consumers may assume framework-specific divergence is called out explicitly rather than hidden in generic prose.
- Consumers must not assume unspecified fallback behavior, cleanup ordering, or helper ownership beyond what this adapter spec documents.

## 20. Platform Support Matrix

| Capability / behavior                     | Web          | Desktop      | Mobile       | SSR          | Notes                                                                                                |
| ----------------------------------------- | ------------ | ------------ | ------------ | ------------ | ---------------------------------------------------------------------------------------------------- |
| documented structural and state semantics | full support | full support | full support | full support | This utility does not have additional platform variance beyond its existing framework and SSR rules. |

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

Dioxus 0.7.x allows the adapter to keep root attrs in a memo and spread them into the rendered node. `ButtonAsChild` uses the existing explicit render callback contract instead of vnode mutation: the adapter passes `AsChildRenderProps { attrs }`, and the callback root spreads `..attrs`. Required contexts use `try_use_context::<T>().expect(...)`; optional ones use `try_use_context::<T>()`.

## 24. Canonical Implementation Sketch

```rust
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ButtonSketchProps {
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub loading: bool,
    pub children: Element,
}

#[component]
pub fn Button(props: ButtonSketchProps) -> Element {
    let core_props = button::Props {
        disabled: props.disabled,
        loading: props.loading,
        ..Default::default()
    };

    let machine = use_machine::<button::Machine>(core_props);
    let root_attrs = machine.derive(|api| api.root_attrs());
    let loading_attrs = machine.derive(|api| api.loading_indicator_attrs());
    let is_loading = machine.derive(button::Api::is_loading);
    let mut last_pointer = use_signal(|| false);

    rsx! {
        button {
            ..root_attrs.read().clone(),
            onpointerdown: move |ev| {
                last_pointer.set(true);
                machine.send.call(button::Event::Press);
                if machine.with_api_snapshot(button::Api::should_prevent_focus_on_press) {
                    ev.prevent_default();
                }
            },
            onpointerup: move |_| machine.send.call(button::Event::Release),
            onfocus: move |_| {
                let is_keyboard = !*last_pointer.read();
                last_pointer.set(false);
                machine.send.call(button::Event::Focus { is_keyboard });
            },
            onblur: move |_| machine.send.call(button::Event::Blur),
            onclick: move |_| machine.send.call(button::Event::Click),

            if *is_loading.read() {
                rsx! { span { ..loading_attrs.read().clone() } }
            }
            span { "data-ars-part": "content", {props.children} }
        }
    }
}
```

For native `<button>`, the adapter must deduplicate Space-key handlers as required by the core accessibility contract.

## 25. Reference Implementation Skeleton

```rust,no_check
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
- `Content` must remain a documented structural node for native `<button>` rendering.
- `ButtonAsChild` reassigns `Root` only and does not render adapter-owned `Content` or `LoadingIndicator` wrappers.
- `LoadingIndicator` must stay structurally distinct and expose core status/live attrs whenever it is rendered.
- Callbacks must follow normalized press and activation semantics rather than raw DOM event order.

## 27. Accessibility and SSR Notes

- `LoadingIndicator` exposes loading progress through `role="status"` and `aria-live="polite"` while rendered.
- `Content` is part of the accessible name unless overridden by `aria-label` or `aria-labelledby`.
- Loading uses `aria-disabled="true"` and `aria-busy="true"` instead of the HTML `disabled` attribute.
- SSR must preserve stable IDs and initial loading state.

## 28. Parity Summary and Intentional Deviations

Parity summary: native `Button` has full core prop, part, and event parity. `ButtonAsChild` has root attr parity only and intentionally omits native form props and press callbacks until shared as-child event forwarding exists.

Intentional deviations: `ButtonAsChild` filters native-only button/form attrs because the callback API cannot prove the child root is a native `<button>`. The native `Button` remains the full form-capable path.

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
