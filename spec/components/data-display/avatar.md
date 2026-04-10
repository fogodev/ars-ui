---
component: Avatar
category: data-display
tier: stateful
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    ark-ui: Avatar
    radix-ui: Avatar
---

# Avatar

An avatar displays a user's identity through their profile image, with a graceful fallback
to initials derived from the user's name when the image is unavailable or fails to load.

## 1. State Machine

### 1.1 States

| State      | Description                                                              |
| ---------- | ------------------------------------------------------------------------ |
| `Loading`  | Image src present; load in progress.                                     |
| `Loaded`   | Image loaded successfully; `<img>` is visible.                           |
| `Error`    | Image failed to load; fallback is shown.                                 |
| `Fallback` | No src provided; fallback shown immediately (or after `fallback_delay`). |

### 1.2 Events

| Event        | Payload  | Description                            |
| ------------ | -------- | -------------------------------------- |
| `ImageLoad`  | —        | The `<img>` onload fired successfully. |
| `ImageError` | —        | The `<img>` onerror fired.             |
| `SetSrc`     | `String` | The src prop changed; restart loading. |

### 1.3 Context

```rust
/// Context for the Avatar component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The image URL.
    pub src: Option<String>,
    /// Full name used to derive initials for the fallback.
    pub name: Option<String>,
    /// Current load phase.
    pub loading_status: LoadingStatus,
    /// Milliseconds to wait before revealing fallback (avoids FOUC when image loads quickly).
    pub fallback_delay: u32,
    /// Whether the fallback timer has elapsed.
    pub fallback_visible: bool,
    /// Resolved locale for message formatting.
    pub locale: Locale,
    /// Resolved messages for initials extraction.
    pub messages: Messages,
}

/// Current load phase of the avatar image.
#[derive(Clone, Debug, PartialEq)]
pub enum LoadingStatus {
    /// Image is loading.
    Loading,
    /// Image has loaded successfully.
    Loaded,
    /// Image failed to load.
    Error,
}
```

### 1.4 Props

```rust
/// Props for the Avatar component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Image URL.
    pub src: Option<String>,
    /// Full name for initials derivation and alt text.
    pub name: Option<String>,
    /// Delay (ms) before showing fallback; avoids flash when image loads fast.
    pub fallback_delay: u32,
    /// Visual size token.
    pub size: Size,
    /// Circle (default) or square crop.
    pub shape: Shape,
    /// Custom initials extraction logic. When provided, overrides the
    /// built-in locale-aware initials derivation from `name`.
    pub get_initials: Option<Callback<String, String>>,
    // Change callbacks (on_load, on_error) provided by the adapter layer
}

/// Visual size of the avatar.
#[derive(Clone, Debug, PartialEq)]
pub enum Size {
    /// Extra small size.
    Xs,
    /// Small size.
    Sm,
    /// Medium size.
    Md,
    /// Large size.
    Lg,
    /// Extra large size.
    Xl,
}

/// Visual shape of the avatar.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// Circle shape.
    Circle,
    /// Square shape.
    Square,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            src: None,
            name: None,
            fallback_delay: 600,
            size: Size::Md,
            shape: Shape::Circle,
            get_initials: None,
        }
    }
}
```

### 1.5 Initials Logic

The `initials(locale)` method applies the following locale-aware rules:

1. **CJK locales** (`zh`, `ja`, `ko`): Take the first 1-2 grapheme clusters directly from
   the name string (no word boundary splitting, since CJK names may not use whitespace).
2. **Latin/Cyrillic/Arabic/other scripts**: Split name on whitespace, take the first grapheme
   cluster of each word (max 2), uppercase the result.
3. **Mononym cultures**: If only one word is present, a single initial is returned.
4. **Custom override**: Supply an `Messages::initials_fn` to override the built-in
   logic for edge cases:

```rust
/// Messages for the Avatar component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Override initials extraction for locale-aware ordering.
    pub initials_fn: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
}
impl Default for Messages {
    fn default() -> Self {
        Self {
            initials_fn: MessageFn::new(|name, _locale| {
                let parts = name.split_whitespace().collect::<Vec<_>>();
                match parts.as_slice() {
                    [] => String::new(),
                    [single] => single.chars().next()
                        .map(|c| c.to_uppercase().to_string())
                        .unwrap_or_default(),
                    [first, .., last] => {
                        let first = first.chars().next().unwrap_or_default();
                        let last = last.chars().next().unwrap_or_default();
                        format!("{first}{last}").to_uppercase()
                    }
                }
            }),
        }
    }
}
impl ComponentMessages for Messages {}
```

### 1.6 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap};

/// States for the Avatar component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Image is loading.
    Loading,
    /// Image has loaded successfully.
    Loaded,
    /// Image failed to load.
    Error,
    /// Fallback is shown.
    Fallback,
}

/// Events for the Avatar component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The `<img>` onload fired successfully.
    ImageLoad,
    /// The `<img>` onerror fired.
    ImageError,
    /// The src prop changed; restart loading.
    SetSrc(String),
}

/// Machine for the Avatar component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let locale = env.locale.clone();
        let messages = messages.clone();
        let state = if props.src.is_some() { State::Loading } else { State::Fallback };
        (state, Context {
            src: props.src.clone(),
            name: props.name.clone(),
            loading_status: if props.src.is_some() {
                LoadingStatus::Loading
            } else {
                LoadingStatus::Error
            },
            fallback_delay: props.fallback_delay,
            fallback_visible: props.src.is_none(), // immediately visible when no src
            locale,
            messages,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx:   &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Loading, Event::ImageLoad) => {
                Some(TransitionPlan::to(State::Loaded).apply(|ctx| {
                    ctx.loading_status = LoadingStatus::Loaded;
                }))
                // on_load callback notification is handled by the adapter layer.
            }
            (State::Loading, Event::ImageError) => {
                Some(TransitionPlan::to(State::Error).apply(|ctx| {
                    ctx.loading_status = LoadingStatus::Error;
                    ctx.fallback_visible = true;
                }))
                // on_error callback notification is handled by the adapter layer.
            }
            (_, Event::SetSrc(new_src)) => {
                let src = new_src.clone();
                Some(TransitionPlan::to(State::Loading).apply(move |ctx| {
                    ctx.src = Some(src);
                    ctx.loading_status = LoadingStatus::Loading;
                    ctx.fallback_visible = false;
                }))
            }
            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx:   &'a Self::Context,
        props: &'a Self::Props,
        send:  &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "avatar"]
pub enum Part {
    Root,
    Image,
    Fallback,
}

/// API for the Avatar component.
pub struct Api<'a> {
    /// Current state of the avatar.
    state: &'a State,
    /// Current context of the avatar.
    ctx:   &'a Context,
    /// Current props of the avatar.
    props: &'a Props,
    /// Send event to the avatar.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Extracts initials from a name string, locale-aware.
    /// Delegates to `Messages::initials_fn` for locale-aware extraction.
    pub fn initials(&self) -> String {
        match &self.ctx.name {
            None => String::new(),
            Some(name) => (self.ctx.messages.initials_fn)(name, &self.ctx.locale),
        }
    }

    /// Checks if the image is visible.
    pub fn is_image_visible(&self) -> bool {
        *self.state == State::Loaded
    }

    /// Checks if the fallback is visible.
    pub fn is_fallback_visible(&self) -> bool {
        self.ctx.fallback_visible || matches!(self.state, State::Error | State::Fallback)
    }

    /// Root attributes for the avatar.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Data("ars-shape"), match self.props.shape {
            Shape::Circle => "circle",
            Shape::Square => "square",
        });
        p.set(HtmlAttr::Data("ars-size"), match self.props.size {
            Size::Xs => "xs", Size::Sm => "sm",
            Size::Md => "md", Size::Lg => "lg",
            Size::Xl => "xl",
        });
        p.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Loading  => "loading",
            State::Loaded   => "loaded",
            State::Error    => "error",
            State::Fallback => "fallback",
        });
        p
    }

    /// Image attributes for the avatar.
    pub fn image_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Image.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        if let Some(src) = &self.ctx.src {
            p.set(HtmlAttr::Src, src);
        }
        p.set(HtmlAttr::Alt, self.ctx.name.as_deref().unwrap_or_default());
        // Hidden from AT when fallback is shown
        if !self.is_image_visible() {
            p.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
            p.set_style(CssProperty::Display, "none");
        }
        // Event handlers (load/error for image) are typed methods on the Api struct.
        p
    }

    /// Fallback attributes for the avatar.
    pub fn fallback_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Fallback.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        if !self.is_fallback_visible() {
            p.set_style(CssProperty::Display, "none");
        }
        // Decorative when image is also present (screen readers read the <img> alt)
        p.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        p
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Image => self.image_attrs(),
            Part::Fallback => self.fallback_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Avatar
├── Root       (container; data-ars-scope="avatar" data-ars-part="root")
├── Image      (<img>; hidden when fallback visible)
└── Fallback   (span with initials; hidden when image loaded)
```

| Part       | Element            | Key Attributes                                        |
| ---------- | ------------------ | ----------------------------------------------------- |
| `Root`     | `<span>` / `<div>` | `data-ars-shape`, `data-ars-size`, `data-ars-state`   |
| `Image`    | `<img>`            | `src`, `alt="{name}"`, `aria-hidden` when not visible |
| `Fallback` | `<span>`           | `aria-hidden="true"` (decorative)                     |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- `<img>` carries `alt="{name}"` for screen readers.
- When the image fails and fallback is shown, the `<img>` gets `aria-hidden="true"` so
  screen readers do not announce a broken image.
- The `Fallback` span is `aria-hidden="true"` since the `<img>` alt already conveys the name.
- If the avatar is used in a context where no name is available, the `Root` should receive
  `aria-label` from the parent component.
- The `name: String` prop MUST be used as `aria-label` on the Root element so that the
  avatar always has an accessible name, regardless of image load state.
- The `<img>` element gets `alt={name}` for screen readers.
- When the image fails to load and a fallback (initials) is shown, the `aria-label` with
  the `name` on the Root ensures the accessible name is preserved.

## 4. Internationalization

- Initials extraction is locale-aware (see §1.5 above).
- The `fallback_delay` prevents a flash of fallback on fast connections.

## 5. Library Parity

> Compared against: Ark UI (`Avatar`), Radix UI (`Avatar`).

### 5.1 Props

| Feature            | ars-ui             | Ark UI                   | Radix UI                 | Notes                                       |
| ------------------ | ------------------ | ------------------------ | ------------------------ | ------------------------------------------- |
| `src`              | `Option<String>`   | -- (set via Image child) | -- (set via Image child) | ars-ui lifts src to props for state machine |
| `fallback_delay`   | `u32` (ms)         | --                       | `delayMs` on Fallback    | Same concept, different location            |
| `name`             | `Option<String>`   | --                       | --                       | ars-ui original for initials derivation     |
| `size`             | `Size` enum        | --                       | --                       | ars-ui original styling token               |
| `shape`            | `Shape` enum       | --                       | --                       | ars-ui original styling token               |
| `get_initials`     | `Option<Callback>` | --                       | --                       | ars-ui original custom initials             |
| `on_status_change` | Adapter layer      | `onStatusChange`         | `onLoadingStatusChange`  | Same behavior, adapter-wired                |

**Gaps:** None.

### 5.2 Anatomy

| Part     | ars-ui     | Ark UI     | Radix UI   | Notes |
| -------- | ---------- | ---------- | ---------- | ----- |
| Root     | `Root`     | `Root`     | `Root`     | --    |
| Image    | `Image`    | `Image`    | `Image`    | --    |
| Fallback | `Fallback` | `Fallback` | `Fallback` | --    |

**Gaps:** None.

### 5.3 Events

| Callback      | ars-ui                           | Ark UI           | Radix UI                | Notes                                  |
| ------------- | -------------------------------- | ---------------- | ----------------------- | -------------------------------------- |
| Status change | `on_load` / `on_error` (adapter) | `onStatusChange` | `onLoadingStatusChange` | ars-ui splits into two typed callbacks |

**Gaps:** None.

### 5.4 Features

| Feature              | ars-ui                              | Ark UI             | Radix UI                        |
| -------------------- | ----------------------------------- | ------------------ | ------------------------------- |
| Image loading states | Yes (Loading/Loaded/Error/Fallback) | Yes (loaded/error) | Yes (idle/loading/loaded/error) |
| Fallback delay       | Yes (`fallback_delay`)              | No                 | Yes (`delayMs`)                 |
| Initials derivation  | Yes (locale-aware)                  | No                 | No                              |
| Size/shape tokens    | Yes                                 | No                 | No                              |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** Ark UI and Radix UI treat Avatar as a thin image-with-fallback wrapper. ars-ui extends this with locale-aware initials derivation, size/shape styling tokens, and name-based alt text -- features that neither reference library provides.
- **Recommended additions:** None.
