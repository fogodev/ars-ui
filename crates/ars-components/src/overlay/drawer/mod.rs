//! Drawer slide-in overlay machine.
//!
//! Owns dialog-like open/close state, edge placement resolution, semantic IDs,
//! ARIA/data attribute output, bottom-sheet snap state, adapter-normalized drag
//! math, z-index intent, and adapter-resolvable focus/scroll/inert effects.
//!
//! The agnostic core never measures the panel or viewport, never captures
//! pointers, never traps focus by traversing DOM nodes, and never resolves live
//! elements by ID. Framework adapters provide live handles, pointer capture,
//! geometry measurement, focus trap wiring, portal integration, and actual
//! transform/style application.

use alloc::{
    string::{String, ToString as _},
    vec,
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_a11y::FocusTarget;
use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    CssProperty, Direction, Env, HtmlAttr, Locale, MessageFn, PendingEffect, ResolvedDirection,
    TransitionPlan,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

use crate::utility::dismissable::DismissAttempt;

type SnapValueTextFn = dyn Fn(f64, &Locale) -> String + Send + Sync;

const DISMISS_THRESHOLD: f64 = 0.7;
const VELOCITY_THRESHOLD: f64 = 0.5;

/// States for the [`Drawer`](self) component.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum State {
    /// The drawer is closed and not visible.
    #[default]
    Closed,

    /// The drawer is open and settled.
    Open,

    /// The drawer is being dragged by an adapter-normalized pointer gesture.
    Dragging(f64),
}

/// Events accepted by the [`Drawer`](self) state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open the drawer.
    Open,

    /// Close the drawer.
    Close,

    /// Toggle the drawer open or closed.
    Toggle,

    /// Start a drag gesture at the adapter-normalized offset.
    DragStart(f64),

    /// Move an active drag gesture to the adapter-normalized offset.
    DragMove(f64),

    /// End a drag gesture with adapter-normalized offset and velocity.
    ///
    /// `offset` is normalized on the drawer axis: `0.0` means fully open and
    /// `1.0` means fully dismissed. Positive velocity moves toward dismissal;
    /// negative velocity moves toward expansion.
    DragEnd {
        /// Final normalized drag offset.
        offset: f64,

        /// Final normalized velocity, in drawer-axis units per second.
        velocity: f64,
    },

    /// Snap to the requested snap-point index.
    SnapTo(usize),

    /// Adapter reported the allocated z-index.
    SetZIndex(u32),

    /// Close because the backdrop was activated.
    CloseOnBackdropClick,

    /// Close because Escape was pressed.
    CloseOnEscape,

    /// A title part mounted and should label content.
    RegisterTitle,

    /// A title part unmounted and should no longer label content.
    UnregisterTitle,

    /// A description part mounted and should describe content.
    RegisterDescription,

    /// A description part unmounted and should no longer describe content.
    UnregisterDescription,

    /// Re-apply context-backed props after prop changes.
    SyncProps,
}

/// Logical placement for the drawer before text-direction resolution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Placement {
    /// The top edge of the viewport.
    Top,

    /// The bottom edge of the viewport.
    Bottom,

    /// The left edge of the viewport.
    Left,

    /// The right edge of the viewport.
    #[default]
    Right,

    /// The inline-start edge: left in LTR, right in RTL.
    Start,

    /// The inline-end edge: right in LTR, left in RTL.
    End,
}

impl Placement {
    /// Converts logical placement to a physical placement.
    #[must_use]
    pub const fn to_physical(self, dir: ResolvedDirection) -> ResolvedPlacement {
        match (self, dir) {
            (Self::Start, ResolvedDirection::Ltr)
            | (Self::End, ResolvedDirection::Rtl)
            | (Self::Left, _) => ResolvedPlacement::Left,
            (Self::Start, ResolvedDirection::Rtl)
            | (Self::End, ResolvedDirection::Ltr)
            | (Self::Right, _) => ResolvedPlacement::Right,
            (Self::Top, _) => ResolvedPlacement::Top,
            (Self::Bottom, _) => ResolvedPlacement::Bottom,
        }
    }

    /// Closed-state CSS translation token for this placement.
    #[must_use]
    pub const fn as_css_translate(self) -> &'static str {
        match self {
            Self::Top => "translateY(-100%)",
            Self::Bottom => "translateY(100%)",
            Self::Left => "translateX(-100%)",
            Self::Right | Self::Start | Self::End => "translateX(100%)",
        }
    }

    /// Stable data-attribute token for this logical placement.
    #[must_use]
    pub const fn as_data_attr(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
            Self::Start => "start",
            Self::End => "end",
        }
    }
}

/// Physical drawer placement after resolving [`Placement::Start`] and
/// [`Placement::End`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ResolvedPlacement {
    /// The top edge of the viewport.
    Top,

    /// The bottom edge of the viewport.
    Bottom,

    /// The left edge of the viewport.
    Left,

    /// The right edge of the viewport.
    #[default]
    Right,
}

impl ResolvedPlacement {
    /// Stable data-attribute token for this physical placement.
    #[must_use]
    pub const fn as_data_attr(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    /// Closed-state CSS translation token for this physical placement.
    #[must_use]
    pub const fn as_css_translate(self) -> &'static str {
        match self {
            Self::Top => "translateY(-100%)",
            Self::Bottom => "translateY(100%)",
            Self::Left => "translateX(-100%)",
            Self::Right => "translateX(100%)",
        }
    }
}

/// Runtime context for [`Drawer`](self).
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the drawer is logically open.
    pub open: bool,

    /// Whether the drawer is modal.
    pub modal: bool,

    /// Logical placement configured by props.
    pub placement: Placement,

    /// Physical placement resolved from [`placement`](Self::placement) and
    /// [`dir`](Self::dir).
    pub resolved_placement: ResolvedPlacement,

    /// Text direction used to resolve logical placement.
    pub dir: Direction,

    /// Whether backdrop clicks may close the drawer.
    pub close_on_backdrop: bool,

    /// Whether the Escape key may close the drawer.
    pub close_on_escape: bool,

    /// Whether the body should be scroll-locked while the drawer is open.
    pub prevent_scroll: bool,

    /// Whether focus should be restored on close.
    pub restore_focus: bool,

    /// Adapter-resolved focus target for opening.
    pub initial_focus: Option<FocusTarget>,

    /// Adapter-resolved focus target for closing.
    pub final_focus: Option<FocusTarget>,

    /// Hydration-stable semantic IDs for drawer parts.
    pub ids: ComponentIds,

    /// Whether a title was registered.
    pub has_title: bool,

    /// Whether a description was registered.
    pub has_description: bool,

    /// Sanitized snap-point fractions in ascending order.
    pub snap_points: Vec<f64>,

    /// Current snap-point index.
    pub current_snap: usize,

    /// Current snap-point fraction.
    pub snap_height: f64,

    /// Adapter-allocated z-index, when reported.
    pub z_index: Option<u32>,

    /// Locale used to resolve messages.
    pub locale: Locale,

    /// Localized messages.
    pub messages: Messages,
}

/// Localizable strings for [`Drawer`](self).
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible role description for content. Defaults to `"drawer"`.
    pub role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the close trigger. Defaults to `"Close drawer"`.
    pub close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the drag handle slider.
    pub drag_handle_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible value text for snap-point slider handles.
    pub snap_value_text: MessageFn<SnapValueTextFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            role_description: MessageFn::static_str("drawer"),
            close_label: MessageFn::static_str("Close drawer"),
            drag_handle_label: MessageFn::static_str("Drawer snap position"),
            snap_value_text: MessageFn::new(|value: f64, _locale: &Locale| {
                alloc::format!("{:.0}%", value * 100.0)
            }),
        }
    }
}

impl ComponentMessages for Messages {}

/// Immutable configuration for a [`Drawer`](self) instance.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Controlled open state.
    pub open: Option<bool>,

    /// Initial uncontrolled open state.
    pub default_open: bool,

    /// Logical drawer placement.
    pub placement: Placement,

    /// Whether the drawer is modal.
    pub modal: bool,

    /// Whether backdrop clicks may close the drawer.
    pub close_on_backdrop: bool,

    /// Whether Escape may close the drawer.
    pub close_on_escape: bool,

    /// Whether the drawer should prevent body scroll.
    pub prevent_scroll: bool,

    /// Whether focus should restore when the drawer closes.
    pub restore_focus: bool,

    /// Adapter-resolved focus target used on open.
    pub initial_focus: Option<FocusTarget>,

    /// Adapter-resolved focus target used on close.
    pub final_focus: Option<FocusTarget>,

    /// Text direction for logical placement resolution.
    pub dir: Direction,

    /// Heading level for the title part.
    pub title_level: u8,

    /// Bottom-sheet snap points as viewport-height fractions.
    pub snap_points: Option<Vec<f64>>,

    /// Initial snap-point index.
    pub default_snap_index: usize,

    /// Callback invoked after open state changes.
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,

    /// Whether adapters should lazily mount content.
    pub lazy_mount: bool,

    /// Whether adapters should unmount content after exit.
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

impl Props {
    /// Returns default drawer props.
    ///
    /// ```
    /// use ars_components::overlay::drawer::{Placement, Props};
    /// use ars_core::Direction;
    ///
    /// let props = Props::new()
    ///     .id("settings")
    ///     .placement(Placement::Start)
    ///     .dir(Direction::Rtl);
    ///
    /// assert_eq!(props.id, "settings");
    /// assert_eq!(props.placement, Placement::Start);
    /// assert_eq!(props.dir, Direction::Rtl);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id).
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`open`](Self::open).
    #[must_use]
    pub const fn open(mut self, value: Option<bool>) -> Self {
        self.open = value;
        self
    }

    /// Sets [`default_open`](Self::default_open).
    #[must_use]
    pub const fn default_open(mut self, value: bool) -> Self {
        self.default_open = value;
        self
    }

    /// Sets [`placement`](Self::placement).
    #[must_use]
    pub const fn placement(mut self, value: Placement) -> Self {
        self.placement = value;
        self
    }

    /// Sets [`dir`](Self::dir).
    #[must_use]
    pub const fn dir(mut self, value: Direction) -> Self {
        self.dir = value;
        self
    }

    /// Sets [`modal`](Self::modal).
    #[must_use]
    pub const fn modal(mut self, value: bool) -> Self {
        self.modal = value;
        self
    }

    /// Sets [`close_on_backdrop`](Self::close_on_backdrop).
    #[must_use]
    pub const fn close_on_backdrop(mut self, value: bool) -> Self {
        self.close_on_backdrop = value;
        self
    }

    /// Sets [`close_on_escape`](Self::close_on_escape).
    #[must_use]
    pub const fn close_on_escape(mut self, value: bool) -> Self {
        self.close_on_escape = value;
        self
    }

    /// Sets [`prevent_scroll`](Self::prevent_scroll).
    #[must_use]
    pub const fn prevent_scroll(mut self, value: bool) -> Self {
        self.prevent_scroll = value;
        self
    }

    /// Sets [`restore_focus`](Self::restore_focus).
    #[must_use]
    pub const fn restore_focus(mut self, value: bool) -> Self {
        self.restore_focus = value;
        self
    }

    /// Sets [`initial_focus`](Self::initial_focus).
    #[must_use]
    pub const fn initial_focus(mut self, value: Option<FocusTarget>) -> Self {
        self.initial_focus = value;
        self
    }

    /// Sets [`final_focus`](Self::final_focus).
    #[must_use]
    pub const fn final_focus(mut self, value: Option<FocusTarget>) -> Self {
        self.final_focus = value;
        self
    }

    /// Sets [`title_level`](Self::title_level).
    #[must_use]
    pub const fn title_level(mut self, value: u8) -> Self {
        self.title_level = value;
        self
    }

    /// Sets [`snap_points`](Self::snap_points).
    #[must_use]
    pub fn snap_points(mut self, value: Option<Vec<f64>>) -> Self {
        self.snap_points = value;
        self
    }

    /// Sets [`default_snap_index`](Self::default_snap_index).
    #[must_use]
    pub const fn default_snap_index(mut self, value: usize) -> Self {
        self.default_snap_index = value;
        self
    }

    /// Sets [`lazy_mount`](Self::lazy_mount).
    #[must_use]
    pub const fn lazy_mount(mut self, value: bool) -> Self {
        self.lazy_mount = value;
        self
    }

    /// Sets [`unmount_on_exit`](Self::unmount_on_exit).
    #[must_use]
    pub const fn unmount_on_exit(mut self, value: bool) -> Self {
        self.unmount_on_exit = value;
        self
    }

    /// Registers [`on_open_change`](Self::on_open_change).
    #[must_use]
    pub fn on_open_change<F>(mut self, f: F) -> Self
    where
        F: Fn(bool) + Send + Sync + 'static,
    {
        self.on_open_change = Some(Callback::new(f));
        self
    }

    /// Registers [`on_escape_key_down`](Self::on_escape_key_down).
    #[must_use]
    pub fn on_escape_key_down<F>(mut self, f: F) -> Self
    where
        F: Fn(DismissAttempt<()>) + Send + Sync + 'static,
    {
        self.on_escape_key_down = Some(Callback::new(f));
        self
    }

    /// Registers [`on_interact_outside`](Self::on_interact_outside).
    #[must_use]
    pub fn on_interact_outside<F>(mut self, f: F) -> Self
    where
        F: Fn(DismissAttempt<()>) + Send + Sync + 'static,
    {
        self.on_interact_outside = Some(Callback::new(f));
        self
    }
}

/// Anatomy parts exposed by the [`Drawer`](self) connect API.
#[derive(ComponentPart)]
#[scope = "drawer"]
pub enum Part {
    /// The root container element.
    Root,

    /// The trigger button that opens the drawer.
    Trigger,

    /// The decorative backdrop.
    Backdrop,

    /// The wrapper that positions the drawer content.
    Positioner,

    /// The drawer content element.
    Content,

    /// The optional title element.
    Title,

    /// The optional description element.
    Description,

    /// The optional header region.
    Header,

    /// The optional body region.
    Body,

    /// The optional footer region.
    Footer,

    /// The optional close trigger.
    CloseTrigger,

    /// The optional drag handle for snap-point interaction.
    DragHandle,
}

/// Typed identifier for every named effect intent the drawer machine emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Open state changed.
    OpenChange,

    /// Adapter should inert background content.
    SetBackgroundInert,

    /// Adapter should restore background content interactivity.
    RemoveBackgroundInert,

    /// Adapter should acquire body scroll lock.
    ScrollLockAcquire,

    /// Adapter should release body scroll lock.
    ScrollLockRelease,

    /// Adapter should resolve initial focus.
    FocusInitial,

    /// Adapter should focus first tabbable content when needed.
    FocusFirstTabbable,

    /// Adapter should restore focus to the close target.
    RestoreFocus,

    /// Adapter should allocate a z-index and send [`Event::SetZIndex`].
    AllocateZIndex,

    /// Adapter should release the allocated z-index claim.
    ReleaseZIndex,

    /// Snap-point state changed.
    SnapChange,
}

/// State machine for the [`Drawer`](self) component.
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
        let open = props.open.unwrap_or(props.default_open);
        let state = if open { State::Open } else { State::Closed };
        let ids = ComponentIds::from_id(&props.id);
        let dir = props.dir.resolve(env.locale.direction());
        let snap_points = normalize_snap_points(props.snap_points.as_deref());
        let current_snap = clamp_snap_index(props.default_snap_index, &snap_points);
        let snap_height = snap_points.get(current_snap).copied().unwrap_or(1.0);

        (
            state,
            Context {
                open,
                modal: props.modal,
                placement: props.placement,
                resolved_placement: props.placement.to_physical(dir),
                dir: props.dir,
                close_on_backdrop: props.close_on_backdrop,
                close_on_escape: props.close_on_escape,
                prevent_scroll: props.prevent_scroll,
                restore_focus: props.restore_focus,
                initial_focus: props.initial_focus,
                final_focus: props.final_focus,
                ids,
                has_title: false,
                has_description: false,
                snap_points,
                current_snap,
                snap_height,
                z_index: None,
                locale: env.locale.clone(),
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
            (State::Closed, Event::Open | Event::Toggle) => Some(open_plan(ctx)),

            (State::Open | State::Dragging(_), Event::Close | Event::Toggle) => {
                Some(close_plan(ctx))
            }

            (State::Open | State::Dragging(_), Event::CloseOnBackdropClick)
                if ctx.close_on_backdrop =>
            {
                Some(close_plan(ctx))
            }

            (State::Open | State::Dragging(_), Event::CloseOnEscape) if ctx.close_on_escape => {
                Some(close_plan(ctx))
            }
            (State::Open, Event::DragStart(offset))
            | (State::Dragging(_), Event::DragMove(offset)) => {
                let offset = clamp_fraction(*offset);
                Some(TransitionPlan::to(State::Dragging(offset)))
            }

            (State::Dragging(_), Event::DragEnd { offset, velocity }) => {
                Some(resolve_drag_end_plan(ctx, *offset, *velocity))
            }

            (State::Open | State::Dragging(_), Event::SnapTo(index)) if snap_points_active(ctx) => {
                let index = clamp_snap_index(*index, &ctx.snap_points);
                let height = ctx.snap_points.get(index).copied().unwrap_or(1.0);
                let snap_changed = index != ctx.current_snap || height != ctx.snap_height;
                let mut plan = TransitionPlan::to(State::Open).apply(move |ctx: &mut Context| {
                    ctx.open = true;
                    ctx.current_snap = index;
                    ctx.snap_height = height;
                });

                if snap_changed {
                    plan = plan.with_effect(PendingEffect::named(Effect::SnapChange));
                }

                Some(plan)
            }

            (State::Open | State::Dragging(_), Event::SetZIndex(z_index)) => {
                let z_index = *z_index;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.z_index = Some(z_index);
                }))
            }

            (_, Event::RegisterTitle) if !ctx.has_title => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.has_title = true;
                }))
            }
            (_, Event::UnregisterTitle) if ctx.has_title => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.has_title = false;
                }))
            }

            (_, Event::RegisterDescription) if !ctx.has_description => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.has_description = true;
                }))
            }
            (_, Event::UnregisterDescription) if ctx.has_description => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.has_description = false;
                }))
            }
            (_, Event::SyncProps) => {
                let modal = props.modal;
                let placement = props.placement;
                let dir = props.dir;
                let resolved_placement = placement.to_physical(dir.resolve(ctx.locale.direction()));
                let close_on_backdrop = props.close_on_backdrop;
                let close_on_escape = props.close_on_escape;
                let prevent_scroll = props.prevent_scroll;
                let restore_focus = props.restore_focus;
                let initial_focus = props.initial_focus;
                let final_focus = props.final_focus;
                let snap_points = normalize_snap_points(props.snap_points.as_deref());
                let default_snap_index = props.default_snap_index;

                let mut plan = TransitionPlan::context_only(move |ctx: &mut Context| {
                    let current_snap = if ctx.snap_points == snap_points {
                        clamp_snap_index(ctx.current_snap, &snap_points)
                    } else {
                        clamp_snap_index(default_snap_index, &snap_points)
                    };
                    let snap_height = snap_points.get(current_snap).copied().unwrap_or(1.0);

                    ctx.modal = modal;
                    ctx.placement = placement;
                    ctx.resolved_placement = resolved_placement;
                    ctx.dir = dir;
                    ctx.close_on_backdrop = close_on_backdrop;
                    ctx.close_on_escape = close_on_escape;
                    ctx.prevent_scroll = prevent_scroll;
                    ctx.restore_focus = restore_focus;
                    ctx.initial_focus = initial_focus;
                    ctx.final_focus = final_focus;
                    ctx.snap_points = snap_points.clone();
                    ctx.current_snap = current_snap;
                    ctx.snap_height = snap_height;
                });

                if ctx.open {
                    match (ctx.modal, modal) {
                        (true, false) => {
                            plan = plan
                                .with_effect(PendingEffect::named(Effect::RemoveBackgroundInert));
                        }
                        (false, true) => {
                            plan =
                                plan.with_effect(PendingEffect::named(Effect::SetBackgroundInert));
                        }
                        _ => {}
                    }

                    match (ctx.prevent_scroll, prevent_scroll) {
                        (true, false) => {
                            plan =
                                plan.with_effect(PendingEffect::named(Effect::ScrollLockRelease));
                        }
                        (false, true) => {
                            plan =
                                plan.with_effect(PendingEffect::named(Effect::ScrollLockAcquire));
                        }
                        _ => {}
                    }
                }

                Some(plan)
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
        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        let mut events = Vec::new();
        let context_changed = context_relevant_props_changed(old, new);

        if let (old_open, Some(new_open)) = (old.open, new.open)
            && old_open != Some(new_open)
        {
            if new_open {
                if context_changed {
                    events.push(Event::SyncProps);
                }
                events.push(Event::Open);
            } else {
                events.push(Event::Close);
                if context_changed {
                    events.push(Event::SyncProps);
                }
            }
        } else if context_changed {
            events.push(Event::SyncProps);
        }

        events
    }

    fn initial_effects(
        state: &Self::State,
        context: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        if !matches!(state, State::Open | State::Dragging(_)) {
            return Vec::new();
        }

        let mut effects = vec![
            PendingEffect::named(Effect::OpenChange),
            PendingEffect::named(Effect::FocusInitial),
            PendingEffect::named(Effect::FocusFirstTabbable),
            PendingEffect::named(Effect::AllocateZIndex),
        ];

        if context.prevent_scroll {
            effects.push(PendingEffect::named(Effect::ScrollLockAcquire));
        }

        if context.modal {
            effects.push(PendingEffect::named(Effect::SetBackgroundInert));
        }

        effects
    }
}

fn open_plan(ctx: &Context) -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Open)
        .apply(|ctx: &mut Context| {
            ctx.open = true;
        })
        .with_effect(PendingEffect::named(Effect::OpenChange))
        .with_effect(PendingEffect::named(Effect::FocusInitial))
        .with_effect(PendingEffect::named(Effect::FocusFirstTabbable))
        .with_effect(PendingEffect::named(Effect::AllocateZIndex));

    if ctx.prevent_scroll {
        plan = plan.with_effect(PendingEffect::named(Effect::ScrollLockAcquire));
    }

    if ctx.modal {
        plan = plan.with_effect(PendingEffect::named(Effect::SetBackgroundInert));
    }

    plan
}

fn close_plan(ctx: &Context) -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Closed)
        .apply(|ctx: &mut Context| {
            ctx.open = false;
            ctx.z_index = None;
        })
        .with_effect(PendingEffect::named(Effect::OpenChange))
        .with_effect(PendingEffect::named(Effect::ReleaseZIndex));

    if ctx.prevent_scroll {
        plan = plan.with_effect(PendingEffect::named(Effect::ScrollLockRelease));
    }

    if ctx.modal {
        plan = plan.with_effect(PendingEffect::named(Effect::RemoveBackgroundInert));
    }

    if ctx.restore_focus {
        plan = plan.with_effect(PendingEffect::named(Effect::RestoreFocus));
    }

    plan
}

fn resolve_drag_end_plan(ctx: &Context, offset: f64, velocity: f64) -> TransitionPlan<Machine> {
    let offset = clamp_fraction(offset);

    if offset >= DISMISS_THRESHOLD {
        return close_plan(ctx);
    }

    if !snap_points_active(ctx) {
        return TransitionPlan::to(State::Open).apply(|ctx: &mut Context| {
            ctx.open = true;
        });
    }

    let current_height = dismiss_offset_to_snap_height(offset);
    let target = resolve_snap_index(ctx, current_height, velocity);
    let height = ctx.snap_points[target];
    let snap_changed = target != ctx.current_snap || height != ctx.snap_height;

    let mut plan = TransitionPlan::to(State::Open).apply(move |ctx: &mut Context| {
        ctx.open = true;
        ctx.current_snap = target;
        ctx.snap_height = height;
    });

    if snap_changed {
        plan = plan.with_effect(PendingEffect::named(Effect::SnapChange));
    }

    plan
}

fn context_relevant_props_changed(old: &Props, new: &Props) -> bool {
    old.placement != new.placement
        || old.dir != new.dir
        || old.modal != new.modal
        || old.close_on_backdrop != new.close_on_backdrop
        || old.close_on_escape != new.close_on_escape
        || old.prevent_scroll != new.prevent_scroll
        || old.restore_focus != new.restore_focus
        || old.initial_focus != new.initial_focus
        || old.final_focus != new.final_focus
        || old.snap_points != new.snap_points
        || old.default_snap_index != new.default_snap_index
}

fn normalize_snap_points(points: Option<&[f64]>) -> Vec<f64> {
    let mut points = points
        .unwrap_or(&[])
        .iter()
        .copied()
        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
        .collect::<Vec<_>>();

    points.sort_by(f64::total_cmp);
    points.dedup_by(|a, b| (*a - *b).abs() <= f64::EPSILON);

    points
}

fn clamp_snap_index(index: usize, snap_points: &[f64]) -> usize {
    if snap_points.is_empty() {
        0
    } else {
        index.min(snap_points.len() - 1)
    }
}

fn resolve_snap_index(ctx: &Context, current_height: f64, velocity: f64) -> usize {
    if velocity.abs() > VELOCITY_THRESHOLD {
        if velocity < 0.0 {
            return (ctx.current_snap + 1).min(ctx.snap_points.len() - 1);
        }

        return ctx.current_snap.saturating_sub(1);
    }

    nearest_snap_index(&ctx.snap_points, current_height)
}

fn nearest_snap_index(snap_points: &[f64], current_height: f64) -> usize {
    let current_height = clamp_fraction(current_height);

    snap_points
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (*a - current_height)
                .abs()
                .total_cmp(&(*b - current_height).abs())
        })
        .map_or(0, |(index, _)| index)
}

fn snap_points_active(ctx: &Context) -> bool {
    ctx.placement == Placement::Bottom && !ctx.snap_points.is_empty()
}

fn dismiss_offset_to_snap_height(offset: f64) -> f64 {
    1.0 - clamp_fraction(offset)
}

const fn clamp_fraction(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Connected API surface for the [`Drawer`](self) component.
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
    /// Returns `true` when the drawer is open or dragging.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self.state, State::Open | State::Dragging(_))
    }

    /// Returns `true` when the drawer is modal.
    #[must_use]
    pub const fn is_modal(&self) -> bool {
        self.ctx.modal
    }

    /// Returns the resolved physical placement.
    #[must_use]
    pub const fn resolved_placement(&self) -> ResolvedPlacement {
        self.ctx.resolved_placement
    }

    /// Returns the active snap-point index.
    #[must_use]
    pub const fn current_snap(&self) -> usize {
        self.ctx.current_snap
    }

    /// Returns the active snap-point fraction.
    #[must_use]
    pub const fn snap_height(&self) -> f64 {
        self.ctx.snap_height
    }

    /// Returns the value of [`Props::lazy_mount`].
    #[must_use]
    pub const fn lazy_mount(&self) -> bool {
        self.props.lazy_mount
    }

    /// Returns the value of [`Props::unmount_on_exit`].
    #[must_use]
    pub const fn unmount_on_exit(&self) -> bool {
        self.props.unmount_on_exit
    }

    const fn state_token(&self) -> &'static str {
        match self.state {
            State::Closed => "closed",
            State::Open | State::Dragging(_) => "open",
        }
    }

    /// Attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token());

        attrs
    }

    /// Attributes for the trigger button.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("trigger"))
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.is_open() { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.part("content"),
            );

        attrs
    }

    /// Adapter handler: the trigger was clicked.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// Attributes for the backdrop element.
    #[must_use]
    pub fn backdrop_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Backdrop.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if !self.ctx.close_on_backdrop {
            attrs.set(HtmlAttr::Inert, "");
        }

        attrs
    }

    /// Adapter handler: the backdrop was clicked.
    pub fn on_backdrop_click(&self) {
        (self.send)(Event::CloseOnBackdropClick);
    }

    /// Attributes for the positioner wrapper.
    #[must_use]
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(
                HtmlAttr::Data("ars-placement"),
                self.ctx.resolved_placement.as_data_attr(),
            )
            .set_style(
                CssProperty::Custom("ars-drawer-closed-transform"),
                self.ctx.resolved_placement.as_css_translate(),
            );

        if let Some(z_index) = self.ctx.z_index {
            attrs.set_style(CssProperty::Custom("ars-z-index"), z_index.to_string());
        }

        attrs
    }

    /// Attributes for the content element.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("content"))
            .set(HtmlAttr::Role, "dialog")
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(
                HtmlAttr::Data("ars-placement"),
                self.ctx.resolved_placement.as_data_attr(),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::RoleDescription),
                (self.ctx.messages.role_description)(&self.ctx.locale),
            )
            .set(HtmlAttr::TabIndex, "-1");

        if self.ctx.modal {
            attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "true");
        }

        if self.ctx.has_title {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("title"),
            );
        }

        if self.ctx.has_description {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                self.ctx.ids.part("description"),
            );
        }

        if matches!(self.state, State::Dragging(_)) {
            attrs.set(HtmlAttr::Data("ars-dragging"), "");
        }

        if snap_points_active(self.ctx) {
            attrs
                .set(HtmlAttr::Class, "ars-touch-none")
                .set_style(CssProperty::OverscrollBehavior, "contain");
        }

        attrs
    }

    /// Adapter handler: a key was pressed on content.
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Escape => (self.send)(Event::CloseOnEscape),

            KeyboardKey::ArrowUp | KeyboardKey::PageUp if snap_points_active(self.ctx) => {
                (self.send)(Event::SnapTo(self.ctx.current_snap.saturating_add(1)));
            }

            KeyboardKey::ArrowDown | KeyboardKey::PageDown if snap_points_active(self.ctx) => {
                (self.send)(Event::SnapTo(self.ctx.current_snap.saturating_sub(1)));
            }

            KeyboardKey::Home if snap_points_active(self.ctx) => {
                (self.send)(Event::SnapTo(0));
            }

            KeyboardKey::End if snap_points_active(self.ctx) => {
                (self.send)(Event::SnapTo(self.ctx.snap_points.len() - 1));
            }

            _ => {}
        }
    }

    /// Attributes for the title element.
    #[must_use]
    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("title"))
            .set(
                HtmlAttr::Data("ars-heading-level"),
                self.props.title_level.clamp(1, 6).to_string(),
            );

        attrs
    }

    /// Attributes for the description element.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("description"));

        attrs
    }

    /// Attributes for the header region.
    #[must_use]
    pub fn header_attrs(&self) -> AttrMap {
        part_only_attrs(&Part::Header)
    }

    /// Attributes for the body region.
    #[must_use]
    pub fn body_attrs(&self) -> AttrMap {
        part_only_attrs(&Part::Body)
    }

    /// Attributes for the footer region.
    #[must_use]
    pub fn footer_attrs(&self) -> AttrMap {
        part_only_attrs(&Part::Footer)
    }

    /// Attributes for the close trigger.
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

    /// Adapter handler: the close trigger was clicked.
    pub fn on_close_trigger_click(&self) {
        (self.send)(Event::Close);
    }

    /// Attributes for the drag handle.
    #[must_use]
    pub fn drag_handle_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DragHandle.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if snap_points_active(self.ctx) {
            let value = self.ctx.snap_height;

            attrs
                .set(HtmlAttr::Role, "slider")
                .set(HtmlAttr::Class, "ars-touch-none")
                .set(HtmlAttr::TabIndex, "0")
                .set(
                    HtmlAttr::Aria(AriaAttr::Label),
                    (self.ctx.messages.drag_handle_label)(&self.ctx.locale),
                )
                .set(HtmlAttr::Aria(AriaAttr::Orientation), "vertical")
                .set(HtmlAttr::Aria(AriaAttr::ValueMin), "0")
                .set(
                    HtmlAttr::Aria(AriaAttr::ValueMax),
                    (self.ctx.snap_points.len() - 1).to_string(),
                )
                .set(
                    HtmlAttr::Aria(AriaAttr::ValueNow),
                    self.ctx.current_snap.to_string(),
                )
                .set(
                    HtmlAttr::Aria(AriaAttr::ValueText),
                    (self.ctx.messages.snap_value_text)(value, &self.ctx.locale),
                );
        }

        attrs
    }

    /// Adapter handler: the drag handle received a keydown event.
    pub fn on_drag_handle_keydown(&self, data: &KeyboardEventData) {
        self.on_content_keydown(data);
    }
}

fn part_only_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
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

#[cfg(test)]
mod tests {
    use alloc::{rc::Rc, string::ToString, vec};
    use core::cell::RefCell;
    use std::sync::{Arc, Mutex};

    use ars_core::{
        AriaAttr, AttrMap, CssProperty, Direction, Env, HtmlAttr, Machine as MachineTrait,
        MessageFn, PendingEffect, ResolvedDirection, SendResult, Service,
    };
    use ars_interactions::{KeyboardEventData, KeyboardKey};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "drawer".to_string(),
            ..Props::default()
        }
    }

    fn fresh_service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn open_service(props: Props) -> Service<Machine> {
        let mut props = props;

        props.default_open = true;

        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn effect_names(result: &SendResult<Machine>) -> Vec<Effect> {
        result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect()
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

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn recorder() -> (Rc<RefCell<Vec<Event>>>, impl Fn(Event)) {
        let sent = Rc::new(RefCell::new(Vec::new()));

        let handler = {
            let sent = Rc::clone(&sent);
            move |event| sent.borrow_mut().push(event)
        };

        (sent, handler)
    }

    fn assert_syncs_when_only(mut mutate: impl FnMut(&mut Props)) {
        let old = test_props();
        let mut new = old.clone();

        mutate(&mut new);

        let events = <Machine as MachineTrait>::on_props_changed(&old, &new);

        assert_eq!(events, vec![Event::SyncProps]);
    }

    #[test]
    fn init_default_open_false_starts_closed() {
        let service = fresh_service(test_props());

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(service.context().ids.part("trigger"), "drawer-trigger");
        assert_eq!(service.context().ids.part("content"), "drawer-content");
        assert_eq!(service.context().ids.part("title"), "drawer-title");
        assert_eq!(
            service.context().ids.part("description"),
            "drawer-description"
        );
    }

    #[test]
    fn init_default_open_true_starts_open() {
        let service = fresh_service(Props {
            default_open: true,
            ..test_props()
        });

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
    }

    #[test]
    fn placement_start_resolves_left_in_ltr_and_right_in_rtl() {
        assert_eq!(
            Placement::Start.to_physical(ResolvedDirection::Ltr),
            ResolvedPlacement::Left
        );
        assert_eq!(
            Placement::Start.to_physical(ResolvedDirection::Rtl),
            ResolvedPlacement::Right
        );
    }

    #[test]
    fn placement_and_resolved_placement_tokens_cover_all_sides() {
        assert_eq!(Placement::Top.as_css_translate(), "translateY(-100%)");
        assert_eq!(Placement::Bottom.as_css_translate(), "translateY(100%)");
        assert_eq!(Placement::Left.as_css_translate(), "translateX(-100%)");
        assert_eq!(Placement::Right.as_css_translate(), "translateX(100%)");
        assert_eq!(Placement::Start.as_css_translate(), "translateX(100%)");
        assert_eq!(Placement::End.as_css_translate(), "translateX(100%)");

        assert_eq!(Placement::Top.as_data_attr(), "top");
        assert_eq!(Placement::Bottom.as_data_attr(), "bottom");
        assert_eq!(Placement::Left.as_data_attr(), "left");
        assert_eq!(Placement::Right.as_data_attr(), "right");
        assert_eq!(Placement::Start.as_data_attr(), "start");
        assert_eq!(Placement::End.as_data_attr(), "end");

        assert_eq!(
            ResolvedPlacement::Top.as_css_translate(),
            "translateY(-100%)"
        );
        assert_eq!(
            ResolvedPlacement::Bottom.as_css_translate(),
            "translateY(100%)"
        );
        assert_eq!(
            ResolvedPlacement::Left.as_css_translate(),
            "translateX(-100%)"
        );
        assert_eq!(
            ResolvedPlacement::Right.as_css_translate(),
            "translateX(100%)"
        );
        assert_eq!(ResolvedPlacement::Top.as_data_attr(), "top");
        assert_eq!(ResolvedPlacement::Bottom.as_data_attr(), "bottom");
        assert_eq!(ResolvedPlacement::Left.as_data_attr(), "left");
        assert_eq!(ResolvedPlacement::Right.as_data_attr(), "right");
    }

    #[test]
    fn placement_prop_controls_data_ars_placement_and_translate() {
        let ltr = fresh_service(Props {
            placement: Placement::Start,
            dir: Direction::Ltr,
            ..test_props()
        });

        let rtl = fresh_service(Props {
            placement: Placement::Start,
            dir: Direction::Rtl,
            ..test_props()
        });

        let ltr_attrs = ltr.connect(&|_| {}).positioner_attrs();
        let rtl_attrs = rtl.connect(&|_| {}).positioner_attrs();

        assert_eq!(
            ltr_attrs.get(&HtmlAttr::Data("ars-placement")),
            Some("left")
        );
        assert_eq!(
            rtl_attrs.get(&HtmlAttr::Data("ars-placement")),
            Some("right")
        );
        assert!(ltr_attrs.styles().contains(&(
            CssProperty::Custom("ars-drawer-closed-transform"),
            "translateX(-100%)".to_string(),
        )));
        assert!(rtl_attrs.styles().contains(&(
            CssProperty::Custom("ars-drawer-closed-transform"),
            "translateX(100%)".to_string(),
        )));
    }

    #[test]
    fn direction_auto_resolves_logical_placement_from_env_locale_on_init_and_sync() {
        let env = Env {
            locale: ars_i18n::locales::ar(),
            ..Env::default()
        };
        let mut service = Service::<Machine>::new(
            Props {
                placement: Placement::Start,
                dir: Direction::Auto,
                ..test_props()
            },
            &env,
            &Messages::default(),
        );

        assert_eq!(
            service.context().resolved_placement,
            ResolvedPlacement::Right
        );

        drop(service.set_props(Props {
            placement: Placement::End,
            dir: Direction::Auto,
            ..test_props()
        }));

        assert_eq!(
            service.context().resolved_placement,
            ResolvedPlacement::Left
        );
    }

    #[test]
    fn props_builder_round_trips_mutation_sensitive_fields() {
        let open_changes = Arc::new(Mutex::new(Vec::new()));
        let escape_count = Arc::new(Mutex::new(0usize));
        let outside_count = Arc::new(Mutex::new(0usize));

        let open_changes_for_cb = Arc::clone(&open_changes);
        let escape_count_for_cb = Arc::clone(&escape_count);
        let outside_count_for_cb = Arc::clone(&outside_count);

        let props = Props::new()
            .id("settings-drawer")
            .snap_points(Some(vec![0.25, 0.5, 1.0]))
            .lazy_mount(true)
            .unmount_on_exit(true)
            .on_open_change(move |open| open_changes_for_cb.lock().unwrap().push(open))
            .on_escape_key_down(move |_| *escape_count_for_cb.lock().unwrap() += 1)
            .on_interact_outside(move |_| *outside_count_for_cb.lock().unwrap() += 1);

        assert_eq!(props.id, "settings-drawer");
        assert_eq!(props.snap_points.as_deref(), Some(&[0.25, 0.5, 1.0][..]));
        assert!(props.lazy_mount);
        assert!(props.unmount_on_exit);

        props.on_open_change.as_ref().unwrap()(true);
        props.on_escape_key_down.as_ref().unwrap()(DismissAttempt::default());
        props.on_interact_outside.as_ref().unwrap()(DismissAttempt::default());

        assert_eq!(*open_changes.lock().unwrap(), vec![true]);
        assert_eq!(*escape_count.lock().unwrap(), 1);
        assert_eq!(*outside_count.lock().unwrap(), 1);
    }

    #[test]
    fn content_attrs_emit_role_dialog() {
        let service = fresh_service(test_props());

        let attrs = service.connect(&|_| {}).content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("dialog"));
    }

    #[test]
    fn content_attrs_emit_modal_role_description_labelledby_and_describedby_when_registered() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::RegisterTitle));
        drop(service.send(Event::RegisterDescription));

        let attrs = service.connect(&|_| {}).content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Modal)), Some("true"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::RoleDescription)),
            Some("drawer")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("drawer-title")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("drawer-description")
        );
    }

    #[test]
    fn focus_trap_is_adapter_intent_not_dom_lookup() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Open);

        let names = effect_names(&result);

        assert!(names.contains(&Effect::FocusInitial));
        assert!(names.contains(&Effect::FocusFirstTabbable));
        assert_eq!(service.context().ids.part("content"), "drawer-content");
    }

    #[test]
    fn backdrop_attrs_are_decorative_and_inert() {
        let service = fresh_service(Props {
            close_on_backdrop: false,
            ..test_props()
        });

        let attrs = service.connect(&|_| {}).backdrop_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Inert), Some(""));
    }

    #[test]
    fn backdrop_attrs_leave_click_enabled_backdrop_interactive() {
        let service = fresh_service(Props {
            close_on_backdrop: true,
            ..test_props()
        });

        let attrs = service.connect(&|_| {}).backdrop_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Inert), None);
    }

    #[test]
    fn backdrop_and_escape_close_guards_allow_and_block_transitions() {
        let mut backdrop_enabled = open_service(Props {
            close_on_backdrop: true,
            ..test_props()
        });

        assert!(
            backdrop_enabled
                .send(Event::CloseOnBackdropClick)
                .state_changed
        );
        assert_eq!(backdrop_enabled.state(), &State::Closed);

        let mut backdrop_disabled = open_service(Props {
            close_on_backdrop: false,
            ..test_props()
        });

        assert!(
            !backdrop_disabled
                .send(Event::CloseOnBackdropClick)
                .state_changed
        );
        assert_eq!(backdrop_disabled.state(), &State::Open);

        let mut escape_enabled = open_service(Props {
            close_on_escape: true,
            ..test_props()
        });

        assert!(escape_enabled.send(Event::CloseOnEscape).state_changed);
        assert_eq!(escape_enabled.state(), &State::Closed);

        let mut escape_disabled = open_service(Props {
            close_on_escape: false,
            ..test_props()
        });

        assert!(!escape_disabled.send(Event::CloseOnEscape).state_changed);
        assert_eq!(escape_disabled.state(), &State::Open);
    }

    #[test]
    fn title_and_description_registration_are_idempotent() {
        let mut service = fresh_service(test_props());

        let first_title = service.send(Event::RegisterTitle);
        let second_title = service.send(Event::RegisterTitle);
        let first_description = service.send(Event::RegisterDescription);
        let second_description = service.send(Event::RegisterDescription);

        assert!(first_title.context_changed);
        assert!(!second_title.context_changed);
        assert!(first_description.context_changed);
        assert!(!second_description.context_changed);
        assert!(service.context().has_title);
        assert!(service.context().has_description);
    }

    #[test]
    fn title_and_description_unregistration_clear_aria_idrefs() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::RegisterTitle));
        drop(service.send(Event::RegisterDescription));

        let title = service.send(Event::UnregisterTitle);
        let second_title = service.send(Event::UnregisterTitle);
        let description = service.send(Event::UnregisterDescription);
        let second_description = service.send(Event::UnregisterDescription);
        let attrs = service.connect(&|_| {}).content_attrs();

        assert!(title.context_changed);
        assert!(!second_title.context_changed);
        assert!(description.context_changed);
        assert!(!second_description.context_changed);
        assert!(!service.context().has_title);
        assert!(!service.context().has_description);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)), None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)), None);
    }

    #[test]
    fn snap_to_updates_current_snap_and_snap_height() {
        let mut service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            ..test_props()
        });

        let result = service.send(Event::SnapTo(2));

        assert!(result.context_changed);
        assert_eq!(service.context().current_snap, 2);
        assert_eq!(service.context().snap_height, 1.0);
        assert!(effect_names(&result).contains(&Effect::SnapChange));
    }

    #[test]
    fn snap_to_is_inactive_for_non_bottom_drawers() {
        let mut service = open_service(Props {
            placement: Placement::Right,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        let result = service.send(Event::SnapTo(2));

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert_eq!(service.context().current_snap, 1);
        assert_eq!(service.context().snap_height, 0.5);
        assert!(!effect_names(&result).contains(&Effect::SnapChange));
    }

    #[test]
    fn snap_to_same_index_does_not_emit_snap_change() {
        let mut service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        let result = service.send(Event::SnapTo(1));

        assert_eq!(service.context().current_snap, 1);
        assert_eq!(service.context().snap_height, 0.5);
        assert!(!effect_names(&result).contains(&Effect::SnapChange));
    }

    #[test]
    fn snap_to_without_snap_points_does_not_emit_snap_change() {
        let mut service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: None,
            ..test_props()
        });

        let result = service.send(Event::SnapTo(99));

        assert_eq!(service.context().current_snap, 0);
        assert_eq!(service.context().snap_height, 1.0);
        assert!(!effect_names(&result).contains(&Effect::SnapChange));
    }

    #[test]
    fn snap_to_emits_snap_change_when_index_or_height_changes_independently() {
        let service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });
        let props = service.props().clone();

        let mut same_index_changed_height = service.context().clone();
        same_index_changed_height.current_snap = 1;
        same_index_changed_height.snap_height = 0.25;
        let plan = <Machine as MachineTrait>::transition(
            &State::Open,
            &Event::SnapTo(1),
            &same_index_changed_height,
            &props,
        )
        .expect("SnapTo should produce a transition plan");
        assert!(
            plan.effects
                .iter()
                .any(|effect| effect.name == Effect::SnapChange)
        );

        let mut changed_index_same_height = service.context().clone();
        changed_index_same_height.current_snap = 0;
        changed_index_same_height.snap_height = 0.5;
        let plan = <Machine as MachineTrait>::transition(
            &State::Open,
            &Event::SnapTo(1),
            &changed_index_same_height,
            &props,
        )
        .expect("SnapTo should produce a transition plan");
        assert!(
            plan.effects
                .iter()
                .any(|effect| effect.name == Effect::SnapChange)
        );
    }

    #[test]
    fn snap_points_are_filtered_sorted_deduped_and_default_snap_clamps() {
        let service = fresh_service(Props {
            snap_points: Some(vec![f64::NAN, f64::INFINITY, -0.1, 0.5, 0.25, 0.5, 1.1]),
            default_snap_index: 99,
            ..test_props()
        });

        assert_eq!(service.context().snap_points, vec![0.25, 0.5]);
        assert_eq!(service.context().current_snap, 1);
        assert_eq!(service.context().snap_height, 0.5);
    }

    #[test]
    fn drag_end_snaps_to_nearest_point_with_low_velocity() {
        let mut service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        drop(service.send(Event::DragStart(0.0)));
        drop(service.send(Event::DragMove(0.45)));
        drop(service.send(Event::DragEnd {
            offset: 0.45,
            velocity: 0.0,
        }));

        assert_eq!(service.state(), &State::Open);
        assert_eq!(service.context().current_snap, 1);
        assert_eq!(service.context().snap_height, 0.5);
    }

    #[test]
    fn drag_end_low_velocity_can_snap_to_last_point() {
        let mut service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.1, 0.3, 0.6]),
            default_snap_index: 1,
            ..test_props()
        });

        drop(service.send(Event::DragStart(0.0)));

        let result = service.send(Event::DragEnd {
            offset: 0.05,
            velocity: 0.0,
        });

        assert_eq!(service.context().current_snap, 2);
        assert_eq!(service.context().snap_height, 0.6);
        assert!(effect_names(&result).contains(&Effect::SnapChange));
    }

    #[test]
    fn drag_end_uses_velocity_to_select_next_snap() {
        let mut service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        drop(service.send(Event::DragStart(0.0)));
        drop(service.send(Event::DragEnd {
            offset: 0.5,
            velocity: -0.75,
        }));

        assert_eq!(service.context().current_snap, 2);
        assert_eq!(service.context().snap_height, 1.0);

        drop(service.send(Event::DragStart(0.0)));
        drop(service.send(Event::DragEnd {
            offset: 0.5,
            velocity: 0.75,
        }));

        assert_eq!(service.context().current_snap, 1);
        assert_eq!(service.context().snap_height, 0.5);
    }

    #[test]
    fn drag_end_velocity_threshold_is_strict_and_directional() {
        let mut at_threshold = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.1, 0.3, 0.6]),
            default_snap_index: 1,
            ..test_props()
        });

        drop(at_threshold.send(Event::DragStart(0.0)));
        drop(at_threshold.send(Event::DragEnd {
            offset: 0.4,
            velocity: VELOCITY_THRESHOLD,
        }));

        assert_eq!(at_threshold.context().current_snap, 2);

        let mut above_positive = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.1, 0.3, 0.6]),
            default_snap_index: 1,
            ..test_props()
        });

        drop(above_positive.send(Event::DragStart(0.0)));
        drop(above_positive.send(Event::DragEnd {
            offset: 0.6,
            velocity: VELOCITY_THRESHOLD + 0.01,
        }));

        assert_eq!(above_positive.context().current_snap, 0);

        let mut at_negative_threshold = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.1, 0.3, 0.6]),
            default_snap_index: 1,
            ..test_props()
        });

        drop(at_negative_threshold.send(Event::DragStart(0.0)));
        drop(at_negative_threshold.send(Event::DragEnd {
            offset: 0.4,
            velocity: -VELOCITY_THRESHOLD,
        }));

        assert_eq!(at_negative_threshold.context().current_snap, 2);

        let mut below_negative = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.1, 0.3, 0.6]),
            default_snap_index: 1,
            ..test_props()
        });

        drop(below_negative.send(Event::DragStart(0.0)));
        drop(below_negative.send(Event::DragEnd {
            offset: 0.1,
            velocity: -VELOCITY_THRESHOLD - 0.01,
        }));

        assert_eq!(below_negative.context().current_snap, 2);
    }

    #[test]
    fn drag_end_negative_velocity_at_largest_snap_stays_in_range() {
        let mut service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.1, 0.3, 0.6]),
            default_snap_index: 2,
            ..test_props()
        });

        drop(service.send(Event::DragStart(0.0)));
        drop(service.send(Event::DragEnd {
            offset: 0.3,
            velocity: -VELOCITY_THRESHOLD - 0.01,
        }));

        assert_eq!(service.context().current_snap, 2);
        assert_eq!(service.context().snap_height, 0.6);
    }

    #[test]
    fn drag_end_low_velocity_maps_dismiss_offset_to_visible_snap_height() {
        let mut service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        drop(service.send(Event::DragStart(0.0)));
        drop(service.send(Event::DragEnd {
            offset: 0.1,
            velocity: 0.0,
        }));

        assert_eq!(service.context().current_snap, 2);
        assert_eq!(service.context().snap_height, 1.0);
    }

    #[test]
    fn drag_end_emits_snap_change_when_index_or_height_changes_independently() {
        let service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });
        let props = service.props().clone();

        let mut same_index_changed_height = service.context().clone();
        same_index_changed_height.current_snap = 2;
        same_index_changed_height.snap_height = 0.5;
        let plan = <Machine as MachineTrait>::transition(
            &State::Dragging(0.1),
            &Event::DragEnd {
                offset: 0.1,
                velocity: 0.0,
            },
            &same_index_changed_height,
            &props,
        )
        .expect("DragEnd should produce a transition plan");
        assert!(
            plan.effects
                .iter()
                .any(|effect| effect.name == Effect::SnapChange)
        );

        let mut changed_index_same_height = service.context().clone();
        changed_index_same_height.current_snap = 0;
        changed_index_same_height.snap_height = 1.0;
        let plan = <Machine as MachineTrait>::transition(
            &State::Dragging(0.1),
            &Event::DragEnd {
                offset: 0.1,
                velocity: 0.0,
            },
            &changed_index_same_height,
            &props,
        )
        .expect("DragEnd should produce a transition plan");
        assert!(
            plan.effects
                .iter()
                .any(|effect| effect.name == Effect::SnapChange)
        );
    }

    #[test]
    fn drag_end_closes_when_normalized_offset_reaches_dismiss_threshold() {
        let mut service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            ..test_props()
        });

        drop(service.send(Event::DragStart(0.0)));

        let result = service.send(Event::DragEnd {
            offset: 0.75,
            velocity: 0.0,
        });

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert!(effect_names(&result).contains(&Effect::OpenChange));
    }

    #[test]
    fn content_attrs_emit_data_ars_dragging_while_dragging() {
        let mut service = open_service(test_props());

        drop(service.send(Event::DragStart(0.25)));

        let attrs = service.connect(&|_| {}).content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-dragging")), Some(""));
    }

    #[test]
    fn drag_handle_attrs_emit_slider_semantics_when_snap_points_exist() {
        let service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        let attrs = service.connect(&|_| {}).drag_handle_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("slider"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Drawer snap position")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("vertical")
        );
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("2"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("1"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueText)), Some("50%"));
    }

    #[test]
    fn drag_handle_attrs_do_not_emit_slider_semantics_for_non_bottom_drawers() {
        let service = open_service(Props {
            placement: Placement::Right,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        let attrs = service.connect(&|_| {}).drag_handle_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), None);
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), None);
    }

    #[test]
    fn set_z_index_updates_positioner_style() {
        let mut service = open_service(test_props());

        drop(service.send(Event::SetZIndex(1400)));

        let attrs = service.connect(&|_| {}).positioner_attrs();

        assert!(
            attrs
                .styles()
                .contains(&(CssProperty::Custom("ars-z-index"), "1400".to_string(),))
        );
    }

    #[test]
    fn set_z_index_is_ignored_after_close_releases_claim() {
        let mut service = open_service(test_props());

        drop(service.send(Event::SetZIndex(1400)));
        drop(service.send(Event::Close));
        let late = service.send(Event::SetZIndex(1500));

        assert!(!late.context_changed);
        assert_eq!(service.state(), &State::Closed);
        assert_eq!(service.context().z_index, None);
    }

    #[test]
    fn close_releases_z_index_and_focus_intents() {
        let mut service = open_service(Props {
            restore_focus: true,
            ..test_props()
        });

        drop(service.send(Event::SetZIndex(1400)));

        let result = service.send(Event::Close);
        let names = effect_names(&result);

        assert!(names.contains(&Effect::ReleaseZIndex));
        assert!(names.contains(&Effect::RestoreFocus));
        assert!(service.context().z_index.is_none());
    }

    #[test]
    fn on_props_changed_syncs_context_fields() {
        let old = test_props();
        let new = Props {
            placement: Placement::Start,
            dir: Direction::Rtl,
            modal: false,
            close_on_escape: false,
            snap_points: Some(vec![0.25, 0.5]),
            default_snap_index: 1,
            ..test_props()
        };

        let events = <Machine as MachineTrait>::on_props_changed(&old, &new);

        assert!(events.contains(&Event::SyncProps));
    }

    #[test]
    fn on_props_changed_syncs_context_before_controlled_open_change() {
        let old = Props {
            modal: true,
            prevent_scroll: true,
            open: Some(false),
            ..test_props()
        };
        let new = Props {
            modal: false,
            prevent_scroll: false,
            open: Some(true),
            ..old.clone()
        };

        let events = <Machine as MachineTrait>::on_props_changed(&old, &new);

        assert_eq!(events, vec![Event::SyncProps, Event::Open]);
    }

    #[test]
    fn on_props_changed_closes_before_syncing_context_for_controlled_close() {
        let old = Props {
            modal: false,
            prevent_scroll: false,
            open: Some(true),
            ..test_props()
        };
        let new = Props {
            modal: true,
            prevent_scroll: true,
            open: Some(false),
            ..old.clone()
        };

        let events = <Machine as MachineTrait>::on_props_changed(&old, &new);

        assert_eq!(events, vec![Event::Close, Event::SyncProps]);
    }

    #[test]
    fn controlled_open_uses_synced_context_for_open_effects() {
        let mut service = fresh_service(Props {
            modal: true,
            prevent_scroll: true,
            open: Some(false),
            ..test_props()
        });

        let result = service.set_props(Props {
            modal: false,
            prevent_scroll: false,
            open: Some(true),
            ..test_props()
        });

        let names = effect_names(&result);

        assert_eq!(service.state(), &State::Open);
        assert!(!service.context().modal);
        assert!(!service.context().prevent_scroll);
        assert!(!names.contains(&Effect::SetBackgroundInert));
        assert!(!names.contains(&Effect::ScrollLockAcquire));
        assert!(names.contains(&Effect::OpenChange));
    }

    #[test]
    fn on_props_changed_detects_each_context_relevant_field_independently() {
        assert_syncs_when_only(|props| props.placement = Placement::Left);
        assert_syncs_when_only(|props| props.dir = Direction::Rtl);
        assert_syncs_when_only(|props| props.modal = false);
        assert_syncs_when_only(|props| props.close_on_backdrop = false);
        assert_syncs_when_only(|props| props.close_on_escape = false);
        assert_syncs_when_only(|props| props.prevent_scroll = false);
        assert_syncs_when_only(|props| props.restore_focus = false);
        assert_syncs_when_only(|props| props.initial_focus = Some(FocusTarget::First));
        assert_syncs_when_only(|props| props.final_focus = Some(FocusTarget::Last));
        assert_syncs_when_only(|props| props.snap_points = Some(vec![0.25, 0.5]));
        assert_syncs_when_only(|props| props.default_snap_index = 1);
    }

    #[test]
    fn on_props_changed_ignores_non_context_fields_and_same_controlled_open() {
        let old = Props {
            open: Some(true),
            ..test_props()
        };

        let mut new = old.clone();

        new.title_level = 6;
        new.lazy_mount = true;
        new.unmount_on_exit = true;

        let events = <Machine as MachineTrait>::on_props_changed(&old, &new);

        assert!(events.is_empty());
    }

    #[test]
    fn sync_props_event_updates_context_to_current_props() {
        let mut service = fresh_service(test_props());

        service.set_props(Props {
            placement: Placement::End,
            dir: Direction::Rtl,
            modal: false,
            close_on_backdrop: false,
            close_on_escape: false,
            prevent_scroll: false,
            restore_focus: false,
            initial_focus: Some(FocusTarget::AutofocusMarked),
            final_focus: Some(FocusTarget::PreviouslyActive),
            snap_points: Some(vec![0.25, 0.75]),
            default_snap_index: 1,
            ..test_props()
        });

        let result = service.send(Event::SyncProps);

        assert!(result.context_changed);
        assert_eq!(service.context().placement, Placement::End);
        assert_eq!(
            service.context().resolved_placement,
            ResolvedPlacement::Left
        );
        assert!(!service.context().modal);
        assert!(!service.context().close_on_backdrop);
        assert!(!service.context().close_on_escape);
        assert!(!service.context().prevent_scroll);
        assert!(!service.context().restore_focus);
        assert_eq!(
            service.context().initial_focus,
            Some(FocusTarget::AutofocusMarked)
        );
        assert_eq!(
            service.context().final_focus,
            Some(FocusTarget::PreviouslyActive)
        );
        assert_eq!(service.context().snap_points, vec![0.25, 0.75]);
        assert_eq!(service.context().current_snap, 1);
        assert_eq!(service.context().snap_height, 0.75);
    }

    #[test]
    fn sync_props_preserves_active_snap_for_non_snap_prop_updates() {
        let mut service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        drop(service.send(Event::SnapTo(2)));

        let result = service.set_props(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 0,
            modal: false,
            close_on_escape: false,
            ..test_props()
        });

        assert!(result.context_changed);
        assert_eq!(service.context().current_snap, 2);
        assert_eq!(service.context().snap_height, 1.0);
    }

    #[test]
    fn sync_props_while_open_releases_modal_and_scroll_effects_when_disabled() {
        let mut service = open_service(Props {
            modal: true,
            prevent_scroll: true,
            ..test_props()
        });

        let result = service.set_props(Props {
            modal: false,
            prevent_scroll: false,
            ..test_props()
        });
        let names = effect_names(&result);

        assert!(!service.context().modal);
        assert!(!service.context().prevent_scroll);
        assert!(names.contains(&Effect::RemoveBackgroundInert));
        assert!(names.contains(&Effect::ScrollLockRelease));
        assert!(!names.contains(&Effect::SetBackgroundInert));
        assert!(!names.contains(&Effect::ScrollLockAcquire));
    }

    #[test]
    fn sync_props_while_open_acquires_modal_and_scroll_effects_when_enabled() {
        let mut service = open_service(Props {
            modal: false,
            prevent_scroll: false,
            ..test_props()
        });

        let result = service.set_props(Props {
            modal: true,
            prevent_scroll: true,
            ..test_props()
        });
        let names = effect_names(&result);

        assert!(service.context().modal);
        assert!(service.context().prevent_scroll);
        assert!(names.contains(&Effect::SetBackgroundInert));
        assert!(names.contains(&Effect::ScrollLockAcquire));
        assert!(!names.contains(&Effect::RemoveBackgroundInert));
        assert!(!names.contains(&Effect::ScrollLockRelease));
    }

    #[test]
    fn sync_props_while_closed_does_not_emit_modal_or_scroll_effects() {
        let mut service = fresh_service(Props {
            modal: true,
            prevent_scroll: true,
            ..test_props()
        });

        let result = service.set_props(Props {
            modal: false,
            prevent_scroll: false,
            ..test_props()
        });
        let names = effect_names(&result);

        assert!(!service.context().open);
        assert!(!names.contains(&Effect::RemoveBackgroundInert));
        assert!(!names.contains(&Effect::ScrollLockRelease));
    }

    #[test]
    fn controlled_close_with_context_sync_does_not_emit_acquire_effects() {
        let mut service = open_service(Props {
            modal: false,
            prevent_scroll: false,
            open: Some(true),
            ..test_props()
        });

        let result = service.set_props(Props {
            modal: true,
            prevent_scroll: true,
            open: Some(false),
            ..test_props()
        });
        let names = effect_names(&result);

        assert_eq!(service.state(), &State::Closed);
        assert!(service.context().modal);
        assert!(service.context().prevent_scroll);
        assert!(!names.contains(&Effect::SetBackgroundInert));
        assert!(!names.contains(&Effect::ScrollLockAcquire));
        assert!(names.contains(&Effect::OpenChange));
        assert!(names.contains(&Effect::ReleaseZIndex));
    }

    #[test]
    fn api_accessors_return_current_context_and_props() {
        let service = open_service(Props {
            modal: false,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 2,
            lazy_mount: true,
            unmount_on_exit: true,
            ..test_props()
        });

        let api = service.connect(&|_| {});

        assert!(api.is_open());
        assert!(!api.is_modal());
        assert_eq!(api.resolved_placement(), ResolvedPlacement::Right);
        assert_eq!(api.current_snap(), 2);
        assert_eq!(api.snap_height(), 1.0);
        assert!(api.lazy_mount());
        assert!(api.unmount_on_exit());

        let default_service = open_service(Props {
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        let default_api = default_service.connect(&|_| {});

        assert!(default_api.is_modal());
        assert_eq!(default_api.snap_height(), 0.5);
        assert!(!default_api.lazy_mount());
        assert!(!default_api.unmount_on_exit());
    }

    #[test]
    fn initial_effects_for_default_open_emit_open_lifecycle_intents() {
        let mut service = open_service(test_props());

        let names = service
            .take_initial_effects()
            .into_iter()
            .map(|effect: PendingEffect<Machine>| effect.name)
            .collect::<Vec<_>>();

        assert!(names.contains(&Effect::OpenChange));
        assert!(names.contains(&Effect::FocusInitial));
        assert!(names.contains(&Effect::FocusFirstTabbable));
        assert!(names.contains(&Effect::ScrollLockAcquire));
        assert!(names.contains(&Effect::SetBackgroundInert));
        assert!(names.contains(&Effect::AllocateZIndex));
    }

    #[test]
    fn content_keydown_escape_sends_close_event() {
        let (sent, handler) = recorder();

        let service = open_service(test_props());

        let api = service.connect(&handler);

        api.on_content_keydown(&keyboard_data(KeyboardKey::Escape));

        assert_eq!(*sent.borrow(), vec![Event::CloseOnEscape]);
    }

    #[test]
    fn api_event_handlers_send_expected_events() {
        let (sent, handler) = recorder();

        let service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        let api = service.connect(&handler);

        api.on_trigger_click();
        api.on_backdrop_click();
        api.on_close_trigger_click();
        api.on_drag_handle_keydown(&keyboard_data(KeyboardKey::ArrowUp));
        api.on_content_keydown(&keyboard_data(KeyboardKey::ArrowDown));
        api.on_content_keydown(&keyboard_data(KeyboardKey::Home));
        api.on_content_keydown(&keyboard_data(KeyboardKey::End));

        assert_eq!(
            *sent.borrow(),
            vec![
                Event::Toggle,
                Event::CloseOnBackdropClick,
                Event::Close,
                Event::SnapTo(2),
                Event::SnapTo(0),
                Event::SnapTo(0),
                Event::SnapTo(2),
            ]
        );
    }

    #[test]
    fn content_home_key_without_snap_points_is_noop() {
        let (sent, handler) = recorder();

        let service = open_service(test_props());

        service
            .connect(&handler)
            .on_content_keydown(&keyboard_data(KeyboardKey::Home));

        assert!(sent.borrow().is_empty());
    }

    #[test]
    fn snap_keys_with_snap_points_are_inactive_for_non_bottom_drawers() {
        let (sent, handler) = recorder();

        let service = open_service(Props {
            placement: Placement::Right,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        service
            .connect(&handler)
            .on_content_keydown(&keyboard_data(KeyboardKey::Home));
        service
            .connect(&handler)
            .on_content_keydown(&keyboard_data(KeyboardKey::End));
        service
            .connect(&handler)
            .on_content_keydown(&keyboard_data(KeyboardKey::ArrowUp));

        assert!(sent.borrow().is_empty());
    }

    #[test]
    fn connect_api_part_attrs_matches_inherent_attrs_for_every_part() {
        let service = open_service(Props {
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Trigger), api.trigger_attrs());
        assert_eq!(api.part_attrs(Part::Backdrop), api.backdrop_attrs());
        assert_eq!(api.part_attrs(Part::Positioner), api.positioner_attrs());
        assert_eq!(api.part_attrs(Part::Content), api.content_attrs());
        assert_eq!(api.part_attrs(Part::Title), api.title_attrs());
        assert_eq!(api.part_attrs(Part::Description), api.description_attrs());
        assert_eq!(api.part_attrs(Part::Header), api.header_attrs());
        assert_eq!(api.part_attrs(Part::Body), api.body_attrs());
        assert_eq!(api.part_attrs(Part::Footer), api.footer_attrs());
        assert_eq!(
            api.part_attrs(Part::CloseTrigger),
            api.close_trigger_attrs()
        );
        assert_eq!(api.part_attrs(Part::DragHandle), api.drag_handle_attrs());
    }

    #[test]
    fn snapshot_drawer_root_closed() {
        let service = fresh_service(test_props());

        assert_snapshot!(
            "drawer_root_closed",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_root_open() {
        let service = open_service(test_props());

        assert_snapshot!(
            "drawer_root_open",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_root_dragging() {
        let mut service = open_service(test_props());

        drop(service.send(Event::DragStart(0.2)));

        assert_snapshot!(
            "drawer_root_dragging",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_trigger_closed() {
        let service = fresh_service(test_props());

        assert_snapshot!(
            "drawer_trigger_closed",
            snapshot_attrs(&service.connect(&|_| {}).trigger_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_trigger_open() {
        let service = open_service(test_props());

        assert_snapshot!(
            "drawer_trigger_open",
            snapshot_attrs(&service.connect(&|_| {}).trigger_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_backdrop_closed() {
        let service = fresh_service(test_props());

        assert_snapshot!(
            "drawer_backdrop_closed",
            snapshot_attrs(&service.connect(&|_| {}).backdrop_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_backdrop_open() {
        let service = open_service(test_props());

        assert_snapshot!(
            "drawer_backdrop_open",
            snapshot_attrs(&service.connect(&|_| {}).backdrop_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_positioner_each_physical_placement() {
        for placement in [
            Placement::Top,
            Placement::Bottom,
            Placement::Left,
            Placement::Right,
        ] {
            let service = fresh_service(Props {
                placement,
                ..test_props()
            });

            assert_snapshot!(
                format!("drawer_positioner_{}", placement.as_data_attr()),
                snapshot_attrs(&service.connect(&|_| {}).positioner_attrs())
            );
        }
    }

    #[test]
    fn snapshot_drawer_positioner_with_z_index() {
        let mut service = open_service(test_props());

        drop(service.send(Event::SetZIndex(1500)));

        assert_snapshot!(
            "drawer_positioner_with_z_index",
            snapshot_attrs(&service.connect(&|_| {}).positioner_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_content_modal() {
        let service = open_service(test_props());

        assert_snapshot!(
            "drawer_content_modal",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_content_non_modal() {
        let service = open_service(Props {
            modal: false,
            ..test_props()
        });

        assert_snapshot!(
            "drawer_content_non_modal",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_content_with_title_and_description() {
        let mut service = open_service(test_props());

        drop(service.send(Event::RegisterTitle));
        drop(service.send(Event::RegisterDescription));

        assert_snapshot!(
            "drawer_content_with_title_and_description",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_content_dragging() {
        let mut service = open_service(test_props());

        drop(service.send(Event::DragStart(0.3)));

        assert_snapshot!(
            "drawer_content_dragging",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_title_heading_clamp_low() {
        let service = open_service(Props {
            title_level: 0,
            ..test_props()
        });

        assert_snapshot!(
            "drawer_title_heading_clamp_low",
            snapshot_attrs(&service.connect(&|_| {}).title_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_title_heading_clamp_high() {
        let service = open_service(Props {
            title_level: 9,
            ..test_props()
        });

        assert_snapshot!(
            "drawer_title_heading_clamp_high",
            snapshot_attrs(&service.connect(&|_| {}).title_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_description() {
        let service = open_service(test_props());

        assert_snapshot!(
            "drawer_description",
            snapshot_attrs(&service.connect(&|_| {}).description_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_header_body_footer() {
        let service = open_service(test_props());

        let api = service.connect(&|_| {});

        assert_snapshot!("drawer_header", snapshot_attrs(&api.header_attrs()));
        assert_snapshot!("drawer_body", snapshot_attrs(&api.body_attrs()));
        assert_snapshot!("drawer_footer", snapshot_attrs(&api.footer_attrs()));
    }

    #[test]
    fn snapshot_drawer_close_trigger_default_label() {
        let service = open_service(test_props());

        assert_snapshot!(
            "drawer_close_trigger_default_label",
            snapshot_attrs(&service.connect(&|_| {}).close_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_close_trigger_custom_label() {
        let messages = Messages {
            close_label: MessageFn::static_str("Dismiss panel"),
            ..Messages::default()
        };

        let service = Service::<Machine>::new(test_props(), &Env::default(), &messages);

        assert_snapshot!(
            "drawer_close_trigger_custom_label",
            snapshot_attrs(&service.connect(&|_| {}).close_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_drag_handle_without_snap_points() {
        let service = open_service(test_props());

        assert_snapshot!(
            "drawer_drag_handle_without_snap_points",
            snapshot_attrs(&service.connect(&|_| {}).drag_handle_attrs())
        );
    }

    #[test]
    fn snapshot_drawer_drag_handle_with_snap_points() {
        let service = open_service(Props {
            placement: Placement::Bottom,
            snap_points: Some(vec![0.25, 0.5, 1.0]),
            default_snap_index: 1,
            ..test_props()
        });

        assert_snapshot!(
            "drawer_drag_handle_with_snap_points",
            snapshot_attrs(&service.connect(&|_| {}).drag_handle_attrs())
        );
    }
}
