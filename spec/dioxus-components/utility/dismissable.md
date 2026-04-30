---
adapter: dioxus
component: dismissable
category: utility
source: components/utility/dismissable.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Dismissable — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Dismissable`](../../components/utility/dismissable.md) behavior to Dioxus 0.7.x. The adapter owns document-level listeners, portal-aware inside boundaries, and both repeated dismiss buttons rendered around the dismissable content.

## 2. Public Adapter API

The adapter exposes everything through a single `dismissable` module
(reachable via `use ars_dioxus::prelude::*;`). The module re-exports the
agnostic `ars_components::utility::dismissable::*` surface alongside the
Dioxus-side wrappers, so callers spell every type with the same prefix:

```rust,no_check
pub fn dismissable::use_dismissable(
    root_ref: ReadSignal<Option<Rc<MountedData>>>,
    props: dismissable::Props,
    inside_boundaries: ReadSignal<Vec<String>>,
) -> dismissable::Handle

#[derive(Clone, Copy)]
pub struct dismissable::Handle {
    /// Arena-backed Dioxus callback. Invoke with `dismiss.call(())` to
    /// fire `props.on_dismiss(DismissReason::DismissButton)` if a
    /// callback is registered.
    pub dismiss: dioxus::prelude::Callback<()>,
    /// Stable id used for overlay-stack registration, portal-owner
    /// matching, and DOM root resolution. Stored in the Dioxus arena via
    /// [`CopyValue`] so `Handle` is `Copy`. Read the underlying `String`
    /// with `overlay_id.read()` (borrow guard) or
    /// `overlay_id.with(|id| …)` (closure).
    pub overlay_id: dioxus::prelude::CopyValue<String>,
}

#[derive(Props, Clone, Debug, PartialEq)]
pub struct RegionProps {
    pub props: dismissable::Props,
    #[props(optional)]
    pub inside_boundaries: Option<ReadSignal<Vec<String>>>,
    #[props(optional)]
    pub dismiss_label: Option<String>,
    #[props(optional, into)]
    pub locale: Option<Signal<Locale>>,
    #[props(optional)]
    pub messages: Option<dismissable::Messages>,
    pub children: Element,
}

#[component]
pub fn Region(props: RegionProps) -> Element
```

`Handle` is intentionally `Copy`. Both fields live in the active Dioxus
scope's arena — [`Callback`](dioxus::prelude::Callback) and
[`CopyValue`](dioxus::prelude::CopyValue) are both arena-backed `Copy`
newtypes. Consumers can move the handle into multiple closures or pass
it through the rsx tree without explicit clones; it stays valid until
the owning scope unmounts.

The public surface matches the full core `Props`, including `on_interact_outside`, `on_escape_key_down`, `on_dismiss`, `disable_outside_pointer_events`, and `exclude_ids`. The agnostic core owns the shared `dismissable::Messages` fallback bundle, while the adapter-owned `Region` resolves that bundle from `ArsProvider` / `locale`, falling back to `"Dismiss"`; `dismiss_label` is an explicit final-label override.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core props.
- Event parity: outside pointer, outside focus, Escape, and dismiss-button activation all map to the core callbacks.
- Structure parity: the adapter must render both dismiss-button instances, not just mention them abstractly.

## 4. Part Mapping

| Core part / structure      | Required? | Adapter rendering target                           | Ownership                                          | Attr source                                   | Notes                                        |
| -------------------------- | --------- | -------------------------------------------------- | -------------------------------------------------- | --------------------------------------------- | -------------------------------------------- |
| `DismissButton` (start)    | repeated  | native `<button>` before dismissable content       | adapter-owned                                      | `dismiss_button_attrs(label)`                 | First visually hidden dismiss control.       |
| dismissable content region | required  | consumer children inside the dismissable container | consumer-owned content inside adapter-owned region | root attrs plus adapter listener registration | Structural node, not a separate core `Part`. |
| `DismissButton` (end)      | repeated  | native `<button>` after dismissable content        | adapter-owned                                      | `dismiss_button_attrs(label)`                 | Second visually hidden dismiss control.      |

## 5. Attr Merge and Ownership Rules

| Target node                                     | Core attrs                                        | Adapter-owned attrs                                                     | Consumer attrs                                      | Merge order                                                                            | Ownership notes                                                              |
| ----------------------------------------------- | ------------------------------------------------- | ----------------------------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `Root` / dismissable region                     | `api.root_attrs()`                                | listener registration markers and structural `data-*` helpers if needed | consumer root attrs                                 | core containment, state, and accessibility attrs win; `class`/`style` merge additively | adapter-owned root region                                                    |
| `DismissButton (start)` / `DismissButton (end)` | per-button core attrs                             | native button defaults and structural part markers                      | consumer decoration only if the buttons are exposed | core dismiss semantics win                                                             | adapter-owned dismiss controls unless a documented compound API exposes them |
| content region                                  | no separate core attrs beyond being inside `Root` | adapter-owned content-region wrapper attrs if present                   | consumer children content                           | root-controlled containment attrs remain on the region owner                           | consumer-owned children inside adapter-owned containment structure           |

- Consumers must not override root attrs in ways that break inside/outside containment.
- Dismiss-button handlers compose around normalized dismiss logic; consumer callbacks may observe the dismiss event but must not bypass containment guards.

## 6. Composition / Context Contract

Usually composed by overlays. The adapter must allow additional “inside” boundaries such as trigger elements or portal roots so outside detection matches the core contract.

## 7. Prop Sync and Event Mapping

Dismissable state is primarily interaction-driven. Configuration props are generally non-reactive after mount unless a wrapper re-registers listeners. Callback timing must be documented relative to normalized dismiss decisions.

| Adapter prop                               | Mode                      | Sync trigger            | Machine event / update path     | Visible effect                                    | Notes                                            |
| ------------------------------------------ | ------------------------- | ----------------------- | ------------------------------- | ------------------------------------------------- | ------------------------------------------------ |
| outside-interaction config                 | non-reactive adapter prop | render time only        | listener setup configuration    | determines which outside interactions dismiss     | dynamic changes require listener re-registration |
| open/active state if controlled by wrapper | controlled                | prop change after mount | controlled open-state sync path | attaches or detaches outside-interaction behavior | wrapper-defined when applicable                  |

| UI event                    | Preconditions                                               | Machine event / callback path                                     | Ordering notes                                          | Notes                                          |
| --------------------------- | ----------------------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------- | ---------------------------------------------- |
| outside pointer interaction | listener attached and target outside containment            | normalized outside-pointer path -> dismiss callback / close event | containment check runs before callbacks                 | must be portal-aware                           |
| outside focus movement      | listener attached and next focus target outside containment | normalized outside-focus path -> dismiss callback / close event   | focus target validation runs before callbacks           | client-only listener                           |
| `Escape`                    | region active and Escape dismissal enabled                  | normalized Escape path -> dismiss callback / close event          | Escape handling runs before notification-only callbacks | must not fire during SSR                       |
| dismiss-button activation   | dismiss button rendered and interactive                     | direct dismiss path                                               | button activation normalizes before public callbacks    | both dismiss buttons follow the same semantics |

## 8. Registration and Cleanup Contract

- Global listeners register only after the root region and any portal-aware containment references are available.
- Start and end dismiss buttons do not need separate global registration, but their existence must remain synchronized with the dismissable region.
- Cleanup must remove listeners, timers, retries, and any detached portal handles before unmount completes.

| Registered entity            | Registration trigger                   | Identity key         | Cleanup trigger                             | Cleanup action                              | Notes                                |
| ---------------------------- | -------------------------------------- | -------------------- | ------------------------------------------- | ------------------------------------------- | ------------------------------------ |
| outside pointer listener     | active dismissable mount on the client | dismissable instance | close, inactive state, or component cleanup | remove listener                             | client-only                          |
| outside focus listener       | active dismissable mount on the client | dismissable instance | close, inactive state, or component cleanup | remove listener                             | client-only                          |
| Escape listener              | active dismissable mount on the client | dismissable instance | close, inactive state, or component cleanup | remove listener                             | client-only                          |
| portal containment reference | portal-aware setup                     | dismissable instance | portal change or component cleanup          | drop stored node handles / containment refs | prevents stale inside/outside checks |

## 9. Ref and Node Contract

| Target part / node                         | Ref required?                 | Ref owner                                      | Node availability                  | Composition rule                                                   | Notes                                                                |
| ------------------------------------------ | ----------------------------- | ---------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------ | -------------------------------------------------------------------- |
| dismissable root region                    | yes                           | adapter-owned                                  | required after mount               | compose only if a wrapper also needs the live root node            | Outside-interaction containment requires a concrete node handle.     |
| portal or additional inside-boundary nodes | yes when supplied by wrappers | consumer-owned but registered with the adapter | client-only                        | compose by registering boundary handles, not by mutating ownership | IDs alone are insufficient once portal-aware containment is in play. |
| dismiss buttons                            | no                            | adapter-owned                                  | always structural, handle optional | no composition unless the buttons are separately exposed           | Their semantics do not depend on stored refs.                        |

## 10. State Machine Boundary Rules

- machine-owned state: active dismissable behavior, containment decisions, and normalized outside-interaction outcomes.
- adapter-local derived bookkeeping: listener handles, portal boundary refs, retry timers, and transient pointer/focus event objects.
- forbidden local mirrors: do not keep a local open/dismissed flag that can diverge from normalized dismiss callbacks or wrapper-controlled state.
- allowed snapshot-read contexts: listener callbacks, mount effects, and cleanup when checking the latest containment rules.

## 11. Callback Payload Contract

| Callback              | Payload source             | Payload shape                                  | Timing                                              | Cancelable?                   | Notes                                                |
| --------------------- | -------------------------- | ---------------------------------------------- | --------------------------------------------------- | ----------------------------- | ---------------------------------------------------- | ---------------------------------------------------------------------- | --- | ---------------------------------- |
| `on_interact_outside` | normalized adapter payload | `{ original_event, interaction_type: "pointer" | "focus", target_within_registered_boundary: bool }` | before final dismiss decision | yes                                                  | Used to veto dismissal while preserving normalized containment checks. |
| `on_escape_key_down`  | raw framework event        | framework keyboard event                       | before final dismiss decision                       | yes                           | Only fires on the client while listeners are active. |
| `on_dismiss`          | machine-derived snapshot   | `{ reason: "outside-pointer"                   | "outside-focus"                                     | "escape"                      | "dismiss-button" }`                                  | after the dismiss decision is finalized                                | no  | Observational completion callback. |

## 12. Failure and Degradation Rules

| Condition                                                                      | Policy             | Notes                                                                                                                                        |
| ------------------------------------------------------------------------------ | ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| root node ref missing after mount                                              | fail fast          | Containment and outside-interaction listeners cannot behave correctly without a root node.                                                   |
| optional portal boundary handle missing                                        | degrade gracefully | Fall back to root-only containment and document reduced boundary awareness.                                                                  |
| platform-specific click-outside dispatch differs on Desktop or webview targets | degrade gracefully | Use the documented target-platform containment path and validate behavior on the actual host instead of assuming browser-identical dispatch. |
| browser listener APIs absent during SSR                                        | no-op              | Render the structure and defer all outside-interaction behavior until mount.                                                                 |

## 13. Identity and Key Policy

| Registered or repeated structure          | Identity source  | Duplicates allowed?                    | DOM order must match registration order? | SSR/hydration stability                           | Notes                                                              |
| ----------------------------------------- | ---------------- | -------------------------------------- | ---------------------------------------- | ------------------------------------------------- | ------------------------------------------------------------------ |
| dismissable root containment registration | instance-derived | not applicable                         | not applicable                           | root identity must remain stable across hydration | Listener ownership is tied to the dismissable instance.            |
| inside-boundary registrations             | composite        | yes when boundaries are distinct nodes | not applicable                           | boundary identity may be client-only              | Identity is boundary node handle plus owning dismissable instance. |

## 14. SSR and Client Boundary Rules

- SSR may render the dismissable region and both dismiss buttons, but all outside-interaction listeners are client-only.
- Root and dismiss-button structure must remain stable across hydration.
- Portal boundary handles are server-safe absent and become required after mount when portal-aware containment is used.
- No dismiss callback may fire during SSR.

## 15. Performance Constraints

- Document, focus, and Escape listeners must not churn on every render; they should only re-register when active state or containment configuration changes.
- Boundary registration should update incrementally instead of rebuilding the entire containment set on unrelated rerenders.
- Cleanup must remove all listeners and timers owned by the dismissable instance in one pass.
- Containment checks should prefer stored handles over repeated DOM queries.

## 16. Implementation Dependencies

| Dependency     | Required?   | Dependency type         | Why it must exist first                                                              | Notes                                                                      |
| -------------- | ----------- | ----------------------- | ------------------------------------------------------------------------------------ | -------------------------------------------------------------------------- |
| `ars-provider` | recommended | context contract        | Environment scoping simplifies DOM boundary resolution and portal-aware containment. | Especially useful for overlay consumers.                                   |
| `focus-scope`  | recommended | behavioral prerequisite | Overlay shells often compose focus trapping with outside-interaction handling.       | Not required for the utility itself, but important for composite overlays. |

## 17. Recommended Implementation Sequence

1. Establish the dismissable root ref and any inside-boundary registration surfaces.
2. Render the root region plus both dismiss buttons and content region.
3. Register document-level outside-pointer, outside-focus, and Escape listeners on the client.
4. Normalize outside-interaction callbacks and dismiss-button activation.
5. Add portal-aware boundary tracking and verify listener cleanup order.

## 18. Anti-Patterns

- Do not treat a root ID as sufficient when the contract requires a live node handle for containment.
- Do not attach outside-interaction listeners during SSR.
- Do not omit one of the dismiss buttons when the documented pattern requires both.

## 19. Consumer Expectations and Guarantees

- Consumers may assume containment is portal-aware after registered boundary handles are available.
- Consumers may assume paired dismiss buttons and root structure remain stable across hydration.
- Consumers must not assume IDs alone are sufficient for outside-interaction containment once portal-aware boundaries are in play.

## 20. Platform Support Matrix

| Capability / behavior           | Web          | Desktop       | Mobile        | SSR          | Notes                                                                        |
| ------------------------------- | ------------ | ------------- | ------------- | ------------ | ---------------------------------------------------------------------------- |
| outside-interaction listeners   | full support | fallback path | fallback path | client-only  | Desktop and webview targets may need host-specific click-outside validation. |
| paired dismiss-button structure | full support | full support  | full support  | full support | Structure remains stable across targets.                                     |

## 21. Debug Diagnostics and Production Policy

| Condition                                                                  | Debug build behavior | Production behavior | Notes                                                                                                 |
| -------------------------------------------------------------------------- | -------------------- | ------------------- | ----------------------------------------------------------------------------------------------------- |
| Desktop or webview click-outside dispatch differs from browser assumptions | debug warning        | degrade gracefully  | Validate containment against the actual Dioxus target instead of assuming browser-identical dispatch. |

## 22. Shared Adapter Helper Notes

| Helper concept                    | Required?   | Responsibility                                                                                                                                                                                                  | Reused by                                                      | Notes                                                                                                                                                                        |
| --------------------------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| node-boundary registration helper | required    | `ars_dom::outside_interaction::target_is_inside_boundary` — walks DOM ancestors checking root containment, ancestor `id` matches, and `data-ars-portal-owner` ↔ overlay-stack portal ownership.                 | `dismissable`, overlays, `focus-scope`                         | IDs are insufficient once live-node containment is required; portal-owner walking is the documented path. See `spec/foundation/11-dom-utilities.md` §12.1.                   |
| platform capability helper        | recommended | `ars_dom::outside_interaction::install_outside_interaction_listeners` — installs document `pointerdown`+`focusin` and root-scoped `keydown` listeners gated on `overlay_stack::is_topmost`; returns a teardown. | `dismissable`, `download-trigger`, `drop-zone`, `action-group` | Web wires real listeners; non-web Dioxus targets (Desktop, mobile, SSR) return a no-op teardown so adapters call uniformly. See `spec/foundation/11-dom-utilities.md` §12.2. |

## 23. Framework-Specific Behavior

Dioxus uses platform-aware listener registration and hook cleanup. Optional environment scoping should be read via `try_use_context::<ArsContext>()` when present. On Dioxus Desktop or embedded webview targets, click-outside behavior can differ from browser tab assumptions because focus movement, native window boundaries, and pointer retargeting do not always mirror web DOM dispatch exactly; the adapter must validate containment against the actual target platform rather than assuming browser-only semantics.

## 24. Canonical Implementation Sketch

```rust
use ars_dioxus::{attr_map_to_dioxus, prelude::*};
use dioxus::prelude::*;

#[derive(Props, Clone, Debug, PartialEq)]
pub struct RegionProps {
    pub props: dismissable::Props,
    #[props(optional)]
    pub inside_boundaries: Option<ReadSignal<Vec<String>>>,
    #[props(optional)]
    pub dismiss_label: Option<String>,
    #[props(optional, into)]
    pub locale: Option<Signal<Locale>>,
    #[props(optional)]
    pub messages: Option<dismissable::Messages>,
    pub children: Element,
}

#[component]
pub fn Region(props: RegionProps) -> Element {
    let RegionProps { props, inside_boundaries, dismiss_label, locale, messages, children } = props;

    let boundaries_fallback = use_signal(Vec::<String>::new);
    let boundaries = inside_boundaries.unwrap_or_else(|| ReadSignal::from(boundaries_fallback));

    let provider_locale = resolve_locale(None);
    let resolved_locale = locale
        .as_ref()
        .map_or(provider_locale, |locale| locale.read().clone());
    let resolved_messages = use_messages(messages.as_ref(), Some(&resolved_locale));
    let dismiss_label =
        dismiss_label.unwrap_or_else(|| (resolved_messages.dismiss_label)(&resolved_locale));

    let api = dismissable::Api::new(props.clone(), dismiss_label);
    let root_attrs = attr_map_to_dioxus(api.root_attrs(), &ars_core::StyleStrategy::Inline, None).attrs;
    let attrs = api.dismiss_button_attrs();
    let start_attrs = attr_map_to_dioxus(attrs.clone(), &ars_core::StyleStrategy::Inline, None).attrs;
    let end_attrs = attr_map_to_dioxus(attrs, &ars_core::StyleStrategy::Inline, None).attrs;

    // `onmounted` populates the ref once the root <div> is in the DOM;
    // `use_dismissable` reads it inside the listener-install effect.
    // Mirrors the Leptos adapter's `NodeRef` pattern.
    let mut root_ref = use_signal(|| None::<Rc<MountedData>>);

    // `Handle` is `Copy`, so the same value can move into both onclick
    // closures without explicit clones.
    let handle = use_dismissable(ReadSignal::from(root_ref), props, boundaries);

    rsx! {
        div {
            ..root_attrs,
            onmounted: move |evt| { root_ref.set(Some(evt.data())); },
            button { onclick: move |_| { handle.dismiss.call(()); }, ..start_attrs }
            {children}
            button { onclick: move |_| { handle.dismiss.call(()); }, ..end_attrs }
        }
    }
}
```

For the common case the adapter ships [`dismissable::Region`] (with
[`dismissable::RegionProps`]) which already renders the paired-button
anatomy above. Both dismiss buttons must be native `<button>` elements;
both fire `props.on_dismiss(dismissable::DismissReason::DismissButton)`
directly via the handle,
bypassing the veto-capable callbacks (the user explicitly clicked the
visually-hidden control, so dismissal is unconditional).

## 25. Reference Implementation Skeleton

```rust,no_check
let machine = use_machine_or_normalized_handle(props);
let root_ref = create_root_ref();
let boundary_registry = create_boundary_registration_helper();
let listeners = create_outside_interaction_helper();

render_root_and_paired_dismiss_buttons(root_ref);
register_portal_or_inside_boundaries(boundary_registry, props);
attach_client_only_listeners(listeners, root_ref, boundary_registry, machine);
normalize_outside_pointer_focus_and_escape_callbacks(machine);

on_cleanup(|| {
    listeners.teardown();
    boundary_registry.release_all();
    cancel_pending_retries_or_timers();
});
```

## 26. Adapter Invariants

- Both dismiss-button instances must be documented separately when the pattern renders paired dismiss controls.
- Dismiss buttons should use native `<button>` semantics unless the spec explicitly documents an alternate trigger strategy.
- Document-level outside-interaction listeners must not attach during SSR.
- Outside-interaction setup must tolerate delayed root or portal availability during mount.
- Pending retries, timers, and global listeners must be cancelled before unmount completes.
- Dismiss callbacks must document their timing relative to outside pointer, outside focus, and Escape handling.
- Portal-aware inside and outside detection must preserve the core containment contract.

## 27. Accessibility and SSR Notes

- Both dismiss buttons must remain reachable to screen readers and keyboard users.
- They are visually hidden but semantically active.
- SSR may render the region and dismiss buttons, but listener registration is client-only.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop and behavior parity, including both repeated dismiss-button instances.

Intentional deviations: none.

Traceability note: This adapter spec now makes explicit the core adapter-owned concerns for live containment handles, paired dismiss structure, portal-aware boundary registration, outside-interaction ordering, and client-only listener cleanup.

## 29. Test Scenarios

- start and end dismiss buttons both rendered
- outside pointer interaction
- outside focus interaction
- Escape ordering
- excluded IDs and inside-boundary handling
- portal-aware inside-region detection
- Dioxus Desktop or webview click-outside behavior matches the documented containment caveats

## 30. Test Oracle Notes

| Behavior                                    | Preferred oracle type | Notes                                                                                                            |
| ------------------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------------------------- |
| outside interaction handling                | callback order        | Assert veto-capable outside callbacks before `on_dismiss`.                                                       |
| listener teardown and boundary release      | cleanup side effects  | Verify document listeners and boundary refs are removed on cleanup.                                              |
| paired dismiss buttons and region structure | rendered structure    | Assert both dismiss buttons remain present around the content region.                                            |
| inside-boundary registration                | context registration  | Use boundary registry state or equivalent observable registration surface.                                       |
| platform-sensitive click-outside handling   | callback order        | Verify Desktop or webview click-outside semantics using the documented target-platform containment expectations. |

Cheap verification recipe:

1. Render the region with both dismiss buttons and assert the documented structure before testing outside interactions.
2. Fire outside pointer, outside focus, or Escape in isolation and verify veto-capable callbacks run before `on_dismiss`.
3. Unmount the region and assert document listeners plus registered inside boundaries are released; on Dioxus Desktop or webview targets, repeat the outside-click check on the target runtime through `ars_test_harness_dioxus::desktop::DesktopHarness` (the headless [`VirtualDom`] harness for non-web Dioxus builds, documented in [`spec/testing/15-test-harness.md`](../../testing/15-test-harness.md) §5.4), asserting that mounting the region returns a structurally-valid `dismissable::Handle` with a non-empty `overlay_id`, that `Handle::dismiss` invokes `on_dismiss` with `DismissReason::DismissButton`, that `on_interact_outside`, `on_escape_key_down`, and `on_dismiss` stay silent across `flush()` (no document listeners install on the non-web cfg branch), and that dropping the harness runs `use_drop`-style cleanup without synthesising any callback firings.

## 31. Implementation Checklist

- [ ] Root and boundary refs are registered before outside-interaction handling starts.
- [ ] Both dismiss buttons and the content region render in the documented order.
- [ ] Outside-pointer, outside-focus, and Escape callbacks fire in the documented order.
- [ ] Client-only listeners and portal boundary registrations clean up correctly.
- [ ] Dioxus Desktop or webview click-outside behavior is validated against the documented platform caveats.
- [ ] Rendered structure and cleanup side effects match the documented test oracles.
