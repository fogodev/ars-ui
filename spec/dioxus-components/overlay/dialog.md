---
adapter: dioxus
component: dialog
category: overlay
source: components/overlay/dialog.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Dialog — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Dialog`](../../components/overlay/dialog.md) contract onto Dioxus 0.7.x. Dialog is the most complex modal overlay and serves as the foundation pattern for AlertDialog and Drawer. The adapter owns:

- Compound component tree: `Dialog`, `Trigger`, `Backdrop`, `Positioner`, `Content`, `Title`, `Description`, `CloseTrigger`.
- Portal rendering of backdrop and content into the `ArsProvider` portal root.
- Z-index allocation via `ZIndexAllocator` for the backdrop and positioner.
- Focus trapping by composing `FocusScope` inside the content part.
- Outside-interaction dismissal by composing `Dismissable` for backdrop click and Escape.
- Scroll lock acquisition and release, including scrollbar-width compensation.
- Inert attribute management on DOM siblings of the portal root in modal mode, including the `DIALOG_STACK` for nested dialogs.
- Presence composition for lazy mount, unmount-on-exit, and entry/exit CSS animations.
- Controlled `open` prop synchronization with the core machine.
- Title and Description ID registration for `aria-labelledby` / `aria-describedby` wiring.
- PreventableEvent gating: invoking `on_escape_key_down` and `on_interact_outside` callbacks before sending dismiss events to the machine.
- Focus restoration with fallback chain on close.
- Multi-platform considerations: Web (full support), Desktop (webview-based focus/scroll), Mobile (webview-based), SSR (trigger-only rendering).

## 2. Public Adapter API

```rust
pub mod dialog {
    #[derive(Props, Clone, PartialEq)]
    pub struct DialogProps {
        #[props(optional)]
        pub id: Option<String>,
        #[props(optional)]
        pub open: Option<Signal<bool>>,
        #[props(optional, default = false)]
        pub default_open: bool,
        #[props(optional, default = true)]
        pub modal: bool,
        #[props(optional, default = true)]
        pub close_on_backdrop: bool,
        #[props(optional, default = true)]
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
        pub role: Option<dialog::Role>,
        #[props(optional)]
        pub title_level: Option<u8>,
        #[props(optional)]
        pub messages: Option<dialog::Messages>,
        #[props(optional, default = false)]
        pub lazy_mount: bool,
        #[props(optional, default = false)]
        pub unmount_on_exit: bool,
        #[props(optional)]
        pub on_open_change: Option<EventHandler<bool>>,
        #[props(optional)]
        pub on_escape_key_down: Option<EventHandler<PreventableEvent>>,
        #[props(optional)]
        pub on_interact_outside: Option<EventHandler<PreventableEvent>>,
        #[props(optional)]
        pub locale: Option<Locale>,
        pub children: Element,
    }

    #[component]
    pub fn Dialog(props: DialogProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct TriggerProps {
        #[props(default = false)]
        pub as_child: bool,
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
        #[props(default = false)]
        pub as_child: bool,
        pub children: Element,
    }

    #[component]
    pub fn Title(props: TitleProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct DescriptionProps {
        #[props(default = false)]
        pub as_child: bool,
        pub children: Element,
    }

    #[component]
    pub fn Description(props: DescriptionProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct CloseTriggerProps {
        #[props(default = false)]
        pub as_child: bool,
        pub children: Element,
    }

    #[component]
    pub fn CloseTrigger(props: CloseTriggerProps) -> Element
}
```

All props default to the core `Props::default()` values. `open` is `Option<Signal<bool>>` for controlled mode; when `None`, the machine is uncontrolled with `default_open`. Dioxus `Signal<T>` is `Copy`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with core `Props`, including `lazy_mount`, `unmount_on_exit`, `title_level`, `messages`, `locale`, all callbacks, and all focus-related props.
- Event parity: `Open`, `Close`, `Toggle`, `CloseOnBackdropClick`, `CloseOnEscape`, `RegisterTitle`, `RegisterDescription` all map to adapter-driven paths.
- Structure parity: all eight core parts are rendered. Portal rendering, backdrop sibling pattern, and Presence composition are adapter-owned.
- Machine ownership: `use_machine::<dialog::Machine>(...)` is the single source of truth.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target                       | Ownership                | Attr source                 | Notes                                  |
| --------------------- | --------- | ---------------------------------------------- | ------------------------ | --------------------------- | -------------------------------------- |
| `Root`                | required  | invisible context wrapper (`{props.children}`) | adapter-owned            | `api.root_attrs()`          | No DOM node; provides context only.    |
| `Trigger`             | required  | `<button>` (or consumer child when `as_child`) | adapter-owned by default | `api.trigger_attrs()`       | Renders inline, not in portal.         |
| `Backdrop`            | required  | `<div>` in portal root                         | adapter-owned            | `api.backdrop_attrs()`      | Sibling of Positioner inside portal.   |
| `Positioner`          | required  | `<div>` in portal root                         | adapter-owned            | `api.positioner_attrs()`    | Sibling of Backdrop inside portal.     |
| `Content`             | required  | `<div>` inside Positioner                      | adapter-owned            | `api.content_attrs()`       | `role="dialog"` or `"alertdialog"`.    |
| `Title`               | optional  | `<h{level}>` inside Content                    | adapter-owned by default | `api.title_attrs()`         | Heading level from `title_level` prop. |
| `Description`         | optional  | `<p>` inside Content                           | adapter-owned by default | `api.description_attrs()`   | Wired to `aria-describedby`.           |
| `CloseTrigger`        | optional  | `<button>` inside Content                      | adapter-owned by default | `api.close_trigger_attrs()` | `aria-label` from Messages.            |

## 5. Attr Merge and Ownership Rules

| Target node    | Core attrs                                                                         | Adapter-owned attrs                        | Consumer attrs                 | Merge order                                           | Ownership notes                                        |
| -------------- | ---------------------------------------------------------------------------------- | ------------------------------------------ | ------------------------------ | ----------------------------------------------------- | ------------------------------------------------------ |
| `Root`         | `api.root_attrs()` (scope, part, state)                                            | none (no DOM node)                         | none                           | not applicable                                        | context-only wrapper                                   |
| `Trigger`      | `api.trigger_attrs()` (id, aria-haspopup, aria-expanded, aria-controls)            | click handler                              | consumer attrs when `as_child` | core ARIA attrs win; `class`/`style` merge additively | adapter-owned default; consumer-owned under `as_child` |
| `Backdrop`     | `api.backdrop_attrs()` (aria-hidden, inert, state)                                 | click handler for dismiss                  | no consumer attrs              | core attrs apply as-is                                | always adapter-owned, decorative                       |
| `Positioner`   | `api.positioner_attrs()` (scope, part)                                             | z-index CSS custom property                | consumer `class`/`style` merge | adapter z-index wins                                  | adapter-owned structural                               |
| `Content`      | `api.content_attrs()` (role, aria-modal, aria-labelledby, aria-describedby, state) | keydown handler, tabindex during animation | consumer `class`/`style` merge | core ARIA and role attrs win                          | adapter-owned                                          |
| `Title`        | `api.title_attrs()` (id, scope, part, heading-level data attr)                     | none                                       | consumer attrs when `as_child` | core id wins                                          | adapter-owned default                                  |
| `Description`  | `api.description_attrs()` (id, scope, part)                                        | none                                       | consumer attrs when `as_child` | core id wins                                          | adapter-owned default                                  |
| `CloseTrigger` | `api.close_trigger_attrs()` (scope, part, aria-label)                              | click handler                              | consumer attrs when `as_child` | core aria-label wins                                  | adapter-owned default                                  |

- `id`, `role`, `aria-*`, and `data-ars-*` attrs must preserve the core contract even when consumer attrs are present.
- `class` and `style` are additive on all parts that accept consumer attrs.
- Under `as_child`, root reassignment changes rendered-node ownership only; core accessibility attrs remain non-negotiable.

## 6. Composition / Context Contract

`Dialog` provides a `Context` via `use_context_provider`. All child parts consume it via `try_use_context::<Context>().expect("dialog::Trigger must be used inside Dialog")`.

```rust
#[derive(Clone, Copy)]
struct Context {
    send: Callback<dialog::Event>,
    open: Memo<bool>,
    trigger_id: Memo<String>,
    content_id: Memo<String>,
    title_id: Memo<String>,
    description_id: Memo<String>,
    has_title: Signal<bool>,
    has_description: Signal<bool>,
    modal: bool,
    role: dialog::Role,
    lazy_mount: bool,
    unmount_on_exit: bool,
    prevent_scroll: bool,
    restore_focus: bool,
    close_on_backdrop: bool,
    close_on_escape: bool,
    initial_focus: Option<FocusTarget>,
    final_focus: Option<FocusTarget>,
    title_level: u8,
    dialog_id: String,
    on_escape_key_down: Option<EventHandler<PreventableEvent>>,
    on_interact_outside: Option<EventHandler<PreventableEvent>>,
}
```

**Composed utility contexts consumed:**

- `ArsProvider` context for portal root resolution.
- `ZIndexAllocator` context for z-index allocation.
- `FocusScope` composed inside `Content` for focus trapping.
- `Dismissable` composed inside `Content` for outside-interaction detection.
- `Presence` machine composed to control mount/unmount lifecycle.

**Title and Description registration**: `Title` on mount sends `Event::RegisterTitle` to the machine. `Description` on mount sends `Event::RegisterDescription`. Both set the corresponding `has_title` / `has_description` context signals so `aria-labelledby` / `aria-describedby` are wired before focus moves into the content.

## 7. Prop Sync and Event Mapping

| Adapter prop        | Mode                 | Sync trigger              | Machine event / update path     | Visible effect                            | Notes                                            |
| ------------------- | -------------------- | ------------------------- | ------------------------------- | ----------------------------------------- | ------------------------------------------------ |
| `open`              | controlled           | signal change after mount | `Event::Open` / `Event::Close`  | opens or closes the dialog                | uses deferred `use_effect` with prev-value guard |
| `default_open`      | uncontrolled default | init only                 | sets initial state              | dialog starts open or closed              | read once at machine init                        |
| `modal`             | non-reactive         | render time only          | `Props.modal`                   | determines focus trap, inert, scroll lock |                                                  |
| `close_on_backdrop` | non-reactive         | render time only          | `Props.close_on_backdrop`       | enables/disables backdrop dismiss         |                                                  |
| `close_on_escape`   | non-reactive         | render time only          | `Props.close_on_escape`         | enables/disables Escape dismiss           |                                                  |
| `prevent_scroll`    | non-reactive         | render time only          | `Props.prevent_scroll`          | applies scroll lock on open               |                                                  |
| `restore_focus`     | non-reactive         | render time only          | `Props.restore_focus`           | restores focus on close                   |                                                  |
| `lazy_mount`        | non-reactive         | render time only          | adapter-local (Presence config) | delays first render until open            |                                                  |
| `unmount_on_exit`   | non-reactive         | render time only          | adapter-local (Presence config) | removes content DOM on close              |                                                  |

| UI event           | Preconditions                    | Machine event / callback path                     | Ordering notes                                                 | Notes                         |
| ------------------ | -------------------------------- | ------------------------------------------------- | -------------------------------------------------------------- | ----------------------------- |
| Trigger click      | trigger rendered                 | `Event::Toggle`                                   | immediate send                                                 |                               |
| Backdrop click     | dialog open, `close_on_backdrop` | PreventableEvent -> `Event::CloseOnBackdropClick` | `on_interact_outside` fires first; only sends if not prevented | adapter obligation            |
| Escape keydown     | dialog open, `close_on_escape`   | PreventableEvent -> `Event::CloseOnEscape`        | `on_escape_key_down` fires first; only sends if not prevented  | topmost dialog in stack only  |
| CloseTrigger click | dialog open                      | `Event::Close`                                    | immediate send                                                 |                               |
| Title mount        | title component mounts           | `Event::RegisterTitle`                            | fires before content receives focus                            | sets `has_title = true`       |
| Description mount  | description component mounts     | `Event::RegisterDescription`                      | fires before content receives focus                            | sets `has_description = true` |

## 8. Registration and Cleanup Contract

| Registered entity       | Registration trigger               | Identity key | Cleanup trigger             | Cleanup action                            | Notes                      |
| ----------------------- | ---------------------------------- | ------------ | --------------------------- | ----------------------------------------- | -------------------------- |
| dialog stack entry      | dialog opens (modal)               | dialog id    | dialog closes               | `dialog_stack_pop()`                      | global `DIALOG_STACK`      |
| scroll lock             | dialog opens with `prevent_scroll` | dialog id    | dialog closes               | restore body overflow and scroll position | outermost dialog owns lock |
| inert siblings          | dialog opens (modal)               | dialog id    | dialog closes               | remove inert from siblings                | via `set_background_inert` |
| FocusScope              | content mounts                     | dialog id    | content unmounts            | deactivate scope, restore focus           | composed utility           |
| Dismissable listeners   | content mounts                     | dialog id    | content unmounts            | remove document listeners                 | composed utility           |
| z-index allocation      | backdrop/positioner mount          | dialog id    | backdrop/positioner unmount | release z-index slot                      | via ZIndexAllocator        |
| portal node             | content mounts                     | dialog id    | content unmounts            | remove portal children                    | via ArsProvider            |
| Presence machine        | root mounts                        | dialog id    | root unmounts               | cancel animations                         | composed machine           |
| Escape keydown listener | content mounts                     | dialog id    | content unmounts            | remove listener                           | client-only                |

## 9. Ref and Node Contract

| Target part / node | Ref required?     | Ref owner         | Node availability                 | Composition rule                     | Notes                                 |
| ------------------ | ----------------- | ----------------- | --------------------------------- | ------------------------------------ | ------------------------------------- |
| Trigger            | yes               | adapter-owned     | always structural handle optional | compose with `as_child` consumer ref | needed for focus restoration target   |
| Backdrop           | no                | adapter-owned     | client-only                       | no composition                       | decorative, no ref needed             |
| Positioner         | no                | adapter-owned     | client-only                       | no composition                       | structural wrapper                    |
| Content            | yes               | adapter-owned     | required after mount              | compose with FocusScope root ref     | focus trap boundary, Dismissable root |
| Title              | no                | adapter-owned     | always structural handle optional | no composition                       | ID-based wiring sufficient            |
| Description        | no                | adapter-owned     | always structural handle optional | no composition                       | ID-based wiring sufficient            |
| CloseTrigger       | no                | adapter-owned     | always structural handle optional | no composition                       | standard button                       |
| Portal root        | yes (environment) | ArsProvider-owned | client-only                       | consume from context                 | backdrop and positioner render here   |

## 10. State Machine Boundary Rules

- Machine-owned state: `open`, `has_title`, `has_description`, `role`, `modal`, all context fields.
- Adapter-local derived bookkeeping: Presence machine signals, z-index allocation handle, scroll lock cleanup handle, inert cleanup handle, FocusScope activation state, dialog stack position, animation state tracking.
- Forbidden local mirrors: do not keep a local `is_open` signal that can diverge from `api.is_open()`. Read open state exclusively from the machine via `derive`.
- Allowed snapshot-read contexts: PreventableEvent callback invocations (reading latest props before sending events), cleanup effects (reading latest context for restoration targets), Presence sync effects.

## 11. Callback Payload Contract

| Callback              | Payload source             | Payload shape           | Timing                                       | Cancelable? | Notes                                       |
| --------------------- | -------------------------- | ----------------------- | -------------------------------------------- | ----------- | ------------------------------------------- |
| `on_open_change`      | machine-derived snapshot   | `bool` (new open state) | after state transition completes             | no          | fires on both open and close                |
| `on_escape_key_down`  | normalized adapter payload | `PreventableEvent`      | before `Event::CloseOnEscape` is sent        | yes         | adapter must check `is_default_prevented()` |
| `on_interact_outside` | normalized adapter payload | `PreventableEvent`      | before `Event::CloseOnBackdropClick` is sent | yes         | adapter must check `is_default_prevented()` |

## 12. Failure and Degradation Rules

| Condition                                      | Policy             | Notes                                                               |
| ---------------------------------------------- | ------------------ | ------------------------------------------------------------------- |
| Content ref missing after mount                | fail fast          | Focus trapping and Dismissable require a live node handle.          |
| Portal root missing (no ArsProvider)           | fail fast          | Dialog content cannot render without a portal target.               |
| ZIndexAllocator context missing                | degrade gracefully | Fall back to unmanaged z-index; emit debug warning.                 |
| FocusScope activation fails                    | degrade gracefully | Dialog remains open but focus may escape; emit debug warning.       |
| Title not rendered (no `aria-labelledby`)      | warn and ignore    | Dialog remains functional but screen readers lack a label.          |
| Trigger removed from DOM before close          | degrade gracefully | Focus restoration uses fallback chain (ancestor, then body).        |
| `inert` attribute not supported by browser     | degrade gracefully | Use `aria-hidden` + `tabindex` polyfill via `set_background_inert`. |
| Desktop/Mobile webview scroll behavior differs | degrade gracefully | Use platform-appropriate scroll lock strategy.                      |
| Desktop focus dispatch differs from browser    | degrade gracefully | Validate focus trap against actual host behavior.                   |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source               | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                  | Notes                                          |
| -------------------------------- | ----------------------------- | ------------------- | ---------------------------------------- | -------------------------------------------------------- | ---------------------------------------------- |
| dialog machine instance          | data-derived (from `id` prop) | no                  | not applicable                           | id must remain stable across hydration                   | `ComponentIds::from_id` generates all part IDs |
| dialog stack entry               | data-derived (dialog id)      | no                  | not applicable                           | client-only                                              | global `DIALOG_STACK` keyed by dialog id       |
| z-index allocation               | instance-derived              | no                  | not applicable                           | client-only                                              | allocated on mount, released on unmount        |
| title/description registration   | instance-derived              | not applicable      | not applicable                           | registration is client-only, IDs stable across hydration | `RegisterTitle` / `RegisterDescription` events |

## 14. SSR and Client Boundary Rules

- SSR renders `Dialog` context wrapper and `Trigger` with full ARIA attrs (`aria-haspopup`, `aria-expanded="false"`, `aria-controls`).
- SSR does NOT render `Backdrop`, `Positioner`, `Content`, or any portal content. These are client-only.
- `Title` and `Description` are client-only (inside portal content).
- `CloseTrigger` is client-only (inside portal content).
- All DOM side effects (scroll lock, inert, FocusScope, Dismissable listeners, z-index allocation, dialog stack) are client-only.
- No callback (`on_open_change`, `on_escape_key_down`, `on_interact_outside`) may fire during SSR.
- Hydration: the trigger node structure must remain stable. The `aria-expanded` attribute on the trigger is `"false"` during SSR and updates reactively on the client.
- `default_open: true` during SSR: the trigger renders with `aria-expanded="true"` but dialog content is not present. Screen readers will not find the referenced `aria-controls` target until the client mounts. This is acceptable because the dialog is interactive-only.

## 15. Performance Constraints

- Controlled `open` prop sync must use a deferred `use_effect` with prev-value guard to prevent unnecessary open/close cycles.
- Presence machine sync must not re-evaluate on unrelated parent rerenders. Dioxus `Signal<T>` is `Copy` and subscription-based, so unnecessary clones are avoided.
- Focus trapping listeners (FocusScope) attach once on content mount and detach on unmount; they must not churn on every render.
- Scroll lock and inert attribute management run as effects tied to the dialog open state, not on every render cycle.
- Z-index allocation should be a single context read on mount, not a per-render computation.
- PreventableEvent creation and callback invocation must be synchronous and not allocate unnecessarily.
- Dialog stack operations (`push`/`pop`) must be O(n) in the number of open dialogs, not O(n^2).

## 16. Implementation Dependencies

| Dependency          | Required?   | Dependency type         | Why it must exist first                                        | Notes                                    |
| ------------------- | ----------- | ----------------------- | -------------------------------------------------------------- | ---------------------------------------- |
| `presence`          | required    | composition contract    | dialog composes Presence for mount/unmount animation lifecycle | must be implemented before dialog        |
| `ars-provider`      | required    | context contract        | portal root resolution for backdrop and content rendering      | dialog content renders into portal       |
| `z-index-allocator` | required    | context contract        | z-index management for backdrop and positioner stacking        | prevents hardcoded z-index values        |
| `focus-scope`       | required    | behavioral prerequisite | focus trapping inside modal dialog content                     | composed inside Content                  |
| `dismissable`       | required    | behavioral prerequisite | outside-interaction detection for backdrop click and Escape    | composed inside Content                  |
| `button`            | recommended | shared helper           | Trigger and CloseTrigger use button semantics                  | not strictly required but shared pattern |

## 17. Recommended Implementation Sequence

1. Implement `Dialog` with machine initialization, context provision via `use_context_provider`, and controlled `open` prop sync.
2. Implement `Trigger` consuming context for toggle behavior and ARIA attrs.
3. Implement portal rendering for backdrop and positioner via `ArsProvider`.
4. Implement `Backdrop` with z-index allocation, click handler, and PreventableEvent gating for `on_interact_outside`.
5. Implement `Positioner` with z-index allocation.
6. Implement `Content` with Presence composition, FocusScope composition, Dismissable composition, Escape key handling with PreventableEvent gating, and `role`/`aria-modal`/`aria-labelledby`/`aria-describedby` wiring.
7. Implement scroll lock acquisition/release tied to open state.
8. Implement inert attribute management and `DIALOG_STACK` integration for nested dialogs.
9. Implement `Title` with heading-level rendering and `RegisterTitle` event dispatch.
10. Implement `Description` with `RegisterDescription` event dispatch.
11. Implement `CloseTrigger` with close handler and `aria-label` from Messages.
12. Implement focus restoration with fallback chain on close.
13. Verify SSR trigger rendering and client-only overlay mount.
14. Verify Desktop and Mobile webview behavior for focus trapping and scroll lock.
15. Verify nested dialog stacking, Escape routing, and focus restoration across nesting levels.

## 18. Anti-Patterns

- Do not render backdrop as a parent of content; they must be siblings in the portal root (backdrop sibling pattern).
- Do not send `Event::CloseOnEscape` without first invoking `on_escape_key_down` with a `PreventableEvent` and checking `is_default_prevented()`.
- Do not send `Event::CloseOnBackdropClick` without first invoking `on_interact_outside` with a `PreventableEvent` and checking `is_default_prevented()`.
- Do not keep a local `is_open` mirror signal; derive open state exclusively from the machine.
- Do not hardcode z-index values; use the `ZIndexAllocator` context.
- Do not render dialog content inline with the trigger; it must go through the portal root.
- Do not acquire scroll lock in nested dialogs; only the outermost dialog in the stack owns the lock.
- Do not attach FocusScope, Dismissable listeners, or Escape handlers during SSR.
- Do not activate FocusScope before `animationstart`; set `tabindex="-1"` on the content container during the animation delay.
- Do not skip the `inert` polyfill fallback when `supports_inert()` returns false.
- Do not remove `inert` from siblings when closing an inner nested dialog; recalculate for the new topmost dialog.
- Do not render `Title` as a generic `<div>`; it must be `<h{level}>` using the `title_level` prop.
- Do not assume browser-only focus dispatch semantics on Desktop or Mobile targets; validate against the actual host.

## 19. Consumer Expectations and Guarantees

- Consumers may assume that `on_open_change` fires after every open/close transition with the new boolean state.
- Consumers may assume that `on_escape_key_down` and `on_interact_outside` are invoked before the close transition, and that calling `prevent_default()` cancels the close.
- Consumers may assume that focus is trapped inside the dialog content when `modal=true`.
- Consumers may assume that focus returns to the trigger (or `final_focus` target) on close when `restore_focus=true`.
- Consumers may assume that the trigger renders during SSR with correct ARIA attributes.
- Consumers may assume that nested dialogs stack correctly with per-dialog Escape handling.
- Consumers may assume that `aria-labelledby` and `aria-describedby` are set before focus moves into the content.
- Consumers must not assume that dialog content is in the DOM during SSR.
- Consumers must not assume that the backdrop is a parent of the content.
- Consumers must not assume that z-index values are predictable; they are allocated dynamically.
- Consumers must not assume that `lazy_mount` content is rendered before the first open.
- Consumers must not assume that Desktop or Mobile focus trapping behaves identically to browser Web.

## 20. Platform Support Matrix

| Capability / behavior        | Web          | Desktop       | Mobile         | SSR            | Notes                                               |
| ---------------------------- | ------------ | ------------- | -------------- | -------------- | --------------------------------------------------- |
| trigger rendering with ARIA  | full support | full support  | full support   | full support   | stable across hydration                             |
| portal content rendering     | full support | full support  | full support   | SSR-safe empty | content renders client-only into portal root        |
| focus trapping (FocusScope)  | full support | fallback path | fallback path  | not applicable | Desktop/Mobile webview may differ in focus dispatch |
| scroll lock                  | full support | fallback path | fallback path  | not applicable | webview scroll behavior may vary                    |
| inert attribute on siblings  | full support | full support  | full support   | not applicable | polyfill path for older webview engines             |
| Escape key dismiss           | full support | full support  | not applicable | not applicable | Mobile lacks physical Escape key                    |
| backdrop click dismiss       | full support | full support  | full support   | not applicable | pointer events work across platforms                |
| z-index allocation           | full support | full support  | full support   | not applicable | client-only context allocation                      |
| Presence animation lifecycle | full support | full support  | full support   | not applicable | CSS animation support in all webviews               |
| focus restoration            | full support | fallback path | fallback path  | not applicable | Desktop/Mobile focus return may differ              |
| nested dialog stacking       | full support | full support  | full support   | not applicable | client-only DIALOG_STACK management                 |

## 21. Debug Diagnostics and Production Policy

| Condition                                           | Debug build behavior | Production behavior | Notes                                     |
| --------------------------------------------------- | -------------------- | ------------------- | ----------------------------------------- |
| Content used outside Dialog                         | fail fast            | fail fast           | `.expect()` on context access             |
| Portal root (ArsProvider) missing                   | fail fast            | fail fast           | cannot render overlay content             |
| ZIndexAllocator context missing                     | debug warning        | degrade gracefully  | fall back to unmanaged z-index            |
| Title not rendered (missing aria-labelledby)        | debug warning        | no-op               | accessibility degradation                 |
| FocusScope activation failure                       | debug warning        | degrade gracefully  | focus may escape modal                    |
| Focus restoration target removed from DOM           | debug warning        | degrade gracefully  | falls back through ancestor chain to body |
| Scroll lock body height decreased during lock       | no-op                | degrade gracefully  | clamp scroll position silently            |
| Inert polyfill active (browser lacks native inert)  | debug warning        | no-op               | polyfill applied transparently            |
| Nested dialog detected with conflicting scroll lock | no-op                | no-op               | inner dialog skips scroll lock by design  |
| Desktop/Mobile webview focus dispatch differs       | debug warning        | degrade gracefully  | validate against actual host behavior     |

## 22. Shared Adapter Helper Notes

| Helper concept                 | Required?   | Responsibility                                                          | Reused by                                                                  | Notes                           |
| ------------------------------ | ----------- | ----------------------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------- |
| portal rendering helper        | required    | render children into ArsProvider portal root                            | dialog, alert-dialog, drawer, popover, tooltip, hover-card, toast          | shared portal insertion/removal |
| z-index allocation helper      | required    | allocate and release z-index slots from ZIndexAllocator                 | dialog, alert-dialog, drawer, popover, tooltip, hover-card, floating-panel | context-based allocation        |
| focus-scope composition helper | required    | compose FocusScope around content with activation lifecycle             | dialog, alert-dialog, drawer                                               | modal focus trapping            |
| dismiss helper                 | required    | compose Dismissable with PreventableEvent gating                        | dialog, alert-dialog, drawer, popover                                      | outside-interaction + Escape    |
| scroll lock helper             | required    | acquire/release body scroll lock with compensation                      | dialog, alert-dialog, drawer                                               | nested dialog aware             |
| inert management helper        | required    | set/clear inert on portal siblings via DIALOG_STACK                     | dialog, alert-dialog, drawer                                               | includes polyfill fallback      |
| focus restoration helper       | recommended | restore focus with fallback chain on overlay close                      | dialog, alert-dialog, drawer, popover                                      | trigger -> ancestor -> body     |
| merge attrs helper             | recommended | merge core attrs with consumer and adapter-owned attrs                  | all overlay components                                                     | `class`/`style` additive merge  |
| warning helper                 | recommended | emit debug-only warnings for missing contexts or degraded behavior      | all overlay components                                                     | compile-gated diagnostics       |
| platform capability helper     | recommended | normalize focus/scroll assumptions for Web, Desktop, and Mobile targets | dialog, alert-dialog, drawer, dismissable                                  | target-specific caveats         |

## 23. Framework-Specific Behavior

Dioxus uses ID-based element access patterns rather than `NodeRef`-based patterns for some DOM operations, since Dioxus signals are `Copy` and the virtual DOM manages element lifecycle. Portal rendering uses `rsx!` fragments rendered into the portal root node obtained from `try_use_context::<ArsContext>()`. Cleanup uses `use_drop` for all registered effects, listeners, and stack entries.

Controlled `open` prop sync uses `use_effect` (not body-level sync) because open/close dispatches events, which is an intentional exception to body-level sync.

FocusScope activation timing: the adapter must wait for `animationstart` (or immediate activation when no animation is configured) before activating FocusScope. During the animation delay, `tabindex="-1"` is set on the content container to prevent premature focus entry.

The `DIALOG_STACK` on Web targets is a `thread_local!` with `RefCell<Vec<String>>` (WASM is single-threaded). On Desktop targets, it uses `std::sync::Mutex<Vec<String>>` since Desktop runs on a multi-threaded runtime, though dialog operations are typically main-thread-only.

On Dioxus Desktop and Mobile targets, focus trapping and scroll lock behavior may differ from browser assumptions because the webview engine version and native window focus model can vary. The adapter must validate these behaviors against the actual host platform rather than assuming browser-identical dispatch. This is especially relevant for:

- **Desktop**: Focus may escape the webview to native window controls. The adapter should document this as a known limitation and degrade gracefully.
- **Mobile**: Virtual keyboard interaction may conflict with scroll lock. The adapter should release scroll lock temporarily when a text input inside the dialog receives focus.

Dioxus callbacks use `.call()` (not `.run()` as in Leptos).

## 24. Canonical Implementation Sketch

```rust
use dioxus::prelude::*;

#[component]
pub fn Dialog(props: DialogProps) -> Element {
    let core_props = dialog::Props {
        open: props.open.map(|s| *s.read()),
        default_open: props.default_open,
        modal: props.modal,
        close_on_backdrop: props.close_on_backdrop,
        close_on_escape: props.close_on_escape,
        prevent_scroll: props.prevent_scroll,
        restore_focus: props.restore_focus,
        initial_focus: props.initial_focus.clone(),
        final_focus: props.final_focus.clone(),
        role: props.role.unwrap_or(dialog::Role::Dialog),
        title_level: props.title_level.unwrap_or(2),
        messages: props.messages.clone(),
        lazy_mount: props.lazy_mount,
        unmount_on_exit: props.unmount_on_exit,
        locale: props.locale.clone(),
        ..Default::default()
    };

    let machine = use_machine::<dialog::Machine>(core_props);
    let is_open = machine.derive(|api| api.is_open());

    // Controlled open prop sync (deferred effect with prev-value guard)
    if let Some(open_sig) = props.open {
        let send = machine.send;
        let mut prev_open: Signal<Option<bool>> = use_signal(|| None);
        use_effect(move || {
            let new_open = *open_sig.read();
            let prev = prev_open.read().clone();
            if prev.as_ref() != Some(&new_open) {
                if prev.is_some() {
                    if new_open {
                        send.call(dialog::Event::Open);
                    } else {
                        send.call(dialog::Event::Close);
                    }
                }
                *prev_open.write() = Some(new_open);
            }
        });
    }

    // Notify consumer of open state changes
    let on_open_change = props.on_open_change.clone();
    use_effect(move || {
        let open_val = is_open();
        if let Some(ref cb) = on_open_change {
            cb.call(open_val);
        }
    });

    let title_id = machine.derive(|api| api.title_id().to_string());
    let description_id = machine.derive(|api| api.description_id().to_string());
    let trigger_id = machine.derive(|api| api.trigger_id().to_string());
    let content_id = machine.derive(|api| api.content_id().to_string());

    use_context_provider(|| Context {
        send: machine.send,
        open: is_open,
        trigger_id,
        content_id,
        title_id,
        description_id,
        has_title: use_signal(|| false),
        has_description: use_signal(|| false),
        modal: props.modal,
        role: props.role.unwrap_or(dialog::Role::Dialog),
        // ... remaining fields
    });

    rsx! { {props.children} }
}

#[component]
pub fn Trigger(props: TriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("dialog::Trigger must be used inside Dialog");

    rsx! {
        button {
            r#type: "button",
            id: "{ctx.trigger_id}",
            "data-ars-scope": "dialog",
            "data-ars-part": "trigger",
            "aria-haspopup": "dialog",
            "aria-expanded": "{ctx.open}",
            "aria-controls": "{ctx.content_id}",
            onclick: move |_| ctx.send.call(dialog::Event::Toggle),
            {props.children}
        }
    }
}

#[component]
pub fn Content(props: ContentProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("dialog::Content must be used inside Dialog");

    // Presence composition
    let presence = use_machine::<presence::Machine>(presence::Props::default());
    use_effect(move || {
        if ctx.open() {
            presence.send.call(presence::Event::Show);
        } else {
            presence.send.call(presence::Event::Hide);
        }
    });
    let is_present = presence.derive(|api| api.is_present());

    // Escape key with PreventableEvent gating
    let on_escape = ctx.on_escape_key_down;
    let send = ctx.send;

    if !is_present() {
        return rsx! {};
    }

    rsx! {
        // Backdrop — sibling of Positioner
        div {
            "data-ars-scope": "dialog",
            "data-ars-part": "backdrop",
            "data-ars-state": if ctx.open() { "open" } else { "closed" },
            "aria-hidden": "true",
            inert: true,
            onclick: move |_| {
                let mut preventable = PreventableEvent::new();
                if let Some(ref cb) = ctx.on_interact_outside {
                    cb.call(preventable.clone());
                }
                if !preventable.is_default_prevented() {
                    send.call(dialog::Event::CloseOnBackdropClick);
                }
            },
        }
        // Positioner — sibling of Backdrop
        div {
            "data-ars-scope": "dialog",
            "data-ars-part": "positioner",
            // Content
            div {
                id: "{ctx.content_id}",
                role: if ctx.role == dialog::Role::AlertDialog { "alertdialog" } else { "dialog" },
                "aria-modal": if ctx.modal { "true" } else { "false" },
                "aria-labelledby": if ctx.has_title() { ctx.title_id.to_string() } else { String::new() },
                "aria-describedby": if ctx.has_description() { ctx.description_id.to_string() } else { String::new() },
                "data-ars-scope": "dialog",
                "data-ars-part": "content",
                "data-ars-state": if ctx.open() { "open" } else { "closed" },
                onkeydown: move |e: KeyboardEvent| {
                    if e.key() == Key::Escape {
                        let mut preventable = PreventableEvent::new();
                        if let Some(ref cb) = on_escape {
                            cb.call(preventable.clone());
                        }
                        if !preventable.is_default_prevented() {
                            send.call(dialog::Event::CloseOnEscape);
                        }
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
        .expect("dialog::Title must be used inside Dialog");
    ctx.send.call(dialog::Event::RegisterTitle);
    *ctx.has_title.write() = true;

    rsx! {
        h2 {
            id: "{ctx.title_id}",
            "data-ars-scope": "dialog",
            "data-ars-part": "title",
            {props.children}
        }
    }
}

#[component]
pub fn Description(props: DescriptionProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("dialog::Description must be used inside Dialog");
    ctx.send.call(dialog::Event::RegisterDescription);
    *ctx.has_description.write() = true;

    rsx! {
        p {
            id: "{ctx.description_id}",
            "data-ars-scope": "dialog",
            "data-ars-part": "description",
            {props.children}
        }
    }
}

#[component]
pub fn CloseTrigger(props: CloseTriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("dialog::CloseTrigger must be used inside Dialog");
    let close_label = machine.derive(|api| api.close_trigger_attrs());

    rsx! {
        button {
            r#type: "button",
            "data-ars-scope": "dialog",
            "data-ars-part": "close-trigger",
            "aria-label": "{close_label}",
            onclick: move |_| ctx.send.call(dialog::Event::Close),
            {props.children}
        }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
// ── Dialog ──
let machine = use_machine::<dialog::Machine>(core_props);
let is_open = machine.derive(|api| api.is_open());
sync_controlled_open(open_signal, machine.send, prev_open_guard);
notify_on_open_change(is_open, on_open_change);
use_context_provider(|| build_dialog_context(machine));
// render: {props.children}

// ── Trigger ──
let ctx = require_dialog_context();
let trigger_attrs = derive_trigger_attrs(ctx);
// render: button { ..trigger_attrs, onclick: toggle, {children} }

// ── Content (complex — full sequencing) ──
let ctx = require_dialog_context();

// 1. Presence composition
let presence = use_machine::<presence::Machine>(presence_props);
sync_open_to_presence(ctx.open, presence.send);
let is_present = presence.derive(|api| api.is_present());

// 2. Portal rendering
let portal_root = require_environment_portal_root();

// 3. Z-index allocation
let z_index = allocate_z_index_from_context();

// 4. Backdrop with PreventableEvent gating
let on_backdrop_click = create_preventable_backdrop_handler(
    ctx.on_interact_outside,
    ctx.send,
);

// 5. Escape key with PreventableEvent gating
let on_escape = create_preventable_escape_handler(
    ctx.on_escape_key_down,
    ctx.send,
);

// 6. Client-only effects (gated behind is_present)
use_effect(move || {
    if !is_present() { return; }

    // 6a. Scroll lock (outermost dialog only)
    let scroll_cleanup = if is_outermost_in_stack() && ctx.prevent_scroll {
        Some(prevent_body_scroll())
    } else {
        None
    };

    // 6b. Inert management via DIALOG_STACK
    if ctx.modal {
        dialog_stack_push(&ctx.dialog_id);
    }

    // 6c. FocusScope activation (after animationstart)
    let focus_cleanup = activate_focus_scope_after_animation(
        content_ref,
        ctx.initial_focus,
        ctx.modal,
    );
});

use_drop(move || {
    // Reverse order cleanup
    deactivate_focus_scope();
    if ctx.modal {
        dialog_stack_pop(&ctx.dialog_id);
    }
    release_scroll_lock();
    if ctx.restore_focus {
        restore_focus_with_fallback(&ctx.trigger_id);
    }
    release_z_index(z_index);
});

// render (when present):
//   div { ..backdrop_attrs, onclick: on_backdrop_click }
//   div { ..positioner_attrs, style: z_index_style,
//     div { ..content_attrs, onkeydown: on_escape,
//       {children}
//     }
//   }

// ── Title ──
let ctx = require_dialog_context();
ctx.send.call(dialog::Event::RegisterTitle);
// render: h{level} { ..title_attrs, {children} }

// ── Description ──
let ctx = require_dialog_context();
ctx.send.call(dialog::Event::RegisterDescription);
// render: p { ..description_attrs, {children} }

// ── CloseTrigger ──
let ctx = require_dialog_context();
// render: button { ..close_trigger_attrs, onclick: close, {children} }
```

## 26. Adapter Invariants

- The backdrop and positioner/content MUST be siblings in the portal root, never parent-child.
- `Event::CloseOnEscape` MUST NOT be sent without first invoking `on_escape_key_down` with a `PreventableEvent` and checking `is_default_prevented()`.
- `Event::CloseOnBackdropClick` MUST NOT be sent without first invoking `on_interact_outside` with a `PreventableEvent` and checking `is_default_prevented()`.
- Escape key MUST route to the topmost dialog in `DIALOG_STACK` only.
- FocusScope MUST NOT activate until after `animationstart` fires (or immediately when no animation is configured).
- `aria-labelledby` and `aria-describedby` MUST be set on the content element BEFORE focus moves into the dialog.
- Scroll lock MUST be owned by the outermost dialog in the stack. Inner dialogs skip scroll lock acquisition.
- `dialog_stack_pop()` MUST be called during the close transition before the next event can be processed.
- Inert recalculation on nested dialog close MUST re-apply inert for the new topmost dialog, not simply remove all inert attributes.
- Content MUST render into the portal root obtained from ArsProvider context.
- Z-index MUST be allocated from ZIndexAllocator context, not hardcoded.
- Controlled `open` sync MUST use a deferred `use_effect` with prev-value guard to avoid open/close loops.
- No DOM side effects (scroll lock, inert, listeners, focus) may execute during SSR.
- `Title` MUST render as `<h{level}>` using the `title_level` prop (clamped 1..=6), not a generic `<div>`.
- Focus restoration on close MUST follow the fallback chain: original trigger -> nearest focusable ancestor -> `<body>`.
- On Desktop and Mobile targets, focus trapping and scroll lock behavior MUST be validated against the actual host platform, not assumed to match browser behavior.
- Dioxus callbacks use `.call()`, not `.run()`.

## 27. Accessibility and SSR Notes

- The trigger renders with `aria-haspopup="dialog"`, `aria-expanded`, and `aria-controls` during SSR. This gives screen readers correct semantics before client hydration.
- During SSR with `default_open: true`, the trigger shows `aria-expanded="true"` but dialog content is not present. Screen readers will not find the referenced `aria-controls` target until the client mounts. This is acceptable because the dialog is interactive-only.
- `aria-modal="true"` on the content element works together with `inert` on background siblings to fully contain the screen reader virtual cursor. The `inert` attribute prevents NVDA browse mode and VoiceOver virtual cursor from escaping the dialog.
- When `inert` is not supported (older webview engines on Desktop/Mobile), the adapter falls back to `aria-hidden="true"` plus `tabindex="-1"` on all tabbable elements in background siblings, plus a document-level Tab trap listener.
- `Title` registers via `Event::RegisterTitle` so that `aria-labelledby` is set before focus moves into the content. This ensures screen readers announce the title on focus entry.
- `Description` registers via `Event::RegisterDescription` for `aria-describedby` wiring.
- The `CloseTrigger` receives its `aria-label` from `Messages.close_label`, resolved with the current locale.
- A 100ms delay between DOM insertion and focus move allows screen readers to register the new dialog landmark.
- On Desktop targets, screen reader behavior depends on the webview engine (WebKitGTK, WebView2, etc.) and may differ from browser Chrome/Firefox/Safari. The adapter should document known differences as platform notes.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, event, part, and behavior parity. All eight parts (Root, Trigger, Backdrop, Positioner, Content, Title, Description, CloseTrigger) are rendered. All core features (modal/non-modal, focus trapping, scroll lock, inert background, nested dialogs, Presence animation, preventable dismiss, lazy mount, focus restoration fallback, role=alertdialog) are implemented.

Intentional deviations: none.

Traceability note: This adapter spec makes explicit the following core adapter-owned concerns: PreventableEvent gating before dismiss events, portal rendering via ArsProvider, z-index allocation via ZIndexAllocator, FocusScope composition with animation-aware activation timing, Dismissable composition for outside-interaction detection, DIALOG_STACK management for nested dialogs with inert recalculation, scroll lock with nested-dialog awareness and scrollbar-width compensation, focus restoration fallback chain, Title/Description ID registration for aria-labelledby/describedby wiring, SSR trigger rendering with client-only overlay mount, inert polyfill fallback, and multi-platform (Web/Desktop/Mobile) focus and scroll lock validation.

## 29. Test Scenarios

- dialog opens on trigger click and closes on CloseTrigger click
- controlled `open` Signal drives open/close state
- `on_open_change` callback fires with correct boolean on both open and close
- `on_escape_key_down` fires before close; calling `prevent_default()` prevents close
- `on_interact_outside` fires before close on backdrop click; calling `prevent_default()` prevents close
- backdrop and content are siblings in the portal root (not parent-child)
- focus trapping: Tab/Shift+Tab cycle within content when modal
- focus moves to `initial_focus` target on open
- focus returns to trigger (or `final_focus` target) on close
- focus restoration fallback when trigger removed from DOM
- scroll lock applied on open, restored on close with scrollbar-width compensation
- inert attribute set on background siblings when modal; removed on close
- nested dialog: inner Escape closes inner only; outer remains open
- nested dialog: closing inner restores focus to element within outer dialog
- nested dialog: scroll lock remains until all dialogs close
- lazy_mount: content not in DOM until first open
- unmount_on_exit: content removed from DOM after close
- Presence animation: entry animation plays after mount, exit animation plays before unmount
- SSR: trigger renders with ARIA attrs; content not rendered
- Title registers `aria-labelledby` before focus enters content
- Description registers `aria-describedby`
- `role="alertdialog"` renders correct role on content
- z-index allocated from ZIndexAllocator, not hardcoded
- Desktop: focus trapping validated against webview host
- Desktop: scroll lock validated against webview scroll behavior
- Mobile: Escape key behavior documented as not applicable

## 30. Test Oracle Notes

| Behavior                                          | Preferred oracle type | Notes                                                                                                            |
| ------------------------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------------------------- |
| trigger ARIA attrs (haspopup, expanded, controls) | DOM attrs             | assert attrs present on trigger element                                                                          |
| content role and aria-modal                       | DOM attrs             | assert `role="dialog"` and `aria-modal="true"`                                                                   |
| aria-labelledby / aria-describedby wiring         | DOM attrs             | assert IDs match title/description element IDs                                                                   |
| backdrop sibling pattern                          | rendered structure    | assert backdrop and positioner are siblings, not parent-child                                                    |
| open/close state transitions                      | machine state         | assert `api.is_open()` matches expected state                                                                    |
| on_open_change callback                           | callback order        | assert callback fires after state transition with correct boolean                                                |
| PreventableEvent gating                           | callback order        | assert `on_escape_key_down` / `on_interact_outside` fire before close event; assert prevented event blocks close |
| focus trapping                                    | DOM attrs             | assert FocusScope active; Tab wraps within content                                                               |
| focus restoration                                 | cleanup side effects  | assert focus returns to trigger or fallback target                                                               |
| scroll lock                                       | DOM attrs             | assert body has `overflow: hidden` when open; restored on close                                                  |
| inert on siblings                                 | DOM attrs             | assert `inert` attribute on portal root siblings when modal                                                      |
| nested dialog stacking                            | machine state         | assert stack ordering matches open order; Escape targets topmost                                                 |
| Presence mount/unmount                            | rendered structure    | assert content in/out of DOM matches Presence state                                                              |
| SSR trigger rendering                             | hydration structure   | assert trigger HTML includes all ARIA attrs; content absent                                                      |
| Desktop focus behavior                            | DOM attrs             | validate FocusScope against webview host focus dispatch                                                          |

Cheap verification recipe:

1. Render a dialog with trigger, content, title, and close trigger. Assert the trigger has `aria-haspopup="dialog"` and `aria-expanded="false"`.
2. Click the trigger. Assert content appears in portal root with `role="dialog"`, `aria-modal="true"`, and `aria-labelledby` matching the title ID.
3. Assert backdrop and positioner are siblings inside the portal root.
4. Press Escape. Assert `on_escape_key_down` fires before the dialog closes. Repeat with `prevent_default()` and assert dialog remains open.
5. Open again, click backdrop. Assert `on_interact_outside` fires before close. Repeat with `prevent_default()`.
6. Open nested dialog inside content. Press Escape. Assert inner closes, outer remains. Press Escape again, assert outer closes.
7. Unmount and assert all cleanup effects (scroll lock, inert, FocusScope, dialog stack, z-index) are released.
8. On Dioxus Desktop, repeat steps 2-6 and validate focus trapping and scroll lock against the webview host rather than only a browser harness.

## 31. Implementation Checklist

- [ ] `Dialog` initializes machine with full props and provides `Context` via `use_context_provider`.
- [ ] Controlled `open` prop sync uses deferred `use_effect` with prev-value guard.
- [ ] `Trigger` renders with all core trigger attrs and toggle handler.
- [ ] Backdrop and positioner render as siblings in portal root (backdrop sibling pattern).
- [ ] `Backdrop` applies `aria-hidden="true"`, `inert`, state attrs, and click handler with PreventableEvent gating.
- [ ] `Positioner` receives z-index from ZIndexAllocator.
- [ ] `Content` renders with `role`, `aria-modal`, `aria-labelledby`, `aria-describedby`, and state attrs.
- [ ] Escape key handler invokes `on_escape_key_down` before sending `Event::CloseOnEscape`.
- [ ] Backdrop click handler invokes `on_interact_outside` before sending `Event::CloseOnBackdropClick`.
- [ ] FocusScope composes inside content with animation-aware activation timing.
- [ ] Scroll lock acquired on open (outermost dialog only), released on close with compensation.
- [ ] Inert attribute set on background siblings via DIALOG_STACK; recalculated on nested close.
- [ ] `Title` renders as `<h{level}>` and dispatches `Event::RegisterTitle`.
- [ ] `Description` dispatches `Event::RegisterDescription`.
- [ ] `CloseTrigger` renders with `aria-label` from Messages and close handler.
- [ ] Presence machine composes for lazy mount, unmount-on-exit, and animation lifecycle.
- [ ] Focus restoration follows fallback chain on close.
- [ ] SSR renders trigger with ARIA attrs; content is client-only.
- [ ] `on_open_change` fires after every open/close transition.
- [ ] Nested dialog stacking, Escape routing, and focus restoration verified.
- [ ] All cleanup effects (scroll lock, inert, FocusScope, Dismissable, dialog stack, z-index, portal) execute via `use_drop`.
- [ ] No DOM side effects during SSR.
- [ ] Desktop/Mobile focus trapping and scroll lock validated against webview host.
- [ ] Callbacks use `.call()` (not `.run()`).
