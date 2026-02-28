---
component: Button
category: utility
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: [toggle-button]
references:
  react-aria: Button
---

# Button

A fundamental interactive control. The `Button` is the most commonly used interactive element in any UI library. Its state machine handles loading states, focus visibility discrimination (keyboard vs pointer), and pressed states. All other interactive components that use a button element internally ([`Toggle`](toggle.md), [`Accordion`](../navigation/accordion.md) trigger, [`Dialog`](../overlay/dialog.md) close button, etc.) share this base pattern.

## 1. State Machine

### 1.1 States

| State     | Description                                                              |
| --------- | ------------------------------------------------------------------------ |
| `Idle`    | Default resting state. Not focused, not pressed.                         |
| `Focused` | The button has received focus. Keyboard focus also sets `focus_visible`. |
| `Pressed` | The button is actively being pressed (pointer held down or Space held).  |
| `Loading` | The button is in a loading state. Interaction is disabled.               |

### 1.2 Events

| Event         | Payload             | Description                                                                                                         |
| ------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `Focus`       | `is_keyboard: bool` | Focus received; flag indicates keyboard vs pointer source.                                                          |
| `Blur`        | —                   | Focus lost; resets `focus_visible`.                                                                                 |
| `Press`       | —                   | Pointer/key press begins (pointerdown or keydown Space).                                                            |
| `Release`     | —                   | Pointer/key press ends (pointerup or keyup Space).                                                                  |
| `Click`       | —                   | The button was activated (click or Enter key). **Notification-only** — does not change state. See transition table. |
| `SetLoading`  | `bool`              | Programmatically enter or exit loading state.                                                                       |
| `SetDisabled` | `bool`              | Programmatically set the disabled state.                                                                            |

### 1.3 Context

```rust
use ars_core::{Bindable};

/// The context for the `Button` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    /// Whether the button is disabled.
    pub disabled: bool,
    /// Whether the button is in a loading state.
    pub loading: bool,
    /// Whether the button is pressed.
    pub pressed: bool,
    /// Whether the button is focused.
    pub focused: bool,
    /// Whether the button has keyboard focus.
    pub focus_visible: bool,
    /// The active locale, inherited from ArsProvider context.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// Props for the `Button` component.
#[derive(Clone, Debug, PartialEq, Eq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Whether the button is disabled.
    pub disabled: bool,
    /// Whether the button is in a loading state.
    pub loading: bool,
    /// Consumer-provided variant string (e.g. "primary", "destructive"). Headless: no built-in enum.
    pub variant: Option<String>,
    /// Consumer-provided size string (e.g. "sm", "lg", "icon"). Headless: no built-in enum.
    pub size: Option<String>,
    /// The HTML button type. Defaults to "button" to prevent accidental form submission.
    pub r#type: Type,
    /// The id of the form this button is associated with.
    pub form: Option<String>,
    /// The name submitted with form data.
    pub name: Option<String>,
    /// The value submitted with form data.
    pub value: Option<String>,
    /// When true, renders props onto the child element instead of a <button>.
    pub as_child: bool,
    /// Locale override. When `None`, inherits from nearest `ArsProvider` context.
    pub locale: Option<Locale>,
    /// When true, removes the button from sequential Tab navigation (sets `tabindex="-1"`).
    pub exclude_from_tab_order: bool,
    /// Form override: specifies the URL for form submission.
    pub form_action: Option<String>,
    /// Form override: HTTP method for submission.
    pub form_method: Option<String>,
    /// Form override: encoding type.
    pub form_enc_type: Option<String>,
    /// Form override: browsing context for form response.
    pub form_target: Option<String>,
    /// Form override: bypass form validation on submit.
    pub form_no_validate: bool,
    /// Auto-focus the button on mount.
    pub auto_focus: bool,
    /// When true, the adapter suppresses focus events on pointer press.
    pub prevent_focus_on_press: bool,
    /// Localizable messages for the button. When `None`, resolved via `resolve_messages()`.
    pub messages: Option<Messages>,
}

/// The type of the button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Button,
    Submit,
    Reset,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            disabled: false,
            loading: false,
            variant: None,
            size: None,
            r#type: Type::Button,
            form: None,
            name: None,
            value: None,
            as_child: false,
            locale: None,
            exclude_from_tab_order: false,
            form_action: None,
            form_method: None,
            form_enc_type: None,
            form_target: None,
            form_no_validate: false,
            auto_focus: false,
            prevent_focus_on_press: false,
            messages: None,
        }
    }
}
```

### 1.5 Event Ordering Contract: Release/Blur

When both `Release` and `Blur` are pending in the same drain cycle (e.g., the user clicks outside the button while the pointer is still down), `Blur` MUST be processed before `Release`. This prevents the button from briefly entering `Focused` state (via `Pressed → Release → Focused`) only to immediately receive `Blur`, which would produce a redundant `Focused → Idle` transition and associated effect setup/cleanup churn.

**Required ordering:** `Blur` before `Release` within a single drain cycle.

Alternatively, the adapter MAY skip effect setup for intermediate states within a single drain cycle. If the drain loop processes `[Release, Blur]` and the intermediate state (`Focused`) is immediately superseded by the final state (`Idle`), the adapter SHOULD only set up effects for `Idle`, not for the transient `Focused` state.

> **Cross-reference:** Effects are tagged with the state that produced them, and the adapter discards effects from states that were superseded within the same drain cycle.

### 1.6 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap};

/// The states for the `Button` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Focused,
    Pressed,
    Loading,
}

/// The events for the `Button` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Focus { is_keyboard: bool },
    Blur,
    Press,
    Release,
    Click,
    SetLoading(bool),
    SetDisabled(bool),
}

/// The machine for the `Button` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let initial_state = if props.loading {
            State::Loading
        } else {
            State::Idle
        };

        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        let ctx = Context {
            disabled: props.disabled,
            loading: props.loading,
            pressed: false,
            focused: false,
            focus_visible: false,
            locale,
            messages,
        };

        (initial_state, ctx)
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        _props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled && !matches!(event, Event::Focus { .. } | Event::Blur | Event::SetDisabled(_) | Event::SetLoading(_)) {
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
            (State::Focused, Event::Blur) => Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            // ── Press / Release ──────────────────────────────────────────────
            (State::Idle | State::Focused, Event::Press) => {
                Some(TransitionPlan::to(State::Pressed).apply(|ctx| {
                    ctx.pressed = true;
                }))
            }
            (State::Pressed, Event::Release) => {
                Some(TransitionPlan::to(if ctx.focused {
                    State::Focused
                } else {
                    State::Idle
                }).apply(|ctx| {
                    ctx.pressed = false;
                }))
            }
            (State::Pressed, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }
            (State::Pressed, Event::Blur) => Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                ctx.pressed = false;
                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            // ── Loading ──────────────────────────────────────────────────────
            (State::Loading, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }
            (State::Loading, Event::Blur) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            (_, Event::SetLoading(true)) => Some(TransitionPlan::to(State::Loading).apply(|ctx| {
                ctx.loading = true;
                ctx.pressed = false;
            })),
            (State::Loading, Event::SetLoading(false)) => Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                ctx.loading = false;
            })),

            // Click is notification-only — no state change.
            (_, Event::Click) => None,

            // ── Disabled ────────────────────────────────────────────────────
            (State::Pressed, Event::SetDisabled(true)) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.disabled = true;
                    ctx.pressed = false;
                    ctx.focused = false;
                }))
            }
            (_, Event::SetDisabled(disabled)) => {
                let d = *disabled;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.disabled = d;
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        let mut events = Vec::new();
        if new.loading != old.loading {
            events.push(Event::SetLoading(new.loading));
        }
        if new.disabled != old.disabled {
            events.push(Event::SetDisabled(new.disabled));
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

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "button"]
pub enum Part {
    Root,
    LoadingIndicator,
    Content,
}

/// The API for the `Button` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn is_loading(&self) -> bool {
        matches!(self.state, State::Loading)
    }

    pub fn is_disabled(&self) -> bool {
        self.ctx.disabled || self.ctx.loading
    }

    pub fn is_focus_visible(&self) -> bool {
        self.ctx.focus_visible
    }

    pub fn is_pressed(&self) -> bool {
        self.ctx.pressed
    }

    pub fn should_prevent_focus_on_press(&self) -> bool {
        self.props.prevent_focus_on_press
    }

    /// Root <button> element attributes.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);

        let type_str = match self.props.r#type {
            Type::Button => "button",
            Type::Submit => "submit",
            Type::Reset => "reset",
        };
        p.set(HtmlAttr::Type, type_str);

        if let Some(ref variant) = self.props.variant {
            p.set(HtmlAttr::Data("ars-variant"), variant.as_str());
        }
        if let Some(ref size) = self.props.size {
            p.set(HtmlAttr::Data("ars-size"), size.as_str());
        }

        if self.is_loading() {
            p.set_bool(HtmlAttr::Data("ars-loading"), true);
            p.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            p.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
            let loading_text = (self.ctx.messages.loading_label)(&self.ctx.locale);
            if !loading_text.is_empty() {
                p.set(HtmlAttr::Aria(AriaAttr::Label), loading_text);
            }
        } else if self.ctx.disabled {
            p.set_bool(HtmlAttr::Disabled, true);
            p.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.focus_visible {
            p.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        if self.ctx.pressed {
            p.set_bool(HtmlAttr::Data("ars-pressed"), true);
        }

        if let Some(ref form) = self.props.form {
            p.set(HtmlAttr::Form, form.as_str());
        }
        if let Some(ref name) = self.props.name {
            p.set(HtmlAttr::Name, name.as_str());
        }
        if let Some(ref value) = self.props.value {
            p.set(HtmlAttr::Value, value.as_str());
        }
        if self.props.exclude_from_tab_order {
            p.set(HtmlAttr::TabIndex, "-1");
        }
        if let Some(ref form_action) = self.props.form_action {
            p.set(HtmlAttr::FormAction, form_action.as_str());
        }
        if let Some(ref form_method) = self.props.form_method {
            p.set(HtmlAttr::FormMethod, form_method.as_str());
        }
        if let Some(ref form_enc_type) = self.props.form_enc_type {
            p.set(HtmlAttr::FormEncType, form_enc_type.as_str());
        }
        if let Some(ref form_target) = self.props.form_target {
            p.set(HtmlAttr::FormTarget, form_target.as_str());
        }
        if self.props.form_no_validate {
            p.set_bool(HtmlAttr::FormNoValidate, true);
        }
        if self.props.auto_focus {
            p.set_bool(HtmlAttr::AutoFocus, true);
        }
        if self.props.prevent_focus_on_press {
            p.set(HtmlAttr::Data("ars-prevent-focus-on-press"), "true");
        }

        #[cfg(debug_assertions)]
        if !p.has(HtmlAttr::Aria(AriaAttr::Label)) && !p.has(HtmlAttr::Aria(AriaAttr::LabelledBy)) {
            // Adapters check whether the button has text content children.
            // If no text content and no aria-label/aria-labelledby is provided,
            // emit: "Button has no accessible name. Provide `aria-label` for icon-only buttons."
        }

        p
    }

    /// Loading indicator element attributes (shown during loading state).
    pub fn loading_indicator_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::LoadingIndicator.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        p
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::LoadingIndicator => self.loading_indicator_attrs(),
            Part::Content => AttrMap::new(), // Content is a slot, no special attrs
        }
    }
}
```

## 2. Anatomy

```text
Button
├── Root               <button>  data-ars-scope="button" data-ars-part="root"
├── LoadingIndicator   <span>    data-ars-part="loading-indicator" (when loading)
└── Content            <span>    data-ars-part="content" (label text / icons slot)
```

| Part             | Element    | Key Attributes                                                  |
| ---------------- | ---------- | --------------------------------------------------------------- |
| Root             | `<button>` | `data-ars-scope="button"`, `type`, `aria-disabled`, `aria-busy` |
| LoadingIndicator | `<span>`   | `data-ars-part="loading-indicator"`, `aria-hidden="true"`       |
| Content          | `<span>`   | `data-ars-part="content"` (label slot)                          |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property                 | Value                                                 |
| ------------------------ | ----------------------------------------------------- |
| Role                     | `button` (implicit on `<button>`)                     |
| `type`                   | `"button"` (default, prevents accidental form submit) |
| `aria-disabled`          | `"true"` when loading or disabled                     |
| `aria-busy`              | `"true"` when loading                                 |
| `aria-label`             | Loading text when in loading state                    |
| `data-ars-focus-visible` | Present only on keyboard focus                        |

- The default element is `<button>`, which has implicit `role="button"`. Never use a `<div>` or `<a>` as a button without explicit ARIA.
- **Loading state**: Use `aria-disabled="true"` (not `disabled`) to keep the button in the tab order, allowing screen reader users to discover it and hear "Loading…" in the accessible name.
- **Destructive variant**: No additional ARIA needed. The destructive nature must be communicated in the button label (e.g., "Delete account" not just "Confirm").
- **Keyboard**: Native `<button>` activates on both Enter and Space. The machine mirrors this — Space fires Press/Release events; Enter fires Click directly.
- **Icon-only buttons**: Must have an `aria-label` providing a descriptive accessible name (e.g., "Close dialog", "Delete item").
- Buttons MUST meet the minimum 44x44 CSS pixel touch target size (see foundation/03-accessibility.md §7.1.1).
- **Disabled and tab order**: The `disabled` HTML attribute removes the button from the tab order. For screen reader discoverability, the loading state uses `aria-disabled="true"` with manual event prevention.

### 3.2 Forced Colors Mode

Loading indicator visual elements MUST use `currentColor` or `forced-color-adjust: auto` to remain visible in Windows High Contrast Mode.

### 3.3 Native Element Handler Deduplication

When the `Button` component renders onto a native `<button>` element (the default), the framework adapter **must strip** the Space key `keydown`/`keyup` handlers from the `AttrMap` before applying them to the DOM. Native `<button>` elements already synthesize `click` events from Space key presses, so attaching the machine's Space handlers would cause duplicate activation. The adapter should detect this by checking whether the target element is a `<button>` and, if so, omit handlers whose sole purpose is Space key handling. When rendering via `as_child` onto a non-button element (e.g., `<div role="button">`), all keyboard handlers must be preserved.

> **Adapter Note:** Native `<button>` elements handle Enter/Space natively. Adapters must deduplicate keyboard handlers to avoid double-firing.

## 4. Internationalization

### 4.1 Messages

```rust
/// Localizable strings for the `Button` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label applied when the button is in a loading state.
    pub loading_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            loading_label: MessageFn::static_str("Loading"),
        }
    }
}

impl ComponentMessages for Messages {}
```

- RTL layouts: `Button` content (icon + text) should reverse naturally via `dir="rtl"` on the document root. No special `Button`-level handling is required.

### 4.2 Adapter Callback Props

Adapters MUST expose these callback props:

| Prop              | Type                               | Description                    |
| ----------------- | ---------------------------------- | ------------------------------ |
| `on_press_start`  | `Option<Callback<PressEventData>>` | Fires on Press event           |
| `on_press_end`    | `Option<Callback<PressEventData>>` | Fires on Release event         |
| `on_press_change` | `Option<Callback<bool>>`           | Fires when press state changes |
| `on_press_up`     | `Option<Callback<PressEventData>>` | Fires on pointer/key up        |

Where `PressEventData` is defined in `05-interactions.md` and includes `pointer_type`, `shift_key`, `ctrl_key`, `meta_key`, `alt_key`.

### 4.3 Adapter Contract: Loading + Submit Prevention

When `type="submit"` and `is_loading()` is true, the root element has `aria-disabled="true"` but NOT the HTML `disabled` attribute. However, `aria-disabled` does **not** prevent native `<button type="submit">` from submitting the form.

**Adapters MUST** call `event.preventDefault()` on the native `submit` event when `is_loading()` returns true, to prevent double-submission. Similarly, when `type="reset"` and `is_loading()` is true, the adapter MUST prevent the native reset action if reset-during-loading is undesired.

### 4.4 Adapter Contract: Prevent Focus on Press

When `should_prevent_focus_on_press()` returns true, the adapter MUST call `event.preventDefault()` on the `pointerdown` event to suppress the browser's default focus behavior. This prevents the button from stealing focus when used inside composite widgets (e.g., ComboBox trigger button).

## 5. Library Parity

> Compared against: React Aria (`Button`).

### 5.1 Props

| Feature                | ars-ui                   | React Aria            | Notes                                   |
| ---------------------- | ------------------------ | --------------------- | --------------------------------------- |
| Disabled               | `disabled`               | `isDisabled`          | Both libraries                          |
| Loading/Pending        | `loading`                | `isPending`           | Both libraries; RA calls it `isPending` |
| Type                   | `r#type: Type`           | `type`                | Both libraries                          |
| Form                   | `form`                   | `form`                | Both libraries                          |
| Name                   | `name`                   | `name`                | Both libraries                          |
| Value                  | `value`                  | `value`               | Both libraries                          |
| Exclude from tab order | `exclude_from_tab_order` | `excludeFromTabOrder` | Both libraries                          |
| Prevent focus on press | `prevent_focus_on_press` | `preventFocusOnPress` | Both libraries                          |
| Auto focus             | `auto_focus`             | `autoFocus`           | Both libraries                          |
| as_child               | `as_child`               | `render`              | Different composition patterns          |
| Form action            | `form_action`            | `formAction`          | Both libraries                          |
| Form method            | `form_method`            | `formMethod`          | Both libraries                          |
| Form enc type          | `form_enc_type`          | `formEncType`         | Both libraries                          |
| Form target            | `form_target`            | `formTarget`          | Both libraries                          |
| Form no validate       | `form_no_validate`       | `formNoValidate`      | Both libraries                          |
| Variant                | `variant`                | --                    | ars-ui addition (headless pass-through) |
| Size                   | `size`                   | --                    | ars-ui addition (headless pass-through) |

**Gaps:** None.

### 5.2 Anatomy

| Part             | ars-ui             | React Aria | Notes                  |
| ---------------- | ------------------ | ---------- | ---------------------- |
| Root             | `Root`             | `Button`   | Both libraries         |
| LoadingIndicator | `LoadingIndicator` | --         | ars-ui structural part |
| Content          | `Content`          | --         | ars-ui structural part |

**Gaps:** None. ars-ui provides more granular anatomy.

### 5.3 Events

| Callback     | ars-ui                         | React Aria                           | Notes                                             |
| ------------ | ------------------------------ | ------------------------------------ | ------------------------------------------------- |
| Press events | `on_press_start/end/change/up` | `onPress/onPressStart/End/Change/Up` | Both libraries                                    |
| Hover events | Adapter-level                  | `onHoverStart/End/Change`            | RA exposes hover; ars-ui handles at adapter level |
| Focus events | Adapter-level                  | `onFocus/onBlur/onFocusChange`       | RA exposes focus; ars-ui handles at adapter level |
| Key events   | Adapter-level                  | `onKeyDown/onKeyUp`                  | RA exposes key events                             |

**Gaps:** None. Event handling is equivalent; ars-ui handles hover/focus/key at the adapter layer.

### 5.4 Features

| Feature                  | ars-ui | React Aria        |
| ------------------------ | ------ | ----------------- |
| Loading/Pending state    | Yes    | Yes               |
| Focus-visible            | Yes    | Yes               |
| Press tracking           | Yes    | Yes               |
| Disabled (aria-disabled) | Yes    | Yes               |
| Form submission          | Yes    | Yes               |
| as_child composition     | Yes    | Yes (render prop) |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui uses a `loading` boolean with explicit `State::Loading`; React Aria uses `isPending`. ars-ui adds `variant`/`size` pass-through props. React Aria exposes hover/focus/key callbacks as props; ars-ui handles these at the adapter layer.
- **Recommended additions:** None.
