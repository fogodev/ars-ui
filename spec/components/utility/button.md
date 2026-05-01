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
use ars_core::{Locale, MessageFn};

/// The context for the `Button` component.
#[derive(Clone, Debug, PartialEq)]
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
use ars_core::{SafeUrl, UnsafeUrlError};
use core::fmt::{self, Display};

/// Props for the `Button` component.
#[derive(Clone, Debug, PartialEq, Eq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Whether the button is disabled.
    pub disabled: bool,
    /// Whether the button is in a loading state.
    pub loading: bool,
    /// Visual style token exposed as `data-ars-variant`.
    pub variant: Variant,
    /// Visual size token exposed as `data-ars-size`.
    pub size: Size,
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
    /// When true, removes the button from sequential Tab navigation (sets `tabindex="-1"`).
    pub exclude_from_tab_order: bool,
    /// Form override: safe URL for form submission.
    pub form_action: Option<SafeUrl>,
    /// Form override: HTTP method for submission.
    pub form_method: Option<FormMethod>,
    /// Form override: encoding type.
    pub form_enc_type: Option<FormEncType>,
    /// Form override: browsing context for form response.
    pub form_target: Option<FormTarget>,
    /// Form override: bypass form validation on submit.
    pub form_no_validate: bool,
    /// Auto-focus the button on mount.
    pub auto_focus: bool,
    /// When true, the adapter suppresses focus events on pointer press.
    pub prevent_focus_on_press: bool,
}

/// Visual style token for the button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Variant {
    /// Default neutral button style.
    #[default]
    Default,
    /// Primary action style.
    Primary,
    /// Secondary action style.
    Secondary,
    /// Destructive or dangerous action style.
    Destructive,
    /// Outlined button style.
    Outline,
    /// Low-chrome ghost button style.
    Ghost,
    /// Link-like button style.
    Link,
}

impl Variant {
    /// Returns the data-attribute token for this variant.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Destructive => "destructive",
            Self::Outline => "outline",
            Self::Ghost => "ghost",
            Self::Link => "link",
        }
    }
}

impl Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Visual size token for the button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Size {
    /// Small button size.
    Sm,
    /// Medium button size.
    #[default]
    Md,
    /// Large button size.
    Lg,
    /// Icon-only button size.
    Icon,
}

impl Size {
    /// Returns the data-attribute token for this size.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
            Self::Icon => "icon",
        }
    }
}

impl Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The type of the button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Type {
    /// A non-submitting button.
    #[default]
    Button,
    /// A form submit button.
    Submit,
    /// A form reset button.
    Reset,
}

impl Type {
    /// Returns the HTML `type` token for this button type.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Submit => "submit",
            Self::Reset => "reset",
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Form submission method override for a submit button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FormMethod {
    /// Submit using HTTP GET.
    Get,
    /// Submit using HTTP POST.
    Post,
    /// Close an ancestor dialog without network submission.
    Dialog,
}

impl FormMethod {
    /// Returns the HTML `formmethod` token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Post => "post",
            Self::Dialog => "dialog",
        }
    }
}

impl Display for FormMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Form encoding type override for a submit button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FormEncType {
    /// Standard URL-encoded form body.
    UrlEncoded,
    /// Multipart form body, typically for file uploads.
    MultipartFormData,
    /// Plain text form body.
    TextPlain,
}

impl FormEncType {
    /// Returns the HTML `formenctype` token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UrlEncoded => "application/x-www-form-urlencoded",
            Self::MultipartFormData => "multipart/form-data",
            Self::TextPlain => "text/plain",
        }
    }
}

impl Display for FormEncType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Browsing context override for a form submit response.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FormTarget {
    /// Submit into the current browsing context.
    Self_,
    /// Submit into a new browsing context.
    Blank,
    /// Submit into the parent browsing context.
    Parent,
    /// Submit into the top-level browsing context.
    Top,
    /// Submit into a named browsing context.
    Named(String),
}

impl FormTarget {
    /// Returns the HTML `formtarget` token.
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Self_ => "_self",
            Self::Blank => "_blank",
            Self::Parent => "_parent",
            Self::Top => "_top",
            Self::Named(name) => name.as_str(),
        }
    }
}

impl Display for FormTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            disabled: false,
            loading: false,
            variant: Variant::Default,
            size: Size::Md,
            r#type: Type::Button,
            form: None,
            name: None,
            value: None,
            as_child: false,
            exclude_from_tab_order: false,
            form_action: None,
            form_method: None,
            form_enc_type: None,
            form_target: None,
            form_no_validate: false,
            auto_focus: false,
            prevent_focus_on_press: false,
        }
    }
}

impl Props {
    /// Returns a fresh `Props` with every field at its `Default` value.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `id`.
    pub fn id(mut self, id: impl Into<String>) -> Self { self.id = id.into(); self }
    /// Sets `disabled`.
    pub const fn disabled(mut self, value: bool) -> Self { self.disabled = value; self }
    /// Sets `loading`.
    pub const fn loading(mut self, value: bool) -> Self { self.loading = value; self }
    /// Sets `variant`.
    pub const fn variant(mut self, variant: Variant) -> Self { self.variant = variant; self }
    /// Sets `size`.
    pub const fn size(mut self, size: Size) -> Self { self.size = size; self }
    /// Sets `type`.
    pub const fn button_type(mut self, ty: Type) -> Self { self.r#type = ty; self }
    /// Sets `form`.
    pub fn form(mut self, form: impl Into<String>) -> Self { self.form = Some(form.into()); self }
    /// Sets `name`.
    pub fn name(mut self, name: impl Into<String>) -> Self { self.name = Some(name.into()); self }
    /// Sets `value`.
    pub fn value(mut self, value: impl Into<String>) -> Self { self.value = Some(value.into()); self }
    /// Sets `as_child`.
    pub const fn as_child(mut self, value: bool) -> Self { self.as_child = value; self }
    /// Sets `exclude_from_tab_order`.
    pub const fn exclude_from_tab_order(mut self, value: bool) -> Self { self.exclude_from_tab_order = value; self }
    /// Sets `form_action` from an already validated URL.
    pub fn form_action(mut self, action: SafeUrl) -> Self { self.form_action = Some(action); self }
    /// Validates and sets `form_action`.
    pub fn try_form_action(mut self, action: impl Into<String>) -> Result<Self, UnsafeUrlError> {
        self.form_action = Some(SafeUrl::new(action)?);
        Ok(self)
    }
    /// Sets `form_method`.
    pub const fn form_method(mut self, method: FormMethod) -> Self { self.form_method = Some(method); self }
    /// Sets `form_enc_type`.
    pub const fn form_enc_type(mut self, enc_type: FormEncType) -> Self { self.form_enc_type = Some(enc_type); self }
    /// Sets `form_target`.
    pub fn form_target(mut self, target: FormTarget) -> Self { self.form_target = Some(target); self }
    /// Sets `form_no_validate`.
    pub const fn form_no_validate(mut self, value: bool) -> Self { self.form_no_validate = value; self }
    /// Sets `auto_focus`.
    pub const fn auto_focus(mut self, value: bool) -> Self { self.auto_focus = value; self }
    /// Sets `prevent_focus_on_press`.
    pub const fn prevent_focus_on_press(mut self, value: bool) -> Self { self.prevent_focus_on_press = value; self }
}
```

`Props::new()` is the documented entry point for component construction. The
chainable setters mirror the standardized builder pattern used by
`Dismissable`: setters consume and return `Self`, boolean setters are `const`
where possible, string setters accept `impl Into<String>`, and callback-free
configuration does not require `Some(...)` boilerplate. Submit behavior is
explicit: `Type::Button` is the default to avoid accidental form submission;
consumers opt into native submit behavior with `.button_type(Type::Submit)`.

### 1.5 Event Ordering Contract: Release/Blur

When both `Release` and `Blur` are pending in the same drain cycle (e.g., the user clicks outside the button while the pointer is still down), `Blur` MUST be processed before `Release`. This prevents the button from briefly entering `Focused` state (via `Pressed → Release → Focused`) only to immediately receive `Blur`, which would produce a redundant `Focused → Idle` transition and associated effect setup/cleanup churn.

**Required ordering:** `Blur` before `Release` within a single drain cycle.

Alternatively, the adapter MAY skip effect setup for intermediate states within a single drain cycle. If the drain loop processes `[Release, Blur]` and the intermediate state (`Focused`) is immediately superseded by the final state (`Idle`), the adapter SHOULD only set up effects for `Idle`, not for the transient `Focused` state.

> **Cross-reference:** Effects are tagged with the state that produced them, and the adapter discards effects from states that were superseded within the same drain cycle.

### 1.6 Full Machine Implementation

```rust
use ars_core::{AttrMap, TransitionPlan, sanitize_url};
use core::fmt::{self, Display};

/// The states for the `Button` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Focused,
    Pressed,
    Loading,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => f.write_str("idle"),
            Self::Focused => f.write_str("focused"),
            Self::Pressed => f.write_str("pressed"),
            Self::Loading => f.write_str("loading"),
        }
    }
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
    type Messages = Messages;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let initial_state = if props.loading {
            State::Loading
        } else {
            State::Idle
        };

        let locale = env.locale.clone();
        let messages = messages.clone();
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
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled && !matches!(event, Event::Blur | Event::SetDisabled(_) | Event::SetLoading(_)) {
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
            (State::Loading, Event::SetLoading(false)) => Some(TransitionPlan::to(if ctx.focused {
                State::Focused
            } else {
                State::Idle
            }).apply(|ctx| {
                ctx.loading = false;
            })),
            (_, Event::SetLoading(false)) => Some(TransitionPlan::context_only(|ctx| {
                ctx.loading = false;
            })),

            // Click is notification-only — no state change.
            (_, Event::Click) => None,

            // ── Disabled ────────────────────────────────────────────────────
            (State::Focused | State::Pressed, Event::SetDisabled(true)) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.disabled = true;
                    ctx.pressed = false;
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }
            (State::Loading, Event::SetDisabled(true)) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.disabled = true;
                    ctx.pressed = false;
                    ctx.focused = false;
                    ctx.focus_visible = false;
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
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
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
        matches!(self.state, State::Loading) || self.ctx.loading
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

    /// Dispatches a focus event.
    pub fn on_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Dispatches a blur event.
    pub fn on_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches a press event.
    pub fn on_press(&self) {
        (self.send)(Event::Press);
    }

    /// Dispatches a release event.
    pub fn on_release(&self) {
        (self.send)(Event::Release);
    }

    /// Dispatches a click event.
    pub fn on_click(&self) {
        (self.send)(Event::Click);
    }

    /// Root <button> element attributes.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Role, "button");
        p.set(HtmlAttr::Data("ars-state"), self.state.to_string());

        p.set(HtmlAttr::Type, self.props.r#type.as_str());
        p.set(HtmlAttr::Data("ars-variant"), self.props.variant.as_str());
        p.set(HtmlAttr::Data("ars-size"), self.props.size.as_str());

        if self.is_loading() {
            p.set_bool(HtmlAttr::Data("ars-loading"), true);
            p.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            p.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
        }

        if self.ctx.disabled {
            p.set_bool(HtmlAttr::Disabled, true);
            p.set_bool(HtmlAttr::Data("ars-disabled"), true);
            p.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
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
        if self.props.exclude_from_tab_order || (self.props.as_child && self.ctx.disabled) {
            p.set(HtmlAttr::TabIndex, "-1");
        } else if self.props.as_child && !self.ctx.disabled {
            p.set(HtmlAttr::TabIndex, "0");
        }
        if let Some(ref form_action) = self.props.form_action {
            p.set(HtmlAttr::FormAction, sanitize_url(form_action.as_str()));
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

        p
    }

    /// Loading indicator element attributes (shown during loading state).
    pub fn loading_indicator_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::LoadingIndicator.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        if self.is_loading() {
            p.set_bool(HtmlAttr::Data("ars-loading"), true);
            p.set(HtmlAttr::Role, "status");
            p.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
            let loading_text = (self.ctx.messages.loading_label)(&self.ctx.locale);
            if !loading_text.is_empty() {
                p.set(HtmlAttr::Aria(AriaAttr::Label), loading_text);
            }
        } else {
            p.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        }
        p
    }

    /// Content slot element attributes.
    pub fn content_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Data("ars-loading"), if self.is_loading() { "true" } else { "false" });
        p
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::LoadingIndicator => self.loading_indicator_attrs(),
            Part::Content => self.content_attrs(),
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

| Part             | Element    | Key Attributes                                                                                                         |
| ---------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------- |
| Root             | `<button>` | `data-ars-scope="button"`, `data-ars-state`, `data-ars-variant`, `data-ars-size`, `type`, `aria-disabled`, `aria-busy` |
| LoadingIndicator | `<span>`   | `data-ars-part="loading-indicator"`, `role="status"` and `aria-live="polite"` when loading                             |
| Content          | `<span>`   | `data-ars-part="content"`, `data-ars-loading="true\|false"`                                                            |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property                 | Value                                                          |
| ------------------------ | -------------------------------------------------------------- |
| Role                     | `button` (implicit on `<button>`, explicit in attrs)           |
| `type`                   | `"button"` (default, prevents accidental form submit)          |
| `aria-disabled`          | `"true"` when loading or disabled                              |
| `aria-busy`              | `"true"` when loading                                          |
| `aria-label`             | Not set by root loading state; preserve the button action name |
| `data-ars-focus-visible` | Present only on keyboard focus                                 |
| `data-ars-state`         | `"idle"`, `"focused"`, `"pressed"`, or `"loading"`             |
| `data-ars-variant`       | `Variant::as_str()` visual token                               |
| `data-ars-size`          | `Size::as_str()` visual token                                  |

- The default element is `<button>`, which has implicit `role="button"`. Never use a `<div>` or `<a>` as a button without explicit ARIA.
- **Loading state**: Use `aria-disabled="true"` (not `disabled`) to keep the button in the tab order. Preserve the button's action-oriented accessible name; loading progress is exposed through `aria-busy` on the root and the `LoadingIndicator` status part.
- **Destructive variant**: No additional ARIA needed. The destructive nature must be communicated in the button label (e.g., "Delete account" not just "Confirm").
- **Keyboard**: Native `<button>` activates on both Enter and Space. The machine mirrors this — Space fires Press/Release events; Enter fires Click directly.
- **Icon-only buttons**: Must have an `aria-label` providing a descriptive accessible name (e.g., "Close dialog", "Delete item").
- Buttons MUST meet the minimum 44x44 CSS pixel touch target size (see foundation/03-accessibility.md §7.1.1).
- **Disabled and tab order**: The `disabled` HTML attribute removes native buttons from the tab order. For `as_child` composition, the agnostic core emits `tabindex="-1"` while disabled so component-owned attrs remove non-button children from sequential keyboard navigation even when the child would otherwise be focusable. For screen reader discoverability, the loading state uses `aria-disabled="true"` with manual event prevention and remains tabbable unless explicitly excluded.

### 3.2 Accessible Name Diagnostics

The framework adapters, not the agnostic core, are responsible for accessible-name diagnostics. Adapters can inspect rendered children and merged consumer attributes; the core `root_attrs()` cannot know whether text content, `aria-label`, or `aria-labelledby` will be present after rendering. In debug builds, adapters SHOULD warn when a rendered button has no accessible name if a shared accessible-name inspection helper is available. Adapter implementations should not duplicate incomplete accessible-name computation locally. When emitted, use this wording:

```text
Button has no accessible name. Provide `aria-label` for icon-only buttons.
```

### 3.3 Forced Colors Mode

Loading indicator visual elements MUST use `currentColor` or `forced-color-adjust: auto` to remain visible in Windows High Contrast Mode.

### 3.4 Native Element Handler Deduplication

When the `Button` component renders onto a native `<button>` element (the default), the framework adapter must rely on native Enter/Space activation and must not attach duplicate Space key `keydown`/`keyup` activation handlers. When rendering via `as_child`, the first adapter contract forwards root attrs only; a consumer-owned non-button root is responsible for any additional keyboard activation handling until an adapter event-forwarding slot exists.

> **Adapter Note:** Native `<button>` elements handle Enter/Space natively. Adapters must deduplicate keyboard handlers to avoid double-firing.

### 3.5 `as_child` Native Attribute Filtering

The agnostic core always emits the complete root `AttrMap`, including native button and form attributes. Framework adapters that apply `root_attrs()` to a consumer child element MUST filter native-only attributes when the target element cannot legally receive them. For example, `type`, `form`, `formaction`, `formmethod`, `formenctype`, `formtarget`, `formnovalidate`, `name`, and `value` are valid on a native `<button>` but must not be applied blindly to a `<div role="button">`. Adapters SHOULD preserve behavioral and accessibility attrs such as `role`, `tabindex`, `aria-*`, and `data-ars-*` for non-button children.

## 4. Internationalization

### 4.1 Messages

```rust
/// Localizable strings for the `Button` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label applied to the loading indicator status part.
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

Native Button adapter components MUST expose these callback props:

| Prop              | Type                           | Description                    |
| ----------------- | ------------------------------ | ------------------------------ |
| `on_press_start`  | `Option<Callback<PressEvent>>` | Fires on Press event           |
| `on_press_end`    | `Option<Callback<PressEvent>>` | Fires on Release event         |
| `on_press`        | `Option<Callback<PressEvent>>` | Fires on activation            |
| `on_press_change` | `Option<Callback<bool>>`       | Fires when press state changes |
| `on_press_up`     | `Option<Callback<PressEvent>>` | Fires on pointer/key up        |

Where `PressEvent` is re-exported from `ars-interactions` and includes normalized pointer type, event type, coordinates when available, modifier state, and propagation control. `as_child` adapter components are attr-forwarding only until the shared as-child contract grows event-handler forwarding.

### 4.3 Adapter Contract: Loading + Submit Prevention

When `type="submit"` and `is_loading()` is true, the root element has `aria-disabled="true"` but NOT the HTML `disabled` attribute. However, `aria-disabled` does **not** prevent native `<button type="submit">` from submitting the form.

Adapters must call `event.preventDefault()` on the activation event that would trigger native submit/reset when `is_loading()` returns true. For native `<button>` roots this can be handled on the click event before the browser performs the default submit or reset action.

### 4.4 Adapter Contract: Prevent Focus on Press

When `should_prevent_focus_on_press()` returns true, the adapter MUST call `event.preventDefault()` on the `pointerdown` event to suppress the browser's default focus behavior. This prevents the button from stealing focus when used inside composite widgets (e.g., ComboBox trigger button).

## 5. Library Parity

> Compared against: React Aria (`Button`).

### 5.1 Props

| Feature                | ars-ui                       | React Aria            | Notes                                   |
| ---------------------- | ---------------------------- | --------------------- | --------------------------------------- |
| Disabled               | `disabled`                   | `isDisabled`          | Both libraries                          |
| Loading/Pending        | `loading`                    | `isPending`           | Both libraries; RA calls it `isPending` |
| Type                   | `r#type: Type`               | `type`                | Both libraries                          |
| Form                   | `form`                       | `form`                | Both libraries                          |
| Name                   | `name`                       | `name`                | Both libraries                          |
| Value                  | `value`                      | `value`               | Both libraries                          |
| Exclude from tab order | `exclude_from_tab_order`     | `excludeFromTabOrder` | Both libraries                          |
| Prevent focus on press | `prevent_focus_on_press`     | `preventFocusOnPress` | Both libraries                          |
| Auto focus             | `auto_focus`                 | `autoFocus`           | Both libraries                          |
| as_child               | `as_child`                   | `render`              | Different composition patterns          |
| Form action            | `form_action: SafeUrl`       | `formAction`          | ars-ui validates before attr emission   |
| Form method            | `form_method: FormMethod`    | `formMethod`          | ars-ui uses a closed enum vocabulary    |
| Form enc type          | `form_enc_type: FormEncType` | `formEncType`         | ars-ui uses a closed enum vocabulary    |
| Form target            | `form_target: FormTarget`    | `formTarget`          | Known targets are enum variants         |
| Form no validate       | `form_no_validate`           | `formNoValidate`      | Both libraries                          |
| Variant                | `variant: Variant`           | --                    | ars-ui typed visual token               |
| Size                   | `size: Size`                 | --                    | ars-ui typed visual token               |

**Gaps:** None.

### 5.2 Anatomy

| Part             | ars-ui             | React Aria | Notes                  |
| ---------------- | ------------------ | ---------- | ---------------------- |
| Root             | `Root`             | `Button`   | Both libraries         |
| LoadingIndicator | `LoadingIndicator` | --         | ars-ui structural part |
| Content          | `Content`          | --         | ars-ui structural part |

**Gaps:** None. ars-ui provides more granular anatomy.

### 5.3 Events

| Callback     | ars-ui                                  | React Aria                           | Notes                                                                  |
| ------------ | --------------------------------------- | ------------------------------------ | ---------------------------------------------------------------------- |
| Press events | `on_press/on_press_start/end/change/up` | `onPress/onPressStart/End/Change/Up` | Native `Button` has parity; `as_child` awaits event-handler forwarding |
| Hover events | Adapter-level                           | `onHoverStart/End/Change`            | RA exposes hover; ars-ui handles at adapter level                      |
| Focus events | Adapter-level                           | `onFocus/onBlur/onFocusChange`       | RA exposes focus; ars-ui handles at adapter level                      |
| Key events   | Adapter-level                           | `onKeyDown/onKeyUp`                  | RA exposes key events                                                  |

**Gaps:** `as_child` event-handler forwarding is intentionally not exposed yet. Native `Button` event handling is equivalent for the press surface ars-ui supports; hover/focus/key callbacks are adapter-owned internals rather than public Button props.

### 5.4 Features

| Feature                  | ars-ui | React Aria        |
| ------------------------ | ------ | ----------------- |
| Loading/Pending state    | Yes    | Yes               |
| Focus-visible            | Yes    | Yes               |
| Press tracking           | Yes    | Yes               |
| Disabled (aria-disabled) | Yes    | Yes               |
| Form submission          | Yes    | Yes               |
| as_child composition     | Yes    | Yes (render prop) |

**Gaps:** `as_child` currently forwards attrs only; consumers own any non-native keyboard activation and event handler wiring on the reassigned root.

### 5.5 Summary

- **Overall:** Native Button press/form/loading behavior has parity with the supported React Aria Button surface.
- **Divergences:** ars-ui uses a `loading` boolean with explicit `State::Loading`; React Aria uses `isPending`. ars-ui adds typed `Variant`/`Size` visual tokens. React Aria exposes hover/focus/key callbacks as props; ars-ui handles these at the adapter layer.
- **Recommended additions:** Add shared as-child event-handler forwarding before exposing press callbacks on `ButtonAsChild`.
