//! DateRangeField component state machine and connect API.
//!
//! This module implements the framework-agnostic two-field date range input
//! defined in `spec/components/date-time/date-range-field.md`. It composes two
//! [`date_field`] instances (start and end) with shared range coordination,
//! min/max validation, focus-field tracking, and form integration. There is no
//! popover — the two fields render inline with a visual separator between them.

#[cfg(test)]
mod tests;

use alloc::{format, string::String, sync::Arc, vec::Vec};
use core::{
    cmp::Ordering,
    fmt::{self, Debug},
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, IntlBackend, Locale, MessageFn, NoEffect, TransitionPlan,
};
use ars_i18n::{CalendarDate, DateRange};

use super::date_field;

/// Identifies which of the two child fields is currently active.
///
/// Owned by `DateRangeField` as the first range component to require it; later
/// range components (e.g. `DateRangePicker`) reuse this enum rather than
/// redefining it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveField {
    /// The start field is active.
    Start,

    /// The end field is active.
    End,
}

/// States for the `DateRangeField` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Neither field holds focus.
    Idle,

    /// The start field holds focus.
    StartFocused,

    /// The end field holds focus.
    EndFocused,
}

/// Events for the `DateRangeField` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Focus moved to the start field.
    FocusStart,

    /// Focus moved to the end field.
    FocusEnd,

    /// Focus left the entire component.
    BlurAll,

    /// Set the full range programmatically.
    SetRange(Option<DateRange>),

    /// The start field's value changed.
    StartValueChange(Option<CalendarDate>),

    /// The end field's value changed.
    EndValueChange(Option<CalendarDate>),
}

/// Context for the `DateRangeField` component.
#[derive(Clone)]
pub struct Context {
    /// Controlled or uncontrolled range value.
    ///
    /// This is the *derived complete* range: it is `Some` only when both
    /// [`start_date`](Self::start_date) and [`end_date`](Self::end_date) are
    /// set, and is always normalized so `start <= end`.
    pub value: Bindable<Option<DateRange>>,

    /// The start field's current value, tracked independently so a range can be
    /// assembled incrementally as each field changes.
    pub start_date: Option<CalendarDate>,

    /// The end field's current value, tracked independently so a range can be
    /// assembled incrementally as each field changes.
    pub end_date: Option<CalendarDate>,

    /// The active field, if either currently holds focus.
    pub active_field: Option<ActiveField>,

    /// The minimum selectable date applied to both fields.
    pub min: Option<CalendarDate>,

    /// The maximum selectable date applied to both fields.
    pub max: Option<CalendarDate>,

    /// The resolved locale.
    pub locale: Locale,

    /// Backend used for locale-dependent labels in range descriptions.
    pub intl_backend: Arc<dyn IntlBackend>,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Whether the component is required for form submission.
    pub required: bool,

    /// Form field name for a single hidden input carrying the range value.
    pub name: Option<String>,

    /// Form field name for a separate hidden input carrying the start date.
    pub start_name: Option<String>,

    /// Form field name for a separate hidden input carrying the end date.
    pub end_name: Option<String>,

    /// Derived component part ids.
    pub ids: ComponentIds,

    /// Whether the `Description` part is rendered.
    pub has_description: bool,

    /// Whether the `ErrorMessage` part is rendered.
    pub has_error_message: bool,

    /// When `true`, numeric segments in both child fields display with leading
    /// zeros (e.g. "03" instead of "3").
    pub force_leading_zeros: bool,
}

impl Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("value", &self.value)
            .field("start_date", &self.start_date)
            .field("end_date", &self.end_date)
            .field("active_field", &self.active_field)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("locale", &self.locale)
            .field("intl_backend", &"<dyn IntlBackend>")
            .field("messages", &self.messages)
            .field("disabled", &self.disabled)
            .field("readonly", &self.readonly)
            .field("required", &self.required)
            .field("name", &self.name)
            .field("start_name", &self.start_name)
            .field("end_name", &self.end_name)
            .field("ids", &self.ids)
            .field("has_description", &self.has_description)
            .field("has_error_message", &self.has_error_message)
            .field("force_leading_zeros", &self.force_leading_zeros)
            .finish()
    }
}

impl Context {
    /// Returns `true` when the current range violates the configured min/max
    /// bounds.
    ///
    /// The stored range always satisfies `start <= end` (it is normalized on
    /// every value change), so the only remaining source of invalidity is a
    /// start earlier than `min` or an end later than `max`.
    #[must_use]
    pub fn is_invalid(&self) -> bool {
        let Some(range) = self.value.get() else {
            return false;
        };

        let below_min = self.min.as_ref().is_some_and(|min| {
            matches!(
                range.start.compare_within_calendar(min),
                Some(Ordering::Less)
            )
        });

        let above_max = self.max.as_ref().is_some_and(|max| {
            matches!(
                range.end.compare_within_calendar(max),
                Some(Ordering::Greater)
            )
        });

        below_min || above_max
    }
}

/// Props for the `DateRangeField` component.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// The stable DOM id for the component.
    pub id: String,

    /// The externally controlled value of the component.
    ///
    /// `None` means uncontrolled. `Some(None)` means the parent controls the
    /// component as empty. `Some(Some(range))` means a concrete range.
    pub value: Option<Option<DateRange>>,

    /// The initial value used when the component is uncontrolled.
    pub default_value: Option<DateRange>,

    /// The minimum selectable date.
    pub min: Option<CalendarDate>,

    /// The maximum selectable date.
    pub max: Option<CalendarDate>,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Whether the component is required for form submission.
    pub required: bool,

    /// Form field name for a single hidden input carrying the range value.
    pub name: Option<String>,

    /// Form field name for the start date (alternative to `name`).
    ///
    /// When set, the start date submits as a separate form field.
    pub start_name: Option<String>,

    /// Form field name for the end date (alternative to `name`).
    ///
    /// When set, the end date submits as a separate form field.
    pub end_name: Option<String>,

    /// When `true`, numeric segments in both child fields display with leading
    /// zeros. Defaults to `false`, which uses locale-aware formatting.
    pub force_leading_zeros: bool,

    /// Whether a `Description` element is rendered. When `true`, the root's
    /// `aria-describedby` references the description part id.
    pub has_description: bool,

    /// Whether an `ErrorMessage` element is rendered. When `true`, the root's
    /// `aria-describedby` references the error-message part id.
    pub has_error_message: bool,
}

impl Props {
    /// Returns a fresh [`Props`] value with the `DateRangeField` defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id), the stable DOM id for the component.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`value`](Self::value), the externally controlled range value.
    #[must_use]
    pub fn value(mut self, value: Option<DateRange>) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets [`default_value`](Self::default_value), the uncontrolled initial range.
    #[must_use]
    pub fn default_value(mut self, value: Option<DateRange>) -> Self {
        self.default_value = value;
        self
    }

    /// Sets [`min`](Self::min), the minimum selectable date.
    #[must_use]
    pub fn min(mut self, value: Option<CalendarDate>) -> Self {
        self.min = value;
        self
    }

    /// Sets [`max`](Self::max), the maximum selectable date.
    #[must_use]
    pub fn max(mut self, value: Option<CalendarDate>) -> Self {
        self.max = value;
        self
    }

    /// Sets [`disabled`](Self::disabled), whether the component is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`readonly`](Self::readonly), whether the component is read-only.
    #[must_use]
    pub const fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Sets [`required`](Self::required), whether the component is required.
    #[must_use]
    pub const fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Sets [`name`](Self::name), the combined hidden input form name.
    #[must_use]
    pub fn name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    /// Sets [`start_name`](Self::start_name), the separate start-date form name.
    #[must_use]
    pub fn start_name(mut self, name: Option<String>) -> Self {
        self.start_name = name;
        self
    }

    /// Sets [`end_name`](Self::end_name), the separate end-date form name.
    #[must_use]
    pub fn end_name(mut self, name: Option<String>) -> Self {
        self.end_name = name;
        self
    }

    /// Sets [`force_leading_zeros`](Self::force_leading_zeros).
    #[must_use]
    pub const fn force_leading_zeros(mut self, force: bool) -> Self {
        self.force_leading_zeros = force;
        self
    }

    /// Sets [`has_description`](Self::has_description).
    #[must_use]
    pub const fn has_description(mut self, has_description: bool) -> Self {
        self.has_description = has_description;
        self
    }

    /// Sets [`has_error_message`](Self::has_error_message).
    #[must_use]
    pub const fn has_error_message(mut self, has_error_message: bool) -> Self {
        self.has_error_message = has_error_message;
        self
    }
}

/// Closure type for the range description message, given the formatted start
/// and end date strings plus the active locale.
type RangeDescriptionFn = dyn Fn(&str, &str, &Locale) -> String + Send + Sync;

/// Translatable messages for the `DateRangeField` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the start date field.
    pub start_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the end date field.
    pub end_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Text rendered between the start and end fields.
    pub separator_text: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Screen-reader description of the full range, given formatted start and
    /// end date strings.
    pub range_description: MessageFn<RangeDescriptionFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            start_label: MessageFn::static_str("Start date"),
            end_label: MessageFn::static_str("End date"),
            separator_text: MessageFn::static_str(" \u{2013} "),
            range_description: MessageFn::new(|start: &str, end: &str, _locale: &Locale| {
                format!("{start} to {end}")
            }),
        }
    }
}

impl ComponentMessages for Messages {}

/// State machine for the `DateRangeField` component.
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
        let value = if let Some(value) = &props.value {
            Bindable::controlled(value.clone())
        } else {
            Bindable::uncontrolled(props.default_value.clone())
        };

        let initial_range = value.get().clone();
        let start_date = initial_range.as_ref().map(|range| range.start.clone());
        let end_date = initial_range.as_ref().map(|range| range.end.clone());

        let ctx = Context {
            value,
            start_date,
            end_date,
            active_field: None,
            min: props.min.clone(),
            max: props.max.clone(),
            locale: env.locale.clone(),
            intl_backend: Arc::clone(&env.intl_backend),
            messages: messages.clone(),
            disabled: props.disabled,
            readonly: props.readonly,
            required: props.required,
            name: props.name.clone(),
            start_name: props.start_name.clone(),
            end_name: props.end_name.clone(),
            ids: ComponentIds::from_id(&props.id),
            has_description: props.has_description,
            has_error_message: props.has_error_message,
            force_leading_zeros: props.force_leading_zeros,
        };

        (State::Idle, ctx)
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled {
            return None;
        }

        match event {
            Event::FocusStart => Some(TransitionPlan::to(State::StartFocused).apply(
                |ctx: &mut Context| {
                    ctx.active_field = Some(ActiveField::Start);
                },
            )),

            Event::FocusEnd => Some(TransitionPlan::to(State::EndFocused).apply(
                |ctx: &mut Context| {
                    ctx.active_field = Some(ActiveField::End);
                },
            )),

            Event::BlurAll => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                ctx.active_field = None;
            })),

            Event::SetRange(range) => {
                if ctx.readonly {
                    return None;
                }

                let range = range.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.start_date = range.as_ref().map(|range| range.start.clone());
                    ctx.end_date = range.as_ref().map(|range| range.end.clone());
                    ctx.value.set(range);
                }))
            }

            Event::StartValueChange(date) => {
                if ctx.readonly {
                    return None;
                }

                let date = date.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    apply_start_change(ctx, date);
                }))
            }

            Event::EndValueChange(date) => {
                if ctx.readonly {
                    return None;
                }

                let date = date.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    apply_end_change(ctx, date);
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
        Api::new(state, ctx, props, send)
    }
}

/// Applies a start-date change, then recomputes the derived range.
fn apply_start_change(ctx: &mut Context, start: Option<CalendarDate>) {
    ctx.start_date = start;

    recompute_range(ctx);
}

/// Applies an end-date change, then recomputes the derived range.
fn apply_end_change(ctx: &mut Context, end: Option<CalendarDate>) {
    ctx.end_date = end;

    recompute_range(ctx);
}

/// Recomputes the derived complete range from the two field values.
///
/// When both fields hold a value the range is normalized (swapping if needed so
/// `start <= end`) and the normalized order is reflected back into the field
/// values. When either field is empty the range is incomplete and the value is
/// cleared.
fn recompute_range(ctx: &mut Context) {
    match (ctx.start_date.clone(), ctx.end_date.clone()) {
        (Some(start), Some(end)) => {
            if let Some(range) = DateRange::normalized(start, end) {
                ctx.start_date = Some(range.start.clone());
                ctx.end_date = Some(range.end.clone());
                ctx.value.set(Some(range));
            }
        }

        _ => ctx.value.set(None),
    }
}

/// Structural parts exposed by the `DateRangeField` connect API.
#[derive(ComponentPart)]
#[scope = "date-range-field"]
pub enum Part {
    /// The root container grouping both fields.
    Root,

    /// The shared label for the entire range.
    Label,

    /// The wrapper around the start `DateField`.
    StartField,

    /// The visual separator between the two fields.
    Separator,

    /// The wrapper around the end `DateField`.
    EndField,

    /// The optional description element.
    Description,

    /// The optional error message element.
    ErrorMessage,

    /// The combined hidden form input.
    HiddenInput,
}

/// API for the `DateRangeField` component.
pub struct Api<'a> {
    /// The state of the component.
    state: &'a State,

    /// The context of the component.
    ctx: &'a Context,

    /// The props of the component.
    props: &'a Props,

    /// The send function.
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

impl<'a> Api<'a> {
    /// Creates a `DateRangeField` connect API from machine state.
    #[must_use]
    pub const fn new(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Self {
        Self {
            state,
            ctx,
            props,
            send,
        }
    }

    // ── AttrMap getters ──────────────────────────────────────────────────

    /// Returns attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Role, "group")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            )
            .set(HtmlAttr::Data("ars-state"), self.state_name());

        let mut described_by = Vec::new();

        if self.ctx.has_description {
            described_by.push(self.ctx.ids.part("description"));
        }

        if self.ctx.has_error_message {
            described_by.push(self.ctx.ids.part("error-message"));
        }

        if !described_by.is_empty() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                described_by.join(" "),
            );
        }

        if self.ctx.disabled {
            attrs
                .set_bool(HtmlAttr::Data("ars-disabled"), true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs
                .set_bool(HtmlAttr::Data("ars-readonly"), true)
                .set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.is_invalid() {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        attrs
    }

    /// Returns attributes for the shared label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("label"));

        attrs
    }

    /// Returns marker attributes for the start-field wrapper.
    ///
    /// The wrapper carries only the scope/part data hooks; the embedded child
    /// `DateField` is configured through [`Api::start_field_props`].
    #[must_use]
    pub fn start_field_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StartField.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Returns marker attributes for the end-field wrapper.
    ///
    /// The wrapper carries only the scope/part data hooks; the embedded child
    /// `DateField` is configured through [`Api::end_field_props`].
    #[must_use]
    pub fn end_field_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::EndField.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the separator element.
    #[must_use]
    pub fn separator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Separator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Returns the separator text rendered between the fields (e.g. " – ").
    #[must_use]
    pub fn separator_text(&self) -> String {
        (self.ctx.messages.separator_text)(&self.ctx.locale)
    }

    /// Returns a screen-reader description of the full range (e.g. "March 1,
    /// 2025 to March 15, 2025"), or `None` if no complete range is selected.
    #[must_use]
    pub fn range_description(&self) -> Option<String> {
        let range = self.ctx.value.get().as_ref()?;

        let start = format_date_label(
            &range.start,
            self.ctx.intl_backend.as_ref(),
            &self.ctx.locale,
        );

        let end = format_date_label(&range.end, self.ctx.intl_backend.as_ref(), &self.ctx.locale);

        Some((self.ctx.messages.range_description)(
            &start,
            &end,
            &self.ctx.locale,
        ))
    }

    /// Builds the child [`date_field::Props`] for the start date input.
    ///
    /// The start field's lower bound is the global `min`; its upper bound is the
    /// current end date when a range is selected, otherwise the global `max`, so
    /// both global bounds and cross-field coordination apply.
    #[must_use]
    pub fn start_field_props(&self) -> date_field::Props {
        let max_value = self.ctx.end_date.clone().or_else(|| self.ctx.max.clone());

        date_field::Props {
            id: self.ctx.ids.part("start"),
            value: Some(self.ctx.start_date.clone()),
            min_value: self.ctx.min.clone(),
            max_value,
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            required: self.ctx.required,
            aria_label: Some((self.ctx.messages.start_label)(&self.ctx.locale)),
            force_leading_zeros: self.ctx.force_leading_zeros,
            ..date_field::Props::default()
        }
    }

    /// Builds the child [`date_field::Props`] for the end date input.
    ///
    /// The end field's upper bound is the global `max`; its lower bound is the
    /// current start date when a range is selected, otherwise the global `min`.
    #[must_use]
    pub fn end_field_props(&self) -> date_field::Props {
        let min_value = self.ctx.start_date.clone().or_else(|| self.ctx.min.clone());

        date_field::Props {
            id: self.ctx.ids.part("end"),
            value: Some(self.ctx.end_date.clone()),
            min_value,
            max_value: self.ctx.max.clone(),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            required: self.ctx.required,
            aria_label: Some((self.ctx.messages.end_label)(&self.ctx.locale)),
            force_leading_zeros: self.ctx.force_leading_zeros,
            ..date_field::Props::default()
        }
    }

    /// Returns attributes for the description element.
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

    /// Returns attributes for the error message element.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Role, "alert");

        attrs
    }

    /// Returns attributes for the combined hidden input element.
    ///
    /// The value is the ISO 8601 interval `YYYY-MM-DD/YYYY-MM-DD`, or empty when
    /// no range is selected.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "hidden");

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }

        let value = if let Some(range) = self.ctx.value.get() {
            range.to_iso8601()
        } else {
            String::new()
        };

        attrs.set(HtmlAttr::Value, value);

        attrs
    }

    /// Returns attributes for the separate hidden input carrying the start date.
    ///
    /// Only meaningful when `start_name` is set on `Props`.
    #[must_use]
    pub fn start_hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Type, "hidden");

        if let Some(name) = &self.ctx.start_name {
            attrs.set(HtmlAttr::Name, name);
        }

        let value = if let Some(range) = self.ctx.value.get() {
            range.start.to_iso8601()
        } else {
            String::new()
        };

        attrs.set(HtmlAttr::Value, value);

        attrs
    }

    /// Returns attributes for the separate hidden input carrying the end date.
    ///
    /// Only meaningful when `end_name` is set on `Props`.
    #[must_use]
    pub fn end_hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Type, "hidden");

        if let Some(name) = &self.ctx.end_name {
            attrs.set(HtmlAttr::Name, name);
        }

        let value = if let Some(range) = self.ctx.value.get() {
            range.end.to_iso8601()
        } else {
            String::new()
        };

        attrs.set(HtmlAttr::Value, value);

        attrs
    }

    // ── Event dispatch (called by adapters) ──────────────────────────────

    /// Notifies the machine that focus moved to the start field.
    pub fn focus_start(&self) {
        (self.send)(Event::FocusStart);
    }

    /// Notifies the machine that focus moved to the end field.
    pub fn focus_end(&self) {
        (self.send)(Event::FocusEnd);
    }

    /// Notifies the machine that focus left the entire component.
    pub fn blur(&self) {
        (self.send)(Event::BlurAll);
    }

    /// Sets the full range programmatically.
    pub fn set_range(&self, range: Option<DateRange>) {
        (self.send)(Event::SetRange(range));
    }

    /// Reports a start-date change from the start field.
    pub fn set_start_value(&self, date: Option<CalendarDate>) {
        (self.send)(Event::StartValueChange(date));
    }

    /// Reports an end-date change from the end field.
    pub fn set_end_value(&self, date: Option<CalendarDate>) {
        (self.send)(Event::EndValueChange(date));
    }

    // ── Convenience getters ──────────────────────────────────────────────

    /// Returns the currently selected range, if any.
    #[must_use]
    pub fn selected_range(&self) -> Option<&DateRange> {
        self.ctx.value.get().as_ref()
    }

    /// Returns the active field, if either currently holds focus.
    #[must_use]
    pub const fn active_field(&self) -> Option<ActiveField> {
        self.ctx.active_field
    }

    /// Returns `true` when either field currently holds focus.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        !matches!(self.state, State::Idle)
    }

    /// Returns `true` when the current range violates the min/max bounds.
    #[must_use]
    pub fn is_invalid(&self) -> bool {
        self.ctx.is_invalid()
    }

    /// Returns the stable state token used for `data-ars-state`.
    #[must_use]
    pub const fn state_name(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::StartFocused => "start-focused",
            State::EndFocused => "end-focused",
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::StartField => self.start_field_attrs(),
            Part::Separator => self.separator_attrs(),
            Part::EndField => self.end_field_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

/// Formats a single date as a human-readable label for screen-reader range
/// descriptions (e.g. "March 1, 2025").
fn format_date_label(date: &CalendarDate, backend: &dyn IntlBackend, locale: &Locale) -> String {
    format!(
        "{} {}, {}",
        backend.month_long_name(date.month(), locale),
        date.day(),
        date.year(),
    )
}
