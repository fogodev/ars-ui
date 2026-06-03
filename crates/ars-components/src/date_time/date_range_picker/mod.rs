//! `DateRangePicker` component state machine and connect API.
//!
//! Framework-agnostic implementation of the composite date range picker defined
//! in `spec/components/date-time/date-range-picker.md`. The machine pairs two
//! [`DateField`](super::date_field) inputs (start and end) with an embedded
//! [`RangeCalendar`](super::range_calendar) shown in a popover, and owns the
//! popover open/close lifecycle, the canonical selected range, per-field value
//! coordination, preset ranges, form integration, and the ARIA/`data-ars-*`
//! attribute surface.
//!
//! ## Spec-vs-implementation reconciliation
//!
//! The spec's §1 code examples predate the finalized sibling components. This
//! implementation keeps the spec's semantics while using the real APIs:
//!
//! - **Value-based fields, not text parsing.** A [`DateField`](super::date_field)
//!   is a *segmented* input that emits an [`Option<CalendarDate>`], not a raw
//!   string, and has no `format` string. The spec's `StartInputChange { value:
//!   String }` / `format: DateFormat` / in-core `parse_date` are replaced by
//!   [`Event::StartValueChange`]/[`Event::EndValueChange`] carrying
//!   `Option<CalendarDate>` and per-field `start_date`/`end_date`, mirroring the
//!   proven [`date_range_field`](super::date_range_field).
//! - **Dedicated [`RangeCalendar`](super::range_calendar).** Range selection
//!   lives in its own component; [`calendar`](super::calendar) has no `is_range`
//!   mode. [`Api::range_calendar_props`] builds [`range_calendar::Props`], and
//!   the adapter bridges its completed-range change back as
//!   [`Event::SelectRangeComplete`].
//! - **Real ISO helpers.** Form values use [`DateRange::to_iso8601`] (the
//!   `"start/end"` interval) and [`CalendarDate::to_iso8601`]; there is no
//!   `to_iso_string`.
//! - **`NoEffect`, adapter-owned focus.** Live focus, popover positioning, and
//!   return-focus are driven by the adapter from element handles per the issue's
//!   element/ref note; the agnostic core emits no effects and exposes state via
//!   the connect API (mirrors [`date_range_field`](super::date_range_field)).
//! - **Controlled-prop sync.** A [`Event::SyncProps`] event plus
//!   [`Machine::on_props_changed`](ars_core::Machine::on_props_changed) keep a
//!   controlled `value` and the cached `min`/`max`/`today`/`presets`/flags live.
//! - **Presets.** [`Preset`] adds named, pre-localized ranges (selected via
//!   [`Event::SelectPreset`] / the [`Part::PresetTrigger`] anatomy part) so a
//!   consumer can offer "Last 7 days"-style shortcuts. The consumer supplies a
//!   concrete [`DateRange`] computed relative to the same injected `today`,
//!   keeping the core deterministic.

#[cfg(test)]
mod tests;

use alloc::{
    boxed::Box,
    string::{String, ToString as _},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, IntlBackend, Locale, MessageFn, NoEffect, TransitionPlan,
};
use ars_i18n::{CalendarDate, DateRange};
use ars_interactions::KeyboardKey;

// `ActiveField` is owned by `date_range_field` as the first range component to
// need it; per its own documentation, later range components reuse the enum
// rather than redefining it.
pub use super::date_range_field::ActiveField;
use super::{
    date_field,
    date_range_field::{date_out_of_bounds, format_date_label},
    range_calendar,
};

// ────────────────────────────────────────────────────────────────────
// State / Event
// ────────────────────────────────────────────────────────────────────

/// States for the `DateRangePicker` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// The calendar popover is closed.
    Closed,

    /// The calendar popover is open.
    Open,
}

/// Events for the `DateRangePicker` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open the calendar popover.
    Open,

    /// Close the calendar popover.
    Close,

    /// Toggle the popover open/closed.
    Toggle,

    /// The embedded calendar completed a range selection.
    SelectRangeComplete {
        /// The selected range.
        range: DateRange,
    },

    /// The start field's value changed (segmented `DateField` emits a date, not
    /// raw text).
    StartValueChange(Option<CalendarDate>),

    /// The end field's value changed.
    EndValueChange(Option<CalendarDate>),

    /// A preset range was chosen by its index into [`Props::presets`].
    SelectPreset {
        /// Zero-based index into the configured presets.
        index: usize,
    },

    /// Clear the selected range (the clear trigger).
    Clear,

    /// Focus entered the component.
    FocusIn,

    /// Focus left the component entirely.
    FocusOut,

    /// A key was pressed on the trigger or within the popover.
    KeyDown {
        /// The key that was pressed.
        key: KeyboardKey,
    },

    /// Re-apply context-backed prop fields after a props change.
    ///
    /// Emitted by [`Machine::on_props_changed`](ars_core::Machine::on_props_changed)
    /// so a controlled `value` and the cached `min`/`max`/`today`/`presets`/
    /// flags follow parent-driven prop updates.
    SyncProps(Box<Props>),
}

// ────────────────────────────────────────────────────────────────────
// Preset
// ────────────────────────────────────────────────────────────────────

/// A named preset range offered as a one-click shortcut (e.g. "Last 7 days").
///
/// The consumer supplies an already-localized [`label`](Self::label) and a
/// concrete [`range`](Self::range). Relative presets (such as "Last 7 days")
/// are computed by the consumer against the same `today` injected into
/// [`Props::today`], which keeps the agnostic core deterministic and free of
/// closures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preset {
    /// The pre-localized label shown on the preset trigger.
    pub label: String,

    /// The concrete range applied when this preset is selected.
    pub range: DateRange,
}

impl Preset {
    /// Creates a preset from a label and a concrete range.
    #[must_use]
    pub fn new(label: impl Into<String>, range: DateRange) -> Self {
        Self {
            label: label.into(),
            range,
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Props for the `DateRangePicker` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// The stable DOM id for the component. [`ComponentIds`] derives part ids
    /// from it.
    pub id: String,

    /// Controlled range value. `None` = uncontrolled, `Some(None)` =
    /// controlled-and-empty, `Some(Some(range))` = controlled with a range.
    pub value: Option<Option<DateRange>>,

    /// The initial range used when the component is uncontrolled.
    pub default_value: Option<DateRange>,

    /// The minimum selectable date (forwarded to both fields and the calendar).
    pub min: Option<CalendarDate>,

    /// The maximum selectable date (forwarded to both fields and the calendar).
    pub max: Option<CalendarDate>,

    /// The "today" date, injected by the adapter for testability and SSR
    /// determinism. Forwarded to the embedded [`RangeCalendar`](super::range_calendar)
    /// so an empty picker opens on the current month. Defaults to a fixed date;
    /// adapters inject the real today.
    pub today: CalendarDate,

    /// Named preset ranges offered as one-click shortcuts (rendered as
    /// [`Part::PresetTrigger`] items inside the popover).
    pub presets: Vec<Preset>,

    /// Number of months displayed side-by-side in the calendar popover.
    /// Default: `2`. Forwarded to the embedded calendar's `visible_months`.
    pub visible_months: usize,

    /// Right-to-left layout direction (forwarded to the embedded calendar).
    pub is_rtl: bool,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only (viewing allowed, editing blocked).
    pub readonly: bool,

    /// Whether the component is required for form submission.
    pub required: bool,

    /// When `true`, numeric segments in both child fields display with leading
    /// zeros. Defaults to `false`, which uses locale-aware formatting.
    pub force_leading_zeros: bool,

    /// Whether a `Description` element is rendered. When `true`, the control's
    /// `aria-describedby` references the description part id.
    pub has_description: bool,

    /// Whether an `ErrorMessage` element is rendered. When `true`, the control's
    /// `aria-describedby` references the error-message part id.
    pub has_error_message: bool,

    /// Form field name for a single hidden input carrying the range value as the
    /// ISO 8601 interval `YYYY-MM-DD/YYYY-MM-DD`.
    pub name: Option<String>,

    /// Form field name for a separate hidden input carrying the start date
    /// (alternative to [`name`](Self::name)).
    pub start_name: Option<String>,

    /// Form field name for a separate hidden input carrying the end date.
    pub end_name: Option<String>,

    /// Whether to close the popover after a range is completed. Default: `true`.
    pub close_on_select: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            min: None,
            max: None,
            today: CalendarDate::new_gregorian(2025, 1, 1)
                .expect("2025-01-01 is a valid Gregorian date"),
            presets: Vec::new(),
            visible_months: 2,
            is_rtl: false,
            disabled: false,
            readonly: false,
            required: false,
            force_leading_zeros: false,
            has_description: false,
            has_error_message: false,
            name: None,
            start_name: None,
            end_name: None,
            close_on_select: true,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] value with the `DateRangePicker` defaults.
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

    /// Sets [`today`](Self::today), the adapter-injected current date.
    #[must_use]
    pub fn today(mut self, today: CalendarDate) -> Self {
        self.today = today;
        self
    }

    /// Sets [`presets`](Self::presets), the named preset ranges.
    #[must_use]
    pub fn presets(mut self, presets: Vec<Preset>) -> Self {
        self.presets = presets;
        self
    }

    /// Sets [`visible_months`](Self::visible_months), the side-by-side month count.
    #[must_use]
    pub const fn visible_months(mut self, visible_months: usize) -> Self {
        self.visible_months = visible_months;
        self
    }

    /// Sets [`is_rtl`](Self::is_rtl), the layout direction.
    #[must_use]
    pub const fn is_rtl(mut self, is_rtl: bool) -> Self {
        self.is_rtl = is_rtl;
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

    /// Sets [`close_on_select`](Self::close_on_select).
    #[must_use]
    pub const fn close_on_select(mut self, close_on_select: bool) -> Self {
        self.close_on_select = close_on_select;
        self
    }
}

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// Context for the `DateRangePicker` component.
#[derive(Clone)]
pub struct Context {
    /// The canonical selected range.
    ///
    /// `Some` only when both [`start_date`](Self::start_date) and
    /// [`end_date`](Self::end_date) are set; always normalized so `start <= end`.
    pub value: Bindable<Option<DateRange>>,

    /// Whether the popover is open (always uncontrolled).
    pub open: Bindable<bool>,

    /// The start field's current value, tracked independently so a range can be
    /// assembled incrementally as each field changes.
    pub start_date: Option<CalendarDate>,

    /// The end field's current value, tracked independently.
    pub end_date: Option<CalendarDate>,

    /// Which field most recently received a value change.
    pub active_field: ActiveField,

    /// The minimum selectable date.
    pub min: Option<CalendarDate>,

    /// The maximum selectable date.
    pub max: Option<CalendarDate>,

    /// The adapter-injected "today" date, forwarded to the embedded calendar.
    pub today: CalendarDate,

    /// Named preset ranges offered as one-click shortcuts.
    pub presets: Vec<Preset>,

    /// Number of months displayed side-by-side, forwarded to the calendar.
    pub visible_months: usize,

    /// Right-to-left layout direction, forwarded to the calendar.
    pub is_rtl: bool,

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

    /// When `true`, numeric segments in both child fields display with leading
    /// zeros.
    pub force_leading_zeros: bool,

    /// Whether the `Description` part is rendered.
    pub has_description: bool,

    /// Whether the `ErrorMessage` part is rendered.
    pub has_error_message: bool,

    /// Form field name for a single hidden input carrying the range value.
    pub name: Option<String>,

    /// Form field name for a separate hidden input carrying the start date.
    pub start_name: Option<String>,

    /// Form field name for a separate hidden input carrying the end date.
    pub end_name: Option<String>,

    /// Derived component part ids.
    pub ids: ComponentIds,
}

impl Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("value", &self.value)
            .field("open", &self.open)
            .field("start_date", &self.start_date)
            .field("end_date", &self.end_date)
            .field("active_field", &self.active_field)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("today", &self.today)
            .field("presets", &self.presets)
            .field("visible_months", &self.visible_months)
            .field("is_rtl", &self.is_rtl)
            .field("locale", &self.locale)
            .field("intl_backend", &"<dyn IntlBackend>")
            .field("messages", &self.messages)
            .field("disabled", &self.disabled)
            .field("readonly", &self.readonly)
            .field("required", &self.required)
            .field("force_leading_zeros", &self.force_leading_zeros)
            .field("has_description", &self.has_description)
            .field("has_error_message", &self.has_error_message)
            .field("name", &self.name)
            .field("start_name", &self.start_name)
            .field("end_name", &self.end_name)
            .field("ids", &self.ids)
            .finish()
    }
}

impl Context {
    /// Returns `true` when the current range violates the configured min/max
    /// bounds. A normalized range lies within `[min, max]` iff neither endpoint
    /// is out of bounds.
    #[must_use]
    pub fn is_invalid(&self) -> bool {
        let Some(range) = self.value.get() else {
            return false;
        };

        date_out_of_bounds(Some(&range.start), self.min.as_ref(), self.max.as_ref())
            || date_out_of_bounds(Some(&range.end), self.min.as_ref(), self.max.as_ref())
    }
}

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// Closure type for the range description message, given the formatted start
/// and end date strings plus the active locale.
type RangeDescriptionFn = dyn Fn(&str, &str, &Locale) -> String + Send + Sync;

/// Translatable messages for the `DateRangePicker` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the trigger button.
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the start date field.
    pub start_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the end date field.
    pub end_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Screen-reader description of the full range, given formatted start and
    /// end date strings.
    pub range_description: MessageFn<RangeDescriptionFn>,

    /// Accessible label for the clear trigger button.
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Open date range picker"),
            start_label: MessageFn::static_str("Start date"),
            end_label: MessageFn::static_str("End date"),
            range_description: MessageFn::new(|start: &str, end: &str, _locale: &Locale| {
                alloc::format!("{start} to {end}")
            }),
            clear_label: MessageFn::static_str("Clear date range"),
        }
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// State machine for the `DateRangePicker` component.
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
            open: Bindable::uncontrolled(false),
            start_date,
            end_date,
            active_field: ActiveField::Start,
            min: props.min.clone(),
            max: props.max.clone(),
            today: props.today.clone(),
            presets: props.presets.clone(),
            visible_months: props.visible_months,
            is_rtl: props.is_rtl,
            locale: env.locale.clone(),
            intl_backend: Arc::clone(&env.intl_backend),
            messages: messages.clone(),
            disabled: props.disabled,
            readonly: props.readonly,
            required: props.required,
            force_leading_zeros: props.force_leading_zeros,
            has_description: props.has_description,
            has_error_message: props.has_error_message,
            name: props.name.clone(),
            start_name: props.start_name.clone(),
            end_name: props.end_name.clone(),
            ids: ComponentIds::from_id(&props.id),
        };

        (State::Closed, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // `SyncProps` must process even when disabled so adapter-driven prop
        // updates (including re-enabling) still reach the context.
        if ctx.disabled && !matches!(event, Event::SyncProps(_)) {
            return None;
        }

        match (state, event) {
            (_, Event::SyncProps(next)) => {
                let next = next.as_ref().clone();

                // Disabling an open picker must dismiss it: while disabled the
                // guard above blocks Close/Escape/FocusOut, so a popover left
                // open here could never be closed again.
                if next.disabled && *state == State::Open {
                    return Some(TransitionPlan::to(State::Closed).apply(
                        move |ctx: &mut Context| {
                            sync_props(ctx, &next);
                            ctx.open.set(false);
                        },
                    ));
                }

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    sync_props(ctx, &next);
                }))
            }

            (State::Closed, Event::Open | Event::Toggle) => {
                Some(TransitionPlan::to(State::Open).apply(|ctx: &mut Context| {
                    ctx.open.set(true);
                }))
            }

            (State::Open, Event::Close | Event::Toggle) => Some(
                TransitionPlan::to(State::Closed).apply(|ctx: &mut Context| {
                    ctx.open.set(false);
                }),
            ),

            (_, Event::SelectRangeComplete { range }) => {
                if ctx.readonly {
                    return None;
                }

                // Accept the completed range regardless of open state: a browser
                // can fire `FocusOut` (closing the popover) before the calendar
                // reports the cell click, and the selection must not be dropped.
                // The close side-effect only applies when the picker was open.
                let range = range.clone();

                let should_close = props.close_on_select && *state == State::Open;

                let next_state = if should_close {
                    State::Closed
                } else {
                    state.clone()
                };

                Some(
                    TransitionPlan::to(next_state).apply(move |ctx: &mut Context| {
                        apply_complete_range(ctx, range, should_close);
                    }),
                )
            }

            (_, Event::SelectPreset { index }) => {
                if ctx.readonly {
                    return None;
                }

                let range = ctx.presets.get(*index)?.range.clone();
                // A preset always completes a full range; close iff configured
                // and currently open (closing a closed popover is a no-op).
                let should_close = props.close_on_select && *state == State::Open;

                let next_state = if should_close {
                    State::Closed
                } else {
                    state.clone()
                };

                Some(
                    TransitionPlan::to(next_state).apply(move |ctx: &mut Context| {
                        apply_complete_range(ctx, range, should_close);
                    }),
                )
            }

            (_, Event::StartValueChange(date)) => {
                if ctx.readonly {
                    return None;
                }

                let date = date.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.active_field = ActiveField::Start;
                    ctx.start_date = date;
                    recompute_range(ctx);
                }))
            }

            (_, Event::EndValueChange(date)) => {
                if ctx.readonly {
                    return None;
                }

                let date = date.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.active_field = ActiveField::End;
                    ctx.end_date = date;
                    recompute_range(ctx);
                }))
            }

            (_, Event::Clear) => {
                if ctx.readonly {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.start_date = None;
                    ctx.end_date = None;
                    ctx.value.set(None);
                }))
            }

            (State::Open, Event::KeyDown { key }) if *key == KeyboardKey::Escape => {
                Self::transition(state, &Event::Close, ctx, props)
            }

            (State::Closed, Event::KeyDown { key }) if *key == KeyboardKey::ArrowDown => {
                Self::transition(state, &Event::Open, ctx, props)
            }

            (State::Open, Event::FocusOut) => Self::transition(state, &Event::Close, ctx, props),

            _ => None,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        debug_assert_eq!(
            old.id, new.id,
            "date_range_picker::Props.id must remain stable after init"
        );

        if old == new {
            Vec::new()
        } else {
            vec![Event::SyncProps(Box::new(new.clone()))]
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

/// Re-derives mutable context fields from changed props.
///
/// `id` is fixed at construction (see [`Machine::on_props_changed`]). For a
/// controlled component (`props.value` is `Some`) the controlled override and
/// the per-field working values are reconciled to the new range; an uncontrolled
/// component keeps its internal working state and merely drops any stale
/// controlled override. `open` is always uncontrolled and is left untouched.
fn sync_props(ctx: &mut Context, props: &Props) {
    ctx.min = props.min.clone();
    ctx.max = props.max.clone();
    ctx.today = props.today.clone();
    ctx.presets = props.presets.clone();
    ctx.visible_months = props.visible_months;
    ctx.is_rtl = props.is_rtl;
    ctx.disabled = props.disabled;
    ctx.readonly = props.readonly;
    ctx.required = props.required;
    ctx.force_leading_zeros = props.force_leading_zeros;
    ctx.has_description = props.has_description;
    ctx.has_error_message = props.has_error_message;
    ctx.name = props.name.clone();
    ctx.start_name = props.start_name.clone();
    ctx.end_name = props.end_name.clone();

    if let Some(controlled) = &props.value {
        ctx.value.sync_controlled(Some(controlled.clone()));
        // Keep the internal (pending) value in lockstep with the controlled
        // override. `get()` returns the override while controlled, so this is
        // invisible now; it matters if the component later returns to
        // uncontrolled mode (`value` prop dropped to `None`), where the revealed
        // internal value must stay consistent with the per-field values below
        // rather than expose a stale earlier value.
        ctx.value.set(controlled.clone());
        ctx.start_date = controlled.as_ref().map(|range| range.start.clone());
        ctx.end_date = controlled.as_ref().map(|range| range.end.clone());
    } else {
        // Genuinely uncontrolled: drop any stale override but keep the internal
        // working state and per-field values so in-progress edits survive an
        // unrelated prop update (mirrors `date_range_field`).
        ctx.value.sync_controlled(None);
    }
}

/// Applies a completed range to the context: the per-field values, the canonical
/// value, and (when `close`) the open state.
fn apply_complete_range(ctx: &mut Context, range: DateRange, close: bool) {
    ctx.start_date = Some(range.start.clone());
    ctx.end_date = Some(range.end.clone());
    ctx.value.set(Some(range));

    if close {
        ctx.open.set(false);
    }
}

/// Recomputes the canonical range from the two field values.
///
/// When both fields hold a value the range is normalized (swapping if needed so
/// `start <= end`) and the normalized order is reflected back into the field
/// values. When either field is empty, or the two dates are not comparable, the
/// range is cleared.
fn recompute_range(ctx: &mut Context) {
    match (ctx.start_date.clone(), ctx.end_date.clone()) {
        (Some(start), Some(end)) => {
            if let Some(range) = DateRange::normalized(start, end) {
                ctx.start_date = Some(range.start.clone());
                ctx.end_date = Some(range.end.clone());
                ctx.value.set(Some(range));
            } else {
                ctx.value.set(None);
            }
        }
        _ => ctx.value.set(None),
    }
}

// ────────────────────────────────────────────────────────────────────
// Anatomy
// ────────────────────────────────────────────────────────────────────

/// Structural parts exposed by the `DateRangePicker` connect API.
#[derive(ars_core::ComponentPart)]
#[scope = "date-range-picker"]
pub enum Part {
    /// The outermost container.
    Root,

    /// The label pointing at the start input.
    Label,

    /// The group wrapping the two inputs and triggers.
    Control,

    /// The start date input (a `DateField`).
    StartInput,

    /// The visual separator between the two inputs.
    Separator,

    /// The end date input (a `DateField`).
    EndInput,

    /// The button that toggles the calendar popover.
    Trigger,

    /// The button that clears the selected range.
    ClearTrigger,

    /// A preset shortcut button. The index selects into [`Props::presets`].
    PresetTrigger {
        /// Zero-based index into the configured presets.
        index: usize,
    },

    /// The floating positioner for the popover content.
    Positioner,

    /// The popover content (`role="dialog"`) containing the calendar.
    Content,

    /// The optional description element.
    Description,

    /// The optional error message element.
    ErrorMessage,

    /// The hidden form input carrying the ISO 8601 range value.
    HiddenInput,
}

// ────────────────────────────────────────────────────────────────────
// Connect API
// ────────────────────────────────────────────────────────────────────

/// API for the `DateRangePicker` component.
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
    /// Creates a `DateRangePicker` connect API from machine state.
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
            .set(HtmlAttr::Data("ars-state"), self.state_name());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.ctx.required {
            attrs.set_bool(HtmlAttr::Data("ars-required"), true);
        }

        if self.ctx.is_invalid() {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        attrs
    }

    /// Returns attributes for the label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("label"))
            .set(HtmlAttr::For, self.ctx.ids.part("start-input"));

        attrs
    }

    /// Returns attributes for the control element (the group wrapping the two
    /// inputs and triggers).
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "group")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

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

        attrs
    }

    /// Returns marker attributes for the start-input wrapper.
    ///
    /// The wrapper carries only the scope/part data hooks; the embedded child
    /// `DateField` is configured through [`Api::start_field_props`].
    #[must_use]
    pub fn start_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StartInput.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Returns marker attributes for the end-input wrapper.
    ///
    /// The wrapper carries only the scope/part data hooks; the embedded child
    /// `DateField` is configured through [`Api::end_field_props`].
    #[must_use]
    pub fn end_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::EndInput.data_attrs();

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

    /// Builds the child [`date_field::Props`] for the start date input.
    ///
    /// Both child fields are bounded only by the global `min`/`max`, never by
    /// the opposite endpoint: a child `DateField` clamps completed dates to its
    /// own bounds, so binding the start field to the current end date would clamp
    /// away an out-of-order edit before the parent can perform the normalizing
    /// swap. Cross-field ordering is enforced by the parent's range
    /// recomputation when the changed value reaches it. `invalid` reflects
    /// whether this field's
    /// own value lies outside the global bounds.
    #[must_use]
    pub fn start_field_props(&self) -> date_field::Props {
        date_field::Props {
            id: self.ctx.ids.part("start-input"),
            value: Some(self.ctx.start_date.clone()),
            min_value: self.ctx.min.clone(),
            max_value: self.ctx.max.clone(),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            required: self.ctx.required,
            invalid: date_out_of_bounds(
                self.ctx.start_date.as_ref(),
                self.ctx.min.as_ref(),
                self.ctx.max.as_ref(),
            ),
            aria_label: Some((self.ctx.messages.start_label)(&self.ctx.locale)),
            force_leading_zeros: self.ctx.force_leading_zeros,
            ..date_field::Props::default()
        }
    }

    /// Builds the child [`date_field::Props`] for the end date input.
    ///
    /// Bounded only by the global `min`/`max` for the same reason as
    /// [`Api::start_field_props`]; `invalid` reflects whether the end field's own
    /// value lies outside the global bounds.
    #[must_use]
    pub fn end_field_props(&self) -> date_field::Props {
        date_field::Props {
            id: self.ctx.ids.part("end-input"),
            value: Some(self.ctx.end_date.clone()),
            min_value: self.ctx.min.clone(),
            max_value: self.ctx.max.clone(),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            required: self.ctx.required,
            invalid: date_out_of_bounds(
                self.ctx.end_date.as_ref(),
                self.ctx.min.as_ref(),
                self.ctx.max.as_ref(),
            ),
            aria_label: Some((self.ctx.messages.end_label)(&self.ctx.locale)),
            force_leading_zeros: self.ctx.force_leading_zeros,
            ..date_field::Props::default()
        }
    }

    /// Returns attributes for the trigger element.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("trigger"))
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.trigger_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.is_open() { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.part("content"),
            );

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Returns attributes for the clear trigger element.
    ///
    /// The button is disabled when the component is disabled or read-only, or
    /// when no range is selected — matching the machine, which rejects
    /// [`Event::Clear`] in those states.
    #[must_use]
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.clear_label)(&self.ctx.locale),
            );

        if self.ctx.disabled || self.ctx.readonly || self.ctx.value.get().is_none() {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Returns attributes for the preset trigger at `index`.
    ///
    /// The trigger is disabled when the component is disabled or read-only, or
    /// when the index is out of range — matching the machine, which rejects
    /// [`Event::SelectPreset`] in those states.
    #[must_use]
    pub fn preset_trigger_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::PresetTrigger { index }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Data("ars-index"), index.to_string());

        if self.ctx.disabled || self.ctx.readonly || index >= self.ctx.presets.len() {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Returns attributes for the positioner element.
    #[must_use]
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the content element.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("content"))
            .set(HtmlAttr::Role, "dialog")
            .set(HtmlAttr::Aria(AriaAttr::Modal), "false")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        attrs
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

        // A disabled control is excluded from form submission; mark it disabled
        // rather than submit a stale value (mirrors `date_range_field`).
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
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
    /// Only meaningful when `start_name` is set on `Props`. The value is the
    /// start field's own date, so a partially-entered range still submits the
    /// start date.
    #[must_use]
    pub fn start_hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Type, "hidden");

        if let Some(name) = &self.ctx.start_name {
            attrs.set(HtmlAttr::Name, name);
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        let value = if let Some(start) = &self.ctx.start_date {
            start.to_iso8601()
        } else {
            String::new()
        };

        attrs.set(HtmlAttr::Value, value);

        attrs
    }

    /// Returns attributes for the separate hidden input carrying the end date.
    ///
    /// Only meaningful when `end_name` is set on `Props`. The value is the end
    /// field's own date.
    #[must_use]
    pub fn end_hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Type, "hidden");

        if let Some(name) = &self.ctx.end_name {
            attrs.set(HtmlAttr::Name, name);
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        let value = if let Some(end) = &self.ctx.end_date {
            end.to_iso8601()
        } else {
            String::new()
        };

        attrs.set(HtmlAttr::Value, value);

        attrs
    }

    /// Builds the child [`range_calendar::Props`] for the embedded calendar.
    ///
    /// The calendar reflects the picker's canonical range as a controlled value,
    /// and inherits the picker's bounds, today, visible-month count, and layout
    /// direction. Defaults supply `page_behavior` and the remaining fields.
    #[must_use]
    pub fn range_calendar_props(&self) -> range_calendar::Props {
        range_calendar::Props {
            id: self.ctx.ids.part("calendar"),
            value: Some(self.ctx.value.get().clone()),
            min: self.ctx.min.clone(),
            max: self.ctx.max.clone(),
            today: self.ctx.today.clone(),
            visible_months: self.ctx.visible_months,
            is_rtl: self.ctx.is_rtl,
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            ..range_calendar::Props::default()
        }
    }

    /// Returns a screen-reader description of the full range (e.g. "March 1, 2025
    /// to March 15, 2025"), or `None` if no complete range is selected.
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

    // ── Event dispatch (called by adapters) ──────────────────────────────

    /// Opens the popover.
    pub fn open(&self) {
        (self.send)(Event::Open);
    }

    /// Closes the popover.
    pub fn close(&self) {
        (self.send)(Event::Close);
    }

    /// Toggles the popover.
    pub fn toggle(&self) {
        (self.send)(Event::Toggle);
    }

    /// Clears the selected range.
    pub fn clear(&self) {
        (self.send)(Event::Clear);
    }

    /// Reports a completed range selection from the embedded calendar.
    pub fn select_range(&self, range: DateRange) {
        (self.send)(Event::SelectRangeComplete { range });
    }

    /// Selects the preset at `index`.
    pub fn select_preset(&self, index: usize) {
        (self.send)(Event::SelectPreset { index });
    }

    /// Reports a start-date change from the start field.
    pub fn set_start_value(&self, date: Option<CalendarDate>) {
        (self.send)(Event::StartValueChange(date));
    }

    /// Reports an end-date change from the end field.
    pub fn set_end_value(&self, date: Option<CalendarDate>) {
        (self.send)(Event::EndValueChange(date));
    }

    /// Notifies the machine that focus entered the component.
    pub fn focus_in(&self) {
        (self.send)(Event::FocusIn);
    }

    /// Notifies the machine that focus left the component entirely.
    pub fn focus_out(&self) {
        (self.send)(Event::FocusOut);
    }

    /// Dispatches a keyboard event from the trigger or popover.
    pub fn on_key_down(&self, key: KeyboardKey) {
        (self.send)(Event::KeyDown { key });
    }

    // ── Convenience getters ──────────────────────────────────────────────

    /// Returns `true` when the popover is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self.state, State::Open)
    }

    /// Returns the currently selected range, if any.
    #[must_use]
    pub fn selected_range(&self) -> Option<&DateRange> {
        self.ctx.value.get().as_ref()
    }

    /// Returns the field that most recently received a value change.
    #[must_use]
    pub const fn active_field(&self) -> ActiveField {
        self.ctx.active_field
    }

    /// Returns the configured preset ranges.
    #[must_use]
    pub fn presets(&self) -> &[Preset] {
        &self.ctx.presets
    }

    /// Returns the label of the preset at `index`, if it exists.
    #[must_use]
    pub fn preset_label(&self, index: usize) -> Option<&str> {
        self.ctx
            .presets
            .get(index)
            .map(|preset| preset.label.as_str())
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
            State::Closed => "closed",
            State::Open => "open",
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::StartInput => self.start_input_attrs(),
            Part::Separator => self.separator_attrs(),
            Part::EndInput => self.end_input_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::PresetTrigger { index } => self.preset_trigger_attrs(index),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}
