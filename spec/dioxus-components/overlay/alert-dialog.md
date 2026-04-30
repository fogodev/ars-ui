---
adapter: dioxus
component: alert-dialog
category: overlay
source: components/overlay/alert-dialog.md
source_foundation: foundation/09-adapter-dioxus.md
---

# AlertDialog -- Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`AlertDialog`](../../components/overlay/alert-dialog.md) component to Dioxus 0.7.x. AlertDialog reuses Dialog's state machine entirely (see [Dialog](../../components/overlay/dialog.md)) with stricter dismiss defaults: `close_on_backdrop: false`, `close_on_escape: false`, and `role: AlertDialog`. The adapter owns portal rendering, backdrop sibling placement, focus trapping with initial focus on CancelTrigger, scroll lock, inert background management, Presence composition for entry/exit animations, z-index allocation, PreventableEvent gating for dismiss callbacks, the `is_destructive` styling attribute on ActionTrigger, and multi-platform behavior (Web, Desktop, Mobile, SSR).

## 2. Public Adapter API

```rust,no_check
pub mod alert_dialog {
    #[derive(Props, Clone, PartialEq)]
    pub struct AlertDialogProps {
        #[props(optional)]
        pub id: Option<String>,
        #[props(optional)]
        pub open: Option<Signal<bool>>,
        #[props(optional, default = false)]
        pub default_open: bool,
        #[props(optional, default = true)]
        pub modal: bool,
        #[props(optional, default = false)]
        pub close_on_backdrop: bool,
        #[props(optional, default = false)]
        pub close_on_escape: bool,
        #[props(optional, default = true)]
        pub prevent_scroll: bool,
        #[props(optional, default = true)]
        pub restore_focus: bool,
        #[props(optional)]
        pub initial_focus: Option<FocusTarget>,
        #[props(optional)]
        pub final_focus: Option<FocusTarget>,
        #[props(optional)]
        pub title_level: Option<u8>,
        #[props(optional)]
        pub messages: Option<alert_dialog::Messages>,
        #[props(optional)]
        pub locale: Option<Locale>,
        #[props(optional, default = false)]
        pub lazy_mount: bool,
        #[props(optional, default = false)]
        pub unmount_on_exit: bool,
        #[props(optional, default = false)]
        pub is_destructive: bool,
        #[props(optional)]
        pub on_open_change: Option<EventHandler<bool>>,
        #[props(optional)]
        pub on_escape_key_down: Option<EventHandler<PreventableEvent>>,
        #[props(optional)]
        pub on_interact_outside: Option<EventHandler<PreventableEvent>>,
        pub children: Element,
    }

    #[component]
    pub fn AlertDialog(props: AlertDialogProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct TriggerProps {
        pub children: Element,
    }

    #[component]
    pub fn Trigger(props: TriggerProps) -> Element

    #[component]
    pub fn Backdrop() -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct PositionerProps {
        pub children: Element,
    }

    #[component]
    pub fn Positioner(props: PositionerProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct ContentProps {
        pub children: Element,
    }

    #[component]
    pub fn Content(props: ContentProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct TitleProps {
        pub children: Element,
    }

    #[component]
    pub fn Title(props: TitleProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct DescriptionProps {
        pub children: Element,
    }

    #[component]
    pub fn Description(props: DescriptionProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct CancelTriggerProps {
        pub children: Element,
    }

    #[component]
    pub fn CancelTrigger(props: CancelTriggerProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct ActionTriggerProps {
        pub children: Element,
    }

    #[component]
    pub fn ActionTrigger(props: ActionTriggerProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct CloseTriggerProps {
        pub children: Element,
    }

    #[component]
    pub fn CloseTrigger(props: CloseTriggerProps) -> Element
}
```

Root props expose the full core AlertDialog prop set: `open` (as `Option<Signal<bool>>`), `default_open`, `modal`, `close_on_backdrop`, `close_on_escape`, `prevent_scroll`, `restore_focus`, `initial_focus`, `final_focus`, `title_level`, `messages`, `locale`, `lazy_mount`, `unmount_on_exit`, `is_destructive`, `on_open_change` (as `Option<EventHandler<bool>>`), `on_escape_key_down`, and `on_interact_outside`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core AlertDialog props. Default overrides (`close_on_backdrop: false`, `close_on_escape: false`, `role: AlertDialog`) are applied when constructing the core Props.
- Event parity: Open, Close, Toggle, CloseOnBackdropClick, CloseOnEscape, RegisterTitle, and RegisterDescription all map to adapter-level interactions. PreventableEvent gating for backdrop click and Escape is adapter-owned.
- Structure parity: all 10 parts (Root, Trigger, Backdrop, Positioner, Content, Title, Description, CancelTrigger, ActionTrigger, CloseTrigger) must be rendered with their core attrs. Backdrop and Content render as siblings inside the portal root.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target                         | Ownership     | Attr source                  | Notes                                                        |
| --------------------- | --------- | ------------------------------------------------ | ------------- | ---------------------------- | ------------------------------------------------------------ |
| Root                  | required  | `<div>` wrapper around compound component tree   | adapter-owned | `api.root_attrs()`           | Top-level context provider.                                  |
| Trigger               | required  | `<button>` outside the portal                    | adapter-owned | `api.trigger_attrs()`        | Toggles open state.                                          |
| Backdrop              | required  | `<div>` sibling of Positioner inside portal root | adapter-owned | `api.backdrop_attrs()`       | `aria-hidden="true"`, `inert`. No click-to-dismiss.          |
| Positioner            | required  | `<div>` sibling of Backdrop inside portal root   | adapter-owned | `api.positioner_attrs()`     | Centers Content.                                             |
| Content               | required  | `<div>` inside Positioner                        | adapter-owned | `api.content_attrs()`        | `role="alertdialog"`, `aria-modal="true"`.                   |
| Title                 | required  | `<h2>` (or `<h{level}>`) inside Content          | adapter-owned | `api.title_attrs()`          | `aria-labelledby` target.                                    |
| Description           | required  | `<p>` inside Content                             | adapter-owned | `api.description_attrs()`    | `aria-describedby` target.                                   |
| CancelTrigger         | required  | `<button>` inside Content                        | adapter-owned | `api.cancel_trigger_attrs()` | Receives initial focus. Closes dialog.                       |
| ActionTrigger         | required  | `<button>` inside Content                        | adapter-owned | `api.action_trigger_attrs()` | `data-ars-destructive` when `is_destructive`. Closes dialog. |
| CloseTrigger          | optional  | `<button>` inside Content                        | adapter-owned | `api.close_trigger_attrs()`  | Explicit close.                                              |

## 5. Attr Merge and Ownership Rules

| Target node   | Core attrs                   | Adapter-owned attrs                                     | Consumer attrs             | Merge order                                                             | Ownership notes                                       |
| ------------- | ---------------------------- | ------------------------------------------------------- | -------------------------- | ----------------------------------------------------------------------- | ----------------------------------------------------- |
| Root          | `api.root_attrs()`           | structural `data-*` markers                             | consumer root attrs        | core scope, state, and part attrs win; `class`/`style` merge additively | adapter-owned root wrapper                            |
| Trigger       | `api.trigger_attrs()`        | `type="button"`, event handlers                         | consumer trigger attrs     | core ARIA attrs win                                                     | adapter-owned button                                  |
| Backdrop      | `api.backdrop_attrs()`       | none beyond core                                        | none                       | core attrs apply directly                                               | adapter-owned; consumers do not add attrs to backdrop |
| Positioner    | `api.positioner_attrs()`     | z-index CSS custom property                             | consumer positioner attrs  | core part attrs win                                                     | adapter-owned centering wrapper                       |
| Content       | `api.content_attrs()`        | `tabindex="-1"` during animation delay, keydown handler | consumer content attrs     | core role, ARIA attrs win; `class`/`style` merge additively             | adapter-owned content container                       |
| Title         | `api.title_attrs()`          | heading-level semantic element                          | consumer title attrs       | core ID attr wins                                                       | adapter-owned heading                                 |
| Description   | `api.description_attrs()`    | none beyond core                                        | consumer description attrs | core ID attr wins                                                       | adapter-owned paragraph                               |
| CancelTrigger | `api.cancel_trigger_attrs()` | `type="button"`, click handler                          | consumer cancel attrs      | core ARIA label wins                                                    | adapter-owned button; receives initial focus          |
| ActionTrigger | `api.action_trigger_attrs()` | `type="button"`, click handler                          | consumer action attrs      | core ARIA label and `data-ars-destructive` win                          | adapter-owned button                                  |
| CloseTrigger  | `api.close_trigger_attrs()`  | `type="button"`, click handler                          | consumer close attrs       | core part attrs win                                                     | adapter-owned button                                  |

- Consumers must not override `role`, `aria-modal`, `aria-labelledby`, or `aria-describedby` on Content.
- Consumers must not override `aria-hidden` or `inert` on Backdrop.

## 6. Composition / Context Contract

Root provides an `Context` context consumed by all descendant parts. The context carries the machine handle (state, send, service, context_version), derived `open` signal, derived `title_id` and `description_id`, and `is_destructive` flag. Published via `use_context_provider`, consumed via `try_use_context::<Context>().expect("must be inside AlertDialog")`.

AlertDialog composes:

- `FocusScope` for focus trapping within Content, with initial focus directed to CancelTrigger.
- `Dismissable` for outside-interaction detection, though AlertDialog defaults to rejecting outside dismiss.
- `Presence` for mount/unmount animation lifecycle on the portal content.
- `ArsProvider` for portal root resolution.
- `ZIndexAllocator` for z-index allocation on the Positioner.

Title and Description parts send `RegisterTitle` and `RegisterDescription` events on mount to wire `aria-labelledby` and `aria-describedby` on Content.

## 7. Prop Sync and Event Mapping

| Adapter prop        | Mode         | Sync trigger            | Machine event / update path              | Visible effect                                 | Notes                                                                            |
| ------------------- | ------------ | ----------------------- | ---------------------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------------- |
| `open`              | controlled   | prop change after mount | `Event::Open` / `Event::Close`           | opens or closes the alert dialog               | deferred `use_effect`, not body-level sync, because open/close dispatches events |
| `is_destructive`    | non-reactive | render time only        | no machine event; passed through context | `data-ars-destructive` on ActionTrigger        | styling-only prop                                                                |
| `close_on_backdrop` | non-reactive | render time only        | guards `CloseOnBackdropClick` transition | whether backdrop click dismisses               | defaults to `false`                                                              |
| `close_on_escape`   | non-reactive | render time only        | guards `CloseOnEscape` transition        | whether Escape dismisses                       | defaults to `false`                                                              |
| `messages`          | non-reactive | render time only        | resolved via `resolve_messages`          | ARIA labels on CancelTrigger and ActionTrigger | merged with defaults                                                             |
| `lazy_mount`        | non-reactive | render time only        | Presence composition                     | defers portal content render until first open  | no machine event                                                                 |
| `unmount_on_exit`   | non-reactive | render time only        | Presence composition                     | removes portal content after close animation   | no machine event                                                                 |

| UI event            | Preconditions                                | Machine event / callback path                     | Ordering notes                                     | Notes                             |
| ------------------- | -------------------------------------------- | ------------------------------------------------- | -------------------------------------------------- | --------------------------------- |
| Trigger click       | Trigger rendered                             | `Event::Toggle`                                   | immediate                                          | toggles open/closed               |
| Escape keydown      | Content focused, `close_on_escape` check     | PreventableEvent -> `Event::CloseOnEscape`        | adapter gates with PreventableEvent before sending | defaults to no-op for AlertDialog |
| Backdrop click      | Backdrop rendered, `close_on_backdrop` check | PreventableEvent -> `Event::CloseOnBackdropClick` | adapter gates with PreventableEvent before sending | defaults to no-op for AlertDialog |
| CancelTrigger click | CancelTrigger rendered                       | `Event::Close` then `on_open_change(false)`       | close event fires first                            | safe action                       |
| ActionTrigger click | ActionTrigger rendered                       | `Event::Close` then `on_open_change(false)`       | close event fires first                            | destructive/confirming action     |
| CloseTrigger click  | CloseTrigger rendered                        | `Event::Close`                                    | immediate                                          | explicit close button             |
| Title mount         | Title rendered                               | `Event::RegisterTitle`                            | mount effect                                       | wires `aria-labelledby`           |
| Description mount   | Description rendered                         | `Event::RegisterDescription`                      | mount effect                                       | wires `aria-describedby`          |

## 8. Registration and Cleanup Contract

| Registered entity       | Registration trigger                  | Identity key       | Cleanup trigger             | Cleanup action                                      | Notes                                |
| ----------------------- | ------------------------------------- | ------------------ | --------------------------- | --------------------------------------------------- | ------------------------------------ |
| dialog stack entry      | open transition                       | dialog instance ID | close transition or unmount | pop from `DIALOG_STACK`, re-apply inert for new top | adapter-level static stack           |
| scroll lock             | open transition with `prevent_scroll` | dialog instance    | close transition or unmount | restore body overflow and scroll position           | outermost dialog owns lock; web-only |
| focus scope             | open transition                       | dialog instance    | close transition or unmount | deactivate scope, restore focus to trigger          | web and desktop                      |
| inert background        | open transition                       | dialog instance    | close transition or unmount | remove `inert` from siblings                        | web-only                             |
| z-index allocation      | portal mount                          | dialog instance    | portal unmount              | release allocated z-index                           | via `ZIndexAllocator` context        |
| Presence listeners      | Presence mount                        | dialog instance    | Presence unmount            | remove animation event listeners                    | web-only                             |
| Escape keydown listener | Content mount                         | dialog instance    | Content unmount             | remove keydown listener                             | all platforms                        |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                 | Composition rule                                              | Notes                                   |
| ------------------ | ------------- | ------------- | --------------------------------- | ------------------------------------------------------------- | --------------------------------------- |
| Root               | no            | adapter-owned | always structural handle optional | no composition needed                                         | wrapper only                            |
| Trigger            | yes           | adapter-owned | always structural handle optional | stores trigger ID for focus restoration                       | focus restoration target                |
| Backdrop           | no            | adapter-owned | client-only (portal)              | no composition needed                                         | decorative                              |
| Positioner         | no            | adapter-owned | client-only (portal)              | no composition needed                                         | centering wrapper                       |
| Content            | yes           | adapter-owned | required after mount              | FocusScope container, Dismissable root, Escape keydown target | focus trap boundary                     |
| CancelTrigger      | yes           | adapter-owned | required after mount              | initial focus target                                          | adapter must focus this element on open |
| ActionTrigger      | no            | adapter-owned | always structural handle optional | no composition needed                                         | receives `data-ars-destructive`         |
| CloseTrigger       | no            | adapter-owned | always structural handle optional | no composition needed                                         | optional close button                   |
| Title              | no            | adapter-owned | always structural handle optional | sends RegisterTitle on mount                                  | ID-based wiring                         |
| Description        | no            | adapter-owned | always structural handle optional | sends RegisterDescription on mount                            | ID-based wiring                         |

## 10. State Machine Boundary Rules

- machine-owned state: open/closed, role, modal, close_on_backdrop, close_on_escape, prevent_scroll, restore_focus, initial_focus, final_focus, has_title, has_description, locale, messages.
- adapter-local derived bookkeeping: content node ref, cancel-trigger node ref, trigger node ref, Presence machine handle, z-index value, scroll lock restore handle, inert cleanup handle, dialog stack position.
- forbidden local mirrors: do not keep a local `is_open` signal that can diverge from the machine state. Derive open status from the machine via `derive(|api| api.is_open())`.
- allowed snapshot-read contexts: render derivation, mount effects, cleanup callbacks, PreventableEvent gating in event handlers.

## 11. Callback Payload Contract

| Callback              | Payload source           | Payload shape           | Timing                                      | Cancelable? | Notes                                                                               |
| --------------------- | ------------------------ | ----------------------- | ------------------------------------------- | ----------- | ----------------------------------------------------------------------------------- |
| `on_open_change`      | machine-derived snapshot | `bool` (new open state) | after state transition                      | no          | fires on every open/close transition                                                |
| `on_escape_key_down`  | raw framework event      | `PreventableEvent`      | before `CloseOnEscape` event is sent        | yes         | adapter creates PreventableEvent, invokes callback, checks `is_default_prevented()` |
| `on_interact_outside` | raw framework event      | `PreventableEvent`      | before `CloseOnBackdropClick` event is sent | yes         | adapter creates PreventableEvent, invokes callback, checks `is_default_prevented()` |

## 12. Failure and Degradation Rules

| Condition                                       | Policy             | Notes                                                                                         |
| ----------------------------------------------- | ------------------ | --------------------------------------------------------------------------------------------- |
| Content ref missing after mount                 | fail fast          | Focus trap, Escape listener, and Dismissable all require the Content node.                    |
| CancelTrigger ref missing for initial focus     | degrade gracefully | Fall back to first focusable element inside Content.                                          |
| Portal root missing (ArsProvider absent)        | fail fast          | AlertDialog content must render in a portal (web). On Desktop/Mobile, portal may be implicit. |
| `inert` attribute not supported by browser      | fallback path      | Apply `aria-hidden="true"` and `tabindex="-1"` polyfill on siblings (web-only concern).       |
| Title not rendered (RegisterTitle never fires)  | warn and ignore    | Content lacks `aria-labelledby` but remains functional.                                       |
| Description not rendered                        | warn and ignore    | Content lacks `aria-describedby` but remains functional.                                      |
| Trigger removed from DOM before close           | degrade gracefully | Focus restoration uses fallback chain: nearest focusable ancestor, then body.                 |
| Desktop/Mobile platform missing scroll lock API | no-op              | Scroll lock is a web-only concern; skip on Desktop/Mobile.                                    |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed?         | DOM order must match registration order? | SSR/hydration stability                               | Notes                 |
| -------------------------------- | ---------------- | --------------------------- | ---------------------------------------- | ----------------------------------------------------- | --------------------- |
| dialog stack entry               | instance-derived | no (one entry per instance) | not applicable                           | instance identity must remain stable across hydration | keyed by component ID |
| z-index allocation               | instance-derived | no                          | not applicable                           | client-only allocation                                | released on unmount   |
| Title/Description registration   | instance-derived | no                          | not applicable                           | stable across hydration                               | ID-based wiring       |

## 14. SSR and Client Boundary Rules

- SSR renders Root and Trigger. Portal content (Backdrop, Positioner, Content, Title, Description, CancelTrigger, ActionTrigger, CloseTrigger) is not rendered during SSR when closed (default state).
- When `default_open: true`, SSR renders the portal structure without focus trapping, scroll lock, inert management, or Escape listeners.
- All focus management, scroll lock, inert manipulation, dialog stack registration, z-index allocation, and animation listeners are client-only.
- Content ref, CancelTrigger ref, and Trigger ref are server-safe absent.
- Hydration must preserve the Root and Trigger structure. Portal content hydration stability depends on whether the dialog was open during SSR.
- No callback may fire during SSR.
- On Desktop and Mobile targets, SSR does not apply; all rendering is client-side.

## 15. Performance Constraints

- Dialog stack push/pop and inert recalculation must not scan the entire DOM; they operate on siblings of the portal root only (web).
- Scroll lock acquisition must happen once on open, not on every render.
- Presence animation listeners attach once and clean up once; they must not churn on content rerenders.
- Derived signals for `is_open`, `title_id`, and `description_id` must not trigger unnecessary child rerenders when the values have not changed. Dioxus `Signal<T>` is `Copy`, so derived values avoid cloning overhead.
- Z-index allocation and release happen once per open/close cycle.

## 16. Implementation Dependencies

| Dependency          | Required? | Dependency type      | Why it must exist first                                                | Notes                                                      |
| ------------------- | --------- | -------------------- | ---------------------------------------------------------------------- | ---------------------------------------------------------- |
| Dialog adapter      | required  | shared helper        | AlertDialog reuses Dialog's machine and shares modal overlay patterns. | AlertDialog may delegate to shared Dialog adapter helpers. |
| Presence adapter    | required  | composition contract | Entry/exit animation lifecycle management.                             | Must be composed to control portal content mount/unmount.  |
| FocusScope adapter  | required  | composition contract | Focus trapping within Content.                                         | Initial focus directed to CancelTrigger.                   |
| Dismissable adapter | required  | composition contract | Outside-interaction detection and containment.                         | AlertDialog defaults to rejecting outside dismiss.         |
| ArsProvider         | required  | context contract     | Portal root resolution (web).                                          | Content renders inside portal on web targets.              |
| ZIndexAllocator     | required  | context contract     | Z-index allocation for Positioner.                                     | Prevents hardcoded z-index values.                         |

## 17. Recommended Implementation Sequence

1. Implement Root with machine initialization, context publication via `use_context_provider`, and controlled `open` prop sync.
2. Implement Trigger with toggle click handler and ARIA attrs.
3. Implement portal rendering (Backdrop + Positioner + Content as siblings in portal root) gated by Presence.
4. Wire Content with `role="alertdialog"`, `aria-modal`, `aria-labelledby`, `aria-describedby`, and Escape keydown handler with PreventableEvent gating.
5. Implement FocusScope composition on Content with initial focus on CancelTrigger.
6. Implement Title and Description with RegisterTitle/RegisterDescription mount events.
7. Implement CancelTrigger and ActionTrigger with close handlers and `data-ars-destructive` attr.
8. Implement CloseTrigger with close handler.
9. Wire scroll lock, inert background, and dialog stack push/pop effects.
10. Verify cleanup ordering: focus restoration, scroll lock release, inert removal, dialog stack pop, Presence unmount, z-index release.
11. Verify Desktop and Mobile platform behavior (focus trapping without DOM scroll lock or inert).

## 18. Anti-Patterns

- Do not allow backdrop click to dismiss the AlertDialog by default; the core contract defaults `close_on_backdrop` to `false`.
- Do not allow Escape to dismiss the AlertDialog by default; the core contract defaults `close_on_escape` to `false`.
- Do not focus the ActionTrigger (destructive action) on open; initial focus must go to CancelTrigger (safe action) to prevent accidental confirmation.
- Do not attach focus trapping, scroll lock, or inert manipulation during SSR.
- Do not render Backdrop as a parent wrapper around Content; use the backdrop sibling pattern inside the portal root.
- Do not hardcode z-index values; use ZIndexAllocator.
- Do not send `CloseOnEscape` or `CloseOnBackdropClick` events without first running PreventableEvent gating through the corresponding callback.
- Do not keep a local `is_open` signal separate from the machine-derived state.
- Do not skip the dialog stack push/pop on open/close transitions.
- Do not assume scroll lock or `inert` APIs are available on Desktop or Mobile; gate those effects to web targets only.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the AlertDialog will not close on backdrop click or Escape unless they explicitly override `close_on_backdrop` or `close_on_escape` to `true`.
- Consumers may assume initial focus lands on CancelTrigger when the AlertDialog opens, unless `initial_focus` is explicitly set.
- Consumers may assume `data-ars-destructive` is present on ActionTrigger when `is_destructive` is `true`.
- Consumers may assume focus returns to the Trigger element when the AlertDialog closes.
- Consumers may assume Title and Description are wired to `aria-labelledby` and `aria-describedby` on Content when rendered.
- Consumers may assume the adapter handles platform differences (web vs Desktop vs Mobile) transparently.
- Consumers must not assume the AlertDialog closes on backdrop click or Escape by default.
- Consumers must not assume focus management or scroll lock operates during SSR.
- Consumers must not assume portal content is in the DOM when the AlertDialog is closed (Presence controls mount/unmount).
- Consumers must not assume scroll lock or inert work identically on Desktop/Mobile as on web.

## 20. Platform Support Matrix

| Capability / behavior          | Web           | Desktop        | Mobile         | SSR            | Notes                                              |
| ------------------------------ | ------------- | -------------- | -------------- | -------------- | -------------------------------------------------- |
| AlertDialog open/close         | full support  | full support   | full support   | SSR-safe empty | state machine initializes; effects are client-only |
| Focus trapping                 | full support  | full support   | full support   | client-only    | FocusScope activates after mount                   |
| Initial focus on CancelTrigger | full support  | full support   | full support   | client-only    | focus movement is a platform operation             |
| Scroll lock                    | full support  | not applicable | not applicable | client-only    | body overflow manipulation is web-only             |
| Inert background               | full support  | not applicable | not applicable | client-only    | `inert` attr set on DOM siblings; web-only         |
| `inert` polyfill               | fallback path | not applicable | not applicable | not applicable | `aria-hidden` + `tabindex` fallback; web-only      |
| Portal rendering               | full support  | full support   | full support   | SSR-safe empty | Desktop/Mobile may use implicit portal             |
| Z-index allocation             | full support  | full support   | not applicable | SSR-safe empty | allocation is client-only                          |
| Presence animation             | full support  | fallback path  | fallback path  | client-only    | Desktop/Mobile may not support CSS animations      |
| PreventableEvent gating        | full support  | full support   | full support   | not applicable | no callbacks fire during SSR                       |
| `data-ars-destructive` attr    | full support  | full support   | full support   | SSR-safe empty | styling attribute on ActionTrigger                 |

## 21. Debug Diagnostics and Production Policy

| Condition                                        | Debug build behavior | Production behavior | Notes                                         |
| ------------------------------------------------ | -------------------- | ------------------- | --------------------------------------------- |
| Content ref missing after mount                  | fail fast            | fail fast           | AlertDialog cannot function without Content   |
| CancelTrigger ref missing for initial focus      | debug warning        | degrade gracefully  | falls back to first focusable in Content      |
| Title not rendered                               | debug warning        | warn and ignore     | missing `aria-labelledby` is an a11y concern  |
| Description not rendered                         | debug warning        | warn and ignore     | missing `aria-describedby` is an a11y concern |
| Portal root missing (web)                        | fail fast            | fail fast           | overlay must render in portal                 |
| `inert` not supported (web)                      | debug warning        | degrade gracefully  | polyfill path applies                         |
| Trigger removed before focus restore             | debug warning        | degrade gracefully  | fallback chain applies                        |
| Platform capability unavailable (Desktop/Mobile) | debug warning        | no-op               | scroll lock and inert skipped                 |

## 22. Shared Adapter Helper Notes

| Helper concept             | Required?   | Responsibility                                                           | Reused by                                                | Notes                               |
| -------------------------- | ----------- | ------------------------------------------------------------------------ | -------------------------------------------------------- | ----------------------------------- |
| Portal rendering helper    | required    | render overlay content into ArsProvider portal root                      | Dialog, AlertDialog, Drawer, Popover, Tooltip, HoverCard | shared across all overlay adapters  |
| Focus-scope helper         | required    | activate focus trap, manage initial focus target, restore focus on close | Dialog, AlertDialog, Drawer                              | modal overlay focus pattern         |
| Dismiss helper             | required    | outside-interaction detection with PreventableEvent gating               | Dialog, AlertDialog, Drawer, Popover                     | Dismissable composition             |
| Dialog stack helper        | required    | push/pop dialog IDs, manage inert on siblings                            | Dialog, AlertDialog, Drawer                              | shared static `DIALOG_STACK`        |
| Scroll lock helper         | required    | prevent body scroll, compensate scrollbar width, restore on close        | Dialog, AlertDialog, Drawer                              | outermost modal owns lock; web-only |
| Z-index allocation helper  | required    | allocate and release z-index from ZIndexAllocator context                | all overlay adapters                                     | prevents hardcoded values           |
| Merge helper               | recommended | merge core attrs with adapter and consumer attrs                         | all adapters                                             | `class`/`style` additive merge      |
| Warning helper             | recommended | emit debug diagnostics for missing refs, titles, descriptions            | all overlay adapters                                     | debug-only warnings                 |
| Platform capability helper | recommended | detect web vs Desktop vs Mobile and gate platform-specific effects       | all overlay adapters                                     | shared platform detection           |

## 23. Framework-Specific Behavior

Dioxus uses `use_drop` for cleanup work (focus restoration, scroll lock release, inert removal, dialog stack pop, Presence listener removal). Controlled `open` prop is watched via a deferred `use_effect` that compares previous and current values before dispatching Open/Close events; this is an intentional exception to body-level sync because open/close dispatches events. Context is published via `use_context_provider(|| Context { ... })` and consumed via `try_use_context::<Context>().expect("must be inside AlertDialog")`. Dioxus `Signal<T>` is `Copy`, so the context struct should derive `Clone, Copy` and signals are read via `*signal.read()`. On Desktop and Mobile targets, scroll lock and `inert` management are skipped since those are web-only DOM APIs. The `rsx!` macro is used for rendering.

## 24. Canonical Implementation Sketch

```rust
use dioxus::prelude::*;

#[derive(Clone, Copy)]
struct Context {
    open: ReadSignal<bool>,
    send: Callback<alert_dialog::Event>,
    title_id: Memo<String>,
    description_id: Memo<String>,
    service: Signal<Service<alert_dialog::Machine>>,
    context_version: ReadSignal<u64>,
    is_destructive: bool,
}

#[derive(Props, Clone, PartialEq)]
pub struct AlertDialogProps {
    #[props(optional)]
    pub open: Option<Signal<bool>>,

    #[props(optional, default = false)]
    pub default_open: bool,

    #[props(optional, default = false)]
    pub is_destructive: bool,

    #[props(optional)]
    pub on_open_change: Option<EventHandler<bool>>,

    #[props(optional)]
    pub on_escape_key_down: Option<EventHandler<alert_dialog::PreventableEvent>>,

    #[props(optional)]
    pub on_interact_outside: Option<EventHandler<alert_dialog::PreventableEvent>>,

    pub children: Element,
}

#[component]
pub fn AlertDialog(props: AlertDialogProps) -> Element {
    let core_props = alert_dialog::Props {
        open: props.open.as_ref().map(|s| *s.read()),
        default_open: props.default_open,
        is_destructive: props.is_destructive,
        ..Default::default()
    };

    let machine = use_machine::<alert_dialog::Machine>(core_props);
    let UseMachineReturn { state, send, .. } = machine;

    // Controlled open sync (deferred, not body-level)
    if let Some(open_sig) = props.open {
        let send_clone = send;
        let mut prev_open: Signal<Option<bool>> = use_signal(|| None);
        use_effect(move || {
            let new_open = *open_sig.read();
            let prev = prev_open.read().clone();
            if prev.as_ref() != Some(&new_open) {
                if prev.is_some() {
                    if new_open {
                        send_clone.call(alert_dialog::Event::Open);
                    } else {
                        send_clone.call(alert_dialog::Event::Close);
                    }
                }
                *prev_open.write() = Some(new_open);
            }
        });
    }

    let open = machine.derive(|api| api.is_open());
    let title_id = machine.derive(|api| api.title_id().to_string());
    let description_id = machine.derive(|api| api.description_id().to_string());

    use_context_provider(|| Context {
        open: open.into(),
        send,
        title_id,
        description_id,
        service: machine.service,
        context_version: machine.context_version,
        is_destructive: props.is_destructive,
    });

    rsx! { {props.children} }
}

#[component]
pub fn Trigger(props: TriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("alert_dialog::Trigger must be inside AlertDialog");

    rsx! {
        button {
            r#type: "button",
            "data-ars-scope": "alert-dialog",
            "data-ars-part": "trigger",
            "aria-haspopup": "dialog",
            "aria-expanded": (*ctx.open.read()).to_string(),
            onclick: move |_| ctx.send.call(alert_dialog::Event::Toggle),
            {props.children}
        }
    }
}

#[component]
pub fn Content(props: ContentProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("alert_dialog::Content must be inside AlertDialog");
    let is_open = *ctx.open.read();

    let title_id = ctx.title_id;
    let desc_id = ctx.description_id;
    let send = ctx.send;

    if !is_open {
        return rsx! {};
    }

    rsx! {
        // Backdrop (sibling of positioner)
        div {
            "data-ars-scope": "alert-dialog",
            "data-ars-part": "backdrop",
            "data-ars-state": if is_open { "open" } else { "closed" },
            "aria-hidden": "true",
            inert: true,
        }
        // Positioner
        div {
            "data-ars-scope": "alert-dialog",
            "data-ars-part": "positioner",
            onclick: move |e| e.stop_propagation(),
            div {
                role: "alertdialog",
                "aria-modal": "true",
                "aria-labelledby": title_id.read().clone(),
                "aria-describedby": desc_id.read().clone(),
                "data-ars-scope": "alert-dialog",
                "data-ars-part": "content",
                "data-ars-state": if is_open { "open" } else { "closed" },
                onkeydown: move |e: KeyboardEvent| {
                    if dioxus_key_to_keyboard_key(&e.key()).0 == KeyboardKey::Escape {
                        // PreventableEvent gating would happen here
                        send.call(alert_dialog::Event::CloseOnEscape);
                    }
                },
                {props.children}
            }
        }
    }
}

#[component]
pub fn Title(props: TitleProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("alert_dialog::Title must be inside AlertDialog");

    // Send RegisterTitle on mount
    use_effect(move || {
        ctx.send.call(alert_dialog::Event::RegisterTitle);
    });

    rsx! {
        h2 {
            id: ctx.title_id.read().clone(),
            "data-ars-scope": "alert-dialog",
            "data-ars-part": "title",
            {props.children}
        }
    }
}

#[component]
pub fn Description(props: DescriptionProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("alert_dialog::Description must be inside AlertDialog");

    use_effect(move || {
        ctx.send.call(alert_dialog::Event::RegisterDescription);
    });

    rsx! {
        p {
            id: ctx.description_id.read().clone(),
            "data-ars-scope": "alert-dialog",
            "data-ars-part": "description",
            {props.children}
        }
    }
}

#[component]
pub fn CancelTrigger(props: CancelTriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("alert_dialog::CancelTrigger must be inside AlertDialog");

    rsx! {
        button {
            r#type: "button",
            "data-ars-scope": "alert-dialog",
            "data-ars-part": "cancel-trigger",
            onclick: move |_| ctx.send.call(alert_dialog::Event::Close),
            {props.children}
        }
    }
}

#[component]
pub fn ActionTrigger(props: ActionTriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("alert_dialog::ActionTrigger must be inside AlertDialog");

    rsx! {
        button {
            r#type: "button",
            "data-ars-scope": "alert-dialog",
            "data-ars-part": "action-trigger",
            "data-ars-destructive": if ctx.is_destructive { Some("") } else { None },
            onclick: move |_| ctx.send.call(alert_dialog::Event::Close),
            {props.children}
        }
    }
}

#[component]
pub fn CloseTrigger(props: CloseTriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("alert_dialog::CloseTrigger must be inside AlertDialog");

    rsx! {
        button {
            r#type: "button",
            "data-ars-scope": "alert-dialog",
            "data-ars-part": "close-trigger",
            onclick: move |_| ctx.send.call(alert_dialog::Event::Close),
            {props.children}
        }
    }
}

// Usage:
// rsx! {
//     AlertDialog { is_destructive: true, on_open_change: move |open| log!("{open}"),
//         alert_dialog::Trigger { "Delete Account" }
//         alert_dialog::Content {
//             alert_dialog::Title { "Are you sure?" }
//             alert_dialog::Description { "This action cannot be undone." }
//             alert_dialog::CancelTrigger { "Cancel" }
//             alert_dialog::ActionTrigger { "Delete" }
//         }
//     }
// }
```

## 25. Reference Implementation Skeleton

```rust,no_check
// Root
let machine = use_machine::<alert_dialog::Machine>(core_props);
let is_open = machine.derive(|api| api.is_open());
let ctx = build_alert_dialog_context(machine, is_destructive);
use_context_provider(|| ctx);
sync_controlled_open_prop(open_signal, machine.send);  // deferred use_effect

// Portal content (inside Content component)
let content_ref = create_content_ref();
let cancel_ref = create_cancel_trigger_ref();
let presence = use_machine::<presence::Machine>(presence_props);
sync_dialog_open_to_presence(is_open, presence.send);

// Client-only effects (gated by platform detection)
register_focus_scope(content_ref, cancel_ref);            // trap + initial focus
register_dialog_stack(dialog_id);                          // push on open
if is_web_platform() {
    apply_scroll_lock(prevent_scroll);                     // body overflow (web-only)
    apply_inert_background(dialog_id);                     // inert on siblings (web-only)
}
allocate_z_index(positioner_ref);                          // ZIndexAllocator
wire_escape_handler(content_ref, on_escape_key_down, send);  // PreventableEvent gating
wire_backdrop_handler(backdrop_ref, on_interact_outside, send);

// Render
render_root_with_context(children);
render_trigger(toggle_handler);
render_portal_content_gated_by_presence(
    backdrop, positioner, content, title, description,
    cancel_trigger, action_trigger, close_trigger,
);

// Cleanup
use_drop(|| {
    restore_focus_to_trigger(trigger_ref);
    if is_web_platform() {
        release_scroll_lock();
        remove_inert_from_siblings();
    }
    dialog_stack_pop(dialog_id);
    presence.cleanup();
    release_z_index();
});
```

## 26. Adapter Invariants

- `role="alertdialog"` must always be set on Content; never fall back to `role="dialog"`.
- `close_on_backdrop` must default to `false`; the adapter must not send `CloseOnBackdropClick` unless the consumer has explicitly overridden this default.
- `close_on_escape` must default to `false`; the adapter must not send `CloseOnEscape` unless the consumer has explicitly overridden this default.
- Initial focus must land on CancelTrigger (safe action), not ActionTrigger (destructive action) or the first focusable element, unless `initial_focus` is explicitly set.
- PreventableEvent gating must run before sending `CloseOnEscape` or `CloseOnBackdropClick` events; if `is_default_prevented()` returns true, the event must not be sent.
- `data-ars-destructive` must be present on ActionTrigger when `is_destructive` is `true` and absent when `false`.
- Backdrop and Content must be siblings in the portal root (backdrop sibling pattern), not parent-child.
- Dialog stack push must happen on open; pop must happen on close or unmount.
- Scroll lock, inert background, and focus scope must all clean up before unmount completes.
- Focus restoration must use the fallback chain (trigger, nearest focusable ancestor, body) when the trigger is unavailable.
- No focus management, scroll lock, inert manipulation, or callback invocation during SSR.
- Web-only effects (scroll lock, inert) must be gated by platform detection and skipped on Desktop/Mobile.
- Dioxus `Callback` uses `.call()` for event dispatch, not `.run()`.

## 27. Accessibility and SSR Notes

- `role="alertdialog"` combined with `aria-modal="true"` announces the dialog as an alert requiring action.
- `aria-labelledby` points to Title; `aria-describedby` points to Description. Both require the corresponding parts to be rendered and registered.
- Initial focus on CancelTrigger prevents accidental destructive action confirmation by keyboard users.
- Focus trap (via FocusScope) prevents Tab/Shift+Tab from leaving the AlertDialog.
- `inert` on background siblings prevents screen reader virtual cursor from escaping the AlertDialog (web-only).
- SSR renders the closed-state structure only; all interactive behavior is client-only.
- On Desktop targets, focus trapping works via the platform's native focus management. `inert` is not available; the adapter relies on focus scope containment alone.
- On Mobile targets, the AlertDialog should present as a blocking modal; platform-specific accessibility announcements may differ from web.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, part, event, and behavior parity. All 10 parts render with correct attrs. PreventableEvent gating is adapter-owned. Focus, scroll lock, inert, and dialog stack management follow the Dialog adapter pattern with AlertDialog-specific overrides. Multi-platform behavior is documented for Web, Desktop, Mobile, and SSR.

Intentional deviations: none.

Traceability note: This adapter spec makes explicit the core adapter-owned concerns for PreventableEvent gating on dismiss callbacks, CancelTrigger initial focus, `data-ars-destructive` attr on ActionTrigger, portal rendering with backdrop sibling pattern, dialog stack management, scroll lock (web-only), inert background (web-only), focus trapping, z-index allocation, Presence composition, controlled open prop sync, and multi-platform effect gating.

## 29. Test Scenarios

- AlertDialog opens on Trigger click and Content receives `role="alertdialog"` with `aria-modal="true"`
- initial focus lands on CancelTrigger, not ActionTrigger or first focusable
- Escape does not close AlertDialog when `close_on_escape` is `false` (default)
- backdrop click does not close AlertDialog when `close_on_backdrop` is `false` (default)
- CancelTrigger click closes AlertDialog
- ActionTrigger click closes AlertDialog
- `data-ars-destructive` present on ActionTrigger when `is_destructive` is `true`
- `data-ars-destructive` absent on ActionTrigger when `is_destructive` is `false`
- `aria-labelledby` wired to Title ID when Title is rendered
- `aria-describedby` wired to Description ID when Description is rendered
- focus returns to Trigger on close
- focus trap keeps Tab/Shift+Tab within Content
- controlled `open` Signal sync opens and closes AlertDialog
- `on_open_change` EventHandler fires with correct boolean after open/close transitions
- PreventableEvent gating prevents close when `on_escape_key_down` calls `prevent_default()` (only when `close_on_escape` is overridden to `true`)
- scroll lock applied on open, released on close (web-only)
- inert applied to portal root siblings on open, removed on close (web-only)
- dialog stack correctly manages nested AlertDialog/Dialog combinations
- Presence controls mount/unmount of portal content
- Desktop target: AlertDialog functions without scroll lock or inert

## 30. Test Oracle Notes

| Behavior                                         | Preferred oracle type | Notes                                                             |
| ------------------------------------------------ | --------------------- | ----------------------------------------------------------------- |
| `role="alertdialog"` and `aria-modal` on Content | DOM attrs             | assert role and aria-modal attributes                             |
| initial focus on CancelTrigger                   | DOM attrs             | assert `document.activeElement` matches CancelTrigger (web)       |
| Escape/backdrop dismiss blocked                  | machine state         | assert state remains Open after Escape/backdrop click             |
| CancelTrigger/ActionTrigger close                | machine state         | assert state transitions to Closed                                |
| `data-ars-destructive` on ActionTrigger          | DOM attrs             | assert data attribute presence/absence                            |
| `aria-labelledby`/`aria-describedby` wiring      | DOM attrs             | assert Content attrs reference Title/Description IDs              |
| focus restoration to Trigger                     | cleanup side effects  | assert focus returns to Trigger element after close               |
| scroll lock and inert cleanup                    | cleanup side effects  | assert body overflow restored and `inert` removed (web)           |
| dialog stack ordering                            | context registration  | assert stack contains correct dialog IDs in order                 |
| on_open_change callback                          | callback order        | assert callback fires after state transition with correct boolean |
| portal content mount/unmount                     | rendered structure    | assert portal content present when open, absent when closed       |
| PreventableEvent gating                          | callback order        | assert event not sent when callback prevents default              |

Cheap verification recipe:

1. Render AlertDialog, click Trigger, and assert Content has `role="alertdialog"` and focus is on CancelTrigger.
2. Press Escape and assert AlertDialog remains open (default behavior). Press CancelTrigger and assert AlertDialog closes.
3. Reopen, click ActionTrigger, and assert AlertDialog closes and focus returns to Trigger.
4. With `is_destructive: true`, assert `data-ars-destructive` is present on ActionTrigger.
5. Unmount and assert scroll lock released (web), inert removed (web), and dialog stack empty.
6. On Desktop target, repeat steps 1-4 and verify focus trapping works without scroll lock or inert.

## 31. Implementation Checklist

- [ ] Root initializes machine with AlertDialog defaults (`close_on_backdrop: false`, `close_on_escape: false`, `role: AlertDialog`).
- [ ] Root publishes `Context` via `use_context_provider` with machine handle, `open`, `title_id`, `description_id`, `is_destructive`.
- [ ] Controlled `open` Signal sync uses deferred `use_effect` with previous-value comparison.
- [ ] Trigger toggles open state via `send.call(Event::Toggle)` and renders correct ARIA attrs.
- [ ] Portal content (Backdrop, Positioner, Content) renders with backdrop sibling pattern.
- [ ] Content has `role="alertdialog"`, `aria-modal="true"`, `aria-labelledby`, `aria-describedby`.
- [ ] FocusScope activates on Content with initial focus on CancelTrigger.
- [ ] Title sends `RegisterTitle` on mount; Description sends `RegisterDescription` on mount.
- [ ] CancelTrigger closes AlertDialog on click via `send.call(Event::Close)`.
- [ ] ActionTrigger closes AlertDialog on click; `data-ars-destructive` present when `is_destructive` is `true`.
- [ ] CloseTrigger closes AlertDialog on click.
- [ ] Escape keydown handler gates with PreventableEvent before sending `CloseOnEscape`.
- [ ] Backdrop click handler gates with PreventableEvent before sending `CloseOnBackdropClick`.
- [ ] `on_open_change` EventHandler fires after state transitions.
- [ ] Dialog stack push on open, pop on close/unmount.
- [ ] Scroll lock applied on open, released on close/unmount (web-only).
- [ ] Inert applied to portal root siblings on open, removed on close/unmount (web-only).
- [ ] Z-index allocated from ZIndexAllocator on portal mount, released on unmount.
- [ ] Presence controls portal content mount/unmount with animation support.
- [ ] Focus restoration uses fallback chain on close.
- [ ] All client-only effects clean up via `use_drop`.
- [ ] No focus management, scroll lock, inert, or callbacks during SSR.
- [ ] Platform-specific effects gated (scroll lock and inert are web-only).
