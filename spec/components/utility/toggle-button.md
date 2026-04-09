---
component: ToggleButton
category: utility
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: [button, toggle-group, toggle]
references:
    react-aria: ToggleButton
---

# ToggleButton

A `ToggleButton` is a button that toggles between pressed and unpressed states, combining
press interaction semantics from `Button` with the toggle state semantics of `Toggle`. It is
the building block for `ToggleGroup`.

Unlike `Toggle` (which is a minimal On/Off state machine), `ToggleButton` tracks focus and
pressed states explicitly, enabling focus-visible styling and press feedback. Unlike `Button`
(which is stateless regarding pressed), `ToggleButton` maintains a persistent `pressed` state
across interactions.

## 1. State Machine

### 1.1 States

| State     | Description                                                              |
| --------- | ------------------------------------------------------------------------ |
| `Idle`    | Default resting state. Not focused, not being pressed.                   |
| `Focused` | The button has received focus. Keyboard focus also sets `focus_visible`. |
| `Pressed` | The button is actively being pressed (pointer held down or Space held).  |

### 1.2 Events

| Event         | Payload             | Description                                                         |
| ------------- | ------------------- | ------------------------------------------------------------------- |
| `Focus`       | `is_keyboard: bool` | Focus received; flag indicates keyboard vs pointer source.          |
| `Blur`        | ---                 | Focus lost; resets `focus_visible`.                                 |
| `Press`       | ---                 | Pointer/key press begins (pointerdown or keydown Space).            |
| `Release`     | ---                 | Pointer/key press ends (pointerup or keyup Space). Toggles pressed. |
| `Toggle`      | ---                 | Programmatically toggle the pressed state.                          |
| `SetPressed`  | `bool`              | Programmatically set the pressed state to a specific value.         |
| `SetDisabled` | `bool`              | Programmatically set the disabled state.                            |
| `Reset`       | ---                 | Restore pressed to `default_pressed` (form reset).                  |

### 1.3 Context

```rust
use ars_core::Bindable;

/// The context for the `ToggleButton` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Controlled/uncontrolled pressed (toggle) value.
    pub pressed: Bindable<bool>,
    /// Whether the button is disabled.
    pub disabled: bool,
    /// Whether the button is focused.
    pub focused: bool,
    /// Whether the button has keyboard focus (for focus-visible styles).
    pub focus_visible: bool,
}
```

### 1.4 Props

```rust
/// Props for the `ToggleButton` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled pressed value. If Some, the component is controlled.
    pub pressed: Option<bool>,
    /// Default uncontrolled pressed value.
    pub default_pressed: bool,
    /// Whether the button is disabled.
    pub disabled: bool,
    /// Whether the field is in an invalid state.
    pub invalid: bool,
    /// Whether the field is required.
    pub required: bool,
    /// Identifier when used within a `ToggleGroup`. The group uses this
    /// value to track which items are selected.
    pub value: Option<String>,
    /// Form field name. When set, a hidden `<input>` is rendered for
    /// native form submission (standalone use outside ToggleGroup).
    pub name: Option<String>,
    /// Associates the toggle button with a `<form>` element by `id`, even if the button
    /// is not a descendant of that form. Threaded to `HiddenInputConfig::form_id`.
    pub form: Option<String>,
    /// When true, the adapter suppresses focus events on pointer press. Used by composite
    /// widgets (e.g., ComboBox trigger) where pressing should not steal focus from a managed
    /// focus context.
    pub prevent_focus_on_press: bool,
    /// Fires when the pointer enters the toggle button.
    pub on_hover_start: Option<Callback<dyn Fn() + Send + Sync>>,
    /// Fires when the pointer leaves the toggle button.
    pub on_hover_end: Option<Callback<dyn Fn() + Send + Sync>>,
    /// Fires when hover state changes. Receives `true` on hover start, `false` on hover end.
    /// Hover callbacks are fired by the adapter's hover interaction layer.
    pub on_hover_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            pressed: None,
            default_pressed: false,
            disabled: false,
            invalid: false,
            required: false,
            value: None,
            name: None,
            form: None,
            prevent_focus_on_press: false,
            on_hover_start: None,
            on_hover_end: None,
            on_hover_change: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap};

// ── States ───────────────────────────────────────────────────────────────────

/// The states for the `ToggleButton` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Default resting state. Not focused, not being pressed.
    Idle,
    /// The button has received focus. Keyboard focus also sets `focus_visible`.
    Focused,
    /// The button is actively being pressed (pointer held down or Space held).
    Pressed,
}

// ── Events ───────────────────────────────────────────────────────────────────

/// The events for the `ToggleButton` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Focus received; flag indicates keyboard vs pointer source.
    Focus {
        /// Flag indicates keyboard vs pointer source.
        is_keyboard: bool,
    },
    /// Focus lost; resets `focus_visible`.
    Blur,
    /// Pointer/key press begins (pointerdown or keydown Space).
    Press,
    /// Pointer/key press ends (pointerup or keyup Space). Toggles pressed.
    Release,
    /// Programmatically toggle the pressed state.
    Toggle,
    /// Programmatically set the pressed state to a specific value.
    SetPressed(bool),
    /// Programmatically set the disabled state.
    SetDisabled(bool),
    /// Restore pressed to `default_pressed` (form reset).
    Reset,
}

// ── Machine ───────────────────────────────────────────────────────────────────

/// The machine for the `ToggleButton` component.
pub struct Machine;

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;
impl ComponentMessages for Messages {}

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, _env: &Env, _messages: &Self::Messages) -> (Self::State, Self::Context) {
        let pressed = match props.pressed {
            Some(v) => Bindable::controlled(v),
            None => Bindable::uncontrolled(props.default_pressed),
        };

        let ctx = Context {
            pressed,
            disabled: props.disabled,
            focused: false,
            focus_visible: false,
        };

        (State::Idle, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // Disabled guard: blocks value-changing events but allows Focus/Blur
        // so the button remains discoverable by screen readers.
        // Allow SetDisabled so props can re-enable the button.
        // Allow Reset so form reset works even when disabled.
        if ctx.disabled && !matches!(event, Event::Focus { .. } | Event::Blur | Event::SetDisabled(_) | Event::Reset) {
            return None;
        }

        match (state, event) {
            // ── Focus / Blur ─────────────────────────────────────────────────
            (State::Idle, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }
            // Focus source change while already focused — update focus_visible.
            (State::Focused, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focus_visible = is_kb;
                }))
            }
            (State::Focused, Event::Blur) => Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            // ── Press / Release ──────────────────────────────────────────────
            (State::Idle | State::Focused, Event::Press) => {
                Some(TransitionPlan::to(State::Pressed).apply(|_ctx| {}))
            }
            (State::Pressed, Event::Release) => {
                // Toggle the pressed state on release (mirrors physical button behavior).
                let new_pressed = !*ctx.pressed.get();
                let next_state = if ctx.focused { State::Focused } else { State::Idle };
                Some(TransitionPlan::to(next_state).apply(move |ctx| {
                    ctx.pressed.set(new_pressed);
                }))
            }
            // Touch devices: focus may arrive after press has already started.
            (State::Pressed, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }
            (State::Pressed, Event::Blur) => Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            // ── Toggle ──────────────────────────────────────────────────────
            // Toggle and SetPressed are intentionally handled from all states,
            // including Pressed. This allows programmatic control even while
            // the user is actively pressing the button.
            (_, Event::Toggle) => {
                let new_pressed = !*ctx.pressed.get();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.pressed.set(new_pressed);
                }))
            }

            // ── SetPressed ──────────────────────────────────────────────────
            (_, Event::SetPressed(value)) => {
                let value = *value;
                if *ctx.pressed.get() == value { return None; }
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.pressed.set(value);
                }))
            }

            // ── SetDisabled ─────────────────────────────────────────────────
            (_, Event::SetDisabled(disabled)) => {
                let d = *disabled;
                // If disabling while in Pressed or Focused, transition to Idle.
                let needs_idle = d && matches!(state, State::Pressed | State::Focused);
                if needs_idle {
                    Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                        ctx.disabled = d;
                        ctx.focused = false;
                        ctx.focus_visible = false;
                    }))
                } else {
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.disabled = d;
                    }))
                }
            }

            // ── Reset ───────────────────────────────────────────────────────
            (_, Event::Reset) => {
                let default = props.default_pressed;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.pressed.set(default);
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        let mut events = Vec::new();
        if new.disabled != old.disabled {
            events.push(Event::SetDisabled(new.disabled));
        }
        match (old.pressed, new.pressed) {
            (Some(false) | None, Some(true)) => events.push(Event::SetPressed(true)),
            (Some(true), Some(false)) => events.push(Event::SetPressed(false)),
            _ => {}
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

#### 1.5.1 Event Ordering Contract: Release/Blur

When both `Release` and `Blur` are pending in the same drain cycle, `Blur` MUST be processed
before `Release`. This prevents the button from briefly entering `Focused` state only to
immediately receive `Blur`. See `Button` spec section 2.1 for the full rationale.

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "toggle-button"]
pub enum Part {
    Root,
}

/// The API for the `ToggleButton` component.
pub struct Api<'a> {
    /// The current state of the toggle button.
    state: &'a State,
    /// The context of the toggle button.
    ctx: &'a Context,
    /// The props of the toggle button.
    props: &'a Props,
    /// The send function for the toggle button.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Returns true if the toggle button is in the pressed (on) state.
    pub fn is_pressed(&self) -> bool {
        *self.ctx.pressed.get()
    }

    /// Returns true if the toggle button is focused.
    pub fn is_focused(&self) -> bool {
        self.ctx.focused
    }

    /// Returns true if the toggle button has keyboard focus.
    pub fn is_focus_visible(&self) -> bool {
        self.ctx.focus_visible
    }

    /// Returns true if the toggle button is disabled.
    pub fn is_disabled(&self) -> bool {
        self.ctx.disabled
    }

    /// Props for the root `<button>` element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle => "idle",
            State::Focused => "focused",
            State::Pressed => "pressed",
        });
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), if self.is_pressed() { "true" } else { "false" });

        if self.ctx.disabled {
            // Use aria-disabled instead of the HTML disabled attribute so the
            // button remains in the tab order and discoverable by screen readers.
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.props.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if self.props.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if self.is_pressed() {
            attrs.set_bool(HtmlAttr::Data("ars-pressed"), true);
        }

        if let Some(ref value) = self.props.value {
            attrs.set(HtmlAttr::Data("ars-value"), value.as_str());
        }

        // Tabindex: when used inside a ToggleGroup with roving tabindex,
        // the group's item_attrs() overrides this value.
        attrs.set(HtmlAttr::TabIndex, "0");

        // Event handlers (focus, blur, press, release, keydown, keyup)
        // are typed methods on the Api struct.

        attrs
    }

    /// Hidden input configuration for native form submission (standalone use).
    /// Returns `None` when `name` is not set, the button is disabled, or the
    /// button is not pressed (matching checkbox semantics — absent from FormData
    /// when unchecked).
    pub fn hidden_input_config(&self) -> Option<HiddenInputConfig> {
        let name = self.props.name.as_ref()?;
        if self.ctx.disabled { return None; }
        if !self.is_pressed() { return None; }

        Some(HiddenInputConfig {
            name: name.clone(),
            value: HiddenInputValue::Single(
                self.props.value.clone().unwrap_or_else(|| "on".into()),
            ),
            form_id: self.props.form.clone(),
            disabled: self.ctx.disabled,
        })
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
toggle-button
  root    <button>    data-ars-scope="toggle-button" data-ars-part="root"
                      aria-pressed="true|false"
                      data-ars-state="idle|focused|pressed"
                      data-ars-pressed (present when toggled on)
```

| Part | Element    | Key Attributes                                                    |
| ---- | ---------- | ----------------------------------------------------------------- |
| Root | `<button>` | `aria-pressed`, `data-ars-state`, `type="button"`, `tabindex="0"` |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property        | Value                        |
| --------------- | ---------------------------- |
| `aria-pressed`  | `"true"` / `"false"` on Root |
| `aria-disabled` | Present when `disabled=true` |
| `aria-invalid`  | Present when `invalid=true`  |
| `aria-required` | Present when `required=true` |

- `aria-pressed="true|false"` communicates toggle state to screen readers.
- `type="button"` prevents form submission.
- **Disabled handling**: Uses `aria-disabled="true"` instead of the HTML `disabled` attribute so
  the button remains in the tab order and discoverable by screen readers. The state machine allows
  `Focus`/`Blur` events through when disabled to support this.
- **Focus ring**: `data-ars-focus-visible` is present only on keyboard focus. CSS should target
  `[data-ars-focus-visible]` to show the focus ring; suppress it on pointer focus.
- **Icon-only buttons**: If a `ToggleButton` has no visible text content, it MUST have an
  `aria-label` attribute providing a descriptive accessible name.
- **aria-describedby**: When an icon-only ToggleButton has an associated Tooltip, the adapter
  SHOULD wire `aria-describedby` to the tooltip's element ID.
- **Touch target**: Interactive toggle buttons MUST meet the minimum 44x44 CSS pixel touch target
  size (see foundation/03-accessibility.md section 7.1.1). Icon-only toggle buttons are especially at risk
  of undersizing.
- **Within ToggleGroup**: When used inside a `ToggleGroup`, the group's `item_attrs(value)` overrides
  the toggle button's ARIA attributes (e.g., `role="radio"` + `aria-checked` for single-selection
  mode) and manages roving tabindex. The toggle button defers to the group for these attributes.

### 3.2 Keyboard Interaction

| Key   | Action                                                     |
| ----- | ---------------------------------------------------------- |
| Enter | Toggles pressed state                                      |
| Space | Toggles pressed state (Press on keydown, Release on keyup) |
| Tab   | Moves focus to next focusable element                      |

### 3.3 Disabled Click Prevention

When `is_disabled()` returns true, the adapter MUST call `event.preventDefault()` on click and
keyboard activation events to prevent the native `<button>` from firing. Because the button uses
`aria-disabled` instead of the HTML `disabled` attribute (to remain in the tab order), the browser
does not natively suppress these events.

### 3.4 Native Element Handler Deduplication

Same as `Button` section 5.1: when rendering onto a native `<button>`, the adapter must strip
Space key handlers to prevent duplicate activation. See `Button` spec for full details.

### 3.5 Forced Colors Mode

In Windows High Contrast Mode (`@media (forced-colors: active)`), custom background colors are stripped.
The pressed/unpressed state MUST remain distinguishable via border, outline, or system colors:

```css
@media (forced-colors: active) {
    [data-ars-pressed="true"] {
        outline: 2px solid ButtonText;
        outline-offset: -2px;
    }
}
```

### 3.6 Screen Reader Test Protocol

| Action        | NVDA/JAWS                                                    | VoiceOver                                     |
| ------------- | ------------------------------------------------------------ | --------------------------------------------- |
| Tab to button | "[label], toggle button, pressed/not pressed"                | "[label], toggle button, pressed/not pressed" |
| Press Space   | "[label], toggle button, pressed/not pressed" (state change) | "[label], pressed/not pressed"                |
| When disabled | "[label], toggle button, dimmed/unavailable"                 | "[label], dimmed, toggle button"              |

## 4. Internationalization

ToggleButton has no localizable messages. The `aria-pressed` attribute communicates
toggle state to assistive technology directly; no additional label suffixes are needed.

## 5. Form Integration

- **Standalone**: When `name` is set, the `hidden_input_config()` method returns a
  `HiddenInputConfig` for a hidden `<input>` that submits with native HTML forms. The value
  is `props.value` (or `"on"` when no value prop is set) when pressed; when not pressed, the
  method returns `None` so the field is absent from `FormData` (matching checkbox semantics).
- **In ToggleGroup**: Form integration is managed by the group. The toggle button's `name` prop
  is ignored when used within a `ToggleGroup`.
- **No form participation by default**: Without a `name` prop, `ToggleButton` does not participate
  in form submission, matching `Toggle` behavior.
- **Validation**: `aria-invalid` and `aria-required` on the root element communicate validation
  state to assistive technology. The `invalid` and `required` props are typically set by the
  `Field` component wrapping the toggle button.
- **Required validation path**: When `required` is true and the toggle is unpressed, no hidden
  input is rendered, so native `required` validation will not fire. To enforce required validation,
  the adapter MUST register with `FormContext` using a `RequiredValidator`, or render the hidden
  `<input>` with the `required` attribute even when unpressed (with an empty value).
- **Form reset**: When a parent `<form>` is reset, the adapter sends `Event::Reset` to the
  machine, which restores `pressed` to `default_pressed`.
- **FieldCtx merge**: When used inside a `Field`, the adapter merges `disabled`/`invalid`/`required`
  from `FieldCtx` (per `07-forms.md` section 12.6).

## 6. Adapter Contract

### 6.1 Adapter Callback Props

Adapters MUST expose these callback props to consumers:

| Prop              | Type                               | Description                                           |
| ----------------- | ---------------------------------- | ----------------------------------------------------- |
| `on_change`       | `Option<Callback<bool>>`           | Fires after pressed state changes, with the new value |
| `on_press_start`  | `Option<Callback<PressEventData>>` | Fires on Press event                                  |
| `on_press_end`    | `Option<Callback<PressEventData>>` | Fires on Release event                                |
| `on_press_change` | `Option<Callback<bool>>`           | Fires when press state changes (is_pressing)          |
| `on_press_up`     | `Option<Callback<PressEventData>>` | Fires on pointer/key up                               |

Where `PressEventData` is defined in `05-interactions.md`.

### 6.2 ToggleGroup Context Override

When rendered inside a `ToggleGroup`, the adapter uses the group's `item_attrs(value)` for the
root element and ignores the `ToggleButton`'s own `root_attrs()`. The `ToggleButton` machine
still runs for focus/press state tracking, but its connect API attrs are not rendered directly.

### 6.3 Hidden Input SSR

The hidden `<input>` element MUST be rendered in SSR HTML when `name` is set and `disabled` is
false. This ensures progressive enhancement -- forms submit correctly even before JavaScript loads.

In Dioxus fullstack, the hidden `<input>` MUST render during the server component pass (not deferred to `use_effect`). The `hidden_input_config()` call is pure -- it reads `ctx` and `props` with no side effects -- so it is safe to call during SSR.

### 6.4 Controlled/Uncontrolled Mode Switching

Switching from uncontrolled (`pressed: None`) to controlled (`pressed: Some(false)`) mode at
runtime is not supported. The `Bindable` created during `init()` retains its original mode. If
the consumer needs to switch modes, the component must be unmounted and remounted.

## 7. Library Parity

> Compared against: React Aria (`ToggleButton`).

### 7.1 Props

| Feature                | ars-ui                      | React Aria                | Notes                                |
| ---------------------- | --------------------------- | ------------------------- | ------------------------------------ |
| Controlled pressed     | `pressed: Option<bool>`     | `isSelected`              | Same concept, different naming       |
| Default pressed        | `default_pressed`           | `defaultSelected`         | Same concept                         |
| Disabled               | `disabled`                  | `isDisabled`              | Both libraries                       |
| Invalid                | `invalid`                   | --                        | ars-ui addition for form integration |
| Required               | `required`                  | --                        | ars-ui addition for form integration |
| Value (for groups)     | `value: Option<String>`     | via `id`                  | RA uses `id` as key within group     |
| Form name              | `name`                      | --                        | ars-ui addition                      |
| Prevent focus on press | `prevent_focus_on_press`    | `preventFocusOnPress`     | Both libraries                       |
| Hover callbacks        | `on_hover_start/end/change` | `onHoverStart/End/Change` | Both libraries                       |

**Gaps:** None.

### 7.2 Anatomy

| Part | ars-ui | React Aria     | Notes               |
| ---- | ------ | -------------- | ------------------- |
| Root | `Root` | `ToggleButton` | Single-part in both |

**Gaps:** None.

### 7.3 Events

| Callback     | ars-ui                                  | React Aria                           | Notes          |
| ------------ | --------------------------------------- | ------------------------------------ | -------------- |
| Change       | `Bindable` change / adapter `on_change` | `onChange`                           | Both libraries |
| Press events | `on_press_start/end/change/up`          | `onPress/onPressStart/End/Change/Up` | Both libraries |
| Hover events | `on_hover_start/end/change`             | `onHoverStart/End/Change`            | Both libraries |

**Gaps:** None.

### 7.4 Features

| Feature                  | ars-ui | React Aria |
| ------------------------ | ------ | ---------- |
| Toggle state             | Yes    | Yes        |
| Controlled/uncontrolled  | Yes    | Yes        |
| Focus-visible            | Yes    | Yes        |
| Press interaction states | Yes    | Yes        |
| Form integration         | Yes    | --         |
| ToggleGroup composition  | Yes    | Yes        |

**Gaps:** None.

### 7.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui adds form integration (`name`, `invalid`, `required`, hidden input) not present in React Aria's ToggleButton.
- **Recommended additions:** None.
