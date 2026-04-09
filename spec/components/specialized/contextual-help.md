---
component: ContextualHelp
category: specialized
tier: stateless
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: [popover]
references:
  react-aria: ContextualHelp
---

# ContextualHelp

ContextualHelp provides on-demand contextual information via a trigger icon button
and a non-modal popover. Two variants exist:

- **Help** (`?` icon) — deeper educational guidance, may link to documentation.
- **Info** (`i` icon) — brief, specific, contextual clarification.

This is the standard pattern for inline help or info buttons adjacent to form labels.
Equivalent to React Spectrum's `ContextualHelp`.

ContextualHelp is **not** a standalone state machine. It is a thin composition layer
over the existing `popover::Machine`, providing a pre-wired trigger button,
variant-aware labeling, and structured content anatomy. Internally it instantiates
a `popover::Machine` with the following hardcoded configuration:

- `modal: false`
- `close_on_escape: true`
- `close_on_interact_outside: true`
- Positioning parameters forwarded from Props (`placement`, `offset`, `cross_offset`,
  `should_flip`, `container_padding`)

Framework adapters create the `popover::Machine` inside the ContextualHelp
component, so consumers never interact with Popover directly.

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// "?" icon — deeper educational guidance, may link to docs.
    #[default]
    Help,
    /// "i" icon — brief, specific, contextual clarification.
    Info,
}
```

## 1. API

### 1.1 Props

```rust
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Help or Info variant.
    pub variant: Variant,
    /// Preferred placement relative to the trigger. Default: `Placement::BottomStart`.
    pub placement: Placement,
    /// Offset along the main axis in pixels.
    pub offset: f64,
    /// Offset along the cross axis in pixels.
    pub cross_offset: f64,
    /// Whether the popover flips when it would overflow.
    pub should_flip: bool,
    /// Padding between the popover and container edges.
    pub container_padding: f64,
    /// Text direction override (inherited from locale if `None`).
    pub dir: Option<Direction>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            variant: Variant::Help,
            placement: Placement::BottomStart,
            offset: 0.0,
            cross_offset: 0.0,
            should_flip: true,
            container_padding: 12.0,
            dir: None,
        }
    }
}
```

### 1.2 Connect / API

`Api` wraps `popover::Api` and adds trigger-specific attributes:

```rust
#[derive(ComponentPart)]
#[scope = "contextual-help"]
pub enum Part {
    Root,
    Trigger,
    Content,
    Heading,
    Body,
    Footer,
    DismissButton,
}

pub struct Api<'a> {
    popover_api: popover::Api<'a>,
    props: &'a Props,
    locale: Locale,
    messages: Messages,
}

impl<'a> Api<'a> {
    pub fn new(popover_api: popover::Api<'a>, props: &'a Props, env: &Env, messages: &Messages) -> Self {
        let locale = env.locale.clone();
        let messages = messages.clone();
        Self { popover_api, props, locale, messages }
    }

    pub fn is_open(&self) -> bool {
        self.popover_api.is_open()
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        let label = match self.props.variant {
            Variant::Help => &self.messages.help_label,
            Variant::Info => &self.messages.info_label,
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label(&self.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.is_open() { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.popover_api.content_id());
        attrs.set(HtmlAttr::Data("ars-variant"), match self.props.variant {
            Variant::Help => "help",
            Variant::Info => "info",
        });
        attrs
    }

    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.popover_api.heading_id());
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Id, self.popover_api.content_id());
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs
    }

    pub fn heading_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Heading.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.popover_api.heading_id());
        attrs
    }

    pub fn body_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Body.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn footer_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Footer.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn dismiss_button_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DismissButton.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.messages.close_label)(&self.locale));
        attrs
    }

    pub fn on_trigger_click(&self) {
        self.popover_api.toggle();
    }

    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Escape {
            self.popover_api.close();
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Content => self.content_attrs(),
            Part::Heading => self.heading_attrs(),
            Part::Body => self.body_attrs(),
            Part::Footer => self.footer_attrs(),
            Part::DismissButton => self.dismiss_button_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
ContextualHelp
├── Root              (required)
├── Trigger           (required — <button> with "?" or "i" icon)
└── Content           (required — role="dialog", non-modal)
    ├── Heading       (required — title)
    ├── Body          (required — main help text)
    ├── Footer        (optional — e.g., "Learn more" link)
    └── DismissButton (required — visually hidden, for screen readers)
```

| Part          | Element    | Key Attributes                                          |
| ------------- | ---------- | ------------------------------------------------------- |
| Root          | `<div>`    | container                                               |
| Trigger       | `<button>` | `aria-label`, `aria-haspopup="dialog"`, `aria-expanded` |
| Content       | `<div>`    | `role="dialog"`, `aria-labelledby`, `tabindex="-1"`     |
| Heading       | `<h3>`     | `id` (linked from `aria-labelledby`)                    |
| Body          | `<div>`    | main help text                                          |
| Footer        | `<div>`    | optional link/action area                               |
| DismissButton | `<button>` | `aria-label` (close label), visually hidden             |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part          | Role     | Properties                                                                                   |
| ------------- | -------- | -------------------------------------------------------------------------------------------- |
| Trigger       | `button` | `aria-label` (variant-dependent), `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls` |
| Content       | `dialog` | `aria-labelledby` (Heading id), `tabindex="-1"`                                              |
| DismissButton | `button` | `aria-label` (close label), visually hidden                                                  |

### 3.2 Keyboard Interaction

| Key           | Behavior                                                  |
| ------------- | --------------------------------------------------------- |
| Enter / Space | Toggle popover (on trigger)                               |
| Escape        | Close popover, return focus to trigger                    |
| Tab           | Navigate focusable content; can leave popover (non-modal) |

### 3.3 Focus Management

- Non-modal: focus is **not** trapped. Tab moves out of the popover normally. Uses `FocusScope::popover()` preset.
- Focus moves into content on open (first focusable element, or the content container if no focusable children).
- Focus returns to the trigger on close.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the trigger button in Help variant. Default: `"Help"`.
    pub help_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the trigger button in Info variant. Default: `"Information"`.
    pub info_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the visually-hidden dismiss button. Default: `"Dismiss"`.
    pub close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            help_label: MessageFn::static_str("Help"),
            info_label: MessageFn::static_str("Information"),
            close_label: MessageFn::static_str("Dismiss"),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                           | Default (en)    | Description                          |
| ----------------------------- | --------------- | ------------------------------------ |
| `contextual_help.help_label`  | `"Help"`        | Trigger button label (Help variant)  |
| `contextual_help.info_label`  | `"Information"` | Trigger button label (Info variant)  |
| `contextual_help.close_label` | `"Dismiss"`     | Visually-hidden dismiss button label |

- **Trigger label**: `aria-label` uses `Messages` fields, which are locale-aware `MessageFn` values.
- **RTL**: When `dir` is set (or inherited from locale context), the `dir` attribute is emitted on the content container. Popover placement adapts logical-to-physical direction automatically (e.g., `BottomStart` becomes right-aligned in RTL).
- **Content**: Heading, body, and footer content is provided by the caller — the library handles structural accessibility only, not content translation.

## 5. Library Parity

> No counterpart in Ark UI, Radix UI, or React Aria component libraries.

ContextualHelp is based on the React Spectrum `ContextualHelp` pattern (a design system component, not part of React Aria's headless library). It composes ars-ui's `Popover` machine with a pre-configured trigger button and structured content anatomy (Heading/Body/Footer).

### 5.1 Summary

- **Overall:** No library counterpart to compare against. The component follows React Spectrum's design pattern as the canonical reference.
- **Divergences:** N/A.
- **Recommended additions:** None.
