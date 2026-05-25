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

/// Formats a locale-aware percent string into determinate progress value text.
pub type DeterminateTextFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

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
        if !valid_bounds(min, max) {
            return 0.0;
        }

        let (min, max) = normalize_bounds(min, max);

        if let Some(value) = value {
            if !value.is_finite() {
                return 0.0;
            }

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

    /// Formats the determinate progress percentage for screen readers.
    pub determinate: MessageFn<DeterminateTextFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            loading: MessageFn::static_str("Loading…"),
            complete: MessageFn::static_str("Complete"),
            determinate: MessageFn::new(alloc::sync::Arc::new(|percent: &str, _locale: &Locale| {
                alloc::format!("{percent} complete")
            }) as alloc::sync::Arc<DeterminateTextFn>),
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
        let state = state_for_value(value, props.min, props.max);

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
                let controlled_prop = props.value;
                let effective_value = effective_event_value(value, controlled_prop);

                Some(
                    TransitionPlan::to(state_for_value(effective_value, min, max)).apply(
                        move |ctx: &mut Context| {
                            set_context_value(ctx, value, controlled_prop);

                            let value = *ctx.value.get();

                            ctx.indeterminate = value.is_none();
                            ctx.percent = Context::compute_percent(value, min, max);
                        },
                    ),
                )
            }

            Event::SetMax(max) => {
                let max = *max;
                let value = *ctx.value.get();
                let min = ctx.min;

                Some(TransitionPlan::to(state_for_value(value, min, max)).apply(
                    move |ctx: &mut Context| {
                        ctx.max = max;
                        ctx.indeterminate = value.is_none();
                        ctx.percent = Context::compute_percent(value, min, max);
                    },
                ))
            }

            Event::Complete => {
                let controlled_prop = props.value;
                let value = if valid_bounds(ctx.min, ctx.max) {
                    Some(ctx.max)
                } else {
                    effective_event_value(Some(ctx.max), controlled_prop)
                };
                let target_state = if valid_bounds(ctx.min, ctx.max) {
                    State::Complete
                } else {
                    state_for_value(value, ctx.min, ctx.max)
                };

                Some(
                    TransitionPlan::to(target_state).apply(move |ctx: &mut Context| {
                        complete_context_value(ctx, controlled_prop);

                        let value = *ctx.value.get();

                        ctx.percent = if valid_bounds(ctx.min, ctx.max) {
                            ctx.indeterminate = false;
                            100.0
                        } else {
                            ctx.indeterminate = value.is_none();
                            Context::compute_percent(value, ctx.min, ctx.max)
                        };
                    }),
                )
            }

            Event::Reset => {
                let controlled_prop = props.value;
                let value = effective_event_value(None, controlled_prop);
                let next_state = if controlled_prop.is_some() {
                    state_for_value(value, ctx.min, ctx.max)
                } else {
                    State::Idle
                };

                Some(
                    TransitionPlan::to(next_state).apply(move |ctx: &mut Context| {
                        set_context_value(ctx, None, controlled_prop);

                        let value = *ctx.value.get();

                        ctx.indeterminate = value.is_none();
                        ctx.percent = Context::compute_percent(value, ctx.min, ctx.max);
                    }),
                )
            }

            Event::SyncProps => {
                let min = props.min;
                let max = props.max;
                let orientation = props.orientation;
                let controlled_prop = props.value;
                let value = sync_props_effective_value(ctx, props);

                Some(TransitionPlan::to(state_for_value(value, min, max)).apply(
                    move |ctx: &mut Context| {
                        ctx.min = min;
                        ctx.max = max;
                        ctx.orientation = orientation;
                        sync_props_value(ctx, value, controlled_prop);
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

        (self.ctx.messages.determinate)(
            &formatter.format_percent(self.ctx.percent / 100.0, Some(0)),
            &self.ctx.locale,
        )
    }

    /// Returns root attributes for the progress.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);
        let (min, max) = normalize_bounds(self.ctx.min, self.ctx.max);

        attrs
            .set(HtmlAttr::Id, self.props.id.clone())
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
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), min.to_string())
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), max.to_string())
            .set(HtmlAttr::Aria(AriaAttr::ValueText), self.value_text());

        if self.is_complete() && valid_bounds(self.ctx.min, self.ctx.max) {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), self.ctx.max.to_string());
        } else if !self.ctx.indeterminate
            && let Some(value) = self.ctx.value.get()
        {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::ValueNow),
                self.display_value_now(*value).to_string(),
            );
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
            .set(HtmlAttr::StrokeDasharray, circumference.to_string())
            .set(HtmlAttr::StrokeDashoffset, offset.to_string());

        attrs
    }

    fn display_value_now(&self, value: f64) -> f64 {
        if self.is_complete() && valid_bounds(self.ctx.min, self.ctx.max) {
            self.ctx.max
        } else {
            display_value_now(value, self.ctx.min, self.ctx.max)
        }
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

fn state_for_value(value: Option<f64>, min: f64, max: f64) -> State {
    match value {
        None => State::Loading,
        Some(value) if valid_bounds(min, max) && value.is_finite() && value >= max => {
            State::Complete
        }
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

fn set_context_value(ctx: &mut Context, value: Option<f64>, controlled_prop: Option<Option<f64>>) {
    if let Some(value) = controlled_prop {
        ctx.value = Bindable::controlled(value);
    } else if ctx.value.is_controlled() {
        ctx.value = Bindable::uncontrolled(value);
    } else {
        ctx.value.set(value);
    }
}

fn sync_props_value(ctx: &mut Context, value: Option<f64>, controlled_prop: Option<Option<f64>>) {
    if let Some(value) = controlled_prop {
        ctx.value = Bindable::controlled(value);
    } else if ctx.value.is_controlled() {
        ctx.value = Bindable::uncontrolled(value);
    }
}

fn complete_context_value(ctx: &mut Context, controlled_prop: Option<Option<f64>>) {
    if controlled_prop.is_some() {
        ctx.value = Bindable::controlled(Some(ctx.max));
    } else if ctx.value.is_controlled() {
        ctx.value = Bindable::uncontrolled(Some(ctx.max));
    } else {
        ctx.value.set(Some(ctx.max));
    }
}

fn sync_props_effective_value(ctx: &Context, props: &Props) -> Option<f64> {
    if let Some(value) = props.value {
        value
    } else if ctx.value.is_controlled() {
        props.default_value
    } else {
        *ctx.value.get()
    }
}

const fn normalize_bounds(min: f64, max: f64) -> (f64, f64) {
    if valid_bounds(min, max) {
        (min, max)
    } else {
        (0.0, 100.0)
    }
}

const fn valid_bounds(min: f64, max: f64) -> bool {
    min.is_finite() && max.is_finite() && min < max
}

const fn clamp_value(value: f64, min: f64, max: f64) -> f64 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        min
    }
}

const fn display_value_now(value: f64, raw_min: f64, raw_max: f64) -> f64 {
    let (min, max) = normalize_bounds(raw_min, raw_max);

    if valid_bounds(raw_min, raw_max) {
        clamp_value(value, min, max)
    } else {
        min
    }
}

const fn effective_event_value(
    event_value: Option<f64>,
    controlled_prop: Option<Option<f64>>,
) -> Option<f64> {
    match controlled_prop {
        Some(value) => value,
        None => event_value,
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

    #[test]
    fn progress_sanitizes_non_finite_and_out_of_range_values() {
        let high = service(Props::new().id("progress").default_value(150.0).max(100.0));
        let high_api = high.connect(&|_| {});

        assert_eq!(high_api.percent(), 100.0);
        assert_eq!(
            high_api
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("100")
        );

        let non_finite = service(
            Props::new()
                .id("progress")
                .default_value(f64::NAN)
                .min(10.0)
                .max(80.0),
        );
        let non_finite_api = non_finite.connect(&|_| {});

        assert_eq!(non_finite_api.percent(), 0.0);
        assert_eq!(
            non_finite_api
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("10")
        );
        assert_eq!(
            Context::compute_percent(Some(f64::INFINITY), 10.0, 80.0),
            0.0
        );
        assert_eq!(Context::compute_percent(Some(50.0), 100.0, 100.0), 0.0);

        let invalid_bounds = service(
            Props::new()
                .id("progress")
                .default_value(50.0)
                .min(100.0)
                .max(0.0),
        );

        assert_eq!(invalid_bounds.state(), &State::Idle);
        let invalid_bounds_api = invalid_bounds.connect(&|_| {});

        assert_eq!(invalid_bounds_api.value_text(), "0% complete");
        assert_eq!(
            invalid_bounds_api
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("0")
        );

        let mut invalid_complete = service(
            Props::new()
                .id("progress")
                .default_value(50.0)
                .min(100.0)
                .max(0.0),
        );

        drop(invalid_complete.send(Event::Complete));

        assert_eq!(invalid_complete.state(), &State::Idle);
        assert_eq!(invalid_complete.context().percent, 0.0);
    }

    #[test]
    fn progress_syncs_value_control_mode_changes() {
        let mut uncontrolled = service(Props::new().id("progress").default_value(20.0));

        assert!(!uncontrolled.context().value.is_controlled());

        drop(uncontrolled.set_props(Props::new().id("progress").value(Some(50.0))));

        assert!(uncontrolled.context().value.is_controlled());
        assert_eq!(uncontrolled.context().value.get(), &Some(50.0));

        drop(uncontrolled.send(Event::Reset));

        assert!(uncontrolled.context().value.is_controlled());
        assert_eq!(uncontrolled.context().value.get(), &Some(50.0));
        assert!(!uncontrolled.context().indeterminate);

        let mut controlled = service(Props::new().id("progress").value(Some(50.0)));

        assert!(controlled.context().value.is_controlled());

        drop(controlled.set_props(Props::new().id("progress").default_value(15.0)));

        assert!(!controlled.context().value.is_controlled());
        assert_eq!(controlled.context().value.get(), &Some(15.0));

        drop(controlled.send(Event::SetValue(Some(40.0))));
        drop(controlled.set_props(Props::new().id("progress").default_value(15.0).max(200.0)));

        assert!(!controlled.context().value.is_controlled());
        assert_eq!(controlled.context().value.get(), &Some(40.0));
        assert_eq!(controlled.context().percent, 20.0);
    }

    #[test]
    fn progress_complete_event_sets_public_completion_consistently() {
        let mut controlled = service(Props::new().id("progress").value(Some(50.0)));

        drop(controlled.send(Event::Complete));

        let controlled_api = controlled.connect(&|_| {});

        assert_eq!(controlled.state(), &State::Complete);
        assert_eq!(controlled.context().value.get(), &Some(100.0));
        assert_eq!(controlled_api.percent(), 100.0);
        assert_eq!(controlled_api.value_text(), "Complete");
        assert_eq!(
            controlled_api
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("100")
        );

        let mut controlled_indeterminate = service(Props::new().id("progress").value(None));

        drop(controlled_indeterminate.send(Event::Complete));

        let controlled_indeterminate_api = controlled_indeterminate.connect(&|_| {});

        assert_eq!(controlled_indeterminate.state(), &State::Complete);
        assert!(!controlled_indeterminate_api.is_indeterminate());
        assert_eq!(
            controlled_indeterminate_api
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("100")
        );
    }
}
