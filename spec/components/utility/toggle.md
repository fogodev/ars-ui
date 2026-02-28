---
component: Toggle
category: utility
tier: stateful
foundation_deps: [architecture, accessibility]
shared_deps: []
related: [toggle-group, toggle-button]
references:
  ark-ui: Toggle
  radix-ui: Toggle
---

# Toggle

A `Toggle` is a button that maintains a pressed/unpressed state. It is semantically distinct from a `Checkbox` (which represents a form value) and is appropriate for UI state toggles like bold/italic in a text editor or enabling/disabling a view mode.

> **Not a form control.** Toggle does not participate in HTML form submission. It is a UI state toggle, not a form control. For form-participating toggle behavior, use `ToggleButton` within a `ToggleGroup` (which supports `name` for hidden input submission), or use `Checkbox`.
>
> **No validation states.** Toggle does not expose validation states (`invalid`, `required`). For validated toggle behavior, use `Checkbox` with form integration.
>
> **Deliberate `forms` omission.** The `foundation_deps` for Toggle intentionally exclude `forms` because Toggle is a pure UI state component with no form participation semantics.

## 1. State Machine

### 1.1 States

| State | Description                |
| ----- | -------------------------- |
| `Off` | The toggle is not pressed. |
| `On`  | The toggle is pressed.     |

Both `Off` and `On` can also be `Focused` and `FocusVisible` — these are tracked in the context rather than as separate states to avoid combinatorial explosion.

```rust
/// The state of the `Toggle` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The toggle is not pressed.
    Off,
    /// The toggle is pressed.
    On,
}
```

### 1.2 Events

| Event         | Payload             | Description               |
| ------------- | ------------------- | ------------------------- |
| `Toggle`      | —                   | Flip between On and Off.  |
| `TurnOn`      | —                   | Explicitly set to On.     |
| `TurnOff`     | —                   | Explicitly set to Off.    |
| `Focus`       | `is_keyboard: bool` | Focus received.           |
| `Blur`        | —                   | Focus lost.               |
| `SetDisabled` | `bool`              | Sync disabled from props. |

```rust
/// The events for the `Toggle` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Toggle between On and Off.
    Toggle,
    /// Explicitly set to On.
    TurnOn,
    /// Explicitly set to Off.
    TurnOff,
    /// Focus received.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Focus lost.
    Blur,
    /// Sync disabled state from props.
    SetDisabled(bool),
}
```

### 1.3 Context

```rust
/// The context of the `Toggle` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Controlled/uncontrolled pressed value.
    pub pressed: Bindable<bool>,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is focused.
    pub focused: bool,
    /// Whether focus was received via keyboard (for focus-visible styles).
    pub focus_visible: bool,
}
```

### 1.4 Props

```rust
/// Props for the `Toggle` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled pressed value. If Some, the component is controlled.
    pub pressed: Option<bool>,
    /// Default uncontrolled pressed value.
    pub default_pressed: bool,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Callback invoked when the pressed state changes.
    /// When the pressed state changes, invoke `on_change` with the new pressed value.
    pub on_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
}
```

### 1.5 Full Machine Implementation

```rust
/// The machine for the `Toggle` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let pressed = match props.pressed {
            Some(v) => Bindable::controlled(v),
            None => Bindable::uncontrolled(props.default_pressed),
        };
        let state = if *pressed.get() { State::On } else { State::Off };
        (state, Context { pressed, disabled: props.disabled, focused: false, focus_visible: false })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        _props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        // Disabled guard: blocks value-changing events but allows Focus/Blur
        // so the toggle remains discoverable by screen readers.
        if ctx.disabled && !matches!(event, Event::Focus { .. } | Event::Blur | Event::SetDisabled(_)) {
            return None;
        }

        match event {
            Event::Toggle => {
                let next = match state {
                    State::Off => State::On,
                    State::On => State::Off,
                };
                let new_val = next == State::On;
                Some(TransitionPlan::to(next).apply(move |ctx| {
                    ctx.pressed.set(new_val);
                }))
            }
            Event::TurnOn => {
                if *state == State::On { return None; }
                Some(TransitionPlan::to(State::On).apply(|ctx| { ctx.pressed.set(true); }))
            }
            Event::TurnOff => {
                if *state == State::Off { return None; }
                Some(TransitionPlan::to(State::Off).apply(|ctx| { ctx.pressed.set(false); }))
            }
            Event::Focus { is_keyboard } => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }
            Event::Blur => Some(TransitionPlan::context_only(|ctx| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),
            Event::SetDisabled(disabled) => {
                let d = *disabled;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.disabled = d;
                }))
            }
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        let mut events = Vec::new();
        if new.disabled != old.disabled {
            events.push(Event::SetDisabled(new.disabled));
        }
        match (old.pressed, new.pressed) {
            (Some(false) | None, Some(true)) => events.push(Event::TurnOn),
            (Some(true), Some(false)) => events.push(Event::TurnOff),
            _ => {}
        }
        events
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "toggle"]
pub enum Part {
    Root,
    Indicator,
}

/// The API for the `Toggle` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the toggle is pressed.
    pub fn is_pressed(&self) -> bool { *self.state == State::On }

    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Data("ars-state"), if self.is_pressed() { "on" } else { "off" });
        p.set(HtmlAttr::Aria(AriaAttr::Pressed), if self.is_pressed() { "true" } else { "false" });
        p.set(HtmlAttr::Type, "button");
        if self.ctx.disabled {
            // Use aria-disabled instead of the HTML disabled attribute so the
            // toggle remains focusable and discoverable by screen readers.
            p.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            p.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if self.ctx.focus_visible {
            p.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        if self.is_pressed() {
            p.set_bool(HtmlAttr::Data("ars-pressed"), true);
        }
        p
    }

    /// The attributes for the indicator element.
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Data("ars-state"), if self.is_pressed() { "on" } else { "off" });
        p
    }

    pub fn on_root_click(&self) { (self.send)(Event::Toggle); }
    pub fn on_root_focus(&self, is_keyboard: bool) { (self.send)(Event::Focus { is_keyboard }); }
    pub fn on_root_blur(&self) { (self.send)(Event::Blur); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Indicator => self.indicator_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Toggle
├── Root        <button>  data-ars-scope="toggle" data-ars-part="root"
│                         aria-pressed="true|false" data-ars-state="on|off"
└── Indicator   <span>    data-ars-scope="toggle" data-ars-part="indicator"
                          (optional, conditional content based on state)
```

| Part      | Element    | Key Attributes                                                      |
| --------- | ---------- | ------------------------------------------------------------------- |
| Root      | `<button>` | `aria-pressed`, `data-ars-state="on\|off"`, `type="button"`         |
| Indicator | `<span>`   | `data-ars-state="on\|off"` (optional — conditional content display) |

### 2.1 Indicator Part

The `Indicator` part enables conditional content rendering based on toggle state, equivalent to Ark UI's `Toggle.Indicator`. Consumers render content conditionally based on `data-ars-state`:

```html
<toggle::Indicator>
  <!-- Shown when on -->
  <CheckIcon />
</toggle::Indicator>
```

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property        | Value                                 |
| --------------- | ------------------------------------- |
| Role            | `button` (implicit on `<button>`)     |
| `aria-pressed`  | `"true"` / `"false"`                  |
| `aria-disabled` | Present when `disabled=true`          |
| `type`          | `"button"` (prevents form submission) |

- `data-ars-state="on|off"` enables CSS styling with `[data-ars-state="on"]`.
- **Disabled handling**: Uses `aria-disabled="true"` instead of the HTML `disabled` attribute so the toggle remains in the tab order and discoverable by screen readers. The state machine allows `Focus`/`Blur` events through when disabled to support this.
- Use meaningful labels: "Bold" not "B". If icon-only, provide `aria-label`.
- Interactive toggles MUST meet the minimum 44x44 CSS pixel touch target size (see foundation/03-accessibility.md §7.1.1).

### 3.2 Keyboard Interaction

| Key           | Action       |
| ------------- | ------------ |
| Space / Enter | Toggle state |
| Tab           | Move focus   |

### 3.3 Forced Colors Mode

In `@media (forced-colors: active)`, the pressed state indicator MUST remain visible. Use border or outline rather than background color:

```css
@media (forced-colors: active) {
  [data-ars-state="on"] {
    outline: 2px solid ButtonText;
    outline-offset: -2px;
  }
}
```

### 3.4 Screen Reader Testing

| Action            | Expected announcement                                        |
| ----------------- | ------------------------------------------------------------ |
| Tab to toggle     | '[label], toggle button, pressed/not pressed'                |
| Press Space/Enter | '[label], toggle button, pressed/not pressed' (state change) |
| When disabled     | '[label], toggle button, dimmed/unavailable'                 |

### 3.5 `aria-pressed` Timing on Async State Changes

When the toggle's `on_change` handler performs an asynchronous operation (e.g., API call), the timing of the `aria-pressed` attribute update depends on the chosen strategy:

- **Optimistic (default recommendation)**: Update `aria-pressed` immediately on click, before the async operation completes. If the async operation fails, revert `aria-pressed` to its previous value. This provides better UX for low-latency operations because the user sees immediate feedback.

- **Pessimistic**: Keep the current `aria-pressed` value until the async operation confirms success. Only then update `aria-pressed` to the new value. Use this pattern for destructive or irreversible actions where premature visual feedback could mislead the user.

- **During async pending state**: The button SHOULD show a loading indicator (e.g., spinner via `data-ars-loading="true"`) but keep its current pressed state. The loading indicator signals that the action is in progress. Do not toggle `aria-pressed` to an intermediate value — it must always reflect either the current confirmed state (pessimistic) or the optimistically assumed state (optimistic).

- **Error rollback (optimistic)**: When the async operation fails after an optimistic update, the adapter MUST revert the toggle state by sending `Event::Toggle` (or `Event::TurnOn`/`Event::TurnOff` as appropriate) to return `aria-pressed` to its pre-click value. A brief error indication (e.g., toast notification) should inform the user.

## 4. Internationalization

- Label text is consumer-provided.
- `data-ars-state` values (`on`, `off`) are stable API tokens, not localized.

## 5. Library Parity

> Compared against: Ark UI (`Toggle`), Radix UI (`Toggle`).

### 5.1 Props

| Feature            | ars-ui                  | Ark UI            | Radix UI          | Notes                                 |
| ------------------ | ----------------------- | ----------------- | ----------------- | ------------------------------------- |
| Controlled pressed | `pressed: Option<bool>` | `pressed`         | `pressed`         | All libraries                         |
| Default pressed    | `default_pressed`       | `defaultPressed`  | `defaultPressed`  | All libraries                         |
| Disabled           | `disabled`              | --                | `disabled`        | Ark exposes disabled via context only |
| on_change callback | `on_change`             | `onPressedChange` | `onPressedChange` | All libraries                         |

**Gaps:** None.

### 5.2 Anatomy

| Part      | ars-ui      | Ark UI      | Radix UI | Notes                                |
| --------- | ----------- | ----------- | -------- | ------------------------------------ |
| Root      | `Root`      | `Root`      | `Root`   | All libraries                        |
| Indicator | `Indicator` | `Indicator` | --       | Ark UI has Indicator; Radix does not |

**Gaps:** None. ars-ui matches Ark UI's Indicator part.

### 5.3 Features

| Feature                 | ars-ui           | Ark UI       | Radix UI     |
| ----------------------- | ---------------- | ------------ | ------------ |
| Pressed state           | Yes              | Yes          | Yes          |
| Controlled/uncontrolled | Yes              | Yes          | Yes          |
| Indicator part          | Yes              | Yes          | --           |
| Focus-visible tracking  | Yes              | --           | --           |
| Data attributes         | `data-ars-state` | `data-state` | `data-state` |

**Gaps:** None.

### 5.4 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui tracks focus/focus-visible in context (not exposed by Ark/Radix). ars-ui uses `data-ars-state` prefix vs `data-state`.
- **Recommended additions:** None.
