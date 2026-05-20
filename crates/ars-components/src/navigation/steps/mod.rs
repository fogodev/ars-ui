//! Steps navigation component.
//!
//! Steps owns the agnostic progress state for multi-step workflows:
//! current-step tracking, per-step statuses, linear navigation guards,
//! validation/skippable callbacks, orientation attributes, and completion
//! effect intents.

use alloc::{
    format,
    string::{String, ToString as _},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::{
    cmp::Ordering,
    fmt::{self, Debug},
    num::NonZeroU32,
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Env, HtmlAttr, Locale, MessageFn, Orientation, PendingEffect, TransitionPlan,
};

/// The only steps machine state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum State {
    /// Current step is tracked in context.
    #[default]
    Idle,
}

/// Status of a single step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    /// The step has not been visited.
    Incomplete,

    /// The step is the active step.
    Current,

    /// The step is complete.
    Complete,

    /// The step has an error.
    Error,
}

impl Status {
    /// Returns the token rendered into `data-ars-state`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Incomplete => "incomplete",
            Self::Current => "current",
            Self::Complete => "complete",
            Self::Error => "error",
        }
    }
}

/// Events accepted by the steps state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Navigate directly to a zero-based step.
    GoToStep(u32),

    /// Advance to the next step.
    NextStep,

    /// Move to the previous step.
    PrevStep,

    /// Mark a step as complete.
    CompleteStep(u32),

    /// Explicitly set a step status.
    SetStatus {
        /// Zero-based step index.
        step: u32,

        /// New step status.
        status: Status,
    },

    /// Synchronize render props into context.
    SyncProps,
}

/// Typed effect intents emitted by the steps machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// The active step changed.
    StepChange,

    /// The user advanced past the final step.
    Complete,
}

/// Per-item step label template.
pub type StepLabelFn = dyn Fn(usize, usize, &Locale) -> String + Send + Sync;

/// Localized steps messages.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Root label.
    pub root_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Per-item step label template.
    pub step_label: MessageFn<StepLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            root_label: MessageFn::static_str("Steps"),
            step_label: MessageFn::new(Arc::new(|current: usize, total: usize, _locale: &Locale| {
                format!("Step {current} of {total}")
            }) as Arc<StepLabelFn>),
        }
    }
}

impl ComponentMessages for Messages {}

/// Immutable configuration for a [`Steps`](self) instance.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Controlled zero-based current step.
    pub step: Option<u32>,

    /// Initial uncontrolled zero-based step.
    pub default_step: u32,

    /// Total step count.
    pub count: NonZeroU32,

    /// Optional initial status list.
    pub statuses: Option<Vec<Status>>,

    /// Whether forward navigation is restricted to sequential advancement.
    pub linear: bool,

    /// Optional validation callback for the current step before advancing.
    pub is_step_valid: Option<Callback<dyn Fn(u32) -> bool + Send + Sync>>,

    /// Optional predicate that allows intermediate steps to be skipped.
    pub is_step_skippable: Option<Callback<dyn Fn(u32) -> bool + Send + Sync>>,

    /// Callback fired by the step-change effect.
    pub on_step_change: Option<Callback<dyn Fn(u32) + Send + Sync>>,

    /// Callback fired by the completion effect.
    pub on_complete: Option<Callback<dyn Fn() + Send + Sync>>,

    /// Visual stacking axis.
    pub orientation: Orientation,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            step: None,
            default_step: 0,
            count: NonZeroU32::new(1).expect("literal is non-zero"),
            statuses: None,
            linear: false,
            is_step_valid: None,
            is_step_skippable: None,
            on_step_change: None,
            on_complete: None,
            orientation: Orientation::Horizontal,
        }
    }
}

impl Props {
    /// Returns default steps props.
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

    /// Sets [`step`](Self::step).
    #[must_use]
    pub const fn step(mut self, step: u32) -> Self {
        self.step = Some(step);
        self
    }

    /// Clears [`step`](Self::step), switching to uncontrolled mode.
    #[must_use]
    pub const fn uncontrolled(mut self) -> Self {
        self.step = None;
        self
    }

    /// Sets [`default_step`](Self::default_step).
    #[must_use]
    pub const fn default_step(mut self, step: u32) -> Self {
        self.default_step = step;
        self
    }

    /// Sets [`count`](Self::count).
    #[must_use]
    pub const fn count(mut self, count: NonZeroU32) -> Self {
        self.count = count;
        self
    }

    /// Sets [`statuses`](Self::statuses).
    #[must_use]
    pub fn statuses(mut self, statuses: Vec<Status>) -> Self {
        self.statuses = Some(statuses);
        self
    }

    /// Sets [`linear`](Self::linear).
    #[must_use]
    pub const fn linear(mut self, linear: bool) -> Self {
        self.linear = linear;
        self
    }

    /// Sets [`is_step_valid`](Self::is_step_valid).
    #[must_use]
    pub fn is_step_valid(
        mut self,
        callback: impl Into<Callback<dyn Fn(u32) -> bool + Send + Sync>>,
    ) -> Self {
        self.is_step_valid = Some(callback.into());
        self
    }

    /// Sets [`is_step_skippable`](Self::is_step_skippable).
    #[must_use]
    pub fn is_step_skippable(
        mut self,
        callback: impl Into<Callback<dyn Fn(u32) -> bool + Send + Sync>>,
    ) -> Self {
        self.is_step_skippable = Some(callback.into());
        self
    }

    /// Sets [`on_step_change`](Self::on_step_change).
    #[must_use]
    pub fn on_step_change(
        mut self,
        callback: impl Into<Callback<dyn Fn(u32) + Send + Sync>>,
    ) -> Self {
        self.on_step_change = Some(callback.into());
        self
    }

    /// Sets [`on_complete`](Self::on_complete).
    #[must_use]
    pub fn on_complete(mut self, callback: impl Into<Callback<dyn Fn() + Send + Sync>>) -> Self {
        self.on_complete = Some(callback.into());
        self
    }

    /// Sets [`orientation`](Self::orientation).
    #[must_use]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }
}

/// Runtime context for the steps machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current zero-based step.
    pub step: Bindable<u32>,

    /// Total step count.
    pub count: NonZeroU32,

    /// Per-step statuses.
    pub statuses: Vec<Status>,

    /// Last status prop seen by sync, used to preserve runtime progress across unrelated prop changes.
    pub statuses_prop: Option<Vec<Status>>,

    /// Whether linear mode is enabled.
    pub linear: bool,

    /// Visual stacking axis.
    pub orientation: Orientation,

    /// Stable ids derived from props.
    pub ids: ComponentIds,

    /// Active locale.
    pub locale: Locale,

    /// Localized messages.
    pub messages: Messages,
}

/// Anatomy parts exposed by the steps connect API.
#[derive(ComponentPart)]
#[scope = "steps"]
pub enum Part {
    /// Root wrapper.
    Root,

    /// List container.
    List,

    /// Step item.
    Item {
        /// Zero-based step index.
        index: u32,
    },

    /// Step indicator.
    Indicator {
        /// Zero-based step index.
        index: u32,
    },

    /// Step title.
    Title {
        /// Zero-based step index.
        index: u32,
    },

    /// Step description.
    Description {
        /// Zero-based step index.
        index: u32,
    },

    /// Progress separator after a step.
    Separator {
        /// Zero-based step index before this separator.
        after_index: u32,
    },

    /// Step content panel.
    Content {
        /// Zero-based step index.
        index: u32,
    },

    /// Previous-step trigger.
    PrevTrigger,

    /// Next-step trigger.
    NextTrigger,
}

/// Steps state machine.
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
        let count = props.count;

        let initial = clamp_step(props.step.unwrap_or(props.default_step), count);

        let step = if props.step.is_some() {
            Bindable::controlled(initial)
        } else {
            Bindable::uncontrolled(initial)
        };

        (
            State::Idle,
            Context {
                step,
                count,
                statuses: normalized_statuses(props.statuses.clone(), count, initial),
                statuses_prop: props.statuses.clone(),
                linear: props.linear,
                orientation: props.orientation,
                ids: ComponentIds::from_id(&props.id),
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::GoToStep(target) => {
                if *target >= ctx.count.get() {
                    return None;
                }

                step_change_plan(ctx, props, *target)
            }

            Event::NextStep => {
                let current = *ctx.step.get();

                if let Some(is_valid) = &props.is_step_valid
                    && !is_valid(current)
                {
                    return None;
                }

                let next = current + 1;

                if next >= ctx.count.get() {
                    return Some(
                        TransitionPlan::context_only(|_ctx: &mut Context| {}).with_effect(
                            PendingEffect::new(
                                Effect::Complete,
                                |_ctx: &Context, props: &Props, _send| {
                                    if let Some(callback) = &props.on_complete {
                                        (callback)();
                                    }

                                    ars_core::no_cleanup()
                                },
                            ),
                        ),
                    );
                }

                step_change_plan(ctx, props, next)
            }
            Event::PrevStep => {
                let current = *ctx.step.get();

                if current == 0 {
                    return None;
                }

                step_change_plan(ctx, props, current - 1)
            }
            Event::CompleteStep(step) => {
                let step = *step;

                if step >= ctx.count.get() || step == *ctx.step.get() {
                    return None;
                }

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    if let Some(status) = ctx.statuses.get_mut(step as usize) {
                        *status = Status::Complete;
                    }
                }))
            }
            Event::SetStatus { step, status } => {
                let step = *step;
                let status = *status;

                if step >= ctx.count.get()
                    || (status == Status::Current && ctx.step.is_controlled())
                {
                    return None;
                }

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    if status == Status::Current {
                        if let Some(slot) = ctx.statuses.get_mut(step as usize) {
                            *slot = status;
                        }

                        ctx.step.set(step);

                        normalize_current_status(&mut ctx.statuses, step);
                    } else if let Some(slot) = ctx.statuses.get_mut(step as usize) {
                        *slot = status;
                    }
                }))
            }
            Event::SyncProps => {
                let count = props.count;
                let controlled = props.step.map(|step| clamp_step(step, count));
                let target = controlled.unwrap_or_else(|| clamp_step(*ctx.step.get(), count));
                let statuses_prop = props.statuses.clone();
                let statuses = if statuses_prop == ctx.statuses_prop {
                    normalized_statuses(Some(ctx.statuses.clone()), count, target)
                } else {
                    normalized_statuses(statuses_prop.clone(), count, target)
                };
                let linear = props.linear;
                let orientation = props.orientation;

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.count = count;
                    ctx.step.sync_controlled(controlled);
                    ctx.step.set(target);
                    ctx.statuses = statuses;
                    ctx.statuses_prop = statuses_prop;
                    ctx.linear = linear;
                    ctx.orientation = orientation;
                }))
            }
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
        if old.step != new.step
            || old.default_step != new.default_step
            || old.count != new.count
            || old.statuses != new.statuses
            || old.linear != new.linear
            || old.orientation != new.orientation
        {
            vec![Event::SyncProps]
        } else {
            Vec::new()
        }
    }
}

/// Connected API for a [`Steps`](self) service.
pub struct Api<'a> {
    /// Current state.
    state: &'a State,

    /// Current context.
    ctx: &'a Context,

    /// Current props.
    props: &'a Props,

    /// Event dispatcher.
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Returns the current zero-based step.
    #[must_use]
    pub fn current_step(&self) -> u32 {
        *self.ctx.step.get()
    }

    /// Returns the total step count.
    #[must_use]
    pub const fn step_count(&self) -> u32 {
        self.ctx.count.get()
    }

    /// Returns `true` at the first step.
    #[must_use]
    pub fn is_first_step(&self) -> bool {
        self.current_step() == 0
    }

    /// Returns `true` at the last step.
    #[must_use]
    pub fn is_last_step(&self) -> bool {
        self.current_step() + 1 >= self.ctx.count.get()
    }

    /// Returns the status for a step.
    #[must_use]
    pub fn step_status(&self, index: u32) -> Option<&Status> {
        self.ctx.statuses.get(index as usize)
    }

    /// Returns the current state.
    #[must_use]
    pub const fn state(&self) -> State {
        *self.state
    }

    /// Returns the current props.
    #[must_use]
    pub const fn props(&self) -> &Props {
        self.props
    }

    /// Attributes for the root wrapper.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(
                HtmlAttr::Data("ars-orientation"),
                orientation_token(self.ctx.orientation),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.root_label)(&self.ctx.locale),
            );

        attrs
    }

    /// Attributes for the list container.
    #[must_use]
    pub fn list_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = Part::List.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Role, "list");

        attrs
    }

    /// Attributes for a step item.
    #[must_use]
    pub fn item_attrs(&self, index: u32) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] =
            (Part::Item { index }).data_attrs();

        let status = self.status_or_incomplete(index);

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Data("ars-index"), index.to_string())
            .set(HtmlAttr::Data("ars-state"), status.as_str())
            .set(HtmlAttr::Role, "listitem")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.step_label)(
                    index as usize + 1,
                    self.ctx.count.get() as usize,
                    &self.ctx.locale,
                ),
            );

        if self.current_step() == index {
            attrs.set(HtmlAttr::Aria(AriaAttr::Current), "step");
        }

        attrs
    }

    /// Attributes for a step indicator.
    #[must_use]
    pub fn indicator_attrs(&self, index: u32) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] =
            (Part::Indicator { index }).data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(
                HtmlAttr::Data("ars-state"),
                self.status_or_incomplete(index).as_str(),
            )
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Attributes for a step title.
    #[must_use]
    pub fn title_attrs(&self, index: u32) -> AttrMap {
        self.indexed_attrs(&Part::Title { index }, index)
    }

    /// Attributes for a step description.
    #[must_use]
    pub fn description_attrs(&self, index: u32) -> AttrMap {
        self.indexed_attrs(&Part::Description { index }, index)
    }

    /// Attributes for a separator after a step.
    #[must_use]
    pub fn separator_attrs(&self, after_index: u32) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] =
            (Part::Separator { after_index }).data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Data("ars-index"), after_index.to_string())
            .set_bool(
                HtmlAttr::Data("ars-completed"),
                after_index < self.current_step(),
            )
            .set(HtmlAttr::Role, "separator")
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Attributes for a step content panel.
    #[must_use]
    pub fn content_attrs(&self, index: u32) -> AttrMap {
        let mut attrs = self.indexed_attrs(&Part::Content { index }, index);

        if self.current_step() != index {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Attributes for the previous-step trigger.
    #[must_use]
    pub fn prev_trigger_attrs(&self) -> AttrMap {
        let mut attrs = self.button_attrs(&Part::PrevTrigger);

        if self.is_first_step() {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Attributes for the next-step trigger.
    #[must_use]
    pub fn next_trigger_attrs(&self) -> AttrMap {
        let mut attrs = self.button_attrs(&Part::NextTrigger);

        if self.is_last_step() {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Dispatches previous-step navigation when not already at the first step.
    pub fn on_prev_trigger_click(&self) {
        if !self.is_first_step() {
            (self.send)(Event::PrevStep);
        }
    }

    /// Dispatches next-step navigation when not already at the last step.
    pub fn on_next_trigger_click(&self) {
        if !self.is_last_step() {
            (self.send)(Event::NextStep);
        }
    }

    /// Dispatches direct step navigation.
    pub fn on_item_click(&self, index: u32) {
        (self.send)(Event::GoToStep(index));
    }

    fn status_or_incomplete(&self, index: u32) -> Status {
        self.ctx
            .statuses
            .get(index as usize)
            .copied()
            .unwrap_or(Status::Incomplete)
    }

    fn indexed_attrs(&self, part: &Part, index: u32) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = part.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Data("ars-index"), index.to_string());

        attrs
    }

    fn button_attrs(&self, part: &Part) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = part.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Type, "button");

        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::List => self.list_attrs(),
            Part::Item { index } => self.item_attrs(index),
            Part::Indicator { index } => self.indicator_attrs(index),
            Part::Title { index } => self.title_attrs(index),
            Part::Description { index } => self.description_attrs(index),
            Part::Separator { after_index } => self.separator_attrs(after_index),
            Part::Content { index } => self.content_attrs(index),
            Part::PrevTrigger => self.prev_trigger_attrs(),
            Part::NextTrigger => self.next_trigger_attrs(),
        }
    }
}

fn clamp_step(step: u32, count: NonZeroU32) -> u32 {
    step.min(count.get().saturating_sub(1))
}

fn normalized_statuses(
    statuses: Option<Vec<Status>>,
    count: NonZeroU32,
    current: u32,
) -> Vec<Status> {
    let mut statuses = statuses.unwrap_or_else(|| vec![Status::Incomplete; count.get() as usize]);

    statuses.resize(count.get() as usize, Status::Incomplete);

    normalize_current_status(&mut statuses, current);

    statuses
}

fn normalize_current_status(statuses: &mut [Status], current: u32) {
    for (index, status) in statuses.iter_mut().enumerate() {
        if index as u32 == current {
            *status = Status::Current;
        } else if *status == Status::Current {
            *status = Status::Incomplete;
        }
    }
}

fn step_change_plan(ctx: &Context, props: &Props, target: u32) -> Option<TransitionPlan<Machine>> {
    let current = *ctx.step.get();

    if target == current || target >= ctx.count.get() {
        return None;
    }

    if target > current
        && let Some(is_valid) = &props.is_step_valid
        && !is_valid(current)
    {
        return None;
    }

    if ctx.linear
        && matches!(target.checked_sub(current), Some(2..))
        && !can_skip_intermediate(props, current, target)
    {
        return None;
    }

    let controlled = ctx.step.is_controlled();

    Some(
        TransitionPlan::context_only(move |ctx: &mut Context| {
            if controlled {
                return;
            }

            if matches!(target.cmp(&current), Ordering::Greater)
                && let Some(status) = ctx.statuses.get_mut(current as usize)
            {
                *status = Status::Complete;
            }

            normalize_current_status(&mut ctx.statuses, target);

            ctx.step.set(target);
        })
        .with_effect(PendingEffect::new(
            Effect::StepChange,
            move |_ctx: &Context, props: &Props, _send| {
                if let Some(callback) = &props.on_step_change {
                    (callback)(target);
                }

                ars_core::no_cleanup()
            },
        )),
    )
}

fn can_skip_intermediate(props: &Props, current: u32, target: u32) -> bool {
    let Some(is_skippable) = &props.is_step_skippable else {
        return false;
    };

    (current + 1..target).all(is_skippable.as_ref())
}

const fn orientation_token(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, sync::Arc, vec, vec::Vec};
    use std::sync::{Mutex, MutexGuard};

    use ars_core::{Service, StrongSend};

    use super::*;

    fn props() -> Props {
        Props::new()
            .id("steps")
            .count(NonZeroU32::new(4).expect("non-zero"))
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().expect("test mutex should not be poisoned")
    }

    #[test]
    fn attrs_cover_all_step_anatomy_parts() {
        let service = service(
            props()
                .default_step(1)
                .orientation(Orientation::Vertical)
                .statuses(vec![
                    Status::Complete,
                    Status::Current,
                    Status::Incomplete,
                    Status::Error,
                ]),
        );
        let api = service.connect(&|_| {});

        insta::assert_snapshot!("steps_root_vertical", snapshot_attrs(&api.root_attrs()));
        insta::assert_snapshot!("steps_list", snapshot_attrs(&api.list_attrs()));
        insta::assert_snapshot!("steps_item_current", snapshot_attrs(&api.item_attrs(1)));
        insta::assert_snapshot!("steps_item_error", snapshot_attrs(&api.item_attrs(3)));
        insta::assert_snapshot!(
            "steps_indicator_complete",
            snapshot_attrs(&api.indicator_attrs(0))
        );
        insta::assert_snapshot!("steps_title", snapshot_attrs(&api.title_attrs(1)));
        insta::assert_snapshot!(
            "steps_description",
            snapshot_attrs(&api.description_attrs(1))
        );
        insta::assert_snapshot!(
            "steps_separator_completed",
            snapshot_attrs(&api.separator_attrs(0))
        );
        insta::assert_snapshot!(
            "steps_content_visible",
            snapshot_attrs(&api.content_attrs(1))
        );
        insta::assert_snapshot!(
            "steps_content_hidden",
            snapshot_attrs(&api.content_attrs(2))
        );
        insta::assert_snapshot!(
            "steps_prev_enabled",
            snapshot_attrs(&api.prev_trigger_attrs())
        );
        insta::assert_snapshot!(
            "steps_next_enabled",
            snapshot_attrs(&api.next_trigger_attrs())
        );
    }

    #[test]
    fn first_and_last_steps_disable_respective_triggers() {
        let first = service(props().default_step(0));

        assert!(
            first
                .connect(&|_| {})
                .prev_trigger_attrs()
                .get_value(&HtmlAttr::Disabled)
                .is_some()
        );

        let last = service(props().default_step(3));

        assert!(
            last.connect(&|_| {})
                .next_trigger_attrs()
                .get_value(&HtmlAttr::Disabled)
                .is_some()
        );
    }

    #[test]
    fn next_prev_and_direct_navigation_update_statuses() {
        let mut service = service(props().default_step(0));

        drop(service.send(Event::NextStep));

        assert_eq!(*service.context().step.get(), 1);
        assert_eq!(service.context().statuses[0], Status::Complete);
        assert_eq!(service.context().statuses[1], Status::Current);

        drop(service.send(Event::PrevStep));

        assert_eq!(*service.context().step.get(), 0);
        assert_eq!(service.context().statuses[0], Status::Current);
        assert_eq!(service.context().statuses[1], Status::Incomplete);

        drop(service.send(Event::GoToStep(3)));

        assert_eq!(*service.context().step.get(), 3);
        assert_eq!(service.context().statuses[3], Status::Current);
    }

    #[test]
    fn controlled_navigation_keeps_statuses_aligned_until_prop_sync() {
        let mut service = service(props().step(1));

        drop(service.send(Event::NextStep));

        assert_eq!(*service.context().step.get(), 1);
        assert_eq!(service.context().statuses[1], Status::Current);
        assert_eq!(
            service
                .context()
                .statuses
                .iter()
                .filter(|status| **status == Status::Current)
                .count(),
            1
        );

        drop(service.set_props(props().step(2)));

        assert_eq!(*service.context().step.get(), 2);
        assert_eq!(service.context().statuses[2], Status::Current);
    }

    #[test]
    fn current_and_out_of_range_navigation_are_noops() {
        let mut service = service(props().default_step(1));

        let result = service.send(Event::GoToStep(1));

        assert!(!result.context_changed);
        assert_eq!(*service.context().step.get(), 1);

        let context_before = service.context().clone();

        assert!(step_change_plan(service.context(), service.props(), 4).is_none());
        assert_eq!(service.context(), &context_before);

        let result = service.send(Event::GoToStep(99));

        assert!(!result.context_changed);
        assert_eq!(service.context(), &context_before);
    }

    #[test]
    fn complete_and_status_events_ignore_out_of_range_steps() {
        let mut service = service(props().default_step(1));

        let before = service.context().clone();

        let result = service.send(Event::CompleteStep(4));

        assert!(!result.context_changed);
        assert_eq!(service.context(), &before);

        let result = service.send(Event::SetStatus {
            step: 4,
            status: Status::Error,
        });

        assert!(!result.context_changed);
        assert_eq!(service.context(), &before);
    }

    #[test]
    fn set_status_current_moves_active_step_and_normalizes() {
        let mut service = service(props().default_step(1));

        drop(service.send(Event::SetStatus {
            step: 3,
            status: Status::Current,
        }));

        assert_eq!(*service.context().step.get(), 3);
        assert_eq!(service.context().statuses[1], Status::Incomplete);
        assert_eq!(service.context().statuses[3], Status::Current);
    }

    #[test]
    fn controlled_set_status_current_keeps_statuses_aligned() {
        let mut service = service(props().step(1));

        let result = service.send(Event::SetStatus {
            step: 3,
            status: Status::Current,
        });

        assert!(!result.context_changed);
        assert_eq!(*service.context().step.get(), 1);
        assert_eq!(service.context().statuses[1], Status::Current);
        assert_eq!(service.context().statuses[3], Status::Incomplete);
    }

    #[test]
    fn set_status_can_mark_active_step_error() {
        let mut service = service(props().default_step(1));

        drop(service.send(Event::SetStatus {
            step: 1,
            status: Status::Error,
        }));

        assert_eq!(*service.context().step.get(), 1);
        assert_eq!(service.context().statuses[1], Status::Error);
        assert_eq!(
            service
                .context()
                .statuses
                .iter()
                .filter(|status| **status == Status::Current)
                .count(),
            0
        );
        assert_eq!(
            service
                .connect(&|_| {})
                .item_attrs(1)
                .get(&HtmlAttr::Aria(AriaAttr::Current)),
            Some("step")
        );
    }

    #[test]
    fn completing_current_step_preserves_current_status() {
        let mut service = service(props().default_step(3));

        drop(service.send(Event::NextStep));

        assert_eq!(*service.context().step.get(), 3);
        assert_eq!(service.context().statuses[3], Status::Current);
        assert_eq!(
            service
                .context()
                .statuses
                .iter()
                .filter(|status| **status == Status::Current)
                .count(),
            1
        );

        drop(service.send(Event::CompleteStep(3)));

        assert_eq!(*service.context().step.get(), 3);
        assert_eq!(service.context().statuses[3], Status::Current);
    }

    #[test]
    fn sync_props_preserves_runtime_statuses_when_status_prop_is_unchanged() {
        let mut service = service(props().default_step(0));

        drop(service.send(Event::NextStep));
        drop(service.send(Event::SetStatus {
            step: 2,
            status: Status::Error,
        }));

        drop(service.set_props(props().default_step(0).orientation(Orientation::Vertical)));

        assert_eq!(*service.context().step.get(), 1);
        assert_eq!(service.context().statuses[0], Status::Complete);
        assert_eq!(service.context().statuses[1], Status::Current);
        assert_eq!(service.context().statuses[2], Status::Error);
        assert_eq!(service.context().orientation, Orientation::Vertical);
    }

    #[test]
    fn linear_mode_blocks_skipping_without_skippable_callback() {
        let mut service = service(props().linear(true));

        let result = service.send(Event::GoToStep(3));

        assert!(!result.context_changed);
        assert_eq!(*service.context().step.get(), 0);
    }

    #[test]
    fn linear_mode_allows_adjacent_next_step() {
        let mut service = service(props().linear(true));

        drop(service.send(Event::NextStep));

        assert_eq!(*service.context().step.get(), 1);
        assert_eq!(service.context().statuses[0], Status::Complete);
        assert_eq!(service.context().statuses[1], Status::Current);
    }

    #[test]
    fn linear_mode_allows_skip_when_intermediate_steps_are_skippable() {
        let mut service = service(
            props()
                .linear(true)
                .is_step_skippable(|step| step == 1 || step == 2),
        );

        drop(service.send(Event::GoToStep(3)));

        assert_eq!(*service.context().step.get(), 3);
    }

    #[test]
    fn validation_blocks_next_step() {
        let mut service = service(props().is_step_valid(|_| false));

        let result = service.send(Event::NextStep);

        assert!(!result.context_changed);
        assert_eq!(*service.context().step.get(), 0);
    }

    #[test]
    fn validation_blocks_direct_forward_navigation() {
        let mut service = service(props().is_step_valid(|step| step != 0));

        let result = service.send(Event::GoToStep(1));

        assert!(!result.context_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(*service.context().step.get(), 0);
    }

    #[test]
    fn step_change_and_complete_effects_run_callbacks() {
        let changed = Arc::new(Mutex::new(Vec::new()));
        let completed = Arc::new(Mutex::new(0usize));
        let changed_clone = Arc::clone(&changed);
        let completed_clone = Arc::clone(&completed);

        let mut service = service(
            Props::new()
                .id("steps")
                .count(NonZeroU32::new(2).expect("non-zero"))
                .on_step_change(move |step| lock(&changed_clone).push(step))
                .on_complete(move || *lock(&completed_clone) += 1),
        );

        let send: StrongSend<Event> = Arc::new(|_| {});

        let result = service.send(Event::NextStep);

        assert_eq!(result.pending_effects[0].name, Effect::StepChange);

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        let result = service.send(Event::NextStep);

        assert_eq!(result.pending_effects[0].name, Effect::Complete);

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(lock(&changed).as_slice(), &[1]);
        assert_eq!(*lock(&completed), 1);
    }

    #[test]
    fn controlled_step_change_callback_receives_requested_target() {
        let changed = Arc::new(Mutex::new(Vec::new()));
        let changed_clone = Arc::clone(&changed);
        let mut service = service(
            props()
                .step(1)
                .on_step_change(move |step| lock(&changed_clone).push(step)),
        );

        let result = service.send(Event::GoToStep(3));

        assert_eq!(*service.context().step.get(), 1);

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(lock(&changed).as_slice(), &[3]);
    }

    #[test]
    fn api_handlers_dispatch_navigation_events() {
        let sent = core::cell::RefCell::new(Vec::new());
        let send = |event| sent.borrow_mut().push(event);

        let service = service(props().default_step(1));

        let api = service.connect(&send);

        api.on_prev_trigger_click();
        api.on_next_trigger_click();
        api.on_item_click(3);

        assert_eq!(
            sent.borrow().as_slice(),
            &[Event::PrevStep, Event::NextStep, Event::GoToStep(3)]
        );
    }

    #[test]
    fn api_reports_count_status_and_separator_completion() {
        let service = service(props().default_step(1).statuses(vec![
            Status::Complete,
            Status::Current,
            Status::Incomplete,
            Status::Error,
        ]));

        let api = service.connect(&|_| {});

        assert_eq!(api.step_count(), 4);
        assert_eq!(api.step_status(0), Some(&Status::Complete));
        assert_eq!(api.step_status(3), Some(&Status::Error));
        assert_eq!(api.step_status(4), None);
        assert_eq!(
            api.separator_attrs(0).get(&HtmlAttr::Data("ars-completed")),
            Some("true")
        );
        assert_eq!(
            api.separator_attrs(1).get(&HtmlAttr::Data("ars-completed")),
            Some("false")
        );
    }

    #[test]
    fn on_props_changed_detects_each_context_field() {
        let old = props()
            .step(1)
            .default_step(2)
            .statuses(vec![
                Status::Complete,
                Status::Current,
                Status::Incomplete,
                Status::Error,
            ])
            .linear(true)
            .orientation(Orientation::Vertical);

        assert!(<Machine as ars_core::Machine>::on_props_changed(&old, &old).is_empty());

        for new in [
            old.clone().step(2),
            old.clone().default_step(3),
            old.clone()
                .count(NonZeroU32::new(5).expect("non-zero step count")),
            old.clone().statuses(vec![
                Status::Current,
                Status::Incomplete,
                Status::Complete,
                Status::Error,
            ]),
            old.clone().linear(false),
            old.clone().orientation(Orientation::Horizontal),
        ] {
            assert_eq!(
                <Machine as ars_core::Machine>::on_props_changed(&old, &new),
                [Event::SyncProps],
                "expected SyncProps for {new:?}"
            );
        }
    }

    #[test]
    fn part_attrs_dispatches_every_part() {
        let service = service(props().default_step(1));

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::List), api.list_attrs());
        assert_eq!(api.part_attrs(Part::Item { index: 1 }), api.item_attrs(1));
        assert_eq!(
            api.part_attrs(Part::Indicator { index: 1 }),
            api.indicator_attrs(1)
        );
        assert_eq!(api.part_attrs(Part::Title { index: 1 }), api.title_attrs(1));
        assert_eq!(
            api.part_attrs(Part::Description { index: 1 }),
            api.description_attrs(1)
        );
        assert_eq!(
            api.part_attrs(Part::Separator { after_index: 0 }),
            api.separator_attrs(0)
        );
        assert_eq!(
            api.part_attrs(Part::Content { index: 1 }),
            api.content_attrs(1)
        );
        assert_eq!(api.part_attrs(Part::PrevTrigger), api.prev_trigger_attrs());
        assert_eq!(api.part_attrs(Part::NextTrigger), api.next_trigger_attrs());
    }
}
