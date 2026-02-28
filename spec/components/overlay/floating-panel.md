---
component: FloatingPanel
category: overlay
tier: complex
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [z-index-stacking]
related: []
references:
  ark-ui: FloatingPanel
---

# FloatingPanel

A `FloatingPanel` is a draggable, resizable floating window that can be minimized, maximized,
and closed. Used for tool palettes, inspector panels, chat widgets, and multi-window
interfaces within a web application.

## 1. State Machine

### 1.1 States

```rust
/// The state of the floating panel.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Panel is visible at its normal position and size.
    Idle,
    /// Panel is being dragged to a new position.
    Moving,
    /// Panel is being resized from a specific handle.
    Resizing {
        /// The handle being resized.
        handle: ResizeHandle,
    },
    /// Panel is minimized (collapsed to title bar only).
    Minimized,
    /// Panel is maximized (fills the available area).
    Maximized,
}

/// Which edge or corner the user is dragging to resize.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResizeHandle {
    /// The top edge of the panel.
    N,
    /// The bottom edge of the panel.
    S,
    /// The right edge of the panel.
    E,
    /// The left edge of the panel.
    W,
    /// The top-right corner of the panel.
    NE,
    /// The top-left corner of the panel.
    NW,
    /// The bottom-right corner of the panel.
    SE,
    /// The bottom-left corner of the panel.
    SW,
}

impl ResizeHandle {
    /// All eight resize handles.
    pub const ALL: [ResizeHandle; 8] = [
        Self::N, Self::S, Self::E, Self::W,
        Self::NE, Self::NW, Self::SE, Self::SW,
    ];

    /// CSS cursor style for this handle.
    pub fn cursor(&self) -> &'static str {
        match self {
            Self::N | Self::S   => "ns-resize",
            Self::E | Self::W   => "ew-resize",
            Self::NE | Self::SW => "nesw-resize",
            Self::NW | Self::SE => "nwse-resize",
        }
    }
}
```

### 1.2 Events

```rust
/// The events of the floating panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Drag started on the title bar / drag handle.
    DragStart,
    /// Mouse/touch moved during drag (delta x, delta y in pixels).
    DragMove(f64, f64),
    /// Drag ended.
    DragEnd,
    /// Resize started from a specific handle.
    ResizeStart(ResizeHandle),
    /// Mouse/touch moved during resize (delta x, delta y).
    ResizeMove(f64, f64),
    /// Resize ended.
    ResizeEnd,
    /// Minimize the panel.
    Minimize,
    /// Maximize the panel (fill available area).
    Maximize,
    /// Restore from minimized or maximized state.
    Restore,
    /// Close the panel.
    Close,
    /// Bring this panel to the front (highest z-index).
    BringToFront,
    /// Focus received on the panel.
    Focus {
        /// Whether the focus is from a keyboard.
        is_keyboard: bool,
    },
    /// Focus lost from the panel.
    Blur,
    /// Escape key pressed while panel is focused.
    CloseOnEscape,
    /// Internal: set z-index after allocation (sent by BringToFront effect).
    SetZIndex(u32),
}
```

### 1.3 Context

```rust
/// The context of the floating panel.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current position (x, y) in pixels from the viewport origin.
    pub position: (f64, f64),
    /// Current size (width, height) in pixels.
    pub size: (f64, f64),
    /// Minimum allowed size.
    pub min_size: (f64, f64),
    /// Maximum allowed size.
    pub max_size: (f64, f64),
    /// Current z-index (managed by `ZIndexAllocator`).
    pub z_index: u32,
    /// Whether the panel is focused.
    pub focused: bool,
    /// Whether focus was received via keyboard.
    pub focus_visible: bool,
    /// Whether the panel is currently minimized.
    pub minimized: bool,
    /// Whether the panel is currently maximized.
    pub maximized: bool,
    /// Whether the panel is open (visible).
    pub open: bool,
    /// Current locale for i18n message formatting.
    pub locale: Locale,
    /// Saved position before maximize (for restore).
    pub pre_maximize_position: Option<(f64, f64)>,
    /// Saved size before maximize (for restore).
    pub pre_maximize_size: Option<(f64, f64)>,
    /// Active resize handle during resize.
    pub active_resize_handle: Option<ResizeHandle>,
    /// Component instance IDs.
    pub ids: ComponentIds,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// The props of the floating panel.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled open state. When `Some`, the consumer controls open/close.
    pub open: Option<bool>,
    /// Whether the panel is open by default (uncontrolled). Default: true.
    pub default_open: bool,
    /// Initial position (x, y) in pixels.
    pub initial_position: (f64, f64),
    /// Initial size (width, height) in pixels.
    pub initial_size: (f64, f64),
    /// Minimum allowed size (width, height).
    pub min_size: (f64, f64),
    /// Maximum allowed size (width, height). `(f64::INFINITY, f64::INFINITY)` for no max.
    pub max_size: (f64, f64),
    /// Whether the panel can be resized.
    pub resizable: bool,
    /// Whether the panel can be dragged.
    pub draggable: bool,
    /// Whether the panel has a close button.
    pub closable: bool,
    /// Whether the panel can be minimized.
    pub minimizable: bool,
    /// Whether the panel can be maximized.
    pub maximizable: bool,
    /// Whether the panel is modal (blocks interaction with background).
    pub modal: bool,
    /// Whether to constrain the panel within the viewport bounds.
    pub constrain_to_viewport: bool,
    /// Whether Escape key closes the panel. Default: true.
    pub close_on_escape: bool,
    /// Whether the panel can be dragged/resized beyond its boundary. Default: true.
    pub allow_overflow: bool,
    /// Maintain width-to-height ratio during resize. Default: false.
    pub lock_aspect_ratio: bool,
    /// Snap-to-grid size in pixels. `1.0` = no snapping. Default: 1.0.
    pub grid_size: f64,
    /// Whether to remember size/position across open/close cycles. Default: false.
    /// When true, reopening restores the last position and size instead of `initial_*` values.
    pub persist_rect: bool,
    /// When true, panel content is not mounted until first opened. Default: false.
    pub lazy_mount: bool,
    /// When true, panel content is removed from the DOM after closing. Default: false.
    pub unmount_on_exit: bool,
    /// Callback invoked when the panel open state changes.
    pub on_open_change: Option<Callback<bool>>,
    /// Callback invoked when the panel position changes during drag.
    pub on_position_change: Option<Callback<(f64, f64)>>,
    /// Callback invoked when drag ends (final position).
    pub on_position_change_end: Option<Callback<(f64, f64)>>,
    /// Callback invoked when the panel size changes during resize.
    pub on_size_change: Option<Callback<(f64, f64)>>,
    /// Callback invoked when resize ends (final size).
    pub on_size_change_end: Option<Callback<(f64, f64)>>,
    /// Callback invoked when the panel stage changes (minimized/maximized/idle).
    pub on_stage_change: Option<Callback<Stage>>,
    /// Internationalized messages for accessible labels.
    pub messages: Option<Messages>,
    /// Locale override. When `None`, resolved via `resolve_locale()`.
    pub locale: Option<Locale>,
}

/// The stage of the floating panel (for callbacks and data attributes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    /// Normal/default state.
    Default,
    /// Panel is minimized.
    Minimized,
    /// Panel is maximized.
    Maximized,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: true,
            initial_position: (100.0, 100.0),
            initial_size: (400.0, 300.0),
            min_size: (200.0, 150.0),
            max_size: (f64::INFINITY, f64::INFINITY),
            resizable: true,
            draggable: true,
            closable: true,
            minimizable: true,
            maximizable: true,
            modal: false,
            constrain_to_viewport: true,
            close_on_escape: true,
            allow_overflow: true,
            lock_aspect_ratio: false,
            grid_size: 1.0,
            persist_rect: false,
            lazy_mount: false,
            unmount_on_exit: false,
            on_open_change: None,
            on_position_change: None,
            on_position_change_end: None,
            on_size_change: None,
            on_size_change_end: None,
            on_stage_change: None,
            messages: None,
            locale: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, ComponentIds, AttrMap};

/// The machine of the floating panel.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let ids = ComponentIds::from_id(&props.id);
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        (State::Idle, Context {
            position: props.initial_position,
            size: props.initial_size,
            min_size: props.min_size,
            max_size: props.max_size,
            z_index: 1,
            focused: false,
            focus_visible: false,
            minimized: false,
            maximized: false,
            open: true,
            locale,
            pre_maximize_position: None,
            pre_maximize_size: None,
            active_resize_handle: None,
            ids,
            messages,
        })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // ── Drag ────────────────────────────────────────────────────
            (State::Idle, Event::DragStart) if props.draggable && !ctx.maximized => {
                Some(TransitionPlan::to(State::Moving))
            }
            (State::Moving, Event::DragMove(dx, dy)) => {
                let dx = *dx;
                let dy = *dy;
                Some(TransitionPlan::context_only(move |ctx| {
                    let (x, y) = ctx.position;
                    let new_pos = Self::clamp_position(
                        (x + dx, y + dy),
                        ctx.size,
                        props.constrain_to_viewport,
                    );
                    ctx.position = Self::snap_to_grid(new_pos, props.grid_size);
                }))
            }
            (State::Moving, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Idle))
            }

            // ── Resize ──────────────────────────────────────────────────
            (State::Idle, Event::ResizeStart(handle)) if props.resizable && !ctx.maximized => {
                let handle = *handle;
                Some(TransitionPlan::to(State::Resizing { handle }).apply(move |ctx| {
                    ctx.active_resize_handle = Some(handle);
                }))
            }
            (State::Resizing { handle }, Event::ResizeMove(dx, dy)) => {
                let dx = *dx;
                let dy = *dy;
                let handle = *handle;
                Some(TransitionPlan::context_only(move |ctx| {
                    let (mut w, mut h) = ctx.size;
                    let (mut x, mut y) = ctx.position;

                    match handle {
                        ResizeHandle::E  => { w += dx; }
                        ResizeHandle::W  => { w -= dx; x += dx; }
                        ResizeHandle::S  => { h += dy; }
                        ResizeHandle::N  => { h -= dy; y += dy; }
                        ResizeHandle::SE => { w += dx; h += dy; }
                        ResizeHandle::SW => { w -= dx; x += dx; h += dy; }
                        ResizeHandle::NE => { w += dx; h -= dy; y += dy; }
                        ResizeHandle::NW => { w -= dx; x += dx; h -= dy; y += dy; }
                    }

                    w = w.clamp(ctx.min_size.0, ctx.max_size.0);
                    h = h.clamp(ctx.min_size.1, ctx.max_size.1);

                    // Enforce aspect ratio lock
                    if props.lock_aspect_ratio {
                        let initial_ratio = props.initial_size.0 / props.initial_size.1;
                        match handle {
                            ResizeHandle::E | ResizeHandle::W => { h = w / initial_ratio; }
                            ResizeHandle::N | ResizeHandle::S => { w = h * initial_ratio; }
                            _ => { h = w / initial_ratio; } // corners: width wins
                        }
                    }

                    ctx.size = (w, h);
                    ctx.position = (x, y);
                }))
            }
            (State::Resizing { .. }, Event::ResizeEnd) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.active_resize_handle = None;
                }))
            }

            // ── Minimize ────────────────────────────────────────────────
            (State::Idle, Event::Minimize) if props.minimizable => {
                Some(TransitionPlan::to(State::Minimized).apply(|ctx| {
                    ctx.minimized = true;
                }))
            }
            (State::Minimized, Event::Restore) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.minimized = false;
                }))
            }

            // ── Maximize ────────────────────────────────────────────────
            (State::Idle, Event::Maximize) if props.maximizable => {
                Some(TransitionPlan::to(State::Maximized).apply(|ctx| {
                    ctx.pre_maximize_position = Some(ctx.position);
                    ctx.pre_maximize_size = Some(ctx.size);
                    ctx.maximized = true;
                }))
            }
            (State::Maximized, Event::Restore) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    if let (Some(pos), Some(sz)) = (ctx.pre_maximize_position, ctx.pre_maximize_size) {
                        ctx.position = pos;
                        ctx.size = sz;
                    }
                    ctx.maximized = false;
                    ctx.pre_maximize_position = None;
                    ctx.pre_maximize_size = None;
                }))
            }
            // Double-click title bar toggles maximize.
            (State::Maximized, Event::Maximize) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    if let (Some(pos), Some(sz)) = (ctx.pre_maximize_position, ctx.pre_maximize_size) {
                        ctx.position = pos;
                        ctx.size = sz;
                    }
                    ctx.maximized = false;
                    ctx.pre_maximize_position = None;
                    ctx.pre_maximize_size = None;
                }))
            }

            // ── Close ───────────────────────────────────────────────────
            (_, Event::Close) if props.closable => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.open = false;
                }))
            }

            // ── Bring to front ──────────────────────────────────────────
            (_, Event::BringToFront) => {
                Some(TransitionPlan::context_only(|_ctx| {})
                    .with_named_effect("z-index", |_ctx, _props, send| {
                        let new_z = resolve_z_allocator().allocate();
                        send.upgrade_and_send(Event::SetZIndex(new_z));
                        no_cleanup()
                    }))
            }

            (_, Event::SetZIndex(z)) => {
                let z = *z;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.z_index = z;
                }))
            }

            // ── Escape key close ────────────────────────────────────────
            (State::Idle | State::Minimized | State::Maximized, Event::CloseOnEscape)
                if props.close_on_escape => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.open = false;
                }))
            }

            // ── Focus ───────────────────────────────────────────────────
            (_, Event::Focus { is_keyboard }) => {
                let ik = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = ik;
                }))
            }
            (_, Event::Blur) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            _ => None,
        }
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

impl Machine {
    /// Clamp position to keep the panel within viewport bounds.
    fn clamp_position(
        pos: (f64, f64),
        size: (f64, f64),
        constrain: bool,
    ) -> (f64, f64) {
        if !constrain {
            return pos;
        }
        // Ensure at least the title bar (top 32px) remains visible.
        (pos.0.max(-size.0 + 40.0), pos.1.max(0.0))
    }

    fn snap_to_grid(pos: (f64, f64), grid_size: f64) -> (f64, f64) {
        if grid_size <= 1.0 { return pos; }
        (
            (pos.0 / grid_size).round() * grid_size,
            (pos.1 / grid_size).round() * grid_size,
        )
    }
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "floating-panel"]
pub enum Part {
    Root,
    Header,
    DragHandle,
    Title,
    Content,
    Footer,
    ResizeHandle { handle: ResizeHandle },
    CloseTrigger,
    MinimizeTrigger,
    MaximizeTrigger,
    StageTrigger,
}

/// The API of the floating panel.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Role, "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("title"));
        if self.props.modal {
            attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "true");
        }
        let state_str = match self.state {
            State::Idle       => "idle",
            State::Moving     => "moving",
            State::Resizing { .. } => "resizing",
            State::Minimized  => "minimized",
            State::Maximized  => "maximized",
        };
        attrs.set(HtmlAttr::Data("ars-state"), state_str);
        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        if self.ctx.minimized {
            attrs.set_bool(HtmlAttr::Data("ars-minimized"), true);
        }
        if self.ctx.maximized {
            attrs.set_bool(HtmlAttr::Data("ars-maximized"), true);
        }
        if matches!(self.state, State::Moving) {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }
        if matches!(self.state, State::Resizing { .. }) {
            attrs.set_bool(HtmlAttr::Data("ars-resizing"), true);
        }
        let stage = match self.state {
            State::Minimized => "minimized",
            State::Maximized => "maximized",
            _ => "default",
        };
        attrs.set(HtmlAttr::Data("ars-stage"), stage);
        attrs.set_style(CssProperty::Position, "fixed");
        attrs.set_style(CssProperty::Left, format!("{}px", self.ctx.position.0));
        attrs.set_style(CssProperty::Top, format!("{}px", self.ctx.position.1));
        attrs.set_style(CssProperty::Width, format!("{}px", self.ctx.size.0));
        attrs.set_style(CssProperty::Height, format!("{}px", self.ctx.size.1));
        attrs.set_style(CssProperty::ZIndex, self.ctx.z_index.to_string());
        attrs
    }

    pub fn header_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Header.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn drag_handle_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DragHandle.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.move_label)(&self.ctx.locale));
        if self.props.draggable && !self.ctx.maximized {
            attrs.set_style(CssProperty::Cursor, "grab");
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

    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.ctx.minimized {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    pub fn footer_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Footer.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.ctx.minimized {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    pub fn resize_handle_attrs(&self, handle: ResizeHandle) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ResizeHandle { handle }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let handle_str = match handle {
            ResizeHandle::N  => "n",  ResizeHandle::S  => "s",
            ResizeHandle::E  => "e",  ResizeHandle::W  => "w",
            ResizeHandle::NE => "ne", ResizeHandle::NW => "nw",
            ResizeHandle::SE => "se", ResizeHandle::SW => "sw",
        };
        attrs.set(HtmlAttr::Data("ars-handle"), handle_str);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.resize_handle_label)(handle, &self.ctx.locale));
        if self.props.resizable && !self.ctx.maximized {
            attrs.set_style(CssProperty::Cursor, handle.cursor());
        }
        attrs
    }

    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.close_label)(&self.ctx.locale));
        attrs
    }

    pub fn minimize_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::MinimizeTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        if self.ctx.minimized {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.restore_label)(&self.ctx.locale));
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.minimize_label)(&self.ctx.locale));
        }
        attrs
    }

    pub fn maximize_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::MaximizeTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        if self.ctx.maximized {
            attrs.set_bool(HtmlAttr::Data("ars-maximized"), true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.restore_label)(&self.ctx.locale));
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.maximize_label)(&self.ctx.locale));
        }
        attrs
    }

    pub fn stage_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StageTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        let stage = if self.ctx.minimized {
            "minimized"
        } else if self.ctx.maximized {
            "maximized"
        } else {
            "default"
        };
        attrs.set(HtmlAttr::Data("ars-state"), stage);
        attrs.set(
            HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.stage_trigger_label)(stage, &self.ctx.locale),
        );
        attrs
    }

    pub fn on_stage_trigger_click(&self) {
        if self.ctx.minimized || self.ctx.maximized {
            (self.send)(Event::Restore);
        } else {
            (self.send)(Event::Minimize);
        }
    }

    // ── Convenience getters ─────────────────────────────────────────────

    pub fn is_open(&self) -> bool { self.ctx.open }
    pub fn is_minimized(&self) -> bool { self.ctx.minimized }
    pub fn is_maximized(&self) -> bool { self.ctx.maximized }
    pub fn is_moving(&self) -> bool { matches!(self.state, State::Moving) }
    pub fn is_resizing(&self) -> bool { matches!(self.state, State::Resizing { .. }) }
    pub fn position(&self) -> (f64, f64) { self.ctx.position }
    pub fn size(&self) -> (f64, f64) { self.ctx.size }

    pub fn on_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Escape => (self.send)(Event::Close),
            KeyboardKey::ArrowUp    if self.ctx.focused => (self.send)(Event::DragStart),
            KeyboardKey::ArrowDown  if self.ctx.focused => (self.send)(Event::DragStart),
            KeyboardKey::ArrowLeft  if self.ctx.focused => (self.send)(Event::DragStart),
            KeyboardKey::ArrowRight if self.ctx.focused => (self.send)(Event::DragStart),
            _ => {}
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Header => self.header_attrs(),
            Part::DragHandle => self.drag_handle_attrs(),
            Part::Title => self.title_attrs(),
            Part::Content => self.content_attrs(),
            Part::Footer => self.footer_attrs(),
            Part::ResizeHandle { handle } => self.resize_handle_attrs(handle),
            Part::CloseTrigger => self.close_trigger_attrs(),
            Part::MinimizeTrigger => self.minimize_trigger_attrs(),
            Part::MaximizeTrigger => self.maximize_trigger_attrs(),
            Part::StageTrigger => self.stage_trigger_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
FloatingPanel
├── Root                  role="dialog", position:fixed
│   ├── Header
│   │   ├── DragHandle    cursor:grab
│   │   ├── Title
│   │   ├── StageTrigger  cycles: Normal → Minimized → Normal
│   │   ├── MinimizeTrigger
│   │   ├── MaximizeTrigger
│   │   └── CloseTrigger
│   ├── Content           hidden when minimized
│   ├── Footer            hidden when minimized
│   └── ResizeHandle (×8) N, S, E, W, NE, NW, SE, SW
```

| Part            | Element    | Key Attributes                                                           |
| --------------- | ---------- | ------------------------------------------------------------------------ |
| Root            | `<div>`    | `role="dialog"`, `aria-labelledby`, inline position/size styles          |
| Header          | `<div>`    | `data-ars-scope="floating-panel"`                                        |
| DragHandle      | `<div>`    | `aria-label`, `cursor:grab` (when draggable)                             |
| Title           | `<h2>`     | `id` for `aria-labelledby`                                               |
| Content         | `<div>`    | `hidden` when minimized                                                  |
| Footer          | `<div>`    | `hidden` when minimized                                                  |
| ResizeHandle    | `<div>`    | `data-ars-handle`, `aria-label`, directional cursor                      |
| CloseTrigger    | `<button>` | `type="button"`, `aria-label`                                            |
| MinimizeTrigger | `<button>` | `type="button"`, `aria-label` (Minimize / Restore)                       |
| MaximizeTrigger | `<button>` | `type="button"`, `aria-label` (Maximize / Restore), `data-ars-maximized` |
| StageTrigger    | `<button>` | `type="button"`, `aria-label` (Minimize / Restore), `data-ars-state`     |

**11 part types** (`ResizeHandle` appears up to 8 times, one per edge/corner).

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part         | Property          | Value                                                        |
| ------------ | ----------------- | ------------------------------------------------------------ |
| Root         | `role`            | `"dialog"`                                                   |
| Root         | `aria-labelledby` | Title ID                                                     |
| Root         | `aria-modal`      | `"true"` (when modal)                                        |
| StageTrigger | `role`            | `"button"` (implicit via `<button>`)                         |
| StageTrigger | `aria-label`      | From Messages: "Minimize" / "Restore" based on current stage |

### 3.2 Keyboard Interaction

| Key         | Action                                                 |
| ----------- | ------------------------------------------------------ |
| Escape      | Close the panel                                        |
| Tab         | Cycle through interactive elements within the panel    |
| Arrows      | Nudge position (when panel root is focused)            |
| Enter/Space | Activate StageTrigger (standard `<button>` activation) |

### 3.3 Focus Management

- When modal, the adapter uses FocusScope to trap focus and sets `inert` on background content.
- Window control buttons (close, minimize, maximize, stage trigger) each have an accessible label from Messages. The maximize button label changes to "Restore" when maximized. The stage trigger label reflects the next action: "Minimize" when in default stage, "Restore" when minimized or maximized.
- Resize handles have `aria-label` describing their direction (e.g., "Resize bottom-right"). Handles have at least 44×44px touch target per WCAG 2.5.5.
- Drag handle area uses `cursor:grab`/`cursor:grabbing` as visual feedback.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the close button.
    pub close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the minimize button.
    pub minimize_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the maximize button.
    pub maximize_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the restore button (when maximized/minimized).
    pub restore_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for a resize handle.
    pub resize_handle_label: MessageFn<dyn Fn(ResizeHandle, &Locale) -> String + Send + Sync>,
    /// Accessible label for the drag handle / move action.
    pub move_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the stage trigger button.
    /// Receives the current stage (`"default"`, `"minimized"`, `"maximized"`) and the locale.
    /// Returns the next-action label: "Minimize" when default, "Restore" when minimized/maximized.
    pub stage_trigger_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            close_label: MessageFn::static_str("Close panel"),
            minimize_label: MessageFn::static_str("Minimize panel"),
            maximize_label: MessageFn::static_str("Maximize panel"),
            restore_label: MessageFn::static_str("Restore panel"),
            resize_handle_label: MessageFn::new(|handle, _locale| {
                let dir = match handle {
                    ResizeHandle::N  => "top",
                    ResizeHandle::S  => "bottom",
                    ResizeHandle::E  => "right",
                    ResizeHandle::W  => "left",
                    ResizeHandle::NE => "top-right",
                    ResizeHandle::NW => "top-left",
                    ResizeHandle::SE => "bottom-right",
                    ResizeHandle::SW => "bottom-left",
                };
                format!("Resize {dir}")
            }),
            move_label: MessageFn::static_str("Move panel"),
            stage_trigger_label: MessageFn::new(|stage, _locale| {
                match stage {
                    "minimized" | "maximized" => "Restore".to_string(),
                    _ => "Minimize".to_string(),
                }
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

- All button labels and descriptions come from `Messages`, following the `MessageFn<dyn Fn>` i18n pattern. The `stage_trigger_label` receives both locale and the current stage string so it can produce context-appropriate labels.
- Position and size values are in CSS pixels (direction-neutral).
- In RTL layouts, the panel's initial position is mirrored by the adapter.

## 5. Z-Index Management

`FloatingPanel` integrates with [`ZIndexAllocator`](../utility/z-index-allocator.md):

```rust
// On focus or BringToFront:
let new_z = z_allocator.allocate();
ctx.z_index = new_z;
```

Each panel gets a unique z-index, ensuring the most recently focused panel is on top.
The allocator's overflow detection prevents z-index exhaustion.

## 6. Move Integration

`FloatingPanel` composes with `use_move` from `05-interactions.md` for drag functionality:

```rust
// Adapter wiring (conceptual):
use_move(MoveOptions {
    on_move_start: |_| send(Event::DragStart),
    on_move: |delta| send(Event::DragMove(delta.x, delta.y)),
    on_move_end: |_| send(Event::DragEnd),
});
```

This provides unified pointer and keyboard-based movement with proper event cleanup.

## 7. Dependencies

All overlay components depend on:

- `ars-core`: Machine, TransitionPlan, PendingEffect, AttrMap
- `ars-dom`: positioning engine, focus utilities, portal, scroll management, inert attribute management
- `ars-a11y`: FocusScope, focus trap for Dialog/AlertDialog

## 8. Library Parity

> Compared against: Ark UI (`FloatingPanel`).

Radix UI and React Aria do not have a FloatingPanel component.

### 8.1 Props

| Feature                   | ars-ui                   | Ark UI                | Notes                                     |
| ------------------------- | ------------------------ | --------------------- | ----------------------------------------- |
| Controlled open           | `open`                   | `open`                | Same                                      |
| Default open              | `default_open`           | `defaultOpen`         | Same                                      |
| Default position          | `initial_position`       | `defaultPosition`     | Same concept                              |
| Default size              | `initial_size`           | `defaultSize`         | Same concept                              |
| Min size                  | `min_size`               | `minSize`             | Same                                      |
| Max size                  | `max_size`               | `maxSize`             | Same                                      |
| Resizable                 | `resizable`              | `resizable`           | Same                                      |
| Draggable                 | `draggable`              | `draggable`           | Same                                      |
| Closable                  | `closable`               | --                    | ars-ui addition                           |
| Minimizable               | `minimizable`            | --                    | ars-ui addition                           |
| Maximizable               | `maximizable`            | --                    | ars-ui addition                           |
| Modal                     | `modal`                  | --                    | ars-ui addition                           |
| Constrain to viewport     | `constrain_to_viewport`  | --                    | ars-ui addition                           |
| Close on Escape           | `close_on_escape`        | `closeOnEscape`       | Same                                      |
| Allow overflow            | `allow_overflow`         | `allowOverflow`       | Same                                      |
| Lock aspect ratio         | `lock_aspect_ratio`      | `lockAspectRatio`     | Same                                      |
| Grid size                 | `grid_size`              | `gridSize`            | Same                                      |
| Persist rect              | `persist_rect`           | `persistRect`         | Same                                      |
| Lazy mount                | `lazy_mount`             | `lazyMount`           | Same                                      |
| Unmount on exit           | `unmount_on_exit`        | --                    | ars-ui addition                           |
| Controlled position       | --                       | `position`            | Ark UI controlled position                |
| Controlled size           | --                       | `size`                | Ark UI controlled size                    |
| Boundary element          | --                       | `getBoundaryEl`       | Ark UI only                               |
| Strategy (fixed/absolute) | (always fixed)           | `strategy`            | Ark UI allows absolute; ars-ui uses fixed |
| Dir                       | --                       | `dir`                 | Ark UI only                               |
| Open change               | `on_open_change`         | `onOpenChange`        | Same                                      |
| Position change           | `on_position_change`     | `onPositionChange`    | Same                                      |
| Position change end       | `on_position_change_end` | `onPositionChangeEnd` | Same                                      |
| Size change               | `on_size_change`         | `onSizeChange`        | Same                                      |
| Stage change              | `on_stage_change`        | `onStageChange`       | Same                                      |

**Gaps:** None. Ark UI's `position`/`size` controlled props and `getBoundaryEl` are advanced features; ars-ui uses `constrain_to_viewport` for boundary control.

### 8.2 Anatomy

| Part            | ars-ui            | Ark UI        | Notes                            |
| --------------- | ----------------- | ------------- | -------------------------------- |
| Root            | Root              | Root          | Container, role="dialog"         |
| Header          | Header            | Header        | Title bar area                   |
| DragHandle      | DragHandle        | DragTrigger   | Drag initiator                   |
| Title           | Title             | Title         | Panel heading                    |
| Content         | Content           | Body          | Main content                     |
| Footer          | Footer            | --            | ars-ui addition                  |
| ResizeHandle    | ResizeHandle (x8) | ResizeTrigger | Resize handles                   |
| CloseTrigger    | CloseTrigger      | CloseTrigger  | Close button                     |
| MinimizeTrigger | MinimizeTrigger   | --            | ars-ui addition                  |
| MaximizeTrigger | MaximizeTrigger   | --            | ars-ui addition                  |
| StageTrigger    | StageTrigger      | StageTrigger  | Minimize/restore toggle          |
| Positioner      | --                | Positioner    | Ark UI wrapper                   |
| Control         | --                | Control       | Ark UI window controls container |

**Gaps:** None. Ark UI's `Positioner` and `Control` are structural wrappers; ars-ui uses inline styles on Root for positioning and the Header for window controls.

### 8.3 Events

| Callback            | ars-ui                   | Ark UI                | Notes               |
| ------------------- | ------------------------ | --------------------- | ------------------- |
| Open change         | `on_open_change`         | `onOpenChange`        | Same                |
| Position change     | `on_position_change`     | `onPositionChange`    | Same                |
| Position change end | `on_position_change_end` | `onPositionChangeEnd` | Same                |
| Size change         | `on_size_change`         | `onSizeChange`        | Same                |
| Stage change        | `on_stage_change`        | `onStageChange`       | Same                |
| Exit complete       | (Presence)               | `onExitComplete`      | Handled by Presence |

**Gaps:** None.

### 8.4 Features

| Feature                        | ars-ui          | Ark UI |
| ------------------------------ | --------------- | ------ |
| Drag to move                   | Yes             | Yes    |
| Resize from edges/corners      | Yes (8 handles) | Yes    |
| Minimize                       | Yes             | Yes    |
| Maximize                       | Yes             | Yes    |
| Restore                        | Yes             | Yes    |
| Close                          | Yes             | Yes    |
| Bring to front (z-index)       | Yes             | Yes    |
| Grid snapping                  | Yes             | Yes    |
| Lock aspect ratio              | Yes             | Yes    |
| Allow overflow                 | Yes             | Yes    |
| Persist rect across open/close | Yes             | Yes    |
| Modal mode                     | Yes             | --     |
| Viewport constraint            | Yes             | --     |
| Keyboard drag (arrows)         | Yes             | --     |
| Focus visible tracking         | Yes             | --     |
| Animation support              | Yes (Presence)  | Yes    |

**Gaps:** None.

### 8.5 Summary

- **Overall:** Full parity with Ark UI; exceeds reference with additional features.
- **Divergences:** (1) ars-ui adds modal mode with focus trapping and inert background. (2) ars-ui adds dedicated minimize/maximize trigger parts (Ark UI uses a single StageTrigger that cycles through states). (3) ars-ui adds keyboard-based panel movement using arrow keys. (4) ars-ui uses `ZIndexAllocator` from the shared z-index stacking spec for bring-to-front behavior.
- **Recommended additions:** None.
- `ars-interactions`: click-outside detection for Popover, `use_move` for FloatingPanel drag
