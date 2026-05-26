---
component: Collapsible
category: layout
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [layout-shared-types]
related: []
references:
    ark-ui: Collapsible
    radix-ui: Collapsible
    react-aria: Disclosure
---

# Collapsible

`Collapsible` is an expandable/collapsible container that toggles visibility of a content region. It follows the WAI-ARIA Disclosure pattern and serves as the building block for the Accordion component. Supports controlled and uncontrolled open state, lazy mounting, and exit animations via `Presence` integration (`lazy_mount` and `unmount_on_exit` props).

## 1. State Machine

### 1.1 States

```rust
/// States of the `Collapsible`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum State {
    /// The content region is visible.
    Open,
    /// The content region is hidden.
    #[default]
    Closed,
}
```

### 1.2 Events

```rust
/// Events sent to the `Collapsible`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Event {
    /// Toggle between open and closed states.
    Toggle,
    /// Programmatically set the open state.
    SetOpen(bool),
    /// The trigger or content received focus.
    Focus {
        /// Whether the focus is received via keyboard.
        is_keyboard: bool,
    },
    /// The trigger or content lost focus.
    Blur,
    /// Synchronize prop-backed context fields.
    SyncProps,
    /// Synchronize the controlled open value.
    SyncControlledOpen(bool),
}
```

### 1.3 Context

```rust
/// Runtime context for the `Collapsible` state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the collapsible is open. Uses `Bindable` to support both
    /// controlled (parent owns the value) and uncontrolled (internal) modes.
    pub open: Bindable<bool>,
    /// When `true`, the trigger is non-interactive and toggle is suppressed.
    pub disabled: bool,
    /// Whether the trigger currently has focus.
    pub focused: bool,
    /// Whether focus was received via keyboard (for focus-visible styles).
    pub focus_visible: bool,
    /// Component identifiers for ARIA attribute generation.
    pub ids: ComponentIds,
    /// When `Some`, the collapsed state shows partial content at this CSS height
    /// (e.g., `"80px"`) instead of fully hiding. Enables "read more" patterns.
    pub collapsed_height: Option<String>,
    /// When `Some`, the collapsed state shows partial content at this CSS width
    /// (e.g., `"120px"`) instead of fully hiding. Enables horizontal collapse patterns.
    pub collapsed_width: Option<String>,
    /// Resolved locale for message formatting.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// Configuration props for the `Collapsible` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,
    /// Controlled open state. When `Some`, the consumer owns the value.
    pub open: Option<bool>,
    /// Initial open state for uncontrolled usage. Ignored when `open` is `Some`.
    pub default_open: bool,
    /// Disables interaction when `true`.
    pub disabled: bool,
    /// When true, content is not mounted until the collapsible is first opened.
    /// Works with Presence at the adapter layer. Default: false.
    pub lazy_mount: bool,
    /// When true, content is removed from the DOM after collapsing.
    /// Works with Presence for exit animations at the adapter layer. Default: false.
    pub unmount_on_exit: bool,
    /// When `Some`, the collapsed state shows partial content at this CSS height
    /// instead of fully hiding (e.g., `"80px"`). Default: `None`.
    pub collapsed_height: Option<String>,
    /// When `Some`, the collapsed state shows partial content at this CSS width
    /// instead of fully hiding (e.g., `"120px"`). Default: `None`.
    pub collapsed_width: Option<String>,
}

impl Default for Props {
    fn default() -> Self {
        Props {
            id: String::new(),
            open: None,
            default_open: false,
            disabled: false,
            lazy_mount: false,
            unmount_on_exit: false,
            collapsed_height: None,
            collapsed_width: None,
        }
    }
}

impl Props {
    /// Returns fresh collapsible props with documented defaults.
    pub fn new() -> Self { Self::default() }

    /// Sets the component instance ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets `open`, making the collapsible controlled at mount.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Clears `open`, making the collapsible uncontrolled.
    pub fn uncontrolled(mut self) -> Self {
        self.open = None;
        self
    }

    /// Sets `default_open`.
    pub fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }

    /// Sets `disabled`.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets `lazy_mount`.
    pub fn lazy_mount(mut self, lazy_mount: bool) -> Self {
        self.lazy_mount = lazy_mount;
        self
    }

    /// Sets `unmount_on_exit`.
    pub fn unmount_on_exit(mut self, unmount_on_exit: bool) -> Self {
        self.unmount_on_exit = unmount_on_exit;
        self
    }

    /// Sets `collapsed_height`.
    pub fn collapsed_height(mut self, collapsed_height: impl Into<String>) -> Self {
        self.collapsed_height = Some(collapsed_height.into());
        self
    }

    /// Sets `collapsed_width`.
    pub fn collapsed_width(mut self, collapsed_width: impl Into<String>) -> Self {
        self.collapsed_width = Some(collapsed_width.into());
        self
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
    type Effect = NoEffect;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let initial_open = props.open.unwrap_or(props.default_open);
        let state = if initial_open { State::Open } else { State::Closed };
        let open = match props.open {
            Some(v) => Bindable::controlled(v),
            None    => Bindable::uncontrolled(initial_open),
        };

        let locale = env.locale.clone();
        let messages = messages.clone();
        let ctx = Context {
            open,
            disabled: props.disabled,
            focused: false,
            focus_visible: false,
            ids: ComponentIds::from_id(&props.id),
            collapsed_height: props.collapsed_height.clone(),
            collapsed_width: props.collapsed_width.clone(),
            locale,
            messages,
        };
        (state, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::Toggle if !ctx.disabled => open_transition(ctx, !ctx.open.get()),

            Event::SetOpen(value) if !ctx.disabled => open_transition(ctx, *value),

            Event::Focus { is_keyboard } => {
                let kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = kb;
                }))
            }

            Event::Blur => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            Event::SyncProps => Some(sync_props_plan(ctx, props)),

            Event::SyncControlledOpen(value) => {
                let value = *value;
                let next = if value { State::Open } else { State::Closed };
                Some(TransitionPlan::to(next).apply(move |ctx| {
                    ctx.open.set(value);
                    ctx.open.sync_controlled(Some(value));
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
        let mut events = Vec::new();

        if old.disabled != new.disabled
            || old.collapsed_height != new.collapsed_height
            || old.collapsed_width != new.collapsed_width
            || (old.open.is_some() && new.open.is_none())
        {
            events.push(Event::SyncProps);
        }

        if old.open != new.open {
            if let Some(open) = new.open {
                events.push(Event::SyncControlledOpen(open));
            }
        }

        events
    }
}
```

Controlled-mode `Toggle` and `SetOpen` update the pending internal value via `Bindable::set`, but visible state continues to follow the controlled `open` value until the parent rerenders and `SyncControlledOpen` applies the new controlled prop. `SyncControlledOpen` also updates the bindable's internal fallback to the controlled value so that leaving controlled mode resumes from the latest visible state. When `open` changes from `Some(_)` to `None`, `SyncProps` returns the bindable to uncontrolled mode and the visible state follows that synchronized internal value.

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "collapsible"]
pub enum Part {
    Root,
    Trigger,
    Indicator,
    Content,
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the collapsible is currently open.
    pub fn is_open(&self) -> bool {
        *self.state == State::Open
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs
    }

    pub fn trigger_attrs(&self) -> AttrMap {
        let trigger_id = self.ctx.ids.part("trigger");
        let content_id = self.ctx.ids.part("content");
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, trigger_id);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.is_open() { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), content_id);
        // State-dependent accessible label: "Show content" when collapsed, "Hide content" when expanded.
        let label = if self.is_open() {
            (self.ctx.messages.collapse_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.expand_label)(&self.ctx.locale)
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        if self.ctx.focused {
            attrs.set_bool(HtmlAttr::Data("ars-focus"), true);
        }
        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs
    }

    /// Attrs for the optional visual indicator (e.g., chevron) showing open/closed state.
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    pub fn content_attrs(&self) -> AttrMap {
        let trigger_id = self.ctx.ids.part("trigger");
        let content_id = self.ctx.ids.part("content");
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, content_id);
        attrs.set(HtmlAttr::Role, "region");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), trigger_id);
        let has_collapsed_size = self.ctx.collapsed_height.is_some()
            || self.ctx.collapsed_width.is_some();
        if !self.is_open() {
            // When a collapsed size is set, the content remains visible at that size
            // instead of being fully hidden. The `hidden` attribute is omitted so the
            // partial content is still rendered and accessible.
            if !has_collapsed_size {
                attrs.set_bool(HtmlAttr::Hidden, true);
            }
        }
        if let Some(ref h) = self.ctx.collapsed_height {
            attrs.set_style(CssProperty::Custom("ars-collapsible-collapsed-height"), h);
        }
        if let Some(ref w) = self.ctx.collapsed_width {
            attrs.set_style(CssProperty::Custom("ars-collapsible-collapsed-width"), w);
        }
        if has_collapsed_size {
            attrs.set_bool(HtmlAttr::Data("ars-collapsed-size"), true);
        }
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs
    }

    pub fn on_trigger_click(&self) {
        if !self.ctx.disabled {
            (self.send)(Event::Toggle);
        }
    }

    /// Returns `true` when the key was handled so adapters can prevent the
    /// native button activation click and avoid dispatching a duplicate toggle.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) -> bool {
        if !self.ctx.disabled && (data.key == KeyboardKey::Enter || data.key == KeyboardKey::Space) {
            (self.send)(Event::Toggle);
            return true;
        }

        false
    }

    pub fn on_trigger_focus(&self, is_keyboard: bool) {
        if !self.ctx.disabled {
            (self.send)(Event::Focus { is_keyboard });
        }
    }

    pub fn on_trigger_blur(&self) { (self.send)(Event::Blur); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Indicator => self.indicator_attrs(),
            Part::Content => self.content_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Collapsible
├── Root          <div>      data-ars-scope="collapsible" data-ars-part="root"
│                             data-ars-state="open|closed"
├── Trigger       <button>   data-ars-scope="collapsible" data-ars-part="trigger"
│                             aria-expanded aria-controls
│   └── Indicator <span>     data-ars-scope="collapsible" data-ars-part="indicator"
│                             aria-hidden="true" (optional)
└── Content       <div>      data-ars-scope="collapsible" data-ars-part="content"
                              role="region" aria-labelledby hidden(when closed)
                              --ars-collapsible-collapsed-height --ars-collapsible-collapsed-width
```

| Part      | Element    | Key Attributes                                                                                                                                                                                   |
| --------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Root      | `<div>`    | `data-ars-state="open\|closed"`, `data-ars-disabled`                                                                                                                                             |
| Trigger   | `<button>` | `aria-expanded`, `aria-controls`, `data-ars-focus-visible`                                                                                                                                       |
| Indicator | `<span>`   | `data-ars-state="open\|closed"`, `aria-hidden="true"` (purely decorative)                                                                                                                        |
| Content   | `<div>`    | `role="region"`, `aria-labelledby`, `hidden` when closed (omitted when `collapsed_height`/`collapsed_width` set), `--ars-collapsible-collapsed-height/width` CSS vars, `data-ars-collapsed-size` |

Root, Trigger, and Content are required. Indicator is optional.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

The collapsible follows the [WAI-ARIA Disclosure Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/).

| Element   | Attribute         | Value                                                                                                     |
| --------- | ----------------- | --------------------------------------------------------------------------------------------------------- |
| Trigger   | `aria-expanded`   | `"true"` when open, `"false"` when closed                                                                 |
| Trigger   | `aria-controls`   | ID of the Content element                                                                                 |
| Indicator | `aria-hidden`     | `"true"` (purely decorative; screen readers ignore it)                                                    |
| Content   | `role`            | `"region"`                                                                                                |
| Content   | `aria-labelledby` | ID of the Trigger element                                                                                 |
| Content   | `hidden`          | Present when closed (removes from a11y tree). Omitted when `collapsed_height` or `collapsed_width` is set |

When `disabled` is `true`, the trigger receives the `disabled` attribute and is excluded from the tab order. Click and keyboard handlers are no-ops.

### 3.2 Keyboard Interaction

| Key               | Action                                                     |
| ----------------- | ---------------------------------------------------------- |
| `Enter` / `Space` | Toggles the collapsible open/closed when trigger has focus |
| `Tab`             | Moves focus to the next focusable element                  |
| `Shift+Tab`       | Moves focus to the previous focusable element              |

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the trigger when collapsed. Default: `"Show content"`.
    pub expand_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the trigger when expanded. Default: `"Hide content"`.
    pub collapse_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            expand_label: MessageFn::static_str("Show content"),
            collapse_label: MessageFn::static_str("Hide content"),
        }
    }
}

impl ComponentMessages for Messages {}
```

RTL: The collapsible has no orientation-dependent behaviour. Arrow keys are not used for primary interaction, so RTL mode requires no special handling. Text direction within the content region is inherited from the document or parent element.

## 5. Library Parity

> Compared against: Ark UI (`Collapsible`), Radix UI (`Collapsible`), React Aria (`Disclosure`).

### 5.1 Props

| Feature                 | ars-ui               | Ark UI            | Radix UI      | React Aria        | Notes                                 |
| ----------------------- | -------------------- | ----------------- | ------------- | ----------------- | ------------------------------------- |
| Controlled open         | `open`               | `open`            | `open`        | `isExpanded`      | Same                                  |
| Default open            | `default_open`       | `defaultOpen`     | `defaultOpen` | `defaultExpanded` | Same                                  |
| Disabled                | `disabled`           | `disabled`        | `disabled`    | `isDisabled`      | Same                                  |
| Lazy mount              | `lazy_mount`         | `lazyMount`       | --            | --                | Ark UI + ars-ui                       |
| Unmount on exit         | `unmount_on_exit`    | `unmountOnExit`   | --            | --                | Ark UI + ars-ui                       |
| Force mount (Content)   | --                   | --                | `forceMount`  | --                | Inverse of `lazy_mount`; covered      |
| Collapsed height        | `collapsed_height`   | `collapsedHeight` | --            | --                | Ark UI + ars-ui                       |
| Collapsed width         | `collapsed_width`    | `collapsedWidth`  | --            | --                | Ark UI + ars-ui                       |
| Exit animation callback | Presence integration | `onExitComplete`  | --            | --                | Handled via Presence at adapter layer |

**Gaps:** None.

### 5.2 Anatomy

| Part      | ars-ui      | Ark UI      | Radix UI  | React Aria         | Notes                          |
| --------- | ----------- | ----------- | --------- | ------------------ | ------------------------------ |
| Root      | `Root`      | `Root`      | `Root`    | `Disclosure`       | --                             |
| Trigger   | `Trigger`   | `Trigger`   | `Trigger` | `DisclosureHeader` | React Aria uses header wrapper |
| Indicator | `Indicator` | `Indicator` | --        | --                 | Ark UI + ars-ui                |
| Content   | `Content`   | `Content`   | `Content` | `DisclosurePanel`  | --                             |

**Gaps:** None.

### 5.3 Events

| Callback    | ars-ui            | Ark UI         | Radix UI       | React Aria         | Notes                             |
| ----------- | ----------------- | -------------- | -------------- | ------------------ | --------------------------------- |
| Open change | `Bindable` change | `onOpenChange` | `onOpenChange` | `onExpandedChange` | Handled via Bindable notification |

**Gaps:** None.

### 5.4 Features

| Feature                 | ars-ui | Ark UI | Radix UI | React Aria |
| ----------------------- | ------ | ------ | -------- | ---------- |
| Controlled/uncontrolled | Yes    | Yes    | Yes      | Yes        |
| Disabled state          | Yes    | Yes    | Yes      | Yes        |
| Lazy mount              | Yes    | Yes    | --       | --         |
| Unmount on exit         | Yes    | Yes    | --       | --         |
| Collapsed height/width  | Yes    | Yes    | --       | --         |
| CSS animation vars      | Yes    | Yes    | Yes      | --         |
| Indicator part          | Yes    | Yes    | --       | --         |
| ARIA disclosure pattern | Yes    | Yes    | Yes      | Yes        |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria wraps the trigger in a `DisclosureHeader` for heading-level semantics; ars-ui leaves heading wrapping to the consumer. Radix uses `forceMount` (opt-in mount) vs ars-ui's `lazy_mount` (opt-in lazy); opposite defaults, same capability.
- **Recommended additions:** None.
