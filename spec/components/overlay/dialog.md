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
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The dialog is closed.
    Closed,
    /// The dialog is open.
    Open,
}
```

### 1.2 Events

```rust
/// Events for the `Dialog` component.
#[derive(Clone, Debug, PartialEq)]
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
    // NOTE: AnimationStart and AnimationEnd are NOT part of the Dialog event enum.
    // The Presence machine (presence.md) handles animation lifecycle
    // independently. Dialog only reacts to Presence completion via PresenceComplete.
    /// Register the title of the dialog.
    RegisterTitle,
    /// Register the description of the dialog.
    RegisterDescription,
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
    /// The ID of the trigger element.
    pub trigger_id: String,
    /// The ID of the content element.
    pub content_id: String,
    /// The ID of the title element.
    pub title_id: String,
    /// The ID of the description element.
    pub description_id: String,
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Role {
    /// The dialog role.
    Dialog,
    /// The alert dialog role.
    AlertDialog,
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
#[derive(Clone, Debug)]
pub struct PreventableEvent {
    /// Whether the default behavior has been prevented.
    prevented: bool,
}

impl PreventableEvent {
    pub fn new() -> Self { Self { prevented: false } }
    /// Prevent the default behavior (e.g., prevent dialog from closing).
    pub fn prevent_default(&mut self) { self.prevented = true; }
    /// Whether `prevent_default()` was called.
    pub fn is_default_prevented(&self) -> bool { self.prevented }
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
    /// Call `event.prevent_default()` to prevent the dialog from closing.
    /// Fires before the close transition — if prevented, the transition is cancelled.
    pub on_escape_key_down: Option<Callback<PreventableEvent>>,
    /// Callback invoked when a pointer down or focus event occurs outside the dialog content.
    /// Call `event.prevent_default()` to prevent the dialog from closing.
    /// Fires before the close transition — if prevented, the transition is cancelled.
    pub on_interact_outside: Option<Callback<PreventableEvent>>,
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

**Dialog Open/Close Integration**: The dialog state machine's `Open` and `Close` transitions MUST call `dialog_stack_push()` and `dialog_stack_pop()` respectively as part of their `PendingEffect` setup/cleanup.

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

**Pop timing guarantee**: `dialog_stack_pop()` is executed as a `PendingEffect` during the close transition. Because effects run synchronously before the next event is processed, the stack is updated (and the previously-second dialog becomes topmost) before any subsequent Escape `keydown` event can be handled. This ensures that the second Escape press — if it arrives after the first dialog's close effect runs — correctly targets the next dialog in the stack, not the already-closing one.

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

`FocusScope` MUST NOT activate until the enter animation has started (after `animationstart` fires). During any animation delay, set `tabindex="-1"` on the container to prevent focus escape to background elements.

#### 1.8.3 FocusScope Activation Lifecycle

The following sequence governs FocusScope activation for dialog overlays:

1. **During the animation delay period**, set `tabindex="-1"` on the dialog container to prevent premature focus entry.
2. **Wait for the `animationstart` event** (or activate immediately if no animation is configured) before activating FocusScope.
3. **Once FocusScope activates**, move focus to the initial target (`initial_focus` prop or first focusable element).
4. **If the dialog has `lazy_mount=true`**, FocusScope activation waits for BOTH content settlement (`ContentReady`) AND `animationstart`.

### 1.9 Full Machine Implementation

````rust
use ars_core::{TransitionPlan, PendingEffect, Bindable, AttrMap};

/// The machine for the `Dialog` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let open = props.open.unwrap_or(props.default_open);
        let state = if open { State::Open } else { State::Closed };
        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();
        (state, Context {
            open,
            modal: props.modal,
            close_on_backdrop: props.close_on_backdrop,
            close_on_escape: props.close_on_escape,
            prevent_scroll: props.prevent_scroll,
            restore_focus: props.restore_focus,
            initial_focus: props.initial_focus.clone(),
            final_focus: props.final_focus.clone(),
            role: props.role.clone(),
            trigger_id: ids.part("trigger"),
            content_id: ids.part("content"),
            title_id: ids.part("title"),
            description_id: ids.part("description"),
            has_title: false,
            has_description: false,
            locale,
            messages,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Closed, Event::Open | Event::Toggle) => {
                Some(TransitionPlan::to(State::Open)
                    .apply(|ctx| { ctx.open = true; })
                    .with_effect(PendingEffect::new("prevent-scroll", |ctx, props, _send| {
                        if props.prevent_scroll {
                            // Scroll lock implementation:
                            // 1. Set `overflow: hidden; position: fixed; width: 100vw` on document element.
                            // 2. Measure `window.scrollY` before locking and restore after unlocking.
                            // 3. Nested scrollable containers inside the dialog MUST remain scrollable.
                            // 4. When multiple modals stack, only the topmost modal controls the scroll lock.
                            let restore = prevent_body_scroll();
                            Box::new(restore)
                        } else {
                            no_cleanup()
                        }
                    }))
                    .with_effect(PendingEffect::new("focus-management", |ctx, props, _send| {
                        let initial = props.initial_focus.clone();
                        let content_id = ctx.content_id.clone();
                        focus_initial(&content_id, initial.as_ref());
                        let trigger_id = ctx.trigger_id.clone();
                        let restore = props.restore_focus;
                        let final_focus = props.final_focus.clone();
                        Box::new(move || {
                            if restore {
                                focus_final(&trigger_id, final_focus.as_ref());
                            }
                        })
                    }))
                    .with_effect(PendingEffect::new("set_background_inert", |_ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let cleanup = platform.set_background_inert("ars-portal-root");
                        cleanup
                    }))
                    // The cleanup returned by `set_background_inert()` is stored by the
                    // adapter's PendingEffect lifecycle and runs automatically when this
                    // effect is replaced or the dialog leaves `Open`. That cleanup is the
                    // authoritative teardown path for native inert and polyfill state.
                    // ── Adapter-level static dialog stack specification ──
                    //
                    // Maintain a static `DIALOG_STACK: Vec<DialogId>` at the adapter level.
                    //
                    // On Open: push dialog ID onto stack. Apply `inert` attribute only to
                    //          siblings of the current top dialog.
                    //
                    // On Close: pop dialog ID from stack. If stack is non-empty, re-apply
                    //           `inert` to siblings of the new top. If stack is empty, remove
                    //           all `inert` attributes.
                    //
                    // This ensures closing an inner dialog does not accidentally remove
                    // `inert` from the outer dialog's background.
                    //
                    // ```rust
                    // static DIALOG_STACK: Mutex<Vec<DialogId>> = Mutex::new(Vec::new());
                    // ```
                    .with_effect(PendingEffect::new("focus-first-tabbable", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let content_id = ctx.ids.part("content");
                        platform.focus_first_tabbable(&content_id);
                        no_cleanup()
                    })))
            }
            (State::Open, Event::Close | Event::Toggle) => {
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.open = false; })
                    .with_effect(PendingEffect::new("release-scroll-lock", |_ctx, props, _send| {
                        let platform = use_platform_effects();
                        if props.prevent_scroll {
                            platform.scroll_lock_release();
                        }
                        no_cleanup()
                    }))
                    .with_effect(PendingEffect::new("remove-background-inert", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        // Best-effort direct clearing used for stack recalculation and
                        // defensive close flows. This does not replace the stored cleanup
                        // from `set_background_inert()`.
                        platform.remove_inert_from_siblings(&ctx.ids.part("portal"));
                        no_cleanup()
                    }))
                    .with_effect(PendingEffect::new("restore-focus", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        if platform.document_contains_id(&ctx.trigger_id) {
                            platform.focus_element_by_id(&ctx.trigger_id);
                        } else {
                            // Trigger removed from DOM — fall back to body
                            platform.focus_body();
                        }
                        no_cleanup()
                    })))
            }
            (State::Open, Event::CloseOnBackdropClick) => {
                if ctx.close_on_backdrop {
                    Some(TransitionPlan::to(State::Closed)
                        .apply(|ctx| { ctx.open = false; }))
                } else { None }
            }
            (State::Open, Event::CloseOnEscape) => {
                if ctx.close_on_escape {
                    Some(TransitionPlan::to(State::Closed)
                        .apply(|ctx| { ctx.open = false; }))
                } else { None }
            }
            // Register title/description for aria-labelledby/describedby wiring
            (_, Event::RegisterTitle) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.has_title = true;
                }))
            }
            (_, Event::RegisterDescription) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.has_description = true;
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
}
````

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
>             let mut preventable = PreventableEvent::new();
>             callback.call(&mut preventable);
>             if preventable.is_default_prevented() { return; }
>         }
>         send(Event::CloseOnEscape);
>     }
> }
> ```

### 1.10 Connect / API

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
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.trigger_id);
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.is_open() { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), &self.ctx.content_id);
        attrs
    }

    /// The handler for the trigger click event.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// The attributes for the backdrop element.
    // NOTE: `aria-hidden="true"` and `inert` on the backdrop are always correct
    // (it is decorative). Feature detection for `inert` applies to the
    // `set_background_inert` effect on sibling elements, not to this backdrop element.
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
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.content_id);
        attrs.set(HtmlAttr::Role, match self.ctx.role {
            Role::Dialog => "dialog",
            Role::AlertDialog => "alertdialog",
        });
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        if self.ctx.modal { attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "true"); }
        if self.ctx.has_title {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), &self.ctx.title_id);
        }
        if self.ctx.has_description {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), &self.ctx.description_id);
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
        attrs.set(HtmlAttr::Id, &self.ctx.title_id);
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
        attrs.set(HtmlAttr::Id, &self.ctx.description_id);
        attrs
    }

    /// The attributes for the close trigger element.
    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
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

| Part         | Element    | Key Attributes                                                       |
| ------------ | ---------- | -------------------------------------------------------------------- |
| Root         | `<div>`    | `data-ars-scope="dialog"`, `data-ars-part="root"`, `data-ars-state`  |
| Trigger      | `<button>` | `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls`           |
| Backdrop     | `<div>`    | `aria-hidden="true"`, `inert`, `data-ars-state`                      |
| Positioner   | `<div>`    | `data-ars-scope="dialog"`, `data-ars-part="positioner"`              |
| Content      | `<div>`    | `role="dialog"`, `aria-modal`, `aria-labelledby`, `aria-describedby` |
| Title        | `<h2>`     | `data-ars-heading-level`                                             |
| Description  | `<p>`      | `id` for `aria-describedby` wiring                                   |
| CloseTrigger | `<button>` | `aria-label` from Messages                                           |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part     | Role / Property    | Value                                              |
| -------- | ------------------ | -------------------------------------------------- |
| Content  | `role`             | `"dialog"` or `"alertdialog"`                      |
| Content  | `aria-modal`       | `"true"` when modal                                |
| Content  | `aria-labelledby`  | Title part ID (when title is rendered)             |
| Content  | `aria-describedby` | Description part ID (when description is rendered) |
| Trigger  | `aria-haspopup`    | `"dialog"`                                         |
| Trigger  | `aria-expanded`    | `"true"` / `"false"`                               |
| Trigger  | `aria-controls`    | Content part ID                                    |
| Backdrop | `aria-hidden`      | `"true"` (decorative)                              |
| Backdrop | `inert`            | Always present (decorative, non-interactive)       |

- `inert` on background content: When a modal dialog is open, all DOM siblings of the portal root MUST have the `inert` attribute set. The `inert` attribute prevents all keyboard, pointer, and assistive technology interaction with background content. This is the modern replacement for `aria-hidden="true"` on siblings. The `set_background_inert` effect handles this via `platform.set_background_inert("ars-portal-root")`.
- `aria-hidden="true"` is set as fallback for older browsers that do not support `inert` (Safari < 15.5, Firefox < 112). The adapter should feature-detect `inert` support and use `aria-hidden="true"` only as a fallback.
- Both attributes are removed when dialog closes (handled by the effect cleanup function).

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

When the dialog closes with `restore_focus: true`, the original trigger element may have been removed from the DOM, disabled, or moved to a different container while the dialog was open. The adapter MUST use the following fallback chain:

1. **Original trigger** — if it still exists in the DOM and is focusable (`!disabled`, `offsetParent !== null`).
2. **Nearest focusable ancestor** — walk up from the trigger's last known position (`parentElement`) until a focusable element is found.
3. **`<body>`** — last resort if no focusable element is found in the ancestor chain.

If focus restoration falls through to step 2 or 3, the adapter emits a `focus_restored_failed` event on the dialog's root element with `detail: { intended_target: trigger_id, actual_target: focused_element_id }`. This allows application code to handle edge cases (e.g., re-rendering the trigger or announcing an explanation to screen readers).

```rust
// In the adapter's close effect:
fn restore_focus_with_fallback(trigger_id: &str) {
    let doc = document();
    // Step 1: Try original trigger
    if let Some(el) = doc.get_element_by_id(trigger_id) {
        if is_focusable(&el) {
            el.focus().ok();
            return;
        }
    }
    // Step 2: Walk ancestors from trigger's last known parent
    if let Some(parent) = last_known_parent_of(trigger_id) {
        if let Some(focusable) = find_nearest_focusable_ancestor(&parent) {
            focusable.focus();
            emit_focus_restored_failed(trigger_id, &focusable);
            return;
        }
    }
    // Step 3: Body fallback
    doc.body().focus();
    emit_focus_restored_failed(trigger_id, &doc.body());
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

1. **Mandate `inert` on all siblings** of the modal dialog's portal root via
   `set_background_inert()`. This MUST be applied before the dialog receives focus.
   The `inert` attribute prevents ALL interaction (keyboard, pointer, and assistive
   technology) with background content.
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
#[derive(Clone, Debug)]
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

```rust
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
