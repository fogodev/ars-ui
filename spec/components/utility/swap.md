---
component: Swap
category: utility
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
    ark-ui: Swap
---

# Swap

A component that toggles between two visual states (e.g., sun/moon icon, hamburger/close
icon, play/pause button). Composes with `Presence` for cross-fade or flip animations between
the on and off content.

Combines interactive toggle behavior (role='button', aria-pressed, keyboard activation) with animated content swapping. Inspired by Ark UI's Swap visual transition and Toggle interactivity. State machine: `Off` / `On`.

**Swap vs Switch:** Swap is a visual content-swapping component with animated transitions between two content slots (on/off). It is NOT a form switch. For form-participating switch behavior with `name`, `value`, `required`, `invalid`, and hidden input support, use the Switch component (input category).

Swap does not support `read_only` because it does not participate in forms.

## 1. State Machine

### 1.1 States

| State | Description                       |
| ----- | --------------------------------- |
| `Off` | The component is in an off state. |
| `On`  | The component is in an on state.  |

### 1.2 Events

| Event         | Payload             | Description                       |
| ------------- | ------------------- | --------------------------------- |
| `Toggle`      | ---                 | Toggle between on and off states. |
| `SetOn`       | ---                 | Explicitly set to on.             |
| `SetOff`      | ---                 | Explicitly set to off.            |
| `SetDisabled` | `bool`              | Update disabled state from props. |
| `Focus`       | `is_keyboard: bool` | Focus received.                   |
| `Blur`        | ---                 | Focus lost.                       |

### 1.3 Context

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap, Bindable};

/// The context of the `Swap` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current checked (on/off) state.
    pub checked: Bindable<bool>,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether focus was received via keyboard (for focus-visible styling).
    pub focus_visible: bool,
    /// Component instance IDs.
    pub ids: ComponentIds,
    /// The active locale, inherited from ArsProvider context.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// Props for the `Swap` component.
#[derive(Clone, Debug, PartialEq, HasId, Default)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled checked state. When `Some`, the component is controlled.
    pub checked: Option<bool>,
    /// Default checked state for uncontrolled mode.
    pub default_checked: bool,
    /// Disabled state.
    pub disabled: bool,
    /// Stable accessible label for the root element (e.g., "Toggle dark mode").
    /// Applied as `aria-label`. Should describe the control's purpose, not its current state.
    pub label: Option<String>,
    /// The animation style for the swap transition (see section 6).
    pub animation: Animation,
    /// Callback invoked when the checked state changes.
    /// Adapters invoke this with the new checked value after a Toggle, SetOn, or SetOff transition.
    pub on_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap, Bindable};

/// The state of the `Swap` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is in an off state.
    Off,
    /// The component is in an on state.
    On,
}

/// The events for the `Swap` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Toggle between on and off states.
    Toggle,
    /// Explicitly set to on.
    SetOn,
    /// Explicitly set to off.
    SetOff,
    /// Update disabled state from props.
    SetDisabled(bool),
    /// Focus received.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Focus lost.
    Blur,
}

/// The machine for the `Swap` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let ids = ComponentIds::from_id(&props.id);
        let checked = match props.checked {
            Some(v) => Bindable::controlled(v),
            None    => Bindable::uncontrolled(props.default_checked),
        };
        let locale = env.locale.clone();
        let messages = messages.clone();
        let state = if *checked.get() { State::On } else { State::Off };
        (state, Context {
            checked,
            disabled: props.disabled,
            focus_visible: false,
            ids,
            locale,
            messages,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // Standard disabled guard: allow Focus/Blur for screen reader discoverability,
        // and SetDisabled so props sync always applies.
        if ctx.disabled && !matches!(event, Event::Focus { .. } | Event::Blur | Event::SetDisabled(_)) {
            return None;
        }

        match event {
            Event::Toggle => {
                let next = !ctx.checked.get();
                let next_state = if next { State::On } else { State::Off };
                Some(TransitionPlan::to(next_state).apply(move |ctx| {
                    ctx.checked.set(next);
                }))
            }
            Event::SetOn => {
                Some(TransitionPlan::to(State::On).apply(|ctx| {
                    ctx.checked.set(true);
                }))
            }
            Event::SetOff => {
                Some(TransitionPlan::to(State::Off).apply(|ctx| {
                    ctx.checked.set(false);
                }))
            }
            Event::SetDisabled(disabled) => {
                let disabled = *disabled;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.disabled = disabled;
                }))
            }
            Event::Focus { is_keyboard } => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focus_visible = is_kb;
                }))
            }
            Event::Blur => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focus_visible = false;
                }))
            }
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        let mut events = Vec::new();
        // Sync controlled checked state.
        if old.checked != new.checked {
            match new.checked {
                Some(true)  => events.push(Event::SetOn),
                Some(false) => events.push(Event::SetOff),
                None        => {} // Switching to uncontrolled; no event needed.
            }
        }
        // Sync disabled state.
        if old.disabled != new.disabled {
            events.push(Event::SetDisabled(new.disabled));
        }
        events
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
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "swap"]
pub enum Part {
    Root,
    OnContent,
    OffContent,
}

/// The API for the `Swap` component.
pub struct Api<'a> {
    /// The state of the `Swap` component.
    state: &'a State,
    /// The context of the `Swap` component.
    ctx: &'a Context,
    /// The props of the `Swap` component.
    props: &'a Props,
    /// The send callback for the `Swap` component.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the swap is currently in the "on" state.
    /// Derives from state (authoritative) rather than context Bindable to avoid potential desync.
    pub fn is_on(&self) -> bool {
        *self.state == State::On
    }

    /// Whether the swap is disabled.
    pub fn is_disabled(&self) -> bool {
        self.ctx.disabled
    }

    /// Root element attributes. The root wraps both on and off content.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), if self.is_on() { "true" } else { "false" });
        attrs.set(HtmlAttr::TabIndex, "0");
        let state_str = match self.state {
            State::On  => "on",
            State::Off => "off",
        };
        attrs.set(HtmlAttr::Data("ars-state"), state_str);
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        // Apply accessible label. When Props::label is set, use it as a stable
        // label. Otherwise, fall back to the state-specific Messages label so
        // screen readers announce whether the swap is currently on or off.
        if let Some(label) = &self.props.label {
            if !label.is_empty() {
                attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.as_str());
            }
        } else {
            let label = if self.is_on() {
                (self.ctx.messages.on_label)(&self.ctx.locale)
            } else {
                (self.ctx.messages.off_label)(&self.ctx.locale)
            };
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }

        attrs
    }

    /// Attributes for the "on" content container.
    /// Visible when `is_on()` is true. Composes with Presence for exit animations.
    pub fn on_content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::OnContent.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if !self.is_on() {
            attrs.set_bool(HtmlAttr::Hidden, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        }
        attrs
    }

    /// Attributes for the "off" content container.
    /// Visible when `is_on()` is false. Composes with Presence for exit animations.
    pub fn off_content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::OffContent.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.is_on() {
            attrs.set_bool(HtmlAttr::Hidden, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        }
        attrs
    }

    /// Handle keydown events on the root element.
    pub fn on_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Enter | KeyboardKey::Space => (self.send)(Event::Toggle),
            _ => {}
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::OnContent => self.on_content_attrs(),
            Part::OffContent => self.off_content_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Swap
├── Root              role="button", aria-pressed
│   ├── OnContent     visible when on
│   └── OffContent    visible when off
```

| Part         | Selector                                               | Element               |
| ------------ | ------------------------------------------------------ | --------------------- |
| `Root`       | `[data-ars-scope="swap"][data-ars-part="root"]`        | `<div>` or `<button>` |
| `OnContent`  | `[data-ars-scope="swap"][data-ars-part="on-content"]`  | `<span>`              |
| `OffContent` | `[data-ars-scope="swap"][data-ars-part="off-content"]` | `<span>`              |

**3 parts total.**

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- **`role="button"` + `aria-pressed`**: The root element uses `role="button"` with `aria-pressed` to
  communicate the binary on/off state to assistive technologies. Swap is a visual toggle
  button, not a form switch — `role="switch"` with `aria-checked` is reserved for the
  Switch component which participates in forms.
- **Focus management**: Single tab stop on the root. `data-ars-focus-visible` is set when
  focus was received via keyboard.
- **Labelling**: Consumers should provide an accessible label via `aria-label` or
  `aria-labelledby` on the root element describing what the swap controls (e.g.,
  "Toggle dark mode").
- Swap MUST meet the minimum 44x44 CSS pixel touch target size (see foundation/03-accessibility.md §7.1.1).

#### 3.1.1 Forced Colors Mode

In `@media (forced-colors: active)`, custom colors are stripped. The on/off state MUST remain distinguishable via border, outline, or system color changes:

```css
@media (forced-colors: active) {
    [data-ars-state="on"] {
        outline: 2px solid ButtonText;
        outline-offset: -2px;
    }
}
```

### 3.2 Keyboard Interaction

| Key               | Action            |
| ----------------- | ----------------- |
| `Enter` / `Space` | Toggle the state. |

This matches the WAI-ARIA button pattern.

## 4. Internationalization

### 4.1 Messages

```rust
/// The messages for the `Swap` component.
///
/// `on_label` and `off_label` serve two purposes:
/// 1. When `Props::label` is `None`, they are used as the root element's `aria-label`,
///    toggling based on the current state so screen readers announce the active state.
/// 2. Adapters may also render them as optional VisuallyHidden content inside
///    the on/off content slots for additional state-specific announcements.
#[derive(Clone, Debug)]
pub struct Messages {
    /// State-specific label for the "on" state (e.g., "Dark mode enabled").
    /// Used as `aria-label` fallback on root when `Props::label` is `None`.
    pub on_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// State-specific label for the "off" state (e.g., "Light mode enabled").
    /// Used as `aria-label` fallback on root when `Props::label` is `None`.
    pub off_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            on_label: MessageFn::static_str("On"),
            off_label: MessageFn::static_str("Off"),
        }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Presence Composition

`Swap` composes with [`Presence`](../overlay/presence.md) for animated transitions between on/off content:

```rust,no_check
// The adapter wires two Presence instances:
let on_presence  = Presence::new(swap_api.is_on());
let off_presence = Presence::new(!swap_api.is_on());

// OnContent renders when on_presence.is_mounted()
// OffContent renders when off_presence.is_mounted()
// CSS animations handle the cross-fade or flip effect.
```

This allows consumers to use CSS transitions/animations (e.g., `rotate`, `scale`,
`opacity`) for smooth visual transitions without any JS animation logic.

**Cleanup ordering:** On unmount, the adapter MUST cancel all pending Presence exit animations before cleaning up the parent Swap state machine. If a Presence animation-end callback fires after the Swap's state signal is dropped, it will cause a runtime panic. The recommended pattern: cancel animation timers first, then dispose the Swap Service.

Adapters MAY expose `data-ars-hover` and `data-ars-active` on the root element via adapter-level pointer/hover tracking, consistent with interaction patterns from `05-interactions.md`.

## 6. Swap Transition Pattern

`Swap` is a transition pattern for swapping between two content slots. Only one slot is rendered at a time (not both with a visibility toggle) — the inactive slot is fully unmounted when animations are not running.

**Anatomy (clarified):**

```text
Swap              root container
├── SwapOn        content shown when checked=true (the "on" slot)
└── SwapOff       content shown when checked=false (the "off" slot)
```

**Props:**

| Prop              | Type                                           | Description                                                             |
| ----------------- | ---------------------------------------------- | ----------------------------------------------------------------------- |
| `checked`         | `Option<bool>`                                 | Controlled checked state. When `Some`, the component is controlled.     |
| `default_checked` | `bool`                                         | Default checked state for uncontrolled mode.                            |
| `animation`       | `Animation`                                    | The animation style for the swap transition.                            |
| `on_change`       | `Option<Callback<dyn Fn(bool) + Send + Sync>>` | Fires when swap transition completes. Payload is the new checked value. |
| `disabled`        | `bool`                                         | When true, interaction is disabled.                                     |

**`Animation` enum:**

```rust
/// The animation style for the swap transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Animation {
    /// No animation — instant swap.
    #[default]
    None,
    /// Content rotates 180 degrees during the swap.
    Rotate,
    /// Content flips along the Y-axis (3D flip).
    Flip,
    /// Cross-fade between the two slots.
    Fade,
}
```

**Rendering behavior:**

- Only one of `SwapOn` or `SwapOff` is rendered in the DOM at any time. The inactive slot is not present in the DOM (unlike a visibility toggle which hides both).
- When `animation` is not `None`, the outgoing slot exits via `Presence` and the incoming slot enters via `Presence`, using CSS classes derived from the `Animation` variant.
- The `on_change` callback fires after the transition animation completes (or immediately if `animation` is `None`).

## 7. Library Parity

> Compared against: Ark UI (`Swap`).

### 7.1 Props

| Feature            | ars-ui                  | Ark UI | Notes                                            |
| ------------------ | ----------------------- | ------ | ------------------------------------------------ |
| Controlled checked | `checked: Option<bool>` | --     | Ark Swap has no controlled state; ars-ui adds it |
| Default checked    | `default_checked`       | --     | ars-ui addition                                  |
| Disabled           | `disabled`              | --     | ars-ui addition                                  |
| on_change          | `on_change`             | --     | ars-ui addition                                  |
| Animation          | `animation: Animation`  | --     | ars-ui addition for transition style selection   |
| Label              | `label`                 | --     | ars-ui addition for accessible name              |

**Gaps:** None.

### 7.2 Anatomy

| Part       | ars-ui       | Ark UI | Notes                                      |
| ---------- | ------------ | ------ | ------------------------------------------ |
| Root       | `Root`       | `Root` | Both libraries                             |
| OnContent  | `OnContent`  | `On`   | Both libraries; content slot for on state  |
| OffContent | `OffContent` | `Off`  | Both libraries; content slot for off state |

**Gaps:** None.

### 7.3 Features

| Feature                 | ars-ui                      | Ark UI                   |
| ----------------------- | --------------------------- | ------------------------ |
| Toggle state            | Yes                         | Yes                      |
| Controlled/uncontrolled | Yes                         | -- (Ark Swap is simpler) |
| Animation modes         | Yes (None/Rotate/Flip/Fade) | Yes (CSS-based)          |
| Disabled state          | Yes                         | --                       |
| Keyboard activation     | Yes                         | --                       |
| aria-pressed            | Yes                         | --                       |
| Presence composition    | Yes                         | --                       |

**Gaps:** None.

### 7.4 Summary

- **Overall:** Full parity -- ars-ui is a superset.
- **Divergences:** Ark UI's Swap is primarily a visual content switcher with no interactivity built-in. ars-ui adds full interactive behavior (keyboard, aria-pressed, disabled, controlled state), making it a complete toggle-with-swap-animation component.
- **Recommended additions:** None.
