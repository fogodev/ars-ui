---
component: Dismissable
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
  react-aria: DismissButton
---

# Dismissable

Dismissable is an internal building block primarily used by overlay components (`Dialog`, `Popover`, `Tooltip`, `Select`, `Combobox`, `Menu`, etc.). It is also available for direct consumer use when building custom dismissable surfaces. Consumers building custom overlays, floating panels, or any dismissable surface can use `Props` directly to get consistent click-outside and Escape-to-close behavior without reimplementing it.

A shared behavior primitive for handling outside interactions (click, focus, Escape) that should dismiss an overlay or popover element. Consolidates the "click outside / press Escape to close" pattern used by `Dialog`, `Popover`, `Tooltip`, `Select`, `Combobox`, `Menu`, and others. Matches Ark-UI's `Dismissable` behavior and React Aria's `useInteractOutside` / `DismissButton`.

**Public composition guide**: To use Dismissable in a custom overlay component, create a `Props` value and pass it to the adapter's `use_dismissable()` hook. The hook registers the necessary document-level listeners and returns a cleanup function. See §6 Integration for the pattern used by `Dialog` and `Popover`.

## 1. API

### 1.1 Props

Core `Props` contain only behavioral configuration — callbacks, pointer-event blocking, and excluded IDs. Locale and messages are environment context resolved by the adapter and passed separately to [`dismiss_button_attrs`].

```rust
/// Props for the `Dismissable` component.
#[derive(Clone, Default, PartialEq)]
pub struct Props {
    /// Called when the user interacts outside the dismissable element.
    /// The adapter invokes this on `pointerdown` outside, or `focusin` on an element outside.
    pub on_interact_outside: Option<Callback<dyn Fn(InteractOutsideEvent)>>,
    /// Called when the user presses the Escape key while focus is inside.
    pub on_escape_key_down: Option<Callback<dyn Fn()>>,
    /// Called when a dismiss trigger fires (combines outside interaction and Escape).
    pub on_dismiss: Option<Callback<dyn Fn()>>,
    /// When true, outside pointer events are intercepted and prevented from reaching
    /// underlying elements (pointer-events overlay). Default: false.
    pub disable_outside_pointer_events: bool,
    /// DOM IDs of elements that should NOT trigger an outside interaction when clicked.
    /// Typically includes the trigger button that opened the overlay.
    pub exclude_ids: Vec<String>,
}

impl fmt::Debug for Props {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Props")
            .field("disable_outside_pointer_events", &self.disable_outside_pointer_events)
            .field("on_interact_outside", &self.on_interact_outside.as_ref().map(|_| "<closure>"))
            .field("on_escape_key_down", &self.on_escape_key_down.as_ref().map(|_| "<closure>"))
            .field("on_dismiss", &self.on_dismiss.as_ref().map(|_| "<closure>"))
            .finish()
    }
}

// `InteractOutsideEvent` — defined in `05-interactions.md` §12 (InteractOutside Interaction)
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "dismissable"]
pub enum Part {
    Root,
    DismissButton,
}

/// Returns attrs for the visually-hidden DismissButton element.
///
/// `locale` and `messages` are resolved by the adapter from `ArsProvider`
/// context and passed explicitly — this function has no framework dependency.
pub fn dismiss_button_attrs(locale: &Locale, messages: &Messages) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DismissButton.data_attrs();
    attrs.set(scope_attr, scope_val);
    attrs.set(part_attr, part_val);
    attrs.set(HtmlAttr::Role, "button");
    attrs.set(HtmlAttr::TabIndex, "0");
    attrs.set(HtmlAttr::Aria(AriaAttr::Label), (messages.close_label)(locale));
    attrs.set_bool(HtmlAttr::Data("ars-visually-hidden"), true);
    attrs
}
```

> **DismissButton keyboard activation:** The DismissButton element uses `role="button"`. Adapters must ensure Enter and Space key activation is handled — either by rendering as a native `<button>` element (preferred) or by adding keydown handlers for Enter/Space.

**Element requirement:** The DismissButton MUST be rendered as a native `<button>` element (not a `<div>` with `role="button"`). This ensures correct keyboard activation (Enter and Space) without additional event handlers. If the adapter renders a non-button element, it MUST add `keydown` handlers for Enter and Space activation.

The `DismissButton` is a visually-hidden button placed at the start and end of a dismissable region. It provides screen reader users a click target to dismiss overlays without relying on Escape.

## 2. Anatomy

```text
Dismissable
├── DismissButton  <button>  (visually hidden, start of region)
├── {content}
└── DismissButton  <button>  (visually hidden, end of region)
```

| Part          | Element    | Key Attributes                                                                 |
| ------------- | ---------- | ------------------------------------------------------------------------------ |
| DismissButton | `<button>` | `data-ars-scope="dismissable"`, `data-ars-part="dismiss-button"`, `aria-label` |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

When `disable_outside_pointer_events` is true, screen reader users must still be able to dismiss via Escape. The invisible overlay must not trap keyboard navigation — only pointer events are blocked. A `<DismissButton>` (visually hidden, `aria-label` from `Messages::close_label`) is placed at the start and end of the dismissable region so screen readers can dismiss without pressing Escape.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the visually-hidden DismissButton.
    pub close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            close_label: MessageFn::static_str("Dismiss"),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                       | Default (en-US) | Purpose                                              |
| ------------------------- | --------------- | ---------------------------------------------------- |
| `dismissable.close_label` | `"Dismiss"`     | `aria-label` for the visually-hidden `DismissButton` |

## 5. Behavior

| Trigger                                                | Action                                                                                     |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| `pointerdown` outside element and not in `exclude_ids` | Calls `on_interact_outside(PointerDown)` then `on_dismiss`                                 |
| `focusin` on element outside and not in `exclude_ids`  | Calls `on_interact_outside(FocusOutside)` then `on_dismiss`                                |
| Escape key pressed while focus is inside               | Calls `on_escape_key_down` then `on_dismiss`                                               |
| `disable_outside_pointer_events = true`                | Adds a transparent overlay (`pointer-events: auto`) to block clicks reaching content below |

### 5.1 Effect Setup Safety and Cleanup Ordering

The click-outside detection listener setup must handle timing edge cases:

1. **Mount Verification**: Before attaching the document-level click listener, verify that `document.getElementById(content_id)` returns a non-null element. The `Dismissable` content may not yet be mounted when the effect first runs (e.g., when used inside animated transitions).
2. **Deferred Setup**: If the element is not found, schedule a retry on the next `requestAnimationFrame`. Retry at most 3 times before logging a warning and abandoning setup (the element is likely not going to mount).
3. **Cleanup Ordering**: On teardown, remove the click-outside listener BEFORE the portal element is removed from the DOM. This prevents a brief window where clicks could trigger on a partially-unmounted element.
4. **Idempotent Cleanup**: The cleanup function must be idempotent — calling it multiple times (e.g., due to effect re-runs) must not throw or produce side effects. Track listener attachment state with a boolean guard.
5. **Adapter Note**: The deferred setup with `requestAnimationFrame` retry for mount verification should be implemented as: Leptos — `request_animation_frame` via `web_sys`; Dioxus Web — same via `web_sys`; Dioxus Desktop — `spawn` with a short delay.
6. **Deferred setup cleanup:** The `requestAnimationFrame` retry loop (up to 3 retries) MUST track all pending rAF IDs. If the component unmounts before the retry loop completes, all pending rAF callbacks MUST be cancelled in the cleanup function (`on_cleanup` in Leptos, `use_drop` in Dioxus):

```rust
// Track pending rAF IDs for cleanup
let pending_raf: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(Vec::new()));
let raf_id = request_animation_frame(/* ... */);
pending_raf.borrow_mut().push(raf_id);

// In cleanup:
for id in pending_raf.borrow().iter() {
    cancel_animation_frame(*id);
}
```

> **Platform Note:** Click-outside detection attaches a `pointerdown` listener on `document`. On Dioxus Desktop (webview), `pointer-events: auto` on the transparent overlay may behave differently than in browsers. Test click-outside behavior on each target platform.

### 5.2 SSR Safety

`use_dismissable()` MUST be gated behind a client-only context. During SSR, no document-level listeners are attached and no DOM queries are performed.

- **Leptos:** Wrap listener setup in `Effect::new` or guard with `#[cfg(not(feature = "ssr"))]`.
- **Dioxus:** Perform all listener setup inside `use_effect`, which only runs on the client.

If the adapter attempts to call `document.add_event_listener()` or `document.getElementById()` during SSR, it will panic or produce undefined behavior.

## 6. Integration

Overlay components compose `Dismissable` internally:

```rust
// Inside Dialog, Popover, Select content, Menu content, etc.
let dismissable = dismissable::Props {
    on_dismiss: Some(Callback::new_void(move || machine.send(Event::Close))),
    on_escape_key_down: Some(Callback::new_void(move || machine.send(Event::Close))),
    disable_outside_pointer_events: props.modal,
    exclude_ids: vec![trigger_id.clone()],
    ..Default::default()
};

// The adapter hook resolves locale/messages from ArsProvider and passes them:
// let locale = use_locale();
// let messages = resolve_messages::<dismissable::Messages>(&locale);
// let button_attrs = dismissable::dismiss_button_attrs(&locale, &messages);
```

## 7. Library Parity

> Compared against: React Aria (`DismissButton`).

### 7.1 Props

| Feature                        | ars-ui                           | React Aria  | Notes                                               |
| ------------------------------ | -------------------------------- | ----------- | --------------------------------------------------- |
| on_dismiss                     | `on_dismiss`                     | `onDismiss` | Both libraries                                      |
| on_escape_key_down             | `on_escape_key_down`             | --          | ars-ui addition (RA handles via overlay hooks)      |
| on_interact_outside            | `on_interact_outside`            | --          | ars-ui addition (RA uses `useInteractOutside` hook) |
| on_focus_outside               | `on_focus_outside`               | --          | ars-ui addition                                     |
| on_pointer_down_outside        | `on_pointer_down_outside`        | --          | ars-ui addition                                     |
| disable_outside_pointer_events | `disable_outside_pointer_events` | --          | ars-ui addition for modal behavior                  |
| exclude_ids                    | `exclude_ids`                    | --          | ars-ui addition for trigger exclusion               |

**Gaps:** None.

### 7.2 Anatomy

| Part          | ars-ui          | React Aria      | Notes                                                   |
| ------------- | --------------- | --------------- | ------------------------------------------------------- |
| DismissButton | `DismissButton` | `DismissButton` | Both libraries provide a visually-hidden dismiss button |

**Gaps:** None.

### 7.3 Features

| Feature                       | ars-ui | React Aria                     |
| ----------------------------- | ------ | ------------------------------ |
| Click outside detection       | Yes    | Yes (via `useInteractOutside`) |
| Escape key handling           | Yes    | Yes (via `useOverlayTrigger`)  |
| DismissButton (screen reader) | Yes    | Yes                            |
| Focus outside detection       | Yes    | --                             |
| Pointer event blocking        | Yes    | Yes (via `useModal`)           |

**Gaps:** None.

### 7.4 Summary

- **Overall:** Full parity -- ars-ui is a superset.
- **Divergences:** React Aria splits dismiss behavior across multiple hooks (`DismissButton`, `useInteractOutside`, `useModal`); ars-ui consolidates into a single `Dismissable` component with comprehensive callback props.
- **Recommended additions:** None.
