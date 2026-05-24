//! Tour guided walkthrough overlay machine.
//!
//! The Tour core owns step state, current-step metadata, localized labels,
//! ARIA/data attributes, placement preferences, z-index value storage, and
//! adapter-facing effect intents. It does not look up targets, measure DOM
//! rectangles, scroll elements, move focus, or portal content. Framework
//! adapters resolve live element handles and feed the resulting placement,
//! z-index, and spotlight snapshots back through events.

use alloc::{
    format,
    string::{String, ToString as _},
    vec,
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    CssProperty, Env, HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan, no_cleanup,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

use super::positioning::{Placement, PositioningOptions, PositioningSnapshot};

type LocaleMessage = dyn Fn(&Locale) -> String + Send + Sync;
type ProgressMessage = dyn Fn(usize, usize, &Locale) -> String + Send + Sync;

/// The states of the `Tour` component.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// Tour has not started or has been reset.
    #[default]
    Inactive,

    /// Tour is active and showing a specific step.
    Active {
        /// The index of the current step.
        step_index: usize,
    },

    /// Tour has been completed.
    Completed,
}

/// The presentation type of a tour step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StepType {
    /// Anchored popover-style positioning near the target element.
    #[default]
    Tooltip,

    /// Centered dialog-style presentation.
    Dialog,

    /// Fixed-position floating panel presentation.
    Floating,

    /// Invisible step that waits for an adapter-owned condition.
    Wait,
}

/// The definition of a step in the tour.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    /// CSS selector, element id, or adapter-facing token for the target.
    ///
    /// The core treats this as semantic data and an adapter hint only; it never
    /// resolves the string into a live element.
    pub target: Option<String>,

    /// Step title text.
    pub title: String,

    /// Step description/content text.
    pub content: String,

    /// Presentation type of this step.
    pub step_type: StepType,

    /// Preferred placement of the step content relative to the target.
    pub placement: Placement,

    /// Spotlight border radius in CSS pixels.
    pub spotlight_radius: f64,

    /// Spotlight padding around the target in CSS pixels.
    pub spotlight_offset: f64,
}

impl Default for Step {
    fn default() -> Self {
        Self {
            target: None,
            title: String::new(),
            content: String::new(),
            step_type: StepType::Tooltip,
            placement: Placement::Bottom,
            spotlight_radius: 4.0,
            spotlight_offset: 8.0,
        }
    }
}

/// Adapter-reported rectangle for the spotlight highlight.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpotlightRect {
    /// Left coordinate in CSS pixels.
    pub x: f64,

    /// Top coordinate in CSS pixels.
    pub y: f64,

    /// Width in CSS pixels.
    pub width: f64,

    /// Height in CSS pixels.
    pub height: f64,
}

/// Adapter-reported spotlight measurement.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpotlightSnapshot {
    /// Measured rectangle for the highlighted target.
    pub rect: SpotlightRect,

    /// Padding applied around the target.
    pub offset: f64,

    /// Border radius for the spotlight cutout.
    pub radius: f64,
}

/// Events accepted by the `Tour` state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Start the tour from step 0.
    Start,

    /// Advance to the next step.
    NextStep,

    /// Go back to the previous step.
    PrevStep,

    /// Jump to a specific step by index.
    GoToStep(usize),

    /// User skipped the tour.
    Skip,

    /// Tour completed.
    Complete,

    /// User dismissed the tour.
    Dismiss,

    /// Dynamically add a step at the given index.
    AddStep {
        /// Index at which to insert the step.
        index: usize,

        /// Step to insert.
        step: Step,
    },

    /// Remove the step at the given index.
    RemoveStep(usize),

    /// Replace the step at the given index.
    UpdateStep {
        /// Index of the step to replace.
        index: usize,

        /// Replacement step.
        step: Step,
    },

    /// Callback marker when step changes.
    StepChange(usize),

    /// Focus received on tour content.
    Focus {
        /// Whether focus came from keyboard input.
        is_keyboard: bool,
    },

    /// Focus lost from tour content.
    Blur,

    /// Adapter reported an allocated z-index.
    SetZIndex(u32),

    /// Adapter reported resolved positioning output.
    PositioningUpdate(PositioningSnapshot),

    /// Adapter reported spotlight measurement output.
    SpotlightUpdate(SpotlightSnapshot),

    /// Synchronize context-backed props.
    SyncProps,
}

/// Typed identifier for named effect intents emitted by Tour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter invokes [`Props::on_open_change`].
    OpenChange,

    /// Adapter invokes [`Props::on_step_change`].
    StepChange,

    /// Adapter allocates a z-index.
    AllocateZIndex,

    /// Adapter releases the allocated z-index.
    ReleaseZIndex,

    /// Adapter attaches overlay click handling.
    AttachOverlayClick,

    /// Adapter detaches overlay click handling.
    DetachOverlayClick,

    /// Adapter moves focus to step content.
    FocusStepContent,

    /// Adapter scrolls the current target into view.
    ScrollTargetIntoView,

    /// Adapter positions step content.
    PositionStepContent,

    /// Adapter measures the spotlight target.
    MeasureSpotlight,
}

/// Localizable messages for the Tour component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for the tour root.
    pub label: MessageFn<LocaleMessage>,

    /// Progress text formatter.
    pub progress_text: MessageFn<ProgressMessage>,

    /// Label for the next trigger.
    pub next_label: MessageFn<LocaleMessage>,

    /// Label for the previous trigger.
    pub prev_label: MessageFn<LocaleMessage>,

    /// Label for the skip trigger.
    pub skip_label: MessageFn<LocaleMessage>,

    /// Label for the close trigger.
    pub close_label: MessageFn<LocaleMessage>,

    /// Label for the done trigger.
    pub done_label: MessageFn<LocaleMessage>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Tour"),
            progress_text: MessageFn::new(|current, total, _locale: &Locale| {
                format!("Step {current} of {total}")
            }),
            next_label: MessageFn::static_str("Next"),
            prev_label: MessageFn::static_str("Previous"),
            skip_label: MessageFn::static_str("Skip tour"),
            close_label: MessageFn::static_str("Close"),
            done_label: MessageFn::static_str("Done"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Runtime context for `Tour`.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Active locale used to resolve messages.
    pub locale: Locale,

    /// Step definitions.
    pub steps: Vec<Step>,

    /// Current step index.
    pub current_step: usize,

    /// Total number of steps.
    pub total_steps: usize,

    /// Current target token.
    pub target_element_id: Option<String>,

    /// Whether content is focused.
    pub focused: bool,

    /// Whether focus should be visibly indicated.
    pub focus_visible: bool,

    /// Whether the tour is open.
    pub open: bool,

    /// Component IDs.
    pub ids: ComponentIds,

    /// Positioning preferences for the current step.
    pub positioning: PositioningOptions,

    /// Current resolved placement.
    pub current_placement: Placement,

    /// Latest spotlight measurement.
    pub spotlight: Option<SpotlightSnapshot>,

    /// Adapter allocated z-index.
    pub z_index: Option<u32>,

    /// Localized messages.
    pub messages: Messages,
}

/// Props for the `Tour` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Step definitions for the tour.
    pub steps: Vec<Step>,

    /// Controlled open state.
    pub open: Option<bool>,

    /// Whether the tour opens by default.
    pub default_open: bool,

    /// Whether to automatically start the tour on mount.
    pub auto_start: bool,

    /// Whether clicking the overlay dismisses the tour.
    pub close_on_overlay_click: bool,

    /// Whether Escape dismisses the tour.
    pub close_on_escape: bool,

    /// Whether arrow-key navigation is enabled.
    pub keyboard_navigation: bool,

    /// Callback invoked when open state changes.
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,

    /// Callback invoked when the current step changes.
    pub on_step_change: Option<Callback<dyn Fn(usize) + Send + Sync>>,

    /// Whether content is not mounted until started.
    pub lazy_mount: bool,

    /// Whether content is removed after completing.
    pub unmount_on_exit: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            steps: Vec::new(),
            open: None,
            default_open: false,
            auto_start: false,
            close_on_overlay_click: true,
            close_on_escape: true,
            keyboard_navigation: true,
            on_open_change: None,
            on_step_change: None,
            lazy_mount: false,
            unmount_on_exit: false,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] with default values.
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

    /// Sets [`steps`](Self::steps).
    #[must_use]
    pub fn steps(mut self, steps: Vec<Step>) -> Self {
        self.steps = steps;
        self
    }

    /// Sets [`open`](Self::open).
    #[must_use]
    pub const fn open(mut self, open: Option<bool>) -> Self {
        self.open = open;
        self
    }

    /// Sets [`default_open`](Self::default_open).
    #[must_use]
    pub const fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }

    /// Sets [`auto_start`](Self::auto_start).
    #[must_use]
    pub const fn auto_start(mut self, auto_start: bool) -> Self {
        self.auto_start = auto_start;
        self
    }

    /// Sets [`close_on_overlay_click`](Self::close_on_overlay_click).
    #[must_use]
    pub const fn close_on_overlay_click(mut self, value: bool) -> Self {
        self.close_on_overlay_click = value;
        self
    }

    /// Sets [`close_on_escape`](Self::close_on_escape).
    #[must_use]
    pub const fn close_on_escape(mut self, value: bool) -> Self {
        self.close_on_escape = value;
        self
    }

    /// Sets [`keyboard_navigation`](Self::keyboard_navigation).
    #[must_use]
    pub const fn keyboard_navigation(mut self, value: bool) -> Self {
        self.keyboard_navigation = value;
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

    /// Registers [`on_step_change`](Self::on_step_change).
    #[must_use]
    pub fn on_step_change<F>(mut self, f: F) -> Self
    where
        F: Fn(usize) + Send + Sync + 'static,
    {
        self.on_step_change = Some(Callback::new(f));
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
}

/// Anatomy parts exposed by the `Tour` connect API.
#[derive(ComponentPart)]
#[scope = "tour"]
pub enum Part {
    /// Root container.
    Root,

    /// Visual page overlay.
    Overlay,

    /// Spotlight highlight.
    Highlight,

    /// Step content dialog.
    StepContent,

    /// Step title.
    StepTitle,

    /// Step description.
    StepDescription,

    /// Next or done trigger.
    NextTrigger,

    /// Previous trigger.
    PrevTrigger,

    /// Skip trigger.
    SkipTrigger,

    /// Close trigger.
    CloseTrigger,

    /// Progress text container.
    Progress,

    /// Step indicator dot.
    StepIndicator {
        /// Step index represented by this indicator.
        index: usize,
    },
}

fn current_step_data(steps: &[Step], index: usize) -> (Option<String>, PositioningOptions) {
    let step = steps.get(index);

    let target = step.and_then(|step| step.target.clone());

    let mut positioning = PositioningOptions::default();

    if let Some(step) = step {
        positioning.placement = step.placement;
        positioning.offset.main_axis = step.spotlight_offset;
    }

    (target, positioning)
}

fn open_change_effect(open: bool) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::OpenChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_open_change {
                callback(open);
            }

            no_cleanup()
        },
    )
}

fn step_change_effect(index: usize) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::StepChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_step_change {
                callback(index);
            }

            no_cleanup()
        },
    )
}

fn open_lifecycle_effects(step: &Step) -> Vec<PendingEffect<Machine>> {
    let mut effects = vec![
        PendingEffect::named(Effect::AllocateZIndex),
        PendingEffect::named(Effect::FocusStepContent),
    ];

    if step.target.is_some() && matches!(step.step_type, StepType::Tooltip) {
        effects.extend([
            PendingEffect::named(Effect::ScrollTargetIntoView),
            PendingEffect::named(Effect::PositionStepContent),
            PendingEffect::named(Effect::MeasureSpotlight),
        ]);
    }

    effects.push(PendingEffect::named(Effect::AttachOverlayClick));

    effects
}

fn close_lifecycle_effects() -> [PendingEffect<Machine>; 2] {
    [
        PendingEffect::named(Effect::DetachOverlayClick),
        PendingEffect::named(Effect::ReleaseZIndex),
    ]
}

fn open_plan(step_index: usize, ctx: &Context) -> TransitionPlan<Machine> {
    let (target, positioning) = current_step_data(&ctx.steps, step_index);

    let current_placement = positioning.placement;

    let step = ctx.steps[step_index].clone();

    let mut plan =
        TransitionPlan::to(State::Active { step_index }).apply(move |ctx: &mut Context| {
            ctx.current_step = step_index;
            ctx.target_element_id = target.clone();
            ctx.positioning = positioning.clone();
            ctx.current_placement = current_placement;
            ctx.spotlight = None;
            ctx.z_index = None;
            ctx.open = true;
        });

    plan = plan.with_effect(open_change_effect(true));
    plan = plan.with_effect(step_change_effect(step_index));

    for effect in open_lifecycle_effects(&step) {
        plan = plan.with_effect(effect);
    }

    plan
}

fn step_change_plan(next: usize, ctx: &Context) -> TransitionPlan<Machine> {
    let (target, positioning) = current_step_data(&ctx.steps, next);

    let current_placement = positioning.placement;

    let step = ctx.steps[next].clone();

    let mut plan =
        TransitionPlan::to(State::Active { step_index: next }).apply(move |ctx: &mut Context| {
            ctx.current_step = next;
            ctx.target_element_id = target.clone();
            ctx.positioning = positioning.clone();
            ctx.current_placement = current_placement;
            ctx.spotlight = None;
        });

    plan = plan.with_effect(step_change_effect(next));
    plan = plan.with_effect(PendingEffect::named(Effect::FocusStepContent));

    if step.target.is_some() && matches!(step.step_type, StepType::Tooltip) {
        plan = plan
            .with_effect(PendingEffect::named(Effect::ScrollTargetIntoView))
            .with_effect(PendingEffect::named(Effect::PositionStepContent))
            .with_effect(PendingEffect::named(Effect::MeasureSpotlight));
    }

    plan
}

fn close_to_inactive_plan() -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Inactive).apply(|ctx: &mut Context| {
        ctx.open = false;
        ctx.focused = false;
        ctx.focus_visible = false;
        ctx.spotlight = None;
        ctx.z_index = None;
    });

    plan = plan.with_effect(open_change_effect(false));

    for effect in close_lifecycle_effects() {
        plan = plan.with_effect(effect);
    }

    plan
}

fn complete_plan() -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Completed).apply(|ctx: &mut Context| {
        ctx.open = false;
        ctx.focused = false;
        ctx.focus_visible = false;
        ctx.spotlight = None;
        ctx.z_index = None;
    });

    plan = plan.with_effect(open_change_effect(false));

    for effect in close_lifecycle_effects() {
        plan = plan.with_effect(effect);
    }

    plan
}

/// State machine for the `Tour` component.
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

        let total_steps = props.steps.len();

        let open = props.open.unwrap_or(props.default_open || props.auto_start) && total_steps > 0;

        let state = if open {
            State::Active { step_index: 0 }
        } else {
            State::Inactive
        };

        let (target, positioning) = current_step_data(&props.steps, 0);

        let current_placement = positioning.placement;

        (
            state,
            Context {
                locale: env.locale.clone(),
                steps: props.steps.clone(),
                current_step: 0,
                total_steps,
                target_element_id: target,
                focused: false,
                focus_visible: false,
                open,
                ids,
                positioning,
                current_placement,
                spotlight: None,
                z_index: None,
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
            (State::Inactive | State::Completed, Event::Start) if ctx.total_steps > 0 => {
                Some(open_plan(0, ctx))
            }

            (State::Active { step_index }, Event::NextStep) => {
                let next = step_index + 1;

                if next >= ctx.total_steps {
                    Some(complete_plan())
                } else {
                    Some(step_change_plan(next, ctx))
                }
            }

            (State::Active { step_index }, Event::PrevStep) if *step_index > 0 => {
                Some(step_change_plan(step_index - 1, ctx))
            }

            (State::Active { .. }, Event::GoToStep(index)) if *index < ctx.total_steps => {
                Some(step_change_plan(*index, ctx))
            }

            (State::Active { .. }, Event::Skip | Event::Dismiss) => Some(close_to_inactive_plan()),

            (State::Active { .. }, Event::Complete) => Some(complete_plan()),

            (_, Event::Focus { is_keyboard }) => {
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = is_keyboard;
                }))
            }

            (_, Event::Blur) => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            (State::Active { .. }, Event::AddStep { index, step }) => {
                let index = *index;
                let step = step.clone();
                let new_step = if index <= ctx.current_step {
                    ctx.current_step + 1
                } else {
                    ctx.current_step
                };

                Some(
                    TransitionPlan::to(State::Active {
                        step_index: new_step,
                    })
                    .apply(move |ctx: &mut Context| {
                        let clamped = index.min(ctx.steps.len());

                        ctx.steps.insert(clamped, step.clone());
                        ctx.total_steps = ctx.steps.len();
                        ctx.current_step = new_step;

                        let (target, positioning) = current_step_data(&ctx.steps, ctx.current_step);

                        ctx.target_element_id = target;
                        ctx.current_placement = positioning.placement;
                        ctx.positioning = positioning;
                    }),
                )
            }

            (State::Active { .. }, Event::RemoveStep(index)) if *index < ctx.total_steps => {
                let index = *index;
                let step_index = ctx.current_step;

                if ctx.total_steps <= 1 {
                    Some(close_to_inactive_plan().apply(move |ctx: &mut Context| {
                        ctx.steps.clear();
                        ctx.total_steps = 0;
                        ctx.current_step = 0;
                        ctx.target_element_id = None;
                    }))
                } else {
                    let new_step = if index < step_index {
                        step_index - 1
                    } else if index == step_index {
                        step_index.min(ctx.total_steps - 2)
                    } else {
                        step_index
                    };

                    Some(
                        TransitionPlan::to(State::Active {
                            step_index: new_step,
                        })
                        .apply(move |ctx: &mut Context| {
                            ctx.steps.remove(index);

                            ctx.total_steps = ctx.steps.len();
                            ctx.current_step = new_step;

                            let (target, positioning) = current_step_data(&ctx.steps, new_step);

                            ctx.target_element_id = target;
                            ctx.current_placement = positioning.placement;
                            ctx.positioning = positioning;
                            ctx.spotlight = None;
                        }),
                    )
                }
            }

            (State::Active { .. }, Event::UpdateStep { index, step })
                if *index < ctx.total_steps =>
            {
                let index = *index;
                let step = step.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.steps[index] = step.clone();

                    if index == ctx.current_step {
                        let (target, positioning) = current_step_data(&ctx.steps, index);

                        ctx.target_element_id = target;
                        ctx.current_placement = positioning.placement;
                        ctx.positioning = positioning;
                        ctx.spotlight = None;
                    }
                }))
            }

            (_, Event::SetZIndex(z_index)) => {
                let z_index = *z_index;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.z_index = Some(z_index);
                }))
            }

            (State::Active { .. }, Event::PositioningUpdate(snapshot)) => {
                let snapshot = *snapshot;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.current_placement = snapshot.placement;
                }))
            }

            (State::Active { .. }, Event::SpotlightUpdate(snapshot)) => {
                let snapshot = *snapshot;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.spotlight = Some(snapshot);
                }))
            }

            (State::Active { .. }, Event::SyncProps) if props.steps.is_empty() => Some(
                TransitionPlan::to(State::Inactive).apply(|ctx: &mut Context| {
                    ctx.steps.clear();

                    ctx.total_steps = 0;
                    ctx.current_step = 0;
                    ctx.target_element_id = None;
                    ctx.open = false;
                    ctx.spotlight = None;
                    ctx.z_index = None;
                }),
            ),

            (State::Active { .. }, Event::SyncProps) => {
                let steps = props.steps.clone();
                let new_step = ctx.current_step.min(steps.len().saturating_sub(1));
                Some(
                    TransitionPlan::to(State::Active {
                        step_index: new_step,
                    })
                    .apply(move |ctx: &mut Context| {
                        ctx.steps = steps.clone();
                        ctx.total_steps = ctx.steps.len();
                        ctx.current_step = new_step;

                        let (target, positioning) = current_step_data(&ctx.steps, ctx.current_step);

                        ctx.target_element_id = target;
                        ctx.current_placement = positioning.placement;
                        ctx.positioning = positioning;
                        ctx.spotlight = None;
                    }),
                )
            }

            (_, Event::SyncProps) => {
                let steps = props.steps.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.steps = steps.clone();
                    ctx.total_steps = ctx.steps.len();
                    ctx.current_step = ctx.current_step.min(ctx.total_steps.saturating_sub(1));

                    let (target, positioning) = current_step_data(&ctx.steps, ctx.current_step);

                    ctx.target_element_id = target;
                    ctx.current_placement = positioning.placement;
                    ctx.positioning = positioning;
                    ctx.spotlight = None;
                }))
            }

            (_, Event::StepChange(index)) => {
                Some(TransitionPlan::new().with_effect(step_change_effect(*index)))
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
        assert_eq!(old.id, new.id, "Tour id cannot change after initialization");

        let mut events = Vec::new();

        if old.steps != new.steps
            || old.close_on_overlay_click != new.close_on_overlay_click
            || old.close_on_escape != new.close_on_escape
            || old.keyboard_navigation != new.keyboard_navigation
            || old.lazy_mount != new.lazy_mount
            || old.unmount_on_exit != new.unmount_on_exit
        {
            events.push(Event::SyncProps);
        }

        if let (was, Some(now)) = (old.open, new.open)
            && was != Some(now)
        {
            events.push(if now { Event::Start } else { Event::Dismiss });
        }

        events
    }

    fn initial_effects(
        state: &Self::State,
        context: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        if let State::Active { step_index } = state
            && let Some(step) = context.steps.get(*step_index)
        {
            let mut effects = vec![open_change_effect(true), step_change_effect(*step_index)];

            effects.extend(open_lifecycle_effects(step));

            effects
        } else {
            Vec::new()
        }
    }
}

/// Connected API surface for the `Tour` component.
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
    /// Returns the current step index.
    #[must_use]
    pub const fn current_step(&self) -> usize {
        self.ctx.current_step
    }

    /// Returns the total number of steps.
    #[must_use]
    pub const fn total_steps(&self) -> usize {
        self.ctx.total_steps
    }

    /// Returns whether the current step is the first step.
    #[must_use]
    pub const fn is_first_step(&self) -> bool {
        self.ctx.current_step == 0
    }

    /// Returns whether the current step is the last step.
    #[must_use]
    pub const fn is_last_step(&self) -> bool {
        self.ctx.current_step >= self.ctx.total_steps.saturating_sub(1)
    }

    /// Returns whether the tour is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.ctx.open
    }

    /// Returns the current step definition.
    #[must_use]
    pub fn current_step_def(&self) -> Option<&Step> {
        self.ctx.steps.get(self.ctx.current_step)
    }

    /// Returns progress as a percentage from `0.0` to `100.0`.
    #[must_use]
    pub fn progress_percent(&self) -> f64 {
        if self.ctx.total_steps == 0 {
            0.0
        } else {
            ((self.ctx.current_step + 1) as f64 / self.ctx.total_steps as f64) * 100.0
        }
    }

    /// Returns localized progress text.
    #[must_use]
    pub fn progress_text(&self) -> String {
        (self.ctx.messages.progress_text)(
            self.ctx.current_step + 1,
            self.ctx.total_steps,
            &self.ctx.locale,
        )
    }

    /// Returns whether there is a next step.
    #[must_use]
    pub const fn has_next_step(&self) -> bool {
        self.ctx.current_step + 1 < self.ctx.total_steps
    }

    /// Returns whether there is a previous step.
    #[must_use]
    pub const fn has_prev_step(&self) -> bool {
        self.ctx.current_step > 0
    }

    /// Returns current positioning preferences.
    #[must_use]
    pub const fn positioning(&self) -> &PositioningOptions {
        &self.ctx.positioning
    }

    /// Returns latest adapter-reported spotlight snapshot.
    #[must_use]
    pub const fn spotlight_snapshot(&self) -> Option<SpotlightSnapshot> {
        self.ctx.spotlight
    }

    /// Returns adapter-allocated z-index.
    #[must_use]
    pub const fn z_index(&self) -> Option<u32> {
        self.ctx.z_index
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
            State::Inactive => "inactive",
            State::Active { .. } => "active",
            State::Completed => "completed",
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
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.label)(&self.ctx.locale),
            )
            .set(HtmlAttr::Data("ars-state"), self.state_token());

        attrs
    }

    /// Attributes for the visual overlay.
    #[must_use]
    pub fn overlay_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Overlay.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if let Some(z_index) = self.ctx.z_index {
            attrs.set_style(CssProperty::Custom("ars-z-index"), z_index.to_string());
        }

        attrs
    }

    /// Attributes for the spotlight highlight.
    #[must_use]
    pub fn highlight_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Highlight.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if let Some(target) = &self.ctx.target_element_id {
            attrs.set(HtmlAttr::Data("ars-target"), target.clone());
        }

        if let Some(snapshot) = self.ctx.spotlight {
            attrs
                .set_style(CssProperty::Position, "fixed")
                .set_style(
                    CssProperty::Left,
                    format!("{}px", snapshot.rect.x - snapshot.offset),
                )
                .set_style(
                    CssProperty::Top,
                    format!("{}px", snapshot.rect.y - snapshot.offset),
                )
                .set_style(
                    CssProperty::Width,
                    format!("{}px", snapshot.rect.width + snapshot.offset * 2.0),
                )
                .set_style(
                    CssProperty::Height,
                    format!("{}px", snapshot.rect.height + snapshot.offset * 2.0),
                )
                .set_style(CssProperty::BorderRadius, format!("{}px", snapshot.radius));
        }

        attrs
    }

    /// Attributes for the step content element.
    #[must_use]
    pub fn step_content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StepContent.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("step-content"))
            .set(HtmlAttr::Role, "dialog")
            .set(HtmlAttr::Aria(AriaAttr::Modal), "false")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("step-title"),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                self.ctx.ids.part("step-description"),
            )
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(
                HtmlAttr::Data("ars-placement"),
                self.ctx.current_placement.as_str(),
            );

        if let State::Active { step_index } = self.state {
            attrs.set(HtmlAttr::Data("ars-step"), step_index.to_string());
        }

        if let Some(z_index) = self.ctx.z_index {
            attrs.set_style(CssProperty::Custom("ars-z-index"), z_index.to_string());
        }

        attrs
    }

    /// Attributes for the step title element.
    #[must_use]
    pub fn step_title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StepTitle.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("step-title"));

        attrs
    }

    /// Attributes for the step description element.
    #[must_use]
    pub fn step_description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StepDescription.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("step-description"));

        attrs
    }

    /// Attributes for the next trigger.
    #[must_use]
    pub fn next_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::NextTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button");

        if self.is_last_step() {
            attrs.set_bool(HtmlAttr::Data("ars-last-step"), true).set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.done_label)(&self.ctx.locale),
            );
        } else {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.next_label)(&self.ctx.locale),
            );
        }

        attrs
    }

    /// Attributes for the previous trigger.
    #[must_use]
    pub fn prev_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::PrevTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.prev_label)(&self.ctx.locale),
            );

        if self.is_first_step() {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Attributes for the skip trigger.
    #[must_use]
    pub fn skip_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SkipTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.skip_label)(&self.ctx.locale),
            );

        attrs
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

    /// Attributes for the progress element.
    #[must_use]
    pub fn progress_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Progress.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite");

        attrs
    }

    /// Attributes for a step indicator.
    #[must_use]
    pub fn step_indicator_attrs(&self, step_index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            (Part::StepIndicator { index: step_index }).data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-step"), step_index.to_string());

        if step_index == self.ctx.current_step {
            attrs.set_bool(HtmlAttr::Data("ars-current"), true);
        }

        attrs
    }

    /// Adapter handler: next trigger was activated.
    pub fn on_next_trigger_click(&self) {
        (self.send)(Event::NextStep);
    }

    /// Adapter handler: previous trigger was activated.
    pub fn on_prev_trigger_click(&self) {
        (self.send)(Event::PrevStep);
    }

    /// Adapter handler: skip trigger was activated.
    pub fn on_skip_trigger_click(&self) {
        (self.send)(Event::Skip);
    }

    /// Adapter handler: close trigger was activated.
    pub fn on_close_trigger_click(&self) {
        (self.send)(Event::Dismiss);
    }

    /// Adapter handler: overlay was clicked.
    pub fn on_overlay_click(&self) {
        if self.props.close_on_overlay_click {
            (self.send)(Event::Dismiss);
        }
    }

    /// Adapter handler: content keydown.
    pub fn on_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Escape if self.props.close_on_escape => (self.send)(Event::Dismiss),

            KeyboardKey::ArrowRight if self.props.keyboard_navigation => {
                (self.send)(Event::NextStep);
            }

            KeyboardKey::ArrowLeft if self.props.keyboard_navigation => {
                (self.send)(Event::PrevStep);
            }

            _ => {}
        }
    }

    /// Adapter handler: content received focus.
    pub fn on_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Adapter handler: content lost focus.
    pub fn on_blur(&self) {
        (self.send)(Event::Blur);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Overlay => self.overlay_attrs(),
            Part::Highlight => self.highlight_attrs(),
            Part::StepContent => self.step_content_attrs(),
            Part::StepTitle => self.step_title_attrs(),
            Part::StepDescription => self.step_description_attrs(),
            Part::NextTrigger => self.next_trigger_attrs(),
            Part::PrevTrigger => self.prev_trigger_attrs(),
            Part::SkipTrigger => self.skip_trigger_attrs(),
            Part::CloseTrigger => self.close_trigger_attrs(),
            Part::Progress => self.progress_attrs(),
            Part::StepIndicator { index } => self.step_indicator_attrs(index),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{rc::Rc, string::ToString as _, sync::Arc, vec};
    use core::cell::RefCell;
    use std::sync::Mutex;

    use ars_core::{Env, Machine as _, Service};
    use insta::assert_snapshot;

    use super::*;

    fn step(target: &str, title: &str) -> Step {
        Step {
            target: Some(target.to_string()),
            title: title.to_string(),
            content: format!("{title} body"),
            ..Step::default()
        }
    }

    fn props() -> Props {
        Props {
            id: "tour".to_string(),
            steps: vec![
                step("#one", "One"),
                step("#two", "Two"),
                step("#three", "Three"),
            ],
            ..Props::default()
        }
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn open_service() -> Service<Machine> {
        let mut service = service(props());

        drop(service.send(Event::Start));

        service
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
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

    fn effect_names(result: &ars_core::SendResult<Machine>) -> Vec<Effect> {
        result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    #[test]
    fn initial_inactive_state() {
        let service = service(props());

        assert_eq!(service.state(), &State::Inactive);
        assert!(!service.context().open);
        assert_eq!(service.context().total_steps, 3);
    }

    #[test]
    fn builder_methods_set_identity_and_steps() {
        let custom_steps = vec![step("#custom", "Custom")];

        let props = Props::new()
            .id("custom-tour")
            .steps(custom_steps.clone())
            .lazy_mount(true)
            .unmount_on_exit(true);

        assert_eq!(props.id, "custom-tour");
        assert_eq!(props.steps, custom_steps);
        assert!(props.lazy_mount);
        assert!(props.unmount_on_exit);
    }

    #[test]
    fn start_activates_first_step_when_steps_exist() {
        let mut service = service(props());

        let result = service.send(Event::Start);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Active { step_index: 0 });
        assert!(service.context().open);
        assert_eq!(service.context().target_element_id.as_deref(), Some("#one"));
    }

    #[test]
    fn start_with_empty_steps_does_not_open() {
        let mut service = service(Props {
            id: "tour".to_string(),
            steps: Vec::new(),
            ..Props::default()
        });

        let result = service.send(Event::Start);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Inactive);
    }

    #[test]
    fn next_previous_and_go_to_step_update_current_step() {
        let mut service = open_service();

        drop(service.send(Event::NextStep));

        assert_eq!(service.state(), &State::Active { step_index: 1 });
        assert_eq!(service.context().target_element_id.as_deref(), Some("#two"));

        drop(service.send(Event::PrevStep));

        assert_eq!(service.state(), &State::Active { step_index: 0 });

        let invalid = service.send(Event::PrevStep);

        assert!(!invalid.state_changed);

        drop(service.send(Event::GoToStep(2)));

        assert_eq!(service.state(), &State::Active { step_index: 2 });

        let invalid = service.send(Event::GoToStep(3));

        assert!(!invalid.state_changed);
        assert_eq!(service.state(), &State::Active { step_index: 2 });

        let invalid = service.send(Event::GoToStep(99));

        assert!(!invalid.state_changed);
    }

    #[test]
    fn next_on_last_step_completes_and_closes() {
        let mut service = open_service();

        drop(service.send(Event::GoToStep(2)));

        let result = service.send(Event::NextStep);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Completed);
        assert!(!service.context().open);
    }

    #[test]
    fn skip_and_dismiss_close_to_inactive() {
        for event in [Event::Skip, Event::Dismiss] {
            let mut service = open_service();

            let result = service.send(event);

            assert!(result.state_changed);
            assert_eq!(service.state(), &State::Inactive);
            assert!(!service.context().open);
        }
    }

    #[test]
    fn dynamic_step_management_keeps_context_coherent() {
        let mut service = open_service();

        drop(service.send(Event::GoToStep(1)));
        drop(service.send(Event::AddStep {
            index: 0,
            step: step("#zero", "Zero"),
        }));

        assert_eq!(service.context().current_step, 2);
        assert_eq!(service.context().total_steps, 4);
        assert_eq!(service.context().target_element_id.as_deref(), Some("#two"));

        drop(service.send(Event::UpdateStep {
            index: 2,
            step: step("#two-updated", "Two updated"),
        }));

        assert_eq!(
            service.context().target_element_id.as_deref(),
            Some("#two-updated")
        );

        drop(service.send(Event::RemoveStep(2)));

        assert_eq!(service.context().current_step, 2);
        assert_eq!(service.context().total_steps, 3);
        assert_eq!(
            service.context().target_element_id.as_deref(),
            Some("#three")
        );
    }

    #[test]
    fn dynamic_remove_and_update_guard_boundary_indexes() {
        let mut service = open_service();

        drop(service.send(Event::GoToStep(2)));
        drop(service.send(Event::RemoveStep(0)));

        assert_eq!(service.state(), &State::Active { step_index: 1 });
        assert_eq!(service.context().current_step, 1);
        assert_eq!(
            service.context().target_element_id.as_deref(),
            Some("#three")
        );

        let mut service = open_service();

        drop(service.send(Event::GoToStep(2)));
        drop(service.send(Event::RemoveStep(2)));

        assert_eq!(service.state(), &State::Active { step_index: 1 });
        assert_eq!(service.context().current_step, 1);
        assert_eq!(service.context().target_element_id.as_deref(), Some("#two"));

        let mut service = Service::<Machine>::new(
            Props {
                steps: vec![
                    step("#one", "One"),
                    step("#two", "Two"),
                    step("#three", "Three"),
                    step("#four", "Four"),
                    step("#five", "Five"),
                ],
                ..props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::Start));
        drop(service.send(Event::GoToStep(3)));
        drop(service.send(Event::RemoveStep(3)));

        assert_eq!(service.state(), &State::Active { step_index: 3 });
        assert_eq!(service.context().current_step, 3);
        assert_eq!(
            service.context().target_element_id.as_deref(),
            Some("#five")
        );

        let mut service = open_service();

        let before = service.context().clone();

        let invalid_remove = service.send(Event::RemoveStep(3));

        assert!(!invalid_remove.state_changed);
        assert_eq!(service.context().steps, before.steps);
        assert_eq!(service.context().current_step, before.current_step);

        let invalid_update = service.send(Event::UpdateStep {
            index: 3,
            step: step("#missing", "Missing"),
        });

        assert!(!invalid_update.context_changed);
        assert_eq!(service.context().steps, before.steps);
    }

    #[test]
    fn sync_props_updates_active_empty_and_inactive_contexts() {
        let mut service = open_service();

        drop(service.send(Event::SetZIndex(1400)));
        drop(service.send(Event::SpotlightUpdate(SpotlightSnapshot {
            rect: SpotlightRect {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0,
            },
            offset: 5.0,
            radius: 6.0,
        })));

        service.set_props(Props {
            steps: Vec::new(),
            ..props()
        });

        assert_eq!(service.state(), &State::Inactive);
        assert!(!service.context().open);
        assert_eq!(service.context().total_steps, 0);
        assert_eq!(service.context().target_element_id, None);
        assert_eq!(service.context().z_index, None);
        assert_eq!(service.context().spotlight, None);

        let mut service = open_service();

        drop(service.send(Event::GoToStep(2)));

        service.set_props(Props {
            steps: vec![step("#one-new", "One new"), step("#two-new", "Two new")],
            ..props()
        });

        assert_eq!(service.state(), &State::Active { step_index: 1 });
        assert_eq!(service.context().current_step, 1);
        assert_eq!(
            service.context().target_element_id.as_deref(),
            Some("#two-new")
        );

        let mut inactive_service =
            Service::<Machine>::new(props(), &Env::default(), &Messages::default());

        inactive_service.set_props(Props {
            steps: vec![step("#inactive", "Inactive")],
            ..props()
        });

        assert_eq!(inactive_service.state(), &State::Inactive);
        assert_eq!(inactive_service.context().total_steps, 1);
        assert_eq!(
            inactive_service.context().target_element_id.as_deref(),
            Some("#inactive")
        );
    }

    #[test]
    fn focus_and_blur_update_focus_flags() {
        let mut service = open_service();

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert!(service.context().focused);
        assert!(service.context().focus_visible);

        drop(service.send(Event::Blur));

        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn adapter_feedback_events_store_dom_free_outputs() {
        let mut service = open_service();

        drop(service.send(Event::SetZIndex(1550)));
        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::TopStart,
            arrow: None,
        })));
        drop(service.send(Event::SpotlightUpdate(SpotlightSnapshot {
            rect: SpotlightRect {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 50.0,
            },
            offset: 8.0,
            radius: 6.0,
        })));

        assert_eq!(service.context().z_index, Some(1550));
        assert_eq!(service.context().current_placement, Placement::TopStart);
        assert!(service.context().spotlight.is_some());
    }

    #[test]
    fn explicit_step_change_event_emits_callback_effect() {
        let mut service = service(props());

        let result = service.send(Event::StepChange(2));

        assert!(!result.state_changed);
        assert_eq!(effect_names(&result), [Effect::StepChange]);
    }

    #[test]
    fn callback_effects_run_with_open_and_step_values() {
        let open_values = Arc::new(Mutex::new(Vec::new()));
        let step_values = Arc::new(Mutex::new(Vec::new()));
        let open_for_cb = Arc::clone(&open_values);
        let step_for_cb = Arc::clone(&step_values);

        let mut service = service(
            props()
                .on_open_change(move |open| open_for_cb.lock().expect("open mutex").push(open))
                .on_step_change(move |step| step_for_cb.lock().expect("step mutex").push(step)),
        );

        let start = service.send(Event::Start);
        let send: Arc<dyn Fn(Event) + Send + Sync> = Arc::new(|_: Event| {});
        for effect in start.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        let next = service.send(Event::NextStep);
        let send: Arc<dyn Fn(Event) + Send + Sync> = Arc::new(|_: Event| {});
        for effect in next.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        let complete = service.send(Event::Complete);
        let send: Arc<dyn Fn(Event) + Send + Sync> = Arc::new(|_: Event| {});
        for effect in complete.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(&*open_values.lock().expect("open mutex"), &[true, false]);
        assert_eq!(&*step_values.lock().expect("step mutex"), &[0, 1]);
    }

    #[test]
    fn keyboard_and_click_handlers_dispatch_guarded_events() {
        let captured = Rc::new(RefCell::new(Vec::new()));
        let send = {
            let captured = Rc::clone(&captured);
            move |event| captured.borrow_mut().push(event)
        };

        let service = open_service();

        let api = service.connect(&send);

        api.on_keydown(&keyboard_data(KeyboardKey::Escape));
        api.on_keydown(&keyboard_data(KeyboardKey::ArrowRight));
        api.on_keydown(&keyboard_data(KeyboardKey::ArrowLeft));
        api.on_overlay_click();
        api.on_next_trigger_click();
        api.on_prev_trigger_click();
        api.on_skip_trigger_click();
        api.on_close_trigger_click();
        api.on_focus(true);
        api.on_blur();

        assert_eq!(
            &*captured.borrow(),
            &[
                Event::Dismiss,
                Event::NextStep,
                Event::PrevStep,
                Event::Dismiss,
                Event::NextStep,
                Event::PrevStep,
                Event::Skip,
                Event::Dismiss,
                Event::Focus { is_keyboard: true },
                Event::Blur,
            ],
        );
    }

    #[test]
    fn guarded_handlers_do_not_dispatch_when_disabled() {
        let service = service(Props {
            close_on_overlay_click: false,
            close_on_escape: false,
            keyboard_navigation: false,
            ..props()
        });

        let captured = Rc::new(RefCell::new(Vec::new()));
        let send = {
            let captured = Rc::clone(&captured);
            move |event| captured.borrow_mut().push(event)
        };

        let api = service.connect(&send);

        api.on_keydown(&keyboard_data(KeyboardKey::Escape));
        api.on_keydown(&keyboard_data(KeyboardKey::ArrowRight));
        api.on_keydown(&keyboard_data(KeyboardKey::ArrowLeft));
        api.on_overlay_click();

        assert!(captured.borrow().is_empty());
    }

    #[test]
    fn open_and_close_emit_lifecycle_effects() {
        let mut service = service(props());

        let start = service.send(Event::Start);

        let names = effect_names(&start);

        assert!(names.contains(&Effect::OpenChange));
        assert!(names.contains(&Effect::StepChange));
        assert!(names.contains(&Effect::AllocateZIndex));
        assert!(names.contains(&Effect::AttachOverlayClick));
        assert!(names.contains(&Effect::FocusStepContent));
        assert!(names.contains(&Effect::PositionStepContent));
        assert!(names.contains(&Effect::MeasureSpotlight));

        let close = service.send(Event::Dismiss);

        let names = effect_names(&close);

        assert!(names.contains(&Effect::OpenChange));
        assert!(names.contains(&Effect::DetachOverlayClick));
        assert!(names.contains(&Effect::ReleaseZIndex));
    }

    #[test]
    fn lifecycle_effects_only_request_target_geometry_for_tooltip_steps_with_targets() {
        let mut dialog_service = service(Props {
            steps: vec![Step {
                step_type: StepType::Dialog,
                ..step("#dialog", "Dialog")
            }],
            ..props()
        });

        let names = effect_names(&dialog_service.send(Event::Start));

        assert!(!names.contains(&Effect::ScrollTargetIntoView));
        assert!(!names.contains(&Effect::PositionStepContent));
        assert!(!names.contains(&Effect::MeasureSpotlight));

        let mut no_target_service = service(Props {
            steps: vec![Step {
                target: None,
                ..step("#missing", "Missing")
            }],
            ..props()
        });

        let names = effect_names(&no_target_service.send(Event::Start));

        assert!(!names.contains(&Effect::ScrollTargetIntoView));
        assert!(!names.contains(&Effect::PositionStepContent));
        assert!(!names.contains(&Effect::MeasureSpotlight));

        let mut service = service(Props {
            steps: vec![
                step("#first", "First"),
                Step {
                    step_type: StepType::Dialog,
                    ..step("#second", "Second")
                },
            ],
            ..props()
        });

        drop(service.send(Event::Start));

        let names = effect_names(&service.send(Event::NextStep));

        assert!(names.contains(&Effect::FocusStepContent));
        assert!(!names.contains(&Effect::ScrollTargetIntoView));
        assert!(!names.contains(&Effect::PositionStepContent));
        assert!(!names.contains(&Effect::MeasureSpotlight));
    }

    #[test]
    fn initial_active_state_returns_initial_effects_once() {
        let mut service = service(Props {
            auto_start: true,
            ..props()
        });

        assert_eq!(service.state(), &State::Active { step_index: 0 });

        let names = service
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect::<Vec<_>>();

        assert!(names.contains(&Effect::OpenChange));
        assert!(names.contains(&Effect::StepChange));
        assert!(names.contains(&Effect::AllocateZIndex));
        assert!(service.take_initial_effects().is_empty());
    }

    #[test]
    fn on_props_changed_emits_controlled_and_sync_events() {
        let old = props();

        let open = Props {
            open: Some(true),
            ..props()
        };

        assert_eq!(Machine::on_props_changed(&old, &open), [Event::Start]);

        let changed_steps = Props {
            steps: vec![step("#new", "New")],
            ..props()
        };

        assert_eq!(
            Machine::on_props_changed(&old, &changed_steps),
            [Event::SyncProps]
        );

        let sync_cases = [
            Props {
                close_on_overlay_click: false,
                ..props()
            },
            Props {
                close_on_escape: false,
                ..props()
            },
            Props {
                keyboard_navigation: false,
                ..props()
            },
            Props {
                lazy_mount: true,
                ..props()
            },
            Props {
                unmount_on_exit: true,
                ..props()
            },
        ];

        for new in sync_cases {
            assert_eq!(Machine::on_props_changed(&old, &new), [Event::SyncProps]);
        }
    }

    #[test]
    #[should_panic(expected = "Tour id cannot change after initialization")]
    fn on_props_changed_panics_when_id_changes() {
        let old = props();
        let new = Props {
            id: "different".to_string(),
            ..props()
        };

        drop(Machine::on_props_changed(&old, &new));
    }

    #[test]
    fn connect_api_part_attrs_match_direct_methods() {
        use ars_core::ConnectApi as _;

        let service = open_service();

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Overlay), api.overlay_attrs());
        assert_eq!(api.part_attrs(Part::Highlight), api.highlight_attrs());
        assert_eq!(api.part_attrs(Part::StepContent), api.step_content_attrs());
        assert_eq!(api.part_attrs(Part::StepTitle), api.step_title_attrs());
        assert_eq!(
            api.part_attrs(Part::StepDescription),
            api.step_description_attrs()
        );
        assert_eq!(api.part_attrs(Part::NextTrigger), api.next_trigger_attrs());
        assert_eq!(api.part_attrs(Part::PrevTrigger), api.prev_trigger_attrs());
        assert_eq!(api.part_attrs(Part::SkipTrigger), api.skip_trigger_attrs());
        assert_eq!(
            api.part_attrs(Part::CloseTrigger),
            api.close_trigger_attrs()
        );
        assert_eq!(api.part_attrs(Part::Progress), api.progress_attrs());
        assert_eq!(
            api.part_attrs(Part::StepIndicator { index: 0 }),
            api.step_indicator_attrs(0)
        );
    }

    #[test]
    fn accessors_report_current_tour_state() {
        let default_service = open_service();

        let default_api = default_service.connect(&|_| {});

        assert!(!default_api.has_prev_step());
        assert!(!default_api.lazy_mount());
        assert!(!default_api.unmount_on_exit());

        let mut service = service(Props {
            lazy_mount: true,
            unmount_on_exit: true,
            ..props()
        });

        assert!(!service.connect(&|_| {}).is_open());

        drop(service.send(Event::Start));
        drop(service.send(Event::NextStep));
        drop(service.send(Event::SetZIndex(1550)));
        drop(service.send(Event::SpotlightUpdate(SpotlightSnapshot {
            rect: SpotlightRect {
                x: 10.0,
                y: 20.0,
                width: 30.0,
                height: 40.0,
            },
            offset: 8.0,
            radius: 6.0,
        })));

        let api = service.connect(&|_| {});

        assert_eq!(api.current_step(), 1);
        assert_eq!(api.total_steps(), 3);
        assert!(!api.is_first_step());
        assert!(!api.is_last_step());
        assert!(api.is_open());
        assert!(api.has_next_step());
        assert!(api.has_prev_step());
        assert!((api.progress_percent() - (200.0 / 3.0)).abs() < 1e-10);
        assert_eq!(api.progress_text(), "Step 2 of 3");
        assert_eq!(
            api.current_step_def().map(|step| step.title.as_str()),
            Some("Two")
        );
        assert_eq!(api.spotlight_snapshot(), service.context().spotlight);
        assert_eq!(api.z_index(), Some(1550));
        assert!(api.lazy_mount());
        assert!(api.unmount_on_exit());

        drop(service.send(Event::NextStep));

        let api = service.connect(&|_| {});

        assert_eq!(api.current_step(), 2);
        assert!(!api.has_next_step());
        assert!(api.has_prev_step());
    }

    #[test]
    fn snapshot_root_inactive() {
        let service = service(props());

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).root_attrs()));
    }

    #[test]
    fn snapshot_active_first_step_all_parts() {
        let service = open_service();

        let api = service.connect(&|_| {});

        assert_snapshot!("tour_overlay", snapshot_attrs(&api.overlay_attrs()));
        assert_snapshot!("tour_highlight", snapshot_attrs(&api.highlight_attrs()));
        assert_snapshot!(
            "tour_step_content",
            snapshot_attrs(&api.step_content_attrs())
        );
        assert_snapshot!("tour_step_title", snapshot_attrs(&api.step_title_attrs()));
        assert_snapshot!(
            "tour_step_description",
            snapshot_attrs(&api.step_description_attrs())
        );
        assert_snapshot!(
            "tour_next_trigger",
            snapshot_attrs(&api.next_trigger_attrs())
        );
        assert_snapshot!(
            "tour_prev_trigger",
            snapshot_attrs(&api.prev_trigger_attrs())
        );
        assert_snapshot!(
            "tour_skip_trigger",
            snapshot_attrs(&api.skip_trigger_attrs())
        );
        assert_snapshot!(
            "tour_close_trigger",
            snapshot_attrs(&api.close_trigger_attrs())
        );
        assert_snapshot!("tour_progress", snapshot_attrs(&api.progress_attrs()));
        assert_snapshot!(
            "tour_step_indicator_current",
            snapshot_attrs(&api.step_indicator_attrs(0))
        );
        assert_snapshot!(
            "tour_step_indicator_inactive",
            snapshot_attrs(&api.step_indicator_attrs(1))
        );
    }

    #[test]
    fn snapshot_last_step_done_label() {
        let mut service = open_service();

        drop(service.send(Event::GoToStep(2)));

        assert_snapshot!(snapshot_attrs(
            &service.connect(&|_| {}).next_trigger_attrs()
        ));
    }

    #[test]
    fn snapshot_completed_root() {
        let mut service = open_service();

        drop(service.send(Event::Complete));

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).root_attrs()));
    }

    #[test]
    fn snapshot_custom_messages_and_feedback_attrs() {
        let messages = Messages {
            label: MessageFn::static_str("Tour personalizado"),
            progress_text: MessageFn::new(|current, total, _locale: &Locale| {
                format!("{current}/{total}")
            }),
            next_label: MessageFn::static_str("Siguiente"),
            prev_label: MessageFn::static_str("Anterior"),
            skip_label: MessageFn::static_str("Omitir"),
            close_label: MessageFn::static_str("Cerrar"),
            done_label: MessageFn::static_str("Listo"),
        };

        let mut service = Service::<Machine>::new(props(), &Env::default(), &messages);

        drop(service.send(Event::Start));
        drop(service.send(Event::SetZIndex(1600)));
        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::TopEnd,
            arrow: None,
        })));
        drop(service.send(Event::SpotlightUpdate(SpotlightSnapshot {
            rect: SpotlightRect {
                x: 20.0,
                y: 30.0,
                width: 80.0,
                height: 40.0,
            },
            offset: 6.0,
            radius: 10.0,
        })));

        let api = service.connect(&|_| {});

        assert_snapshot!("tour_custom_root", snapshot_attrs(&api.root_attrs()));
        assert_snapshot!(
            "tour_custom_next",
            snapshot_attrs(&api.next_trigger_attrs())
        );
        assert_snapshot!(
            "tour_feedback_overlay",
            snapshot_attrs(&api.overlay_attrs())
        );
        assert_snapshot!(
            "tour_feedback_content",
            snapshot_attrs(&api.step_content_attrs())
        );
        assert_snapshot!(
            "tour_feedback_highlight",
            snapshot_attrs(&api.highlight_attrs())
        );
    }
}
