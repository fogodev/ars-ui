//! `RangeCalendar` component state machine and connect API.
//!
//! Framework-agnostic implementation of the grid-based date range picker
//! defined in `spec/components/date-time/range-calendar.md`. The machine
//! owns visible month/year state, focused-date roving tab state, two-step
//! range selection, hover preview, min/max constraints, range length
//! constraints, and the connect API attribute contract.

#[cfg(test)]
mod tests;

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString as _},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::{
    cmp::Ordering,
    fmt::{self, Debug},
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Env, HtmlAttr, IntlBackend, KeyboardKey, Locale, MessageFn, TransitionPlan,
};
use ars_i18n::{CalendarDate, DateRange, Weekday};

use super::calendar::{PageBehavior, grid};

// ────────────────────────────────────────────────────────────────────
// State / Event / Effect
// ────────────────────────────────────────────────────────────────────

/// States for the `RangeCalendar` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Calendar rendered but no cell has keyboard focus.
    Idle,

    /// A specific date cell holds keyboard focus within the grid.
    Focused,
}

/// Events for the `RangeCalendar` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Move keyboard focus to a specific date.
    FocusDate {
        /// The date to focus.
        date: CalendarDate,
    },

    /// User selected a date via click or `Enter`/`Space`.
    SelectDate {
        /// The date that was selected.
        date: CalendarDate,
    },

    /// Pointer is hovering over a date cell for pending range preview.
    HoverDate {
        /// The date currently being hovered.
        date: CalendarDate,
    },

    /// Pointer left the grid; clear hover preview.
    HoverEnd,

    /// Navigate to the next month or visible page.
    NextMonth,

    /// Navigate to the previous month or visible page.
    PrevMonth,

    /// Navigate forward by one year.
    NextYear,

    /// Navigate backward by one year.
    PrevYear,

    /// Jump to a specific month.
    SetMonth {
        /// The 1-based month to display.
        month: u8,
    },

    /// Jump to a specific year.
    SetYear {
        /// The year to display.
        year: i32,
    },

    /// Grid received focus.
    FocusIn,

    /// Focus left the grid entirely.
    FocusOut,

    /// Keyboard event on the grid.
    KeyDown {
        /// The key that was pressed.
        key: KeyboardKey,

        /// Whether the shift modifier was held.
        shift: bool,
    },

    /// Synchronize context from a new props snapshot.
    SyncProps(Box<Props>),
}

/// Typed identifier for every named effect intent emitted by `RangeCalendar`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Announce the newly visible month or month range.
    AnnounceMonth,

    /// Announce that the first range endpoint was selected.
    AnnounceRangeStart,

    /// Announce that the range was completed.
    AnnounceRangeComplete,
}

// ────────────────────────────────────────────────────────────────────
// Props / Messages
// ────────────────────────────────────────────────────────────────────

/// Predicate type for marking dates unavailable.
pub type IsDateUnavailableFn = Callback<dyn for<'a> Fn(&'a CalendarDate) -> bool + Send + Sync>;

/// Props for the `RangeCalendar` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled range value. `None` means uncontrolled.
    pub value: Option<Option<DateRange>>,

    /// Default range value for uncontrolled mode.
    pub default_value: Option<DateRange>,

    /// Minimum selectable date.
    pub min: Option<CalendarDate>,

    /// Maximum selectable date.
    pub max: Option<CalendarDate>,

    /// Whether the entire calendar is non-interactive.
    pub disabled: bool,

    /// Whether navigation and focus are allowed but selection is blocked.
    pub readonly: bool,

    /// Predicate returning `true` for dates that are unavailable.
    pub is_date_unavailable: Option<IsDateUnavailableFn>,

    /// Explicit override of the locale default first day of week.
    pub first_day_of_week: Option<Weekday>,

    /// Whether the head row exposes ISO week numbers.
    pub show_week_numbers: bool,

    /// Right-to-left layout direction.
    pub is_rtl: bool,

    /// Number of months displayed side-by-side. Clamped to at least 1.
    pub visible_months: usize,

    /// Controls navigation step size.
    pub page_behavior: PageBehavior,

    /// The "today" date supplied by the adapter for determinism.
    pub today: CalendarDate,

    /// Whether a range with identical start and end dates is valid.
    pub allow_single_date_range: bool,

    /// Minimum inclusive range length in calendar days.
    pub min_range_days: Option<u32>,

    /// Maximum inclusive range length in calendar days.
    pub max_range_days: Option<u32>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            min: None,
            max: None,
            disabled: false,
            readonly: false,
            is_date_unavailable: None,
            first_day_of_week: None,
            show_week_numbers: false,
            is_rtl: false,
            visible_months: 2,
            page_behavior: PageBehavior::Visible,
            today: CalendarDate::new_gregorian(2024, 1, 1)
                .expect("2024-01-01 is a valid Gregorian date"),
            allow_single_date_range: true,
            min_range_days: None,
            max_range_days: None,
        }
    }
}

impl Props {
    /// Creates default props.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`Props::id`].
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`Props::value`].
    #[must_use]
    pub fn value(mut self, value: Option<DateRange>) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets [`Props::default_value`].
    #[must_use]
    pub fn default_value(mut self, value: Option<DateRange>) -> Self {
        self.default_value = value;
        self
    }

    /// Sets [`Props::min`].
    #[must_use]
    pub fn min(mut self, value: Option<CalendarDate>) -> Self {
        self.min = value;
        self
    }

    /// Sets [`Props::max`].
    #[must_use]
    pub fn max(mut self, value: Option<CalendarDate>) -> Self {
        self.max = value;
        self
    }

    /// Sets [`Props::disabled`].
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`Props::readonly`].
    #[must_use]
    pub const fn readonly(mut self, value: bool) -> Self {
        self.readonly = value;
        self
    }

    /// Sets [`Props::is_date_unavailable`].
    #[must_use]
    pub fn is_date_unavailable(mut self, value: Option<IsDateUnavailableFn>) -> Self {
        self.is_date_unavailable = value;
        self
    }

    /// Sets [`Props::first_day_of_week`].
    #[must_use]
    pub const fn first_day_of_week(mut self, value: Option<Weekday>) -> Self {
        self.first_day_of_week = value;
        self
    }

    /// Sets [`Props::show_week_numbers`].
    #[must_use]
    pub const fn show_week_numbers(mut self, value: bool) -> Self {
        self.show_week_numbers = value;
        self
    }

    /// Sets [`Props::is_rtl`].
    #[must_use]
    pub const fn is_rtl(mut self, value: bool) -> Self {
        self.is_rtl = value;
        self
    }

    /// Sets [`Props::visible_months`].
    #[must_use]
    pub const fn visible_months(mut self, value: usize) -> Self {
        self.visible_months = value;
        self
    }

    /// Sets [`Props::page_behavior`].
    #[must_use]
    pub const fn page_behavior(mut self, value: PageBehavior) -> Self {
        self.page_behavior = value;
        self
    }

    /// Sets [`Props::today`].
    #[must_use]
    pub fn today(mut self, value: CalendarDate) -> Self {
        self.today = value;
        self
    }

    /// Sets [`Props::allow_single_date_range`].
    #[must_use]
    pub const fn allow_single_date_range(mut self, value: bool) -> Self {
        self.allow_single_date_range = value;
        self
    }

    /// Sets [`Props::min_range_days`].
    #[must_use]
    pub const fn min_range_days(mut self, value: Option<u32>) -> Self {
        self.min_range_days = value;
        self
    }

    /// Sets [`Props::max_range_days`].
    #[must_use]
    pub const fn max_range_days(mut self, value: Option<u32>) -> Self {
        self.max_range_days = value;
        self
    }
}

/// `MessageFn` carrying a locale-only label closure.
pub type LocaleLabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// `MessageFn` carrying a page step label closure.
pub type PageLabelFn = dyn Fn(usize, &Locale) -> String + Send + Sync;

/// `MessageFn` carrying a date label announcement closure.
pub type RangeStartLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// `MessageFn` carrying a completed range announcement closure.
pub type RangeCompleteLabelFn = dyn Fn(&str, &str, &Locale) -> String + Send + Sync;

/// Locale-specific labels for the `RangeCalendar` component.
#[derive(Clone)]
pub struct Messages {
    /// Accessible label for the previous-month button.
    pub prev_month_label: MessageFn<LocaleLabelFn>,

    /// Accessible label for the next-month button.
    pub next_month_label: MessageFn<LocaleLabelFn>,

    /// Accessible label for previous-page navigation.
    pub prev_page_label: MessageFn<PageLabelFn>,

    /// Accessible label for next-page navigation.
    pub next_page_label: MessageFn<PageLabelFn>,

    /// Separator inserted between month names in a multi-month heading.
    pub month_range_separator: MessageFn<LocaleLabelFn>,

    /// Suffix appended to unavailable cell labels.
    pub unavailable_suffix: MessageFn<LocaleLabelFn>,

    /// Suffix appended to disabled cell labels.
    pub disabled_suffix: MessageFn<LocaleLabelFn>,

    /// Suffix appended to range-start cell labels.
    pub range_start_suffix: MessageFn<LocaleLabelFn>,

    /// Suffix appended to range-end cell labels.
    pub range_end_suffix: MessageFn<LocaleLabelFn>,

    /// Announcement after selecting the first range endpoint.
    pub range_start_label: MessageFn<RangeStartLabelFn>,

    /// Announcement after completing a range.
    pub range_complete_label: MessageFn<RangeCompleteLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            prev_month_label: MessageFn::static_str("Previous month"),
            next_month_label: MessageFn::static_str("Next month"),
            prev_page_label: MessageFn::new(|count: usize, _locale: &Locale| {
                format!("Previous {count} months")
            }),
            next_page_label: MessageFn::new(|count: usize, _locale: &Locale| {
                format!("Next {count} months")
            }),
            month_range_separator: MessageFn::static_str(" \u{2013} "),
            unavailable_suffix: MessageFn::static_str("(unavailable)"),
            disabled_suffix: MessageFn::static_str("(disabled)"),
            range_start_suffix: MessageFn::static_str("(range start)"),
            range_end_suffix: MessageFn::static_str("(range end)"),
            range_start_label: MessageFn::new(|start: &str, _locale: &Locale| {
                format!("Selected {start} as range start. Select an end date.")
            }),
            range_complete_label: MessageFn::new(|start: &str, end: &str, _locale: &Locale| {
                format!("Selected {start} to {end}")
            }),
        }
    }
}

impl Debug for Messages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Messages").finish_non_exhaustive()
    }
}

impl PartialEq for Messages {
    fn eq(&self, other: &Self) -> bool {
        self.prev_month_label == other.prev_month_label
            && self.next_month_label == other.next_month_label
            && self.prev_page_label == other.prev_page_label
            && self.next_page_label == other.next_page_label
            && self.month_range_separator == other.month_range_separator
            && self.unavailable_suffix == other.unavailable_suffix
            && self.disabled_suffix == other.disabled_suffix
            && self.range_start_suffix == other.range_start_suffix
            && self.range_end_suffix == other.range_end_suffix
            && self.range_start_label == other.range_start_label
            && self.range_complete_label == other.range_complete_label
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// Context for the `RangeCalendar` component.
#[derive(Clone)]
pub struct Context {
    /// Selected date range.
    pub value: Bindable<Option<DateRange>>,

    /// First selected date while a range is pending.
    pub anchor_date: Option<CalendarDate>,

    /// Hovered date while a range is pending.
    pub hovering_date: Option<CalendarDate>,

    /// The date that currently holds keyboard focus.
    pub focused_date: CalendarDate,

    /// The first visible month.
    pub visible_month: u8,

    /// The year of the first visible month.
    pub visible_year: i32,

    /// Number of months displayed side-by-side.
    pub visible_months: usize,

    /// Navigation step size for prev/next.
    pub page_behavior: PageBehavior,

    /// Minimum selectable date.
    pub min: Option<CalendarDate>,

    /// Maximum selectable date.
    pub max: Option<CalendarDate>,

    /// Resolved first day of week.
    pub first_day_of_week: Weekday,

    /// Whether the calendar is rendered right-to-left.
    pub is_rtl: bool,

    /// Whether the calendar is globally disabled.
    pub disabled: bool,

    /// Whether the calendar is read-only.
    pub readonly: bool,

    /// Whether week numbers should be rendered.
    pub show_week_numbers: bool,

    /// Date marked as today.
    pub today: CalendarDate,

    /// Predicate marking dates unavailable.
    pub is_date_unavailable_fn: Option<IsDateUnavailableFn>,

    /// Whether same-day ranges are valid.
    pub allow_single_date_range: bool,

    /// Minimum inclusive range length.
    pub min_range_days: Option<u32>,

    /// Maximum inclusive range length.
    pub max_range_days: Option<u32>,

    /// Resolved locale.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Backend used for locale-dependent labels.
    pub intl_backend: Arc<dyn IntlBackend>,

    /// Component part IDs.
    pub ids: ComponentIds,
}

impl Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("value", &self.value)
            .field("anchor_date", &self.anchor_date)
            .field("hovering_date", &self.hovering_date)
            .field("focused_date", &self.focused_date)
            .field("visible_month", &self.visible_month)
            .field("visible_year", &self.visible_year)
            .field("visible_months", &self.visible_months)
            .field("page_behavior", &self.page_behavior)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("first_day_of_week", &self.first_day_of_week)
            .field("is_rtl", &self.is_rtl)
            .field("disabled", &self.disabled)
            .field("readonly", &self.readonly)
            .field("show_week_numbers", &self.show_week_numbers)
            .field("today", &self.today)
            .field(
                "is_date_unavailable_fn",
                &self.is_date_unavailable_fn.as_ref().map(|_| "<callback>"),
            )
            .field("allow_single_date_range", &self.allow_single_date_range)
            .field("min_range_days", &self.min_range_days)
            .field("max_range_days", &self.max_range_days)
            .field("locale", &self.locale)
            .field("messages", &self.messages)
            .field("intl_backend", &"<dyn IntlBackend>")
            .field("ids", &self.ids)
            .finish()
    }
}

impl Context {
    /// Whether `date` is disabled.
    #[must_use]
    pub fn is_date_disabled(&self, date: &CalendarDate) -> bool {
        if self.disabled {
            return true;
        }

        if let Some(min) = &self.min
            && date.compare(min) == Ordering::Less
        {
            return true;
        }

        if let Some(max) = &self.max
            && date.compare(max) == Ordering::Greater
        {
            return true;
        }

        false
    }

    /// Whether `date` is marked unavailable.
    #[must_use]
    pub fn is_date_unavailable(&self, date: &CalendarDate) -> bool {
        self.is_date_unavailable_fn
            .as_ref()
            .is_some_and(|predicate| predicate(date))
    }

    /// Clamps `date` into the configured selectable bounds.
    #[must_use]
    pub fn clamp_date(&self, date: CalendarDate) -> CalendarDate {
        let mut clamped = date;

        if let Some(min) = &self.min
            && clamped.compare(min) == Ordering::Less
        {
            clamped = min.clone();
        }

        if let Some(max) = &self.max
            && clamped.compare(max) == Ordering::Greater
        {
            clamped = max.clone();
        }

        clamped
    }

    /// Whether a confirmed range includes `date`.
    #[must_use]
    pub fn is_in_range(&self, date: &CalendarDate) -> bool {
        self.value
            .get()
            .as_ref()
            .is_some_and(|range| range.contains(date))
    }

    /// Whether `date` is the confirmed range start.
    #[must_use]
    pub fn is_range_start(&self, date: &CalendarDate) -> bool {
        self.value
            .get()
            .as_ref()
            .is_some_and(|range| range.start == *date)
    }

    /// Whether `date` is the confirmed range end.
    #[must_use]
    pub fn is_range_end(&self, date: &CalendarDate) -> bool {
        self.value
            .get()
            .as_ref()
            .is_some_and(|range| range.end == *date)
    }

    /// Whether `date` falls in the pending hover preview range.
    #[must_use]
    pub fn is_in_hover_range(&self, date: &CalendarDate) -> bool {
        match (&self.anchor_date, &self.hovering_date) {
            (Some(anchor), Some(hovering)) => {
                DateRange::normalized(anchor.clone(), hovering.clone())
                    .is_some_and(|range| range.contains(date))
            }

            _ => false,
        }
    }

    /// Whether `date` is the pending range anchor.
    #[must_use]
    pub fn is_anchor(&self, date: &CalendarDate) -> bool {
        self.anchor_date.as_ref() == Some(date)
    }

    /// Whether a range selection is pending.
    #[must_use]
    pub const fn is_range_pending(&self) -> bool {
        self.anchor_date.is_some()
    }

    /// Whether `range` satisfies same-day and length constraints.
    #[must_use]
    pub fn range_is_allowed(&self, range: &DateRange) -> bool {
        if !self.allow_single_date_range && range.start == range.end {
            return false;
        }

        let Some(length) = inclusive_range_days(range) else {
            return false;
        };

        if let Some(min) = self.min_range_days
            && length < min
        {
            return false;
        }

        if let Some(max) = self.max_range_days
            && length > max
        {
            return false;
        }

        true
    }

    /// Scrolls the visible window to include the focused date.
    pub fn sync_visible_to_focused(&mut self) {
        if !grid::is_in_visible_range(
            &self.focused_date,
            self.visible_month,
            self.visible_year,
            self.visible_months,
        ) {
            self.visible_month = self.focused_date.month();
            self.visible_year = self.focused_date.year();
        }
    }

    /// Returns `(month, year)` for the visible month at `offset`.
    #[must_use]
    pub const fn month_year_at_offset(&self, offset: usize) -> (u8, i32) {
        grid::month_year_at_offset(self.visible_month, self.visible_year, offset)
    }

    /// Whether `date` belongs outside the month at `offset`.
    #[must_use]
    pub const fn is_outside_month_at_offset(&self, date: &CalendarDate, offset: usize) -> bool {
        grid::is_outside_month_at_offset(date, self.visible_month, self.visible_year, offset)
    }

    /// Whether `date` belongs outside the first visible month.
    #[must_use]
    pub const fn is_outside_visible_month(&self, date: &CalendarDate) -> bool {
        date.month() != self.visible_month || date.year() != self.visible_year
    }

    /// Builds the 6-week grid for the first visible month.
    #[must_use]
    pub fn weeks(&self) -> Vec<[CalendarDate; 7]> {
        self.weeks_for(0)
    }

    /// Builds the 6-week grid for the month at `offset`.
    #[must_use]
    pub fn weeks_for(&self, offset: usize) -> Vec<[CalendarDate; 7]> {
        grid::weeks_for(
            self.visible_month,
            self.visible_year,
            self.first_day_of_week,
            offset,
        )
    }

    /// Returns the ordered weekday list.
    #[must_use]
    pub fn ordered_weekdays(&self) -> [Weekday; 7] {
        grid::ordered_weekdays(self.first_day_of_week)
    }

    /// Ordered `(weekday, short-label)` pairs for the head row.
    #[must_use]
    pub fn week_day_labels(&self) -> Vec<(Weekday, String)> {
        self.ordered_weekdays()
            .into_iter()
            .map(|wd| (wd, self.intl_backend.weekday_short_label(wd, &self.locale)))
            .collect()
    }

    /// Advances visible month/year by `n` whole months.
    pub const fn advance_month(&mut self, n: i32) {
        let (month, year) = grid::advance_month(self.visible_month, self.visible_year, n);

        self.visible_month = month;
        self.visible_year = year;
    }
}

fn inclusive_range_days(range: &DateRange) -> Option<u32> {
    let days = range.start.days_until(&range.end).ok()?;

    u32::try_from(days).ok()?.checked_add(1)
}

// ────────────────────────────────────────────────────────────────────
// Part / Machine
// ────────────────────────────────────────────────────────────────────

/// Parts of the `RangeCalendar` anatomy.
#[derive(ars_core::ComponentPart)]
#[scope = "range-calendar"]
pub enum Part {
    /// The outer component root.
    Root,

    /// Header container for navigation and heading.
    Header,

    /// The previous-month or previous-page button.
    PrevTrigger,

    /// The next-month or next-page button.
    NextTrigger,

    /// The textual month/year heading.
    Heading,

    /// The grid wrapper for a visible month.
    Grid,

    /// Wrapper grouping multiple visible month grids.
    GridGroup,

    /// The header row of weekday labels.
    HeadRow,

    /// Individual weekday header cell.
    HeadCell {
        /// The weekday this cell describes.
        #[part(default = Weekday::Monday)]
        day: Weekday,
    },

    /// A row of day cells.
    Row {
        /// Zero-based week index.
        week_index: usize,
    },

    /// A grid cell wrapping a date trigger.
    Cell {
        /// The date represented by this cell.
        #[part(default = default_calendar_date())]
        date: CalendarDate,

        /// Zero-based visible month offset.
        offset: usize,
    },

    /// Interactive trigger inside a date cell.
    CellTrigger {
        /// The date represented by this trigger.
        #[part(default = default_calendar_date())]
        date: CalendarDate,

        /// Zero-based visible month offset.
        offset: usize,
    },
}

fn default_calendar_date() -> CalendarDate {
    CalendarDate::new_gregorian(1, 1, 1).expect("1-01-01 is a valid Gregorian date")
}

/// Machine for the `RangeCalendar` component.
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
        let value = if let Some(controlled) = &props.value {
            Bindable::controlled(controlled.clone())
        } else {
            Bindable::uncontrolled(props.default_value.clone())
        };

        let mut initial_date = value
            .get()
            .as_ref()
            .map_or_else(|| props.today.clone(), |range| range.start.clone());

        if let Some(min) = &props.min
            && initial_date.compare(min) == Ordering::Less
        {
            initial_date = min.clone();
        }
        if let Some(max) = &props.max
            && initial_date.compare(max) == Ordering::Greater
        {
            initial_date = max.clone();
        }

        let locale = env.locale.clone();

        let first_day_of_week = props
            .first_day_of_week
            .unwrap_or_else(|| locale.first_day_of_week(&*env.intl_backend));

        let visible_months = props.visible_months.max(1);

        let focused_date = initial_date;

        let visible_month = focused_date.month();
        let visible_year = focused_date.year();

        let ctx = Context {
            value,
            anchor_date: None,
            hovering_date: None,
            focused_date,
            visible_month,
            visible_year,
            visible_months,
            page_behavior: props.page_behavior,
            min: props.min.clone(),
            max: props.max.clone(),
            first_day_of_week,
            is_rtl: props.is_rtl,
            disabled: props.disabled,
            readonly: props.readonly,
            show_week_numbers: props.show_week_numbers,
            today: props.today.clone(),
            is_date_unavailable_fn: props.is_date_unavailable.clone(),
            allow_single_date_range: props.allow_single_date_range,
            min_range_days: props.min_range_days,
            max_range_days: props.max_range_days,
            locale,
            messages: messages.clone(),
            intl_backend: Arc::clone(&env.intl_backend),
            ids: ComponentIds::from_id(&props.id),
        };

        (State::Idle, ctx)
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        debug_assert_eq!(
            old.id, new.id,
            "range_calendar::Props.id must remain stable after init",
        );

        if old == new {
            Vec::new()
        } else {
            vec![Event::SyncProps(Box::new(new.clone()))]
        }
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if matches!(event, Event::FocusOut) {
            return Some(TransitionPlan::to(State::Idle));
        }

        if let Event::SyncProps(props) = event {
            let props = props.as_ref().clone();

            return Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                sync_props_into_ctx(ctx, &props);
            }));
        }

        if ctx.disabled {
            return None;
        }

        match event {
            Event::FocusIn => Some(TransitionPlan::to(State::Focused)),

            Event::FocusOut | Event::SyncProps(_) => None,

            Event::FocusDate { date } => {
                let clamped = ctx.clamp_date(date.clone());
                Some(
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.focused_date = clamped;
                        ctx.sync_visible_to_focused();
                    }),
                )
            }

            Event::SelectDate { date } => apply_select_date(ctx, date.clone()),

            Event::HoverDate { date } => {
                if ctx.readonly || ctx.anchor_date.is_none() {
                    return None;
                }

                let date = date.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.hovering_date = Some(date);
                }))
            }

            Event::HoverEnd => {
                ctx.hovering_date.as_ref()?;

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.hovering_date = None;
                }))
            }

            Event::NextMonth => month_step_plan(ctx, step_for_page_behavior(ctx)),

            Event::PrevMonth => month_step_plan(ctx, -step_for_page_behavior(ctx)),

            Event::NextYear => month_step_plan(ctx, 12),

            Event::PrevYear => month_step_plan(ctx, -12),

            Event::SetMonth { month } => {
                let month = *month;

                if !(1..=12).contains(&month) {
                    return None;
                }

                let delta = i32::from(month) - i32::from(ctx.visible_month);

                let shifted = ctx
                    .focused_date
                    .add(ars_i18n::DateDuration {
                        months: delta,
                        ..ars_i18n::DateDuration::default()
                    })
                    .ok()?;

                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.visible_month = month;
                        ctx.focused_date = ctx.clamp_date(shifted);

                        ctx.sync_visible_to_focused();
                    })
                    .with_effect(announce_month_effect()),
                )
            }

            Event::SetYear { year } => {
                let year = *year;
                let delta_years = year.checked_sub(ctx.visible_year)?;

                let shifted = ctx
                    .focused_date
                    .add(ars_i18n::DateDuration {
                        years: delta_years,
                        ..ars_i18n::DateDuration::default()
                    })
                    .ok()?;

                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.visible_year = year;
                        ctx.focused_date = ctx.clamp_date(shifted);

                        ctx.sync_visible_to_focused();
                    })
                    .with_effect(announce_month_effect()),
                )
            }

            Event::KeyDown { key, shift } => handle_keydown(*key, *shift, ctx),
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        context: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api::new(state, context, props, send)
    }
}

fn sync_props_into_ctx(ctx: &mut Context, props: &Props) {
    ctx.value.sync_controlled(props.value.clone());
    ctx.min = props.min.clone();
    ctx.max = props.max.clone();
    ctx.disabled = props.disabled;
    ctx.readonly = props.readonly;
    ctx.is_date_unavailable_fn = props.is_date_unavailable.clone();
    ctx.show_week_numbers = props.show_week_numbers;
    ctx.is_rtl = props.is_rtl;
    ctx.visible_months = props.visible_months.max(1);
    ctx.page_behavior = props.page_behavior;
    ctx.today = props.today.clone();
    ctx.allow_single_date_range = props.allow_single_date_range;
    ctx.min_range_days = props.min_range_days;
    ctx.max_range_days = props.max_range_days;
    ctx.first_day_of_week = props
        .first_day_of_week
        .unwrap_or_else(|| ctx.locale.first_day_of_week(&*ctx.intl_backend));

    ctx.focused_date = ctx.clamp_date(ctx.focused_date.clone());

    revalidate_pending_selection(ctx);
    ctx.sync_visible_to_focused();
}

fn revalidate_pending_selection(ctx: &mut Context) {
    let anchor_still_selectable = ctx
        .anchor_date
        .as_ref()
        .is_some_and(|date| !ctx.is_date_disabled(date) && !ctx.is_date_unavailable(date));

    if !anchor_still_selectable {
        ctx.anchor_date = None;
        ctx.hovering_date = None;

        return;
    }

    if ctx
        .hovering_date
        .as_ref()
        .is_some_and(|date| ctx.is_date_disabled(date) || ctx.is_date_unavailable(date))
    {
        ctx.hovering_date = None;
    }
}

fn apply_select_date(ctx: &Context, date: CalendarDate) -> Option<TransitionPlan<Machine>> {
    if ctx.readonly || ctx.is_date_disabled(&date) || ctx.is_date_unavailable(&date) {
        return None;
    }

    if let Some(anchor) = &ctx.anchor_date {
        let range = DateRange::normalized(anchor.clone(), date.clone())?;
        if !range_is_selectable(ctx, &range) {
            let focused = date;

            return Some(
                TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                    ctx.focused_date = focused;
                    ctx.sync_visible_to_focused();
                }),
            );
        }

        Some(
            TransitionPlan::to(State::Focused)
                .apply(move |ctx: &mut Context| {
                    ctx.value.set(Some(range));
                    ctx.anchor_date = None;
                    ctx.hovering_date = None;
                    ctx.focused_date = date;
                    ctx.sync_visible_to_focused();
                })
                .with_effect(ars_core::PendingEffect::named(
                    Effect::AnnounceRangeComplete,
                )),
        )
    } else {
        Some(
            TransitionPlan::to(State::Focused)
                .apply(move |ctx: &mut Context| {
                    ctx.anchor_date = Some(date.clone());
                    ctx.hovering_date = None;
                    ctx.focused_date = date;
                    ctx.value.set(None);
                })
                .with_effect(ars_core::PendingEffect::named(Effect::AnnounceRangeStart)),
        )
    }
}

fn range_is_selectable(ctx: &Context, range: &DateRange) -> bool {
    if !ctx.range_is_allowed(range) {
        return false;
    }

    if ctx.is_date_disabled(&range.start) || ctx.is_date_disabled(&range.end) {
        return false;
    }

    if ctx.is_date_unavailable_fn.is_none() {
        return true;
    }

    range_dates_all(ctx, range, |ctx, date| !ctx.is_date_unavailable(date))
}

fn range_dates_all(
    ctx: &Context,
    range: &DateRange,
    mut predicate: impl FnMut(&Context, &CalendarDate) -> bool,
) -> bool {
    let Some(days) = inclusive_range_days(range) else {
        return false;
    };

    for offset in 0..days {
        let Ok(offset) = i32::try_from(offset) else {
            return false;
        };

        let Ok(date) = range.start.add_days(offset) else {
            return false;
        };

        if !predicate(ctx, &date) {
            return false;
        }
    }

    true
}

fn step_for_page_behavior(ctx: &Context) -> i32 {
    match ctx.page_behavior {
        PageBehavior::Visible => i32::try_from(ctx.visible_months).unwrap_or(1).max(1),
        PageBehavior::Single => 1,
    }
}

fn month_step_plan(ctx: &Context, step: i32) -> Option<TransitionPlan<Machine>> {
    let shifted = ctx
        .focused_date
        .add(ars_i18n::DateDuration {
            months: step,
            ..ars_i18n::DateDuration::default()
        })
        .ok()?;

    Some(
        TransitionPlan::context_only(move |ctx: &mut Context| {
            ctx.advance_month(step);
            ctx.focused_date = ctx.clamp_date(shifted);

            ctx.sync_visible_to_focused();
        })
        .with_effect(announce_month_effect()),
    )
}

fn announce_month_effect() -> ars_core::PendingEffect<Machine> {
    ars_core::PendingEffect::named(Effect::AnnounceMonth)
}

fn handle_keydown(key: KeyboardKey, shift: bool, ctx: &Context) -> Option<TransitionPlan<Machine>> {
    if shift {
        match key {
            KeyboardKey::PageUp => return month_step_plan(ctx, -12),
            KeyboardKey::PageDown => return month_step_plan(ctx, 12),
            _ => {}
        }
    }

    let focused = ctx.focused_date.clone();

    let (prev_day_key, next_day_key) = if ctx.is_rtl {
        (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
    } else {
        (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
    };

    let new_focus = if key == prev_day_key {
        focused.add_days(-1).ok()
    } else if key == next_day_key {
        focused.add_days(1).ok()
    } else {
        match key {
            KeyboardKey::ArrowUp => focused.add_days(-7).ok(),

            KeyboardKey::ArrowDown => focused.add_days(7).ok(),

            KeyboardKey::Home => {
                let wd = grid::weekday_sunday_zero(focused.weekday());
                let start = grid::weekday_sunday_zero(ctx.first_day_of_week);
                let offset = i32::from((wd + 7 - start) % 7);

                focused.add_days(-offset).ok()
            }

            KeyboardKey::End => {
                let wd = grid::weekday_sunday_zero(focused.weekday());
                let start = grid::weekday_sunday_zero(ctx.first_day_of_week);
                let offset = i32::from((wd + 7 - start) % 7);

                focused.add_days(6 - offset).ok()
            }

            KeyboardKey::PageUp => focused
                .add(ars_i18n::DateDuration {
                    months: -1,
                    ..ars_i18n::DateDuration::default()
                })
                .ok(),

            KeyboardKey::PageDown => focused
                .add(ars_i18n::DateDuration {
                    months: 1,
                    ..ars_i18n::DateDuration::default()
                })
                .ok(),

            _ => None,
        }
    };

    if let Some(date) = new_focus {
        let clamped = ctx.clamp_date(date);

        return Some(
            TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                ctx.focused_date = clamped;
                ctx.sync_visible_to_focused();
            }),
        );
    }

    match key {
        KeyboardKey::Enter | KeyboardKey::Space => {
            let date = focused;
            Some(
                TransitionPlan::context_only(|_ctx: &mut Context| {})
                    .then(Event::SelectDate { date }),
            )
        }

        _ => None,
    }
}

// ────────────────────────────────────────────────────────────────────
// Api
// ────────────────────────────────────────────────────────────────────

/// API for the `RangeCalendar` component.
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
            .finish_non_exhaustive()
    }
}

impl<'a> Api<'a> {
    /// Creates a new `RangeCalendar` connect API.
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

    /// Returns attributes for the outer root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_name());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.ctx.is_rtl {
            attrs.set(HtmlAttr::Dir, "rtl");
        }

        if self.ctx.is_range_pending() {
            attrs.set_bool(HtmlAttr::Data("ars-range-pending"), true);
        }

        attrs
    }

    /// Returns attributes for the header container.
    #[must_use]
    pub fn header_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Header.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("header"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the previous button.
    #[must_use]
    pub fn prev_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::PrevTrigger.data_attrs();

        let step = self.nav_step_size();

        let label = if step > 1 {
            (self.ctx.messages.prev_page_label)(step, &self.ctx.locale)
        } else {
            (self.ctx.messages.prev_month_label)(&self.ctx.locale)
        };

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("prev-trigger"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Label), label)
            .set(HtmlAttr::TabIndex, "-1");

        if self.is_prev_disabled() {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Returns attributes for the next button.
    #[must_use]
    pub fn next_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::NextTrigger.data_attrs();

        let step = self.nav_step_size();

        let label = if step > 1 {
            (self.ctx.messages.next_page_label)(step, &self.ctx.locale)
        } else {
            (self.ctx.messages.next_month_label)(&self.ctx.locale)
        };

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("next-trigger"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Label), label)
            .set(HtmlAttr::TabIndex, "-1");

        if self.is_next_disabled() {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Returns attributes for the main heading.
    #[must_use]
    pub fn heading_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Heading.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("heading"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite")
            .set(HtmlAttr::Aria(AriaAttr::Atomic), "true");

        attrs
    }

    /// Heading text for the first visible month.
    #[must_use]
    pub fn heading_text(&self) -> String {
        self.heading_text_for(0)
    }

    /// Attributes for the first visible grid.
    #[must_use]
    pub fn grid_attrs(&self) -> AttrMap {
        self.grid_attrs_with_ids(self.ctx.ids.part("grid"), self.ctx.ids.part("heading"))
    }

    /// Attributes for the grid at `offset`.
    #[must_use]
    pub fn grid_attrs_for(&self, offset: usize) -> AttrMap {
        self.grid_attrs_with_ids(
            self.ctx.ids.item("grid", &offset),
            self.ctx.ids.item("heading", &offset),
        )
    }

    fn grid_attrs_with_ids(&self, grid_id: String, heading_id: String) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Grid.data_attrs();

        attrs
            .set(HtmlAttr::Id, grid_id)
            .set(HtmlAttr::Role, "grid")
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::LabelledBy), heading_id)
            .set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Returns attributes for the multi-grid wrapper.
    #[must_use]
    pub fn grid_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::GridGroup.data_attrs();

        attrs
            .set(HtmlAttr::Id, format!("{}-grid-group", self.ctx.ids.id()))
            .set(HtmlAttr::Role, "group")
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Label), self.range_heading_text());

        attrs
    }

    /// Heading attributes for a per-grid hidden heading.
    #[must_use]
    pub fn heading_attrs_for(&self, offset: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Heading.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("heading", &offset))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        attrs
    }

    /// Heading text for the month at `offset`.
    #[must_use]
    pub fn heading_text_for(&self, offset: usize) -> String {
        let (month, year) = self.ctx.month_year_at_offset(offset);

        format!(
            "{} {}",
            self.ctx
                .intl_backend
                .month_long_name(month, &self.ctx.locale),
            year,
        )
    }

    /// Heading text spanning all visible months.
    #[must_use]
    pub fn range_heading_text(&self) -> String {
        if self.ctx.visible_months <= 1 {
            return self.heading_text();
        }

        let first = self.heading_text_for(0);
        let last = self.heading_text_for(self.ctx.visible_months - 1);

        let sep = (self.ctx.messages.month_range_separator)(&self.ctx.locale);

        format!("{first}{sep}{last}")
    }

    /// Returns attributes for the weekday header row.
    #[must_use]
    pub fn head_row_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HeadRow.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for one weekday header cell.
    #[must_use]
    pub fn head_cell_attrs(&self, weekday: Weekday) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::HeadCell { day: weekday }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Scope, "col")
            .set(
                HtmlAttr::Abbr,
                self.ctx
                    .intl_backend
                    .weekday_long_label(weekday, &self.ctx.locale),
            );

        attrs
    }

    /// Returns attributes for a week row.
    #[must_use]
    pub fn row_attrs(&self, week_index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Row { week_index }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-week-index"), week_index.to_string());

        attrs
    }

    /// Returns attributes for a date cell.
    #[must_use]
    pub fn cell_attrs(&self, date: &CalendarDate) -> AttrMap {
        self.cell_attrs_inner(date, None)
    }

    /// Returns attributes for a date cell in a specific month offset.
    #[must_use]
    pub fn cell_attrs_for(&self, date: &CalendarDate, offset: usize) -> AttrMap {
        self.cell_attrs_inner(date, Some(offset))
    }

    /// Whether `date` is outside the month at `offset`.
    #[must_use]
    pub const fn is_outside_month_for(&self, date: &CalendarDate, offset: usize) -> bool {
        self.ctx.is_outside_month_at_offset(date, offset)
    }

    fn cell_attrs_inner(&self, date: &CalendarDate, offset: Option<usize>) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Cell {
            date: date.clone(),
            offset: offset.unwrap_or(0),
        }
        .data_attrs();

        attrs
            .set(HtmlAttr::Role, "gridcell")
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        if self.is_in_range(date) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Selected), "true");
        }

        let outside = if let Some(offset) = offset {
            self.ctx.is_outside_month_at_offset(date, offset)
        } else {
            self.ctx.is_outside_visible_month(date)
        };

        if outside {
            attrs.set_bool(HtmlAttr::Data("ars-outside-month"), true);
        }

        if self.is_disabled(date) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Returns attributes for a date cell trigger.
    #[must_use]
    pub fn cell_trigger_attrs(&self, date: &CalendarDate) -> AttrMap {
        self.cell_trigger_attrs_inner(date, None)
    }

    /// Returns attributes for a date cell trigger in a specific month offset.
    #[must_use]
    pub fn cell_trigger_attrs_for(&self, date: &CalendarDate, offset: usize) -> AttrMap {
        self.cell_trigger_attrs_inner(date, Some(offset))
    }

    fn cell_trigger_attrs_inner(&self, date: &CalendarDate, offset: Option<usize>) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CellTrigger {
            date: date.clone(),
            offset: offset.unwrap_or(0),
        }
        .data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button");

        let disabled = self.is_disabled(date);
        let unavailable = self.is_unavailable(date);
        let focused = &self.ctx.focused_date == date;

        attrs.set(HtmlAttr::TabIndex, if focused { "0" } else { "-1" });

        if disabled || unavailable {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.is_in_range(date) {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Selected), "true")
                .set_bool(HtmlAttr::Data("ars-in-range"), true);
        }

        if self.is_today(date) {
            attrs.set_bool(HtmlAttr::Data("ars-today"), true);
        }

        let outside = if let Some(offset) = offset {
            self.ctx.is_outside_month_at_offset(date, offset)
        } else {
            self.ctx.is_outside_visible_month(date)
        };

        if outside {
            attrs.set_bool(HtmlAttr::Data("ars-outside-month"), true);
        }

        if unavailable {
            attrs.set_bool(HtmlAttr::Data("ars-unavailable"), true);
        }

        if self.is_range_start(date) {
            attrs.set_bool(HtmlAttr::Data("ars-range-start"), true);
        }

        if self.is_range_end(date) {
            attrs.set_bool(HtmlAttr::Data("ars-range-end"), true);
        }

        if self.is_in_hover_range(date) {
            attrs.set_bool(HtmlAttr::Data("ars-in-hover-range"), true);
        }

        if self.is_anchor(date) {
            attrs.set_bool(HtmlAttr::Data("ars-anchor"), true);
        }

        attrs.set(HtmlAttr::Aria(AriaAttr::Label), self.cell_label(date));

        attrs
    }

    fn cell_label(&self, date: &CalendarDate) -> String {
        let base = format_date_label(date, &*self.ctx.intl_backend, &self.ctx.locale);

        if self.is_unavailable(date) {
            let suffix = (self.ctx.messages.unavailable_suffix)(&self.ctx.locale);

            format!("{base} {suffix}")
        } else if self.is_disabled(date) {
            let suffix = (self.ctx.messages.disabled_suffix)(&self.ctx.locale);

            format!("{base} {suffix}")
        } else {
            let mut label = base;

            if self.is_range_start(date) {
                let suffix = (self.ctx.messages.range_start_suffix)(&self.ctx.locale);

                label.push(' ');
                label.push_str(&suffix);
            }

            if self.is_range_end(date) {
                let suffix = (self.ctx.messages.range_end_suffix)(&self.ctx.locale);

                label.push(' ');
                label.push_str(&suffix);
            }

            label
        }
    }

    /// Whether `date` falls inside the confirmed range.
    #[must_use]
    pub fn is_in_range(&self, date: &CalendarDate) -> bool {
        self.ctx.is_in_range(date)
    }

    /// Whether `date` is the confirmed range start.
    #[must_use]
    pub fn is_range_start(&self, date: &CalendarDate) -> bool {
        self.ctx.is_range_start(date)
    }

    /// Whether `date` is the confirmed range end.
    #[must_use]
    pub fn is_range_end(&self, date: &CalendarDate) -> bool {
        self.ctx.is_range_end(date)
    }

    /// Whether `date` falls in the pending hover range.
    #[must_use]
    pub fn is_in_hover_range(&self, date: &CalendarDate) -> bool {
        self.ctx.is_in_hover_range(date)
    }

    /// Whether `date` is the pending anchor.
    #[must_use]
    pub fn is_anchor(&self, date: &CalendarDate) -> bool {
        self.ctx.is_anchor(date)
    }

    /// Whether `date` equals the configured today.
    #[must_use]
    pub fn is_today(&self, date: &CalendarDate) -> bool {
        date == &self.ctx.today
    }

    /// Whether `date` is disabled.
    #[must_use]
    pub fn is_disabled(&self, date: &CalendarDate) -> bool {
        self.ctx.is_date_disabled(date)
    }

    /// Whether `date` is unavailable.
    #[must_use]
    pub fn is_unavailable(&self, date: &CalendarDate) -> bool {
        self.ctx.is_date_unavailable(date)
    }

    /// Returns the configured today date.
    #[must_use]
    pub const fn today(&self) -> &CalendarDate {
        &self.ctx.today
    }

    /// Returns the focused date.
    #[must_use]
    pub const fn focused_date(&self) -> &CalendarDate {
        &self.ctx.focused_date
    }

    /// Whether the grid has focus.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        matches!(self.state, State::Focused)
    }

    /// Number of visible months.
    #[must_use]
    pub const fn visible_month_count(&self) -> usize {
        self.ctx.visible_months
    }

    /// Iterator over visible month offsets.
    #[must_use]
    pub const fn month_offsets(&self) -> core::ops::Range<usize> {
        0..self.ctx.visible_months
    }

    /// Whether week numbers should be rendered.
    #[must_use]
    pub const fn show_week_numbers(&self) -> bool {
        self.ctx.show_week_numbers
    }

    /// Grid weeks for the first visible month.
    #[must_use]
    pub fn weeks(&self) -> Vec<[CalendarDate; 7]> {
        self.ctx.weeks()
    }

    /// Grid weeks for a month offset.
    #[must_use]
    pub fn weeks_for(&self, offset: usize) -> Vec<[CalendarDate; 7]> {
        self.ctx.weeks_for(offset)
    }

    /// Ordered weekday labels.
    #[must_use]
    pub fn week_day_labels(&self) -> Vec<(Weekday, String)> {
        self.ctx.week_day_labels()
    }

    /// Whether the previous trigger should be disabled.
    #[must_use]
    pub fn is_prev_disabled(&self) -> bool {
        if self.ctx.disabled {
            return true;
        }

        let step = self.nav_step_size();

        let (new_first_month, new_first_year) = grid::advance_month(
            self.ctx.visible_month,
            self.ctx.visible_year,
            -i32::try_from(step).unwrap_or(1),
        );

        let last_offset = self.ctx.visible_months.saturating_sub(1);

        let (new_last_month, new_last_year) =
            grid::month_year_at_offset(new_first_month, new_first_year, last_offset);

        let Ok(first_of_new_last) = CalendarDate::new_gregorian(new_last_year, new_last_month, 1)
        else {
            return true;
        };

        let Ok(last_of_new_last) = CalendarDate::new_gregorian(
            new_last_year,
            new_last_month,
            first_of_new_last.days_in_month(),
        ) else {
            return true;
        };

        let Some(min) = &self.ctx.min else {
            return false;
        };

        matches!(last_of_new_last.compare(min), Ordering::Less)
    }

    /// Whether the next trigger should be disabled.
    #[must_use]
    pub fn is_next_disabled(&self) -> bool {
        if self.ctx.disabled {
            return true;
        }

        let step = self.nav_step_size();

        let (new_first_month, new_first_year) = grid::advance_month(
            self.ctx.visible_month,
            self.ctx.visible_year,
            i32::try_from(step).unwrap_or(1),
        );

        let Ok(first_of_new_first) =
            CalendarDate::new_gregorian(new_first_year, new_first_month, 1)
        else {
            return true;
        };

        let Some(max) = &self.ctx.max else {
            return false;
        };

        matches!(first_of_new_first.compare(max), Ordering::Greater)
    }

    /// Handle click on a day cell.
    pub fn on_cell_click(&self, date: CalendarDate) {
        (self.send)(Event::SelectDate { date });
    }

    /// Handle hover over a day cell.
    pub fn on_cell_hover(&self, date: CalendarDate) {
        (self.send)(Event::HoverDate { date });
    }

    /// Handle pointer leaving the grid.
    pub fn on_grid_mouseleave(&self) {
        (self.send)(Event::HoverEnd);
    }

    /// Handle focus entering the grid.
    pub fn on_grid_focusin(&self) {
        (self.send)(Event::FocusIn);
    }

    /// Handle focus leaving the grid.
    pub fn on_grid_focusout(&self, focus_leaving_grid: bool) {
        if focus_leaving_grid {
            (self.send)(Event::FocusOut);
        }
    }

    /// Handle keydown on the grid.
    pub fn on_grid_keydown(&self, key: KeyboardKey, shift: bool) {
        (self.send)(Event::KeyDown { key, shift });
    }

    /// Handle previous button click.
    pub fn on_prev_click(&self) {
        (self.send)(Event::PrevMonth);
    }

    /// Handle next button click.
    pub fn on_next_click(&self) {
        (self.send)(Event::NextMonth);
    }

    const fn nav_step_size(&self) -> usize {
        match self.ctx.page_behavior {
            PageBehavior::Visible => self.ctx.visible_months,
            PageBehavior::Single => 1,
        }
    }

    const fn state_name(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Focused => "focused",
        }
    }
}

fn format_date_label(date: &CalendarDate, backend: &dyn IntlBackend, locale: &Locale) -> String {
    format!(
        "{} {}, {}, {}",
        backend.month_long_name(date.month(), locale),
        date.day(),
        date.year(),
        backend.weekday_long_label(date.weekday(), locale),
    )
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Header => self.header_attrs(),
            Part::PrevTrigger => self.prev_trigger_attrs(),
            Part::NextTrigger => self.next_trigger_attrs(),
            Part::Heading => self.heading_attrs(),
            Part::Grid => self.grid_attrs(),
            Part::GridGroup => self.grid_group_attrs(),
            Part::HeadRow => self.head_row_attrs(),
            Part::HeadCell { day } => self.head_cell_attrs(day),
            Part::Row { week_index } => self.row_attrs(week_index),
            Part::Cell { date, offset } => self.cell_attrs_for(&date, offset),
            Part::CellTrigger { date, offset } => self.cell_trigger_attrs_for(&date, offset),
        }
    }
}
