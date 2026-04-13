# Interactions Specification

## Table of Contents

1. [Design Philosophy](#1-design-philosophy)
2. [Press Interaction](#2-press-interaction)
3. [Hover Interaction](#3-hover-interaction)
4. [Focus Interaction](#4-focus-interaction)
5. [Long Press Interaction](#5-long-press-interaction)
6. [Move Interaction](#6-move-interaction)
7. [Drag and Drop](#7-drag-and-drop)
8. [Interaction Composition](#8-interaction-composition)
9. [Moved sections](#9-moved-sections)
10. [Forced Colors — Interaction-Specific Styling](#10-forced-colors--interaction-specific-styling)
11. [Keyboard Interaction](#11-keyboard-interaction)
12. [InteractOutside Interaction](#12-interactoutside-interaction)

---

## 1. Design Philosophy

### 1.1 Why Abstract Interactions

Web platforms expose three fundamentally different input modalities — pointer devices (mouse, pen), touch, and keyboard — each with distinct event models, timing characteristics, and accessibility expectations. Without normalization, component authors must independently handle all three in every interactive element, leading to:

- **Inconsistent behavior**: A button that responds to `click` but ignores `Enter` and `Space` breaks keyboard navigation. Touch handling that omits scroll-cancel detection causes accidental activations.
- **Platform quirks unhandled**: iOS Safari fires synthetic `click` events on touch after a 300ms delay unless the element has `cursor: pointer`. Android Chrome fires `pointercancel` when a touch begins scrolling. Firefox does not fire `mouseenter`/`mouseleave` on SVG elements consistently.
- **Screen reader gaps**: Virtual cursors in screen readers (NVDA browse mode, VoiceOver) fire synthetic events that look like keyboard events but are not. Without distinguishing virtual activation from genuine keyboard activation, focus-visible logic misfires.
- **Race conditions between modalities**: Hovering while pressing on a touch device can leave stale hover state. Mouse events firing after touch events on iOS can duplicate activations.

`ars-interactions` provides a single, unified abstraction layer over all input modalities. Each interaction type produces a normalized event struct and exposes pre-built `AttrMap` sets (see `01-architecture.md`, §3.2) plus typed handler methods that handle all platform quirks. Components compose these abstractions rather than raw DOM events.

### 1.2 The Four Input Modalities

```rust
/// Represents how an interaction was initiated.
/// Matches the values exposed by the Pointer Events API, extended with
/// virtual activation from screen readers and scripted events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerType {
    /// Physical mouse or trackpad.
    Mouse,
    /// Finger on a touchscreen.
    Touch,
    /// Stylus or digital pen.
    Pen,
    /// Keyboard (Enter, Space, or arrow key).
    Keyboard,
    /// Programmatic / screen reader virtual cursor activation.
    Virtual,
}
```

### 1.3 How Components Compose Multiple Interactions

Most interactive components require more than one interaction abstraction. A `Button` requires both `press` and `focus`. A `Slider` requires `press`, `focus`, and `move`. A drag-and-drop list requires `press`, `long_press`, and `drag`.

Composition works at two levels:

**Level 1 — Attrs merging**: Each interaction's `connect` function returns an `AttrMap` set plus typed handler methods. The `merge_attrs` utility (§8) combines multiple `AttrMap` sets onto a single element, unioning data attributes and styles without collision. Event handlers are composed separately via typed methods on per-component `Api` structs.

**Level 2 — Interaction awareness**: Interactions are designed to be mutually aware. For example, `hover` suppresses hover state while a `press` is active (because touch devices fire both `pointerdown` and `mouseover`). `focus_visible` consults the shared `ModalityContext` to decide whether focus rings should appear. These cross-cutting concerns are handled via provider-scoped shared state, not by the component author.

```text
Button element
  ├── press_attrs()   → data-ars-pressed attribute + typed press handler methods
  ├── hover_attrs()   → data-ars-hovered attribute + typed hover handler methods
  └── focus_attrs()   → data-ars-focused, data-ars-focus-visible attributes + typed focus handler methods
```

### 1.4 Crate Location

All types and functions in this specification live in the `ars-interactions` crate, which sits above `ars-core` and `ars-a11y` in the dependency graph (see `01-architecture.md`, §1.2). It has no framework dependencies. DOM event bridging to actual `web_sys` events is done in `ars-dom`.

### 1.5 Interaction AttrMap Output Reference

Each interaction's `connect` function produces an `AttrMap` (data attributes, ARIA attributes, inline styles) plus typed handler methods on the component's `Api` struct. The complete output per interaction:

| Interaction     | Data Attributes                                                                                            | ARIA Attributes                                                      | Inline Styles                      | Event Handler Methods                                                                                                                            |
| --------------- | ---------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Press**       | `data-ars-pressed`, `data-ars-active` (adapter-managed), `data-ars-disabled` (authoritative when composed) | —                                                                    | `user-select: none` (during press) | `on_pointerdown`, `on_pointerup`, `on_pointerenter`, `on_pointerleave`, `on_pointercancel`, `on_keydown`, `on_keyup`, `on_click`, `on_dragstart` |
| **Hover**       | `data-ars-hovered`                                                                                         | —                                                                    | —                                  | `on_pointerenter`, `on_pointerleave`                                                                                                             |
| **Focus**       | `data-ars-focused`, `data-ars-focus-visible`                                                               | —                                                                    | —                                  | `on_focus`, `on_blur`                                                                                                                            |
| **FocusWithin** | `data-ars-focus-within`, `data-ars-focus-within-visible`                                                   | —                                                                    | —                                  | `on_focusin`, `on_focusout`                                                                                                                      |
| **LongPress**   | `data-ars-long-pressing`                                                                                   | `aria-describedby` (set by component connect, not `current_attrs()`) | —                                  | `on_pointerdown`, `on_pointermove`, `on_pointerup`, `on_pointercancel`, `on_keydown`, `on_keyup`                                                 |
| **Move**        | `data-ars-moving`                                                                                          | —                                                                    | —                                  | `on_pointermove`, `on_pointerdown`, `on_pointerup`, `on_pointercancel`, `on_keydown`, `on_keyup`                                                 |
| **Drag**‡       | `data-ars-dragging`                                                                                        | `aria-description`, `aria-grabbed`†, `aria-dropeffect`†              | —                                  | `on_pointerdown`, `on_pointermove`, `on_pointerup`, `on_pointercancel`, `on_keydown`                                                             |

> **‡ Drag/Drop reactivity model:** Unlike Press/Hover/Focus/LongPress/Move which use `current_attrs()` for fine-grained reactivity, DragResult and DropResult use a **snapshot-based** `attrs` field. The adapter calls `use_drag`/`use_drop` on each render to rebuild the snapshot with current state values. See §7.10 for details.
>
> **†** `aria-grabbed` and `aria-dropeffect` are deprecated in WAI-ARIA 1.2 and **only emitted when the `aria-drag-drop-compat` feature flag is enabled** (default: enabled). New implementations should use `aria-description` and `LiveAnnouncer` (see `03-accessibility.md` §5.1 for LiveAnnouncer; DnD-specific announcements are in §7.8 of this document). These attributes are retained for assistive technology interoperability — JAWS, NVDA, VoiceOver, and TalkBack have not yet fully adopted `aria-description` for drag-and-drop announcements. **Sunset condition:** Remove these attributes when all four screen readers support `aria-description` for DnD. Disable the feature flag to opt out early.

**Combined AttrMap example — Button with Press + Focus + Hover:**

```rust
// In Button's connect():
let press = use_press(press_config.clone());
let hover = use_hover(hover_config);
let focus = use_focus(focus_config);

let mut attrs = merge_attrs([press.current_attrs(&press_config), hover.current_attrs(), focus.current_attrs()]);
// Result when pressed, hovered, and keyboard-focused:
//   data-ars-pressed=""
//   data-ars-hovered=""
//   data-ars-focused=""
//   data-ars-focus-visible=""
// Event handlers wired separately via Api methods:
//   on_pointerdown, on_pointerup, on_pointerenter, on_pointerleave,
//   on_pointercancel, on_keydown, on_keyup, on_click, on_dragstart,
//   on_focus, on_blur
```

---

## 2. Press Interaction

### 2.1 Overview

Press is the fundamental activation interaction: the user intends to activate something. It unifies mouse click, touch tap, keyboard Enter/Space, and virtual cursor activation into a single, consistent `PressEvent`. It also handles the semantic difference between "pressed inside the element" (should activate) and "pressed outside" (should not activate, as the pointer was dragged away after being pressed).

### 2.2 Types

```rust
// ars-interactions/src/press.rs

use std::time::Duration;
use ars_core::{AttrMap, Callback, SharedFlag, SharedState};
use crate::PointerType;

/// Configuration for press interaction behavior.
///
/// Callbacks use [`Callback`] (not raw `Rc`/`Arc`) for automatic
/// platform-appropriate pointer type and built-in `Clone`, `Debug`,
/// and `PartialEq` (by pointer identity).
#[derive(Clone, Debug, PartialEq)]
pub struct PressConfig {
    /// Whether the element is disabled. Disabled elements receive no press events.
    pub disabled: bool,

    /// Prevent text selection on press-and-hold. Defaults to true for button-like
    /// elements, false for text content.
    pub prevent_text_selection: bool,

    /// Whether to allow the press to continue when the pointer leaves the element
    /// while still pressed (useful for sliders, scroll pickers). When false (default),
    /// leaving the element while pressed transitions to PressedOutside and will not
    /// fire on_press on release.
    pub allow_press_on_exit: bool,

    /// Touch scroll cancellation threshold in pixels. If touch displacement exceeds
    /// this value before `touchend`, the press is cancelled (user intended to scroll).
    /// Default: 10 (matching React Aria). Set higher for large touch targets.
    pub scroll_threshold_px: u16,

    /// Called when the element is pressed (pointer down AND within element).
    pub on_press_start: Option<Callback<dyn Fn(PressEvent)>>,

    /// Called when press ends (pointer up, key up, or cancellation).
    pub on_press_end: Option<Callback<dyn Fn(PressEvent)>>,

    /// Called on activation: pointer released inside the element, or Enter/Space
    /// released after having been pressed on this element.
    pub on_press: Option<Callback<dyn Fn(PressEvent)>>,

    /// Called when the pointer's inside/outside state changes while a press is active.
    /// `true` = pointer re-entered the element; `false` = pointer exited.
    pub on_press_change: Option<Callback<dyn Fn(bool)>>,

    /// Fired when a press is released (pointer up / key up / touch end),
    /// regardless of whether the release was inside or outside the element.
    /// Distinct from `on_press_end` (fires on any press conclusion) and `on_press`
    /// (fires only for activations inside the element).
    pub on_press_up: Option<Callback<dyn Fn(PressEvent)>>,

    /// Maximum duration to hold pointer capture before automatically releasing.
    /// Prevents stuck capture states caused by missed `pointerup` events (e.g.,
    /// browser tab switch, OS-level dialog). Defaults to 5000ms.
    /// Components like Signature Pad that require extended pointer capture for
    /// continuous drawing should set this to a higher value (e.g., 30000ms or None
    /// to disable the timeout entirely).
    ///
    /// **Implementation:** The timeout is adapter-implemented as a `PendingEffect`
    /// during `PressedInside`/`PressedOutside` states. When the timeout fires,
    /// the adapter calls `element.releasePointerCapture()` and transitions the
    /// machine to `Idle` via a synthesized `PointerUp` event. To prevent
    /// overlapping timeouts in composed interactions (e.g., Press + LongPress),
    /// each interaction MUST cancel any existing capture timeout before starting
    /// a new one. See §8 Interaction Composition for coordination rules.
    pub pointer_capture_timeout: Option<Duration>,

    /// When set, the press handler checks this shared state on release.
    /// `Some(pointer_type)` suppresses the matching modality's activation
    /// because a long-press already fired for that press. `None` means no
    /// pending long-press suppression. See §8.7 Cross-Interaction Cancellation
    /// Protocol.
    pub long_press_cancel_flag: Option<SharedState<Option<PointerType>>>,
}

impl Default for PressConfig {
    fn default() -> Self {
        Self {
            disabled: false,
            prevent_text_selection: true,
            allow_press_on_exit: false,
            scroll_threshold_px: 10,
            on_press_start: None,
            on_press_end: None,
            on_press: None,
            on_press_change: None,
            on_press_up: None,
            pointer_capture_timeout: Some(Duration::from_millis(5000)),
            long_press_cancel_flag: None,
        }
    }
}
```

#### 2.2.1 Pointer Events Security Model

Press interactions MUST validate event trust and enforce security constraints for pointer capture:

**`event.isTrusted` Validation**: All press handlers (pointerdown, pointerup, click) MUST check `event.isTrusted === true` before processing. Untrusted events (dispatched via `dispatchEvent()` or `element.click()`) MUST be ignored for security-sensitive actions (form submission, navigation, destructive operations). Non-security-sensitive press handlers (e.g., toggling a disclosure) MAY process untrusted events if the component explicitly opts in via `allow_untrusted: bool` prop (default: `false`).

**`setPointerCapture()` Rules**: Components using pointer capture (Slider, Splitter, drag interactions) MUST:

1. Call `setPointerCapture(pointerId)` only from trusted `pointerdown` events (capture fails silently on untrusted events)
2. Release capture on `pointerup` or `pointercancel`
3. Handle `lostpointercapture` event to clean up drag state if capture is stolen

#### 2.2.2 Multi-Touch Pointer ID Filtering

Adapters MUST filter pointer events to process only the primary pointer, ignoring secondary touch contacts:

1. **Check `event.isPrimary`**: On every `pointerdown`, `pointermove`, and `pointerup` event, the adapter MUST verify `event.isPrimary === true` before processing. Secondary pointer events (additional fingers on a touch screen) MUST be ignored entirely.

2. **Track primary `pointerId`**: On the initial `pointerdown` where `isPrimary === true`, the adapter records `event.pointerId` as the active pointer ID. All subsequent `pointermove` and `pointerup` events MUST be filtered to match this recorded ID. This prevents state corruption from interleaved multi-touch events.

3. **Ignore secondary touch events**: Any `pointerdown` where `isPrimary === false` MUST be discarded without side effects — no state changes, no capture, no announcements.

4. **Slider/Splitter note**: Components using `setPointerCapture(pointerId)` (Slider, Splitter, drag interactions) already bind capture to the primary pointer ID from the initial `pointerdown`. Pointer capture inherently routes subsequent events for that `pointerId` to the capturing element, providing an additional layer of filtering. However, the `isPrimary` check remains required as a defense-in-depth measure since `pointerdown` for secondary touches can still fire on the element before capture is evaluated.

**Cross-Origin Iframe Considerations**: Pointer events do NOT bubble across cross-origin iframe boundaries. Components inside iframes receive events normally; parent frames cannot intercept them. This is safe by default. Components MUST NOT attempt to read `event.target` across frame boundaries.

**`getCoalescedEvents()` for High-Frequency Input**: For components tracking continuous pointer movement (Slider drag, ColorPicker area, SignaturePad), adapters SHOULD use `event.getCoalescedEvents()` to access intermediate pointer positions between frames. This provides higher-fidelity input for drawing and precise positioning. Coalesced events are available only on `pointermove` and are not available on Safari < 17.

```rust
/// A normalized press event, independent of input modality.
///
/// **Clone semantics:** Uses [`SharedFlag`] for propagation control so that
/// cloned events share the same propagation flag. Calling `continue_propagation()`
/// on any clone affects the original and all other clones. `SharedFlag` is
/// thread-safe on native targets (`Arc<AtomicBool>`) and lightweight on wasm
/// (`Arc<AtomicBool>`).
/// Calling [`continue_propagation()`](Self::continue_propagation) on any
/// clone affects the original and all other clones.
#[derive(Clone, Debug)]
pub struct PressEvent {
    /// How the press was initiated.
    pub pointer_type: PointerType,

    /// The type of event this represents.
    pub event_type: PressEventType,

    /// Client-space X coordinate. None for keyboard/virtual events.
    pub client_x: Option<f64>,

    /// Client-space Y coordinate. None for keyboard/virtual events.
    pub client_y: Option<f64>,

    /// Modifier keys held at the time of the event.
    pub modifiers: KeyModifiers,

    /// Whether the element was the original target when the press started.
    /// False when the press started inside but pointer moved outside.
    pub is_within_element: bool,

    /// When called, prevents the event handler from stopping propagation.
    /// By default, press events stop propagation. Call
    /// [`continue_propagation()`](Self::continue_propagation) to allow parent
    /// handlers to also receive the event.
    ///
    /// Uses [`SharedFlag`] so cloned events share propagation state across
    /// threads on native targets.
    pub continue_propagation: SharedFlag,
}

impl PressEvent {
    /// Allow the event to propagate to parent handlers.
    pub fn continue_propagation(&self) {
        self.continue_propagation.set(true);
    }

    /// Check whether propagation was allowed.
    pub fn should_propagate(&self) -> bool {
        self.continue_propagation.get()
    }

    /// Creates a child event sharing propagation state with the parent.
    /// The child event's `continue_propagation` points to the same flag.
    pub fn create_child_event(&self) -> PressEvent {
        PressEvent {
            pointer_type: self.pointer_type,
            event_type: self.event_type,
            client_x: self.client_x,
            client_y: self.client_y,
            modifiers: self.modifiers,
            is_within_element: self.is_within_element,
            continue_propagation: self.continue_propagation.clone(),
        }
    }
}
```

> **Safe Usage — `SharedFlag` Cycle Risk**
>
> `PressEvent` uses `SharedFlag` for shared propagation state across cloned events.
> Event handlers MUST NOT capture `continue_propagation` in long-lived closures or
> cleanup functions, as this creates reference cycles that prevent deallocation.
> Extract the boolean value immediately:
>
> ```rust
> let should_propagate = event.continue_propagation.get();
> // Use should_propagate from here — do not hold the SharedFlag.
> ```
>
> `SharedFlag` already handles the wasm/native split internally
> (`Arc<AtomicBool>` on wasm32, `Arc<AtomicBool>` on native). No manual
> `cfg` switching is needed.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressEventType {
    PressStart,
    PressEnd,
    Press,
    /// Fired when a press is released (pointer up / key up / touch end),
    /// regardless of whether the release was inside or outside the element.
    /// Distinct from `PressEnd` (fires on any press conclusion) and `Press`
    /// (fires only for activations inside the element).
    PressUp,
}

/// Keyboard modifier state at the time of an event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}
```

> **⚠ Two `KeyModifiers` types exist — do not confuse them:**
>
> | Type                             | Crate              | Purpose                                        | Fields                         |
> | -------------------------------- | ------------------ | ---------------------------------------------- | ------------------------------ |
> | `ars_interactions::KeyModifiers` | `ars-interactions` | Raw platform key state from DOM events         | `shift`, `ctrl`, `alt`, `meta` |
> | `ars_a11y::KeyModifiers`         | `ars-a11y`         | Cross-platform abstraction (Ctrl/Meta unified) | `shift`, `action`, `alt`       |
>
> Conversion is adapter-side: the adapter must know the `Platform` (Mac vs Windows/Linux) to map `ctrl`/`meta` to the unified `action` modifier. See `ars-a11y::KeyModifiers` in `03-accessibility.md` §4.4 and the `From` impl below.
>
> `Direction` (Ltr/Rtl) is used by `key_to_delta()` — re-exported from `ars-core` (canonical definition in `ars-i18n`).

```rust
// Conversion from raw platform modifiers to accessibility-level modifiers.
// Lives in ars-interactions (which depends on ars-a11y) or the adapter layer.
// This conversion is lossy by design: ars_a11y::KeyModifiers unifies ctrl/meta
// into a single `action` modifier. If both ctrl and meta are pressed simultaneously,
// only the platform-specific action key is preserved.
impl From<(ars_interactions::KeyModifiers, Platform)> for ars_a11y::KeyModifiers {
    fn from((raw, platform): (ars_interactions::KeyModifiers, Platform)) -> Self {
        let action = match platform {
            Platform::MacOs | Platform::IOS => raw.meta,
            _ => raw.ctrl,
        };
        Self {
            shift: raw.shift,
            action,
            alt: raw.alt,
        }
    }
}
```

> **Adapter event dispatch pipeline:**
>
> 1. Raw DOM event → `ars_interactions::KeyModifiers` (ctrl, alt, shift, meta fields)
> 2. Adapter converts via `From` impl → `ars_a11y::KeyModifiers` (adds `action` field mapping Ctrl/Cmd per platform)
> 3. `FocusRing.on_key_down()` receives `ars_a11y::KeyModifiers`
> 4. Component handler receives `ars_interactions::KeyModifiers`
>
> The conversion happens once per keydown event in the adapter's event dispatch layer.

### 2.3 State Machine

The press state machine tracks whether the element is currently being pressed and whether the pointer is within the element's bounds.

```text
States:
  Idle            — No press in progress
  Pressing        — Implementation-internal transient state; resolves to PressedInside or
                    PressedOutside within the same event tick based on pointer position.
                    Never visible between renders. Exists to model the "press started but
                    position not yet resolved" instant.
  PressedInside   — Press started here; pointer is within the element bounds
  PressedOutside  — Press started here; pointer has left the element bounds

Transitions:

  Idle
    ─[PointerDown (button == 0) | TouchStart | KeyDown(Enter|Space)]──→ Pressing
        // Guard: event.button() == 0 (primary button only).
        // Auxiliary buttons (middle-click, right-click) do not initiate press.
        action: capture pointer, prevent text selection (if configured),
                emit on_press_start

  Pressing
    ─[PointerEnter | TouchWithinBounds]──→ PressedInside
    ─[PointerLeave | TouchOutsideBounds]──→ PressedOutside
        action: emit on_press_change(false)

  PressedInside
    ─[PointerLeave | TouchOutsideBounds]──→ PressedOutside
        action: emit on_press_change(false)
    ─[PointerUp | KeyUp | TouchEnd]──────→ Idle
        action: emit on_press_end, emit on_press (activation fires here)
    ─[PointerCancel | Blur | TouchCancel]→ Idle
        action: emit on_press_end (no on_press)

  PressedOutside
    ─[PointerEnter | TouchWithinBounds]──→ PressedInside
        action: emit on_press_change(true)
    ─[PointerUp | KeyUp | TouchEnd]──────→ Idle
        action: emit on_press_end (no on_press, activation does not fire)
    ─[PointerCancel | Blur | TouchCancel]→ Idle
        action: emit on_press_end (no on_press)
```

```rust
/// The current state of the press state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum PressState {
    /// No active press.
    Idle,

    /// Press has begun; position relative to element not yet resolved.
    /// Transient zero-duration state; resolves within same event tick to
    /// `PressedInside` or `PressedOutside` (which carry the `pointer_type`).
    /// `data-ars-pressed` is set once `PressedInside` is reached.
    Pressing { pointer_type: PointerType },

    /// Press is active and the pointer is within the element bounds.
    PressedInside {
        pointer_type: PointerType,
        origin_x: Option<f64>,
        origin_y: Option<f64>,
    },

    /// Press is active but the pointer has moved outside the element bounds.
    PressedOutside {
        pointer_type: PointerType,
    },
}

impl PressState {
    /// Returns `true` when the element is actively pressed within its bounds.
    /// The transient `Pressing` state (which resolves within the same event tick
    /// before any render) returns `false` to prevent `data-ars-pressed` from flashing.
    /// `PressedOutside` also returns `false`, matching the styling and activation
    /// semantics for presses that have left the element.
    pub fn is_pressed(&self) -> bool {
        matches!(self, PressState::PressedInside { .. })
    }

    pub fn is_pressed_inside(&self) -> bool {
        matches!(self, PressState::PressedInside { .. })
    }

    /// Note: this is an associated function, not a method — it does not depend on press state.
    /// Callers may also use `config.disabled` directly.
    pub fn is_disabled(config: &PressConfig) -> bool {
        config.disabled
    }
}
```

#### 2.3.1 Link Element Handling

When the press target is an `<a>` (link) element, the press interaction adjusts its behavior:

1. **Enter** triggers press activation (dispatches a synthetic `click` event for native navigation)
2. **Space** does NOT activate — per HTML spec, Space scrolls the page for links
3. Activation dispatches a `click` event on the DOM element to allow browser link behavior (open in new tab, copy link, etc.)
4. No `preventDefault` on the click to preserve native navigation

When the adapter detects the target element is an `<a>` tag (via DOM inspection at `pointerdown`/`keydown` time), the above link-specific rules apply automatically. No explicit `PressConfig` field is needed — element type detection is adapter-side.

### 2.4 Implementation Notes

**Pointer capture**: On `pointerdown`, call `element.set_pointer_capture(event.pointer_id())`. This ensures `pointermove` and `pointerup` fire on the target element even when the pointer leaves the viewport. Without capture, `pointerup` outside the window is lost and the machine stays stuck in a pressed state.

> **Pointer capture advisory.** When `setPointerCapture()` is active, all pointer events
> have `event.target` set to the capturing element regardless of cursor position. Components
> using InteractOutside during drag MUST use `document.elementFromPoint(e.clientX, e.clientY)`
> for hit-testing instead of `event.target`.
>
> All click-outside detection in `ars-dom` MUST use `document.elementFromPoint(e.clientX, e.clientY)` as the primary hit-test mechanism, falling back to `event.target` only when `elementFromPoint` returns null. This ensures correctness during pointer capture, drag operations, and cross-origin iframes.
>
> **`elementFromPoint()` edge cases:**
>
> - If `elementFromPoint()` returns `null` or `document.documentElement`, treat the result as "outside bounds" (the pointer is over no interactive element).
> - For cross-origin iframes, `elementFromPoint()` returns `null` for coordinates inside the iframe. Fall back to `event.target` in this case.
> - For shadow DOM boundaries, `elementFromPoint()` returns the shadow host. If the component is rendered inside a shadow root, perform an additional `event.target` check inside the shadow root via `shadowRoot.elementFromPoint()`.
> - Elements with `display: none` are never returned by `elementFromPoint()`; no special handling is needed.

#### 2.4.1 Pointer Capture Fallback Strategy

When `setPointerCapture` is unavailable (e.g., older WebKit versions):

1. **Feature Detection**: Check `Element.prototype.setPointerCapture` existence at initialization.
2. **Fallback**: Attach document-level `mousemove` and `mouseup` listeners for the duration of the drag/press interaction. Remove on release.
3. **Touch Handling**: Apply `touch-action: none` CSS on interactive elements to prevent browser scroll/zoom interference during pointer interactions.
4. **Legacy Support**: No MSPointer (IE11) legacy support is required — IE11 is out of scope for this library.

#### 2.4.2 Shadow DOM Event Delegation

When ars-ui components are used inside Shadow DOM (e.g., web components wrapping Leptos/Dioxus output), event delegation and target detection require special handling:

- **True event target:** Use `event.composedPath()[0]` instead of `event.target` to get the true originating element. Events that cross shadow boundaries have their `target` retargeted to the shadow host, hiding the actual element that fired the event.
- **Slot distribution:** Elements distributed into `<slot>` elements fire events from the slot's light DOM position, not the shadow DOM position. `composedPath()` includes both the light DOM and shadow DOM nodes in order, so walk the path to find the relevant ars-ui element.
- **Pointer events across shadow boundaries:** `pointerenter`/`pointerleave` fire on the shadow host when crossing shadow boundaries, not on internal elements. For hover interactions inside shadow DOM, attach listeners to the shadow root or use `pointermove` with hit-testing via `shadowRoot.elementFromPoint()`.
- **Browser quirks:**
  - Firefox: `composedPath()` returns an empty array for events dispatched via `dispatchEvent()` with `composed: false` (the default). Always set `composed: true` when dispatching synthetic events that need to cross shadow boundaries.
  - Safari: `focusin`/`focusout` events do not cross shadow boundaries in older versions (< Safari 16.4). Use `focus`/`blur` with `{ capture: true }` as a fallback.

**Text selection prevention**: During press on a non-text element, set `user-select: none` on `document.body` for the duration of the press. Restore it in the transition to `Idle`. This prevents accidental text selection during click-and-hold without disabling selection on the page globally.

**Touch scroll cancellation**: On `touchstart`, record the initial touch position. On `touchmove`, if displacement exceeds `scroll_threshold_px` (default 10px, matching React Aria) before `touchend`, fire `TouchCancel` to transition to `Idle`. This prevents activating an element when the user meant to scroll through it. The threshold is configurable via `PressConfig::scroll_threshold_px: u16` (default `10`).

**Touch identifier tracking**: Store `Touch.identifier` from the initial `touchstart` event for each tracked touch point. On `touchmove` and `touchend`, iterate `event.changedTouches` and match by identifier to find the correct touch — do NOT assume `changedTouches[0]` is the tracked touch. Touch identifiers are integers assigned by the browser and may be recycled after a touch ends, so clear the stored identifier on `touchend`/`touchcancel`. For the Move interaction (§6), the same identifier-matching approach applies to multi-touch move tracking.

**Keyboard handling**: Keyboards fire `keydown` repeatedly while held. Only the first `keydown` (when `event.repeat()` is false) transitions from `Idle` to `Pressing`. Subsequent repeats are ignored. `keyup` triggers the transition to `Idle`.

**iOS 300ms tap delay**: iOS Safari defers `click` events to check for double-taps. `ars-interactions` uses the Pointer Events API (`pointerdown`/`pointerup`) rather than `click` to bypass this delay, giving immediate feedback.

**Synthetic mouse events after touch**: iOS and Android fire `mouseover`, `mousedown`, `mouseup`, `click` approximately 300ms after a touch sequence. `ars-interactions` sets a flag when a `touchend` is received and suppresses the synthetic mouse events that follow within 300ms. Track the most recent pointer type via `PointerEvent.pointerType`. Only suppress synthetic mouse events when `pointerType === 'touch'`. Do not suppress when `pointerType === 'mouse'` or `'pen'` to avoid breaking hybrid input on iPad.

> **Passive event listener requirement:** All `touchstart` and `touchmove` event listeners that may call `preventDefault()` — including document-level listeners used for scroll-cancel detection — MUST be registered with `{ passive: false }`. Chrome 56+ defaults touch listeners on `document`/`window` to passive, silently ignoring `preventDefault()` calls. The adapter's DOM event registration layer (`ars-dom`) must provide an explicit `passive` parameter, defaulting to `true` for `scroll`/`wheel` and `false` for touch events used by press/move interactions. Always pass `{ passive: false }` explicitly for touch handlers calling `preventDefault()`. Prefer the PointerEvent API over touch-specific events where possible. When multiple touch handlers are needed on the same element, combine them into a single listener to reduce registration overhead.
>
> **Browser Quirk:** Starting with Chrome 56, `touchmove` listeners registered on `document` or `window` default to `{ passive: true }` even if not explicitly specified. This means `event.preventDefault()` is silently ignored for scroll prevention. This applies to any `touchmove` listener added via `addEventListener` without an explicit `passive: false` option. Adapters must ensure all touch listeners that need to cancel scrolling explicitly pass `{ passive: false }` at registration time.
>
> **iOS Safari `pointerup` safety net.** iOS Safari's gesture recognizer can swallow
> `pointerup` events during long press gestures (300ms+). Adapter implementations MUST
> set a safety timeout: if no `pointerup` is received within 5000ms of the last `pointerdown`,
> synthesize a `PointerUp` event to prevent stuck drag/press states.
>
> The 5000ms safety timeout is independent of the 300ms long-press threshold (§5).
> If a real `pointerup` arrives before the timeout fires, cancel the timeout immediately.
> When the timeout fires and synthesizes a `PointerUp`, ignore any real `pointerup` that
> arrives within the next 100ms (debounce window) to prevent duplicate transitions.
>
> **Browser Quirk:** `requestIdleCallback` is unavailable in Safari (as of Safari 17). Any adapter code that uses `requestIdleCallback` for deferred work (e.g., batching attribute updates, lazy cleanup) must feature-detect its availability and fall back to a `MessageChannel` polyfill (not `setTimeout(fn, 0)`, which clamps to ≥4ms). This applies to all scheduling paths in `ars-dom` that defer non-urgent work. Only defer non-critical updates (e.g., attribute batching, lazy cleanup). Test on Safari 17+ to ensure the polyfill does not cause missed event handlers.
>
> **Adapter Implementation Note:** The following JavaScript polyfill is provided for adapter implementers who need `requestIdleCallback` support in Safari and other browsers that lack native support.
>
> ```js
> // Complete requestIdleCallback polyfill using MessageChannel + requestAnimationFrame fallback.
> const scheduleIdle = (() => {
>     if (typeof window.requestIdleCallback === "function") {
>         return (fn) => window.requestIdleCallback(fn);
>     }
>     // MessageChannel-based polyfill: posts a message that fires as a macrotask, bypassing the >= 4ms clamping of setTimeout(fn, 0).
>     const channel = new MessageChannel();
>     const queue = [];
>     channel.port1.onmessage = () => {
>         const fn = queue.shift();
>         if (fn) fn({ timeRemaining: () => 0, didTimeout: false });
>     };
>     return (fn) => {
>         queue.push(fn);
>         channel.port2.postMessage(null);
>     };
> })();
>
> // Cancel polyfill (no-op for MessageChannel variant — the polyfill does not return a cancellable handle.)
> const cancelIdle =
>     typeof window.cancelIdleCallback === "function"
>         ? (id) => window.cancelIdleCallback(id)
>         : () => {};
> ```
>
> **Browser Quirk:** `ResizeObserver` can fire a "ResizeObserver loop limit exceeded" error when an observation callback triggers layout changes that in turn trigger additional observations. This is benign in most cases and does not indicate a real error. Adapters should suppress this at the application root: `window.addEventListener('error', (e) => { if (e.message?.includes('ResizeObserver')) e.stopPropagation(); });`. Components using `ResizeObserver` (e.g., positioning engine, overflow detection) should also guard against infinite loops by deferring layout-triggering updates with `requestAnimationFrame`. Components using `ResizeObserver` that trigger layout changes in the callback MUST defer those changes with `requestAnimationFrame` to break the observation→layout→observation loop.
>
> **Browser Quirk:** `scrollIntoView({ block: "nearest" })` is not supported in Safari versions prior to 15.4. If the adapter needs to scroll a focused option into view (e.g., in Select, Combobox, or Listbox), it must detect support and fall back to manual scroll position calculation using `getBoundingClientRect()` on both the target element and its scrollable container. Compute the required offset and set `container.scrollTop` directly. For nested scrollable containers, iterate all scrollable ancestors and apply the offset to each. Never use `behavior: 'smooth'` with active keyboard navigation — smooth scrolling interferes with rapid arrow-key traversal.
>
> ```rust
> /// Feature-detect `scrollIntoView` options support.
> /// Creates a detached element, calls scrollIntoView with options,
> /// and checks whether the browser accepted the object argument.
> fn supports_scroll_into_view_options() -> bool {
>     // Probe support by calling scrollIntoView with an options object on a detached element.
>     // Uses direct DOM API instead of js_sys::eval to remain CSP-compatible
>     // (eval is blocked by Content-Security-Policy in many deployments).
>     let doc = web_sys::window().and_then(|w| w.document());
>     let Some(doc) = doc else { return false; };
>     let Ok(el) = doc.create_element("div") else { return false; };
>     let opts = web_sys::ScrollIntoViewOptions::new();
>     opts.set_behavior(web_sys::ScrollBehavior::Instant);
>     // If the browser doesn't support options, scrollIntoView ignores the argument.
>     // Detect by checking if ScrollIntoViewOptions is constructible via Reflect.
>     js_sys::Reflect::get(
>         &js_sys::global(),
>         &"ScrollIntoViewOptions".into(),
>     ).map(|v| !v.is_undefined()).unwrap_or(false)
> }
>
> /// Scroll an element into its nearest scrollable ancestor's viewport.
> /// Handles Safari <15.4 fallback, horizontal scrolling, and nested containers.
> pub fn scroll_into_view_if_needed(element: &Element, options: ScrollIntoViewOptions) {
>     if supports_scroll_into_view_options() {
>         element.scroll_into_view_with_scroll_into_view_options(&options);
>     } else {
>         // Manual fallback: iterate all scrollable ancestors
>         let el_rect = element.get_bounding_client_rect();
>         let mut current = nearest_scrollable_ancestor(element);
>         while let Some(container) = current {
>             let c_rect = container.get_bounding_client_rect();
>             // Vertical: scroll up if element is above container, down if below
>             if el_rect.top() < c_rect.top() {
>                 container.set_scroll_top(
>                     container.scroll_top() - (c_rect.top() - el_rect.top()).round() as i32
>                 );
>             } else if el_rect.bottom() > c_rect.bottom() {
>                 container.set_scroll_top(
>                     container.scroll_top() + (el_rect.bottom() - c_rect.bottom()).round() as i32
>                 );
>             }
>             if el_rect.left() < c_rect.left() {
>                container.set_scroll_left(
>                    container.scroll_left() - (c_rect.left() - el_rect.left()).round() as i32
>                );
>            } else if el_rect.right() > c_rect.right() {
>                container.set_scroll_left(
>                    container.scroll_left() + (el_rect.right() - c_rect.right()).round() as i32
>                );
>            }
>             current = nearest_scrollable_ancestor(&container);
>         }
>     }
> }
> ```
>
> **Implementation Note:** CSS `animation-fill-mode: forwards` persists the final keyframe state after the animation ends. If the animated element is subsequently removed or hidden, the browser may retain the computed style from the final keyframe, causing layout or visual artifacts on re-insertion. Adapters implementing enter/exit animations (e.g., Presence, overlay transitions) should remove the animation class or inline style after the `animationend` event fires. Listen for `animationend` on the element and clean up in the handler to avoid stale computed styles. Entry animations MUST use `animation-fill-mode: none` or `backwards` (not `forwards`). Exit animations may use `forwards` only if the element is immediately removed from DOM after `animationend`. If the element persists, remove the animation class in the `animationend` handler.
>
> **Double-RAF Pattern for Animation Detection**: Before reading `getComputedStyle()`
> to detect animation/transition durations, use two nested `requestAnimationFrame` calls.
> The first RAF ensures the browser has processed style recalculation; the second ensures
> the computed values are stable. Without this, CSS cascade changes applied in the same
> frame may not be reflected, causing "zero duration" false negatives.
>
> ```js
> requestAnimationFrame(() => {
>     requestAnimationFrame(() => {
>         const duration = getComputedStyle(el).animationDuration;
>         // Now safe to read
>     });
> });
> ```
>
> **Note:** `set_background_inert` must scope to siblings of the immediate portal container, not the root. See §12.8 (Nested Overlay Handling) for dialog stack tracking requirements.

### 2.5 Output Props

```rust
/// Returns an AttrMap (data attributes only) plus typed handler methods that,
/// when applied to an element, implement full press handling.
pub fn use_press(config: PressConfig) -> PressResult {
    let state = Rc::new(RefCell::new(PressState::Idle));
    let active_presses = Rc::new(RefCell::new(Vec::new()));

    let pressed = state.borrow().is_pressed_inside();

    // Event handlers are registered as typed methods on the component's Api struct:
    //   pointerdown  → Idle ──→ PressedInside (captures pointer)
    //   pointerup    → PressedInside ──→ Idle (fires on_press if inside, fires on_press_up always)
    //   pointerup    → PressedOutside ──→ Idle (fires on_press_up but NOT on_press)
    //   pointerenter → PressedOutside ──→ PressedInside
    //   pointerleave → PressedInside ──→ PressedOutside
    //   keydown(Enter|Space) → Idle ──→ Pressing (non-repeat only)
    //   keyup(Enter|Space)   → PressedInside ──→ Idle (fires on_press, fires on_press_up)
    //   touch/pointercancel  → PressedInside|PressedOutside ──→ Idle (fires on_press_up)

    PressResult { state, active_presses, config, pressed }
}

///
/// `PressResult` attrs are **reactive, not one-shot snapshots**. The `attrs` field
/// was removed in favor of a `current_attrs()` method that reads the live press state
/// each time it is called. This ensures that the `data-ars-pressed` attribute reflects
/// the current state at the point of DOM reconciliation, not the state at the time
/// `use_press()` was initially invoked.
///
/// **Migration:** Instead of reading `press.attrs` once and passing it to `merge_attrs`,
/// call `press.current_attrs()` inside the component's `connect()` method where
/// AttrMap construction happens. This guarantees the attrs are fresh on every render.
///
/// Alternatively, integrate press state into the component machine's `Context`:
/// the machine derives `data-ars-pressed` from its own context state, ensuring
/// reactivity through the normal state machine update cycle.
struct ActivePress {
    pointer_type: PointerType,
    origin_x: Option<f64>,
    origin_y: Option<f64>,
    is_within_element: bool,
}

pub struct PressResult {
    /// Internal state handle — use `current_attrs()` to produce a live AttrMap.
    state: Rc<RefCell<PressState>>,
    active_presses: Rc<RefCell<Vec<ActivePress>>>,
    config: PressConfig,

    /// Whether the element is currently being pressed (reactive signal in adapter).
    pub pressed: bool,
}

impl PressResult {
    /// Produce a fresh `AttrMap` reflecting the current press state.
    /// Call this inside `connect()` — not once at init time — to ensure
    /// the returned attributes are always up to date.
    pub fn current_attrs(&self, config: &PressConfig) -> AttrMap {
        let state = self.state.borrow();
        let mut attrs = AttrMap::new();
        if state.is_pressed_inside() {
            attrs.set_bool(HtmlAttr::Data("ars-pressed"), true);
        }
        if PressState::is_disabled(config) {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
            // Disabled elements use aria-disabled="true" instead of HTML disabled
            // to allow tooltip on hover. Do NOT set pointer-events:none here.
            // Adapters prevent interaction via event handler removal.
        }
        // Note: data-ars-active is managed by the adapter layer (not current_attrs)
        // because it requires RAF-deferred removal after pointerup.
        attrs
    }

    /// Returns the current press state snapshot.
    pub fn current_state(&self) -> PressState { /* ... */ }

    /// Adapter-facing transition helpers used by framework event handlers.
    ///
    /// `begin_press()` handles pointer/key down, `update_pressed_bounds()`
    /// handles enter/leave or hit-tested move for a specific modality,
    /// `end_press()` handles release for a specific modality and returns whether
    /// activation fired, and `cancel_press()` handles cancel/blur for a specific
    /// modality.
    /// `end_press()` MUST consult `config.long_press_cancel_flag` before firing
    /// `on_press`, consuming only the stored modality that matches the current
    /// release so suppression does not leak into the next interaction or a
    /// concurrent modality.
    pub fn begin_press(&mut self, pointer_type: PointerType, client_x: Option<f64>, client_y: Option<f64>, modifiers: KeyModifiers, within_element: bool) { /* ... */ }

    pub fn update_pressed_bounds(&mut self, pointer_type: PointerType, within_element: bool, client_x: Option<f64>, client_y: Option<f64>) { /* ... */ }

    pub fn end_press(&mut self, pointer_type: PointerType, client_x: Option<f64>, client_y: Option<f64>, modifiers: KeyModifiers) -> bool { /* ... */ }

    pub fn cancel_press(&mut self, pointer_type: PointerType, client_x: Option<f64>, client_y: Option<f64>, modifiers: KeyModifiers) { /* ... */ }
}
```

The returned `AttrMap` will contain data attributes (see below). Event handlers are registered
as typed methods on the component's `Api` struct, covering:

- `pointerdown` — initiates press, captures pointer
- `pointerup` — ends press, potentially fires activation
- `pointerenter` — tracks inside/outside during press
- `pointerleave` — tracks inside/outside during press
- `pointercancel` — cancels press
- `keydown` — keyboard press start (Enter/Space)
- `keyup` — keyboard press end
- `click` — used only for virtual/screen reader events (pointer events handle mouse/touch)
- `dragstart` — `prevent_default()` to prevent ghost image during press-and-drag

Data attributes on the element:

- `data-ars-pressed` — present when `is_pressed_inside()` is true (pointer within element bounds during press)

#### 2.5.1 Touch Event Active State

Do **not** rely on the CSS `:active` pseudo-class to style pressed state. On touch devices, browsers keep `:active` set briefly after `pointerup` (browser paint-timing quirk), causing a visual flicker where the element appears unpressed via `data-ars-pressed` while `:active` is still applied.

Instead, use two data attributes:

- **`data-ars-pressed`** — Set synchronously on `pointerdown` / `keydown`, cleared synchronously on `pointerup` / `keyup`. This drives the primary pressed visual state and is tied directly to the press state machine.
- **`data-ars-active`** — Set synchronously on `pointerdown`, cleared **after `pointerup` + one `requestAnimationFrame`**. This attribute bridges the gap between the state machine's synchronous `pointerup` handling and the browser's asynchronous `:active` teardown. Style sheets that previously used `:active` should use `[data-ars-active]` instead.

```rust
// Adapter pseudocode for data-ars-active lifecycle:
// on pointerdown:
//   element.set_attribute("data-ars-active", "")
//
// on pointerup:
//   request_animation_frame(move || {
//       element.remove_attribute("data-ars-active");
//   });
```

`aria-pressed` (for toggle buttons) remains tied to the component's logical pressed state, not to the visual active state.

#### 2.5.2 Passive Event Listeners

Event listeners for `wheel`, `scroll`, `touchstart`, and `touchmove` must be registered with `{ passive: true }` unless the handler calls `preventDefault()`. Press interaction handlers that need to prevent default scrolling on `touchstart` must use `{ passive: false }` explicitly and document the performance trade-off.

#### 2.5.3 Simultaneous Keyboard and Pointer Input

Multiple input modalities can be active simultaneously. For example, a user may hold the mouse button down on an element while also pressing `Space` or `Enter` on the keyboard (e.g., assistive device or split-input scenarios). The press interaction tracks these independently:

- **Per-modality active source bookkeeping** — the implementation keeps one active press record per current modality (`Mouse`, `Touch`, `Pen`, `Keyboard`, `Virtual`) and updates or releases them independently.
- **Modality-targeted helper calls** — adapter event handlers MUST pass the originating `pointer_type` to `update_pressed_bounds()`, `end_press()`, and `cancel_press()` so the correct active source is updated or removed.

The element is considered pressed if **any** active source is still within the element. The `data-ars-pressed` attribute and `is_pressed()` reflect this union. Each modality's release is processed independently: releasing the mouse while `Space` is still held does not clear the pressed state, and vice versa. Activation (`on_press`) fires once per completed press cycle per modality — a pointer release fires activation (if inside), and a key release fires activation, independently.

#### 2.5.4 Press Cancellation by LongPress

When Press and LongPress are composed on the same element (see §8.7 Cross-Interaction Cancellation), Press MUST check the shared `long_press_fired` flag on `pointerup`. If `long_press_fired` is `true`, Press skips `on_press` activation and only fires `on_press_end`. This prevents a completed long-press gesture from also triggering the press action. See §8.7 for the full cancellation protocol.

---

## 3. Hover Interaction

### 3.1 Overview

Hover state represents a pointer being positioned over an element without activation. It applies only to mouse and pen devices; touch and keyboard have no hover concept. The hover interaction also integrates with press: hover state is suppressed while a press is active, preventing false hover during touch interactions that fire both pointer and mouse events.

### 3.2 Types

```rust
// ars-interactions/src/hover.rs

use ars_core::{AttrMap, Callback, HtmlAttr, ModalityContext, SharedState};

use crate::PointerType;

/// Configuration for hover interaction behavior.
///
/// Controls how the hover interaction responds to pointer enter/leave events.
/// Callbacks use [`Callback`] for automatic platform-appropriate pointer type
/// (`Rc` on wasm, `Arc` on native) and built-in `Clone`, `Debug`, and
/// `PartialEq` (by pointer identity).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HoverConfig {
    /// Whether the element is disabled. Disabled elements receive no hover events.
    pub disabled: bool,

    /// Called when the pointer enters the element.
    pub on_hover_start: Option<Callback<dyn Fn(HoverEvent)>>,

    /// Called when the pointer leaves the element.
    pub on_hover_end: Option<Callback<dyn Fn(HoverEvent)>>,

    /// Called whenever hover state changes.
    pub on_hover_change: Option<Callback<dyn Fn(bool)>>,
}

/// A normalized hover event. Only produced for Mouse and Pen pointer types;
/// touch and keyboard do not produce hover events.
#[derive(Clone, Debug)]
pub struct HoverEvent {
    /// Always Mouse or Pen; never Touch, Keyboard, or Virtual.
    pub pointer_type: PointerType,

    /// The type of hover event.
    pub event_type: HoverEventType,
}

/// The kind of hover event being dispatched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverEventType {
    /// The pointer entered the element.
    HoverStart,
    /// The pointer left the element.
    HoverEnd,
}
```

### 3.3 State Machine

```text
States:
  NotHovered   — Pointer is not over the element
  Hovered      — Pointer is over the element

Transitions:

  NotHovered
    ─[PointerEnter where pointer_type ∈ {Mouse, Pen}
       AND global_press_active = false]──→ Hovered
        action: emit on_hover_start, emit on_hover_change(true)

  Hovered
    ─[PointerLeave]──────→ NotHovered
        action: emit on_hover_end, emit on_hover_change(false)
    ─[GlobalPressBegins]─→ NotHovered
        action: emit on_hover_end (hover suppressed by press)
```

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HoverState {
    #[default]
    NotHovered,
    Hovered,
}

impl HoverState {
    /// Returns `true` when the pointer is over the element.
    #[must_use]
    pub fn is_hovered(&self) -> bool {
        matches!(self, HoverState::Hovered)
    }
}
```

### 3.4 Integration with Press

`ars-interactions` reads shared instance-scoped modality state from `ars-core::ModalityContext`. When `HoverState` is `Hovered` and a global press begins — for example, from touch, which fires `pointerover` on iOS before `pointerdown` — the hover interaction immediately transitions to `NotHovered`. This prevents stale hover highlights on mobile while keeping the state isolated to a single provider root instead of a process-global singleton.

```rust
use ars_core::ModalityContext;

/// Hover integration reads the shared modality snapshot instead of a thread-local.
fn should_clear_hover(modality: &dyn ModalityContext) -> bool {
    modality.is_global_press_active()
}

/// Programmatic focus uses the same shared modality context to decide whether
/// a preceding interaction came from a pointer device.
fn had_pointer_interaction(modality: &dyn ModalityContext) -> bool {
    modality.had_pointer_interaction()
}
```

> **Browser Quirk:** Firefox does not fire `mouseenter`/`mouseleave` on SVG `<use>` children. This can cause hover state to become stuck or never activate on icon buttons that use SVG sprites. Workaround: set `pointer-events: fill` on SVG elements that trap pointer events so that pointer events target the SVG root rather than its children. Adapters should apply this style automatically when hover interaction is connected to an SVG element.
>
> The fix applies to all SVG elements that can trap pointer events: `<use>`, `<image>`,
> `<mask>`, `<clipPath>`. Only set `pointer-events: fill` if the element does not already
> have a `pointer-events` style set (to avoid overriding intentional author styles).
> For iOS Safari, suppress hover state entirely on SVG buttons to avoid sticky hover
> from touch interactions. Cache applied fixes in a `WeakMap` (or `WeakSet<Element>`)
> to avoid redundant DOM traversals on re-render.
>
> ```rust
> /// Apply `pointer-events: fill` to SVG elements that trap pointer events.
> /// Targets: <use>, <image>, <mask>, <clipPath>.
> /// Opt-out via `data-ars-no-svg-fix` attribute on the SVG element.
> /// Caches processed elements in a WeakSet to avoid redundant DOM traversals.
> pub fn fix_svg_hover(svg_element: &SvgElement) {
>     if svg_element.has_attribute("data-ars-no-svg-fix") { return; }
>     if SVG_FIX_CACHE.has(svg_element) { return; }
>     let selectors = "use, image, mask, clipPath";
>     if let Ok(node_list) = svg_element.query_selector_all(selectors) {
>         for i in 0..node_list.length() {
>             if let Some(node) = node_list.item(i) {
>                 // Use Element (not HtmlElement) — SVG children like <use>, <image>,
>                 // <mask>, <clipPath> are SvgElement, not HtmlElement. Setting attributes
>                 // via Element::set_attribute works for both HTML and SVG elements.
>                 if let Ok(el) = node.dyn_into::<web_sys::Element>() {
>                     if el.get_attribute("style")
>                         .map_or(true, |s| !s.contains("pointer-events"))
>                     {
>                         let _ = el.set_attribute("style",
>                             &format!("{}pointer-events: fill;",
>                                 el.get_attribute("style").unwrap_or_default()));
>                     }
>                 }
>             }
>         }
>     }
>     SVG_FIX_CACHE.add(svg_element);
> }
> ```

### 3.5 Output Props

```rust
/// Creates a hover interaction state machine with the given configuration.
///
/// Returns a [`HoverResult`] holding the initial `NotHovered` state. Event
/// handlers are registered as typed methods on the component's `Api` struct
/// by the framework adapter — this factory only creates the core state container.
#[must_use]
pub fn use_hover(config: HoverConfig) -> HoverResult {
    let state = SharedState::new(HoverState::NotHovered);
    let _is_disabled = config.disabled;

    let hovered = state.get().is_hovered();

    // Event handlers are registered as typed methods on the component's Api struct:
    //   pointerenter → NotHovered ──→ Hovered (mouse/pen only; ignores touch)
    //   pointerleave → Hovered ──→ NotHovered
    // Shared modality tracking: if modality.is_global_press_active()
    // becomes true while Hovered,
    //   immediately transition to NotHovered (prevents stale hover on mobile).

    HoverResult { hovered, state }
}

/// The output of [`use_hover`], providing live attribute generation and state access.
///
/// `HoverResult` attrs are **reactive, not one-shot snapshots**. Use
/// [`current_attrs()`](Self::current_attrs) inside the component's `connect()`
/// method to ensure attributes reflect the current state at DOM reconciliation.
#[derive(Debug)]
pub struct HoverResult {
    /// Whether the element is currently hovered (reactive signal in adapter).
    pub hovered: bool,
    /// Internal state handle — use [`current_attrs()`](Self::current_attrs) to
    /// produce a live `AttrMap`.
    state: SharedState<HoverState>,
}

impl HoverResult {
    /// Produce a fresh [`AttrMap`] reflecting the current hover state.
    ///
    /// Call this inside `connect()` — not once at init time — to ensure
    /// the returned attributes are always up to date.
    #[must_use]
    pub fn current_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        self.state.with(|s| {
            if s.is_hovered() {
                attrs.set_bool(HtmlAttr::Data("ars-hovered"), true);
            }
        });
        attrs
    }
}
```

The returned `AttrMap` will contain data attributes (see below). Event handlers are registered
as typed methods on the component's `Api` struct, covering:

- `pointerenter` — transitions to `Hovered` for Mouse/Pen (ignores Touch)
- `pointerleave` — transitions to `NotHovered`

Data attributes:

- `data-ars-hovered` — present (no value) when `is_hovered()` is true

---

## 4. Focus Interaction

> **Cross-ref: FocusRing (ars-a11y §3.4):** `FocusState` consumes the shared `ars_core::ModalityContext` while `FocusRing` consumes the same normalized event stream for accessibility-specific focus-visible heuristics. See also §4.4 below for adapter wiring that keeps both consumers synchronized through `ars-dom::ModalityManager`.
>
> **Normative:** `FocusState` (ars-interactions) is the consumer-facing API. Components MUST use `FocusResult.current_attrs()` for focus-visible rendering. Components MUST NOT also call `FocusRing.apply_focus_attrs()` on the same element — doing so would produce duplicate or conflicting `data-ars-focus-visible` attributes. `FocusRing` is the low-level tracker used internally by `FocusState`; direct use is reserved for rare cases where the ars-interactions layer is bypassed.
>
> **VoiceOver iOS:** Components using `aria-activedescendant` MUST implement the fallback `aria-live` region described in `03-accessibility.md` §3.3. VoiceOver iOS does not support `aria-activedescendant`, so a live region must echo the active descendant's label. This applies to Select, Menu, Listbox, and Combobox components.

### 4.1 Overview

Focus interaction provides normalized `focus` and `blur` events and, critically, determines whether focus is "visible" — i.e., whether a focus ring should be displayed. A focus ring should appear for keyboard navigation but is unnecessary and visually noisy for pointer interactions. The Web platform's `:focus-visible` CSS pseudo-class handles some of this, but it has cross-browser inconsistencies and does not integrate with the ars-ui data attribute system. `ars-interactions` provides its own modality tracking that is consistent across all browsers and exposes the result as `data-ars-focus-visible`.

`FocusWithin` extends focus tracking to container elements: the container is marked as focus-containing when any descendant has focus, matching CSS `:focus-within` but again exposed as a data attribute.

### 4.2 Types

```rust
// ars-interactions/src/focus.rs

use std::{cell::RefCell, rc::Rc, sync::Arc};

use ars_core::{AttrMap, Callback, DefaultModalityContext, ModalityContext};

use crate::PointerType;

/// Configuration for focus interaction on a single element.
// Manual Debug impl omitted for brevity — prints closures as "<closure>"
#[derive(Clone)]
pub struct FocusConfig {
    /// Whether the element is disabled.
    pub disabled: bool,

    /// Shared modality context for the current provider root.
    pub modality: Arc<dyn ModalityContext>,

    /// Called when the element receives focus.
    pub on_focus: Option<Callback<dyn Fn(FocusEvent)>>,

    /// Called when the element loses focus.
    pub on_blur: Option<Callback<dyn Fn(FocusEvent)>>,

    /// Called when focus-visible state changes.
    pub on_focus_visible_change: Option<Callback<dyn Fn(bool)>>,
}

/// Configuration for focus-within tracking on a container element.
// Manual Debug impl omitted for brevity — prints closures as "<closure>"
#[derive(Clone)]
pub struct FocusWithinConfig {
    /// Whether the container is disabled.
    pub disabled: bool,

    /// Shared modality context for the current provider root.
    pub modality: Arc<dyn ModalityContext>,

    /// Called when focus enters the container (any descendant focused).
    pub on_focus_within: Option<Callback<dyn Fn(FocusEvent)>>,

    /// Called when focus leaves the container entirely.
    pub on_blur_within: Option<Callback<dyn Fn(FocusEvent)>>,

    /// Called when focus-within-visible state changes.
    pub on_focus_within_visible_change: Option<Callback<dyn Fn(bool)>>,
}

impl Default for FocusConfig {
    fn default() -> Self {
        Self {
            disabled: false,
            modality: Arc::new(DefaultModalityContext::new()),
            on_focus: None,
            on_blur: None,
            on_focus_visible_change: None,
        }
    }
}

impl Default for FocusWithinConfig {
    fn default() -> Self {
        Self {
            disabled: false,
            modality: Arc::new(DefaultModalityContext::new()),
            on_focus_within: None,
            on_blur_within: None,
            on_focus_within_visible_change: None,
        }
    }
}

/// A normalized focus event.
#[derive(Clone, Debug)]
pub struct FocusEvent {
    /// The type of focus event.
    pub event_type: FocusEventType,

    /// The pointer type that triggered this focus, or None if focus was moved
    /// programmatically (e.g., via `element.focus()`).
    pub pointer_type: Option<PointerType>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusEventType {
    Focus,
    Blur,
    FocusWithin,
    BlurWithin,
}
```

### 4.3 State Machine

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FocusState {
    /// Element does not have focus.
    Unfocused,

    /// Element has focus, received via pointer interaction.
    /// Focus ring should NOT be shown.
    FocusedByPointer,

    /// Element has focus, received via keyboard navigation.
    /// Focus ring SHOULD be shown.
    FocusedByKeyboard,

    /// Element has focus, received via programmatic `.focus()` call.
    /// Focus ring shown only if the document's previous modality was keyboard.
    FocusedProgrammatic,
}

impl FocusState {
    pub fn is_focused(&self) -> bool {
        !matches!(self, FocusState::Unfocused)
    }

    /// Returns true if a visible focus indicator should be rendered.
    /// Keyboard focus always shows the ring. Programmatic focus only shows
    /// the ring when there was no preceding pointer interaction (i.e., the
    /// document's modality was keyboard before the `.focus()` call).
    pub fn is_focus_visible(&self, modality: &dyn ModalityContext) -> bool {
        match self {
            FocusState::FocusedByKeyboard => true,
            // Programmatic focus defers to the shared modality context:
            // show the ring only if the user was NOT using a pointer device
            // immediately before the programmatic `.focus()` call.
            FocusState::FocusedProgrammatic => !modality.had_pointer_interaction(),
            _ => false,
        }
    }
}
```

```text
States:
  Unfocused
  FocusedByPointer
  FocusedByKeyboard
  FocusedProgrammatic

Transitions:

  Unfocused
    ─[Focus, global_modality=Pointer]──→ FocusedByPointer
    ─[Focus, global_modality=Keyboard]─→ FocusedByKeyboard
    ─[Focus, no preceding interaction]─→ FocusedProgrammatic

  FocusedByPointer | FocusedByKeyboard | FocusedProgrammatic
    ─[Blur]────────────────────────────→ Unfocused
```

### 4.4 Focus Visible Detection: Shared Modality Tracking

The key to `focus-visible` is tracking the most recent input modality for the active provider root. `ars-core` owns the instance-scoped modality state and `ars-dom` owns the browser listener lifecycle:

```rust
use std::sync::Arc;

use ars_a11y::FocusRing;
use ars_core::{KeyboardKey, KeyModifiers, ModalityContext, PointerType};

pub struct ModalityManager {
    modality: Arc<dyn ModalityContext>,
    focus_ring: FocusRing,
}

impl ModalityManager {
    /// Safe to call multiple times; attaches one listener set per manager instance.
    pub fn ensure_listeners(&self) {
        // Browser-only implementation in ars-dom:
        // - no-op when no Window/Document is available
        // - installs keydown, pointerdown, mousedown, touchstart, and focus(capture)
        // - uses refcounted install/remove semantics
    }

    pub fn on_key_down(&self, key: KeyboardKey, modifiers: KeyModifiers) {
        self.modality.on_key_down(key, modifiers);
        self.focus_ring.on_key_down(key, modifiers);
    }

    pub fn on_pointer_down(&self, pointer_type: PointerType) {
        self.modality.on_pointer_down(pointer_type);
        self.focus_ring.on_pointer_down();
    }

    pub fn on_virtual_input(&self) {
        self.modality.on_virtual_input();
        self.focus_ring.on_virtual_input();
    }
}
```

> **Modality tracking:** `ars_core::ModalityContext` is the canonical source of truth for `FocusState` and other interaction consumers. `FocusRing` consumes the same event stream but remains a separate accessibility heuristic. Adapters MUST use `ars-dom::ModalityManager` instead of updating the context and focus ring independently.
>
> **Adapter wiring example** — use `ModalityManager` from `ars-dom`:
>
> ```rust
> // `manager` is the ModalityManager held in the adapter's state.
> //
> // document.add_event_listener("keydown", move |e: KeyboardEvent| {
> //     let key = KeyboardKey::from_key_str(&e.key());
> //     let modifiers = KeyModifiers {
> //         shift: e.shift_key(),
> //         ctrl: e.ctrl_key(),
> //         alt: e.alt_key(),
> //         meta: e.meta_key(),
> //     };
> //     manager.on_key_down(key, modifiers);
> // });
> //
> // document.add_event_listener("pointerdown", move |e: PointerEvent| {
> //     let pointer_type = match e.pointer_type().as_str() {
> //         "touch" => PointerType::Touch,
> //         "pen" => PointerType::Pen,
> //         _ => PointerType::Mouse,
> //     };
> //     manager.on_pointer_down(pointer_type);
> // });
> ```

When a `focus` event fires on an element, `ars-interactions` reads `config.modality.last_pointer_type()`:

- If the last interaction was `Keyboard` → state becomes `FocusedByKeyboard`, `data-ars-focus-visible` is set.
- If the last interaction was `Mouse`, `Touch`, or `Pen` → state becomes `FocusedByPointer`, no `data-ars-focus-visible`.
- If no prior interaction (programmatic focus) → state becomes `FocusedProgrammatic`, defers to the shared modality context.

### 4.5 Output Props

````rust
pub fn use_focus(config: FocusConfig) -> FocusResult {
    let state = Rc::new(RefCell::new(FocusState::Unfocused));

    let focused = state.borrow().is_focused();
    let focus_visible = state.borrow().is_focus_visible(config.modality.as_ref());

    // Event handlers are registered as typed methods on the component's Api struct:
    //   focus → reads config.modality.last_pointer_type() to determine modality:
    //           Keyboard → FocusedByKeyboard (sets data-ars-focus-visible)
    //           Mouse/Touch/Pen → FocusedByPointer (no focus-visible)
    //           Programmatic → FocusedProgrammatic (defers to shared modality context)
    //   blur  → any focused state ──→ Unfocused

    FocusResult { focused, focus_visible, state }
}

pub struct FocusResult {
    pub focused: bool,
    pub focus_visible: bool,
    state: Rc<RefCell<FocusState>>,
}

impl FocusResult {
    /// Returns the current data attributes for the focus interaction.
    /// Frameworks must call this each render to get up-to-date attrs.
    pub fn current_attrs(&self, config: &FocusConfig) -> AttrMap {
        let state = self.state.borrow();
        let mut attrs = AttrMap::new();
        if state.is_focused() {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }
        if state.is_focus_visible(config.modality.as_ref()) {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs
    }
}

pub fn use_focus_within(config: FocusWithinConfig) -> FocusWithinResult {
    let state = Rc::new(RefCell::new(false));  // tracks whether focus is within
    let visible = Rc::new(RefCell::new(false)); // tracks focus-visible within

    let focus_within = *state.borrow();
    let is_focus_within_visible = *visible.borrow();

    // Event handlers are registered as typed methods on the component's Api struct:
    //   focusin  → set focus_within = true; check config.modality for visibility
    //   focusout → if related_target is outside container, set focus_within = false
    //
    // IMPORTANT — null relatedTarget handling:
    //   When `focusout` fires with `relatedTarget == null` (common in Safari,
    //   and always the case when focus moves into a Shadow DOM boundary),
    //   do NOT immediately set `focus_within = false`. Instead:
    //
    //   1. Defer the check via `queueMicrotask`:
    //      ```js
    //      queueMicrotask(() => {
    //          if (!container.contains(document.activeElement)) {
    //              set_focus_within(false);
    //          }
    //      });
    //      ```
    //
    //   2. For Shadow DOM: use `event.composedPath()` to trace the actual
    //      focus target across shadow boundaries. The first element in the
    //      composed path is the true event target, even when `relatedTarget`
    //      is null due to shadow encapsulation.
    //
    //   3. If `document.activeElement` is the `<body>` element after the
    //      microtask, focus has genuinely left the container.

    FocusWithinResult { focus_within, is_focus_within_visible, state: state.clone(), visible: visible.clone() }
}

pub struct FocusWithinResult {
    pub focus_within: bool,
    pub is_focus_within_visible: bool,
    state: Rc<RefCell<bool>>,
    visible: Rc<RefCell<bool>>,
}

impl FocusWithinResult {
    /// Returns the current data attributes for the focus-within interaction.
    /// Frameworks must call this each render to get up-to-date attrs.
    pub fn current_attrs(&self, config: &FocusWithinConfig) -> AttrMap {
        let _config = config;
        let mut attrs = AttrMap::new();
        if *self.state.borrow() {
            attrs.set_bool(HtmlAttr::Data("ars-focus-within"), true);
        }
        if *self.visible.borrow() {
            attrs.set_bool(HtmlAttr::Data("ars-focus-within-visible"), true);
        }
        attrs
    }
}
````

The returned `AttrMap` will contain data attributes (see below). Event handlers are registered
as typed methods on the component's `Api` struct, covering:

- `focus` — tracks when element receives focus, determines modality
- `blur` — tracks when element loses focus
- `focusin` / `focusout` — used by `use_focus_within` to track container focus

Data attributes:

- `data-ars-focused` — present when `is_focused()` is true
- `data-ars-focus-visible` — present when `is_focus_visible()` is true
- `data-ars-focus-within` — present (on containers) when `is_focus_within()` is true
- `data-ars-focus-within-visible` — present (on containers) when `is_focus_within_visible()` is true

---

## 5. Long Press Interaction

### 5.1 Overview

Long press detects when a user holds a pointer or keyboard key down for longer than a configurable threshold without releasing. It is used for context menus, extra options, or secondary actions. It is distinct from press: a completed long press does not also fire `on_press` (the release after a long press fires `on_press_end` but not `on_press`).

**Cross-interaction cancellation:** When LongPress fires (timer elapses), it MUST signal the co-located Press interaction to suppress `on_press` on the subsequent `pointerup`. This is achieved by setting a shared `long_press_fired` flag that Press checks before firing activation. Alternatively, LongPress can send a synthetic `PointerCancel` event to the Press state machine, transitioning it to `Idle` without activation. See §8.7 for the full cancellation protocol.

Accessibility: Elements with long press actions must communicate this to screen reader users who cannot perform a long press. `ars-interactions` provides `LongPressConfig::accessibility_description` and `LongPressResult::description_attrs()` to link a `VisuallyHidden` description element to the interactive element via `aria-describedby`.

### 5.2 Timing and Threshold Interactions

- **Long-press default timing:** 500ms (configurable via `LongPressConfig::threshold`). This matches iOS long-press behavior.
- **Move threshold:** 5–10px of pointer displacement before a move is recognized. This threshold is independent from the long-press delay — movement within the threshold during a long-press hold does NOT cancel the long press.
- **Scroll-cancel threshold:** Uses the same threshold as move (5–10px). If the pointer moves beyond this threshold during a press or long-press, and the browser detects a scroll gesture (`pointercancel` on Android, momentum scroll on iOS), the interaction is cancelled.
- **Long-press vs. move interaction:** If a pointer moves beyond the move threshold before the long-press timer elapses, the long press is cancelled and `on_long_press_cancel` fires. The move threshold acts as a dead-zone that allows minor finger drift without cancellation.

### 5.3 Types

```rust
// ars-interactions/src/long_press.rs

use crate::{KeyModifiers, PointerType};
use ars_core::{
    AttrMap, Callback, ComponentIds, HtmlAttr, MessageFn, SharedState, TimerHandle,
};
use ars_i18n::Locale;
use core::time::Duration;
use std::{cell::RefCell, rc::Rc};

/// Configuration for long press interaction.
#[derive(Clone, Debug, PartialEq)]
pub struct LongPressConfig {
    /// Whether the element is disabled.
    pub disabled: bool,

    /// Duration the pointer or key must be held before a long press is detected.
    /// Defaults to 500ms (matching iOS long press behavior).
    pub threshold: Duration,

    /// Accessibility description of the long press action.
    /// This text will be set as the content of a VisuallyHidden span, and
    /// its ID will be added to the element's `aria-describedby`.
    /// Example: "Long press to open context menu"
    ///
    /// Uses `Option<String>` instead of `Option<&'static str>` — allows
    /// runtime-generated and localized descriptions.
    pub accessibility_description: Option<String>,

    /// Localized live-announcement text emitted when the long press fires.
    /// The adapter dispatches this with assertive priority.
    pub long_press_announcement: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Called when the hold begins and the interaction enters `Timing`.
    pub on_long_press_start: Option<Callback<dyn Fn(LongPressEvent)>>,

    /// Called when the threshold elapses while still pressed.
    pub on_long_press: Option<Callback<dyn Fn(LongPressEvent)>>,

    /// Called when the long press is cancelled before the threshold fires.
    pub on_long_press_cancel: Option<Callback<dyn Fn(LongPressEvent)>>,

    /// Shared state used to suppress the co-located `Press` activation after a
    /// completed long press.
    ///
    /// The threshold stores `Some(pointer_type)` for the modality that fired.
    /// The matching `Press` release consumes that value and suppresses only the
    /// originating activation.
    pub long_press_cancel_flag: Option<SharedState<Option<PointerType>>>,
}

impl Default for LongPressConfig {
    fn default() -> Self {
        Self {
            disabled: false,
            threshold: Duration::from_millis(500),
            accessibility_description: None,
            long_press_announcement: MessageFn::static_str("Long press activated"),
            on_long_press_start: None,
            on_long_press: None,
            on_long_press_cancel: None,
            long_press_cancel_flag: None,
        }
    }
}

/// A normalized long press event.
#[derive(Clone, Debug)]
pub struct LongPressEvent {
    /// How the long press was initiated.
    pub pointer_type: PointerType,

    /// The type of long press event.
    pub event_type: LongPressEventType,

    /// Client-space X coordinate. None for keyboard events.
    pub client_x: Option<f64>,

    /// Client-space Y coordinate. None for keyboard events.
    pub client_y: Option<f64>,

    /// Modifier keys held at the time of the event.
    pub modifiers: KeyModifiers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LongPressEventType {
    LongPressStart,
    LongPress,
    LongPressCancel,
}
```

### 5.4 State Machine

```text
States:
  Idle         — No press in progress
  Timing       — Press is held; timer is running; threshold not yet reached
  LongPressed  — Threshold elapsed; long press has fired

Transitions:

  Idle
    ─[PointerDown | KeyDown(Enter|Space), not disabled]──→ Timing
        action: reset cancel flag, start timer(threshold), record pointer_type + coordinates, emit on_long_press_start

  Timing
    ─[TimerFired (threshold elapsed)]────────────────────→ LongPressed
        action: emit on_long_press, announce long_press_announcement, set shared cancel flag
    ─[PointerUp | KeyUp before threshold]────────────────→ Idle
        action: cancel timer, emit on_long_press_cancel
    ─[PointerLeave | PointerCancel | Blur]───────────────→ Idle
        action: cancel timer, emit on_long_press_cancel
    ─[TouchMove > threshold_px from origin]──────────────→ Idle
        action: cancel timer (user is scrolling, not long-pressing)

  LongPressed
    ─[PointerUp | KeyUp]─────────────────────────────────→ Idle
        action: (no action; long press already fired)
    ─[PointerCancel | Blur]──────────────────────────────→ Idle
```

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum LongPressState {
    Idle,
    Timing {
        pointer_type: PointerType,
        origin_x: Option<f64>,
        origin_y: Option<f64>,
        /// Opaque handle to the pending threshold timer.
        timer_handle: TimerHandle,
    },
    LongPressed {
        pointer_type: PointerType,
    },
}
```

### 5.5 Accessibility Integration

```rust
pub struct LongPressResult {
    pub is_long_pressing: bool,
    state: Rc<RefCell<LongPressState>>,
    config: LongPressConfig,
}

impl LongPressResult {
    /// Returns data attributes for the long-press target element.
    /// Note: Neither `aria-disabled` nor `data-ars-disabled` is set here.
    /// When LongPress is composed with Press (the common pattern), Press handles
    /// `data-ars-disabled` and the component connect function handles `aria-disabled`.
    /// When LongPress is used standalone, the component connect function must set both.
    pub fn current_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        if matches!(*self.state.borrow(), LongPressState::Timing { .. } | LongPressState::LongPressed { .. }) {
            attrs.set_bool(HtmlAttr::Data("ars-long-pressing"), true);
        }
        attrs
    }
}

/// Data attributes for an associated VisuallyHidden description element.
/// Only Some if `accessibility_description` is set in config.
///
/// Usage:
///   <div {..long_press.current_attrs()}>...</div>
///   if let Some(desc_attrs) = long_press.description_attrs(ids) {
///     <span {..desc_attrs}>{config.accessibility_description}</span>
///   }
impl LongPressResult {
    pub fn description_attrs(&self, ids: &ComponentIds) -> Option<AttrMap> {
        self.config.accessibility_description.as_ref().map(|_desc| {
            let desc_id = ids.part("long-press-desc");
            let mut desc = AttrMap::new();
            desc.set(HtmlAttr::Id, desc_id);
            desc
        })
    }

    /// Returns the current long-press state snapshot.
    pub fn current_state(&self) -> LongPressState { /* ... */ }

    /// Returns the pending threshold timer while the interaction is timing.
    pub fn pending_timer_handle(&self) -> Option<TimerHandle> { /* ... */ }

    /// Adapter-facing transition helpers used by framework event handlers and timers.
    ///
    /// `begin_long_press()` enters `Timing`, `move_long_press()` applies the
    /// move-dead-zone cancellation rule, `cancel_long_press()` handles cancel/blur,
    /// `end_long_press()` handles release, and `fire_long_press()` handles the
    /// threshold timer firing. `fire_long_press()` returns the live-announcement
    /// string that the adapter must send with assertive priority.
    pub fn begin_long_press(&mut self, pointer_type: PointerType, client_x: Option<f64>, client_y: Option<f64>, modifiers: KeyModifiers, timer_handle: TimerHandle) { /* ... */ }
    pub fn move_long_press(&mut self, client_x: f64, client_y: f64) { /* ... */ }
    pub fn cancel_long_press(&mut self, client_x: Option<f64>, client_y: Option<f64>) { /* ... */ }
    pub fn end_long_press(&mut self, client_x: Option<f64>, client_y: Option<f64>) { /* ... */ }
    pub fn fire_long_press(&mut self, locale: &Locale) -> Option<String> { /* ... */ }
}

/// When used standalone (without Press composition), the component connect function
/// MUST set both `aria-disabled="true"` and `data-ars-disabled="true"` when
/// `config.disabled` is true, and MUST keep `tabindex="0"` per 03-accessibility.md §13.
pub fn use_long_press(config: LongPressConfig, ids: &ComponentIds) -> LongPressResult {
    let state = Rc::new(RefCell::new(LongPressState::Idle));

    // Snapshot at call time. Adapters expose as a reactive signal; `current_attrs()`
    // provides live state for attribute rendering.
    let is_long_pressing = matches!(*state.borrow(), LongPressState::Timing { .. } | LongPressState::LongPressed { .. });

    // Event handlers are registered as typed methods on the component's Api struct
    // and delegate into the adapter-facing helpers on LongPressResult:
    //   begin_long_press()  → Idle ──→ Timing (starts threshold timer, emits on_long_press_start)
    //   fire_long_press()   → Timing ──→ LongPressed (emits on_long_press, announces long_press_announcement)
    //   end_long_press()    → Timing ──→ Idle (emits on_long_press_cancel)
    //   cancel_long_press() → Timing ──→ Idle (emits on_long_press_cancel)
    //   move_long_press()   → Timing ──→ Idle when move exceeds threshold_px
    //   end_long_press()    → LongPressed ──→ Idle (no action)

    LongPressResult { is_long_pressing, state, config }
}
```

When `accessibility_description` is set, `description_attrs()` will include `id="{base_id}-long-press-desc"`. The description element should be rendered as a `VisuallyHidden` span adjacent to the interactive element. **The component's connect function must explicitly set `aria-describedby="{base_id}-long-press-desc"` on the target element** — `current_attrs()` does not set it automatically because the target element's AttrMap is managed by the component, not by `use_long_press`.

When the long press fires, the adapter resolves `LongPressConfig::long_press_announcement`
with the active locale and posts it through the provider's live announcer with
assertive priority. This announcement is additive to the static
`accessibility_description` guidance and exists to confirm that the long-press
action actually triggered.

Data attributes on the interactive element:

- `data-ars-long-pressing` — present while `LongPressState::Timing` or `LongPressState::LongPressed`

---

## 6. Move Interaction

### 6.1 Overview

Move interaction tracks continuous pointer or keyboard movement on an element. It is used by sliders, color area pickers, drag handles, and any element where the user controls a value by moving a pointer across a surface. It abstracts mouse drag (mousemove while mousedown), touch drag (touchmove), pen drag, and arrow key repetition into a unified stream of delta events.

Move differs from drag-and-drop (§7): move is about controlling an element's own value via position change; DnD is about transferring items between containers.

### 6.2 Types

```rust
// ars-interactions/src/move_interaction.rs

use crate::{PointerType, KeyModifiers};
use ars_core::AttrMap;
use std::{cell::RefCell, rc::Rc};

/// Configuration for move interaction.
// Manual Debug impl omitted for brevity — prints closures as "<closure>"
#[derive(Clone, Default)]
pub struct MoveConfig {
    /// Whether the element is disabled.
    pub disabled: bool,

    /// Called when movement begins (pointer down or first arrow key).
    pub on_move_start: Option<Rc<dyn Fn(MoveEvent)>>,

    /// Called for each movement delta.
    pub on_move: Option<Rc<dyn Fn(MoveEvent)>>,

    /// Called when movement ends (pointer up or no more arrow keys).
    pub on_move_end: Option<Rc<dyn Fn(MoveEvent)>>,
}

/// A normalized move event describing a positional delta.
#[derive(Clone, Debug)]
pub struct MoveEvent {
    /// How the movement was initiated.
    pub pointer_type: PointerType,

    /// The type of move event.
    pub event_type: MoveEventType,

    /// Horizontal delta in CSS pixels.
    /// Positive = rightward. For keyboard, this is a logical unit (e.g., 1 or -1).
    pub delta_x: f64,

    /// Vertical delta in CSS pixels.
    /// Positive = downward. For keyboard, this is a logical unit.
    pub delta_y: f64,

    /// Modifier keys held at the time of the event.
    pub modifiers: KeyModifiers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveEventType {
    MoveStart,
    Move,
    MoveEnd,
}
```

### 6.3 State Machine

```text
States:
  Idle      — No movement in progress
  Moving    — Movement is active

Transitions:

  Idle
    ─[PointerDown]──────────────────────→ Moving
        action: record start position, set pointer capture, emit on_move_start
        (Pointer Events API covers mouse, pen, AND touch)
    ─[ArrowKey (with focus)]────────────→ Moving (momentary; keydown starts, keyup ends)
        action: compute delta from key (ArrowRight→(1,0), ArrowLeft→(-1,0), etc.)
                emit on_move_start, then immediately on_move with delta

  Moving
    ─[PointerMove]──────────────────────→ Moving (self-transition)
        action: compute delta from previous position, emit on_move
    ─[PointerUp]────────────────────────→ Idle
        action: emit on_move_end, release pointer capture
    ─[PointerCancel]────────────────────→ Idle
        action: emit on_move_end (cancelled)
    ─[ArrowKey Released]────────────────→ Idle
        action: emit on_move_end
```

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum MoveState {
    Idle,
    Moving {
        pointer_type: PointerType,
        /// Last seen client X for delta computation.
        last_x: f64,
        /// Last seen client Y for delta computation.
        last_y: f64,
    },
}
```

### 6.4 Keyboard Arrow Key Deltas

For keyboard-driven move, arrow key deltas are intentionally small (1 logical unit) to allow fine control. Components using `use_move` for keyboard should multiply the delta by a step size appropriate to their value range. Shift+Arrow produces a larger delta (10 units by convention) for faster adjustment.

```rust
use ars_core::Direction;

// Direction (Ltr/Rtl) is re-exported from ars-core (canonical definition: ars-i18n,
// see 04-internationalization.md §3.1). ars-interactions depends on ars-core, not ars-i18n directly.
fn key_to_delta(key: KeyboardKey, dir: Direction, modifiers: KeyModifiers) -> Option<(f64, f64)> {
    debug_assert!(dir != Direction::Auto, "key_to_delta requires a resolved direction");
    let step = if modifiers.shift { 10.0 } else { 1.0 };
    let h_step = if dir.is_rtl() { -step } else { step };
    match key {
        KeyboardKey::ArrowRight => Some((h_step, 0.0)),
        KeyboardKey::ArrowLeft  => Some((-h_step, 0.0)),
        KeyboardKey::ArrowDown  => Some((0.0, step)),
        KeyboardKey::ArrowUp    => Some((0.0, -step)),
        KeyboardKey::Home       => {
            let home = if dir.is_rtl() { f64::INFINITY } else { f64::NEG_INFINITY };
            Some((home, 0.0))
        }
        KeyboardKey::End        => {
            let end = if dir.is_rtl() { f64::NEG_INFINITY } else { f64::INFINITY };
            Some((end, 0.0))
        }
        // Page keys use a fixed 10x multiplier on top of the step (which is 10x when
        // Shift is held). The resulting 100x jump for Shift+Page is intentional:
        // it maps to "jump to boundary" behavior, consistent with Windows/macOS HIG.
        KeyboardKey::PageUp     => Some((0.0, -step * 10.0)),
        KeyboardKey::PageDown   => Some((0.0, step * 10.0)),
        _ => None,
    }
}
```

### 6.5 CSS Zoom / Scale Coordinate Transformation

When a move interaction operates inside a container that has CSS `zoom` or `transform: scale()` applied (common in Splitter panels, scaled canvases, and browser zoom), raw pointer deltas from `pointermove` are in viewport coordinates, not container-local coordinates. Without correction, movements appear faster or slower than expected.

**On `DragStart` (transition to `Moving`)**, compute the scale factor:

```rust
// Compute scale from the difference between CSS and layout dimensions:
let rect = container.get_bounding_client_rect();
let scale_x = rect.width() / container.offset_width() as f64;
let scale_y = rect.height() / container.offset_height() as f64;
// Store scale factors in MoveState::Moving for use during pointermove
```

**On each `PointerMove`**, apply the inverse scale to pointer deltas:

```rust
// In the pointermove handler:
let adjusted_dx = raw_delta_x / scale_x;
let adjusted_dy = raw_delta_y / scale_y;
// Emit MoveEvent with adjusted deltas
```

**Affected components:** This pattern MUST be applied in all coordinate-dependent components:

- **Slider** — thumb drag along the track
- **ColorPicker** — 2D area and hue/alpha strip drag
- **Splitter** — panel resize handle drag (see `spec/components/layout/splitter.md`)

Adapter implementations SHOULD compute the scale factor once on drag start and cache it for the duration of the move, rather than recomputing per frame. If the container's scale can change mid-drag (e.g., animated zoom), recompute on each frame.

### 6.6 Output Props

```rust
pub fn use_move(config: MoveConfig) -> MoveResult {
    let state = Rc::new(RefCell::new(MoveState::Idle));

    // Event handlers are registered as typed methods on the component's Api struct:
    //   pointerdown  → Idle ──→ Moving (captures pointer, records origin)
    //   pointermove  → Moving: compute delta, emit on_move with (dx, dy)
    //   pointerup    → Moving ──→ Idle (emits on_move_end)
    //   pointercancel → Moving ──→ Idle
    //   keydown(Arrow/Home/End/Page) → emit on_move with computed delta from key_to_delta()

    MoveResult { state }
}

/// MoveResult now uses the same `current_attrs()` pattern as Press/Hover/Focus/LongPress,
/// reading from live `Rc<RefCell<State>>` on each call. The adapter calls `current_attrs()`
/// per render (or per reactive read) rather than re-calling `use_move()`.
/// The static class `ars-touch-none` is always included.
pub struct MoveResult {
    state: Rc<RefCell<MoveState>>,
}

impl MoveResult {
    /// Returns the current AttrMap by reading from live state.
    /// Consistent with Press/Hover/Focus/LongPress `current_attrs()` pattern.
    pub fn current_attrs(&self) -> AttrMap {
        let moving = matches!(*self.state.borrow(), MoveState::Moving { .. });
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Class, "ars-touch-none"); // prevents browser touch gesture interception
        if moving {
            attrs.set_bool(HtmlAttr::Data("ars-moving"), true);
        }
        attrs
    }

    /// Whether the element is currently being moved.
    pub fn is_moving(&self) -> bool {
        matches!(*self.state.borrow(), MoveState::Moving { .. })
    }
}
```

> **`touch-action: none` requirement:** The `AttrMap` returned by `use_move` MUST include the `ars-touch-none` class from the companion stylesheet on the target element. This prevents the browser from intercepting touch gestures (pan, pinch-zoom) on elements that use pointer-driven move, ensuring `pointermove` events fire reliably on touch devices.
>
> ```rust
> // Inside use_move implementation:
> attrs.set(HtmlAttr::Class, "ars-touch-none");
> ```
>
> **Safari `touch-action` inheritance caveat:** `touch-action: none` must be applied directly to the interactive element, not just a parent container. Safari does not inherit `touch-action` from ancestors. Always place the `ars-touch-none` class on the element that receives pointer events.

The returned `AttrMap` contains data attributes (see below). Event handlers are registered
as typed methods on the component's `Api` struct, covering:

- `pointerdown` — starts move for mouse/pen, captures pointer
- `pointermove` — emits move deltas (only when Moving)
- `pointerup` / `pointercancel` — ends move (Pointer Events API also covers touch)
- `keydown` — arrow key move start/step (only when element has focus)
- `keyup` — arrow key move end

Data attributes:

- `data-ars-moving` — present while moving

---

## 7. Drag and Drop

### 7.1 Overview

`ars-interactions` provides a complete drag-and-drop system that works across:

- Mouse: HTML5 Drag and Drop API
- Touch: Synthesized from `touchmove` / `touchend` (native iOS/Android touch)
- Keyboard: A full keyboard DnD protocol (Enter to start, Tab to cycle targets, Escape to cancel)
- Screen readers: ARIA live announcements at each stage

The DnD system is built on two halves — draggable sources and drop targets — each with their own config, state machine, and props.

> **Cross-reference:** Shared positioning types used by drop indicators (`Side`, `Rect`, `Overflow`, etc.) are defined in §9.2 below.

#### 7.1.1 Pointer Event Isolation During Drag

During active drag operations, pointer events on sibling elements can cause unintended hover state flicker (e.g., a button adjacent to a draggable element shows hover styling as the drag passes over it). To prevent this:

1. **Set `pointer-events: none`** on all non-drag-participating siblings while a drag is active. The adapter applies this via a CSS class (e.g., `.ars-drag-active *:not(.ars-dragging) { pointer-events: none }`) on the drag container.
2. **Restore on drag end**: Remove the class on `dragend` or `pointerup`, restoring normal pointer event behavior.
3. **Scope**: Only elements within the same drag container are affected. Elements outside the container retain normal pointer events.

### 7.2 Item Types

```rust
// ars-interactions/src/drag_drop.rs

/// The data associated with a drag operation.
/// Multiple types may be present for cross-application compatibility
/// (e.g., both Text and Html for rich paste targets).
#[derive(Clone, Debug)]
pub enum DragItem {
    /// Plain text content.
    Text(String),

    /// URI/URL string.
    Uri(String),

    /// HTML-formatted text.
    Html(String),

    /// A file reference (from file-system drop or file input).
    File {
        name: String,
        mime_type: String,
        /// Size in bytes.
        size: u64,
        /// Opaque file handle; resolved asynchronously in ars-dom.
        handle: FileHandle,
    },

    /// A directory reference.
    Directory {
        name: String,
        /// Opaque handle to enumerate children.
        handle: DirectoryHandle,
    },

    /// Custom application-defined data type.
    /// The `mime_type` field is the MIME type string used in DataTransfer.
    /// Prefer MIME types in the form `application/x-ars-{type}` for app-internal data.
    Custom {
        mime_type: String,
        data: String,
    },
}

/// Opaque handles — resolved by ars-dom against the browser File API.
#[derive(Clone, Debug)]
pub struct FileHandle(/* web_sys::File */);

#[derive(Clone, Debug)]
pub struct DirectoryHandle(/* web_sys::FileSystemDirectoryEntry */);
```

### 7.3 Drop Operation

```rust
/// The type of operation that will occur when items are dropped.
/// Maps to the HTML5 `DataTransfer.dropEffect` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DropOperation {
    /// Move items from source to target (source is removed).
    Move,

    /// Copy items to target (source is preserved).
    Copy,

    /// Create a link/shortcut to the source items.
    Link,

    /// Drop is not accepted; no operation will occur.
    Cancel,
}

impl DropOperation {
    /// Returns the HTML5 DataTransfer dropEffect string.
    pub fn as_drop_effect(&self) -> &'static str {
        match self {
            DropOperation::Move   => "move",
            DropOperation::Copy   => "copy",
            DropOperation::Link   => "link",
            DropOperation::Cancel => "none",
        }
    }
}
```

### 7.4 Drag Source Configuration

```rust
/// Configuration for a draggable element.
// Manual Debug impl omitted for brevity — prints closures as "<closure>"
#[derive(Clone, Default)]
pub struct DragConfig {
    /// Whether dragging is disabled.
    pub disabled: bool,

    /// The items this element exposes for dragging.
    /// Called when a drag begins; items are captured at that moment.
    ///
    /// These zero-argument item suppliers use shared `Arc` ownership directly
    /// rather than `Callback`, because they are value-producing resources
    /// rather than event callbacks and must carry the normalized
    /// `Send + Sync + 'static` contract from `01-architecture.md` §1.5.
    pub items: Option<Arc<dyn Fn() -> Vec<DragItem> + Send + Sync>>,

    /// The set of drop operations this source allows.
    /// Defaults to all operations.
    pub allowed_operations: Option<Vec<DropOperation>>,

    /// Called when drag begins.
    pub on_drag_start: Option<Callback<dyn Fn(DragStartEvent)>>,

    /// Called when the drag ends (regardless of outcome).
    pub on_drag_end: Option<Callback<dyn Fn(DragEndEvent)>>,

    /// For multi-item drag: returns additional selected items to include.
    /// When both `items` and `get_items` are set, their results are **unioned**:
    /// the drag payload is `items() ∪ get_items()`. Use `items` for the primary
    /// dragged element and `get_items` for additional selected items.
    /// When None, only the dragged element's items are transferred.
    pub get_items: Option<Arc<dyn Fn() -> Vec<DragItem> + Send + Sync>>,

    /// Screen reader announcement when drag starts.
    /// Example: "Started dragging {item_name}. Press Tab to navigate to a drop target."
    pub drag_start_announcement: Option<Callback<dyn Fn(&[DragItem]) -> String>>,
}

#[derive(Clone, Debug)]
pub struct DragStartEvent {
    pub items: Vec<DragItem>,
    pub pointer_type: PointerType,
}

#[derive(Clone, Debug)]
pub struct DragEndEvent {
    pub items: Vec<DragItem>,
    pub operation: DropOperation,
    pub pointer_type: PointerType,
    /// True if the drop was accepted by a target.
    pub was_dropped: bool,
}
```

### 7.5 Drop Target Configuration

```rust
/// Configuration for a drop target element.
// Manual Debug impl omitted for brevity — prints closures as "<closure>"
#[derive(Clone, Default)]
pub struct DropConfig {
    /// Whether dropping is disabled.
    pub disabled: bool,

    /// Called when dragged items enter this target.
    pub on_drag_enter: Option<Callback<dyn Fn(DropTargetEvent)>>,

    /// Called when dragged items leave this target.
    pub on_drag_leave: Option<Callback<dyn Fn(DropTargetEvent)>>,

    /// Called on each dragover tick; return the DropOperation to accept.
    /// Return DropOperation::Cancel to reject the drop.
    pub on_drag_over: Option<Callback<dyn Fn(DropTargetEvent) -> DropOperation>>,

    /// Called when items are dropped onto this target.
    pub on_drop: Option<Callback<dyn Fn(DropEvent)>>,

    /// The drop operations this target accepts.
    /// If None, accepts all operations offered by the source.
    pub accepted_operations: Option<Vec<DropOperation>>,

    /// The MIME types / item kinds this target accepts.
    /// If None, accepts any item type.
    pub accepted_types: Option<Vec<String>>,

    /// Where within the target the drop indicator should be shown.
    pub drop_indicator_position: DropIndicatorPosition,

    /// Screen reader announcement when a dragged item enters.
    pub drag_enter_announcement: Option<Callback<dyn Fn(&DropTargetEvent) -> String>>,

    /// Screen reader announcement when drop succeeds.
    pub drop_announcement: Option<Callback<dyn Fn(&DropEvent) -> String>>,
}

/// **Announcement dispatch precedence:** Per-element announcement closures on
/// `DragConfig` and `DropConfig` take precedence over the corresponding
/// `DragAnnouncements` field. When a per-element closure is `None`, the
/// `DragAnnouncements` default is used.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DropIndicatorPosition {
    /// Show indicator overlaid on the drop target (for zones).
    #[default]
    OnTarget,
    /// Show indicator before the target item in reading order.
    /// - Vertical: above the target.
    /// - Horizontal (LTR): left of the target.
    /// - Horizontal (RTL): right of the target (inline-start side).
    Before,
    /// Show indicator after the target item in reading order.
    /// - Vertical: below the target.
    /// - Horizontal (LTR): right of the target.
    /// - Horizontal (RTL): left of the target (inline-end side).
    After,
}

#[derive(Clone, Debug)]
pub struct DropTargetEvent {
    pub items: Vec<DragItemPreview>,
    pub operation: DropOperation,
    pub pointer_type: PointerType,
}

/// A preview of a drag item during hover — only type info, not full data.
/// Full data is only available in on_drop, not on_drag_enter/on_drag_over.
#[derive(Clone, Debug)]
pub struct DragItemPreview {
    pub kind: DragItemKind,
    pub mime_types: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragItemKind {
    Text,
    Uri,
    Html,
    File,
    Directory,
    Custom,
}

#[derive(Clone, Debug)]
pub struct DropEvent {
    /// The full item data, resolved from DataTransfer.
    pub items: Vec<DragItem>,
    pub operation: DropOperation,
    pub pointer_type: PointerType,
    /// Drop position indicator where within the target the drop occurred.
    pub drop_position: DropIndicatorPosition,
}
```

#### 7.5.1 Dropzone Accept Type Validation

When multiple files are dragged, each file's MIME type is validated individually against `accepted_types`. If **all** dragged items are rejected, the dropzone enters a "reject" visual state (adapter applies `data-drop-rejected` attribute). MIME type matching is case-insensitive and normalizes common aliases (e.g., `image/jpg` maps to `image/jpeg`). Wildcard patterns are supported: `image/*` matches any image MIME type.

#### 7.5.2 Text Selection Behavior During Drag-and-Drop

On drag start, adapters must apply `user-select: none` to the dragged element and `pointer-events: none` to drag preview elements. On drag end (drop or cancel), these styles are removed. This prevents browser text selection from interfering with custom drag visuals.

### 7.6 Drag State Machine (Source Side)

```text
States:
  Idle         — Not dragging
  Dragging     — Drag in progress; no target currently under pointer
  DragOver     — Drag in progress; pointer is over a valid drop target
  Dropped      — Drop has been accepted; cleanup in progress

Transitions:

  Idle
    ─[Pointer: mousedown / pointerdown, drag threshold exceeded]──→ Dragging
        action: call items(), build DataTransfer, set drag image,
                emit on_drag_start, announce to screen reader
    ─[Touch: long_press fires on draggable]───────────────────────→ Dragging
        action: synthesize drag from touch position
    ─[Keyboard: Enter key on focused draggable]───────────────────→ Dragging
        action: enter keyboard DnD mode, announce drop targets

  Dragging
    ─[dragenter on a valid drop target]──────────────────────────→ DragOver
    ─[dragend without target]────────────────────────────────────→ Idle
        action: emit on_drag_end(was_dropped=false, operation=Cancel)
    ─[Escape key (keyboard mode)]────────────────────────────────→ Idle
        action: cancel keyboard DnD, emit on_drag_end

  DragOver
    ─[dragleave from all valid targets]──────────────────────────→ Dragging
    ─[drop accepted]─────────────────────────────────────────────→ Dropped
        action: emit on_drag_end(was_dropped=true)
    ─[dragend without accepted drop]─────────────────────────────→ Idle
        action: emit on_drag_end(was_dropped=false)
    ─[Escape key (keyboard mode)]────────────────────────────────→ Idle

  Dropped
    ─[Cleanup complete]──────────────────────────────────────────→ Idle
```

```rust
// PartialEq not derived — FileHandle/DirectoryHandle (web_sys wrappers) do not implement it.
// Use a version counter for change detection instead.
#[derive(Clone, Debug)]
pub enum DragState {
    Idle,
    Dragging {
        items: Vec<DragItem>,
        pointer_type: PointerType,
    },
    DragOver {
        items: Vec<DragItem>,
        pointer_type: PointerType,
        /// ID of the target currently being hovered.
        target_id: String,
        current_operation: DropOperation,
    },
    Dropped {
        operation: DropOperation,
    },
}
```

### 7.7 Keyboard Drag and Drop Protocol

Keyboard DnD follows a modal interaction pattern:

1. **Start**: Press `Enter` on a focused draggable element. The element is now "picked up". Screen reader announces: "Started dragging {name}. Press Tab or Shift+Tab to move between drop targets, Escape to cancel."

2. **Navigation**: `Tab` / `Shift+Tab` cycles through registered drop targets in document order. Each time focus moves to a drop target, `on_drag_enter` is called and the screen reader announces: "Drop target: {target_name}. Press Enter to drop here."

3. **Drop**: `Enter` on a focused drop target accepts the drop. `on_drop` fires. Screen reader announces: "Dropped {name} into {target_name}."

4. **Cancel**: `Escape` at any time cancels the drag. Screen reader announces: "Drag cancelled."

```rust
/// Keyboard DnD registry: tracks all live drop targets for Tab-cycling.
/// Registered automatically by use_drop when keyboard DnD is active.
/// Cleared when the drag ends.
pub struct KeyboardDragRegistry {
    targets: Vec<KeyboardDropTarget>,
    current_index: Option<usize>,
}

pub struct KeyboardDropTarget {
    /// The element ID of the drop target.
    pub element_id: String,
    /// Screen reader label for the target.
    pub label: String,
    pub config: DropConfig,
}
```

### 7.8 Screen Reader DnD Announcements

```rust
/// Screen reader announcements for drag-and-drop operations.
/// Follows the canonical `MessageFn` pattern from `ars-i18n` (`04-internationalization.md` §7.1):
/// cfg-gated `Rc`/`Arc` wrapper, `+ Send + Sync` on all targets, `&Locale` parameter.
///
/// All closure fields use `MessageFn::new()` which delegates to the cfg-gated `From` impls
/// defined in `04-internationalization.md` §7.1 — `Rc` on WASM, `Arc` on native.
///
/// **`+ Send + Sync` bounds:** All `MessageFn` closures include `+ Send + Sync` as a
/// deliberate project-wide convention. On WASM targets the `MessageFn` wrapper uses `Rc`
/// (non-atomic), but the trait object bounds remain `Send + Sync` so that closures are
/// thread-safe for native desktop targets without cfg-gated API differences. See the
/// `MessageFn` definition in `04-internationalization.md` §7.1 for the full rationale.
#[derive(Clone)]
pub struct DragAnnouncements {
    /// "Started dragging {item_count} item(s). Press Tab or Shift+Tab for drop targets, Escape to cancel."
    pub drag_start: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,

    /// "Drop target: {name}. Press Enter to drop here, Escape to cancel."
    pub drag_enter: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,

    /// "Leaving drop target: {name}."
    pub drag_leave: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,

    /// "Dropped {item_count} item(s) into {target_name}."
    pub drop: MessageFn<dyn Fn(usize, &str, &Locale) -> String + Send + Sync>,

    /// "Drag cancelled."
    pub drag_cancel: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for DragAnnouncements {
    fn default() -> Self {
        Self {
            drag_start: MessageFn::new(|count: usize, _locale: &Locale| {
                if count == 1 { format!("Started dragging 1 item. Press Tab or Shift+Tab to move between drop targets, Escape to cancel.") }
                else { format!("Started dragging {count} items. Press Tab or Shift+Tab to move between drop targets, Escape to cancel.") }
            }),
            drag_enter: MessageFn::new(|name: &str, _locale: &Locale| {
                format!("Drop target: {name}. Press Enter to drop here, Escape to cancel.")
            }),
            drag_leave: MessageFn::new(|name: &str, _locale: &Locale| {
                format!("Left drop target: {name}")
            }),
            drop: MessageFn::new(|count: usize, target: &str, _locale: &Locale| {
                if count == 1 { format!("Dropped 1 item into {target}") }
                else { format!("Dropped {count} items into {target}") }
            }),
            drag_cancel: MessageFn::new(|_locale: &Locale| "Drag cancelled".into()),
        }
    }
}

impl core::fmt::Debug for DragAnnouncements {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DragAnnouncements").finish_non_exhaustive()
    }
}
```

Announcements are posted to `ars-a11y`'s `LiveAnnouncer` with `AnnouncementPriority::Assertive` for critical events (drag start, drop, cancel) and `AnnouncementPriority::Polite` for informational events (drag enter, drag move).

```rust
// Adapter dispatch: DragAnnouncements → LiveAnnouncer
// MessageFn<T> implements Deref to the inner Fn trait object (see 04-internationalization.md §7.1),
// so the call-syntax `(field)(args...)` works via auto-deref through Rc/Arc.
let message = (announcements.drag_start)(item_count, &locale);
announcer.announce_with_priority(message, AnnouncementPriority::Assertive);
```

### 7.9 Multi-Item Drag

When a draggable is part of a selection (e.g., dragging one of several selected list items), the drag should include all selected items, not just the one that was grabbed.

```rust
impl DragConfig {
    /// Convenience builder for multi-item drag.
    /// `get_selected_items` returns all items in the current selection.
    pub fn with_selection(
        mut self,
        get_selected_items: impl Fn() -> Vec<DragItem> + Send + Sync + 'static,
    ) -> Self {
        self.get_items = Some(Arc::new(get_selected_items));
        self
    }
}
```

The drag image should reflect the count: when dragging 3 selected items, the ghost image shows a badge "3 items" overlaid.

### 7.10 Drop Indicators and Positioning

> **Design note:** DragResult and DropResult use a snapshot-based `attrs` field rather than the `current_attrs()` pattern used by Press/Hover/Focus/LongPress/Move. This is because drag operations span multiple event cycles and the attrs must remain stable during a drag sequence. Components using DnD should re-call `use_drag`/`use_drop` on each render to refresh the snapshot.

```rust
/// DragResult/DropResult attrs are snapshots. The adapter calls `use_drag`/`use_drop`
/// on each render, so the snapshot is rebuilt with current state values each time.
/// This is unlike Press/Hover/Focus which use `current_attrs()` for fine-grained reactivity.
pub struct DropResult {
    /// Data attributes to spread onto the drop target element.
    pub attrs: AttrMap,

    /// Whether a dragged item is currently over this target.
    pub drag_over: bool,

    /// The operation that will occur if dropped now.
    pub drop_operation: Option<DropOperation>,

    /// Where the drop indicator line should appear.
    pub indicator_position: Option<DropIndicatorPosition>,
}

pub struct DragResult {
    /// Data attributes to spread onto the draggable element.
    pub attrs: AttrMap,

    /// Whether this element is currently being dragged.
    pub dragging: bool,
}

pub fn use_drag(config: DragConfig) -> DragResult {
    let state = Rc::new(RefCell::new(DragState::Idle));

    let dragging = !matches!(*state.borrow(), DragState::Idle);

    let mut attrs = AttrMap::new();
    attrs.set(HtmlAttr::Draggable, "true");
    if dragging {
        attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
    }

    // Event handlers registered on the component's Api struct:
    //   dragstart → build DataTransfer from config.items(), set effect_allowed,
    //               optionally set drag image from config.preview,
    //               transition Idle ──→ Dragging, emit on_drag_start
    //   dragend   → read drop_effect from DataTransfer,
    //               transition ──→ Idle, emit on_drag_end with operation result

    DragResult { attrs, dragging }
}

pub fn use_drop(config: DropConfig) -> DropResult {
    let drag_over = Rc::new(RefCell::new(false));
    let drop_op = Rc::new(RefCell::new(None::<DropOperation>));
    let indicator_pos = Rc::new(RefCell::new(None::<DropIndicatorPosition>));
    let enter_count = Rc::new(RefCell::new(0i32)); // track enter/leave nesting

    let is_drag_over = *drag_over.borrow();
    let current_op = *drop_op.borrow();
    let current_pos = *indicator_pos.borrow();

    let mut attrs = AttrMap::new();
    if is_drag_over {
        attrs.set_bool(HtmlAttr::Data("ars-drag-over"), true);
        if let Some(op) = current_op {
            attrs.set(HtmlAttr::Data("ars-drop-operation"), match op {
                DropOperation::Move => "move",
                DropOperation::Copy => "copy",
                DropOperation::Link => "link",
                DropOperation::Cancel => "none",
            });
        }
        if let Some(pos) = current_pos {
            attrs.set(HtmlAttr::Data("ars-drop-position"), match pos {
                DropIndicatorPosition::Before => "before",
                DropIndicatorPosition::After => "after",
                DropIndicatorPosition::OnTarget => "on",
            });
        }
    }

    // Event handlers registered on the component's Api struct:
    //   dragenter → prevent_default, increment enter_count, set drag_over = true,
    //               validate accepted_types against DataTransfer.types
    //   dragleave → decrement enter_count; if 0, set drag_over = false
    //   dragover  → prevent_default, set drop_effect, compute indicator position
    //   drop      → prevent_default, reset state, extract items, emit on_drop

    DropResult {
        attrs,
        drag_over: is_drag_over,
        drop_operation: current_op,
        indicator_position: current_pos,
    }
}
```

#### 7.10.1 Pointer Capture Error Recovery During Drag

When drag-and-drop uses `setPointerCapture()` (for touch-synthesized drags), an error during drag processing can leave pointer capture active on the wrong element. Subsequent pointer events then route incorrectly, breaking all pointer interactions until the page is reloaded.

Adapter implementations MUST wrap drag effect setup in a try-catch and release pointer capture on error:

```javascript
// Adapter pseudocode for pointer-capture-safe drag:
try {
    element.setPointerCapture(pointerId);
    // ... drag processing ...
} catch (e) {
    // Ensure pointer capture is released even on error
    try {
        element.releasePointerCapture(pointerId);
    } catch (_) {
        // releasePointerCapture throws if pointerId was never captured;
        // this is expected when setPointerCapture itself failed.
    }
    console.warn(
        "ars-interactions: pointer capture released due to error during drag:",
        e,
    );
    // Transition drag state machine back to Idle
    transition(DragState::Idle);
}
```

Additionally, adapters MUST track whether `setPointerCapture` was called and verify a matching `releasePointerCapture` runs during cleanup. If cleanup detects an unmatched capture (capture was set but never released), emit a `console.warn` diagnostic and force-release the capture.

Data attributes on draggable elements:

- `data-ars-dragging` — present while dragging

Data attributes on drop targets:

- `data-ars-drag-over` — present while a dragged item is over this target
- `data-ars-drop-operation="move|copy|link"` — the pending operation (for styling)
- `data-ars-drop-position="before|after|on"` — where the indicator should render

CSS example for a reorderable list:

```css
[data-ars-part="list-item"][data-ars-drop-position="before"]::before {
    content: "";
    display: block;
    height: 2px;
    background: var(--ars-accent);
}
[data-ars-part="list-item"][data-ars-drop-position="after"]::after {
    content: "";
    display: block;
    height: 2px;
    background: var(--ars-accent);
}
```

---

### 7.11 Directional Resolution (RTL Support)

All horizontal keyboard navigation must account for RTL text direction. Components MUST NOT hardcode ArrowLeft as "previous" and ArrowRight as "next".

```rust
// ars-interactions/src/direction.rs

/// Logical direction in reading order, independent of text direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalDirection {
    /// "Next" in reading order (right in LTR, left in RTL).
    Forward,
    /// "Previous" in reading order (left in LTR, right in RTL).
    Backward,
}

/// Resolve a physical arrow key to a logical direction based on text direction.
///
/// Returns `None` for non-horizontal arrow keys (`ArrowUp`, `ArrowDown`) and
/// any other key that is not `ArrowLeft` or `ArrowRight`.
///
/// # Panics
///
/// Debug-asserts that `direction` is not [`Direction::Auto`]. Callers must
/// resolve `Auto` to a concrete `Ltr` or `Rtl` before calling this function.
pub fn resolve_arrow_key(key: KeyboardKey, direction: Direction) -> Option<LogicalDirection> {
    debug_assert!(
        direction != Direction::Auto,
        "resolve_arrow_key requires a resolved direction"
    );
    match (key, direction) {
        (KeyboardKey::ArrowRight, Direction::Ltr)
        | (KeyboardKey::ArrowLeft, Direction::Rtl) => Some(LogicalDirection::Forward),
        (KeyboardKey::ArrowLeft, Direction::Ltr)
        | (KeyboardKey::ArrowRight, Direction::Rtl) => Some(LogicalDirection::Backward),
        _ => None,
    }
}
```

**Affected components**: Tabs, RadioGroup, Slider, Splitter, TreeView, Carousel, Toolbar, Menu (horizontal), and the Move interaction. All MUST use `resolve_arrow_key()` instead of matching raw arrow key strings.

---

## 8. Interaction Composition

### 8.1 The Composition Problem

A typical interactive element like a `Button` requires attrs from three interactions: press, hover, and focus. Each interaction's `use_*` function returns an `AttrMap` set. Naively assigning each set to the element would lose all but the last, since each assignment replaces the previous value for a given attribute.

Composition solves this by merging all attrs sets together, unioning data attributes and styles so all interactions' attributes are applied to the element. Event handlers are composed separately via typed methods on per-component `Api` structs.

### 8.2 merge_attrs

````rust
// ars-interactions/src/compose.rs

use ars_core::AttrMap;

/// Merge multiple AttrMap sets into a single AttrMap.
///
/// Attribute precedence:
///   For data attributes, the LAST value for a given key wins.
///   (The rightmost attrs set is authoritative for attributes.)
///   Exception: Space-separated token attributes (class, rel, ARIA ID lists)
///   are appended with dedup per `AttrMap::set()` semantics, not overwritten.
///
/// Style merging:
///   Styles are merged; the last value for a given property wins.
///
/// Note: Event handlers are no longer part of AttrMap. They are composed
/// separately via typed methods on per-component `Api` structs.
///
/// # Example
///
/// ```rust
/// let press = use_press(press_config.clone());
/// let hover = use_hover(hover_config);
/// let focus = use_focus(focus_config);
///
/// // All three sets of data attributes are applied to the element.
/// let button_attrs = merge_attrs([press.current_attrs(&press_config), hover.current_attrs(), focus.current_attrs()]);
/// ```
pub fn merge_attrs<I>(attrs_iter: I) -> AttrMap
where
    I: IntoIterator<Item = AttrMap>,
{
    let mut merged = AttrMap::new();
    for attrs in attrs_iter {
        merged.merge(attrs);
    }
    merged
}

/// Convenience macro for merging a fixed set of attrs without constructing a Vec.
///
/// ```rust
/// let attrs = merge_attrs!(press.current_attrs(&press_config), hover.current_attrs(), focus.current_attrs());
/// ```
#[macro_export]
macro_rules! merge_attrs {
    ($($attrs:expr),+ $(,)?) => {
        $crate::compose::merge_attrs([$($attrs),+])
    };
}
````

### 8.3 Event Handler Ordering and Precedence

Event handlers are typed methods on per-component `Api` structs and execute in a defined order:

1. The interaction-level handlers (from `use_press`, `use_hover`, etc.) fire first.
2. Component-level handlers (from the component's own connect function) fire second.
3. User-level handlers (from `on_click`, `on_key_down` callbacks passed by the consumer) fire last.

This ordering ensures that interaction state is updated before component logic runs, and component logic runs before user callbacks observe the final state.

```rust
// Conceptual composition example showing how a Button merges multiple interaction
// AttrMaps. In practice, use_press/use_hover/use_focus are initialized ONCE during
// component setup (not inside per-render attrs methods). The adapter holds the
// interaction results and passes them into the connect/attrs call.
pub fn button_attrs(&self) -> AttrMap {
    // 1. Start with interaction attrs
    let send = self.send.clone();
    let press_config = PressConfig {
        on_press: Some(Rc::new(move |_| (send)(Event::Press))),
        ..Default::default()
    };
    let press_result = use_press(press_config.clone());
    let hover_result = use_hover(HoverConfig::default());
    let focus_result = use_focus(FocusConfig::default());

    // 2. Build component-level attrs
    let mut component_attrs = AttrMap::new();
    component_attrs.set(HtmlAttr::Data("ars-scope"), "button");
    component_attrs.set(HtmlAttr::Data("ars-part"), "root");
    component_attrs.set(HtmlAttr::Role, "button");
    // §13 of 03-accessibility.md: aria-disabled elements remain focusable (tabindex "0")
    component_attrs.set(HtmlAttr::TabIndex, "0");

    if self.disabled {
        component_attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        component_attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
    }

    // 3. Merge: interactions first, then component attrs.
    // Event handlers are composed separately via typed Api methods.
    merge_attrs!(press_result.current_attrs(&press_config), hover_result.current_attrs(), focus_result.current_attrs(), component_attrs)
}
```

### 8.4 User-Provided Handler Chaining

When a component consumer attaches additional handlers (e.g., `on_click` on a `<Button>`), the adapter layer registers these as typed callbacks that fire after the interaction handlers:

```rust
// In ars-leptos Button adapter:
pub fn Button(props: Props) -> impl IntoView {
    let ret = use_machine::<button::Machine>(props.into());
    let root_attrs = ret.with_api(|api| api.part_attrs(button::Part::Root));

    // Attach user callbacks via typed Api methods (fire after interaction handlers).
    // Event handlers are NOT part of AttrMap — they are registered on the Api.
    if let Some(on_click) = props.on_click {
        ret.on_click(move |e| on_click(e));
    }

    // Spread root_attrs onto the DOM element
    view! { <div {..root_attrs}>{props.children}</div> }
}
```

### 8.5 Attribute Conflict Resolution

Some attributes may be set by multiple interactions or by both the component and the user. The precedence rules are:

| Attribute      | Winner                                                                               |
| -------------- | ------------------------------------------------------------------------------------ |
| `class`        | Concatenated with dedup — handled by `set()` space-separated semantics               |
| `style`        | Last write wins per property                                                         |
| `aria-*`       | Component attrs win over interaction attrs (component is semantically authoritative) |
| `data-ars-*`   | Last write wins (each interaction owns its own attribute names)                      |
| Event handlers | Composed via typed `Api` methods; all fire in registration order                     |
| `tabindex`     | Component attrs win (component determines focusability)                              |
| `id`           | Last write wins                                                                      |

Note: Class deduplication is handled by `AttrMap::set()` — space-separated token
list attributes (including `class`) are automatically appended with dedup.
See `SPACE_SEPARATED` in `01-architecture.md`.

**Dev-mode `style` conflict warning:** When the `ars-interactions/debug` feature is enabled, `merge_attrs` SHOULD emit a `log::warn!` if the same CSS property is set by two different interaction `AttrMap` sources with different values. This helps component authors detect unintentional style conflicts during development while keeping diagnostics routed through the application's logger setup:

```rust
// In merge_attrs, during style merging (debug feature only):
#[cfg(feature = "debug")]
if let Some(existing_value) = merged.iter_styles().find(|(k, _)| k == &property).map(|(_, v)| v) {
    if existing_value != &value {
        log::warn!(
            "ars-interactions: style property '{}' set by multiple interactions \
             (existing: '{}', new: '{}'). Last write wins.",
            property,
            existing_value,
            value
        );
    }
}
```

### 8.6 Full Composition Example: Slider Thumb

A slider thumb requires press (to detect activation), move (to track drag), focus (for keyboard), and long press (optional: to show a tooltip with the exact value). This demonstrates all interactions composed onto a single element.

```rust
pub fn thumb_attrs(&self, index: usize) -> AttrMap {
    let send = self.send.clone();

    // Press: handles activation start (activates keyboard mode).
    let press_config = PressConfig {
        on_press_start: Some(Rc::new(move |_| {
            (send)(Event::ThumbFocused(index));
        })),
        ..Default::default()
    };
    let press = use_press(press_config.clone());

    // Move: handles drag to change value.
    let send = self.send.clone();
    let move_ = use_move(MoveConfig {
        on_move: Some(Rc::new(move |e: MoveEvent| {
            (send)(Event::ThumbMoved {
                index,
                delta_x: e.delta_x,
                delta_y: e.delta_y,
                pointer_type: e.pointer_type,
            });
        })),
        ..Default::default()
    });

    // Focus: for keyboard navigation and focus ring.
    let focus = use_focus(FocusConfig::default());

    // Long press: show value tooltip after 800ms.
    // ids is passed for description_attrs() which generates the aria-describedby linkage.
    let long_press = use_long_press(LongPressConfig {
        threshold: Duration::from_millis(800),
        accessibility_description: Some(self.ctx.messages.long_press_description.clone()),
        ..Default::default()
    }, &self.ctx.ids);

    // Component-specific attrs.
    let mut thumb_attrs = AttrMap::new();
    thumb_attrs.set(HtmlAttr::Data("ars-scope"), "slider");
    thumb_attrs.set(HtmlAttr::Data("ars-part"), "thumb");
    thumb_attrs.set(HtmlAttr::Role, "slider");
    thumb_attrs.set(HtmlAttr::TabIndex, "0");
    thumb_attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), self.value(index).to_string());
    thumb_attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), self.min().to_string());
    thumb_attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), self.max().to_string());
    thumb_attrs.set(
        HtmlAttr::Aria(AriaAttr::Orientation),
        if self.is_vertical() { "vertical" } else { "horizontal" },
    );

    // Style: position the thumb.
    thumb_attrs.set_style(CssProperty::Position, "absolute");
    // Use `inset-inline-start` instead of `left` so the thumb position is
    // correct in both LTR and RTL layouts. If `CssProperty::InsetInlineStart`
    // is not available as an enum variant, the adapter MUST use the CSS
    // property name `inset-inline-start` directly.
    thumb_attrs.set_style(CssProperty::InsetInlineStart, format!("{}%", self.percent(index)));
    // Prevent browser touch gestures from intercepting drag on the thumb.
    thumb_attrs.set(HtmlAttr::Class, "ars-touch-none");

    // Merge data attributes from all interactions + component attrs.
    // Event handlers are composed separately via typed Api methods.
    merge_attrs!(
        press.current_attrs(&press_config),
        move_.current_attrs(),
        focus.current_attrs(),
        long_press.current_attrs(),
        thumb_attrs,
    )
}
```

### 8.7 Cross-Interaction Cancellation Protocol

When multiple interactions are composed on the same element, some interactions must cancel others to prevent conflicting activations. The canonical example is **Press + LongPress**: when LongPress fires, the subsequent `pointerup` should NOT trigger `on_press`.

**Cancellation contract:**

1. **Shared cancellation state:** When Press and LongPress are both connected to the same element, they share a `long_press_fired: SharedState<Option<PointerType>>` value. This state is created by the composition layer and passed to both interaction configs.

2. **LongPress records the firing modality:** When the long-press timer fires (transition from `Timing` to `LongPressed`), LongPress sets `long_press_fired = Some(pointer_type)`.

3. **Press checks the state on release:** In the release handler, before firing `on_press`, Press checks `long_press_fired`. If it matches the releasing modality, Press consumes the stored value, transitions to `Idle`, and suppresses `on_press` (but still calls `on_press_end`).

4. **State reset:** The shared state is reset to `None` when a new gesture begins from a fully idle press state, and it is also cleared when the matching release consumes the suppression. This prevents stale suppression from leaking across gestures while preserving the originating modality.

```rust
// Shared cancellation state between Press and LongPress
let long_press_fired = SharedState::new(None);

// Pass to LongPress — records the firing modality when the threshold fires
// and resets to None on the next pointerdown / keydown.
let long_press = use_long_press(LongPressConfig {
    long_press_cancel_flag: Some(long_press_fired.clone()),
    ..Default::default()
}, &ids);

// Pass to Press — checks the same shared state on release
let press = use_press(PressConfig {
    long_press_cancel_flag: Some(long_press_fired),
    // When this state contains the releasing modality,
    // on_press is suppressed (only on_press_end fires).
    ..Default::default()
});
```

**Alternative approach — synthetic PointerCancel:** Instead of a shared flag, LongPress can send a synthetic `PointerCancel` event to the Press state machine when the long-press threshold fires. This transitions Press from `PressedInside` to `Idle` without activation. This approach is cleaner when interactions communicate through the event system rather than shared state, but requires the composition layer to wire up the event routing.

**General cancellation rules for other interaction pairs:**

| Interaction A | Interaction B | Cancellation Rule                                                              |
| ------------- | ------------- | ------------------------------------------------------------------------------ |
| Press         | LongPress     | LongPress fires → Press suppresses `on_press` on release                       |
| Press         | Move          | Move exceeds threshold → Press cancels (existing behavior via `pointercancel`) |
| LongPress     | Move          | Move exceeds threshold → LongPress cancels (`on_long_press_cancel`)            |
| Drag          | Press         | Drag threshold exceeded → Press cancels                                        |

---

## 9. Moved Sections

The following sections have been relocated to `11-dom-utilities.md`:

- **Positioning Engine** — `11-dom-utilities.md` §2
- **Z-Index Management** — `11-dom-utilities.md` §6
- **Scroll Locking** — `11-dom-utilities.md` §5

---

_Cross-references:_

- _`01-architecture.md` §3.2 — `AttrMap` type definition_
- _`01-architecture.md` §2.1 — `Machine` trait (interaction state machines are standalone enums, not `Machine` implementors, but follow its naming conventions)_
- _`11-dom-utilities.md` §3.2 — `focus_element` utility from `ars-dom`_
- _`02-component-catalog.md` §2 — Button, Slider components that use these interactions_
- _`ars-a11y` crate — `LiveAnnouncer`, `VisuallyHidden`, `AnnouncementPriority` used by DnD and long press_
- _`11-dom-utilities.md` — `compute_position`, `next_z_index`, `ScrollLock`, `scrollbar_width` utilities_

## 10. Forced Colors — Interaction-Specific Styling

> **General rules:** `03-accessibility.md` §6.1 defines the normative forced-colors rules (system color keywords, SVG fill, no color-only state indicators). This section covers **interaction-specific** forced-colors styling only.

### 10.1 Overview

Interactions that apply visual feedback — focus rings, press states, drag previews, hover highlights — must remain visible when `forced-colors: active`. The general system color keyword mapping and detection APIs are in `03-accessibility.md` §6.1.

### 10.2 Interaction State System Colors

When `@media (forced-colors: active)` matches, interaction-driven visual indicators MUST use these CSS system colors:

| Indicator Type        | System Color to Use | Notes                                           |
| --------------------- | ------------------- | ----------------------------------------------- |
| Focus rings           | `Highlight`         | Replaces custom `box-shadow` or `outline-color` |
| Pressed/active states | `ButtonText`        | Borders or outlines indicating active press     |
| Drag preview          | `Highlight`         | Outline on dragged element                      |
| Selected items        | `Highlight`         | Outline + background for selection              |
| Disabled elements     | `GrayText`          | System-standard disabled appearance             |

### 10.3 Implementation Guidance

The companion stylesheet (`ars-interactions.css`) MUST include a forced-colors block:

```css
@media (forced-colors: active) {
    /* Focus indicators: use transparent outline that becomes visible in forced-colors */
    [data-ars-focus-visible] {
        outline: 3px solid Highlight;
        outline-offset: 2px;
    }

    /* Pressed state: use ButtonText border */
    [data-ars-pressed] {
        outline: 3px solid ButtonText;
    }

    /* Combined focus + pressed: focus ring takes precedence */
    [data-ars-focus-visible][data-ars-pressed] {
        outline: 3px solid Highlight;
        outline-offset: 2px;
    }

    /* Disabled elements */
    [data-ars-disabled] {
        color: GrayText;
        border-color: GrayText;
    }

    /* Drag preview */
    [data-ars-dragging] {
        outline: 2px solid Highlight;
    }

    /* Selected items */
    [data-ars-state~="selected"] {
        outline: 2px solid Highlight;
        color: HighlightText;
        background-color: Highlight;
    }
}
```

> **Transparent outline technique.** For non-forced-colors mode, use `outline: 2px solid transparent` on interactive elements. In forced-colors mode, the browser replaces `transparent` with a system color, making the outline visible without any additional `@media` rule. This is a progressive enhancement — it works even if the forced-colors block is missing, though explicit system color assignment is preferred for clarity.

### 10.4 Detection

```rust
// Canonical implementation lives in ars-dom (re-exported by ars-a11y behind #[cfg(feature = "dom")]).
// See 03-accessibility.md §6.1.
#[cfg(feature = "dom")]
pub use ars_dom::media::is_forced_colors_active;
```

Components that apply inline styles for visual feedback (e.g., drag preview opacity, hover background) SHOULD check `is_forced_colors_active()` and skip custom color overrides when forced colors are active, allowing the system colors to take effect.

> **`matchMedia()` Caching:** Cache `matchMedia()` results in a module-level variable; listen to the `change` event on the `MediaQueryList` for runtime updates. See `03-accessibility.md` §6.1 for the full caching pattern. Do not call `window.matchMedia()` on every render or event handler invocation.

### 10.5 Forced-Colors Testing Requirements

All interactive components MUST be tested under forced-colors mode. The test suite MUST verify:

1. **Focus indicators maintain ≥ 3:1 contrast ratio** against the adjacent background in forced-colors mode. Since the browser assigns system colors, verify that `outline` (not `box-shadow`) is used for focus rings.
2. **Data attributes do not hide content** — elements toggled via `data-ars-state`, `data-ars-disabled`, etc. must remain visible. Verify that `display: none` or `visibility: hidden` is not conditional on color-based selectors.
3. **Windows High Contrast Mode (WHCM) testing** — run integration tests in Edge/Chrome with Windows High Contrast themes (both "High Contrast Black" and "High Contrast White"). Verify all interactive states (hover, pressed, selected, disabled, focus-visible) are distinguishable.
4. **No information conveyed by color alone** — state changes indicated by color (e.g., error = red) must also have a non-color indicator (icon, border, text) visible in forced-colors mode.

```rust
// Example test assertion (conceptual)
#[cfg(test)]
fn test_forced_colors_focus_ring() {
    enable_forced_colors_emulation(); // Via CDP or test flag
    let button = render_component::<Button>();
    button.focus();
    let outline = button.computed_style("outline");
    assert!(outline.contains("solid"), "Focus ring must use outline in forced-colors mode");
}
```

### 10.6 Component Adaptation

When forced colors are active, components MUST adapt their rendering:

1. **Remove decorative backgrounds/gradients** — they become invisible or clash with system colors.
2. **Replace `box-shadow` focus indicators with `outline`** — `box-shadow` is suppressed in forced-colors mode; `outline` is preserved and recolored.
3. **Ensure borders are present on interactive elements** — in forced-colors mode, background color distinctions disappear; borders provide the only visual separation.
4. **Use `currentColor` for SVG icons** — ensures icons inherit the forced system text color.

### 10.7 `prefers-contrast` Integration

In addition to `forced-colors`, the `prefers-contrast` media query detects user preference for increased or decreased contrast:

```css
/* High contrast preference (not necessarily forced-colors) */
@media (prefers-contrast: more) {
    :root {
        --ars-focus-ring-width: 3px;
        --ars-border-width: 2px;
        --ars-focus-ring-offset: 3px;
    }
}

/* Custom contrast preference (forced-colors active) */
@media (prefers-contrast: custom) {
    /* Same as forced-colors — system colors in use */
}
```

```rust
/// Check whether the user prefers increased contrast.
/// Returns true when `@media (prefers-contrast: more)` matches.
pub fn prefers_high_contrast() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-contrast: more)").ok().flatten())
        .map(|mql| mql.matches())
        // Design decision: returns false on SSR/Web Worker where window() is unavailable.
        // This is intentional — high-contrast preference is a browser-only concept.
        .unwrap_or(false)
}
```

Components SHOULD use CSS custom properties (`--ars-focus-ring-width`, `--ars-border-width`) so that `prefers-contrast: more` can widen focus indicators and borders without JavaScript.

---

## 11. Keyboard Interaction

> Cross-references: Equivalent to React Aria `useKeyboard`.

### 11.1 Purpose

While existing interactions handle keyboard input implicitly (Press handles `Enter`/`Space`, Move handles arrows), there is no standalone primitive for components that need custom key handling. The `Keyboard` interaction provides normalized `onKeyDown` / `onKeyUp` callbacks without built-in behavior assumptions.

### 11.2 Configuration

```rust
// ars-interactions/src/keyboard.rs
// Re-export from ars-core for convenience.
pub use ars_core::KeyboardKey;

// NOTE: The canonical definition of `KeyboardKey` lives in `ars-core` so that
// both `ars-a11y` (no_std) and `ars-interactions` (std) can reference it
// without circular dependencies. See 01-architecture.md §1.2.
// The full enum definition below is the canonical definition from `ars-core`.
// This file reproduces it for specification reference; the single source of
// truth is `ars-core/src/keyboard.rs`.

/// Keyboard key value per W3C UI Events KeyboardEvent key Values (2025 Recommendation).
///
/// Named `KeyboardKey` to avoid collision with `ars_collections::Key`.
/// Every W3C named key is a data-less variant — the enum derives `Copy`.
/// Printable characters are not represented here; they live in
/// `KeyboardEventData.character: Option<char>`.
///
/// Reference: https://www.w3.org/TR/uievents-key/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyboardKey {
    // ── Special ──────────────────────────────────────────────
    /// A key whose value is not known or not representable.
    Unidentified,

    // ── Modifier keys ────────────────────────────────────────
    Alt,
    AltGraph,
    CapsLock,
    Control,
    Fn,
    FnLock,
    Meta,
    NumLock,
    ScrollLock,
    Shift,
    Symbol,
    SymbolLock,
    Hyper,
    Super,

    // ── Whitespace keys ──────────────────────────────────────
    Enter,
    Tab,
    /// The Space key. W3C key value is `" "` (a literal space character),
    /// but we model it as a named variant for ergonomic matching.
    /// Adapters normalize both `" "` and framework `Key::Space` to this variant.
    Space,

    // ── Navigation keys ──────────────────────────────────────
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    End,
    Home,
    PageDown,
    PageUp,

    // ── Editing keys ─────────────────────────────────────────
    Backspace,
    Clear,
    Copy,
    CrSel,
    Cut,
    Delete,
    EraseEof,
    ExSel,
    Insert,
    Paste,
    Redo,
    Undo,

    // ── UI keys ──────────────────────────────────────────────
    Accept,
    Again,
    Attn,
    Cancel,
    ContextMenu,
    Escape,
    Execute,
    Find,
    Help,
    Pause,
    Play,
    Props,
    Select,
    ZoomIn,
    ZoomOut,

    // ── Device keys ──────────────────────────────────────────
    BrightnessDown,
    BrightnessUp,
    Eject,
    LogOff,
    Power,
    PowerOff,
    PrintScreen,
    Hibernate,
    Standby,
    WakeUp,

    // ── IME & Composition keys ───────────────────────────────
    AllCandidates,
    Alphanumeric,
    CodeInput,
    Compose,
    Convert,
    Dead,
    FinalMode,
    GroupFirst,
    GroupLast,
    GroupNext,
    GroupPrevious,
    ModeChange,
    NextCandidate,
    NonConvert,
    PreviousCandidate,
    Process,
    SingleCandidate,
    HangulMode,
    HanjaMode,
    JunjaMode,
    Eisu,
    Hankaku,
    Hiragana,
    HiraganaKatakana,
    KanaMode,
    KanjiMode,
    Katakana,
    Romaji,
    Zenkaku,
    ZenkakuHankaku,

    // ── Function keys ────────────────────────────────────────
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Soft1, Soft2, Soft3, Soft4,

    // ── Multimedia keys ──────────────────────────────────────
    ChannelDown,
    ChannelUp,
    Close,
    MailForward,
    MailReply,
    MailSend,
    MediaClose,
    MediaFastForward,
    MediaPause,
    MediaPlay,
    MediaPlayPause,
    MediaRecord,
    MediaRewind,
    MediaStop,
    MediaTrackNext,
    MediaTrackPrevious,
    New,
    Open,
    Print,
    Save,
    SpellCheck,

    // ── Multimedia numpad keys ───────────────────────────────
    Key11,
    Key12,

    // ── Audio keys ───────────────────────────────────────────
    AudioBalanceLeft,
    AudioBalanceRight,
    AudioBassBoostDown,
    AudioBassBoostToggle,
    AudioBassBoostUp,
    AudioFaderFront,
    AudioFaderRear,
    AudioSurroundModeNext,
    AudioTrebleDown,
    AudioTrebleUp,
    AudioVolumeDown,
    AudioVolumeUp,
    AudioVolumeMute,
    MicrophoneToggle,
    MicrophoneVolumeDown,
    MicrophoneVolumeUp,
    MicrophoneVolumeMute,

    // ── Speech keys ──────────────────────────────────────────
    SpeechCorrectionList,
    SpeechInputToggle,

    // ── Application launch keys ──────────────────────────────
    LaunchApplication1,
    LaunchApplication2,
    LaunchCalendar,
    LaunchContacts,
    LaunchMail,
    LaunchMediaPlayer,
    LaunchMusicPlayer,
    LaunchPhone,
    LaunchScreenSaver,
    LaunchSpreadsheet,
    LaunchWebBrowser,
    LaunchWebCam,
    LaunchWordProcessor,

    // ── Browser keys ─────────────────────────────────────────
    BrowserBack,
    BrowserFavorites,
    BrowserForward,
    BrowserHome,
    BrowserRefresh,
    BrowserSearch,
    BrowserStop,

    // ── Mobile phone keys ────────────────────────────────────
    AppSwitch,
    Call,
    Camera,
    CameraFocus,
    EndCall,
    GoBack,
    GoHome,
    HeadsetHook,
    LastNumberRedial,
    Notification,
    MannerMode,
    VoiceDial,

    // ── TV keys ──────────────────────────────────────────────
    Tv,
    Tv3DMode,
    TvAntennaCable,
    TvAudioDescription,
    TvAudioDescriptionMixDown,
    TvAudioDescriptionMixUp,
    TvContentsMenu,
    TvDataService,
    TvInput,
    TvInputComponent1,
    TvInputComponent2,
    TvInputComposite1,
    TvInputComposite2,
    TvInputHdmi1,
    TvInputHdmi2,
    TvInputHdmi3,
    TvInputHdmi4,
    TvInputVga1,
    TvMediaContext,
    TvNetwork,
    TvNumberEntry,
    TvPower,
    TvRadioService,
    TvSatellite,
    TvSatelliteBS,
    TvSatelliteCS,
    TvSatelliteToggle,
    TvTerrestrialAnalog,
    TvTerrestrialDigital,
    TvTimer,

    // ── Media controller keys ────────────────────────────────
    AvrInput,
    AvrPower,
    ColorF0Red,
    ColorF1Green,
    ColorF2Yellow,
    ColorF3Blue,
    ColorF4Grey,
    ColorF5Brown,
    ClosedCaptionToggle,
    Dimmer,
    DisplaySwap,
    Dvr,
    Exit,
    FavoriteClear0,
    FavoriteClear1,
    FavoriteClear2,
    FavoriteClear3,
    FavoriteRecall0,
    FavoriteRecall1,
    FavoriteRecall2,
    FavoriteRecall3,
    FavoriteStore0,
    FavoriteStore1,
    FavoriteStore2,
    FavoriteStore3,
    Guide,
    GuideNextDay,
    GuidePreviousDay,
    Info,
    InstantReplay,
    Link,
    ListProgram,
    LiveContent,
    Lock,
    MediaApps,
    MediaAudioTrack,
    MediaLast,
    MediaSkipBackward,
    MediaSkipForward,
    MediaStepBackward,
    MediaStepForward,
    MediaTopMenu,
    NavigateIn,
    NavigateNext,
    NavigateOut,
    NavigatePrevious,
    NextFavoriteChannel,
    NextUserProfile,
    OnDemand,
    Pairing,
    PinPDown,
    PinPMove,
    PinPToggle,
    PinPUp,
    PlaySpeedDown,
    PlaySpeedReset,
    PlaySpeedUp,
    RandomToggle,
    RcLowBattery,
    RecordSpeedNext,
    RfBypass,
    ScanChannelsToggle,
    ScreenModeNext,
    Settings,
    SplitScreenToggle,
    StbInput,
    StbPower,
    Subtitle,
    Teletext,
    VideoModeNext,
    Wink,
    ZoomToggle,
}

impl KeyboardKey {
    /// Parse a W3C key string into a `KeyboardKey`.
    /// Returns `Unidentified` for unrecognized strings (including single
    /// printable characters — those are handled via `KeyboardEventData.character`).
    pub fn from_key_str(s: &str) -> Self {
        match s {
            "Unidentified" => Self::Unidentified,
            // Modifier keys
            "Alt" => Self::Alt,
            "AltGraph" => Self::AltGraph,
            "CapsLock" => Self::CapsLock,
            "Control" => Self::Control,
            "Fn" => Self::Fn,
            "FnLock" => Self::FnLock,
            "Meta" => Self::Meta,
            "NumLock" => Self::NumLock,
            "ScrollLock" => Self::ScrollLock,
            "Shift" => Self::Shift,
            "Symbol" => Self::Symbol,
            "SymbolLock" => Self::SymbolLock,
            "Hyper" => Self::Hyper,
            "Super" => Self::Super,
            // Whitespace keys
            "Enter" => Self::Enter,
            "Tab" => Self::Tab,
            " " | "Space" => Self::Space,
            // Navigation keys
            "ArrowDown" => Self::ArrowDown,
            "ArrowLeft" => Self::ArrowLeft,
            "ArrowRight" => Self::ArrowRight,
            "ArrowUp" => Self::ArrowUp,
            "End" => Self::End,
            "Home" => Self::Home,
            "PageDown" => Self::PageDown,
            "PageUp" => Self::PageUp,
            // Editing keys
            "Backspace" => Self::Backspace,
            "Clear" => Self::Clear,
            "Copy" => Self::Copy,
            "CrSel" => Self::CrSel,
            "Cut" => Self::Cut,
            "Delete" => Self::Delete,
            "EraseEof" => Self::EraseEof,
            "ExSel" => Self::ExSel,
            "Insert" => Self::Insert,
            "Paste" => Self::Paste,
            "Redo" => Self::Redo,
            "Undo" => Self::Undo,
            // UI keys
            "Accept" => Self::Accept,
            "Again" => Self::Again,
            "Attn" => Self::Attn,
            "Cancel" => Self::Cancel,
            "ContextMenu" => Self::ContextMenu,
            "Escape" => Self::Escape,
            "Execute" => Self::Execute,
            "Find" => Self::Find,
            "Help" => Self::Help,
            "Pause" => Self::Pause,
            "Play" => Self::Play,
            "Props" => Self::Props,
            "Select" => Self::Select,
            "ZoomIn" => Self::ZoomIn,
            "ZoomOut" => Self::ZoomOut,
            // Device keys
            "BrightnessDown" => Self::BrightnessDown,
            "BrightnessUp" => Self::BrightnessUp,
            "Eject" => Self::Eject,
            "LogOff" => Self::LogOff,
            "Power" => Self::Power,
            "PowerOff" => Self::PowerOff,
            "PrintScreen" => Self::PrintScreen,
            "Hibernate" => Self::Hibernate,
            "Standby" => Self::Standby,
            "WakeUp" => Self::WakeUp,
            // IME & Composition keys
            "AllCandidates" => Self::AllCandidates,
            "Alphanumeric" => Self::Alphanumeric,
            "CodeInput" => Self::CodeInput,
            "Compose" => Self::Compose,
            "Convert" => Self::Convert,
            "Dead" => Self::Dead,
            "FinalMode" => Self::FinalMode,
            "GroupFirst" => Self::GroupFirst,
            "GroupLast" => Self::GroupLast,
            "GroupNext" => Self::GroupNext,
            "GroupPrevious" => Self::GroupPrevious,
            "ModeChange" => Self::ModeChange,
            "NextCandidate" => Self::NextCandidate,
            "NonConvert" => Self::NonConvert,
            "PreviousCandidate" => Self::PreviousCandidate,
            "Process" => Self::Process,
            "SingleCandidate" => Self::SingleCandidate,
            "HangulMode" => Self::HangulMode,
            "HanjaMode" => Self::HanjaMode,
            "JunjaMode" => Self::JunjaMode,
            "Eisu" => Self::Eisu,
            "Hankaku" => Self::Hankaku,
            "Hiragana" => Self::Hiragana,
            "HiraganaKatakana" => Self::HiraganaKatakana,
            "KanaMode" => Self::KanaMode,
            "KanjiMode" => Self::KanjiMode,
            "Katakana" => Self::Katakana,
            "Romaji" => Self::Romaji,
            "Zenkaku" => Self::Zenkaku,
            "ZenkakuHankaku" => Self::ZenkakuHankaku,
            // Function keys
            "F1" => Self::F1, "F2" => Self::F2, "F3" => Self::F3,
            "F4" => Self::F4, "F5" => Self::F5, "F6" => Self::F6,
            "F7" => Self::F7, "F8" => Self::F8, "F9" => Self::F9,
            "F10" => Self::F10, "F11" => Self::F11, "F12" => Self::F12,
            "Soft1" => Self::Soft1, "Soft2" => Self::Soft2,
            "Soft3" => Self::Soft3, "Soft4" => Self::Soft4,
            // Multimedia keys
            "ChannelDown" => Self::ChannelDown,
            "ChannelUp" => Self::ChannelUp,
            "Close" => Self::Close,
            "MailForward" => Self::MailForward,
            "MailReply" => Self::MailReply,
            "MailSend" => Self::MailSend,
            "MediaClose" => Self::MediaClose,
            "MediaFastForward" => Self::MediaFastForward,
            "MediaPause" => Self::MediaPause,
            "MediaPlay" => Self::MediaPlay,
            "MediaPlayPause" => Self::MediaPlayPause,
            "MediaRecord" => Self::MediaRecord,
            "MediaRewind" => Self::MediaRewind,
            "MediaStop" => Self::MediaStop,
            "MediaTrackNext" => Self::MediaTrackNext,
            "MediaTrackPrevious" => Self::MediaTrackPrevious,
            "New" => Self::New,
            "Open" => Self::Open,
            "Print" => Self::Print,
            "Save" => Self::Save,
            "SpellCheck" => Self::SpellCheck,
            // Multimedia numpad keys
            "11" => Self::Key11,
            "12" => Self::Key12,
            // Audio keys
            "AudioBalanceLeft" => Self::AudioBalanceLeft,
            "AudioBalanceRight" => Self::AudioBalanceRight,
            "AudioBassBoostDown" => Self::AudioBassBoostDown,
            "AudioBassBoostToggle" => Self::AudioBassBoostToggle,
            "AudioBassBoostUp" => Self::AudioBassBoostUp,
            "AudioFaderFront" => Self::AudioFaderFront,
            "AudioFaderRear" => Self::AudioFaderRear,
            "AudioSurroundModeNext" => Self::AudioSurroundModeNext,
            "AudioTrebleDown" => Self::AudioTrebleDown,
            "AudioTrebleUp" => Self::AudioTrebleUp,
            "AudioVolumeDown" => Self::AudioVolumeDown,
            "AudioVolumeUp" => Self::AudioVolumeUp,
            "AudioVolumeMute" => Self::AudioVolumeMute,
            "MicrophoneToggle" => Self::MicrophoneToggle,
            "MicrophoneVolumeDown" => Self::MicrophoneVolumeDown,
            "MicrophoneVolumeUp" => Self::MicrophoneVolumeUp,
            "MicrophoneVolumeMute" => Self::MicrophoneVolumeMute,
            // Speech keys
            "SpeechCorrectionList" => Self::SpeechCorrectionList,
            "SpeechInputToggle" => Self::SpeechInputToggle,
            // Application launch keys
            "LaunchApplication1" => Self::LaunchApplication1,
            "LaunchApplication2" => Self::LaunchApplication2,
            "LaunchCalendar" => Self::LaunchCalendar,
            "LaunchContacts" => Self::LaunchContacts,
            "LaunchMail" => Self::LaunchMail,
            "LaunchMediaPlayer" => Self::LaunchMediaPlayer,
            "LaunchMusicPlayer" => Self::LaunchMusicPlayer,
            "LaunchPhone" => Self::LaunchPhone,
            "LaunchScreenSaver" => Self::LaunchScreenSaver,
            "LaunchSpreadsheet" => Self::LaunchSpreadsheet,
            "LaunchWebBrowser" => Self::LaunchWebBrowser,
            "LaunchWebCam" => Self::LaunchWebCam,
            "LaunchWordProcessor" => Self::LaunchWordProcessor,
            // Browser keys
            "BrowserBack" => Self::BrowserBack,
            "BrowserFavorites" => Self::BrowserFavorites,
            "BrowserForward" => Self::BrowserForward,
            "BrowserHome" => Self::BrowserHome,
            "BrowserRefresh" => Self::BrowserRefresh,
            "BrowserSearch" => Self::BrowserSearch,
            "BrowserStop" => Self::BrowserStop,
            // Mobile phone keys
            "AppSwitch" => Self::AppSwitch,
            "Call" => Self::Call,
            "Camera" => Self::Camera,
            "CameraFocus" => Self::CameraFocus,
            "EndCall" => Self::EndCall,
            "GoBack" => Self::GoBack,
            "GoHome" => Self::GoHome,
            "HeadsetHook" => Self::HeadsetHook,
            "LastNumberRedial" => Self::LastNumberRedial,
            "Notification" => Self::Notification,
            "MannerMode" => Self::MannerMode,
            "VoiceDial" => Self::VoiceDial,
            // TV keys
            "TV" => Self::Tv,
            "TV3DMode" => Self::Tv3DMode,
            "TVAntennaCable" => Self::TvAntennaCable,
            "TVAudioDescription" => Self::TvAudioDescription,
            "TVAudioDescriptionMixDown" => Self::TvAudioDescriptionMixDown,
            "TVAudioDescriptionMixUp" => Self::TvAudioDescriptionMixUp,
            "TVContentsMenu" => Self::TvContentsMenu,
            "TVDataService" => Self::TvDataService,
            "TVInput" => Self::TvInput,
            "TVInputComponent1" => Self::TvInputComponent1,
            "TVInputComponent2" => Self::TvInputComponent2,
            "TVInputComposite1" => Self::TvInputComposite1,
            "TVInputComposite2" => Self::TvInputComposite2,
            "TVInputHDMI1" => Self::TvInputHdmi1,
            "TVInputHDMI2" => Self::TvInputHdmi2,
            "TVInputHDMI3" => Self::TvInputHdmi3,
            "TVInputHDMI4" => Self::TvInputHdmi4,
            "TVInputVGA1" => Self::TvInputVga1,
            "TVMediaContext" => Self::TvMediaContext,
            "TVNetwork" => Self::TvNetwork,
            "TVNumberEntry" => Self::TvNumberEntry,
            "TVPower" => Self::TvPower,
            "TVRadioService" => Self::TvRadioService,
            "TVSatellite" => Self::TvSatellite,
            "TVSatelliteBS" => Self::TvSatelliteBS,
            "TVSatelliteCS" => Self::TvSatelliteCS,
            "TVSatelliteToggle" => Self::TvSatelliteToggle,
            "TVTerrestrialAnalog" => Self::TvTerrestrialAnalog,
            "TVTerrestrialDigital" => Self::TvTerrestrialDigital,
            "TVTimer" => Self::TvTimer,
            // Media controller keys
            "AVRInput" => Self::AvrInput,
            "AVRPower" => Self::AvrPower,
            "ColorF0Red" => Self::ColorF0Red,
            "ColorF1Green" => Self::ColorF1Green,
            "ColorF2Yellow" => Self::ColorF2Yellow,
            "ColorF3Blue" => Self::ColorF3Blue,
            "ColorF4Grey" => Self::ColorF4Grey,
            "ColorF5Brown" => Self::ColorF5Brown,
            "ClosedCaptionToggle" => Self::ClosedCaptionToggle,
            "Dimmer" => Self::Dimmer,
            "DisplaySwap" => Self::DisplaySwap,
            "DVR" => Self::Dvr,
            "Exit" => Self::Exit,
            "FavoriteClear0" => Self::FavoriteClear0,
            "FavoriteClear1" => Self::FavoriteClear1,
            "FavoriteClear2" => Self::FavoriteClear2,
            "FavoriteClear3" => Self::FavoriteClear3,
            "FavoriteRecall0" => Self::FavoriteRecall0,
            "FavoriteRecall1" => Self::FavoriteRecall1,
            "FavoriteRecall2" => Self::FavoriteRecall2,
            "FavoriteRecall3" => Self::FavoriteRecall3,
            "FavoriteStore0" => Self::FavoriteStore0,
            "FavoriteStore1" => Self::FavoriteStore1,
            "FavoriteStore2" => Self::FavoriteStore2,
            "FavoriteStore3" => Self::FavoriteStore3,
            "Guide" => Self::Guide,
            "GuideNextDay" => Self::GuideNextDay,
            "GuidePreviousDay" => Self::GuidePreviousDay,
            "Info" => Self::Info,
            "InstantReplay" => Self::InstantReplay,
            "Link" => Self::Link,
            "ListProgram" => Self::ListProgram,
            "LiveContent" => Self::LiveContent,
            "Lock" => Self::Lock,
            "MediaApps" => Self::MediaApps,
            "MediaAudioTrack" => Self::MediaAudioTrack,
            "MediaLast" => Self::MediaLast,
            "MediaSkipBackward" => Self::MediaSkipBackward,
            "MediaSkipForward" => Self::MediaSkipForward,
            "MediaStepBackward" => Self::MediaStepBackward,
            "MediaStepForward" => Self::MediaStepForward,
            "MediaTopMenu" => Self::MediaTopMenu,
            "NavigateIn" => Self::NavigateIn,
            "NavigateNext" => Self::NavigateNext,
            "NavigateOut" => Self::NavigateOut,
            "NavigatePrevious" => Self::NavigatePrevious,
            "NextFavoriteChannel" => Self::NextFavoriteChannel,
            "NextUserProfile" => Self::NextUserProfile,
            "OnDemand" => Self::OnDemand,
            "Pairing" => Self::Pairing,
            "PinPDown" => Self::PinPDown,
            "PinPMove" => Self::PinPMove,
            "PinPToggle" => Self::PinPToggle,
            "PinPUp" => Self::PinPUp,
            "PlaySpeedDown" => Self::PlaySpeedDown,
            "PlaySpeedReset" => Self::PlaySpeedReset,
            "PlaySpeedUp" => Self::PlaySpeedUp,
            "RandomToggle" => Self::RandomToggle,
            "RcLowBattery" => Self::RcLowBattery,
            "RecordSpeedNext" => Self::RecordSpeedNext,
            "RfBypass" => Self::RfBypass,
            "ScanChannelsToggle" => Self::ScanChannelsToggle,
            "ScreenModeNext" => Self::ScreenModeNext,
            "Settings" => Self::Settings,
            "SplitScreenToggle" => Self::SplitScreenToggle,
            "STBInput" => Self::StbInput,
            "STBPower" => Self::StbPower,
            "Subtitle" => Self::Subtitle,
            "Teletext" => Self::Teletext,
            "VideoModeNext" => Self::VideoModeNext,
            "Wink" => Self::Wink,
            "ZoomToggle" => Self::ZoomToggle,
            _ => Self::Unidentified,
        }
    }

    /// Return the W3C key string for this variant.
    pub fn as_w3c_str(&self) -> &'static str {
        match self {
            Self::Unidentified => "Unidentified",
            // Modifier keys
            Self::Alt => "Alt",
            Self::AltGraph => "AltGraph",
            Self::CapsLock => "CapsLock",
            Self::Control => "Control",
            Self::Fn => "Fn",
            Self::FnLock => "FnLock",
            Self::Meta => "Meta",
            Self::NumLock => "NumLock",
            Self::ScrollLock => "ScrollLock",
            Self::Shift => "Shift",
            Self::Symbol => "Symbol",
            Self::SymbolLock => "SymbolLock",
            Self::Hyper => "Hyper",
            Self::Super => "Super",
            // Whitespace keys
            Self::Enter => "Enter",
            Self::Tab => "Tab",
            Self::Space => " ",
            // Navigation keys
            Self::ArrowDown => "ArrowDown",
            Self::ArrowLeft => "ArrowLeft",
            Self::ArrowRight => "ArrowRight",
            Self::ArrowUp => "ArrowUp",
            Self::End => "End",
            Self::Home => "Home",
            Self::PageDown => "PageDown",
            Self::PageUp => "PageUp",
            // Editing keys
            Self::Backspace => "Backspace",
            Self::Clear => "Clear",
            Self::Copy => "Copy",
            Self::CrSel => "CrSel",
            Self::Cut => "Cut",
            Self::Delete => "Delete",
            Self::EraseEof => "EraseEof",
            Self::ExSel => "ExSel",
            Self::Insert => "Insert",
            Self::Paste => "Paste",
            Self::Redo => "Redo",
            Self::Undo => "Undo",
            // UI keys
            Self::Accept => "Accept",
            Self::Again => "Again",
            Self::Attn => "Attn",
            Self::Cancel => "Cancel",
            Self::ContextMenu => "ContextMenu",
            Self::Escape => "Escape",
            Self::Execute => "Execute",
            Self::Find => "Find",
            Self::Help => "Help",
            Self::Pause => "Pause",
            Self::Play => "Play",
            Self::Props => "Props",
            Self::Select => "Select",
            Self::ZoomIn => "ZoomIn",
            Self::ZoomOut => "ZoomOut",
            // Device keys
            Self::BrightnessDown => "BrightnessDown",
            Self::BrightnessUp => "BrightnessUp",
            Self::Eject => "Eject",
            Self::LogOff => "LogOff",
            Self::Power => "Power",
            Self::PowerOff => "PowerOff",
            Self::PrintScreen => "PrintScreen",
            Self::Hibernate => "Hibernate",
            Self::Standby => "Standby",
            Self::WakeUp => "WakeUp",
            // IME & Composition keys
            Self::AllCandidates => "AllCandidates",
            Self::Alphanumeric => "Alphanumeric",
            Self::CodeInput => "CodeInput",
            Self::Compose => "Compose",
            Self::Convert => "Convert",
            Self::Dead => "Dead",
            Self::FinalMode => "FinalMode",
            Self::GroupFirst => "GroupFirst",
            Self::GroupLast => "GroupLast",
            Self::GroupNext => "GroupNext",
            Self::GroupPrevious => "GroupPrevious",
            Self::ModeChange => "ModeChange",
            Self::NextCandidate => "NextCandidate",
            Self::NonConvert => "NonConvert",
            Self::PreviousCandidate => "PreviousCandidate",
            Self::Process => "Process",
            Self::SingleCandidate => "SingleCandidate",
            Self::HangulMode => "HangulMode",
            Self::HanjaMode => "HanjaMode",
            Self::JunjaMode => "JunjaMode",
            Self::Eisu => "Eisu",
            Self::Hankaku => "Hankaku",
            Self::Hiragana => "Hiragana",
            Self::HiraganaKatakana => "HiraganaKatakana",
            Self::KanaMode => "KanaMode",
            Self::KanjiMode => "KanjiMode",
            Self::Katakana => "Katakana",
            Self::Romaji => "Romaji",
            Self::Zenkaku => "Zenkaku",
            Self::ZenkakuHankaku => "ZenkakuHankaku",
            // Function keys
            Self::F1 => "F1", Self::F2 => "F2", Self::F3 => "F3",
            Self::F4 => "F4", Self::F5 => "F5", Self::F6 => "F6",
            Self::F7 => "F7", Self::F8 => "F8", Self::F9 => "F9",
            Self::F10 => "F10", Self::F11 => "F11", Self::F12 => "F12",
            Self::Soft1 => "Soft1", Self::Soft2 => "Soft2",
            Self::Soft3 => "Soft3", Self::Soft4 => "Soft4",
            // Multimedia keys
            Self::ChannelDown => "ChannelDown",
            Self::ChannelUp => "ChannelUp",
            Self::Close => "Close",
            Self::MailForward => "MailForward",
            Self::MailReply => "MailReply",
            Self::MailSend => "MailSend",
            Self::MediaClose => "MediaClose",
            Self::MediaFastForward => "MediaFastForward",
            Self::MediaPause => "MediaPause",
            Self::MediaPlay => "MediaPlay",
            Self::MediaPlayPause => "MediaPlayPause",
            Self::MediaRecord => "MediaRecord",
            Self::MediaRewind => "MediaRewind",
            Self::MediaStop => "MediaStop",
            Self::MediaTrackNext => "MediaTrackNext",
            Self::MediaTrackPrevious => "MediaTrackPrevious",
            Self::New => "New",
            Self::Open => "Open",
            Self::Print => "Print",
            Self::Save => "Save",
            Self::SpellCheck => "SpellCheck",
            // Multimedia numpad keys
            Self::Key11 => "11",
            Self::Key12 => "12",
            // Audio keys
            Self::AudioBalanceLeft => "AudioBalanceLeft",
            Self::AudioBalanceRight => "AudioBalanceRight",
            Self::AudioBassBoostDown => "AudioBassBoostDown",
            Self::AudioBassBoostToggle => "AudioBassBoostToggle",
            Self::AudioBassBoostUp => "AudioBassBoostUp",
            Self::AudioFaderFront => "AudioFaderFront",
            Self::AudioFaderRear => "AudioFaderRear",
            Self::AudioSurroundModeNext => "AudioSurroundModeNext",
            Self::AudioTrebleDown => "AudioTrebleDown",
            Self::AudioTrebleUp => "AudioTrebleUp",
            Self::AudioVolumeDown => "AudioVolumeDown",
            Self::AudioVolumeUp => "AudioVolumeUp",
            Self::AudioVolumeMute => "AudioVolumeMute",
            Self::MicrophoneToggle => "MicrophoneToggle",
            Self::MicrophoneVolumeDown => "MicrophoneVolumeDown",
            Self::MicrophoneVolumeUp => "MicrophoneVolumeUp",
            Self::MicrophoneVolumeMute => "MicrophoneVolumeMute",
            // Speech keys
            Self::SpeechCorrectionList => "SpeechCorrectionList",
            Self::SpeechInputToggle => "SpeechInputToggle",
            // Application launch keys
            Self::LaunchApplication1 => "LaunchApplication1",
            Self::LaunchApplication2 => "LaunchApplication2",
            Self::LaunchCalendar => "LaunchCalendar",
            Self::LaunchContacts => "LaunchContacts",
            Self::LaunchMail => "LaunchMail",
            Self::LaunchMediaPlayer => "LaunchMediaPlayer",
            Self::LaunchMusicPlayer => "LaunchMusicPlayer",
            Self::LaunchPhone => "LaunchPhone",
            Self::LaunchScreenSaver => "LaunchScreenSaver",
            Self::LaunchSpreadsheet => "LaunchSpreadsheet",
            Self::LaunchWebBrowser => "LaunchWebBrowser",
            Self::LaunchWebCam => "LaunchWebCam",
            Self::LaunchWordProcessor => "LaunchWordProcessor",
            // Browser keys
            Self::BrowserBack => "BrowserBack",
            Self::BrowserFavorites => "BrowserFavorites",
            Self::BrowserForward => "BrowserForward",
            Self::BrowserHome => "BrowserHome",
            Self::BrowserRefresh => "BrowserRefresh",
            Self::BrowserSearch => "BrowserSearch",
            Self::BrowserStop => "BrowserStop",
            // Mobile phone keys
            Self::AppSwitch => "AppSwitch",
            Self::Call => "Call",
            Self::Camera => "Camera",
            Self::CameraFocus => "CameraFocus",
            Self::EndCall => "EndCall",
            Self::GoBack => "GoBack",
            Self::GoHome => "GoHome",
            Self::HeadsetHook => "HeadsetHook",
            Self::LastNumberRedial => "LastNumberRedial",
            Self::Notification => "Notification",
            Self::MannerMode => "MannerMode",
            Self::VoiceDial => "VoiceDial",
            // TV keys
            Self::Tv => "TV",
            Self::Tv3DMode => "TV3DMode",
            Self::TvAntennaCable => "TVAntennaCable",
            Self::TvAudioDescription => "TVAudioDescription",
            Self::TvAudioDescriptionMixDown => "TVAudioDescriptionMixDown",
            Self::TvAudioDescriptionMixUp => "TVAudioDescriptionMixUp",
            Self::TvContentsMenu => "TVContentsMenu",
            Self::TvDataService => "TVDataService",
            Self::TvInput => "TVInput",
            Self::TvInputComponent1 => "TVInputComponent1",
            Self::TvInputComponent2 => "TVInputComponent2",
            Self::TvInputComposite1 => "TVInputComposite1",
            Self::TvInputComposite2 => "TVInputComposite2",
            Self::TvInputHdmi1 => "TVInputHDMI1",
            Self::TvInputHdmi2 => "TVInputHDMI2",
            Self::TvInputHdmi3 => "TVInputHDMI3",
            Self::TvInputHdmi4 => "TVInputHDMI4",
            Self::TvInputVga1 => "TVInputVGA1",
            Self::TvMediaContext => "TVMediaContext",
            Self::TvNetwork => "TVNetwork",
            Self::TvNumberEntry => "TVNumberEntry",
            Self::TvPower => "TVPower",
            Self::TvRadioService => "TVRadioService",
            Self::TvSatellite => "TVSatellite",
            Self::TvSatelliteBS => "TVSatelliteBS",
            Self::TvSatelliteCS => "TVSatelliteCS",
            Self::TvSatelliteToggle => "TVSatelliteToggle",
            Self::TvTerrestrialAnalog => "TVTerrestrialAnalog",
            Self::TvTerrestrialDigital => "TVTerrestrialDigital",
            Self::TvTimer => "TVTimer",
            // Media controller keys
            Self::AvrInput => "AVRInput",
            Self::AvrPower => "AVRPower",
            Self::ColorF0Red => "ColorF0Red",
            Self::ColorF1Green => "ColorF1Green",
            Self::ColorF2Yellow => "ColorF2Yellow",
            Self::ColorF3Blue => "ColorF3Blue",
            Self::ColorF4Grey => "ColorF4Grey",
            Self::ColorF5Brown => "ColorF5Brown",
            Self::ClosedCaptionToggle => "ClosedCaptionToggle",
            Self::Dimmer => "Dimmer",
            Self::DisplaySwap => "DisplaySwap",
            Self::Dvr => "DVR",
            Self::Exit => "Exit",
            Self::FavoriteClear0 => "FavoriteClear0",
            Self::FavoriteClear1 => "FavoriteClear1",
            Self::FavoriteClear2 => "FavoriteClear2",
            Self::FavoriteClear3 => "FavoriteClear3",
            Self::FavoriteRecall0 => "FavoriteRecall0",
            Self::FavoriteRecall1 => "FavoriteRecall1",
            Self::FavoriteRecall2 => "FavoriteRecall2",
            Self::FavoriteRecall3 => "FavoriteRecall3",
            Self::FavoriteStore0 => "FavoriteStore0",
            Self::FavoriteStore1 => "FavoriteStore1",
            Self::FavoriteStore2 => "FavoriteStore2",
            Self::FavoriteStore3 => "FavoriteStore3",
            Self::Guide => "Guide",
            Self::GuideNextDay => "GuideNextDay",
            Self::GuidePreviousDay => "GuidePreviousDay",
            Self::Info => "Info",
            Self::InstantReplay => "InstantReplay",
            Self::Link => "Link",
            Self::ListProgram => "ListProgram",
            Self::LiveContent => "LiveContent",
            Self::Lock => "Lock",
            Self::MediaApps => "MediaApps",
            Self::MediaAudioTrack => "MediaAudioTrack",
            Self::MediaLast => "MediaLast",
            Self::MediaSkipBackward => "MediaSkipBackward",
            Self::MediaSkipForward => "MediaSkipForward",
            Self::MediaStepBackward => "MediaStepBackward",
            Self::MediaStepForward => "MediaStepForward",
            Self::MediaTopMenu => "MediaTopMenu",
            Self::NavigateIn => "NavigateIn",
            Self::NavigateNext => "NavigateNext",
            Self::NavigateOut => "NavigateOut",
            Self::NavigatePrevious => "NavigatePrevious",
            Self::NextFavoriteChannel => "NextFavoriteChannel",
            Self::NextUserProfile => "NextUserProfile",
            Self::OnDemand => "OnDemand",
            Self::Pairing => "Pairing",
            Self::PinPDown => "PinPDown",
            Self::PinPMove => "PinPMove",
            Self::PinPToggle => "PinPToggle",
            Self::PinPUp => "PinPUp",
            Self::PlaySpeedDown => "PlaySpeedDown",
            Self::PlaySpeedReset => "PlaySpeedReset",
            Self::PlaySpeedUp => "PlaySpeedUp",
            Self::RandomToggle => "RandomToggle",
            Self::RcLowBattery => "RcLowBattery",
            Self::RecordSpeedNext => "RecordSpeedNext",
            Self::RfBypass => "RfBypass",
            Self::ScanChannelsToggle => "ScanChannelsToggle",
            Self::ScreenModeNext => "ScreenModeNext",
            Self::Settings => "Settings",
            Self::SplitScreenToggle => "SplitScreenToggle",
            Self::StbInput => "STBInput",
            Self::StbPower => "STBPower",
            Self::Subtitle => "Subtitle",
            Self::Teletext => "Teletext",
            Self::VideoModeNext => "VideoModeNext",
            Self::Wink => "Wink",
            Self::ZoomToggle => "ZoomToggle",
        }
    }
}

/// Configuration for standalone keyboard event handling.
#[derive(Clone, Debug, Default)]
pub struct KeyboardConfig {
    /// Whether the keyboard interaction is disabled.
    pub disabled: bool,
}

/// Normalized keyboard event data.
///
/// Named `KeyboardEventData` (not `KeyboardEvent`) to avoid collision with
/// `web_sys::KeyboardEvent` in adapter code.
#[derive(Clone, Debug)]
pub struct KeyboardEventData {
    /// The named key, parsed from the W3C key string.
    /// Printable character keys map to `KeyboardKey::Unidentified` — use
    /// `character` for the actual character value.
    pub key: KeyboardKey,

    /// The printable character produced by this key press, if any.
    /// - Pressing `A` → `character: Some('a')` (or `Some('A')` with shift)
    /// - Pressing `Enter` → `character: None`
    /// - Pressing `Space` → `character: Some(' ')`
    pub character: Option<char>,

    // Raw platform modifier state. Note: `ctrl_key` is always the physical Ctrl key
    // on all platforms. On macOS, the Cmd key is `meta_key`, not `ctrl_key`.
    // For a unified Ctrl/Cmd abstraction, use `ars-a11y::KeyModifiers::action`.

    /// The physical key code (e.g., "KeyA", "Digit1").
    /// Uses the `KeyboardEvent.code` standard values.
    pub code: String,

    /// Whether the Shift modifier was held.
    pub shift_key: bool,

    /// Whether the physical Ctrl modifier was held.
    /// Returns raw Ctrl key state. Use `KeyModifiers::action` for
    /// platform-abstracted shortcut logic (Ctrl on Windows/Linux, Cmd on macOS).
    pub ctrl_key: bool,

    /// Whether the Alt modifier was held (Option on macOS).
    pub alt_key: bool,

    /// Whether the Meta modifier was held (Cmd on macOS, Win on Windows).
    pub meta_key: bool,

    /// Whether this is a repeat event (key held down).
    pub repeat: bool,

    /// True when this event fires during active IME composition.
    /// Components MUST NOT act on character keys when this is true,
    /// as the user is in the middle of composing a CJK or other
    /// multi-keystroke character.
    pub is_composing: bool,
}

/// Events emitted by the Keyboard interaction.
/// Named `ArsKeyboardEvent` to avoid collision with `web_sys::KeyboardEvent`.
/// Adapters converting from DOM events should use the full path `web_sys::KeyboardEvent`
/// for the DOM type and this type for the framework-agnostic event.
#[derive(Clone, Debug)]
pub enum ArsKeyboardEvent {
    /// A key was pressed down.
    KeyDown(KeyboardEventData),

    /// A key was released.
    KeyUp(KeyboardEventData),
}
```

### 11.3 Adapter Wiring

The adapter attaches `keydown` and `keyup` listeners to the target element and normalizes the browser `KeyboardEvent` into `ArsKeyboardEvent` before forwarding.

### 11.4 Composition

`KeyboardConfig` can be composed with other interactions via the standard composition pattern (§8). When composed, keyboard events fire alongside Press/Focus/etc. events on the same element. The `Keyboard` interaction never calls `preventDefault()` on its own — that is the consumer's responsibility.

### 11.5 IME Composition Handling

> **Cross-reference:** `03-accessibility.md` §3.6 defines the component-level behavioral contract for IME composition (what to suppress during composition, Firefox late-fire workaround). This section defines the adapter-level event normalization that surfaces composition state to components.

During Input Method Editor (IME) composition (used for Chinese, Japanese, Korean, and other languages requiring multi-keystroke input), `keydown` fires with `key = KeyboardKey::Process` and `is_composing = true`. Components that respond to character input in real time (e.g., type-ahead search in Select/Combobox) **must** check `is_composing` and suppress character-keyed interactions during composition:

```rust
// In a Combobox filter handler:
KeyboardEvent::KeyDown(data) => {
    if data.is_composing {
        return; // Do not filter during IME composition
    }
    // ... process character for search
}
```

The adapter normalizes `compositionstart` / `compositionend` events from the browser into the `is_composing` flag on subsequent `KeyboardEventData` instances.

> **Chrome event ordering:** In Chrome, `keydown` fires **before** `compositionstart`, with `event.key === "Process"` and `isComposing: false`. The adapter MUST treat `key === "Process"` as an indicator that composition is starting, regardless of the `isComposing` flag value. Firefox fires `compositionstart` before `keydown`, so no special handling is needed.
>
> **Browser Quirk:** Legacy browsers signal IME composition via `keyCode === 229` on `keydown` events. Modern browsers expose `KeyboardEvent.isComposing` and fire `compositionstart`/`compositionend` events. Adapter implementors should check `isComposing` first and fall back to `keyCode === 229` detection for older browser support. Both indicators mean "do not process this key as direct input."

### 11.6 Accessibility Guidance

- If a consumer's `on_key_down` handler calls `preventDefault()` for a key, the consuming component should set `aria-keyshortcuts` on the relevant element to expose the shortcut to assistive technology.
- Single printable character shortcuts must comply with WCAG 2.1 SC 2.1.4 (Character Key Shortcuts — Level AA): they must be remappable, toggleable, or only active when the element has focus.
- The `Keyboard` interaction is intentionally neutral — it does not call `preventDefault()` or enforce WCAG compliance. Compliance is the consumer's responsibility.

---

## 12. InteractOutside Interaction

> Cross-references: Equivalent to React Aria `useInteractOutside`.

### 12.1 Purpose of InteractOutside

Detects pointer and touch interactions that occur outside a given element. This is the reusable primitive behind "click outside to close" behavior used by Dialog (backdrop click), Popover, Menu, Select, Combobox, and other overlay components.

Currently each overlay component re-implements this logic independently. `InteractOutside` extracts it into a composable interaction primitive.

### 12.2 Configuration of InteractOutside

```rust
// ars-interactions/src/interact_outside.rs

use ars_core::{Callback, PointerType};

/// Standalone interaction primitive for detecting clicks/interactions outside a target element.
/// Used independently of DismissableLayer for custom components.
///
/// Configuration:
/// - `target_id`: The element to monitor for outside interactions
/// - `portal_owner_ids`: Additional portal-owner IDs considered "inside"
/// - `on_interact_outside`: Callback when interaction outside is detected
/// - `enabled`: Whether detection is active
/// - `pointer_gracing`: Grace period (ms) after pointer leaves before triggering (for submenus)
pub struct InteractOutsideStandalone {
    /// The ID of the element to monitor for outside interactions.
    /// Uses a String ID (not ElementRef) because InteractOutside is in
    /// ars-interactions, which is below ars-dom in the dependency graph.
    pub target_id: String,
    /// Portal-owner IDs corresponding to `data-ars-portal-owner` markers that
    /// should be treated as inside this interaction boundary.
    pub portal_owner_ids: Vec<String>,
    pub on_interact_outside: Option<Callback<dyn Fn(InteractOutsideEvent)>>,
    pub enabled: bool,
    pub pointer_gracing: Option<u32>,
}

/// Configuration for outside-interaction detection (composable version).
#[derive(Clone, Debug, Default)]
pub struct InteractOutsideConfig {
    /// Whether the interaction is disabled.
    pub disabled: bool,

    /// Whether to also detect focus moving outside (not just pointer events).
    /// Default: `false` (pointer-only). Overlay components (Menu, Select, Combobox,
    /// Popover) MUST set this to `true` per §12.5.
    pub detect_focus: bool,
}

/// See `InteractOutsideEvent` definition in §12.6 below (includes EscapeKey variant).
```

### 12.3 Detection Mechanism

The adapter implements outside detection by:

1. Attaching a `pointerdown` listener to the document root during capture phase
2. Using `document.elementFromPoint(e.clientX, e.clientY)` to determine whether the interaction target is within the tracked element (per §2.4.1 pointer capture advisory), falling back to `event.target` when `elementFromPoint` returns null
3. If the resolved target is **not** contained, firing `InteractOutsideEvent::PointerOutside`
4. Optionally monitoring `focusin` events on the document to detect focus leaving the element boundary

> **Outside detection event.** Use `pointerdown` (not `click`) for outside interaction detection.
> iOS Safari does not fire `click` events on non-interactive elements (elements without a
> `click` handler or `cursor:pointer`). `pointerdown` fires reliably on all elements across
> all platforms.

### 12.4 Edge Cases

- **Portaled content**: When content is rendered in a `Portal` (e.g., a dropdown menu), the adapter must consider portal children as "inside" the interaction boundary. This is achieved by maintaining the relevant portal-owner IDs and matching them against `data-ars-portal-owner`.
- **Nested overlays**: If a click occurs inside a child overlay (e.g., a tooltip inside a popover), it should NOT trigger an outside interaction on the parent. The overlay stacking context must be consulted.

> **Portal-aware outside detection.** Since portal content is DOM-detached from the component
> tree, `element.contains(target)` will return false for clicks inside portaled content.
> The detection algorithm must walk up from the `elementFromPoint()`-resolved element (passed
> from §12.3 step 2) using `resolved.closest('[data-ars-portal-owner="COMPONENT_ID"]')` to
> check portal ownership. Any element with a matching `data-ars-portal-owner` attribute is
> considered "inside". Note: `event.target` is unreliable during pointer capture — always use
> the resolved element from `elementFromPoint()`.

- **Touch devices**: On iOS Safari, `touchstart` outside an element does not always fire document-level events. The adapter uses `pointerdown` with `{ capture: true }` on the document, which fires reliably on all elements (consistent with §12.3).
- **iframes**: Cross-origin iframe clicks are undetectable from the parent frame (no events bubble across cross-origin boundaries). The adapter fires `InteractOutside` on iframe `blur` as a proxy, but this only covers the case where focus _leaves_ the iframe — it does NOT fire when a user clicks _inside_ the iframe (focus moves from parent into iframe). This is a known platform limitation with no workaround.

### 12.5 Composition with Overlay Components

After adding `InteractOutside`, overlay component specs can reference it instead of implementing their own outside-click logic:

| Component | Current Implementation               | After `InteractOutside`                      | `detect_focus` |
| --------- | ------------------------------------ | -------------------------------------------- | :------------: |
| Dialog    | `CloseOnBackdropClick` event         | Compose `InteractOutside` on content element |     `true`     |
| Popover   | `CloseOnInteractOutside` event       | Compose `InteractOutside`                    |     `true`     |
| Menu      | Inline pointerdown-outside detection | Compose `InteractOutside`                    |     `true`     |
| Select    | Close on outside click (inline)      | Compose `InteractOutside`                    |     `true`     |
| Combobox  | Close on outside click (inline)      | Compose `InteractOutside`                    |     `true`     |

> **Note**: All overlay components require `detect_focus: true` to satisfy WCAG 2.1 SC 2.1.2 (No Keyboard Trap) and SC 2.4.3 (Focus Order). Without focus-outside detection, keyboard users who Tab past the last focusable element in an open overlay will not trigger a close, leaving the overlay open and `aria-expanded="true"` on the trigger despite focus having moved elsewhere.

The migration to `InteractOutside` is backwards-compatible: existing events remain; the underlying implementation changes to delegate to the shared primitive.

#### 12.5.1 Page Visibility API for Deferred Updates

Components with timers, animations, or periodic announcements MUST respect the Page Visibility API to avoid unnecessary work when the page is hidden:

- **Pause timers and animations:** On `document.visibilitychange` with `document.visibilityState === "hidden"`, pause all active timers (tooltip delays, toast auto-dismiss, debounce timers) and CSS/JS animations. Resume when visibility returns to `"visible"`.
- **Batch announcements:** Live region announcements that fire while the page is hidden SHOULD be batched. On visibility restore, announce the most recent state rather than replaying all missed announcements.
- **Timing accuracy:** Use `performance.now()` for elapsed-time calculations rather than `Date.now()`. When the page is hidden, `setTimeout`/`setInterval` may be throttled to 1-second minimum intervals by the browser. After visibility restore, recalculate elapsed time using `performance.now()` to determine whether a delayed action should fire immediately.
- **`requestIdleCallback` alternative:** For non-urgent deferred work (e.g., lazy attribute updates, analytics), prefer `requestIdleCallback` with a `timeout` parameter. When unavailable (Safari < 18), fall back to `MessageChannel` (not `setTimeout(fn, 0)`, which clamps to ≥4ms).
- **Effect integration:** The adapter's effect cleanup system should automatically pause effects when the page is hidden. Effects that register a `visibilitychange` listener in setup MUST remove it in cleanup.

### 12.6 Escape Key Handling

`InteractOutside` integrates with keyboard dismiss behavior:

- **Escape key** fires `InteractOutsideEvent::EscapeKey` through the same callback as pointer/focus outside events.
- The Escape listener is attached to the **overlay content element** (the portaled element that has focus), not the trigger element. Since portaled content is DOM-detached from the trigger, a listener on the trigger would not receive `keydown` events when focus is inside the portal. The adapter MUST ensure the listener is on the element that actually contains focus.
- After the topmost overlay handles Escape, the adapter MUST call `event.stopPropagation()` on the keydown event to prevent Escape from bubbling to parent overlay elements. This ensures the one-at-a-time top-down closing order specified in §12.8.
- After Escape dismissal, focus MUST return to the trigger element that opened the overlay.
- If `InteractOutsideConfig::disabled` is `true`, Escape is also suppressed.

```rust
/// Extended event enum including Escape key dismiss.
#[derive(Clone, Debug)]
pub enum InteractOutsideEvent {
    /// A pointer event occurred outside the target element(s).
    /// Carries the pointer coordinates and type for gracing behavior
    /// and conditional dismiss logic (e.g., submenus, nested overlays).
    PointerOutside {
        /// Client X coordinate of the pointer event.
        client_x: f64,
        /// Client Y coordinate of the pointer event.
        client_y: f64,
        /// The type of pointer that triggered the event.
        pointer_type: PointerType,
    },
    FocusOutside,
    /// The Escape key was pressed while the overlay had focus.
    EscapeKey,
}
```

### 12.7 Portal Boundary Respect

Portal-rendered content is DOM-detached from the component tree. The detection algorithm must respect portal boundaries:

1. Obtain the resolved element from `elementFromPoint()` (passed from §12.3 step 2). Note: `event.target` is unreliable during pointer capture — always use the resolved element.
2. Check `element.contains(resolved)` for direct DOM containment.
3. If not contained, walk up from the resolved element checking for `[data-ars-portal-owner="COMPONENT_ID"]` attributes.
4. Any element with a matching `data-ars-portal-owner` is considered "inside" the interaction boundary.
5. The `portal_owner_ids` field in `InteractOutsideStandalone` stores those portal-owner IDs. It does not store arbitrary DOM element IDs; direct DOM containment remains the responsibility of step 2.

### 12.8 Nested Overlay Handling

When overlays are nested (e.g., a Menu opens a sub-menu, or a Dialog contains a Select dropdown):

1. **Only the topmost overlay responds to outside interactions.** The overlay stack (managed by the z-index system, `11-dom-utilities.md` §6) determines ordering.
2. **A click inside a child overlay does NOT trigger `InteractOutside` on the parent.** The detection algorithm checks the full overlay stack, not just direct DOM containment.
3. **Closing order is top-down.** Escape key and outside-click dismiss the topmost overlay first. The parent overlay remains open until it receives its own dismiss signal.
4. **Stack registration:** Each overlay registers itself with a global overlay stack on mount and deregisters on unmount. `InteractOutside` consults this stack to determine whether the `elementFromPoint()`-resolved element (from §12.3 step 2) is inside any child overlay. Note: `event.target` is unreliable during pointer capture.

### 12.9 Component Cross-References

The following overlay components use `InteractOutside` (or will migrate to it per §12.5):

- **Dialog** (`components/overlay/dialog.md`) — backdrop click dismissal
- **Popover** (`components/overlay/popover.md`) — close on interact outside
- **Menu** (`components/selection/menu.md`) — close on outside pointer down
- **Select** (`components/selection/select.md`) — close listbox on outside click
- **Combobox** (`components/selection/combobox.md`) — close listbox on outside click
- **HoverCard** (`components/overlay/hover-card.md`) — dismiss on outside interaction
- **DatePicker** (`components/date-time/date-picker.md`) — close calendar overlay on outside click
