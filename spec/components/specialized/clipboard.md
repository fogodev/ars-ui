---
component: Clipboard
category: specialized
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
    ark-ui: Clipboard
---

# Clipboard

A `Clipboard` component copies text to the system clipboard and provides visual/audible
feedback. It wraps the `navigator.clipboard.writeText()` API with a state machine
that tracks the copy lifecycle.

> **Security:** The component uses `navigator.clipboard.writeText()` only (write-only). This requires a secure context (HTTPS) and transient user activation (user gesture). Reading from the clipboard is NOT supported. For iframe usage, the `allow="clipboard-write"` permission policy is required. If `navigator.clipboard` is unavailable (pre-2019 browsers, non-HTTPS, or permission denied), the adapter layer falls back to the legacy `document.execCommand("copy")` approach, or the machine enters `Error` state. Clipboard write operations that do not resolve within **5 seconds** are aborted with a `Timeout` error.

```rust
/// Why a clipboard copy operation failed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CopyFailureReason {
    /// User denied clipboard access.
    PermissionDenied,
    /// Not HTTPS — the Clipboard API requires a secure context.
    NotSecureContext,
    /// Clipboard operation exceeded the 5-second timeout.
    Timeout,
    /// Neither `navigator.clipboard` nor legacy `execCommand("copy")` is available.
    ApiUnavailable,
    /// Unexpected error from the browser API.
    Unknown(String),
}
```

## 1. State Machine

### 1.1 States

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Waiting for user to initiate a copy.
    Idle,
    /// The copy operation is in progress.
    Copying,
    /// Copy succeeded; showing success feedback.
    Copied,
    /// Copy failed.
    Error,
}
```

### 1.2 Events

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// User triggered a copy.
    Copy,
    /// Copy succeeded.
    CopySuccess,
    /// Copy failed with a structured reason.
    CopyError(CopyFailureReason),
    /// Feedback timeout expired; return to idle.
    ResetTimeout,
}
```

### 1.3 Context

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The text to copy. Supports controlled/uncontrolled binding via `Bindable`.
    pub value: Bindable<String>,
    /// How long to show the "copied" feedback (in ms).
    pub feedback_duration_ms: u32,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// The reason the last copy failed (`None` when not in error state).
    pub error: Option<CopyFailureReason>,
    /// Locale for internationalized messages.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Component instance IDs.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// The text to copy to the clipboard. Supports controlled/uncontrolled binding.
    pub value: Option<String>,
    /// Default value for uncontrolled mode.
    pub default_value: String,
    /// Duration to show "copied" feedback in ms.
    pub feedback_duration_ms: u32,
    /// Disabled state.
    pub disabled: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: String::new(),
            feedback_duration_ms: 2000,
            disabled: false,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();

        (State::Idle, Context {
            value: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None    => Bindable::uncontrolled(props.default_value.clone()),
            },
            feedback_duration_ms: props.feedback_duration_ms,
            disabled: props.disabled,
            error: None,
            locale,
            messages,
            ids,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled { return None; }

        match (state, event) {
            (State::Idle, Event::Copy) => {
                Some(TransitionPlan::to(State::Copying).with_named_effect("clipboard-write", |ctx, _props, send| {
                    let value = ctx.value.get().clone();
                    clipboard_write_text(&value, move |result| {
                        match result {
                            Ok(()) => send(Event::CopySuccess),
                            Err(reason) => send(Event::CopyError(reason)),
                        }
                    });
                    no_cleanup()
                }))
            }

            (State::Copying, Event::CopySuccess) => {
                Some(TransitionPlan::to(State::Copied).apply(|ctx| {
                    ctx.error = None;
                }).with_named_effect("announce", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    platform.announce(&(ctx.messages.copied_announcement)(&ctx.locale));
                    no_cleanup()
                }).with_named_effect("feedback-timer", |ctx, _props, send| {
                    let platform = use_platform_effects();
                    let duration = ctx.feedback_duration_ms;
                    let handle = platform.set_timeout(duration, Box::new(move || {
                        send(Event::ResetTimeout);
                    }));
                    let pc = platform.clone();
                    Box::new(move || pc.clear_timeout(handle))
                }))
            }

            (State::Copying, Event::CopyError(reason)) => {
                let reason = reason.clone();
                Some(TransitionPlan::to(State::Error).apply(move |ctx| {
                    ctx.error = Some(reason);
                }).with_named_effect("announce", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    platform.announce(&(ctx.messages.error_announcement)(&ctx.locale));
                    no_cleanup()
                }).with_named_effect("error-feedback-timer", |ctx, _props, send| {
                    let platform = use_platform_effects();
                    let duration = ctx.feedback_duration_ms;
                    let handle = platform.set_timeout(duration, Box::new(move || {
                        send(Event::ResetTimeout);
                    }));
                    let pc = platform.clone();
                    Box::new(move || pc.clear_timeout(handle))
                }))
            }

            (State::Copied, Event::ResetTimeout)
            | (State::Error, Event::ResetTimeout) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.error = None;
                }))
            }

            // Allow re-copy while in Copied or Error state
            (State::Copied, Event::Copy)
            | (State::Error, Event::Copy) => {
                Some(TransitionPlan::to(State::Copying).with_named_effect("clipboard-write", |ctx, _props, send| {
                    let value = ctx.value.get().clone();
                    clipboard_write_text(&value, move |result| {
                        match result {
                            Ok(()) => send(Event::CopySuccess),
                            Err(reason) => send(Event::CopyError(reason)),
                        }
                    });
                    no_cleanup()
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
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "clipboard"]
pub enum Part {
    Root,
    Label,
    Trigger,
    Indicator,
    Status,
    ValueText,
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn is_copied(&self) -> bool { *self.state == State::Copied }
    pub fn is_copying(&self) -> bool { *self.state == State::Copying }
    pub fn is_error(&self) -> bool { *self.state == State::Error }
    pub fn error(&self) -> Option<&CopyFailureReason> { self.ctx.error.as_ref() }
    pub fn copy(&self) { (self.send)(Event::Copy); }

    fn state_str(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Copying => "copying",
            State::Copied => "copied",
            State::Error => "error",
        }
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), self.state_str());
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs
    }

    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs
    }

    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("trigger"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), match self.state {
            State::Idle => (self.ctx.messages.trigger_label)(&self.ctx.locale),
            State::Copying => (self.ctx.messages.copying_label)(&self.ctx.locale),
            State::Copied => (self.ctx.messages.copied_label)(&self.ctx.locale),
            State::Error => (self.ctx.messages.error_label)(&self.ctx.locale),
        });
        attrs.set(HtmlAttr::Data("ars-state"), self.state_str());
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        attrs
    }

    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::Data("ars-state"), self.state_str());
        attrs
    }

    pub fn status_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Status.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "status");
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
        attrs
    }

    pub fn value_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ValueText.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn on_trigger_click(&self) { (self.send)(Event::Copy); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Indicator => self.indicator_attrs(),
            Part::Status => self.status_attrs(),
            Part::ValueText => self.value_text_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Clipboard
├── Root        (required)
├── Label       (optional — describes what will be copied)
├── Trigger     (required — button to initiate copy)
├── Indicator   (optional — visual icon: clipboard → check → x)
├── Status      (required — aria-live region for screen reader feedback)
└── ValueText   (optional — display of the text being copied)
```

| Part      | Element    | Key Attributes                                       |
| --------- | ---------- | ---------------------------------------------------- |
| Root      | `<div>`    | `data-ars-state`, `data-ars-disabled`                |
| Label     | `<label>`  | `id` for association                                 |
| Trigger   | `<button>` | `aria-label` (state-dependent), `aria-disabled`      |
| Indicator | `<span>`   | `aria-hidden="true"`, `data-ars-state`               |
| Status    | `<div>`    | `role="status"`, `aria-live="polite"`, `aria-atomic` |
| ValueText | `<span>`   | displays the text being copied                       |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part      | Role     | Properties                                        |
| --------- | -------- | ------------------------------------------------- |
| Trigger   | `button` | `aria-label` (changes per state), `aria-disabled` |
| Indicator | —        | `aria-hidden="true"` (decorative)                 |
| Status    | `status` | `aria-live="polite"`, `aria-atomic="true"`        |

### 3.2 Keyboard Interaction

| Key           | Action        |
| ------------- | ------------- |
| Enter / Space | Initiate copy |

### 3.3 Screen Reader Announcements

The Status part is an `aria-live="polite"` region. On successful copy, the adapter inserts the `copied_announcement` message. On failure, it inserts the `error_announcement` message. The text is replaced on each state change (`aria-atomic="true"`).

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the copy trigger button. Default: `"Copy to clipboard"`.
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label while copy is in progress. Default: `"Copying..."`.
    pub copying_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Feedback text when copy succeeds. Default: `"Copied!"`.
    pub copied_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Feedback text when copy fails. Default: `"Copy failed, click to retry"`.
    pub error_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Announcement when copy succeeds. Default: `"Copied to clipboard"`.
    pub copied_announcement: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Announcement when copy fails. Default: `"Failed to copy to clipboard"`.
    pub error_announcement: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Copy to clipboard"),
            copying_label: MessageFn::static_str("Copying..."),
            copied_label: MessageFn::static_str("Copied!"),
            error_label: MessageFn::static_str("Copy failed, click to retry"),
            copied_announcement: MessageFn::static_str("Copied to clipboard"),
            error_announcement: MessageFn::static_str("Failed to copy to clipboard"),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                             | Default (en-US)                 | Purpose                    |
| ------------------------------- | ------------------------------- | -------------------------- |
| `clipboard.trigger_label`       | `"Copy to clipboard"`           | Trigger label (idle)       |
| `clipboard.copying_label`       | `"Copying..."`                  | Trigger label (copying)    |
| `clipboard.copied_label`        | `"Copied!"`                     | Trigger label (copied)     |
| `clipboard.error_label`         | `"Copy failed, click to retry"` | Trigger label (error)      |
| `clipboard.copied_announcement` | `"Copied to clipboard"`         | Screen reader announcement |
| `clipboard.error_announcement`  | `"Failed to copy to clipboard"` | Screen reader announcement |

- **RTL**: Layout direction of label and trigger reverses. No special handling needed for the clipboard API itself.

## 5. Library Parity

> Compared against: Ark UI (`Clipboard`).

### 5.1 Props

| Feature                  | ars-ui                    | Ark UI                   | Notes                                       |
| ------------------------ | ------------------------- | ------------------------ | ------------------------------------------- |
| `value` / `defaultValue` | `value` / `default_value` | `value` / `defaultValue` | Equivalent                                  |
| `timeout`                | `feedback_duration_ms`    | `timeout`                | Equivalent (different naming, same purpose) |
| `disabled`               | `disabled`                | --                       | ars-ui adds disabled support                |

**Gaps:** None.

### 5.2 Anatomy

| Part      | ars-ui      | Ark UI      | Notes                                         |
| --------- | ----------- | ----------- | --------------------------------------------- |
| Root      | `Root`      | `Root`      | Equivalent                                    |
| Label     | `Label`     | `Label`     | Equivalent                                    |
| Trigger   | `Trigger`   | `Trigger`   | Equivalent                                    |
| Indicator | `Indicator` | `Indicator` | Equivalent                                    |
| Status    | `Status`    | --          | ars-ui adds live region for SR                |
| ValueText | `ValueText` | `ValueText` | Equivalent                                    |
| Control   | --          | `Control`   | Ark wraps input+trigger; ars-ui omits wrapper |
| Input     | --          | `Input`     | Ark has input field; ars-ui is copy-only      |

**Gaps:** None. Ark's `Control`/`Input` parts are for editable clipboard value; ars-ui uses `Bindable<String>` for value management without a visible input.

### 5.3 Events

| Callback      | ars-ui                    | Ark UI           | Notes      |
| ------------- | ------------------------- | ---------------- | ---------- |
| Status change | State machine transitions | `onStatusChange` | Equivalent |
| Value change  | `Bindable` reactivity     | `onValueChange`  | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature                     | ars-ui                               | Ark UI                |
| --------------------------- | ------------------------------------ | --------------------- |
| Copy to clipboard           | Yes                                  | Yes                   |
| Feedback timer              | Yes                                  | Yes                   |
| Error handling              | Yes (structured `CopyFailureReason`) | No                    |
| Screen reader announcements | Yes (live region)                    | No (data-copied only) |
| Disabled state              | Yes                                  | No                    |

**Gaps:** None. ars-ui exceeds Ark UI with error handling and accessibility.

### 5.5 Summary

- **Overall:** Full parity, with additional error handling and accessibility features.
- **Divergences:** ars-ui has structured error types (`CopyFailureReason`) and a dedicated `Status` live region. Ark UI uses `data-copied` attribute for visual feedback only.
- **Recommended additions:** None.
