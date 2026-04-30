---
component: Dialog
category: overlay
tier: complex
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [z-index-stacking]
related: []
references:
    ark-ui: Dialog
    radix-ui: Dialog
    react-aria: Dialog
---

# Dialog

A modal or non-modal overlay that requires user interaction before returning to the main content.

## 1. State Machine

### 1.1 States

```rust
/// States for the `Dialog` component.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The dialog is closed.
    #[default]
    Closed,
    /// The dialog is open.
    Open,
}
```

### 1.2 Events

The event enum contains only state-control events. Animation lifecycle
(mount, unmount, animation start/end) is owned entirely by the
[`Presence`](./presence.md) machine and composed at the adapter layer (see
[§5](#5-animation-lifecycle-presence-composition)); Dialog never reacts to
Presence events directly.

```rust
/// Events for the `Dialog` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Open the dialog.
    Open,
    /// Close the dialog.
    Close,
    /// Toggle the dialog.
    Toggle,
    /// Close the dialog on backdrop click.
    CloseOnBackdropClick,
    /// Close the dialog on escape key.
    CloseOnEscape,
    /// Register the title of the dialog.
    RegisterTitle,
    /// Register the description of the dialog.
    RegisterDescription,
    /// Re-apply context-backed `Props` fields after a prop change. Emitted
    /// from [`on_props_changed`](#19-full-machine-implementation) when any
    /// non-`open` field that drives `Context` differs between old and new
    /// props (`modal`, `close_on_backdrop`, `close_on_escape`,
    /// `prevent_scroll`, `restore_focus`, `initial_focus`, `final_focus`,
    /// `role`). The transition is context-only — no state change, no
    /// adapter intents emitted.
    SyncProps,
}
```

### 1.3 Context

```rust
/// Context for the `Dialog` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the dialog is open.
    pub open: bool,
    /// Whether the dialog is modal.
    pub modal: bool,
    /// Whether the dialog is closeable on backdrop click.
    pub close_on_backdrop: bool,
    /// Whether the dialog is closeable on escape key.
    pub close_on_escape: bool,
    /// Whether the dialog should prevent scroll.
    pub prevent_scroll: bool,
    /// Whether the dialog should restore focus.
    pub restore_focus: bool,
    /// The initial focus target.
    pub initial_focus: Option<FocusTarget>,
    /// The final focus target.
    pub final_focus: Option<FocusTarget>,
    /// The role of the dialog.
    pub role: Role,
    /// Hydration-stable IDs derived from `Props::id`. Adapters render
    /// each part's `id` attribute via `ids.part("trigger" | "content" |
    /// "title" | "description")`; ARIA wiring (`aria-controls`,
    /// `aria-labelledby`, `aria-describedby`) reads from the same
    /// `part(...)` lookup. This matches the workspace convention shared
    /// with `form`, `field`, `fieldset`, `checkbox`, `textarea`,
    /// `text_field`, and `date_field`.
    pub ids: ComponentIds,
    /// Whether the dialog has a title.
    pub has_title: bool,
    /// Whether the dialog has a description.
    pub has_description: bool,
    /// The current locale for message resolution.
    pub locale: Locale,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

/// The role of the dialog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Role {
    /// The dialog role.
    #[default]
    Dialog,
    /// The alert dialog role.
    AlertDialog,
}

impl Role {
    /// ARIA role token rendered on the content element. Single source of
    /// truth for the `role="dialog"` / `role="alertdialog"` mapping; both
    /// the agnostic `Api::content_attrs` and adapter-side test fixtures
    /// must read the role string through this method.
    pub const fn as_aria_role(self) -> &'static str {
        match self {
            Self::Dialog => "dialog",
            Self::AlertDialog => "alertdialog",
        }
    }
}

// `FocusTarget` — defined in `03-accessibility.md`
```

### 1.4 Props

```rust
/// An event that the consumer can prevent the default behavior of.
/// Used for dismissal callbacks (`on_escape_key_down`, `on_interact_outside`)
/// to let consumers intercept and cancel the default close behavior.
///
/// Example: preventing close when there are unsaved changes.
///
/// The veto flag is shared through `Arc<AtomicBool>` so the value can be
/// passed by-clone into [`Callback`], which requires `Args: 'static` and
/// therefore cannot accept `&mut PreventableEvent`. The same shared-veto
/// pattern is used by [`DismissAttempt`](../utility/dismissable.md).
#[derive(Clone, Debug, Default)]
pub struct PreventableEvent {
    veto: Arc<AtomicBool>,
}

impl PreventableEvent {
    pub fn new() -> Self {
        Self { veto: Arc::new(AtomicBool::new(false)) }
    }
    /// Prevent the default behavior (e.g., prevent dialog from closing). Idempotent.
    pub fn prevent_default(&self) {
        self.veto.store(true, Ordering::SeqCst);
    }
    /// Whether `prevent_default()` was called on this event or any of its clones.
    pub fn is_default_prevented(&self) -> bool {
        self.veto.load(Ordering::SeqCst)
    }
}

/// The props of the dialog.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the component.
    pub id: String,
    /// Whether the dialog is open.
    pub open: Option<bool>,
    /// Whether the dialog is default open.
    pub default_open: bool,
    /// Whether the dialog is modal.
    pub modal: bool,
    /// Whether the dialog is closeable on backdrop click.
    pub close_on_backdrop: bool,
    /// Whether the dialog is closeable on escape key.
    pub close_on_escape: bool,
    /// Whether the dialog should prevent scroll.
    pub prevent_scroll: bool,
    /// Whether the dialog should restore focus.
    pub restore_focus: bool,
    /// The initial focus target.
    pub initial_focus: Option<FocusTarget>,
    /// The final focus target.
    pub final_focus: Option<FocusTarget>,
    /// The role of the dialog.
    pub role: Role,
    /// Heading level for the Title part (renders as `<h{level}>`).
    /// Clamped to 1..=6. Default: `2`.
    pub title_level: u8,
    /// When true, dialog content is not mounted until first opened.
    /// Useful for heavy content that should not render until needed. Default: false.
    pub lazy_mount: bool,
    /// When true, dialog content is removed from the DOM after closing.
    /// Works with Presence for exit animations. Default: false.
    pub unmount_on_exit: bool,
    /// Callback invoked when the dialog open state changes.
    /// Fires after the transition with the new open state value.
    pub on_open_change: Option<Callback<bool>>,
    /// Callback invoked when Escape is pressed while the dialog is open.
    /// The adapter passes a clone of the [`PreventableEvent`] it constructed;
    /// the consumer may call `event.prevent_default()` to prevent the dialog
    /// from closing (the veto flag is shared between clones). Fires before
    /// the close transition — if prevented, the transition is cancelled.
    pub on_escape_key_down: Option<Callback<dyn Fn(PreventableEvent) + Send + Sync>>,
    /// Callback invoked when a pointer down or focus event occurs outside the dialog content.
    /// The adapter passes a clone of the [`PreventableEvent`] it constructed;
    /// the consumer may call `event.prevent_default()` to prevent the dialog
    /// from closing (the veto flag is shared between clones). Fires before
    /// the close transition — if prevented, the transition is cancelled.
    pub on_interact_outside: Option<Callback<dyn Fn(PreventableEvent) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: false,
            modal: true,
            close_on_backdrop: true,
            close_on_escape: true,
            prevent_scroll: true,
            restore_focus: true,
            initial_focus: None,
            final_focus: None,
            role: Role::Dialog,
            title_level: 2,
            lazy_mount: false,
            unmount_on_exit: false,
            on_open_change: None,
            on_escape_key_down: None,
            on_interact_outside: None,
        }
    }
}
```

`Props` also exposes a fluent builder following the workspace convention
established by [`Presence`](./presence.md) and [`Tooltip`](./tooltip.md):
`Props::new()` returns the default value, and a chained setter exists for
each field (`.id(...)`, `.open(...)`, `.modal(...)`, `.on_open_change(...)`,
…). The builder is purely ergonomic — it carries no semantics beyond
mutating the returned `Props`.

> **Adapter-only props.** `lazy_mount` and `unmount_on_exit` are _adapter
> hints_: they configure how the framework adapter composes with
> [`Presence`](./presence.md) (deferred content rendering and
> unmount-after-exit-animation respectively) and **do not affect the
> agnostic state machine**. `transition()` does not read either flag, and
> they are not reflected in any [§1.11](#111-adapter-intent-contract)
> intent. Adapters access them via `Api::lazy_mount()` /
> `Api::unmount_on_exit()` (or directly via `props`) when wiring the
> Presence composition described in [§5](#5-animation-lifecycle-presence-composition).

### 1.5 Scroll Lock Restoration Edge Cases

1. **Scrollbar Width Compensation**: When `overflow: hidden` is applied to the body, add `padding-right` equal to the scrollbar width (`window.innerWidth - document.documentElement.clientWidth`) to prevent layout shift.
2. **Body Height Decrease During Lock**: On restoration, clamp `scrollY` to `document.documentElement.scrollHeight - window.innerHeight` to avoid scrolling past the new document end if content was removed while locked.
3. **Smooth Scroll Interference**: If `scroll-behavior: smooth` is set on the body, temporarily override it to `auto` during scroll position restoration to prevent animated jumps. Restore the original value after `scrollTo()` completes.
4. **Nested Dialog Scroll Lock**: The outermost dialog in the `DIALOG_STACK` owns the scroll lock. Inner dialogs must skip scroll lock acquisition and release. Only when the last dialog is removed from the stack should scroll lock be released.

### 1.6 Inert Attribute Polyfill

The `inert` attribute is used to make background content non-interactive when a modal dialog is
open. Since `inert` is not universally supported, `ars-dom` provides feature detection and a
fallback strategy.

**Feature Detection**:

```rust
// ars-dom/src/inert.rs

/// Returns `true` if the browser natively supports the `inert` attribute.
/// Minimum browser versions: Chrome 102+, Firefox 112+, Safari 15.5+.
pub fn supports_inert() -> bool {
    // Check if 'inert' property exists on HTMLElement.prototype
    let Ok(html_element_ctor) =
        js_sys::Reflect::get(&js_sys::global(), &"HTMLElement".into())
    else {
        return false;
    };
    let Ok(prototype) = js_sys::Reflect::get(&html_element_ctor, &"prototype".into()) else {
        return false;
    };
    js_sys::Reflect::has(&prototype, &"inert".into()).unwrap_or(false)
}
```

**Fallback when `inert` is not supported**: When `supports_inert()` returns `false`, the
`set_background_inert()` function applies the following polyfill to each sibling of the
dialog's portal root:

1. **Set `aria-hidden="true"`** on each sibling element, storing the original `aria-hidden`
   value (if any) for later restoration.
2. **Collect all tabbable elements** within each sibling (elements matching
   `a[href], button, input, select, textarea, [tabindex]` that are not already `disabled`
   or `tabindex="-1"`). For each tabbable element:
    - Store the original `tabindex` value.
    - Set `tabindex="-1"` to remove it from the tab order.
3. **Add a document-level `keydown` listener** that traps Tab key navigation within the
   dialog content. On Tab: if focus would leave the dialog, wrap to the first/last
   tabbable element inside the dialog.

**Cleanup**: The returned cleanup function restores all original `aria-hidden` and `tabindex`
values and removes the document-level `keydown` listener. This cleanup closure is the
authoritative teardown path for background inert state. Any direct helper such as
`remove_inert_from_siblings()` is best-effort only and MUST NOT be treated as a full
replacement for the stored `set_background_inert()` cleanup.

```rust
pub fn set_background_inert(portal_id: &str) -> Box<dyn FnOnce()> {
    if supports_inert() {
        // Native path: set `inert` attribute on siblings
        set_inert_native(portal_id)
    } else {
        // Polyfill path: aria-hidden + tabindex manipulation + Tab trapping
        set_inert_polyfill(portal_id)
    }
}
```

### 1.7 Nested Dialogs

When a `Dialog` opens within another `Dialog` (e.g., a confirmation dialog inside a settings dialog):

1. **Focus scope stacking**: The inner `Dialog` becomes the active `FocusScope`. The outer `Dialog`'s `FocusScope` is suspended (not destroyed).
2. **Closing restores outer scope**: Closing the inner `Dialog` reactivates the outer `Dialog`'s `FocusScope`. Focus returns to the element that opened the inner `Dialog`.
3. **Inert management**: Only the content _behind the topmost_ `Dialog` receives the `inert` attribute. The outer `Dialog`'s content does **not** become inert — the inner `Dialog`'s backdrop covers it visually, and focus trapping prevents keyboard escape, but the outer `Dialog` remains in the DOM and accessible to the stacking logic.
4. **Backdrop interaction**: Only the topmost `Dialog`'s backdrop intercepts pointer events. Clicking the inner `Dialog`'s backdrop closes the inner `Dialog` (if `closeOnInteractOutside` is enabled), not the outer one.
5. **Escape key**: Escape closes the topmost `Dialog` only. A second Escape press closes the next `Dialog` in the stack, and so on.
6. **Scroll lock**: Scroll lock applied by the first `Dialog` remains active until all nested `Dialogs` are closed.

This stacking behavior is managed automatically by the `FocusScope` utility (see `ars-a11y`), which maintains an internal stack of active scopes.

#### 1.7.1 DialogStack: Global Nested Dialog Management

To correctly manage the `inert` attribute across nested modal dialogs, the adapter MUST maintain a global dialog stack:

```rust
use std::sync::Mutex;

/// Global stack tracking open modal dialogs in order.
/// Only the topmost dialog's siblings are non-inert; all others are inert.
static DIALOG_STACK: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Push a dialog onto the stack when it opens.
pub fn dialog_stack_push(dialog_id: &str) {
    let mut stack = DIALOG_STACK.lock().expect("dialog stack poisoned");
    // Set siblings of new dialog as inert
    set_background_inert(dialog_id);
    stack.push(dialog_id.to_string());
}

/// Pop a dialog from the stack when it closes.
pub fn dialog_stack_pop(dialog_id: &str) {
    let mut stack = DIALOG_STACK.lock().expect("dialog stack poisoned");
    stack.retain(|id| id != dialog_id);
    // Clear inert from closed dialog's siblings
    clear_background_inert(dialog_id);
    // Re-apply inert for the new topmost dialog (if any)
    if let Some(top_id) = stack.last() {
        set_background_inert(top_id);
    }
}
```

**Invariants**:

1. **One inert set at a time**: Only the topmost dialog's siblings are marked `inert`. When a new dialog opens, the previous inert markers are subsumed (the new dialog's parent, which includes the old dialog, becomes part of the new inert set).
2. **Correct restoration on pop**: When the topmost dialog closes, inert is re-applied for the new topmost dialog. This ensures `Dialog` A's siblings remain inert after `Dialog` B (opened from within A) closes.
3. **Clean slate on empty**: When the last dialog closes and the stack is empty, ALL `inert` attributes set by the dialog system are cleared. No elements remain inert.
4. **Stack ordering**: Dialogs are always pushed/popped in LIFO order. If a non-topmost dialog closes (unusual but possible via programmatic close), it is removed from its position and inert is recalculated for the current top.

**Dialog Open/Close Integration**: The agnostic state machine emits the
`dialog-set-background-inert` and `dialog-remove-background-inert` intents (see
[§1.11](#111-adapter-intent-contract)). When the adapter receives those intents it MUST
call `dialog_stack_push()` / `dialog_stack_pop()`, passing identifiers it owns
internally — the dialog stack is an adapter-internal data structure. The agnostic core
itself never references the stack.

#### 1.7.2 Escape Key Behavior in Nested Overlays

When multiple overlays are stacked (e.g., a `Dialog` containing a `Select` dropdown containing a `Tooltip`):

1. **Innermost First**: Escape closes the innermost (topmost in `DIALOG_STACK`) overlay only. The event does NOT propagate to outer overlays.
2. **Event Consumption**: The overlay that handles Escape calls `event.stopPropagation()` after closing. Outer overlays do not see the event.
3. **Consumer Opt-Out**: Components can set `close_on_escape: false` to ignore Escape. In this case, the event propagates to the next overlay in the stack.
4. **Non-Modal Overlays**: Tooltips and non-modal popovers also participate in Escape handling. A visible tooltip closes on Escape before the underlying dialog would.
5. **Focus Restoration**: After closing, focus returns to the element that triggered the closed overlay (the trigger element), not to the parent overlay's content.

#### 1.7.3 Escape Key Routing and Deduplication

When multiple dialogs are stacked, Escape key handling MUST route to the topmost dialog only and handle rapid repeated presses gracefully:

**Topmost-only routing**: The global `keydown` handler for Escape MUST consult `DIALOG_STACK.last()` to determine which dialog should receive the event. Only the dialog whose `dialog_id` matches the topmost stack entry processes the Escape. All other open dialogs ignore it. This guarantees that pressing Escape always closes the innermost (most recently opened) dialog first.

**`close_on_escape` guard**: The dialog state machine's Escape transition already includes an early-exit guard: `if !props.close_on_escape { return None; }`. This correctly prevents Escape from closing dialogs that have opted out. When the topmost dialog has `close_on_escape: false`, the Escape key is consumed (to prevent it from propagating to a parent dialog) but no close transition occurs.

**Rapid Escape deduplication**: No special deduplication logic is needed for rapid repeated Escape presses. This is naturally handled by the state machine: the first Escape transitions the topmost dialog from `Open → Closing` (or directly to `Closed` if animations are skipped). A second Escape arriving while the dialog is in `Closing` or `Closed` state is a no-op — the state machine has no valid transition for `Event::Escape` in those states, so it is silently ignored.

**Pop timing guarantee**: The adapter calls `dialog_stack_pop()` while handling the
`dialog-remove-background-inert` adapter intent (see
[§1.11](#111-adapter-intent-contract)) during the close transition. Because adapter
effect handlers run synchronously before the next event is processed, the stack is
updated (and the previously-second dialog becomes topmost) before any subsequent
Escape `keydown` event can be handled. This ensures that the second Escape press — if
it arrives after the first dialog's close intent runs — correctly targets the next
dialog in the stack, not the already-closing one.

#### 1.7.4 Focus Restoration Safety for Nested Dialogs

When multiple dialogs are stacked (`Dialog` A opens `Dialog` B), closing an inner dialog MUST restore focus to the correct element within the parent dialog — not to the parent dialog's original trigger.

**Focus Restoration Stack**: FocusScope maintains a stack of focus restoration targets, not a single `initial_focus` / `final_focus` pair:

1. **Dialog A opens**: FocusScope A records `trigger_a` as restoration target. Focus moves into Dialog A.
2. **User focuses element X inside Dialog A**: FocusScope A tracks `last_focused = X`.
3. **Dialog B opens from within Dialog A**: FocusScope B records `last_focused_in_parent = X` (the element that had focus in A when B opened). Focus moves into Dialog B.
4. **Dialog B closes**: FocusScope B restores focus to `X` (inside Dialog A), NOT to `trigger_a`.
5. **Dialog A closes**: FocusScope A restores focus to `trigger_a`.

**DOM Existence Safety**: Before restoring focus, the restoration target MUST be validated using `restore_focus_safely(target, fallbacks)` -- defined in [`FocusScope` section 1.5.1](../utility/focus-scope.md#151-focus-restoration-safety). The dialog passes its fallback chain as the `fallbacks` slice:

1. The previously focused element in the parent dialog
2. The parent dialog's content container (has `tabindex="-1"`)
3. The parent dialog's first focusable element

If all fallbacks fail, `restore_focus_safely` falls back to the nearest focusable ancestor, then `document.body`.

**Interaction with `restore_focus: false`**: When a dialog sets `restore_focus: false`, that dialog does NOT restore focus on close. However, parent dialogs in the stack are unaffected — they still restore focus normally when they close.

### 1.8 Timing Coordination

**Opening**:

1. `Dialog` state → Open
2. `Presence` mounts the DOM element
3. During animation: set the overlay container as the sole tab target (`tabindex="0"` on container, no FocusScope activation yet) to prevent focus from escaping into background content while the entry animation plays
4. After `animationstart` fires: activate FocusScope (trap focus), move focus to `initial_focus` target
5. ScrollLock applied immediately (step 2)
6. CSS entry animation plays

**Closing**:

1. `Dialog` state → Closed
2. CSS exit animation plays
3. On animationend: FocusScope deactivated, focus restored
4. ScrollLock released
5. Presence unmounts the DOM element

#### 1.8.1 Lazy Content Loading

When using lazy content loading, content MUST be fully rendered before the enter animation starts. The `Presence` machine transitions to `Mounted` state first, which triggers content render; the animation begins only after the content is in the DOM. Use `Suspense` boundaries to handle async content.

#### 1.8.2 Focus Escape During Animation

`FocusScope` MUST NOT activate until the enter animation has started (after
`animationstart` fires). The Content part already carries a static
`tabindex="-1"` (see [§1.10 `content_attrs`](#110-connect--api)), so during
the animation delay the adapter simply moves focus to the Content handle —
no dynamic tabindex toggling is required. The `tabindex="-1"` makes Content
programmatically focusable (`.focus()` works) without making it a Tab stop,
which is the correct steady-state behaviour for a `<div role="dialog">`.

#### 1.8.3 FocusScope Activation Lifecycle

The following sequence governs FocusScope activation for dialog overlays:

1. **During the animation delay period**, set `tabindex="-1"` on the dialog container to prevent premature focus entry.
2. **Wait for the `animationstart` event** (or activate immediately if no animation is configured) before activating FocusScope.
3. **Once FocusScope activates**, move focus to the initial target (`initial_focus` prop or first focusable element).
4. **If the dialog has `lazy_mount=true`**, FocusScope activation waits for BOTH content settlement (`ContentReady`) AND `animationstart`.

### 1.9 Full Machine Implementation

The agnostic core emits **payload-free named effects** only. Adapters subscribe to these
names and resolve element targets through their captured framework handles
(`NodeRef<T>` in Leptos, `MountedData` / `Signal<Option<MountedData>>` in Dioxus) — never
through ID lookup. See [§1.11 Adapter intent contract](#111-adapter-intent-contract) for
the per-name contract.

```rust
use ars_core::{ComponentIds, Env, Machine as MachineTrait, PendingEffect, TransitionPlan};

/// The machine for the `Dialog` component.
pub struct Machine;

impl MachineTrait for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let open = props.open.unwrap_or(props.default_open);
        let state = if open { State::Open } else { State::Closed };
        let ids = ComponentIds::from_id(&props.id);
        (
            state,
            Context {
                open,
                modal: props.modal,
                close_on_backdrop: props.close_on_backdrop,
                close_on_escape: props.close_on_escape,
                prevent_scroll: props.prevent_scroll,
                restore_focus: props.restore_focus,
                initial_focus: props.initial_focus,
                final_focus: props.final_focus,
                role: props.role,
                ids,
                has_title: false,
                has_description: false,
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Closed, Event::Open | Event::Toggle) => {
                let mut plan = TransitionPlan::to(State::Open)
                    .apply(|ctx| {
                        ctx.open = true;
                    })
                    .with_effect(PendingEffect::named(EFFECT_OPEN_CHANGE))
                    .with_effect(PendingEffect::named(EFFECT_FOCUS_INITIAL))
                    .with_effect(PendingEffect::named(EFFECT_FOCUS_FIRST_TABBABLE));
                if ctx.prevent_scroll {
                    plan = plan.with_effect(PendingEffect::named(EFFECT_SCROLL_LOCK_ACQUIRE));
                }
                if ctx.modal {
                    plan = plan.with_effect(PendingEffect::named(EFFECT_SET_BACKGROUND_INERT));
                }
                Some(plan)
            }
            (State::Open, Event::Close | Event::Toggle) => {
                let mut plan = TransitionPlan::to(State::Closed)
                    .apply(|ctx| {
                        ctx.open = false;
                    })
                    .with_effect(PendingEffect::named(EFFECT_OPEN_CHANGE));
                if ctx.prevent_scroll {
                    plan = plan.with_effect(PendingEffect::named(EFFECT_SCROLL_LOCK_RELEASE));
                }
                if ctx.modal {
                    plan = plan.with_effect(PendingEffect::named(EFFECT_REMOVE_BACKGROUND_INERT));
                }
                if ctx.restore_focus {
                    plan = plan.with_effect(PendingEffect::named(EFFECT_RESTORE_FOCUS));
                }
                Some(plan)
            }
            (State::Open, Event::CloseOnBackdropClick) if ctx.close_on_backdrop => Some(
                TransitionPlan::to(State::Closed)
                    .apply(|ctx| {
                        ctx.open = false;
                    })
                    .with_effect(PendingEffect::named(EFFECT_OPEN_CHANGE)),
            ),
            (State::Open, Event::CloseOnEscape) if ctx.close_on_escape => Some(
                TransitionPlan::to(State::Closed)
                    .apply(|ctx| {
                        ctx.open = false;
                    })
                    .with_effect(PendingEffect::named(EFFECT_OPEN_CHANGE)),
            ),
            // Register title/description for aria-labelledby/describedby wiring.
            // Guarded — re-sending after the flag is set is a no-op so the
            // Service does not signal `context_changed = true` spuriously.
            (_, Event::RegisterTitle) if !ctx.has_title => Some(
                TransitionPlan::context_only(|ctx| {
                    ctx.has_title = true;
                }),
            ),
            (_, Event::RegisterDescription) if !ctx.has_description => Some(
                TransitionPlan::context_only(|ctx| {
                    ctx.has_description = true;
                }),
            ),
            // Replay context-backed prop fields after a runtime prop
            // change. Captured by-copy/by-move so the agnostic core does
            // not retain a reference to `props` past the apply closure.
            (_, Event::SyncProps) => {
                let modal = props.modal;
                let close_on_backdrop = props.close_on_backdrop;
                let close_on_escape = props.close_on_escape;
                let prevent_scroll = props.prevent_scroll;
                let restore_focus = props.restore_focus;
                let initial_focus = props.initial_focus;
                let final_focus = props.final_focus;
                let role = props.role;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.modal = modal;
                    ctx.close_on_backdrop = close_on_backdrop;
                    ctx.close_on_escape = close_on_escape;
                    ctx.prevent_scroll = prevent_scroll;
                    ctx.restore_focus = restore_focus;
                    ctx.initial_focus = initial_focus;
                    ctx.final_focus = final_focus;
                    ctx.role = role;
                }))
            }
            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        // Two independent sync axes:
        //
        // 1. **Controlled-mode `open` sync** — when a parent component
        //    flips `props.open` between `Some(true)` ↔ `Some(false)` (or
        //    transitions from uncontrolled to controlled), emit `Open` /
        //    `Close` so the state machine reconciles. The dispatched event
        //    runs `transition`, which emits `EFFECT_OPEN_CHANGE` (§1.11).
        //    Because the consumer initiated the change, the adapter MUST
        //    suppress the resulting `on_open_change` callback to avoid an
        //    echo loop. Adapters typically track the source of the event
        //    in their own dispatch layer.
        //
        // 2. **Context-backed prop sync** — when any non-`open` field that
        //    drives `Context` changes (`modal`, `close_on_backdrop`,
        //    `close_on_escape`, `prevent_scroll`, `restore_focus`,
        //    `initial_focus`, `final_focus`, `role`), emit `SyncProps`.
        //    The corresponding transition replays those props into
        //    `Context` so subsequent state-flipping transitions emit
        //    intents using the freshly-synced configuration.
        //
        // Both events may be emitted from a single `set_props` call; the
        // Service queues them in order and drains them sequentially.
        let mut events = Vec::new();
        if let (was, Some(now)) = (old.open, new.open)
            && was != Some(now)
        {
            events.push(if now { Event::Open } else { Event::Close });
        }
        if context_relevant_props_changed(old, new) {
            events.push(Event::SyncProps);
        }
        events
    }
}

fn context_relevant_props_changed(old: &Props, new: &Props) -> bool {
    old.modal != new.modal
        || old.close_on_backdrop != new.close_on_backdrop
        || old.close_on_escape != new.close_on_escape
        || old.prevent_scroll != new.prevent_scroll
        || old.restore_focus != new.restore_focus
        || old.initial_focus != new.initial_focus
        || old.final_focus != new.final_focus
        || old.role != new.role
}
```

> **Preventable event callbacks.** The `on_escape_key_down` and `on_interact_outside` callbacks are invoked by the adapter layer BEFORE sending the corresponding event to the state machine. If the consumer calls `prevent_default()`, the adapter MUST NOT send the event — the dialog stays open. This pattern keeps the state machine pure (no side-effect callbacks in `transition()`) while giving consumers veto power over dismissal.
>
> **Adapter obligation for `CloseOnEscape`:** Before sending `Event::CloseOnEscape`, the adapter MUST:
>
> 1. Create a `PreventableEvent`
> 2. Invoke `props.on_escape_key_down` with it (if set)
> 3. Only send `Event::CloseOnEscape` if `!event.is_default_prevented()`
>
> **Adapter obligation for `CloseOnBackdropClick`:** Before sending `Event::CloseOnBackdropClick`, the adapter MUST:
>
> 1. Create a `PreventableEvent`
> 2. Invoke `props.on_interact_outside` with it (if set)
> 3. Only send `Event::CloseOnBackdropClick` if `!event.is_default_prevented()`
>
> ```rust
> // Adapter pseudocode for Escape key handling:
> fn on_keydown(event: &KeyboardEvent, props: &Props, send: &dyn Fn(Event)) {
>     if event.key() == "Escape" && props.close_on_escape {
>         if let Some(ref callback) = props.on_escape_key_down {
>             let preventable = PreventableEvent::new();
>             callback.call(preventable.clone());
>             if preventable.is_default_prevented() { return; }
>         }
>         send(Event::CloseOnEscape);
>     }
> }
>
> // Adapter pseudocode for backdrop-click handling — symmetric flow:
> fn on_backdrop_pointer_down(props: &Props, send: &dyn Fn(Event)) {
>     if !props.close_on_backdrop { return; }
>     if let Some(ref callback) = props.on_interact_outside {
>         let preventable = PreventableEvent::new();
>         callback.call(preventable.clone());
>         if preventable.is_default_prevented() { return; }
>     }
>     send(Event::CloseOnBackdropClick);
> }
> ```

### 1.10 Connect / API

> **Element handle contract.** IDs derived from `Context::ids` (via
> `ids.part("trigger" | "content" | "title" | "description")`) are semantic strings used
> solely for ARIA wiring (`aria-labelledby`, `aria-describedby`, `aria-controls`) and the
> `id` attribute on each rendered part for hydration stability. They are **never** used
> as element-lookup keys — the agnostic core never calls `getElementById` or any
> equivalent platform helper. Adapters obtain live element references through framework
> primitives:
>
> - **Leptos** — `NodeRef<T>` captured at the render site for each anatomy part.
> - **Dioxus** — `MountedData` (typically held inside a `Signal<Option<MountedData>>`)
>   captured via the `onmounted` event for each anatomy part.
>
> When an effect intent fires (see [§1.11](#111-adapter-intent-contract)), the adapter
> resolves the target element through the captured handle for that part, never by
> looking up the ID.

```rust
#[derive(ComponentPart)]
#[scope = "dialog"]
pub enum Part {
    Root,
    Trigger,
    Backdrop,
    Positioner,
    Content,
    Title,
    Description,
    CloseTrigger,
}

/// The API for the `Dialog` component.
///
/// `Api` implements `Debug` (omitting the non-`Debug` `send` callback
/// field) for ergonomic logging in tests and dev tools. The `Debug`
/// impl renders the borrowed `state`, `ctx`, and `props` only.
pub struct Api<'a> {
    /// The current state of the dialog.
    state: &'a State,
    /// The current context of the dialog.
    ctx: &'a Context,
    /// The current props of the dialog.
    props: &'a Props,
    /// The event sender for the dialog.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the dialog is open.
    pub fn is_open(&self) -> bool { *self.state == State::Open }

    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs
    }

    /// The attributes for the trigger element.
    ///
    /// `type="button"` is emitted unconditionally. HTML `<button>` defaults
    /// to `type="submit"` inside a `<form>`, which would submit the form on
    /// click; pinning the type to `"button"` is the correct defensive
    /// default for a dialog trigger.
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("trigger"));
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.is_open() { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.ctx.ids.part("content"));
        attrs
    }

    /// The handler for the trigger click event.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// The attributes for the backdrop element.
    // NOTE: `aria-hidden="true"` and `inert` on the backdrop are always correct
    // (it is decorative). Feature detection for `inert` applies to the
    // `dialog-set-background-inert` adapter intent on sibling elements, not to
    // this backdrop element.
    pub fn backdrop_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Backdrop.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::Inert, "");
        attrs
    }

    /// The handler for the backdrop click event.
    pub fn on_backdrop_click(&self) {
        (self.send)(Event::CloseOnBackdropClick);
    }

    /// The attributes for the positioner element.
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// The attributes for the content element.
    ///
    /// `tabindex="-1"` is emitted statically: a `<div role="dialog">` is
    /// otherwise not focusable, but the adapter must be able to call
    /// `.focus()` on the content during the entry animation
    /// ([§1.8.2](#182-focus-escape-during-animation)) and during the
    /// focus-restoration fallback chain
    /// ([§3.3.1](#331-focus-restoration-fallback-chain)). `tabindex="-1"`
    /// makes the element programmatically focusable while keeping it out of
    /// the Tab order.
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("content"));
        attrs.set(HtmlAttr::Role, self.ctx.role.as_aria_role());
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.modal { attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "true"); }
        if self.ctx.has_title {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("title"));
        }
        if self.ctx.has_description {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), self.ctx.ids.part("description"));
        }
        attrs
    }

    /// The handler for the content keydown event.
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Escape {
            (self.send)(Event::CloseOnEscape);
        }
    }

    /// The attributes for the title element.
    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("title"));
        // The adapter renders the Title as <h{level}> using this value.
        // Clamped to valid heading levels 1..=6.
        let level = self.props.title_level.clamp(1, 6);
        attrs.set(HtmlAttr::Data("ars-heading-level"), level.to_string());
        attrs
    }

    /// The attributes for the description element.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        attrs
    }

    /// The attributes for the close trigger element.
    ///
    /// `type="button"` is emitted for the same reason as
    /// [`trigger_attrs`](#1-state-machine): close triggers rendered inside a
    /// surrounding `<form>` must not submit it on click.
    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.close_label)(&self.ctx.locale));
        attrs
    }

    /// The handler for the close trigger click event.
    pub fn on_close_trigger_click(&self) {
        (self.send)(Event::Close);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Backdrop => self.backdrop_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Title => self.title_attrs(),
            Part::Description => self.description_attrs(),
            Part::CloseTrigger => self.close_trigger_attrs(),
        }
    }
}
```

### 1.11 Adapter intent contract

The agnostic [`Machine`](#19-full-machine-implementation) emits each named effect below
without any payload. Adapters subscribe to these names via `SendResult::pending_effects`
and resolve target elements through their captured framework handles (see
[§1.10 Element handle contract](#110-connect--api)). Cross-references in this section
to [§1.5](#15-scroll-lock-restoration-edge-cases), [§1.6](#16-inert-attribute-polyfill),
[§1.7](#17-nested-dialogs), and [§3.3.1](#331-focus-restoration-fallback-chain) describe
the adapter-side behaviour required for each intent — those sections are non-normative
for the agnostic core.

All emission predicates below read from `ctx` (the runtime context), not
`props` directly. `ctx` is initialised from `props` at `init()` time and
remains the authoritative configuration source visible to `transition`.

All effect-name values are prefixed with `dialog-` for the same reason
[`Tooltip`](./tooltip.md) prefixes its names with `tooltip-`: when the
literal surfaces in adapter logs or devtools the component of origin is
self-describing without consulting the `Service<M>` type parameter.

| Effect name                      | When emitted                                           | Adapter obligation                                                                                                                                                                                                         |
| -------------------------------- | ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `dialog-open-change`             | Every state-flipping transition (any `Closed ↔ Open`). | Read `api.is_open()` after the transition runs and invoke `props.on_open_change` with that value if it is `Some`. See the echo-loop note in [§1.9 `on_props_changed`](#19-full-machine-implementation).                    |
| `dialog-scroll-lock-acquire`     | `Closed → Open` and `ctx.prevent_scroll == true`.      | Acquire body-scroll lock following [§1.5](#15-scroll-lock-restoration-edge-cases); only the outermost dialog in the stack actually applies the lock.                                                                       |
| `dialog-scroll-lock-release`     | `Open → Closed` and `ctx.prevent_scroll == true`.      | Release the lock acquired above. Outermost-only ownership applies.                                                                                                                                                         |
| `dialog-set-background-inert`    | `Closed → Open` and `ctx.modal == true`.               | Apply `inert` to portal-root siblings, falling back to the polyfill described in [§1.6](#16-inert-attribute-polyfill). Push the dialog onto the dialog stack ([§1.7.1](#171-dialogstack-global-nested-dialog-management)). |
| `dialog-remove-background-inert` | `Open → Closed` and `ctx.modal == true`.               | Run the cleanup stored at acquire time and pop the dialog stack. Re-apply `inert` for the new top of the stack if non-empty.                                                                                               |
| `dialog-focus-initial`           | `Closed → Open`.                                       | Resolve `props.initial_focus` against the captured content handle and move focus accordingly. If `initial_focus` is `None`, the adapter's default is to leave focus to the next intent (`dialog-focus-first-tabbable`).    |
| `dialog-focus-first-tabbable`    | `Closed → Open`.                                       | If no element gained focus from `dialog-focus-initial`, focus the first tabbable descendant of the captured content handle. The adapter never resolves the content element by ID.                                          |
| `dialog-restore-focus`           | `Open → Closed` and `ctx.restore_focus == true`.       | Use the captured trigger handle's connectedness check to focus it; otherwise walk the [§3.3.1](#331-focus-restoration-fallback-chain) fallback chain, ending at `<body>`.                                                  |

The canonical resolution keys are exported by the implementation as
`pub const &str` so adapters match by const, not by literal string.
Literal-string matching is an anti-pattern: a typo silently produces a
no-op handler.

```rust
// Re-exported from `ars_components::overlay::dialog`.
pub const EFFECT_OPEN_CHANGE: &str             = "dialog-open-change";
pub const EFFECT_SCROLL_LOCK_ACQUIRE: &str     = "dialog-scroll-lock-acquire";
pub const EFFECT_SCROLL_LOCK_RELEASE: &str     = "dialog-scroll-lock-release";
pub const EFFECT_SET_BACKGROUND_INERT: &str    = "dialog-set-background-inert";
pub const EFFECT_REMOVE_BACKGROUND_INERT: &str = "dialog-remove-background-inert";
pub const EFFECT_FOCUS_INITIAL: &str           = "dialog-focus-initial";
pub const EFFECT_FOCUS_FIRST_TABBABLE: &str    = "dialog-focus-first-tabbable";
pub const EFFECT_RESTORE_FOCUS: &str           = "dialog-restore-focus";
```

Adapters MAY subscribe to additional names of their own (e.g. animation
coordination) but MUST NOT reinterpret the names above. The agnostic core
never emits any effect that carries an ID payload, a closure body, or a
`WeakSend` reference; effect names are the entire contract.

## 2. Anatomy

```text
Dialog
├── Root             (required)
├── Trigger          (required — button that opens dialog)
├── Backdrop         (required — semi-transparent overlay behind content)
├── Positioner       (required — centers/positions the content)
├── Content          (required — role="dialog" or "alertdialog")
├── Title            (optional — aria-labelledby target)
├── Description      (optional — aria-describedby target)
└── CloseTrigger     (optional — button inside dialog to close it)
```

| Part         | Element    | Key Attributes                                                                        |
| ------------ | ---------- | ------------------------------------------------------------------------------------- |
| Root         | `<div>`    | `data-ars-scope="dialog"`, `data-ars-part="root"`, `data-ars-state`                   |
| Trigger      | `<button>` | `type="button"`, `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls`           |
| Backdrop     | `<div>`    | `aria-hidden="true"`, `inert`, `data-ars-state`                                       |
| Positioner   | `<div>`    | `data-ars-scope="dialog"`, `data-ars-part="positioner"`                               |
| Content      | `<div>`    | `role="dialog"`, `tabindex="-1"`, `aria-modal`, `aria-labelledby`, `aria-describedby` |
| Title        | `<h2>`     | `data-ars-heading-level`                                                              |
| Description  | `<p>`      | `id` for `aria-describedby` wiring                                                    |
| CloseTrigger | `<button>` | `type="button"`, `aria-label` from Messages                                           |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part         | Role / Property    | Value                                               |
| ------------ | ------------------ | --------------------------------------------------- |
| Content      | `role`             | `"dialog"` or `"alertdialog"`                       |
| Content      | `tabindex`         | `"-1"` (programmatically focusable, not a Tab stop) |
| Content      | `aria-modal`       | `"true"` when modal                                 |
| Content      | `aria-labelledby`  | Title part ID (when title is rendered)              |
| Content      | `aria-describedby` | Description part ID (when description is rendered)  |
| Trigger      | `type`             | `"button"` (defensive default; see §1.10)           |
| Trigger      | `aria-haspopup`    | `"dialog"`                                          |
| Trigger      | `aria-expanded`    | `"true"` / `"false"`                                |
| Trigger      | `aria-controls`    | Content part ID                                     |
| Backdrop     | `aria-hidden`      | `"true"` (decorative)                               |
| Backdrop     | `inert`            | Always present (decorative, non-interactive)        |
| CloseTrigger | `type`             | `"button"` (defensive default; see §1.10)           |
| CloseTrigger | `aria-label`       | from `Messages.close_label`                         |

- `inert` on background content: When a modal dialog is open, all DOM siblings of the portal root MUST have the `inert` attribute set. The `inert` attribute prevents all keyboard, pointer, and assistive technology interaction with background content. This is the modern replacement for `aria-hidden="true"` on siblings. The agnostic core emits the `dialog-set-background-inert` intent (see [§1.11](#111-adapter-intent-contract)); the adapter applies `inert` to the captured portal-root handle's siblings.
- `aria-hidden="true"` is set as fallback for older browsers that do not support `inert` (Safari < 15.5, Firefox < 112). The adapter should feature-detect `inert` support and use `aria-hidden="true"` only as a fallback.
- Both attributes are removed when the dialog closes — the adapter runs the cleanup it stored when handling `dialog-set-background-inert`, triggered by the agnostic core's `dialog-remove-background-inert` intent.

### 3.2 Keyboard Interaction

| Key       | Action                              |
| --------- | ----------------------------------- |
| Escape    | Close the dialog (if enabled)       |
| Tab       | Cycle focus within dialog content   |
| Shift+Tab | Cycle focus backwards within dialog |

### 3.3 Focus Management

1. **On open**: Focus first focusable element inside Content (or `initialFocus` target)
2. **Focus trap**: Tab/Shift+Tab cycle within Content — cannot leave dialog
3. **On close**: Return focus to trigger (or `finalFocus` target), using the fallback chain below if the trigger is unavailable
4. **Scroll lock**: `overflow: hidden` on `<body>` when `preventScroll=true`

#### 3.3.1 Focus Restoration Fallback Chain

When the dialog closes with `restore_focus: true`, the trigger element captured by the
adapter at render time may have been disconnected, disabled, or relocated while the
dialog was open. The adapter handles the agnostic core's `dialog-restore-focus` intent (see
[§1.11](#111-adapter-intent-contract)) using the following fallback chain. **All steps
operate on the framework handle the adapter captured for each part — `NodeRef<T>` in
Leptos, `MountedData` in Dioxus — never via ID lookup.**

1. **Captured trigger handle** — if the handle's element is connected to the document
   (`isConnected` / `parentElement.is_some()`) and is focusable (`!disabled`,
   `offsetParent !== null`).
2. **Nearest focusable ancestor** — walk from the trigger handle's last known parent
   (held by the adapter through the captured node reference) until a focusable element
   is found.
3. **`<body>`** — last resort if no focusable element is found in the ancestor chain.

If focus restoration falls through to step 2 or 3, the adapter emits a
`focus_restored_failed` event on the dialog's root handle with
`detail: { intended_target: <trigger handle>, actual_target: <focused handle> }`. This
allows application code to handle edge cases (e.g., re-rendering the trigger or
announcing an explanation to screen readers). The detail values are framework handles
or DOM `Element`s captured by the adapter — not `id` strings.

```rust
// In the Leptos adapter's `restore-focus` handler — illustrative only.
// `trigger_ref: NodeRef<HtmlButtonElement>` is captured at trigger render time and
// passed into the adapter's effect-handling layer. The agnostic core never sees it.
fn restore_focus_with_fallback(trigger_ref: NodeRef<HtmlButtonElement>) {
    // Step 1: Try the captured trigger handle.
    if let Some(el) = trigger_ref.get() {
        if el.is_connected() && is_focusable(&el) {
            let _ = el.focus();
            return;
        }
        // Step 2: Walk ancestors of the captured handle.
        if let Some(focusable) = find_nearest_focusable_ancestor(&el) {
            let _ = focusable.focus();
            emit_focus_restored_failed(&el, &focusable);
            return;
        }
    }
    // Step 3: Body fallback.
    if let Some(body) = document().body() {
        let _ = body.focus();
        emit_focus_restored_failed_body(&body);
    }
}
```

#### 3.3.2 Tab Order Restoration

When a modal opens, the adapter records the current `document.activeElement` and any modified `tabindex` values within the modal's focus scope. On modal close:

1. All modified `tabindex` values are restored to their original state.
2. Focus returns to the previously active element (or `document.body` if it was removed).
3. For nested modals, each modal maintains its own restoration stack — closing an inner modal restores the outer modal's tab order, not the page's.

### 3.4 Screen Reader Announcements

#### 3.4.1 Announcement Timing

On dialog open:

1. Focus moves to the first focusable element (or the dialog container if no focusable children).
2. The dialog's `aria-labelledby` and `aria-describedby` are set BEFORE focus moves, ensuring screen readers announce the title and description on focus.
3. A 100ms delay between DOM insertion and focus move allows screen readers to register the new landmark.
4. For VoiceOver compatibility, the dialog container itself has `role="dialog"` and `aria-modal="true"` — VoiceOver reads the label on container focus.

#### 3.4.2 Dismiss Event Coalescing

If both `Event::CloseOnEscape` and `Event::CloseOnBackdropClick` arrive in the same event batch, only the first is processed — the second is ignored via a guard that checks `state != Open`. This prevents double-close side effects (e.g., `on_close` callback firing twice). No explicit debouncing is needed because the state guard naturally prevents the second transition.

#### 3.4.3 Screen Reader Virtual Cursor Containment

The Tab/Shift+Tab focus trap alone is **insufficient** to contain screen reader users.
NVDA browse mode (Insert+Space to toggle) and VoiceOver virtual cursor can navigate past
a Tab-based focus trap because they traverse the DOM tree, not the tab order. The `inert`
attribute on sibling elements (described above) is the **primary defense** against virtual
cursor escape — `inert` elements are excluded from the accessibility tree entirely.

Implementation requirements:

1. **Mandate `inert` on all siblings** of the modal dialog's portal root via the
   `dialog-set-background-inert` adapter intent (see
   [§1.11](#111-adapter-intent-contract)). This MUST be applied before the dialog
   receives focus. The `inert` attribute prevents ALL interaction (keyboard, pointer,
   and assistive technology) with background content.
2. **Consider native `<dialog>` element**: The native `<dialog>` element with `showModal()`
   provides browser-native focus trapping that includes virtual cursor containment on
   browsers that support it (Chrome 37+, Firefox 98+, Safari 15.4+). When the adapter
   can use the native element, it SHOULD prefer `<dialog>` + `showModal()` over a custom
   `<div role="dialog">` with manual focus trapping. The state machine remains the same;
   only the DOM element and open/close mechanism change.
3. **Test case**: Verify virtual cursor containment with the following test:

    ```text
    test "modal dialog prevents NVDA browse mode escape" {
        // 1. Open modal dialog
        // 2. Verify all siblings of portal root have `inert` attribute
        // 3. Verify `aria-hidden="true"` fallback is set on siblings when
        //    `inert` is not supported
        // 4. Verify no focusable elements outside dialog are reachable
        //    via sequential Tab presses (focus wraps within dialog)
        // 5. Close dialog → verify `inert` and `aria-hidden` removed
    }
    ```

## 4. Internationalization

### 4.1 Messages

```rust
/// The messages of the dialog.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Close trigger label (default: "Close dialog")
    pub close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self { close_label: MessageFn::static_str("Close dialog") }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Animation Lifecycle (Presence Composition)

All overlay components (`Dialog`, `AlertDialog`, `Drawer`, `Popover`, `Tooltip`, `HoverCard`) **MUST** compose with the [`Presence`](./presence.md) machine for mount/unmount animations.

**Timing contract:**

- **Entry**: Mount the element first (`Presence` → `Mounted`), then apply the `data-ars-state="open"` attribute to trigger the CSS entry animation.
- **Exit**: Set `data-ars-state="closed"` to trigger the CSS exit animation, then wait for `animationend` before unmounting (Presence → `Unmounted`).

> **Lazy content + animation timing.** When `lazy_mount=true`, the overlay content is rendered for the first time during the `Mounted` phase. The adapter MUST ensure lazy content has fully rendered (i.e., is in the DOM) **before** the entry animation starts. If content uses async loading, wrap it in `<Suspense>` with a fallback and delay the `data-ars-state="open"` attribute (which triggers the CSS animation) until `<Suspense>` resolves. Without this, the animation plays on an empty or partially-rendered container.
>
> When `lazy_mount=true`, the `Presence` component enters the `Mounting` state (see [`Presence`](./presence.md)). The adapter MUST delay setting `data-ars-state="open"` until lazy content has fully settled and the `ContentReady` event has been dispatched. This prevents CSS entry animations from triggering on empty or partially-rendered containers.

**Composition pattern:**

```rust,no_check
// In the adapter component (e.g., dialog::Content):
let dialog = use_machine::<dialog::Machine>(dialog_props);
let presence = use_machine::<presence::Machine>(presence::Props::default());

// Sync dialog open state → presence
Effect::new(move || {
    let open = dialog.derive(|api| api.is_open()).get();
    if open {
        presence.send.call(presence::Event::Show);
    } else {
        presence.send.call(presence::Event::Hide);
    }
});

let is_present = presence.derive(|api| api.is_present());
let data_state = dialog.derive(|api| if api.is_open() { "open" } else { "closed" });

// Render only when Presence says to
if is_present.get() {
    // render content with data-ars-state for CSS animation triggers
}
```

## 6. Library Parity

> Compared against: Ark UI (`Dialog`), Radix UI (`Dialog`), React Aria (`Dialog`).

### 6.1 Props

| Feature                      | ars-ui                | Ark UI                   | Radix UI             | React Aria                     | Notes                                                                                 |
| ---------------------------- | --------------------- | ------------------------ | -------------------- | ------------------------------ | ------------------------------------------------------------------------------------- |
| Controlled open              | `open`                | `open`                   | `open`               | `isOpen`                       | All libraries                                                                         |
| Default open                 | `default_open`        | `defaultOpen`            | `defaultOpen`        | `defaultOpen`                  | All libraries                                                                         |
| Modal mode                   | `modal`               | `modal`                  | `modal`              | (ModalOverlay)                 | All libraries                                                                         |
| Close on Escape              | `close_on_escape`     | `closeOnEscape`          | (onEscapeKeyDown)    | `isKeyboardDismissDisabled`    | ars-ui uses boolean prop; Radix uses preventable callback; React Aria inverts boolean |
| Close on outside click       | `close_on_backdrop`   | `closeOnInteractOutside` | (onInteractOutside)  | `isDismissable`                | Same concept, different naming                                                        |
| Prevent scroll               | `prevent_scroll`      | `preventScroll`          | (implicit modal)     | (implicit modal)               | Ark UI explicit; Radix/React Aria implicit for modals                                 |
| Restore focus                | `restore_focus`       | `restoreFocus`           | (onCloseAutoFocus)   | (implicit)                     | All libraries restore focus                                                           |
| Initial focus                | `initial_focus`       | `initialFocusEl`         | (onOpenAutoFocus)    | (implicit)                     | Ark UI uses element ref; Radix uses preventable callback                              |
| Final focus                  | `final_focus`         | `finalFocusEl`           | (onCloseAutoFocus)   | --                             | Ark UI uses element ref; Radix uses callback                                          |
| Role                         | `role`                | `role`                   | --                   | `role`                         | Ark UI and React Aria support dialog/alertdialog                                      |
| Lazy mount                   | `lazy_mount`          | `lazyMount`              | --                   | --                             | Ark UI parity                                                                         |
| Unmount on exit              | `unmount_on_exit`     | `unmountOnExit`          | (forceMount inverse) | --                             | Ark UI parity; Radix uses forceMount                                                  |
| Escape callback              | `on_escape_key_down`  | `onEscapeKeyDown`        | `onEscapeKeyDown`    | --                             | Preventable in ars-ui and Radix                                                       |
| Outside interaction callback | `on_interact_outside` | `onInteractOutside`      | `onInteractOutside`  | `shouldCloseOnInteractOutside` | Preventable in ars-ui; React Aria uses predicate fn                                   |
| Trap focus                   | (implicit modal)      | `trapFocus`              | (implicit modal)     | (implicit modal)               | Ark UI exposes explicit prop                                                          |
| Title level                  | `title_level`         | --                       | --                   | --                             | ars-ui addition for heading semantics                                                 |
| `aria-label`                 | (via Messages)        | `aria-label`             | --                   | --                             | Ark UI passes directly; ars-ui uses Messages                                          |
| Open change callback         | `on_open_change`      | `onOpenChange`           | `onOpenChange`       | `onOpenChange`                 | All libraries                                                                         |

**Gaps:** None. ars-ui covers all props from all three libraries.

### 6.2 Anatomy

| Part         | ars-ui          | Ark UI       | Radix UI    | React Aria                 | Notes                          |
| ------------ | --------------- | ------------ | ----------- | -------------------------- | ------------------------------ |
| Root         | Root            | Root         | Root        | --                         | Container wrapper              |
| Trigger      | Trigger         | Trigger      | Trigger     | (DialogTrigger)            | Open button                    |
| Backdrop     | Backdrop        | Backdrop     | Overlay     | ModalOverlay               | Covers background              |
| Positioner   | Positioner      | Positioner   | --          | --                         | Ark UI parity                  |
| Content      | Content         | Content      | Content     | Dialog                     | Main content                   |
| Title        | Title           | Title        | Title       | (Heading slot)             | Accessible heading             |
| Description  | Description     | Description  | Description | --                         | Accessible description         |
| CloseTrigger | CloseTrigger    | CloseTrigger | Close       | (Button slot)              | Close button                   |
| Portal       | (adapter-level) | --           | Portal      | (UNSTABLE_portalContainer) | Radix has explicit Portal part |

**Gaps:** None. Radix Portal is handled at the adapter level in ars-ui.

### 6.3 Events

| Callback             | ars-ui                | Ark UI                 | Radix UI               | React Aria     | Notes                                       |
| -------------------- | --------------------- | ---------------------- | ---------------------- | -------------- | ------------------------------------------- |
| Open change          | `on_open_change`      | `onOpenChange`         | `onOpenChange`         | `onOpenChange` | All libraries                               |
| Escape key           | `on_escape_key_down`  | `onEscapeKeyDown`      | `onEscapeKeyDown`      | --             | Preventable                                 |
| Outside interaction  | `on_interact_outside` | `onInteractOutside`    | `onInteractOutside`    | --             | Preventable                                 |
| Focus outside        | --                    | `onFocusOutside`       | --                     | --             | Ark UI only; subsumed by onInteractOutside  |
| Pointer down outside | --                    | `onPointerDownOutside` | `onPointerDownOutside` | --             | Ark UI/Radix; subsumed by onInteractOutside |
| Exit complete        | (Presence)            | `onExitComplete`       | --                     | --             | Handled by Presence composition             |
| Open auto focus      | (initial_focus prop)  | --                     | `onOpenAutoFocus`      | --             | Radix uses callback; ars-ui uses prop       |
| Close auto focus     | (final_focus prop)    | --                     | `onCloseAutoFocus`     | --             | Radix uses callback; ars-ui uses prop       |

**Gaps:** None. Granular outside-interaction events (`onFocusOutside`, `onPointerDownOutside`) are intentionally subsumed by the single `on_interact_outside` callback, which is simpler and sufficient.

### 6.4 Features

| Feature                    | ars-ui         | Ark UI                        | Radix UI         | React Aria                         |
| -------------------------- | -------------- | ----------------------------- | ---------------- | ---------------------------------- |
| Modal/non-modal            | Yes            | Yes                           | Yes              | Yes                                |
| Focus trap                 | Yes            | Yes                           | Yes (implicit)   | Yes (implicit)                     |
| Scroll lock                | Yes            | Yes                           | Yes (implicit)   | Yes (implicit)                     |
| Inert background           | Yes            | Yes (via Zag)                 | --               | --                                 |
| Nested dialogs             | Yes            | Yes                           | --               | --                                 |
| Entry/exit animation       | Yes (Presence) | Yes (lazyMount/unmountOnExit) | Yes (forceMount) | Yes (isEntering/isExiting)         |
| Preventable dismiss        | Yes            | Yes                           | Yes              | Yes (shouldCloseOnInteractOutside) |
| Lazy content mount         | Yes            | Yes                           | --               | --                                 |
| Focus restoration fallback | Yes            | --                            | --               | --                                 |
| Role=alertdialog           | Yes            | Yes                           | (AlertDialog)    | Yes                                |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** (1) ars-ui uses `PreventableEvent` callbacks instead of Radix's cancelable event pattern for dismiss interception. (2) ars-ui uses Presence composition for animation lifecycle instead of inline `lazyMount`/`unmountOnExit`/`forceMount` props on the component itself (these are delegated to Presence). (3) Focus targets use props (`initial_focus`, `final_focus`) instead of Radix's auto-focus callbacks.
- **Recommended additions:** None.

> **AlertDialog** and **Drawer**: Same composition pattern applies. AlertDialog uses `role="alertdialog"` but the Presence wrapping is identical.
