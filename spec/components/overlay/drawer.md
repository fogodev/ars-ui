---
component: Drawer
category: overlay
tier: complex
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [z-index-stacking]
related: [dialog]
references:
    ark-ui: Dialog
---

# Drawer

A dialog that slides in from a screen edge.

## 1. State Machine

### 1.1 States

```rust
/// The state of the drawer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The drawer is closed.
    Closed,
    /// The drawer is open.
    Open,
    /// The drawer is being dragged.
    Dragging(f64),
}
```

### 1.2 Events

```rust
/// The events of the drawer.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open the drawer.
    Open,
    /// Close the drawer.
    Close,
    /// Toggle the drawer.
    Toggle,
    /// Start dragging the drawer.
    DragStart(f64),
    /// Move the drawer.
    DragMove(f64),
    /// End dragging the drawer.
    DragEnd {
        /// Final normalized drag offset. `0.0` is fully open; `1.0` is fully dismissed.
        offset: f64,
        /// Final normalized velocity. Positive moves toward dismissal; negative moves toward expansion.
        velocity: f64,
    },
    /// Snap to a snap point.
    SnapTo(usize),
    /// Adapter reported the z-index allocated for an active request.
    SetZIndex {
        /// Request id read from `Context::z_index_request` when processing `AllocateZIndex`.
        request_id: u64,
        /// Allocated z-index value.
        z_index: u32,
    },
    /// Close the drawer on backdrop click.
    CloseOnBackdropClick,
    /// Close the drawer on escape key.
    CloseOnEscape,
    /// Register the title element for `aria-labelledby`.
    RegisterTitle,
    /// Unregister the title element from `aria-labelledby`.
    UnregisterTitle,
    /// Register the description element for `aria-describedby`.
    RegisterDescription,
    /// Unregister the description element from `aria-describedby`.
    UnregisterDescription,
    /// Re-apply context-backed props after a prop change.
    SyncProps,
}
```

### 1.3 Context

```rust
/// The context of the drawer.
/// Follows the Dialog pattern (see Dialog §1.3) with drawer-specific additions.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the drawer is open.
    pub open: bool,
    /// Whether the drawer is modal.
    pub modal: bool,
    /// The placement of the drawer.
    pub placement: Placement,
    /// The resolved physical placement (after Start/End → Left/Right resolution).
    pub resolved_placement: ResolvedPlacement,
    /// Whether the drawer is closeable on backdrop click.
    pub close_on_backdrop: bool,
    /// Whether the drawer is closeable on escape key.
    pub close_on_escape: bool,
    /// Whether the drawer should prevent scroll.
    pub prevent_scroll: bool,
    /// Whether the drawer should restore focus when it closes.
    pub restore_focus: bool,
    /// Initial focus target resolved by the adapter when the drawer opens.
    pub initial_focus: Option<FocusTarget>,
    /// Final focus target resolved by the adapter when the drawer closes.
    pub final_focus: Option<FocusTarget>,
    /// Component instance IDs.
    pub ids: ComponentIds,
    /// Whether a title part was registered.
    pub has_title: bool,
    /// Whether a description part was registered.
    pub has_description: bool,
    /// Sanitized snap-point fractions in ascending order.
    pub snap_points: Vec<f64>,
    /// Index of the currently active snap point.
    pub current_snap: usize,
    /// The resolved height fraction at the current snap.
    pub snap_height: f64,
    /// Adapter-allocated z-index rendered as `--ars-z-index`.
    pub z_index: Option<u32>,
    /// Monotonic request id for correlating z-index allocation feedback.
    pub z_index_request: u64,
    /// The current locale for message resolution.
    pub locale: Locale,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// The props of the drawer.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled open state. When `Some`, the consumer controls open/close.
    pub open: Option<bool>,
    /// Whether the drawer is open by default (uncontrolled). Default: false.
    pub default_open: bool,
    /// The placement of the drawer.
    pub placement: Placement,
    /// Whether the drawer is modal. Default: true.
    pub modal: bool,
    /// Whether the drawer is closeable on backdrop click. Default: true.
    pub close_on_backdrop: bool,
    /// Whether the drawer is closeable on escape key. Default: true.
    pub close_on_escape: bool,
    /// Whether the drawer should prevent scroll. Default: true.
    pub prevent_scroll: bool,
    /// Whether the drawer should restore focus to the trigger on close. Default: true.
    pub restore_focus: bool,
    /// The initial focus target when the drawer opens.
    pub initial_focus: Option<FocusTarget>,
    /// The element to receive focus when the drawer closes.
    pub final_focus: Option<FocusTarget>,
    /// Text direction for logical placement resolution.
    pub dir: Direction,
    /// Heading level for the Title part (renders as `<h{level}>`). Default: 2.
    pub title_level: u8,
    /// Snap points for bottom sheet behavior (see §5. Variant: Bottom Sheet).
    pub snap_points: Option<Vec<f64>>,
    /// Index into `snap_points` for the initial position. Defaults to 0.
    pub default_snap_index: usize,
    /// Callback invoked when the drawer open state changes.
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
    /// When true, drawer content is not mounted until first opened. Default: false.
    pub lazy_mount: bool,
    /// When true, drawer content is removed from the DOM after closing. Default: false.
    pub unmount_on_exit: bool,
    /// Callback invoked before Escape dismissal.
    pub on_escape_key_down: Option<Callback<dyn Fn(DismissAttempt<()>) + Send + Sync>>,
    /// Callback invoked before outside/backdrop dismissal.
    pub on_interact_outside: Option<Callback<dyn Fn(DismissAttempt<()>) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: false,
            placement: Placement::Right,
            modal: true,
            close_on_backdrop: true,
            close_on_escape: true,
            prevent_scroll: true,
            restore_focus: true,
            initial_focus: None,
            final_focus: None,
            dir: Direction::Ltr,
            title_level: 2,
            snap_points: None,
            default_snap_index: 0,
            on_open_change: None,
            lazy_mount: false,
            unmount_on_exit: false,
            on_escape_key_down: None,
            on_interact_outside: None,
        }
    }
}
```

### 1.5 Placement Resolution

```rust
/// The placement of the drawer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    /// The top of the screen.
    Top,
    /// The bottom of the screen.
    Bottom,
    /// The left of the screen.
    Left,
    /// The right of the screen.
    Right,
    /// The start of the screen.
    /// Logical: Left in LTR, Right in RTL
    Start,
    /// The end of the screen.
    /// Logical: Right in LTR, Left in RTL
    End,
}

impl Placement {
    /// Converts the logical placement to a physical placement based on direction.
    pub fn to_physical(self, dir: ResolvedDirection) -> ResolvedPlacement {
        match (self, dir) {
            (Self::Start, ResolvedDirection::Ltr) => ResolvedPlacement::Left,
            (Self::Start, ResolvedDirection::Rtl) => ResolvedPlacement::Right,
            (Self::End, ResolvedDirection::Ltr) => ResolvedPlacement::Right,
            (Self::End, ResolvedDirection::Rtl) => ResolvedPlacement::Left,
            (Self::Top, _) => ResolvedPlacement::Top,
            (Self::Bottom, _) => ResolvedPlacement::Bottom,
            (Self::Left, _) => ResolvedPlacement::Left,
            (Self::Right, _) => ResolvedPlacement::Right,
        }
    }

    /// The CSS translation for the drawer based on the placement.
    pub fn as_css_translate(&self) -> &'static str {
        match self {
            Self::Bottom => "translateY(100%)",
            Self::Top => "translateY(-100%)",
            Self::Left => "translateX(-100%)",
            Self::Right => "translateX(100%)",
            _ => "translateX(100%)",
        }
    }
}

/// Physical placement after resolving logical Start/End directions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResolvedPlacement {
    /// The top of the screen.
    Top,
    /// The bottom of the screen.
    Bottom,
    /// The left of the screen.
    Left,
    /// The right of the screen.
    Right,
}

impl ResolvedPlacement {
    /// The data attribute token for this physical placement.
    pub fn as_data_attr(&self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    /// The CSS translation for the drawer based on the resolved placement.
    pub fn as_css_translate(&self) -> &'static str {
        match self {
            Self::Bottom => "translateY(100%)",
            Self::Top => "translateY(-100%)",
            Self::Left => "translateX(-100%)",
            Self::Right => "translateX(100%)",
        }
    }
}
```

### 1.6 Full Machine Implementation

Drawer follows the Dialog Machine pattern (see [Dialog §1.9](./dialog.md#19-full-machine-implementation)) for open/close transitions, scroll lock, inert attribute management, and focus management.

In `init()`, semantic IDs MUST be derived from `ComponentIds` and used only for ARIA wiring and hydration-stable `id` attributes:

```rust,no_check
ctx.ids = ComponentIds::from_id(&props.id);
ctx.has_title = false;
ctx.has_description = false;
```

The key additions are:

- The `Dragging(f64)` state tracks drag position during swipe-to-dismiss gestures.
- `DragStart`, `DragMove`, `DragEnd { offset, velocity }` events handle adapter-normalized drag interactions. The adapter supplies normalized values only; the core owns snap and dismiss math.
- `SnapTo(usize)` event handles keyboard-initiated snap transitions (see §5 Bottom Sheet).
- `SetZIndex { request_id, z_index }` stores the adapter-allocated z-index only while the drawer is open or dragging and the request id matches `Context::z_index_request`; late adapter acknowledgements after close or after a close/reopen cycle are ignored.
- `RegisterTitle` / `UnregisterTitle` and `RegisterDescription` / `UnregisterDescription` gate `aria-labelledby` / `aria-describedby` so optional title and description parts can mount and unmount without stale ARIA IDREFs.
- `SyncProps` replays context-backed props after prop changes.
- Controlled opening queues `SyncProps` before `Open` so opening effects use current props. Controlled closing queues `Close` before `SyncProps` so the core does not emit acquire effects for props that only apply after the drawer is closed.
- Scroll lock, inert, z-index, and focus effects are represented as adapter-resolvable intents. The core never measures layout, traps focus, captures pointers, restores focus by ID lookup, or traverses the DOM.

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "drawer"]
pub enum Part {
    Root,
    Trigger,
    Backdrop,
    Positioner,
    Content,
    Title,
    Description,
    Header,
    Body,
    Footer,
    CloseTrigger,
    DragHandle,
}

/// The API for the `Drawer` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn is_open(&self) -> bool { *self.state != State::Closed }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let state_str = match self.state {
            State::Closed => "closed",
            State::Open => "open",
            State::Dragging(_) => "open",
        };
        attrs.set(HtmlAttr::Data("ars-state"), state_str);
        attrs
    }

    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.is_open() { "true" } else { "false" });
        attrs
    }

    pub fn on_trigger_click(&self) { (self.send)(Event::Toggle); }

    pub fn backdrop_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Backdrop.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        if !self.ctx.close_on_backdrop {
            attrs.set(HtmlAttr::Inert, "");
        }
        attrs
    }

    pub fn on_backdrop_click(&self) { (self.send)(Event::CloseOnBackdropClick); }

    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "true");
        attrs.set(HtmlAttr::Aria(AriaAttr::RoleDescription), (self.ctx.messages.role_description)(&self.ctx.locale));
        if self.ctx.has_title {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("title"));
        }
        if self.ctx.has_description {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), self.ctx.ids.part("description"));
        }
        attrs.set(HtmlAttr::Data("ars-placement"), self.ctx.resolved_placement.as_data_attr());
        if matches!(self.state, State::Dragging(_)) {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }
        attrs
    }

    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("title"));
        attrs
    }

    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        attrs
    }

    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Escape {
            (self.send)(Event::CloseOnEscape);
        }
    }

    pub fn header_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Header.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
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

    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.close_label)(&self.ctx.locale));
        attrs
    }

    pub fn on_close_trigger_click(&self) { (self.send)(Event::Close); }

    pub fn drag_handle_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DragHandle.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Backdrop => self.backdrop_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Title => self.title_attrs(),
            Part::Description => self.description_attrs(),
            Part::Header => self.header_attrs(),
            Part::Body => self.body_attrs(),
            Part::Footer => self.footer_attrs(),
            Part::CloseTrigger => self.close_trigger_attrs(),
            Part::DragHandle => self.drag_handle_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Drawer
├── Root             (required)
├── Trigger          (required)
├── Backdrop         (required)
├── Positioner       (required)
├── Content          (required — slides in from edge)
├── Title            (optional — labels the drawer via aria-labelledby)
├── Description      (optional — describes the drawer via aria-describedby)
├── Header           (optional)
├── Body             (optional)
├── Footer           (optional)
├── CloseTrigger     (optional)
└── DragHandle       (optional — for snap point drag interaction)
```

| Part         | Element    | Key Attributes                                                                                                     |
| ------------ | ---------- | ------------------------------------------------------------------------------------------------------------------ |
| Root         | `<div>`    | `data-ars-scope="drawer"`, `data-ars-state`                                                                        |
| Trigger      | `<button>` | `aria-haspopup="dialog"`, `aria-expanded`                                                                          |
| Backdrop     | `<div>`    | `aria-hidden="true"`, `inert` only when `close_on_backdrop` is false                                                |
| Positioner   | `<div>`    | `data-ars-scope="drawer"`, `data-ars-placement`, `--ars-z-index` when allocated                                    |
| Content      | `<div>`    | `role="dialog"`, `aria-modal`, `aria-roledescription`, `aria-labelledby`, `aria-describedby`, `data-ars-placement` |
| Title        | `<h{n}>`   | `id` for `aria-labelledby` on Content                                                                              |
| Description  | `<div>`    | `id` for `aria-describedby` on Content                                                                             |
| Header       | `<div>`    | `data-ars-scope="drawer"`, `data-ars-part="header"`                                                                |
| Body         | `<div>`    | `data-ars-scope="drawer"`, `data-ars-part="body"`                                                                  |
| Footer       | `<div>`    | `data-ars-scope="drawer"`, `data-ars-part="footer"`                                                                |
| CloseTrigger | `<button>` | `aria-label` from Messages                                                                                         |
| DragHandle   | `<div>`    | `role="slider"` with an accessible name and `tabindex="0"` (when bottom-sheet snap points are active)               |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

Same as Dialog (see [Dialog §3.1](./dialog.md#31-aria-roles-states-and-properties)), with the following additions:

- The `Drawer` content element MUST set `aria-roledescription` to `(self.ctx.messages.role_description)(&self.ctx.locale)` per `03-accessibility.md` §2.9 to distinguish it from a generic dialog.
- The close trigger MUST use `(self.ctx.messages.close_label)(&self.ctx.locale)` for its `aria-label`.

The agnostic core resolves logical placements (Start/End) to physical directions
from `Props::dir`. Adapters provide the document `Direction` when constructing
props; they do not duplicate placement resolution. When `Props::dir` is
`Direction::Auto`, the core resolves it against the active environment locale.
Start → Left in LTR and Right in RTL.

### 3.2 Keyboard Interaction

| Key        | Action                                                   |
| ---------- | -------------------------------------------------------- |
| Escape     | Close the drawer                                         |
| Tab        | Cycle focus within drawer content                        |
| Arrow Up   | Move to the next larger snap point (expand)              |
| Arrow Down | Move to the next smaller snap point (collapse)           |
| Page Up    | Move to the next larger snap point (same as Arrow Up)    |
| Page Down  | Move to the next smaller snap point (same as Arrow Down) |
| Home       | Move to the minimum snap index                           |
| End        | Move to the maximum snap index                           |

Arrow/Page/Home/End keys are active when focus is on the `Drawer`'s drag handle or `Content` element and `placement == Bottom` with valid snap points.
The adapter sends `Event::SnapTo(index)` for each keyboard-initiated snap transition.

### 3.3 Snap Point Accessibility

The `Drawer`'s drag handle element receives slider semantics for snap navigation:

- `role="slider"`
- `tabindex="0"`
- `aria-label` set to `Messages::drag_handle_label`
- `aria-orientation="vertical"`
- `aria-valuemin="0"`
- `aria-valuemax="{snap_points.len() - 1}"`
- `aria-valuenow="{current_snap_index}"`
- `aria-valuetext` set to a localized description from `Messages` (e.g., "Half screen", "Full screen")

Arrow Up/Down and Home/End on the handle navigate between snap points. Home maps to `aria-valuemin` (`0`) and End maps to `aria-valuemax` (`snap_points.len() - 1`).

> **Touch-action requirement:** The `Drawer`'s drag handle and `Content` element MUST apply the `ars-touch-none` class from the companion stylesheet when `placement == Bottom` and `snap_points` is configured. This prevents the browser from intercepting vertical touch gestures as page scroll or overscroll. Additionally, set `overscroll-behavior: contain` on `Content` to prevent overscroll chaining to the body.

When `state == Dragging(_)`, the `Content` element emits `data-ars-dragging` (presence attribute). CSS consumers can use `[data-ars-dragging] { transition: none; }` to disable animation during drag.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Localized role description (default: "drawer")
    pub role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Close trigger label (default: "Close drawer")
    pub close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Drag handle slider label (default: "Drawer snap position")
    pub drag_handle_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Snap-point slider value text (default: percentage, e.g. "50%")
    pub snap_value_text: MessageFn<dyn Fn(f64, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            role_description: MessageFn::static_str("drawer"),
            close_label: MessageFn::static_str("Close drawer"),
            drag_handle_label: MessageFn::static_str("Drawer snap position"),
            snap_value_text: MessageFn::new(|value: f64, _locale: &Locale| {
                format!("{:.0}%", value * 100.0)
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

- `Placement::Start/End` resolve to correct physical side in RTL.

## 5. Variant: Bottom Sheet

When `placement == Bottom`, Drawer acts as a **bottom sheet** with discrete snap points
that the user can swipe between. Snap points are inactive for `Top`, `Left`,
`Right`, `Start`, and `End` placements even if `Props::snap_points` contains
valid values; those placements remain edge drawers.

### 5.1 Additional Props

```rust,no_check
/// Added to Drawer Props.
/// Ordered list of snap point heights. Values are fractions of viewport height
/// (0.0–1.0). Example: `vec![0.25, 0.5, 1.0]` gives quarter, half, and full.
pub snap_points: Option<Vec<f64>>,
/// Index into `snap_points` for the initial position. Defaults to 0.
pub default_snap_index: usize,
```

### 5.2 Additional Context

```rust,no_check
/// Added to Drawer Context.
/// Index of the currently active snap point.
pub current_snap: usize,
/// The resolved height (fraction) at the current snap.
pub snap_height: f64,
```

### 5.3 Additional Events

```rust,no_check
/// Keyboard-initiated snap transition.
SnapTo(usize),  // index into snap_points
```

The core emits `Effect::SnapChange` when the drawer settles at a new snap point
after drag or keyboard input. Adapters can resolve that intent by notifying any
consumer callback.

### 5.4 Behavior

#### 5.4.1 Velocity-Based Snap Targeting

On `DragEnd`, the adapter supplies normalized offset and velocity from its pointer
tracking. The agnostic core computes snap targeting from that data. Offset uses
`0.0` for fully open and `1.0` for fully dismissed. Positive velocity moves toward
dismissal or a smaller snap point; negative velocity moves toward expansion or a
larger snap point.

```rust
fn resolve_snap(
    snap_points: &[f64],
    current_snap: usize,
    current_height: f64,
    velocity: f64,  // positive = dismissing/collapsing, negative = expanding
) -> usize {
    // If velocity exceeds threshold, snap to the next point in the direction of motion.
    const VELOCITY_THRESHOLD: f64 = 0.5; // viewport-heights per second
    if velocity.abs() > VELOCITY_THRESHOLD {
        let dir = if velocity < 0.0 { 1i32 } else { -1 };
        return (current_snap as i32 + dir).clamp(0, snap_points.len() as i32 - 1) as usize;
    }
    // Otherwise, snap to the nearest point by position.
    nearest_snap_index(snap_points, current_height)
}
```

#### 5.4.2 Rubber-Band Overdrag

When the user drags beyond the largest or smallest snap point, the sheet applies
**rubber-band resistance** — the visual position moves at a decreasing rate relative
to pointer movement:

```rust,no_check
let visual_offset = max_snap + (drag_offset - max_snap) * RUBBER_BAND_FACTOR
```

where `RUBBER_BAND_FACTOR` is `0.3` (30% of additional drag distance). On release,
the sheet animates back to the nearest snap point with a spring curve.

## 6. Library Parity

> Compared against: Ark UI (`Dialog` with placement).

Radix UI and React Aria do not have a Drawer component.

### 6.1 Props

| Feature              | ars-ui               | Ark UI                   | Notes                                                  |
| -------------------- | -------------------- | ------------------------ | ------------------------------------------------------ |
| Controlled open      | `open`               | `open`                   | Same                                                   |
| Default open         | `default_open`       | `defaultOpen`            | Same                                                   |
| Modal                | `modal`              | `modal`                  | Same                                                   |
| Close on Escape      | `close_on_escape`    | `closeOnEscape`          | Same                                                   |
| Close on outside     | `close_on_backdrop`  | `closeOnInteractOutside` | Same                                                   |
| Prevent scroll       | `prevent_scroll`     | `preventScroll`          | Same                                                   |
| Restore focus        | `restore_focus`      | `restoreFocus`           | Same                                                   |
| Initial focus        | `initial_focus`      | `initialFocusEl`         | Same                                                   |
| Final focus          | `final_focus`        | `finalFocusEl`           | Same                                                   |
| Placement            | `placement`          | (CSS positioning)        | Ark UI uses Dialog with CSS; ars-ui has dedicated prop |
| Dir                  | `dir`                | --                       | ars-ui addition for logical placement resolution       |
| Snap points          | `snap_points`        | --                       | ars-ui addition for bottom sheet                       |
| Default snap index   | `default_snap_index` | --                       | ars-ui addition                                        |
| Lazy mount           | `lazy_mount`         | `lazyMount`              | Same                                                   |
| Unmount on exit      | `unmount_on_exit`    | `unmountOnExit`          | Same                                                   |
| Open change callback | `on_open_change`     | `onOpenChange`           | Same                                                   |

**Gaps:** None.

### 6.2 Anatomy

| Part         | ars-ui       | Ark UI       | Notes                                      |
| ------------ | ------------ | ------------ | ------------------------------------------ |
| Root         | Root         | Root         | Container                                  |
| Trigger      | Trigger      | Trigger      | Open button                                |
| Backdrop     | Backdrop     | Backdrop     | Background overlay                         |
| Positioner   | Positioner   | Positioner   | Sliding container                          |
| Content      | Content      | Content      | Drawer body                                |
| Title        | Title        | Title        | Heading                                    |
| Description  | Description  | Description  | Description                                |
| Header       | Header       | --           | ars-ui addition                            |
| Body         | Body         | --           | ars-ui addition                            |
| Footer       | Footer       | --           | ars-ui addition                            |
| CloseTrigger | CloseTrigger | CloseTrigger | Close button                               |
| DragHandle   | DragHandle   | --           | ars-ui addition for snap point interaction |

**Gaps:** None.

### 6.3 Events

| Callback            | ars-ui                  | Ark UI              | Notes        |
| ------------------- | ----------------------- | ------------------- | ------------ |
| Open change         | `on_open_change`        | `onOpenChange`      | Same         |
| Escape key          | (via close_on_escape)   | `onEscapeKeyDown`   | Same concept |
| Outside interaction | (via close_on_backdrop) | `onInteractOutside` | Same concept |

**Gaps:** None.

### 6.4 Features

| Feature                                | ars-ui         | Ark UI    |
| -------------------------------------- | -------------- | --------- |
| Edge placement (top/bottom/left/right) | Yes            | Yes (CSS) |
| Logical placement (start/end)          | Yes            | --        |
| Modal mode                             | Yes            | Yes       |
| Focus trap                             | Yes            | Yes       |
| Scroll lock                            | Yes            | Yes       |
| Swipe to dismiss                       | Yes            | --        |
| Snap points (bottom sheet)             | Yes            | --        |
| Velocity-based snapping                | Yes            | --        |
| Rubber-band overdrag                   | Yes            | --        |
| Keyboard snap navigation               | Yes            | --        |
| DragHandle with slider semantics       | Yes            | --        |
| Animation support                      | Yes (Presence) | Yes       |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity with Ark UI; exceeds reference with drawer-specific features.
- **Divergences:** (1) Ark UI does not have a dedicated Drawer component; it uses Dialog with CSS positioning. ars-ui provides a dedicated component with placement prop, logical direction resolution, and bottom sheet variant. (2) Snap points, swipe-to-dismiss, rubber-band overdrag, and keyboard snap navigation are ars-ui additions not found in any reference library.
- **Recommended additions:** None.
