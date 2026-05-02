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
to initials derived from the user's name, and then to a default icon marker when no initials
can be resolved.

## 1. State Machine

### 1.1 States

| State      | Description                                    |
| ---------- | ---------------------------------------------- |
| `Loading`  | Image src present; load in progress.           |
| `Loaded`   | Image loaded successfully; `<img>` is visible. |
| `Error`    | Image failed to load; fallback is shown.       |
| `Fallback` | No src provided; fallback shown immediately.   |

### 1.2 Events

| Event                  | Payload            | Description                                                    |
| ---------------------- | ------------------ | -------------------------------------------------------------- |
| `ImageLoad`            | —                  | The `<img>` onload fired successfully.                         |
| `ImageError`           | —                  | The `<img>` onerror fired.                                     |
| `SetSrc`               | `Option<ImageSrc>` | The src prop changed; restart loading or show fallback.        |
| `FallbackDelayElapsed` | —                  | The fallback reveal delay elapsed while the image was loading. |

### 1.3 Context

```rust
/// Context for the Avatar component.
#[derive(Clone, Debug)]
pub struct Context {
    /// Validated image URL.
    pub src: Option<ImageSrc>,
    /// Current load phase.
    pub loading_status: LoadingStatus,
    /// Whether fallback content is visible.
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
use core::time::Duration;

/// Props for the Avatar component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Validated image URL.
    pub src: Option<ImageSrc>,
    /// Full name for initials derivation and default accessible text.
    pub name: Option<String>,
    /// Explicit accessible label for the root image wrapper.
    pub aria_label: Option<String>,
    /// Delay before showing fallback; avoids flash when image loads fast.
    pub fallback_delay: Duration,
    /// Visual size token.
    pub size: Size,
    /// Circle (default) or square crop.
    pub shape: Shape,
    /// Custom initials extraction logic. When provided, overrides
    /// `Messages::initials_fn` for this avatar instance.
    pub get_initials: Option<Callback<dyn Fn(String) -> String + Send + Sync>>,
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

impl Size {
    /// Returns the `data-ars-size` value for this avatar size.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Xs => "xs",
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
            Self::Xl => "xl",
        }
    }
}

impl Shape {
    /// Returns the `data-ars-shape` value for this avatar shape.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Circle => "circle",
            Self::Square => "square",
        }
    }
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            src: None,
            name: None,
            aria_label: None,
            fallback_delay: Duration::from_millis(600),
            size: Size::Md,
            shape: Shape::Circle,
            get_initials: None,
        }
    }
}
```

`Props::new()` returns `Props::default()`. Builder setters exist for every public field,
including `id`, `src`, `try_src`, `no_src`, `name`, `no_name`, `aria_label`, `no_aria_label`,
`fallback_delay`, `size`, `shape`, `get_initials`, and `no_get_initials`.
The infallible `src` setter accepts a pre-validated [`ImageSrc`], including values converted
from [`SafeUrl`]. Dynamic string sources use `try_src`, which returns [`ImageSrcError`] when
the source is unsafe. `ImageSrc` accepts the shared safe URL policy plus browser-generated
`blob:` URLs and selected raster `data:image/*;base64,` URLs (`png`, `jpeg`, `jpg`, `gif`,
`webp`, `avif`). SVG data URLs and script URLs are rejected. A `fallback_delay` of
`Duration::ZERO` makes fallback content visible immediately while an image source is loading.

### 1.5 Initials Logic

The `initials(locale)` method applies the following locale-aware rules:

1. **CJK locales** (`zh`, `ja`, `ko`): Take the first 1-2 grapheme clusters directly from
   the name string (no word boundary splitting, since CJK names may not use whitespace).
2. **Latin/Cyrillic/Arabic/other scripts**: Split name on whitespace, take the first grapheme
   cluster of each word (max 2), uppercase the result.
3. **Mononym cultures**: If only one word is present, a single initial is returned.
4. **Per-instance custom override**: Supply `Props::get_initials` to override initials
   extraction for a specific avatar instance.
5. **Message-level custom override**: Supply `Messages::initials_fn` to override the
   built-in logic for locale-aware edge cases across avatar instances:

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
            initials_fn: MessageFn::new(|name, locale| {
                if matches!(locale.language(), "zh" | "ja" | "ko") {
                    return take_graphemes(name, 2);
                }

                let parts = name.split_whitespace().collect::<Vec<_>>();
                match parts.as_slice() {
                    [] => String::new(),
                    [single] => take_graphemes(single, 1).to_uppercase(),
                    [first, .., last] => {
                        let first = take_graphemes(first, 1);
                        let last = take_graphemes(last, 1);
                        format!("{first}{last}").to_uppercase()
                    }
                }
            }),
        }
    }
}
impl ComponentMessages for Messages {}

/// Renderable fallback content resolved by the Avatar API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FallbackContent {
    /// Text initials derived from the avatar name.
    Initials(String),
    /// Default icon fallback when no initials are available.
    Icon,
}
```

### 1.6 Full Machine Implementation

```rust
use ars_core::{AttrMap, TransitionPlan};

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
    SetSrc(Option<ImageSrc>),
    /// The fallback reveal delay elapsed while loading.
    FallbackDelayElapsed,
}

/// Machine for the Avatar component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Messages = Messages;
    type Effect = ars_core::NoEffect;
    type Api<'a> = Api<'a>;

     fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
         let locale = env.locale.clone();
         let messages = messages.clone();
         let state = if props.src.is_some() { State::Loading } else { State::Fallback };
         (state, Context {
             src: props.src.clone(),
             loading_status: if props.src.is_some() {
                 LoadingStatus::Loading
             } else {
                 LoadingStatus::Error
             },
            fallback_visible: props.src.is_none() || props.fallback_delay == Duration::ZERO,
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
                    ctx.fallback_visible = false;
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
            (State::Loading, Event::FallbackDelayElapsed) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.fallback_visible = true;
                }))
            }
            (_, Event::SetSrc(Some(new_src))) => {
                let src = new_src.clone();
                let fallback_visible = props.fallback_delay == Duration::ZERO;
                Some(TransitionPlan::to(State::Loading).apply(move |ctx| {
                    ctx.src = Some(src);
                    ctx.loading_status = LoadingStatus::Loading;
                    ctx.fallback_visible = fallback_visible;
                }))
            }
            (_, Event::SetSrc(None)) => {
                Some(TransitionPlan::to(State::Fallback).apply(|ctx| {
                    ctx.src = None;
                    ctx.loading_status = LoadingStatus::Error;
                    ctx.fallback_visible = true;
                }))
            }
            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        if old.src == new.src {
            Vec::new()
        } else {
            vec![Event::SetSrc(new.src.clone())]
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GroupPart {
    Group,
    GroupItem { index: usize },
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
    /// Uses `Props::get_initials` first, then delegates to
    /// `Messages::initials_fn` for locale-aware extraction.
    pub fn initials(&self) -> String {
        match &self.props.name {
            None => String::new(),
            Some(name) => match &self.props.get_initials {
                Some(get_initials) => get_initials(name.clone()),
                None => (self.ctx.messages.initials_fn)(name, &self.ctx.locale),
            },
        }
    }

    /// Returns the fallback content adapters should render.
    pub fn fallback_content(&self) -> FallbackContent {
        let initials = self.initials();
        if initials.is_empty() {
            FallbackContent::Icon
        } else {
            FallbackContent::Initials(initials)
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
        p.set(HtmlAttr::Role, "img");
        if !self.props.id.is_empty() {
            p.set(HtmlAttr::Id, self.props.id.as_str());
        }
        if let Some(label) = self.props.aria_label.as_ref().or(self.props.name.as_ref()) {
            p.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }
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
            p.set(HtmlAttr::Src, src.as_str());
        }
        // The root is the single accessible image; the inner img is presentational.
        p.set(HtmlAttr::Alt, "");
        p.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        if !self.is_image_visible() {
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
        p.set(HtmlAttr::Data("ars-fallback"), match self.fallback_content() {
            FallbackContent::Initials(_) => "initials",
            FallbackContent::Icon => "icon",
        });
        if !self.is_fallback_visible() {
            p.set_style(CssProperty::Display, "none");
        }
        // Decorative because the root carries the accessible image name.
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

/// Props for an avatar stack group.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct GroupProps {
    /// Component instance ID.
    pub id: String,
    /// Visual size token applied to all grouped avatars.
    pub size: Size,
    /// Visual shape token applied to all grouped avatars.
    pub shape: Shape,
    /// CSS overlap amount between adjacent avatars.
    pub overlap: String,
    /// Accessible label for the avatar group.
    pub aria_label: Option<String>,
}

impl Default for GroupProps {
    fn default() -> Self {
        Self {
            id: String::new(),
            size: Size::Md,
            shape: Shape::Circle,
            overlap: String::from("0.5rem"),
            aria_label: None,
        }
    }
}

impl GroupProps {
    pub fn new() -> Self { Self::default() }
    pub fn id(mut self, id: impl Into<String>) -> Self { self.id = id.into(); self }
    pub fn size(mut self, size: Size) -> Self { self.size = size; self }
    pub fn shape(mut self, shape: Shape) -> Self { self.shape = shape; self }
    pub fn overlap(mut self, overlap: impl Into<String>) -> Self { self.overlap = overlap.into(); self }
    pub fn aria_label(mut self, label: impl Into<String>) -> Self { self.aria_label = Some(label.into()); self }
    pub fn no_aria_label(mut self) -> Self { self.aria_label = None; self }
}

/// API for avatar stack group attributes.
pub struct GroupApi {
    props: GroupProps,
}

impl GroupApi {
    /// Creates a new avatar group API.
    pub fn new(props: GroupProps) -> Self { Self { props } }

    /// Group attributes for an avatar stack.
    pub fn group_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = GroupPart::Group.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Data("ars-size"), self.props.size.as_str());
        p.set(HtmlAttr::Data("ars-shape"), self.props.shape.as_str());
        p.set_style(CssProperty::Custom("ars-avatar-group-overlap"), self.props.overlap.as_str());
        if !self.props.id.is_empty() {
            p.set(HtmlAttr::Id, self.props.id.as_str());
        }
        if let Some(label) = &self.props.aria_label {
            p.set(HtmlAttr::Role, "group");
            p.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }
        p
    }

    /// Item attributes for an avatar stack child.
    pub fn group_item_attrs(&self, index: usize) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            GroupPart::GroupItem { index }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Data("ars-index"), index.to_string());
        p.set_style(CssProperty::Custom("ars-avatar-group-index"), index.to_string());
        p
    }
}
```

## 2. Anatomy

```text
Avatar
├── Root         (container; data-ars-scope="avatar" data-ars-part="root")
├── Image        (<img>; hidden when fallback visible)
├── Fallback     (span with initials or default icon; hidden when image loaded)
└── Group        (optional avatar stack container)
    └── GroupItem (wrapper for each child avatar in a stack)
```

| Part        | Element            | Key Attributes                                                                        |
| ----------- | ------------------ | ------------------------------------------------------------------------------------- |
| `Root`      | `<span>` / `<div>` | `id`, `role="img"`, `aria-label`, `data-ars-shape`, `data-ars-size`, `data-ars-state` |
| `Image`     | `<img>`            | `src`, `alt=""`, `aria-hidden="true"`, hidden when not visible                        |
| `Fallback`  | `<span>`           | `aria-hidden="true"`, `data-ars-fallback="initials\|icon"`                            |
| `Group`     | `<div>`            | `id`, `role="group"` when labelled, `data-ars-size`, `data-ars-shape`                 |
| `GroupItem` | `<span>`           | `data-ars-index`, `--ars-avatar-group-index`                                          |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- The `Root` is the single accessible image and carries `role="img"`.
- `Root` uses `Props::aria_label` when supplied, otherwise `Props::name` when supplied.
- The inner `<img>` is presentational with `alt=""` and `aria-hidden="true"` in every state.
- When the image is not visible, the `<img>` also gets `display: none`.
- The `Fallback` span is decorative and always has `aria-hidden="true"`.
- Avatar groups use `role="group"` only when `aria_label` is supplied; unlabeled stacks
  remain structural wrappers with data attributes only.

## 4. Internationalization

- Initials extraction is locale-aware (see §1.5 above).
- A `fallback_delay` of `Duration::ZERO` makes fallback visible immediately even while an image is loading.
- Non-zero `fallback_delay` prevents a flash of fallback on fast connections. Because the core
  machine emits `NoEffect`, adapters own the timer and must ignore or cancel stale timer
  callbacks after `ImageLoad`, `ImageError`, or `SetSrc`.

## 5. Library Parity

> Compared against: Ark UI (`Avatar`), Radix UI (`Avatar`).

### 5.1 Props

| Feature            | ars-ui             | Ark UI                   | Radix UI                 | Notes                                                |
| ------------------ | ------------------ | ------------------------ | ------------------------ | ---------------------------------------------------- |
| `src`              | `Option<ImageSrc>` | -- (set via Image child) | -- (set via Image child) | ars-ui lifts and validates src for the state machine |
| `fallback_delay`   | `Duration`         | --                       | `delayMs` on Fallback    | Same concept, different location                     |
| `name`             | `Option<String>`   | --                       | --                       | ars-ui original for initials derivation              |
| `aria_label`       | `Option<String>`   | --                       | --                       | Explicit label for nameless or contextual avatars    |
| `size`             | `Size` enum        | --                       | --                       | ars-ui original styling token                        |
| `shape`            | `Shape` enum       | --                       | --                       | ars-ui original styling token                        |
| `get_initials`     | `Option<Callback>` | --                       | --                       | ars-ui original custom initials                      |
| `GroupProps`       | Stack group API    | --                       | --                       | ars-ui original avatar stack support                 |
| `on_status_change` | Adapter layer      | `onStatusChange`         | `onLoadingStatusChange`  | Same behavior, adapter-wired                         |

**Gaps:** None.

### 5.2 Anatomy

| Part      | ars-ui      | Ark UI     | Radix UI   | Notes                         |
| --------- | ----------- | ---------- | ---------- | ----------------------------- |
| Root      | `Root`      | `Root`     | `Root`     | --                            |
| Image     | `Image`     | `Image`    | `Image`    | --                            |
| Fallback  | `Fallback`  | `Fallback` | `Fallback` | --                            |
| Group     | `Group`     | --         | --         | ars-ui avatar stack support   |
| GroupItem | `GroupItem` | --         | --         | ars-ui avatar stack child API |

**Gaps:** None.

### 5.3 Events

| Callback      | ars-ui                           | Ark UI           | Radix UI                | Notes                                  |
| ------------- | -------------------------------- | ---------------- | ----------------------- | -------------------------------------- |
| Status change | `on_load` / `on_error` (adapter) | `onStatusChange` | `onLoadingStatusChange` | ars-ui splits into two typed callbacks |

**Gaps:** None.

### 5.4 Features

| Feature               | ars-ui                              | Ark UI             | Radix UI                        |
| --------------------- | ----------------------------------- | ------------------ | ------------------------------- |
| Image loading states  | Yes (Loading/Loaded/Error/Fallback) | Yes (loaded/error) | Yes (idle/loading/loaded/error) |
| Fallback delay        | Yes (`fallback_delay`)              | No                 | Yes (`delayMs`)                 |
| Initials derivation   | Yes (locale-aware)                  | No                 | No                              |
| Default icon fallback | Yes (`FallbackContent::Icon`)       | Yes                | Yes                             |
| Size/shape tokens     | Yes                                 | No                 | No                              |
| Avatar stack group    | Yes (`GroupApi`)                    | No                 | No                              |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Base Avatar parity plus ars-ui extensions.
- **Divergences:** Ark UI and Radix UI treat Avatar as a thin image-with-fallback wrapper. ars-ui extends this with locale-aware initials derivation, size/shape styling tokens, root-level accessible naming, and a small agnostic group API -- features that neither reference library provides as framework-agnostic core.
- **Recommended additions:** None.
