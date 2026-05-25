//! Progress data-display component machine.
//!
//! Owns determinate and indeterminate progress state, percent derivation, and
//! progressbar attributes. Adapters render visual structure and dispatch value
//! changes; the agnostic core remains DOM-free.

use alloc::{
    string::{String, ToString as _},
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    hash::{Hash, Hasher},
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentMessages, ComponentPart, ConnectApi, CssProperty, Env,
    HtmlAttr, Locale, MessageFn, NoEffect, TransitionPlan,
};
use ars_i18n::number;

/// Layout orientation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// Horizontal layout.
    #[default]
    Horizontal,

    /// Vertical layout.
    Vertical,
}

impl Orientation {
    /// Returns the ARIA and data-attribute token for this orientation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}

/// Props for the Progress component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled value. Outer `None` means uncontrolled; inner `None` means
    /// indeterminate.
    pub value: Option<Option<f64>>,

    /// Uncontrolled initial value.
    pub default_value: Option<f64>,

    /// Lower bound.
    pub min: f64,

    /// Upper bound.
    pub max: f64,

    /// Layout orientation.
    pub orientation: Orientation,

    /// Formatting options for locale-aware value text.
    pub format_options: Option<number::FormatOptions>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            min: 0.0,
            max: 100.0,
            orientation: Orientation::Horizontal,
            format_options: None,
        }
    }
}

impl Props {
    /// Returns fresh progress props with the documented defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the controlled value.
    #[must_use]
    pub const fn value(mut self, value: Option<f64>) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets the uncontrolled initial value.
    #[must_use]
    pub const fn default_value(mut self, value: f64) -> Self {
        self.default_value = Some(value);
        self
    }

    /// Sets the lower bound.
    #[must_use]
    pub const fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
    }

    /// Sets the upper bound.
    #[must_use]
    pub const fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    /// Sets the layout orientation.
    #[must_use]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets locale-aware number formatting options.
    #[must_use]
    pub fn format_options(mut self, options: number::FormatOptions) -> Self {
        self.format_options = Some(options);
        self
    }
}

/// States for the Progress component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Value is set and progress is at rest.
    Idle,

    /// Indeterminate or active progress is underway.
    Loading,

    /// Value has reached the maximum bound.
    Complete,
}

/// Events accepted by the Progress state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Update value; `None` means indeterminate.
    SetValue(Option<f64>),

    /// Update the maximum bound.
    SetMax(f64),

    /// Jump to complete state.
    Complete,

    /// Return to the idle state with an indeterminate value.
    Reset,

    /// Synchronize non-value props from adapter-owned prop changes.
    SyncProps,
}

/// Context for the Progress component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current value. `None` means indeterminate.
    pub value: Bindable<Option<f64>>,

    /// Lower bound.
    pub min: f64,

    /// Upper bound.
    pub max: f64,

    /// Layout orientation.
    pub orientation: Orientation,

    /// Whether the current value is indeterminate.
    pub indeterminate: bool,

    /// Derived percentage clamped into `0..=100`.
    pub percent: f64,

    /// Resolved locale for message formatting.
    pub locale: Locale,

    /// Resolved messages for announcements.
    pub messages: Messages,
}

impl Context {
    /// Computes the percentage value from the given value and bounds.
    #[must_use]
    pub fn compute_percent(value: Option<f64>, min: f64, max: f64) -> f64 {
        if min >= max {
            return 0.0;
        }

        if let Some(value) = value {
            ((value - min) / (max - min) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        }
    }
}

/// Messages for the Progress component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Text announced by screen readers when progress is loading.
    pub loading: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Text announced by screen readers when progress is complete.
    pub complete: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            loading: MessageFn::static_str("Loading…"),
            complete: MessageFn::static_str("Complete"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Structural parts exposed by the Progress connect API.
#[derive(Clone, Debug)]
pub enum Part {
    /// The root progressbar element.
    Root,

    /// The visible label element.
    Label,

    /// The visual track element.
    Track,

    /// The filled range element.
    Range,

    /// The visible formatted value element.
    ValueText,

    /// The circular progress background track.
    CircleTrack,

    /// The circular progress foreground range.
    CircleRange {
        /// Radius used for stroke-dash calculations.
        radius: f64,
    },
}

impl PartialEq for Part {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Root, Self::Root)
            | (Self::Label, Self::Label)
            | (Self::Track, Self::Track)
            | (Self::Range, Self::Range)
            | (Self::ValueText, Self::ValueText)
            | (Self::CircleTrack, Self::CircleTrack) => true,
            (Self::CircleRange { radius: left }, Self::CircleRange { radius: right }) => {
                left.to_bits() == right.to_bits()
            }

            _ => false,
        }
    }
}

impl Eq for Part {}

impl Hash for Part {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        if let Self::CircleRange { radius } = self {
            radius.to_bits().hash(state);
        }
    }
}

impl ComponentPart for Part {
    const ROOT: Self = Self::Root;

    fn scope() -> &'static str {
        "progress"
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Label => "label",
            Self::Track => "track",
            Self::Range => "range",
            Self::ValueText => "value-text",
            Self::CircleTrack => "circle-track",
            Self::CircleRange { .. } => "circle-range",
        }
    }

    fn all() -> Vec<Self> {
        alloc::vec![
            Self::Root,
            Self::Label,
            Self::Track,
            Self::Range,
            Self::ValueText,
            Self::CircleTrack,
            Self::CircleRange { radius: 10.0 },
        ]
    }
}

/// Machine for the Progress component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = NoEffect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let value = props.value.unwrap_or(props.default_value);
        let percent = Context::compute_percent(value, props.min, props.max);
        let state = state_for_value(value, props.max);

        (
            state,
            Context {
                value: match props.value {
                    Some(value) => Bindable::controlled(value),
                    None => Bindable::uncontrolled(props.default_value),
                },
                min: props.min,
                max: props.max,
                orientation: props.orientation,
                indeterminate: value.is_none(),
                percent,
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
            Event::SetValue(value) => {
                let value = *value;
                let min = ctx.min;
                let max = ctx.max;

                Some(TransitionPlan::to(state_for_value(value, max)).apply(
                    move |ctx: &mut Context| {
                        set_context_value(ctx, value);

                        ctx.indeterminate = value.is_none();
                        ctx.percent = Context::compute_percent(value, min, max);
                    },
                ))
            }

            Event::SetMax(max) => {
                let max = *max;
                let value = *ctx.value.get();
                let min = ctx.min;

                Some(TransitionPlan::to(state_for_value(value, max)).apply(
                    move |ctx: &mut Context| {
                        ctx.max = max;
                        ctx.indeterminate = value.is_none();
                        ctx.percent = Context::compute_percent(value, min, max);
                    },
                ))
            }

            Event::Complete => Some(TransitionPlan::to(State::Complete).apply(
                |ctx: &mut Context| {
                    set_context_value(ctx, Some(ctx.max));

                    ctx.indeterminate = false;
                    ctx.percent = 100.0;
                },
            )),

            Event::Reset => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                set_context_value(ctx, None);

                ctx.indeterminate = true;
                ctx.percent = 0.0;
            })),

            Event::SyncProps => {
                let min = props.min;
                let max = props.max;
                let orientation = props.orientation;
                let value = *ctx.value.get();

                Some(TransitionPlan::to(state_for_value(value, max)).apply(
                    move |ctx: &mut Context| {
                        ctx.min = min;
                        ctx.max = max;
                        ctx.orientation = orientation;
                        ctx.indeterminate = value.is_none();
                        ctx.percent = Context::compute_percent(value, min, max);
                    },
                ))
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
        let mut events = Vec::new();

        if old.value != new.value {
            events.push(Event::SetValue(new.value.unwrap_or(new.default_value)));
        }

        if old.min != new.min || old.max != new.max || old.orientation != new.orientation {
            events.push(Event::SyncProps);
        }

        events
    }
}

/// API for the Progress component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("progress::Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .field("props", &self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Returns true when progress is indeterminate.
    #[must_use]
    pub const fn is_indeterminate(&self) -> bool {
        self.ctx.indeterminate
    }

    /// Returns true when progress is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        *self.state == State::Complete
    }

    /// Returns the current percentage.
    #[must_use]
    pub const fn percent(&self) -> f64 {
        self.ctx.percent
    }

    /// Sends a value update event.
    pub fn set_value(&self, value: Option<f64>) {
        (self.send)(Event::SetValue(value));
    }

    /// Returns locale-formatted value text for `aria-valuetext`.
    #[must_use]
    pub fn value_text(&self) -> String {
        if self.ctx.indeterminate {
            return (self.ctx.messages.loading)(&self.ctx.locale);
        }

        if self.is_complete() {
            return (self.ctx.messages.complete)(&self.ctx.locale);
        }

        let formatter = number::Formatter::new(
            &self.ctx.locale,
            self.props.format_options.clone().unwrap_or_default(),
        );

        alloc::format!(
            "{} complete",
            formatter.format_percent(self.ctx.percent / 100.0, Some(0))
        )
    }

    /// Returns root attributes for the progress.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        attrs
            .set(HtmlAttr::Role, "progressbar")
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                self.ctx.orientation.as_str(),
            )
            .set(
                HtmlAttr::Data("ars-orientation"),
                self.ctx.orientation.as_str(),
            )
            .set(HtmlAttr::Data("ars-state"), state_attr(*self.state))
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), self.ctx.min.to_string())
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), self.ctx.max.to_string())
            .set(HtmlAttr::Aria(AriaAttr::ValueText), self.value_text());

        if !self.ctx.indeterminate
            && let Some(value) = self.ctx.value.get()
        {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), value.to_string());
        }

        attrs
    }

    /// Returns label attributes for the progress.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        part_attrs(&Part::Label)
    }

    /// Returns track attributes for the progress.
    #[must_use]
    pub fn track_attrs(&self) -> AttrMap {
        part_attrs(&Part::Track)
    }

    /// Returns range attributes for the progress.
    #[must_use]
    pub fn range_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Range);

        let percent = if self.ctx.indeterminate {
            0.0
        } else {
            self.ctx.percent
        };

        attrs
            .set_style(CssProperty::Width, alloc::format!("{percent}%"))
            .set(
                HtmlAttr::Data("ars-indeterminate"),
                self.ctx.indeterminate.to_string(),
            );

        attrs
    }

    /// Returns value text attributes for the progress.
    #[must_use]
    pub fn value_text_attrs(&self) -> AttrMap {
        part_attrs(&Part::ValueText)
    }

    /// Computes stroke-dashoffset for an SVG circle with the given radius.
    #[must_use]
    pub fn circle_stroke_dashoffset(&self, radius: f64) -> f64 {
        let circumference = 2.0 * core::f64::consts::PI * radius;

        let percent = if self.ctx.indeterminate {
            0.0
        } else {
            self.ctx.percent / 100.0
        };

        circumference * (1.0 - percent)
    }

    /// Returns circle track attributes for the progress.
    #[must_use]
    pub fn circle_track_attrs(&self) -> AttrMap {
        part_attrs(&Part::CircleTrack)
    }

    /// Returns circle range attributes for the progress.
    #[must_use]
    pub fn circle_range_attrs(&self, radius: f64) -> AttrMap {
        let mut attrs = part_attrs(&Part::CircleRange { radius });

        let circumference = 2.0 * core::f64::consts::PI * radius;
        let offset = self.circle_stroke_dashoffset(radius);

        attrs
            .set(
                HtmlAttr::Data("stroke-dasharray"),
                circumference.to_string(),
            )
            .set(HtmlAttr::Data("stroke-dashoffset"), offset.to_string());

        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Track => self.track_attrs(),
            Part::Range => self.range_attrs(),
            Part::ValueText => self.value_text_attrs(),
            Part::CircleTrack => self.circle_track_attrs(),
            Part::CircleRange { radius } => self.circle_range_attrs(radius),
        }
    }
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

const fn state_for_value(value: Option<f64>, max: f64) -> State {
    match value {
        None => State::Loading,
        Some(value) if value >= max => State::Complete,
        Some(_) => State::Idle,
    }
}

const fn state_attr(state: State) -> &'static str {
    match state {
        State::Idle => "idle",
        State::Loading => "loading",
        State::Complete => "complete",
    }
}

fn set_context_value(ctx: &mut Context, value: Option<f64>) {
    if ctx.value.is_controlled() {
        ctx.value.sync_controlled(Some(value));
    } else {
        ctx.value.set(value);
    }
}

#[cfg(test)]
mod tests {
    use ars_core::Service;
    use insta::assert_snapshot;

    use super::*;

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        alloc::format!("{attrs:#?}")
    }

    #[test]
    fn progress_root_determinate_snapshot() {
        let service = service(Props::new().id("progress").default_value(25.0));

        assert_snapshot!(
            "progress_root_determinate",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn progress_root_indeterminate_snapshot() {
        let service = service(Props::new().id("progress"));

        assert_snapshot!(
            "progress_root_indeterminate",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn progress_root_complete_snapshot() {
        let service = service(Props::new().id("progress").default_value(100.0));

        assert_snapshot!(
            "progress_root_complete",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn progress_root_vertical_snapshot() {
        let service = service(
            Props::new()
                .id("progress")
                .default_value(25.0)
                .orientation(Orientation::Vertical),
        );

        assert_snapshot!(
            "progress_root_vertical",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn progress_linear_parts_snapshot() {
        let service = service(Props::new().id("progress").default_value(25.0));

        let api = service.connect(&|_| {});

        assert_snapshot!("progress_label", snapshot_attrs(&api.label_attrs()));
        assert_snapshot!("progress_track", snapshot_attrs(&api.track_attrs()));
        assert_snapshot!("progress_range", snapshot_attrs(&api.range_attrs()));
        assert_snapshot!(
            "progress_value_text",
            snapshot_attrs(&api.value_text_attrs())
        );
    }

    #[test]
    fn progress_circle_parts_snapshot() {
        let service = service(Props::new().id("progress").default_value(25.0));

        let api = service.connect(&|_| {});

        assert_snapshot!(
            "progress_circle_track",
            snapshot_attrs(&api.circle_track_attrs())
        );
        assert_snapshot!(
            "progress_circle_range",
            snapshot_attrs(&api.circle_range_attrs(10.0))
        );
    }
}
