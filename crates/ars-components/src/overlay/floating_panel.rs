//! Draggable and resizable floating panel machine.

use alloc::{
    format,
    string::{String, ToString as _},
    sync::Arc,
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    CssProperty, Env, HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

type ResizeHandleLabelFn = dyn Fn(ResizeHandle, &Locale) -> String + Send + Sync;
type StageLabelFn = dyn Fn(Stage, &Locale) -> String + Send + Sync;
type RectChangeFn = dyn Fn((f64, f64)) + Send + Sync;

/// The state of the floating panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// Panel is visible at its normal position and size.
    #[default]
    Idle,

    /// Panel is being dragged to a new position.
    Moving,

    /// Panel is being resized from a specific handle.
    Resizing {
        /// The handle being resized.
        handle: ResizeHandle,
    },

    /// Panel is minimized.
    Minimized,

    /// Panel is maximized.
    Maximized,
}

/// Which edge or corner the user is dragging to resize.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ResizeHandle {
    /// The top edge.
    #[default]
    N,

    /// The bottom edge.
    S,

    /// The right edge.
    E,

    /// The left edge.
    W,

    /// The top-right corner.
    NE,

    /// The top-left corner.
    NW,

    /// The bottom-right corner.
    SE,

    /// The bottom-left corner.
    SW,
}

impl ResizeHandle {
    /// All eight resize handles.
    pub const ALL: [Self; 8] = [
        Self::N,
        Self::S,
        Self::E,
        Self::W,
        Self::NE,
        Self::NW,
        Self::SE,
        Self::SW,
    ];

    /// Returns the CSS cursor token for this resize handle.
    #[must_use]
    pub const fn cursor(self) -> &'static str {
        match self {
            Self::N | Self::S => "ns-resize",
            Self::E | Self::W => "ew-resize",
            Self::NE | Self::SW => "nesw-resize",
            Self::NW | Self::SE => "nwse-resize",
        }
    }

    /// Returns the stable attribute token for this handle.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::N => "n",
            Self::S => "s",
            Self::E => "e",
            Self::W => "w",
            Self::NE => "ne",
            Self::NW => "nw",
            Self::SE => "se",
            Self::SW => "sw",
        }
    }
}

/// The stage of the floating panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    /// Normal/default stage.
    #[default]
    Default,

    /// Minimized stage.
    Minimized,

    /// Maximized stage.
    Maximized,
}

impl Stage {
    /// Returns the stable attribute token for this stage.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Minimized => "minimized",
            Self::Maximized => "maximized",
        }
    }
}

/// Adapter-supplied viewport rectangle in CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportRect {
    /// Left coordinate.
    pub x: f64,

    /// Top coordinate.
    pub y: f64,

    /// Width in CSS pixels.
    pub width: f64,

    /// Height in CSS pixels.
    pub height: f64,
}

/// Adapter-supplied panel rectangle in CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelRect {
    /// Left coordinate.
    pub x: f64,

    /// Top coordinate.
    pub y: f64,

    /// Width in CSS pixels.
    pub width: f64,

    /// Height in CSS pixels.
    pub height: f64,
}

/// Adapter-supplied metrics for a maximize request.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaximizeMetrics {
    /// Available viewport area to fill.
    pub viewport: ViewportRect,
}

/// Events accepted by the `FloatingPanel` state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Drag started on the drag handle.
    DragStart,

    /// Drag moved by adapter-normalized deltas.
    DragMove {
        /// Horizontal delta.
        dx: f64,

        /// Vertical delta.
        dy: f64,
    },

    /// Keyboard nudge requested a position move by normalized deltas.
    KeyboardMove {
        /// Horizontal delta.
        dx: f64,

        /// Vertical delta.
        dy: f64,
    },

    /// Drag ended.
    DragEnd,

    /// Resize started from a handle.
    ResizeStart(ResizeHandle),

    /// Resize moved by adapter-normalized deltas.
    ResizeMove {
        /// Horizontal delta.
        dx: f64,

        /// Vertical delta.
        dy: f64,
    },

    /// Resize ended.
    ResizeEnd,

    /// Minimize the panel.
    Minimize,

    /// Maximize using adapter-supplied viewport metrics.
    Maximize(MaximizeMetrics),

    /// Restore from minimized or maximized stage.
    Restore,

    /// Close the panel.
    Close,

    /// Bring this panel to the front.
    BringToFront,

    /// Focus entered the panel.
    Focus {
        /// Whether focus came from keyboard modality.
        is_keyboard: bool,
    },

    /// Focus left the panel.
    Blur,

    /// Escape requested close.
    CloseOnEscape,

    /// Adapter supplied an allocated z-index.
    SetZIndex(u32),

    /// Controlled open state changed.
    SetControlledOpen(bool),

    /// Props changed without a controlled open change.
    SyncProps,
}

/// Typed identifier for every named effect intent emitted by `FloatingPanel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter invokes [`Props::on_open_change`].
    OpenChange,

    /// Adapter invokes [`Props::on_position_change`].
    PositionChange,

    /// Adapter invokes [`Props::on_position_change_end`].
    PositionChangeEnd,

    /// Adapter invokes [`Props::on_size_change`].
    SizeChange,

    /// Adapter invokes [`Props::on_size_change_end`].
    SizeChangeEnd,

    /// Adapter invokes [`Props::on_stage_change`].
    StageChange,

    /// Adapter allocates a z-index and dispatches [`Event::SetZIndex`].
    AllocateZIndex,
}

/// Localizable strings for `FloatingPanel`.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the drag handle.
    pub move_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the close trigger.
    pub close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the minimize trigger.
    pub minimize_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the maximize trigger.
    pub maximize_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for restore triggers.
    pub restore_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for resize handles.
    pub resize_handle_label: MessageFn<ResizeHandleLabelFn>,

    /// Accessible label for the combined stage trigger.
    pub stage_trigger_label: MessageFn<StageLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        let resize: Arc<ResizeHandleLabelFn> = Arc::new(|handle, _locale| {
            let direction = match handle {
                ResizeHandle::N => "top",
                ResizeHandle::S => "bottom",
                ResizeHandle::E => "right",
                ResizeHandle::W => "left",
                ResizeHandle::NE => "top-right",
                ResizeHandle::NW => "top-left",
                ResizeHandle::SE => "bottom-right",
                ResizeHandle::SW => "bottom-left",
            };

            format!("Resize {direction}")
        });

        let stage: Arc<StageLabelFn> = Arc::new(|stage, _locale| match stage {
            Stage::Default => String::from("Minimize"),
            Stage::Minimized | Stage::Maximized => String::from("Restore"),
        });

        Self {
            move_label: MessageFn::static_str("Move panel"),
            close_label: MessageFn::static_str("Close panel"),
            minimize_label: MessageFn::static_str("Minimize panel"),
            maximize_label: MessageFn::static_str("Maximize panel"),
            restore_label: MessageFn::static_str("Restore panel"),
            resize_handle_label: MessageFn::new(resize),
            stage_trigger_label: MessageFn::new(stage),
        }
    }
}

impl ComponentMessages for Messages {}

/// Runtime context for `FloatingPanel`.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current position `(x, y)` in CSS pixels.
    pub position: (f64, f64),

    /// Current size `(width, height)` in CSS pixels.
    pub size: (f64, f64),

    /// Minimum allowed size.
    pub min_size: (f64, f64),

    /// Maximum allowed size.
    pub max_size: (f64, f64),

    /// Adapter-allocated z-index.
    pub z_index: u32,

    /// Whether the panel currently contains focus.
    pub focused: bool,

    /// Whether focus should render visibly.
    pub focus_visible: bool,

    /// Whether the panel is minimized.
    pub minimized: bool,

    /// Whether the panel is maximized.
    pub maximized: bool,

    /// Whether the panel is open.
    pub open: bool,

    /// Current locale for message resolution.
    pub locale: Locale,

    /// Saved pre-maximize position for restore.
    pub pre_maximize_position: Option<(f64, f64)>,

    /// Saved pre-maximize size for restore.
    pub pre_maximize_size: Option<(f64, f64)>,

    /// Active resize handle.
    pub active_resize_handle: Option<ResizeHandle>,

    /// Component instance IDs.
    pub ids: ComponentIds,

    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

impl Context {
    /// Returns the current panel stage.
    #[must_use]
    pub const fn stage(&self) -> Stage {
        if self.minimized {
            Stage::Minimized
        } else if self.maximized {
            Stage::Maximized
        } else {
            Stage::Default
        }
    }
}

/// Immutable configuration for a `FloatingPanel` instance.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled open state.
    pub open: Option<bool>,

    /// Initial uncontrolled open state.
    pub default_open: bool,

    /// Initial position.
    pub initial_position: (f64, f64),

    /// Initial size.
    pub initial_size: (f64, f64),

    /// Minimum allowed size.
    pub min_size: (f64, f64),

    /// Maximum allowed size.
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

    /// Whether the panel is modal.
    pub modal: bool,

    /// Whether position is constrained to viewport-style bounds.
    pub constrain_to_viewport: bool,

    /// Whether Escape closes the panel.
    pub close_on_escape: bool,

    /// Whether dragging/resizing may overflow constraints.
    pub allow_overflow: bool,

    /// Whether resize maintains the initial aspect ratio.
    pub lock_aspect_ratio: bool,

    /// Snap-to-grid size in CSS pixels.
    pub grid_size: f64,

    /// Whether adapters should persist rect across open cycles.
    pub persist_rect: bool,

    /// Whether content is not mounted until first opened.
    pub lazy_mount: bool,

    /// Whether content is removed after closing.
    pub unmount_on_exit: bool,

    /// Callback invoked after open state changes.
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,

    /// Callback invoked when position changes during drag.
    pub on_position_change: Option<Callback<RectChangeFn>>,

    /// Callback invoked when drag ends.
    pub on_position_change_end: Option<Callback<RectChangeFn>>,

    /// Callback invoked when size changes during resize.
    pub on_size_change: Option<Callback<RectChangeFn>>,

    /// Callback invoked when resize ends.
    pub on_size_change_end: Option<Callback<RectChangeFn>>,

    /// Callback invoked when panel stage changes.
    pub on_stage_change: Option<Callback<dyn Fn(Stage) + Send + Sync>>,
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
        }
    }
}

impl Props {
    /// Returns `FloatingPanel` props with documented defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Anatomy parts exposed by the `FloatingPanel` connect API.
#[derive(ComponentPart)]
#[scope = "floating-panel"]
pub enum Part {
    /// The root container.
    Root,

    /// The header region.
    Header,

    /// The drag handle.
    DragHandle,

    /// The title element.
    Title,

    /// The content region.
    Content,

    /// The footer region.
    Footer,

    /// A resize handle for an edge or corner.
    ResizeHandle {
        /// The resize handle direction.
        handle: ResizeHandle,
    },

    /// The close trigger.
    CloseTrigger,

    /// The minimize trigger.
    MinimizeTrigger,

    /// The maximize trigger.
    MaximizeTrigger,

    /// The combined stage trigger.
    StageTrigger,
}

fn snap(value: f64, grid: f64) -> f64 {
    if grid <= 1.0 {
        value
    } else {
        (value / grid).round() * grid
    }
}

fn snap_position(position: (f64, f64), grid: f64) -> (f64, f64) {
    (snap(position.0, grid), snap(position.1, grid))
}

fn clamp_position(position: (f64, f64), size: (f64, f64), props: &Props) -> (f64, f64) {
    if !props.constrain_to_viewport || props.allow_overflow {
        position
    } else {
        (position.0.max(-size.0 + 40.0), position.1.max(0.0))
    }
}

fn resize_rect(
    position: (f64, f64),
    size: (f64, f64),
    handle: ResizeHandle,
    dx: f64,
    dy: f64,
    props: &Props,
) -> ((f64, f64), (f64, f64)) {
    let (x, y) = position;
    let (w, h) = size;

    let right = x + w;
    let bottom = y + h;

    let mut new_x = x;
    let mut new_y = y;
    let mut new_w = w;
    let mut new_h = h;

    match handle {
        ResizeHandle::E => new_w = w + dx,

        ResizeHandle::W => {
            new_x = x + dx;
            new_w = w - dx;
        }

        ResizeHandle::S => new_h = h + dy,

        ResizeHandle::N => {
            new_y = y + dy;
            new_h = h - dy;
        }

        ResizeHandle::SE => {
            new_w = w + dx;
            new_h = h + dy;
        }

        ResizeHandle::SW => {
            new_x = x + dx;
            new_w = w - dx;
            new_h = h + dy;
        }

        ResizeHandle::NE => {
            new_w = w + dx;
            new_y = y + dy;
            new_h = h - dy;
        }

        ResizeHandle::NW => {
            new_x = x + dx;
            new_w = w - dx;
            new_y = y + dy;
            new_h = h - dy;
        }
    }

    new_w = new_w.clamp(props.min_size.0, props.max_size.0);
    new_h = new_h.clamp(props.min_size.1, props.max_size.1);

    if props.lock_aspect_ratio {
        let ratio = props.initial_size.0 / props.initial_size.1;

        if matches!(handle, ResizeHandle::N | ResizeHandle::S) {
            new_w = new_h * ratio;
        } else {
            new_h = new_w / ratio;
        }

        new_w = new_w.clamp(props.min_size.0, props.max_size.0);
        new_h = new_h.clamp(props.min_size.1, props.max_size.1);
    }

    if matches!(
        handle,
        ResizeHandle::W | ResizeHandle::NW | ResizeHandle::SW
    ) {
        new_x = right - new_w;
    }

    if matches!(
        handle,
        ResizeHandle::N | ResizeHandle::NE | ResizeHandle::NW
    ) {
        new_y = bottom - new_h;
    }

    ((new_x, new_y), (new_w, new_h))
}

fn stage_plan(target: State, f: impl FnOnce(&mut Context) + 'static) -> TransitionPlan<Machine> {
    TransitionPlan::to(target)
        .apply(f)
        .with_effect(PendingEffect::named(Effect::StageChange))
}

fn close_plan(stage_changed: bool) -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Idle)
        .apply(|ctx: &mut Context| {
            ctx.open = false;
            ctx.focused = false;
            ctx.focus_visible = false;
            ctx.minimized = false;
            ctx.maximized = false;
            ctx.active_resize_handle = None;
            ctx.pre_maximize_position = None;
            ctx.pre_maximize_size = None;
        })
        .with_effect(PendingEffect::named(Effect::OpenChange));

    if stage_changed {
        plan = plan.with_effect(PendingEffect::named(Effect::StageChange));
    }

    plan
}

fn open_controlled_plan(props: &Props) -> TransitionPlan<Machine> {
    let initial_position = props.initial_position;
    let initial_size = props.initial_size;
    let persist_rect = props.persist_rect;

    TransitionPlan::to(State::Idle)
        .apply(move |ctx: &mut Context| {
            ctx.open = true;
            ctx.focused = false;
            ctx.focus_visible = false;
            ctx.minimized = false;
            ctx.maximized = false;
            ctx.active_resize_handle = None;
            ctx.pre_maximize_position = None;
            ctx.pre_maximize_size = None;

            if !persist_rect {
                ctx.position = initial_position;
                ctx.size = initial_size;
            }
        })
        .with_effect(PendingEffect::named(Effect::OpenChange))
        .with_effect(PendingEffect::named(Effect::AllocateZIndex))
}

fn props_changed(old: &Props, new: &Props) -> bool {
    old.min_size != new.min_size
        || old.max_size != new.max_size
        || old.draggable != new.draggable
        || old.resizable != new.resizable
        || old.modal != new.modal
        || old.close_on_escape != new.close_on_escape
        || old.grid_size != new.grid_size
}

/// State machine for `FloatingPanel`.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let ids = ComponentIds::from_id(&props.id);

        let open = props.open.unwrap_or(props.default_open);

        (
            State::Idle,
            Context {
                position: props.initial_position,
                size: props.initial_size,
                min_size: props.min_size,
                max_size: props.max_size,
                z_index: 1,
                focused: false,
                focus_visible: false,
                minimized: false,
                maximized: false,
                open,
                locale: env.locale.clone(),
                pre_maximize_position: None,
                pre_maximize_size: None,
                active_resize_handle: None,
                ids,
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (_, Event::SetControlledOpen(open)) if *open == ctx.open => None,

            (_, Event::SetControlledOpen(true)) => Some(open_controlled_plan(props)),

            (_, Event::SetControlledOpen(false)) => Some(close_plan(ctx.stage() != Stage::Default)),

            (_, Event::SyncProps) => {
                let min_size = props.min_size;
                let max_size = props.max_size;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.min_size = min_size;
                    ctx.max_size = max_size;
                    ctx.size.0 = ctx.size.0.clamp(min_size.0, max_size.0);
                    ctx.size.1 = ctx.size.1.clamp(min_size.1, max_size.1);
                }))
            }

            (_, Event::SetZIndex(z_index)) => {
                let z_index = *z_index;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.z_index = z_index;
                }))
            }

            (_, Event::Focus { is_keyboard }) => {
                let is_keyboard = *is_keyboard;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = is_keyboard;
                    })
                    .with_effect(PendingEffect::named(Effect::AllocateZIndex)),
                )
            }

            (_, Event::Blur) => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            (_, Event::BringToFront) => Some(
                TransitionPlan::new().with_effect(PendingEffect::named(Effect::AllocateZIndex)),
            ),

            (State::Idle, Event::DragStart) if ctx.open && props.draggable && !ctx.maximized => {
                Some(
                    TransitionPlan::to(State::Moving)
                        .with_effect(PendingEffect::named(Effect::AllocateZIndex)),
                )
            }

            (State::Moving, Event::DragMove { dx, dy }) => {
                let dx = *dx;
                let dy = *dy;
                let props = props.clone();
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        let moved = (ctx.position.0 + dx, ctx.position.1 + dy);

                        let constrained = clamp_position(moved, ctx.size, &props);

                        ctx.position = snap_position(constrained, props.grid_size);
                    })
                    .with_effect(PendingEffect::named(Effect::PositionChange)),
                )
            }

            (State::Idle, Event::KeyboardMove { dx, dy })
                if ctx.open && ctx.focused && props.draggable && !ctx.maximized =>
            {
                let dx = *dx;
                let dy = *dy;
                let props = props.clone();
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        let moved = (ctx.position.0 + dx, ctx.position.1 + dy);
                        let constrained = clamp_position(moved, ctx.size, &props);

                        ctx.position = snap_position(constrained, props.grid_size);
                    })
                    .with_effect(PendingEffect::named(Effect::PositionChange))
                    .with_effect(PendingEffect::named(Effect::PositionChangeEnd)),
                )
            }

            (State::Moving, Event::DragEnd) => Some(
                TransitionPlan::to(State::Idle)
                    .with_effect(PendingEffect::named(Effect::PositionChangeEnd)),
            ),

            (State::Idle, Event::ResizeStart(handle))
                if ctx.open && props.resizable && !ctx.maximized =>
            {
                let handle = *handle;
                Some(TransitionPlan::to(State::Resizing { handle }).apply(
                    move |ctx: &mut Context| {
                        ctx.active_resize_handle = Some(handle);
                    },
                ))
            }

            (State::Resizing { handle }, Event::ResizeMove { dx, dy }) => {
                let handle = *handle;
                let dx = *dx;
                let dy = *dy;
                let props = props.clone();
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        let (position, size) =
                            resize_rect(ctx.position, ctx.size, handle, dx, dy, &props);

                        ctx.position = position;
                        ctx.size = size;
                    })
                    .with_effect(PendingEffect::named(Effect::SizeChange)),
                )
            }

            (State::Resizing { .. }, Event::ResizeEnd) => Some(
                TransitionPlan::to(State::Idle)
                    .apply(|ctx: &mut Context| {
                        ctx.active_resize_handle = None;
                    })
                    .with_effect(PendingEffect::named(Effect::SizeChangeEnd)),
            ),

            (State::Idle, Event::Minimize) if ctx.open && props.minimizable => {
                Some(stage_plan(State::Minimized, |ctx| {
                    ctx.minimized = true;
                    ctx.maximized = false;
                }))
            }

            (State::Maximized, Event::Minimize) if ctx.open && props.minimizable => {
                Some(stage_plan(State::Minimized, |ctx| {
                    if let Some(position) = ctx.pre_maximize_position {
                        ctx.position = position;
                    }

                    if let Some(size) = ctx.pre_maximize_size {
                        ctx.size = size;
                    }

                    ctx.pre_maximize_position = None;
                    ctx.pre_maximize_size = None;
                    ctx.minimized = true;
                    ctx.maximized = false;
                }))
            }

            (State::Minimized, Event::Restore) if ctx.open => {
                Some(stage_plan(State::Idle, |ctx| {
                    ctx.minimized = false;
                }))
            }

            (State::Idle, Event::Maximize(metrics)) if ctx.open && props.maximizable => {
                let metrics = *metrics;
                Some(stage_plan(State::Maximized, move |ctx| {
                    ctx.pre_maximize_position = Some(ctx.position);
                    ctx.pre_maximize_size = Some(ctx.size);
                    ctx.position = (metrics.viewport.x, metrics.viewport.y);
                    ctx.size = (metrics.viewport.width, metrics.viewport.height);
                    ctx.minimized = false;
                    ctx.maximized = true;
                }))
            }

            (State::Maximized, Event::Restore | Event::Maximize(_)) if ctx.open => {
                Some(stage_plan(State::Idle, |ctx| {
                    if let Some(position) = ctx.pre_maximize_position {
                        ctx.position = position;
                    }

                    if let Some(size) = ctx.pre_maximize_size {
                        ctx.size = size;
                    }

                    ctx.pre_maximize_position = None;
                    ctx.pre_maximize_size = None;
                    ctx.maximized = false;
                }))
            }

            (_, Event::Close) if props.closable => Some(close_plan(ctx.stage() != Stage::Default)),

            (State::Idle | State::Minimized | State::Maximized, Event::CloseOnEscape)
                if props.close_on_escape =>
            {
                Some(close_plan(ctx.stage() != Stage::Default))
            }

            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        context: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            ctx: context,
            props,
            send,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old.id != new.id {
            panic!("FloatingPanel id cannot change after initialization");
        }

        let mut events = Vec::new();

        if old.open != new.open
            && let Some(open) = new.open
        {
            events.push(Event::SetControlledOpen(open));
        }

        if props_changed(old, new) {
            events.push(Event::SyncProps);
        }

        events
    }

    fn initial_effects(
        _state: &Self::State,
        context: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        if context.open {
            vec![
                PendingEffect::named(Effect::OpenChange),
                PendingEffect::named(Effect::AllocateZIndex),
            ]
        } else {
            Vec::new()
        }
    }
}

/// Connected `FloatingPanel` API.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish()
    }
}

impl Api<'_> {
    /// Returns whether the panel is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.ctx.open
    }

    /// Returns whether the panel is minimized.
    #[must_use]
    pub const fn is_minimized(&self) -> bool {
        self.ctx.minimized
    }

    /// Returns whether the panel is maximized.
    #[must_use]
    pub const fn is_maximized(&self) -> bool {
        self.ctx.maximized
    }

    /// Returns whether the panel is moving.
    #[must_use]
    pub const fn is_moving(&self) -> bool {
        matches!(self.state, State::Moving)
    }

    /// Returns whether the panel is resizing.
    #[must_use]
    pub const fn is_resizing(&self) -> bool {
        matches!(self.state, State::Resizing { .. })
    }

    /// Returns the current position.
    #[must_use]
    pub const fn position(&self) -> (f64, f64) {
        self.ctx.position
    }

    /// Returns the current size.
    #[must_use]
    pub const fn size(&self) -> (f64, f64) {
        self.ctx.size
    }

    /// Returns the current stage.
    #[must_use]
    pub const fn stage(&self) -> Stage {
        self.ctx.stage()
    }

    const fn state_token(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Moving => "moving",
            State::Resizing { .. } => "resizing",
            State::Minimized => "minimized",
            State::Maximized => "maximized",
        }
    }

    /// Returns attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Role, "dialog")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("title"),
            )
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(HtmlAttr::Data("ars-stage"), self.stage().as_str())
            .set_style(CssProperty::Position, "fixed")
            .set_style(CssProperty::Left, format!("{}px", self.ctx.position.0))
            .set_style(CssProperty::Top, format!("{}px", self.ctx.position.1))
            .set_style(CssProperty::Width, format!("{}px", self.ctx.size.0))
            .set_style(CssProperty::Height, format!("{}px", self.ctx.size.1))
            .set_style(CssProperty::ZIndex, self.ctx.z_index.to_string());

        if self.props.modal {
            attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "true");
        }

        if self.ctx.focus_visible {
            attrs.set(HtmlAttr::Data("ars-focus-visible"), "true");
        }

        if self.ctx.minimized {
            attrs.set(HtmlAttr::Data("ars-minimized"), "true");
        }

        if self.ctx.maximized {
            attrs.set(HtmlAttr::Data("ars-maximized"), "true");
        }

        if self.is_moving() {
            attrs.set(HtmlAttr::Data("ars-dragging"), "true");
        }

        if self.is_resizing() {
            attrs.set(HtmlAttr::Data("ars-resizing"), "true");
        }

        attrs
    }

    /// Returns attributes for the header element.
    #[must_use]
    pub fn header_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Header.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the drag handle.
    #[must_use]
    pub fn drag_handle_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DragHandle.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.move_label)(&self.ctx.locale),
            );

        if self.props.draggable && !self.ctx.maximized {
            attrs.set_style(CssProperty::Cursor, "grab");
        }

        attrs
    }

    /// Returns attributes for the title element.
    #[must_use]
    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("title"));

        attrs
    }

    /// Returns attributes for the content element.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if self.ctx.minimized {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Returns attributes for the footer element.
    #[must_use]
    pub fn footer_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Footer.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if self.ctx.minimized {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Returns attributes for a resize handle.
    #[must_use]
    pub fn resize_handle_attrs(&self, handle: ResizeHandle) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            (Part::ResizeHandle { handle }).data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-handle"), handle.as_str())
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.resize_handle_label)(handle, &self.ctx.locale),
            );

        if self.props.resizable && !self.ctx.maximized {
            attrs.set_style(CssProperty::Cursor, handle.cursor());
        }

        attrs
    }

    /// Returns attributes for the close trigger.
    #[must_use]
    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.close_label)(&self.ctx.locale),
            );

        attrs
    }

    /// Returns attributes for the minimize trigger.
    #[must_use]
    pub fn minimize_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::MinimizeTrigger.data_attrs();

        let label = if self.ctx.minimized {
            (self.ctx.messages.restore_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.minimize_label)(&self.ctx.locale)
        };

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Label), label);

        attrs
    }

    /// Returns attributes for the maximize trigger.
    #[must_use]
    pub fn maximize_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::MaximizeTrigger.data_attrs();

        let label = if self.ctx.maximized {
            attrs.set(HtmlAttr::Data("ars-maximized"), "true");

            (self.ctx.messages.restore_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.maximize_label)(&self.ctx.locale)
        };

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Label), label);

        attrs
    }

    /// Returns attributes for the combined stage trigger.
    #[must_use]
    pub fn stage_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StageTrigger.data_attrs();

        let stage = self.stage();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Data("ars-state"), stage.as_str())
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.stage_trigger_label)(stage, &self.ctx.locale),
            );

        attrs
    }

    /// Dispatches drag start.
    pub fn on_drag_start(&self) {
        (self.send)(Event::DragStart);
    }

    /// Dispatches drag movement.
    pub fn on_drag_move(&self, dx: f64, dy: f64) {
        (self.send)(Event::DragMove { dx, dy });
    }

    /// Dispatches drag end.
    pub fn on_drag_end(&self) {
        (self.send)(Event::DragEnd);
    }

    /// Dispatches resize start.
    pub fn on_resize_start(&self, handle: ResizeHandle) {
        (self.send)(Event::ResizeStart(handle));
    }

    /// Dispatches resize movement.
    pub fn on_resize_move(&self, dx: f64, dy: f64) {
        (self.send)(Event::ResizeMove { dx, dy });
    }

    /// Dispatches resize end.
    pub fn on_resize_end(&self) {
        (self.send)(Event::ResizeEnd);
    }

    /// Dispatches close trigger activation.
    pub fn on_close_trigger_click(&self) {
        (self.send)(Event::Close);
    }

    /// Dispatches minimize trigger activation.
    pub fn on_minimize_trigger_click(&self) {
        if self.ctx.minimized {
            (self.send)(Event::Restore);
        } else {
            (self.send)(Event::Minimize);
        }
    }

    /// Dispatches maximize trigger activation.
    pub fn on_maximize_trigger_click(&self, metrics: MaximizeMetrics) {
        (self.send)(Event::Maximize(metrics));
    }

    /// Dispatches stage trigger activation.
    pub fn on_stage_trigger_click(&self) {
        if self.ctx.minimized || self.ctx.maximized {
            (self.send)(Event::Restore);
        } else {
            (self.send)(Event::Minimize);
        }
    }

    /// Dispatches focus entry.
    pub fn on_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Dispatches focus exit.
    pub fn on_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Handles panel keydown and returns whether the key was consumed.
    #[must_use]
    pub fn on_keydown(&self, data: &KeyboardEventData) -> bool {
        match data.key {
            KeyboardKey::Escape
                if self.ctx.open
                    && self.props.close_on_escape
                    && matches!(
                        self.state,
                        State::Idle | State::Minimized | State::Maximized
                    ) =>
            {
                (self.send)(Event::CloseOnEscape);

                true
            }

            KeyboardKey::ArrowRight
            | KeyboardKey::ArrowLeft
            | KeyboardKey::ArrowDown
            | KeyboardKey::ArrowUp
                if self.ctx.open
                    && self.ctx.focused
                    && self.props.draggable
                    && !self.ctx.maximized
                    && matches!(self.state, State::Idle) =>
            {
                let step = if data.shift_key { 10.0 } else { 1.0 };
                let (dx, dy) = match data.key {
                    KeyboardKey::ArrowRight => (step, 0.0),
                    KeyboardKey::ArrowLeft => (-step, 0.0),
                    KeyboardKey::ArrowDown => (0.0, step),
                    KeyboardKey::ArrowUp => (0.0, -step),
                    _ => unreachable!("matched arrow keys above"),
                };

                (self.send)(Event::KeyboardMove { dx, dy });

                true
            }

            _ => false,
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

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};
    use core::cell::RefCell;

    use ars_core::{AriaAttr, AttrMap, CssProperty, Env, HtmlAttr, Service};
    use ars_interactions::{KeyboardEventData, KeyboardKey};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "floating-panel".to_string(),
            ..Props::default()
        }
    }

    fn keyboard_data(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character: None,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
        }
    }

    fn shifted_keyboard_data(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            shift_key: true,
            ..keyboard_data(key)
        }
    }

    fn effect_names(result: &ars_core::SendResult<Machine>) -> Vec<Effect> {
        result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn default_init_starts_idle_open_with_initial_rect() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().open);
        assert_eq!(service.context().position, (100.0, 100.0));
        assert_eq!(service.context().size, (400.0, 300.0));
        assert_eq!(service.context().min_size, (200.0, 150.0));
    }

    #[test]
    fn initial_open_state_emits_open_lifecycle_effects_once() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let initial = service
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect::<Vec<_>>();

        assert_eq!(initial, vec![Effect::OpenChange, Effect::AllocateZIndex]);
        assert!(service.take_initial_effects().is_empty());

        let mut closed = Service::<Machine>::new(
            Props {
                open: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert!(closed.take_initial_effects().is_empty());
    }

    #[test]
    fn controlled_open_overrides_default() {
        let service = Service::<Machine>::new(
            Props {
                open: Some(false),
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().open);
    }

    #[test]
    fn drag_move_updates_position_and_snaps_to_grid() {
        let mut service = Service::<Machine>::new(
            Props {
                grid_size: 10.0,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        let start = service.send(Event::DragStart);

        assert_eq!(service.state(), &State::Moving);
        assert_eq!(effect_names(&start), vec![Effect::AllocateZIndex]);

        let move_result = service.send(Event::DragMove { dx: 13.0, dy: 27.0 });

        assert_eq!(service.context().position, (110.0, 130.0));
        assert_eq!(effect_names(&move_result), vec![Effect::PositionChange]);

        let end = service.send(Event::DragEnd);

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(effect_names(&end), vec![Effect::PositionChangeEnd]);
    }

    #[test]
    fn drag_ignored_when_disabled_or_maximized() {
        let mut disabled = Service::<Machine>::new(
            Props {
                draggable: false,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert!(!disabled.send(Event::DragStart).state_changed);
        assert_eq!(disabled.state(), &State::Idle);

        let mut maximized =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(maximized.send(Event::Maximize(MaximizeMetrics {
            viewport: ViewportRect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            },
        })));

        assert_eq!(maximized.state(), &State::Maximized);
        assert!(!maximized.send(Event::DragStart).state_changed);
    }

    #[test]
    fn resize_edges_and_corners_clamp_to_constraints() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::ResizeStart(ResizeHandle::NW)));

        let resize = service.send(Event::ResizeMove {
            dx: 260.0,
            dy: 200.0,
        });

        assert_eq!(
            service.state(),
            &State::Resizing {
                handle: ResizeHandle::NW
            }
        );
        assert_eq!(service.context().size, (200.0, 150.0));
        assert_eq!(service.context().position, (300.0, 250.0));
        assert_eq!(effect_names(&resize), vec![Effect::SizeChange]);

        let end = service.send(Event::ResizeEnd);

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().active_resize_handle.is_none());
        assert_eq!(effect_names(&end), vec![Effect::SizeChangeEnd]);
    }

    #[test]
    fn minimize_maximize_restore_and_close_track_stage() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let minimize = service.send(Event::Minimize);

        assert_eq!(service.state(), &State::Minimized);
        assert_eq!(service.context().stage(), Stage::Minimized);
        assert_eq!(effect_names(&minimize), vec![Effect::StageChange]);

        drop(service.send(Event::Restore));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().stage(), Stage::Default);

        let maximize = service.send(Event::Maximize(MaximizeMetrics {
            viewport: ViewportRect {
                x: 10.0,
                y: 20.0,
                width: 900.0,
                height: 700.0,
            },
        }));

        assert_eq!(service.state(), &State::Maximized);
        assert_eq!(service.context().position, (10.0, 20.0));
        assert_eq!(service.context().size, (900.0, 700.0));
        assert_eq!(effect_names(&maximize), vec![Effect::StageChange]);

        let minimize_from_maximized = service.send(Event::Minimize);

        assert_eq!(service.state(), &State::Minimized);
        assert_eq!(service.context().stage(), Stage::Minimized);
        assert_eq!(service.context().position, (100.0, 100.0));
        assert_eq!(service.context().size, (400.0, 300.0));
        assert_eq!(
            effect_names(&minimize_from_maximized),
            vec![Effect::StageChange]
        );

        drop(service.send(Event::Restore));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().position, (100.0, 100.0));
        assert_eq!(service.context().size, (400.0, 300.0));

        let close = service.send(Event::Close);

        assert!(close.state_changed);
        assert!(!service.context().open);
        assert_eq!(service.state(), &State::Idle);
        assert_eq!(effect_names(&close), vec![Effect::OpenChange]);
    }

    #[test]
    fn close_from_non_default_stage_emits_stage_change() {
        let mut minimized =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(minimized.send(Event::Minimize));

        let close_minimized = minimized.send(Event::Close);

        assert_eq!(minimized.context().stage(), Stage::Default);
        assert_eq!(
            effect_names(&close_minimized),
            vec![Effect::OpenChange, Effect::StageChange]
        );

        let mut maximized =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(maximized.send(Event::Maximize(MaximizeMetrics {
            viewport: ViewportRect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            },
        })));

        let close_maximized = maximized.send(Event::SetControlledOpen(false));

        assert_eq!(maximized.context().stage(), Stage::Default);
        assert_eq!(
            effect_names(&close_maximized),
            vec![Effect::OpenChange, Effect::StageChange]
        );
    }

    #[test]
    fn escape_focus_z_index_and_props_sync() {
        let mut service = Service::<Machine>::new(
            Props {
                close_on_escape: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        let focus = service.send(Event::Focus { is_keyboard: true });

        assert!(service.context().focused);
        assert!(service.context().focus_visible);
        assert_eq!(effect_names(&focus), vec![Effect::AllocateZIndex]);

        drop(service.send(Event::SetZIndex(88)));

        assert_eq!(service.context().z_index, 88);

        let close = service.send(Event::CloseOnEscape);

        assert!(!service.context().open);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().focused);
        assert_eq!(effect_names(&close), vec![Effect::OpenChange]);

        service.set_props(Props {
            open: Some(true),
            min_size: (300.0, 200.0),
            ..test_props()
        });

        assert!(service.context().open);
        assert_eq!(service.context().min_size, (300.0, 200.0));
    }

    #[test]
    fn controlled_reopen_resets_rect_unless_persisted() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::DragStart));
        drop(service.send(Event::DragMove { dx: 50.0, dy: 60.0 }));
        drop(service.send(Event::DragEnd));
        drop(service.send(Event::ResizeStart(ResizeHandle::SE)));
        drop(service.send(Event::ResizeMove { dx: 20.0, dy: 30.0 }));
        drop(service.send(Event::ResizeEnd));

        let close = service.send(Event::SetControlledOpen(false));

        assert_eq!(effect_names(&close), vec![Effect::OpenChange]);
        assert!(!service.context().open);

        let reopen = service.send(Event::SetControlledOpen(true));

        assert!(service.context().open);
        assert_eq!(service.context().position, (100.0, 100.0));
        assert_eq!(service.context().size, (400.0, 300.0));
        assert_eq!(
            effect_names(&reopen),
            vec![Effect::OpenChange, Effect::AllocateZIndex]
        );

        let noop = service.send(Event::SetControlledOpen(true));

        assert!(!noop.state_changed);
        assert!(!noop.context_changed);
        assert!(noop.pending_effects.is_empty());

        let mut persisted = Service::<Machine>::new(
            Props {
                persist_rect: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(persisted.send(Event::DragStart));
        drop(persisted.send(Event::DragMove { dx: 40.0, dy: 30.0 }));
        drop(persisted.send(Event::DragEnd));
        let saved_position = persisted.context().position;
        let saved_size = persisted.context().size;

        drop(persisted.send(Event::SetControlledOpen(false)));
        drop(persisted.send(Event::SetControlledOpen(true)));

        assert_eq!(persisted.context().position, saved_position);
        assert_eq!(persisted.context().size, saved_size);
    }

    #[test]
    fn controlled_close_clears_active_stage_flags() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::Maximize(MaximizeMetrics {
            viewport: ViewportRect {
                x: 0.0,
                y: 0.0,
                width: 900.0,
                height: 700.0,
            },
        })));

        assert_eq!(service.state(), &State::Maximized);
        assert!(service.context().maximized);

        drop(service.send(Event::SetControlledOpen(false)));

        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().open);
        assert!(!service.context().maximized);
        assert_eq!(service.context().stage(), Stage::Default);
        assert!(service.context().pre_maximize_position.is_none());
        assert!(service.context().pre_maximize_size.is_none());
    }

    #[test]
    fn minimize_trigger_restores_when_minimized() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::Minimize));

        let sent = RefCell::new(Vec::new());
        let send = |event| sent.borrow_mut().push(event);
        let api = service.connect(&send);

        api.on_minimize_trigger_click();

        assert_eq!(sent.into_inner(), vec![Event::Restore]);
    }

    #[test]
    fn minimize_trigger_minimizes_from_maximized() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::Maximize(MaximizeMetrics {
            viewport: ViewportRect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            },
        })));

        let sent = RefCell::new(Vec::new());
        let send = |event| sent.borrow_mut().push(event);
        let api = service.connect(&send);

        api.on_minimize_trigger_click();

        assert_eq!(sent.into_inner(), vec![Event::Minimize]);
    }

    #[test]
    fn locked_aspect_ratio_resize_stays_within_constraints() {
        let mut service = Service::<Machine>::new(
            Props {
                lock_aspect_ratio: true,
                max_size: (500.0, 250.0),
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::ResizeStart(ResizeHandle::E)));
        drop(service.send(Event::ResizeMove { dx: 300.0, dy: 0.0 }));

        assert_eq!(service.context().size, (500.0, 250.0));
    }

    #[test]
    fn arrow_keys_nudge_focused_panel_position() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let unfocused_sent = RefCell::new(Vec::new());
        let send = |event| unfocused_sent.borrow_mut().push(event);
        let unfocused = service.connect(&send);

        assert!(!unfocused.on_keydown(&keyboard_data(KeyboardKey::ArrowRight)));
        assert!(unfocused_sent.into_inner().is_empty());

        drop(service.send(Event::Focus { is_keyboard: true }));

        let sent = RefCell::new(Vec::new());
        let send = |event| sent.borrow_mut().push(event);
        let api = service.connect(&send);

        assert!(api.on_keydown(&shifted_keyboard_data(KeyboardKey::ArrowRight)));
        assert!(api.on_keydown(&keyboard_data(KeyboardKey::ArrowUp)));
        assert!(!api.on_keydown(&keyboard_data(KeyboardKey::Tab)));

        assert_eq!(
            sent.into_inner(),
            vec![
                Event::KeyboardMove { dx: 10.0, dy: 0.0 },
                Event::KeyboardMove { dx: 0.0, dy: -1.0 }
            ]
        );

        let move_result = service.send(Event::KeyboardMove { dx: 10.0, dy: -1.0 });

        assert_eq!(service.context().position, (110.0, 99.0));
        assert_eq!(
            effect_names(&move_result),
            vec![Effect::PositionChange, Effect::PositionChangeEnd]
        );

        drop(service.send(Event::Blur));

        let ignored = service.send(Event::KeyboardMove { dx: 5.0, dy: 0.0 });

        assert!(!ignored.state_changed);
        assert!(!ignored.context_changed);
        assert_eq!(service.context().position, (110.0, 99.0));
    }

    #[test]
    fn keydown_reports_handled_only_when_transition_can_apply() {
        let closed = Service::<Machine>::new(
            Props {
                open: Some(false),
                close_on_escape: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );
        let closed_events = RefCell::new(Vec::new());
        let send = |event| closed_events.borrow_mut().push(event);
        let closed_api = closed.connect(&send);

        assert!(!closed_api.on_keydown(&keyboard_data(KeyboardKey::Escape)));
        assert!(closed_events.into_inner().is_empty());

        let mut open = Service::<Machine>::new(
            Props {
                close_on_escape: false,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(open.send(Event::Focus { is_keyboard: true }));

        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);
        let api = open.connect(&send);

        assert!(!api.on_keydown(&keyboard_data(KeyboardKey::Escape)));
        assert!(api.on_keydown(&keyboard_data(KeyboardKey::ArrowDown)));
        assert_eq!(
            events.into_inner(),
            vec![Event::KeyboardMove { dx: 0.0, dy: 1.0 }]
        );

        drop(open.send(Event::Maximize(MaximizeMetrics {
            viewport: ViewportRect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            },
        })));

        let maximized_events = RefCell::new(Vec::new());
        let send = |event| maximized_events.borrow_mut().push(event);
        let maximized_api = open.connect(&send);

        assert!(!maximized_api.on_keydown(&keyboard_data(KeyboardKey::ArrowRight)));
        assert!(maximized_events.into_inner().is_empty());
    }

    #[test]
    fn connect_api_root_and_controls_attrs() {
        let mut service = Service::<Machine>::new(
            Props {
                modal: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::Focus { is_keyboard: true }));
        drop(service.send(Event::SetZIndex(77)));

        let api = service.connect(&|_| {});

        let root = api.root_attrs();
        let drag_handle = api.drag_handle_attrs();
        let resize = api.resize_handle_attrs(ResizeHandle::SE);

        assert_eq!(root.get(&HtmlAttr::Role), Some("dialog"));
        assert_eq!(root.get(&HtmlAttr::Aria(AriaAttr::Modal)), Some("true"));
        assert_eq!(root.get(&HtmlAttr::Data("ars-focus-visible")), Some("true"));
        assert!(
            root.styles()
                .contains(&(CssProperty::ZIndex, "77".to_string()))
        );
        assert_eq!(
            drag_handle.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Move panel")
        );
        assert_eq!(resize.get(&HtmlAttr::Data("ars-handle")), Some("se"));
        assert_eq!(
            resize.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Resize bottom-right")
        );
    }

    #[test]
    fn floating_panel_snapshots_cover_parts_and_states() {
        let mut service = Service::<Machine>::new(
            Props {
                modal: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::Focus { is_keyboard: true }));
        drop(service.send(Event::SetZIndex(90)));

        let api = service.connect(&|_| {});

        assert_snapshot!(
            "floating_panel_root_idle",
            snapshot_attrs(&api.root_attrs())
        );
        assert_snapshot!("floating_panel_header", snapshot_attrs(&api.header_attrs()));
        assert_snapshot!(
            "floating_panel_drag_handle",
            snapshot_attrs(&api.drag_handle_attrs())
        );
        assert_snapshot!("floating_panel_title", snapshot_attrs(&api.title_attrs()));
        assert_snapshot!(
            "floating_panel_content",
            snapshot_attrs(&api.content_attrs())
        );
        assert_snapshot!("floating_panel_footer", snapshot_attrs(&api.footer_attrs()));
        assert_snapshot!(
            "floating_panel_resize_handle_se",
            snapshot_attrs(&api.resize_handle_attrs(ResizeHandle::SE))
        );
        assert_snapshot!(
            "floating_panel_resize_handle_n",
            snapshot_attrs(&api.resize_handle_attrs(ResizeHandle::N))
        );
        assert_snapshot!(
            "floating_panel_resize_handle_s",
            snapshot_attrs(&api.resize_handle_attrs(ResizeHandle::S))
        );
        assert_snapshot!(
            "floating_panel_resize_handle_e",
            snapshot_attrs(&api.resize_handle_attrs(ResizeHandle::E))
        );
        assert_snapshot!(
            "floating_panel_resize_handle_w",
            snapshot_attrs(&api.resize_handle_attrs(ResizeHandle::W))
        );
        assert_snapshot!(
            "floating_panel_resize_handle_ne",
            snapshot_attrs(&api.resize_handle_attrs(ResizeHandle::NE))
        );
        assert_snapshot!(
            "floating_panel_resize_handle_nw",
            snapshot_attrs(&api.resize_handle_attrs(ResizeHandle::NW))
        );
        assert_snapshot!(
            "floating_panel_resize_handle_sw",
            snapshot_attrs(&api.resize_handle_attrs(ResizeHandle::SW))
        );
        assert_snapshot!(
            "floating_panel_close_trigger",
            snapshot_attrs(&api.close_trigger_attrs())
        );
        assert_snapshot!(
            "floating_panel_minimize_trigger",
            snapshot_attrs(&api.minimize_trigger_attrs())
        );
        assert_snapshot!(
            "floating_panel_maximize_trigger",
            snapshot_attrs(&api.maximize_trigger_attrs())
        );
        assert_snapshot!(
            "floating_panel_stage_trigger",
            snapshot_attrs(&api.stage_trigger_attrs())
        );

        drop(service.send(Event::DragStart));

        let moving = service.connect(&|_| {});

        assert_snapshot!(
            "floating_panel_root_moving",
            snapshot_attrs(&moving.root_attrs())
        );

        drop(service.send(Event::DragEnd));

        drop(service.send(Event::ResizeStart(ResizeHandle::E)));

        let resizing = service.connect(&|_| {});

        assert_snapshot!(
            "floating_panel_root_resizing",
            snapshot_attrs(&resizing.root_attrs())
        );

        drop(service.send(Event::ResizeEnd));

        drop(service.send(Event::Minimize));

        let minimized = service.connect(&|_| {});

        assert_snapshot!(
            "floating_panel_root_minimized",
            snapshot_attrs(&minimized.root_attrs())
        );
        assert_snapshot!(
            "floating_panel_content_minimized",
            snapshot_attrs(&minimized.content_attrs())
        );
        assert_snapshot!(
            "floating_panel_footer_minimized",
            snapshot_attrs(&minimized.footer_attrs())
        );

        drop(service.send(Event::Restore));
        drop(service.send(Event::Maximize(MaximizeMetrics {
            viewport: ViewportRect {
                x: 0.0,
                y: 0.0,
                width: 900.0,
                height: 700.0,
            },
        })));

        let maximized = service.connect(&|_| {});

        assert_snapshot!(
            "floating_panel_root_maximized",
            snapshot_attrs(&maximized.root_attrs())
        );
        assert_snapshot!(
            "floating_panel_maximize_trigger_maximized",
            snapshot_attrs(&maximized.maximize_trigger_attrs())
        );

        let not_draggable = Service::<Machine>::new(
            Props {
                draggable: false,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert_snapshot!(
            "floating_panel_drag_handle_not_draggable",
            snapshot_attrs(&not_draggable.connect(&|_| {}).drag_handle_attrs())
        );
    }

    #[test]
    fn api_handlers_dispatch_expected_events() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());
        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_drag_start();
        api.on_drag_move(1.0, 2.0);
        api.on_drag_end();
        api.on_resize_start(ResizeHandle::S);
        api.on_resize_move(3.0, 4.0);
        api.on_resize_end();
        api.on_close_trigger_click();
        api.on_minimize_trigger_click();
        api.on_maximize_trigger_click(MaximizeMetrics {
            viewport: ViewportRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        });
        api.on_stage_trigger_click();
        api.on_focus(true);
        api.on_blur();

        assert!(api.on_keydown(&keyboard_data(KeyboardKey::Escape)));

        assert_eq!(events.borrow().len(), 13);
    }
}
