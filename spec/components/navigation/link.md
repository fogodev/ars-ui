---
component: Link
category: navigation
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
    react-aria: Link
---

# Link

Accessible link component with client-side router integration. Renders as `<a>` by default.
Supports `is_current` for navigation highlighting (`aria-current`), disabled state, and
external link detection.

## 1. State Machine

### 1.1 States

| State     | Description                                             |
| --------- | ------------------------------------------------------- |
| `Idle`    | Default resting state.                                  |
| `Focused` | The link has keyboard or pointer focus.                 |
| `Pressed` | The link is being activated (pointer down or key held). |

### 1.2 Events

| Event                   | Payload | Description                                                  |
| ----------------------- | ------- | ------------------------------------------------------------ |
| `Focus { is_keyboard }` | `bool`  | The link received focus; tracks whether it was via keyboard. |
| `Blur`                  | —       | Focus left the link.                                         |
| `Press`                 | —       | Pointer down or activation key pressed.                      |
| `PressEnd`              | —       | Pointer up or activation key released.                       |
| `Navigate`              | —       | The link was activated (click or keyboard confirm).          |

### 1.3 Context

```rust
/// Context for the `Link` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The destination URL (validated on construction via `SafeUrl`).
    pub href: SafeUrl,
    /// Optional browsing context target (e.g. `_blank`).
    pub target: Option<String>,
    /// Optional link relationship (e.g. `noopener noreferrer`).
    pub rel: Option<String>,
    /// Indicates the link represents the current item within a set.
    pub is_current: Option<AriaCurrent>,
    /// When true, the link is non-interactive and has no href.
    pub disabled: bool,
    /// Whether the link currently has focus.
    pub focused: bool,
    /// True when focus was received via keyboard (Tab, not click).
    pub focus_visible: bool,
    /// Whether the link is currently being pressed.
    pub pressed: bool,
    /// Generated IDs for sub-parts.
    pub ids: ComponentIds,
    /// Resolved locale for message formatting.
    pub locale: Locale,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

/// Describes the type of current-item indication for `aria-current`.
#[derive(Clone, Debug, PartialEq)]
pub enum AriaCurrent {
    /// The link represents the current page.
    Page,
    /// The link represents the current step.
    Step,
    /// The link represents the current location.
    Location,
    /// The link represents the current date.
    Date,
    /// The link represents the current time.
    Time,
    /// The link represents the current true value.
    True,
}
```

### 1.4 Props

```rust
/// Props for the `Link` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Unique component identifier.
    pub id: String,
    /// The destination URL (validated via `SafeUrl` — rejects `javascript:`, `data:`, `vbscript:` schemes).
    /// See `01-architecture.md` §3.1.1.1 for allowed schemes.
    pub href: SafeUrl,
    /// Optional browsing context target (e.g. `_blank`).
    pub target: Option<String>,
    /// Optional link relationship (e.g. `noopener noreferrer`).
    pub rel: Option<String>,
    /// Marks the link as the current item within a navigation set.
    pub is_current: Option<AriaCurrent>,
    /// Disables the link (removes href, sets `aria-disabled`).
    pub disabled: bool,
    // Change callbacks provided by the adapter layer
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            href: SafeUrl::new("").expect("empty string is a safe URL"),
            target: None,
            rel: None,
            is_current: None,
            disabled: false,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, AttrMap};

// ── States ────────────────────────────────────────────────────────────────────

/// States for the `Link` component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The link is in the idle state.
    Idle,
    /// The link is in the focused state.
    Focused,
    /// The link is in the pressed state.
    Pressed,
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Events for the `Link` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The link received focus; tracks whether it was via keyboard.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// The focus left the link.
    Blur,
    /// The link is being pressed.
    Press,
    /// The link is being released.
    PressEnd,
    /// The link was activated (click or keyboard confirm).
    Navigate,
}

// ── Machine ───────────────────────────────────────────────────────────────────

/// Machine for the `Link` component.
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
        let locale = env.locale.clone();
        let messages = messages.clone();
        (State::Idle, Context {
            href: props.href.clone(),
            target: props.target.clone(),
            rel: props.rel.clone(),
            is_current: props.is_current.clone(),
            disabled: props.disabled,
            focused: false,
            focus_visible: false,
            pressed: false,
            ids,
            locale,
            messages,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {

            // ── Focus ─────────────────────────────────────────────────────────
            (_, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::to(State::Focused)
                    .apply(move |ctx| {
                        ctx.focused = true;
                        ctx.focus_visible = is_kb;
                    }))
            }

            // ── Blur ──────────────────────────────────────────────────────────
            (_, Event::Blur) => {
                Some(TransitionPlan::to(State::Idle)
                    .apply(|ctx| {
                        ctx.focused = false;
                        ctx.focus_visible = false;
                        ctx.pressed = false;
                    }))
            }

            // ── Press ─────────────────────────────────────────────────────────
            (State::Focused, Event::Press) if !ctx.disabled => {
                Some(TransitionPlan::to(State::Pressed)
                    .apply(|ctx| {
                        ctx.pressed = true;
                    }))
            }

            // ── PressEnd ──────────────────────────────────────────────────────
            (State::Pressed, Event::PressEnd) => {
                Some(TransitionPlan::to(State::Focused)
                    .apply(|ctx| {
                        ctx.pressed = false;
                    }))
            }

            // ── Navigate ──────────────────────────────────────────────────────
            (_, Event::Navigate) if !ctx.disabled => {
                Some(TransitionPlan::context_only(|_ctx| {
                    // Navigation side effect handled by the framework adapter
                })
                .with_effect(PendingEffect::new("navigate", |ctx, props, _send| {
                    if let Some(ref on_navigate) = props.on_navigate {
                        on_navigate(&ctx.href);
                    }
                    if let Some(ref on_press) = props.on_press {
                        on_press();
                    }
                    no_cleanup()
                })))
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
use ars_core::AttrMap;

#[derive(ComponentPart)]
#[scope = "link"]
pub enum Part {
    Root,
}

/// API for the `Link` component.
pub struct Api<'a> {
    /// The state of the component.
    state: &'a State,
    /// The context of the component.
    ctx:   &'a Context,
    /// The props of the component.
    props: &'a Props,
    /// The send function to send events to the component.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Attrs for the root `<a>` element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle    => "idle",
            State::Focused => "focused",
            State::Pressed => "pressed",
        });

        if self.ctx.disabled {
            // Disabled links have no href and announce as disabled.
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        } else {
            // Resolve target and rel for external links.
            let is_external = self.ctx.href.as_str().starts_with("http://")
                || self.ctx.href.as_str().starts_with("https://");

            attrs.set(HtmlAttr::Href, self.ctx.href.as_str());

            let target = match &self.ctx.target {
                Some(t) => Some(t.as_str()),
                None if is_external => Some("_blank"),
                None => None,
            };
            if let Some(t) = target {
                attrs.set(HtmlAttr::Target, t);
            }

            let rel = match &self.ctx.rel {
                Some(r) => Some(r.as_str()),
                None if is_external && self.ctx.target.is_none() => {
                    Some("noopener noreferrer")
                }
                None => None,
            };
            if let Some(r) = rel {
                attrs.set(HtmlAttr::Rel, r);
            }

            // External links opening in a new tab get an announcement for AT users (§5.2).
            let opens_new_tab = target == Some("_blank");
            if opens_new_tab {
                attrs.set_bool(HtmlAttr::Data("ars-external"), true);
                attrs.set(
                    HtmlAttr::Aria(AriaAttr::Description),
                    (self.ctx.messages.external_link_label)(&self.ctx.locale),
                );
            }
        }

        // aria-current
        if let Some(ref current) = self.ctx.is_current {
            attrs.set(HtmlAttr::Aria(AriaAttr::Current), match current {
                AriaCurrent::Page     => "page",
                AriaCurrent::Step     => "step",
                AriaCurrent::Location => "location",
                AriaCurrent::Date     => "date",
                AriaCurrent::Time     => "time",
                AriaCurrent::True     => "true",
            });
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        // When the element is not a native <a>, add role and tabindex.
        // Consumers that render a non-<a> element should apply these.
        // role="link" — only needed when not rendering as <a>.
        // tabindex="0" — only needed when not rendering as <a>.

        attrs
    }

    /// Handles a click event on the link.
    pub fn on_click(&self) {
        (self.send)(Event::Navigate);
    }

    /// Handles a focus event on the link.
    pub fn on_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Handles a blur event on the link.
    pub fn on_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Handles a pointer down event on the link.
    pub fn on_pointer_down(&self) {
        (self.send)(Event::Press);
    }

    /// Handles a pointer up event on the link.
    pub fn on_pointer_up(&self) {
        (self.send)(Event::PressEnd);
    }

    /// Returns `true` when the link is disabled.
    pub fn is_disabled(&self) -> bool {
        self.ctx.disabled
    }

    /// Returns `true` when the link has keyboard-visible focus.
    pub fn is_focus_visible(&self) -> bool {
        self.ctx.focus_visible
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
Link
└── Root   <a> href="..."
```

| Part   | Element | Key Attributes                                                                                                                                        |
| ------ | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Root` | `<a>`   | `data-ars-scope="link"`, `data-ars-part="root"`, `data-ars-state`, `href`, `target`, `rel`, `aria-current`, `aria-disabled`, `data-ars-focus-visible` |

When the consumer renders a non-`<a>` element for the root, `root_props()` should be
extended with `role="link"` and `tabindex="0"` to maintain keyboard accessibility.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute       | Value                                    | Condition                   |
| --------------- | ---------------------------------------- | --------------------------- |
| `role`          | `link`                                   | When not rendering as `<a>` |
| `aria-current`  | `page\|step\|location\|date\|time\|true` | When `is_current` is set    |
| `aria-disabled` | `true`                                   | When disabled               |
| `tabindex`      | `0`                                      | When not rendering as `<a>` |

When rendered as a native `<a>` element, the browser provides link semantics automatically.
The `role="link"` attribute is only necessary when the consumer renders a non-anchor element
(e.g., `<span>` or `<div>`). Disabled links omit `href` entirely and set `aria-disabled="true"`
so screen readers announce the disabled state while keeping the element discoverable.

### 3.2 Keyboard Interaction

| Key     | Action                                                                    |
| ------- | ------------------------------------------------------------------------- |
| `Enter` | Activate the link.                                                        |
| `Space` | Activate the link (when `role="link"` is applied to a non-`<a>` element). |

Native `<a>` elements respond to `Enter` by default. `Space` activation is only relevant
when the link role is applied to a non-native element, matching the WAI-ARIA link pattern.

## 4. Router Integration and `link::Target`

`Link` supports both traditional `<a href>` navigation and client-side router navigation via the `Target` enum:

```rust
/// Distinguishes between standard href navigation and client-side routing.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// Standard URL — renders as `<a href="...">`. Navigates via browser.
    Href(SafeUrl),
    /// Client-side route — the adapter intercepts click and delegates to
    /// the framework router (e.g., Leptos `use_navigate()`, Dioxus `navigator()`).
    Route(String),
}
```

When `Target::Route` is used:

- The adapter calls `event.prevent_default()` on click and delegates to the framework's router.
- The rendered `<a>` element still receives an `href` attribute (for SSR crawlability and progressive enhancement). The adapter computes the href from the route string.
- **`is_current` auto-detection**: When `is_current` is `None` and the target is `Route(path)`, the adapter compares `path` against the current route. If they match, `is_current` is automatically set to `Some(AriaCurrent::Page)`. This enables automatic `aria-current="page"` highlighting for the active navigation item without requiring the consumer to track router state.

```rust,no_check
// Adapter-level auto-detection (e.g., in ars-leptos):
let current_path = use_location().pathname.get();
let auto_current = match &props.target {
    Target::Route(path) if props.is_current.is_none() => {
        if current_path == *path {
            Some(AriaCurrent::Page)
        } else {
            None
        }
    }
    _ => props.is_current.clone(),
};
```

The `Props` struct accepts `Target` via the `href` field. For backward compatibility, `SafeUrl` values are wrapped as `Target::Href` automatically.

## 5. External Link Detection

When `href` starts with `http://` or `https://` and `target` is not explicitly set,
`root_props()` automatically applies `target="_blank"` and `rel="noopener noreferrer"`.
This prevents the opened page from accessing `window.opener` and mitigates reverse
tabnapping attacks. When `target` is explicitly provided, the automatic behavior is
skipped — the consumer has full control.

### 5.1 Automatic `rel` for Explicit `target="_blank"`

When the consumer explicitly sets `target: Some("_blank".into())`, the adapter MUST automatically append `rel="noopener noreferrer"` if `rel` is `None`. This ensures security protections are always applied for new-tab links, regardless of whether the URL is detected as external. Consumers can override by explicitly setting the `rel` prop.

### 5.2 External Link Announcements and Iconography

External links (those with `target="_blank"`, whether auto-detected or explicit) MUST announce their behavior to assistive technology users:

- The adapter appends " (opens in new tab)" to the link's `aria-label`, or renders a visually hidden `<span class="sr-only">(opens in new tab)</span>` inside the link element.
- Optionally, the adapter renders an external link icon (e.g., `↗`) with `aria-hidden="true"` since the screen reader announcement already covers the semantics.
- The announcement text is provided via the `LinkMessages` struct to support localization.

## 6. Internationalization

### 6.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Announcement text appended to external links that open in a new tab.
    /// Default: "opens in new tab"
    pub external_link_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            external_link_label: MessageFn::static_str("opens in new tab"),
        }
    }
}

impl ComponentMessages for Messages {}
```

- **Text direction**: `Link` text direction follows the document direction. No special
  locale handling is needed — the browser's native `<a>` element respects `dir` inheritance.
- **Link labels**: All visible link text is consumer-provided. The `Messages` struct
  provides only the external link announcement text used in §5.2.

## 7. Library Parity

> Compared against: React Aria (`Link`).

### 7.1 Props

| Feature         | ars-ui                            | React Aria       | Notes                          |
| --------------- | --------------------------------- | ---------------- | ------------------------------ |
| Href            | `href: SafeUrl`                   | `href: string`   | ars-ui validates via SafeUrl   |
| Target          | `target`                          | `target`         | Full match                     |
| Rel             | `rel`                             | `rel`            | Full match                     |
| Disabled        | `disabled`                        | `isDisabled`     | Full match                     |
| Is current      | `is_current: Option<AriaCurrent>` | --               | ars-ui addition (aria-current) |
| Auto focus      | --                                | `autoFocus`      | Adapter concern                |
| Download        | --                                | `download`       | See below                      |
| Href lang       | --                                | `hrefLang`       | See below                      |
| Ping            | --                                | `ping`           | See below                      |
| Referrer policy | --                                | `referrerPolicy` | See below                      |
| Router options  | `Target::Route` (section 4)       | `routerOptions`  | ars-ui has client-side routing |

**Gaps:**

- **`download`**: React Aria supports the HTML `download` attribute for triggering file downloads. This is a native HTML attribute that the adapter can pass through without machine involvement. Adding it to Props is straightforward.
- **`hrefLang`**, **`ping`**, **`referrerPolicy`**: These are native HTML `<a>` attributes that React Aria passes through. They are rarely used and the adapter can pass them through directly. Not necessary as formal Props.

None of these are behavioral gaps that require state machine changes. They are HTML attribute passthroughs.

### 7.2 Anatomy

| Part | ars-ui         | React Aria | Notes      |
| ---- | -------------- | ---------- | ---------- |
| Root | `Root` (`<a>`) | `Link`     | Full match |

**Gaps:** None.

### 7.3 Events

| Callback | ars-ui                      | React Aria                    | Notes                                        |
| -------- | --------------------------- | ----------------------------- | -------------------------------------------- |
| Navigate | `Navigate` event            | `onPress`                     | ars-ui fires Navigate; adapter handles press |
| Focus    | `Focus` / `Blur` events     | `onFocus` / `onBlur`          | Full match                                   |
| Press    | `Press` / `PressEnd` events | `onPressStart` / `onPressEnd` | Full match                                   |
| Hover    | --                          | `onHoverStart` / `onHoverEnd` | See below                                    |
| Keyboard | --                          | `onKeyDown` / `onKeyUp`       | See below                                    |

**Gaps:**

- **Hover events**: React Aria provides `onHoverStart`/`onHoverEnd` render props. ars-ui tracks focus-visible and pressed states but does not track hover state in the machine. Hover is purely a CSS concern for links (`:hover` pseudo-class). Not a behavioral gap.
- **Keyboard events**: React Aria passes through `onKeyDown`/`onKeyUp`. These are raw DOM event handlers, not state machine concerns. The adapter can pass them through. Not a gap.

### 7.4 Features

| Feature                    | ars-ui                | React Aria          |
| -------------------------- | --------------------- | ------------------- |
| Disabled state             | Yes                   | Yes                 |
| aria-current support       | Yes (6 variants)      | No                  |
| External link detection    | Yes (auto target/rel) | No                  |
| External link announcement | Yes (Messages)        | No                  |
| SafeUrl validation         | Yes                   | No                  |
| Client-side routing        | Yes (Target::Route)   | Yes (routerOptions) |
| Focus visible tracking     | Yes                   | Yes                 |
| Press state tracking       | Yes                   | Yes                 |
| Hover state tracking       | No (CSS concern)      | Yes (render prop)   |

**Gaps:** None worth adopting.

### 7.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui uses `SafeUrl` for XSS-safe href validation, which React Aria does not have. ars-ui adds `is_current` for `aria-current` support and automatic external link detection/announcement. React Aria exposes more HTML passthrough attributes (`download`, `hrefLang`, `ping`, `referrerPolicy`) that are adapter concerns in ars-ui.
- **Recommended additions:** None.
