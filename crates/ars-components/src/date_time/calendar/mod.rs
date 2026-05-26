//! `Calendar` component state machine and connect API.
//!
//! Framework-agnostic implementation of the grid-based date picker defined
//! in `spec/components/date-time/calendar.md`. The machine owns visible
//! month/year, focused-date roving tab state, single and multi-date
//! selection, locale-aware weekday ordering, min/max constraints, and the
//! screen-reader announcement intent emitted when the visible month
//! changes.
//!
//! ## Spec-vs-implementation reconciliation
//!
//! The spec's §1.7 code examples use illustrative pseudo-Rust that
//! pre-dates the current `ars-i18n::CalendarDate` API: it treats months and
//! days as `NonZero<u8>`, calls `add_days` / `add_months` as infallible,
//! and accesses month/day/year as fields. The real types return
//! `Result<_, CalendarError>` and expose month/day/year as `u8` methods.
//! The implementation here keeps the spec's semantics intact while using
//! the real fallible API; cases where the spec implies an arithmetic
//! result cannot fail (grid anchoring, focused-date navigation) propagate
//! using `?`-style early returns that leave context unchanged rather than
//! panic at calendar-system boundaries.
//!
//! ## Range-calendar drift in spec §1.8
//!
//! The spec's §1.8 connect API includes `on_cell_hover(date) ->
//! Event::HoverDate { date }` and `on_grid_mouseleave() ->
//! Event::HoverEnd`, but neither event appears in §1.2 and neither has a
//! transition arm. Those entries belong to the future `RangeCalendar`
//! component; this implementation omits them and the matching
//! `Event::HoverDate` / `Event::HoverEnd` variants. The range-calendar
//! delivery will reintroduce them alongside the rest of its hover/range
//! preview state.

pub mod grid;

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
use ars_i18n::{CalendarDate, Weekday};

use self::grid::{
    advance_month as advance_month_pair, is_in_visible_range as grid_is_in_visible_range,
    is_outside_month_at_offset as grid_is_outside_month_at_offset,
    month_year_at_offset as grid_month_year_at_offset, ordered_weekdays as grid_ordered_weekdays,
    weeks_for as grid_weeks_for,
};

// ────────────────────────────────────────────────────────────────────
// State / Event / Effect
// ────────────────────────────────────────────────────────────────────

/// States for the `Calendar` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Calendar rendered but no cell has keyboard focus.
    Idle,

    /// A specific date cell holds keyboard focus within the grid.
    Focused,
}

/// Events for the `Calendar` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Move keyboard focus to a specific date (clamped to min/max).
    FocusDate {
        /// The date to focus on.
        date: CalendarDate,
    },

    /// User selected a date via click or `Enter`/`Space` on the focused cell.
    ///
    /// In [`SelectionMode::Single`] this sets the value to that date. In
    /// [`SelectionMode::Multiple`] it falls through to [`Event::ToggleDate`]
    /// to keep one user-visible "select this date" event for adapters that
    /// do not know the configured selection mode.
    SelectDate {
        /// The date that was selected.
        date: CalendarDate,
    },

    /// Toggle the date in or out of the multi-selection set.
    ///
    /// Only meaningful when `selection_mode == Multiple`. A toggle that
    /// would add a date when the set already holds `max_selected` entries
    /// is silently dropped.
    ToggleDate {
        /// The date to toggle.
        date: CalendarDate,
    },

    /// Navigate to the next month (advances by `page_behavior` step).
    NextMonth,

    /// Navigate to the previous month (retreats by `page_behavior` step).
    PrevMonth,

    /// Navigate forward by 12 months (one year).
    NextYear,

    /// Navigate backward by 12 months (one year).
    PrevYear,

    /// Jump to a specific month (1-based; out-of-range values are ignored).
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
    ///
    /// The `shift` flag is folded into the event so the adapter does not
    /// need a side-channel to communicate `Shift+PageUp` / `Shift+PageDown`
    /// for year navigation.
    KeyDown {
        /// The key that was pressed.
        key: KeyboardKey,

        /// Whether the shift modifier was held.
        shift: bool,
    },

    /// Synchronise context from a new props snapshot.
    ///
    /// Emitted automatically by [`Machine::on_props_changed`] when the
    /// adapter pushes new props via `Service::set_props`. Updates the
    /// `Bindable` controlled values, refreshes the cached `disabled`,
    /// `readonly`, `min`/`max`, predicate, and layout fields, and
    /// re-clamps `focused_date` into the (possibly tighter) range.
    SyncProps(Box<Props>),
}

/// Typed identifier for every named effect intent the `calendar` machine emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Announce the newly visible month (or month range) to assistive tech
    /// via the adapter's platform announcer. Triggered by every
    /// month/year navigation transition.
    AnnounceMonth,
}

// ────────────────────────────────────────────────────────────────────
// Props sub-types
// ────────────────────────────────────────────────────────────────────

/// Controls how prev/next navigation advances when multiple months are visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageBehavior {
    /// Prev/next advances by the full `visible_months` count.
    #[default]
    Visible,

    /// Prev/next advances by exactly 1 month, regardless of `visible_months`.
    Single,
}

/// Whether the calendar selects a single date or an unordered set of dates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    /// Single-date selection — `value` holds the chosen date (or `None`).
    #[default]
    Single,

    /// Multiple non-contiguous date selection — `selected_dates` holds the
    /// chosen set, optionally capped by `max_selected`.
    Multiple,
}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Ordered set of unique calendar dates, sorted by
/// [`CalendarDate::compare`].
///
/// [`CalendarDate`] implements neither [`Ord`] nor [`PartialOrd`] (different
/// calendar systems share the same `CalendarDate` value type, so global
/// ordering is undefined), but it does expose a chronological
/// [`compare`](CalendarDate::compare) method. `SelectedDates` is a thin
/// newtype wrapper around `Vec<CalendarDate>` that maintains sorted order
/// using that comparator, plays the role of `SelectedDates` for the
/// multi-select variant, and stays usable without any extra trait impls on
/// the foreign type.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectedDates {
    dates: Vec<CalendarDate>,
}

impl SelectedDates {
    /// Creates an empty set.
    #[must_use]
    pub const fn new() -> Self {
        Self { dates: Vec::new() }
    }

    /// Returns the number of selected dates.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.dates.len()
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.dates.is_empty()
    }

    /// Iterates over the selected dates in chronological order.
    pub fn iter(&self) -> core::slice::Iter<'_, CalendarDate> {
        self.dates.iter()
    }

    /// Returns whether `date` is in the set.
    #[must_use]
    pub fn contains(&self, date: &CalendarDate) -> bool {
        self.dates
            .binary_search_by(|candidate| candidate.compare(date))
            .is_ok()
    }

    /// Inserts `date` into the set, returning `true` if it was newly added.
    pub fn insert(&mut self, date: CalendarDate) -> bool {
        if let Err(idx) = self
            .dates
            .binary_search_by(|candidate| candidate.compare(&date))
        {
            self.dates.insert(idx, date);

            true
        } else {
            false
        }
    }

    /// Removes `date` from the set, returning `true` if it was present.
    pub fn remove(&mut self, date: &CalendarDate) -> bool {
        if let Ok(idx) = self
            .dates
            .binary_search_by(|candidate| candidate.compare(date))
        {
            self.dates.remove(idx);

            true
        } else {
            false
        }
    }

    /// Returns the underlying sorted slice.
    #[must_use]
    pub fn as_slice(&self) -> &[CalendarDate] {
        &self.dates
    }
}

impl FromIterator<CalendarDate> for SelectedDates {
    fn from_iter<I: IntoIterator<Item = CalendarDate>>(iter: I) -> Self {
        let mut set = Self::new();

        for date in iter {
            set.insert(date);
        }

        set
    }
}

impl<'a> IntoIterator for &'a SelectedDates {
    type Item = &'a CalendarDate;
    type IntoIter = core::slice::Iter<'a, CalendarDate>;

    fn into_iter(self) -> Self::IntoIter {
        self.dates.iter()
    }
}

/// Predicate type for marking dates unavailable.
///
/// Unavailable dates remain focusable for keyboard navigation but cannot
/// be selected. Wrapped in [`Callback`] so applications can supply
/// closures with captured state (allowlists, computed holiday tables,
/// etc.); the `Arc`-backed pointer equality keeps `Props` `Clone +
/// PartialEq` without the function-pointer comparison foot-gun.
pub type IsDateUnavailableFn = Callback<dyn for<'a> Fn(&'a CalendarDate) -> bool + Send + Sync>;

/// Props for the `Calendar` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID. The adapter supplies a hydration-stable
    /// base; [`ComponentIds`] derives part IDs from it.
    pub id: String,

    /// Controlled single-date value. `None` means uncontrolled;
    /// `Some(None)` means controlled-and-empty; `Some(Some(date))` means
    /// controlled with a date.
    pub value: Option<Option<CalendarDate>>,

    /// Default date for uncontrolled single-select mode.
    pub default_value: Option<CalendarDate>,

    /// Controlled multi-selection set. `None` means uncontrolled.
    pub selected_dates: Option<SelectedDates>,

    /// Default selection set for uncontrolled multi-select mode.
    pub default_selected_dates: SelectedDates,

    /// Selection mode — single by default. Multi-select unlocks the
    /// `selected_dates` set and the `ToggleDate` event.
    pub selection_mode: SelectionMode,

    /// Maximum number of dates that can be selected in `Multiple` mode.
    /// `None` removes the cap. Excess toggles are silently dropped.
    pub max_selected: Option<usize>,

    /// Minimum selectable date.
    pub min: Option<CalendarDate>,

    /// Maximum selectable date.
    pub max: Option<CalendarDate>,

    /// Whether the entire calendar is non-interactive.
    pub disabled: bool,

    /// Whether the calendar allows navigation but blocks selection.
    pub readonly: bool,

    /// Predicate returning `true` for dates that should be marked
    /// unavailable (focusable, but not selectable).
    pub is_date_unavailable: Option<IsDateUnavailableFn>,

    /// Explicit override of the locale's default first day of week.
    /// `None` derives the value from the active [`IntlBackend`].
    pub first_day_of_week: Option<Weekday>,

    /// Whether the head row exposes ISO week numbers (rendered by the
    /// adapter; the machine only forwards this flag to the connect API).
    pub show_week_numbers: bool,

    /// Right-to-left layout direction (mirrors the grid and swaps the
    /// `ArrowLeft`/`ArrowRight` semantics).
    pub is_rtl: bool,

    /// Number of months to display side-by-side. Values below 1 are
    /// clamped to 1 by [`Machine::init`].
    pub visible_months: usize,

    /// Controls navigation step size.
    pub page_behavior: PageBehavior,

    /// The "today" date, injected by the adapter for testability and SSR
    /// determinism.
    pub today: CalendarDate,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            selected_dates: None,
            default_selected_dates: SelectedDates::new(),
            selection_mode: SelectionMode::Single,
            max_selected: None,
            min: None,
            max: None,
            disabled: false,
            readonly: false,
            is_date_unavailable: None,
            first_day_of_week: None,
            show_week_numbers: false,
            is_rtl: false,
            visible_months: 1,
            page_behavior: PageBehavior::Visible,
            today: CalendarDate::new_gregorian(2025, 1, 1)
                .expect("2025-01-01 is a valid Gregorian date"),
        }
    }
}

impl Props {
    /// Creates a new `Props` with defaults plus the supplied ID.
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

    /// Sets [`value`](Self::value) (controlled single-select value).
    #[must_use]
    pub fn value(mut self, value: Option<CalendarDate>) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets [`default_value`](Self::default_value).
    #[must_use]
    pub fn default_value(mut self, value: Option<CalendarDate>) -> Self {
        self.default_value = value;
        self
    }

    /// Sets [`selected_dates`](Self::selected_dates) (controlled multi-select).
    #[must_use]
    pub fn selected_dates(mut self, dates: Option<SelectedDates>) -> Self {
        self.selected_dates = dates;
        self
    }

    /// Sets [`default_selected_dates`](Self::default_selected_dates).
    #[must_use]
    pub fn default_selected_dates(mut self, dates: SelectedDates) -> Self {
        self.default_selected_dates = dates;
        self
    }

    /// Sets [`selection_mode`](Self::selection_mode).
    #[must_use]
    pub const fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Sets [`max_selected`](Self::max_selected).
    #[must_use]
    pub const fn max_selected(mut self, max: Option<usize>) -> Self {
        self.max_selected = max;
        self
    }

    /// Sets [`min`](Self::min).
    #[must_use]
    pub fn min(mut self, date: Option<CalendarDate>) -> Self {
        self.min = date;
        self
    }

    /// Sets [`max`](Self::max).
    #[must_use]
    pub fn max(mut self, date: Option<CalendarDate>) -> Self {
        self.max = date;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`readonly`](Self::readonly).
    #[must_use]
    pub const fn readonly(mut self, value: bool) -> Self {
        self.readonly = value;
        self
    }

    /// Sets [`is_date_unavailable`](Self::is_date_unavailable).
    #[must_use]
    pub fn is_date_unavailable(mut self, predicate: Option<IsDateUnavailableFn>) -> Self {
        self.is_date_unavailable = predicate;
        self
    }

    /// Sets [`first_day_of_week`](Self::first_day_of_week).
    #[must_use]
    pub const fn first_day_of_week(mut self, weekday: Option<Weekday>) -> Self {
        self.first_day_of_week = weekday;
        self
    }

    /// Sets [`show_week_numbers`](Self::show_week_numbers).
    #[must_use]
    pub const fn show_week_numbers(mut self, value: bool) -> Self {
        self.show_week_numbers = value;
        self
    }

    /// Sets [`is_rtl`](Self::is_rtl).
    #[must_use]
    pub const fn is_rtl(mut self, value: bool) -> Self {
        self.is_rtl = value;
        self
    }

    /// Sets [`visible_months`](Self::visible_months).
    #[must_use]
    pub const fn visible_months(mut self, count: usize) -> Self {
        self.visible_months = count;
        self
    }

    /// Sets [`page_behavior`](Self::page_behavior).
    #[must_use]
    pub const fn page_behavior(mut self, behavior: PageBehavior) -> Self {
        self.page_behavior = behavior;
        self
    }

    /// Sets [`today`](Self::today).
    #[must_use]
    pub fn today(mut self, date: CalendarDate) -> Self {
        self.today = date;
        self
    }
}

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// `MessageFn` carrying a locale-only label closure.
pub type LocaleLabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// `MessageFn` carrying a step-count plus locale label closure (used by the
/// multi-month prev/next page labels).
pub type PageLabelFn = dyn Fn(usize, &Locale) -> String + Send + Sync;

/// Locale-specific labels for the `Calendar` component.
///
/// Month names, weekday abbreviations, and full weekday names are resolved
/// from the active [`IntlBackend`] — they are not stored here.
#[derive(Clone)]
pub struct Messages {
    /// Accessible label for the previous-month button (single-month nav).
    pub prev_month_label: MessageFn<LocaleLabelFn>,

    /// Accessible label for the next-month button (single-month nav).
    pub next_month_label: MessageFn<LocaleLabelFn>,

    /// Accessible label for the previous-page button when navigating
    /// multiple months at once. The `usize` parameter is the step count.
    pub prev_page_label: MessageFn<PageLabelFn>,

    /// Accessible label for the next-page button when navigating multiple
    /// months at once. The `usize` parameter is the step count.
    pub next_page_label: MessageFn<PageLabelFn>,

    /// Separator inserted between month names in a multi-month range
    /// heading (e.g., `" – "`).
    pub month_range_separator: MessageFn<LocaleLabelFn>,

    /// Suffix appended to a date cell's `aria-label` when the date is
    /// marked **unavailable** by the user predicate (focusable but not
    /// selectable). The full label reads
    /// `"{month} {day}, {year}, {weekday} {unavailable_suffix}"` with a
    /// leading space inserted automatically — supply the parenthesised
    /// or bracketed phrase verbatim (default: `"(unavailable)"`).
    pub unavailable_suffix: MessageFn<LocaleLabelFn>,

    /// Suffix appended to a date cell's `aria-label` when the date is
    /// **disabled** by the configured `min`/`max` range. See
    /// [`Messages::unavailable_suffix`] for the formatting contract
    /// (default: `"(disabled)"`).
    pub disabled_suffix: MessageFn<LocaleLabelFn>,
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
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// Context for the `Calendar` component.
#[derive(Clone)]
pub struct Context {
    /// Selected date in single-select mode.
    pub value: Bindable<Option<CalendarDate>>,

    /// Selected date set in multi-select mode.
    pub selected_dates: Bindable<SelectedDates>,

    /// Active selection mode.
    pub selection_mode: SelectionMode,

    /// Maximum number of dates the multi-selection set may hold.
    pub max_selected: Option<usize>,

    /// The date that currently holds keyboard focus.
    pub focused_date: CalendarDate,

    /// The first visible month (1-based).
    pub visible_month: u8,

    /// The year of the first visible month.
    pub visible_year: i32,

    /// Number of months displayed side-by-side (>= 1).
    pub visible_months: usize,

    /// Navigation step size for prev/next.
    pub page_behavior: PageBehavior,

    /// Minimum selectable date.
    pub min: Option<CalendarDate>,

    /// Maximum selectable date.
    pub max: Option<CalendarDate>,

    /// Resolved first-day-of-week (locale default unless overridden).
    pub first_day_of_week: Weekday,

    /// Whether the calendar is rendered right-to-left.
    pub is_rtl: bool,

    /// Whether the calendar is globally disabled.
    pub disabled: bool,

    /// Whether the calendar is read-only (focus OK, selection blocked).
    pub readonly: bool,

    /// Whether the head row exposes ISO week numbers.
    pub show_week_numbers: bool,

    /// Date passed in as "today" by the adapter; used to mark today's
    /// cell with `data-ars-today`.
    pub today: CalendarDate,

    /// Predicate marking dates as unavailable. Unavailable dates remain
    /// focusable but are not selectable.
    pub is_date_unavailable_fn: Option<IsDateUnavailableFn>,

    /// Resolved locale (from `Env`).
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Backend used for locale-dependent labels.
    pub intl_backend: Arc<dyn IntlBackend>,

    /// Component part IDs derived from `props.id`.
    pub ids: ComponentIds,
}

impl Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("value", &self.value)
            .field("selected_dates", &self.selected_dates)
            .field("selection_mode", &self.selection_mode)
            .field("max_selected", &self.max_selected)
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
            .field("locale", &self.locale)
            .field("messages", &self.messages)
            .field("intl_backend", &"<dyn IntlBackend>")
            .field("ids", &self.ids)
            .finish()
    }
}

impl Context {
    /// Whether `date` equals the currently selected single-select value.
    #[must_use]
    pub fn is_selected_single(&self, date: &CalendarDate) -> bool {
        self.value.get().as_ref() == Some(date)
    }

    /// Whether `date` is in the multi-selection set.
    #[must_use]
    pub fn is_selected_multi(&self, date: &CalendarDate) -> bool {
        self.selected_dates.get().contains(date)
    }

    /// Whether `date` is selected under the active selection mode.
    #[must_use]
    pub fn is_selected(&self, date: &CalendarDate) -> bool {
        match self.selection_mode {
            SelectionMode::Single => self.is_selected_single(date),
            SelectionMode::Multiple => self.is_selected_multi(date),
        }
    }

    /// Whether `date` is disabled (out of min/max or globally disabled).
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

    /// Whether the user's predicate marks `date` as unavailable.
    #[must_use]
    pub fn is_date_unavailable(&self, date: &CalendarDate) -> bool {
        self.is_date_unavailable_fn
            .as_ref()
            .is_some_and(|predicate| predicate(date))
    }

    /// Clamps `date` into the configured `[min, max]` range.
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

    /// If the focused date has moved outside the visible month range,
    /// scrolls the visible months so the focused date is visible. No-op
    /// when the focused date already lies within the visible range.
    pub fn sync_visible_to_focused(&mut self) {
        if !grid_is_in_visible_range(
            &self.focused_date,
            self.visible_month,
            self.visible_year,
            self.visible_months,
        ) {
            self.visible_month = self.focused_date.month();
            self.visible_year = self.focused_date.year();
        }
    }

    /// Returns `(month, year)` for the visible month at `offset` from the
    /// first visible month.
    #[must_use]
    pub const fn month_year_at_offset(&self, offset: usize) -> (u8, i32) {
        grid_month_year_at_offset(self.visible_month, self.visible_year, offset)
    }

    /// Whether `date` belongs to a month other than the month at the
    /// given visible-month offset.
    #[must_use]
    pub const fn is_outside_month_at_offset(&self, date: &CalendarDate, offset: usize) -> bool {
        grid_is_outside_month_at_offset(date, self.visible_month, self.visible_year, offset)
    }

    /// Whether `date`'s month/year matches the first visible month.
    #[must_use]
    pub const fn is_outside_visible_month(&self, date: &CalendarDate) -> bool {
        date.month() != self.visible_month || date.year() != self.visible_year
    }

    /// Whether `date` falls within any of the visible months.
    #[must_use]
    pub fn is_in_visible_range(&self, date: &CalendarDate) -> bool {
        grid_is_in_visible_range(
            date,
            self.visible_month,
            self.visible_year,
            self.visible_months,
        )
    }

    /// Builds the 6-week grid for the first visible month.
    #[must_use]
    pub fn weeks(&self) -> Vec<[CalendarDate; 7]> {
        self.weeks_for(0)
    }

    /// Builds the 6-week grid for the month at the supplied offset.
    #[must_use]
    pub fn weeks_for(&self, offset: usize) -> Vec<[CalendarDate; 7]> {
        grid_weeks_for(
            self.visible_month,
            self.visible_year,
            self.first_day_of_week,
            offset,
        )
    }

    /// Returns the ordered weekday list starting at `first_day_of_week`.
    #[must_use]
    pub fn ordered_weekdays(&self) -> [Weekday; 7] {
        grid_ordered_weekdays(self.first_day_of_week)
    }

    /// Ordered `(weekday, short_label)` pairs for the head row.
    #[must_use]
    pub fn week_day_labels(&self) -> Vec<(Weekday, String)> {
        self.ordered_weekdays()
            .into_iter()
            .map(|wd| (wd, self.intl_backend.weekday_short_label(wd, &self.locale)))
            .collect()
    }

    /// Advances `(visible_month, visible_year)` by `n` whole months.
    pub const fn advance_month(&mut self, n: i32) {
        let (month, year) = advance_month_pair(self.visible_month, self.visible_year, n);

        self.visible_month = month;
        self.visible_year = year;
    }
}

// ────────────────────────────────────────────────────────────────────
// Part / Machine
// ────────────────────────────────────────────────────────────────────

/// Parts of the `Calendar` anatomy. See spec §2.
#[derive(ars_core::ComponentPart)]
#[scope = "calendar"]
pub enum Part {
    /// The outer component root.
    Root,

    /// The header container holding the prev/heading/next controls.
    Header,

    /// The "previous month" button.
    PrevTrigger,

    /// The "next month" button.
    NextTrigger,

    /// The textual month/year heading.
    Heading,

    /// The grid wrapper element (a single `<table>` per visible month).
    Grid,

    /// Wrapper grouping multiple grids when `visible_months > 1`.
    GridGroup,

    /// The header row of weekday labels.
    HeadRow,

    /// Individual weekday header cell.
    HeadCell {
        /// The weekday this cell describes.
        #[part(default = Weekday::Monday)]
        day: Weekday,
    },

    /// A row of seven day cells (one week).
    Row {
        /// Zero-based index of this week within the grid (0..6).
        week_index: usize,
    },

    /// A grid cell wrapping one date.
    ///
    /// `offset` is the multi-month grid index this cell belongs to
    /// (0 for the first/only visible month, 1 for the second, …). It
    /// drives the offset-aware `outside-month` check inside
    /// [`Api::part_attrs`] so multi-month layouts don't mislabel later
    /// grids' dates as outside-month.
    Cell {
        /// The date represented by this cell.
        #[part(default = default_calendar_date())]
        date: CalendarDate,
        /// Zero-based offset of this cell's grid within the visible
        /// month range. `0` is the default (single-month or first grid).
        offset: usize,
    },

    /// The interactive trigger inside a cell (the actual `<button>`).
    ///
    /// `offset` carries the same multi-month grid index as [`Part::Cell`].
    CellTrigger {
        /// The date represented by this trigger.
        #[part(default = default_calendar_date())]
        date: CalendarDate,
        /// Zero-based offset of this cell's grid within the visible
        /// month range. `0` is the default (single-month or first grid).
        offset: usize,
    },
}

fn default_calendar_date() -> CalendarDate {
    CalendarDate::new_gregorian(1, 1, 1).expect("1-01-01 is a valid Gregorian date")
}

/// Machine for the `Calendar` component.
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

        let selected_dates = if let Some(set) = &props.selected_dates {
            Bindable::controlled(set.clone())
        } else {
            Bindable::uncontrolled(props.default_selected_dates.clone())
        };

        let mut initial_date = match props.selection_mode {
            SelectionMode::Single => value.get().clone().unwrap_or_else(|| props.today.clone()),
            SelectionMode::Multiple => selected_dates
                .get()
                .iter()
                .next()
                .cloned()
                .unwrap_or_else(|| props.today.clone()),
        };

        // Clamp the initial focused date into the configured [min, max]
        // range so the roving-tabindex target on first mount is never on
        // a disabled cell (which would also be HTML-disabled, removing
        // any focusable day target from the grid).
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
        let ids = ComponentIds::from_id(&props.id);
        let focused_date = initial_date;
        let visible_month = focused_date.month();
        let visible_year = focused_date.year();

        let ctx = Context {
            value,
            selected_dates,
            selection_mode: props.selection_mode,
            max_selected: props.max_selected,
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
            locale,
            messages: messages.clone(),
            intl_backend: Arc::clone(&env.intl_backend),
            ids,
        };

        (State::Idle, ctx)
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        debug_assert_eq!(
            old.id, new.id,
            "calendar::Props.id must remain stable after init",
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
        // `FocusOut` must run even when the calendar is disabled — a
        // calendar that becomes disabled while focused still needs its
        // blur cleanup to land, otherwise it stays stuck in
        // `State::Focused` with stale `data-ars-state="focused"`. `SyncProps`
        // must also flow through so parent-driven changes that *take the
        // calendar out of* the disabled state can actually land.
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

            // `FocusOut` and `SyncProps` are handled above the disabled
            // guard. Control never reaches these arms — they exist purely
            // to satisfy match exhaustiveness over `Event`.
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

            Event::SelectDate { date } | Event::ToggleDate { date } => match ctx.selection_mode {
                SelectionMode::Single => apply_select_single(ctx, date.clone()),
                SelectionMode::Multiple => apply_toggle_multi(ctx, date.clone()),
            },

            Event::NextMonth => month_step_plan(ctx, step_for_page_behavior(ctx)),

            Event::PrevMonth => month_step_plan(ctx, -step_for_page_behavior(ctx)),

            Event::NextYear => month_step_plan(ctx, 12),

            Event::PrevYear => month_step_plan(ctx, -12),

            Event::SetMonth { month } => {
                let month = *month;
                if !(1..=12).contains(&month) {
                    return None;
                }

                // Pre-compute the focused-date shift so a boundary
                // failure short-circuits the entire transition (no
                // spurious `AnnounceMonth`). The `delta` here ranges
                // over -11..=11 (u8 differences), no overflow risk.
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
                        // If the clamp pushed focus into another month
                        // (e.g., `SetMonth { month: 6 }` with `max = Jan 25`
                        // clamps back to Jan 25), drag the visible window
                        // along so the focused cell remains rendered.
                        ctx.sync_visible_to_focused();
                    })
                    .with_effect(announce_month_effect()),
                )
            }

            Event::SetYear { year } => {
                let year = *year;
                // `i32 - i32` overflows on extreme inputs
                // (`SetYear { year: i32::MIN }` with positive
                // `visible_year`); bail out via `checked_sub`.
                let delta_years = year.checked_sub(ctx.visible_year)?;
                // Pre-compute the shift so boundary failure → no transition
                // → no `AnnounceMonth`.
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
    // Push controlled values through their `Bindable`s. `sync_controlled`
    // accepts `Option<T>`: `Some(_)` keeps the calendar in controlled
    // mode, `None` flips it back to uncontrolled (revealing the internal
    // value).
    ctx.value.sync_controlled(props.value.clone());
    ctx.selected_dates
        .sync_controlled(props.selected_dates.clone());

    ctx.selection_mode = props.selection_mode;
    ctx.max_selected = props.max_selected;
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

    // `first_day_of_week` is the only field that can come from *either*
    // props or the locale-derived default. An explicit override always
    // wins; clearing the override (`Some(_) → None`) restores the
    // locale-derived value via the cached `intl_backend`, otherwise
    // context would keep the stale weekday in violation of the Props
    // contract.
    ctx.first_day_of_week = props
        .first_day_of_week
        .unwrap_or_else(|| ctx.locale.first_day_of_week(&*ctx.intl_backend));

    // Re-clamp `focused_date` in case `min`/`max` tightened so the roving
    // target never points at an out-of-range cell, then drag the visible
    // window along if the clamp moved focus into a different month.
    let clamped = ctx.clamp_date(ctx.focused_date.clone());
    ctx.focused_date = clamped;
    ctx.sync_visible_to_focused();
}

fn step_for_page_behavior(ctx: &Context) -> i32 {
    match ctx.page_behavior {
        PageBehavior::Visible => i32::try_from(ctx.visible_months).unwrap_or(1).max(1),
        PageBehavior::Single => 1,
    }
}

fn month_step_plan(ctx: &Context, step: i32) -> Option<TransitionPlan<Machine>> {
    // Pre-compute the focused-date shift at plan-build time so a
    // boundary failure short-circuits the whole transition (no spurious
    // `AnnounceMonth` effect). Returning `None` here propagates as "no
    // change" to the machine — visible_month, focused_date, AND the
    // announce effect all stay put.
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
            // If the clamp pushed focus into a month different from the
            // newly-advanced visible window (e.g., NextMonth when
            // focused_date is at `max` already), pull visible back to
            // wherever the clamped focus lives. Without this the
            // roving-tabindex target would have no rendered cell.
            ctx.sync_visible_to_focused();
        })
        .with_effect(announce_month_effect()),
    )
}

fn announce_month_effect() -> ars_core::PendingEffect<Machine> {
    ars_core::PendingEffect::named(Effect::AnnounceMonth)
}

fn apply_select_single(ctx: &Context, date: CalendarDate) -> Option<TransitionPlan<Machine>> {
    if ctx.readonly {
        return None;
    }

    if ctx.is_date_disabled(&date) || ctx.is_date_unavailable(&date) {
        return None;
    }

    Some(
        TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
            ctx.value.set(Some(date.clone()));
            ctx.focused_date = date;
        }),
    )
}

fn apply_toggle_multi(ctx: &Context, date: CalendarDate) -> Option<TransitionPlan<Machine>> {
    if ctx.readonly {
        return None;
    }

    if ctx.is_date_disabled(&date) || ctx.is_date_unavailable(&date) {
        return None;
    }

    let already_selected = ctx.selected_dates.get().contains(&date);
    let max_selected = ctx.max_selected;

    let at_cap =
        !already_selected && max_selected.is_some_and(|cap| ctx.selected_dates.get().len() >= cap);

    if at_cap {
        // Excess toggles are silently dropped per spec §5.4.
        return None;
    }

    Some(
        TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
            let mut next = ctx.selected_dates.get().clone();

            if already_selected {
                next.remove(&date);
            } else {
                next.insert(date.clone());
            }

            ctx.selected_dates.set(next);
            ctx.focused_date = date;
        }),
    )
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

    let new_focus: Option<CalendarDate> = if key == prev_day_key {
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

            let select_event = match ctx.selection_mode {
                SelectionMode::Single => Event::SelectDate { date },
                SelectionMode::Multiple => Event::ToggleDate { date },
            };

            Some(TransitionPlan::context_only(|_ctx: &mut Context| {}).then(select_event))
        }

        _ => None,
    }
}

// ────────────────────────────────────────────────────────────────────
// Api
// ────────────────────────────────────────────────────────────────────

/// API for the `Calendar` component.
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
    /// Creates a new `Calendar` connect API from machine state.
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

    // ── Root / structural attributes ─────────────────────────────────────

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

    /// Returns attributes for the previous-month / previous-page button.
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
            // `type="button"` keeps a click from submitting an enclosing
            // <form>. Without it the browser default (`type="submit"`)
            // turns prev/next paging into form submission.
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

    /// Returns attributes for the next-month / next-page button.
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
            // See `prev_trigger_attrs` — explicit `type="button"` prevents
            // an enclosing <form> from auto-submitting on nav clicks.
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

    /// Returns attributes for the main month/year heading. The heading
    /// is the `aria-live="polite"` announcer for month navigation.
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

    /// Formatted month/year string for the main heading. Multi-month
    /// configurations render the full range via [`Api::range_heading_text`];
    /// this method always describes the first visible month only.
    #[must_use]
    pub fn heading_text(&self) -> String {
        format!(
            "{} {}",
            self.ctx
                .intl_backend
                .month_long_name(self.ctx.visible_month, &self.ctx.locale),
            self.ctx.visible_year,
        )
    }

    /// Attributes for the grid wrapper of the first visible month.
    #[must_use]
    pub fn grid_attrs(&self) -> AttrMap {
        self.grid_attrs_with_ids(self.ctx.ids.part("grid"), self.ctx.ids.part("heading"))
    }

    /// Attributes for the grid of the month at the supplied offset.
    /// Multi-month layouts use the per-grid heading id rather than the
    /// shared live-region heading.
    #[must_use]
    pub fn grid_attrs_for(&self, offset: usize) -> AttrMap {
        let grid_id = self.ctx.ids.item("grid", &offset);
        let heading_id = self.ctx.ids.item("heading", &offset);

        self.grid_attrs_with_ids(grid_id, heading_id)
    }

    fn grid_attrs_with_ids(&self, grid_id: String, heading_id: String) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Grid.data_attrs();

        attrs
            .set(HtmlAttr::Id, grid_id)
            .set(HtmlAttr::Role, "grid")
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::LabelledBy), heading_id);

        if matches!(self.ctx.selection_mode, SelectionMode::Multiple) {
            attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        }

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Attributes for the multi-grid wrapper. Only meaningful when
    /// `visible_months > 1`; single-month layouts skip rendering the
    /// wrapper entirely and use [`Api::grid_attrs`] directly.
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

    /// Heading attributes for a per-grid (offset > 0) heading. Per-grid
    /// headings are visually hidden and intentionally have no `aria-live`
    /// — only the main [`Api::heading_attrs`] heading announces month
    /// changes to avoid duplicate announcements.
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

    /// Heading text for the month at the supplied offset.
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

    /// Heading text that spans every visible month. Equivalent to
    /// [`Api::heading_text`] when `visible_months <= 1`.
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

    /// Attributes for the head row (weekday labels).
    #[must_use]
    pub fn head_row_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HeadRow.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Attributes for a single weekday header cell.
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

    /// Attributes for a body row, including the `data-ars-week-index`
    /// hook adapters use for keyed reconciliation.
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

    /// Attributes for a cell wrapper around a date trigger.
    #[must_use]
    pub fn cell_attrs(&self, date: &CalendarDate) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Cell {
            date: date.clone(),
            offset: 0,
        }
        .data_attrs();

        attrs
            .set(HtmlAttr::Role, "gridcell")
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        if self.is_selected(date) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Selected), "true");
        }

        if self.ctx.is_outside_visible_month(date) {
            attrs.set_bool(HtmlAttr::Data("ars-outside-month"), true);
        }

        if self.is_disabled(date) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Like [`Api::cell_attrs`] but checks "outside month" against the
    /// month at the supplied offset rather than the first visible month.
    /// Multi-month layouts use this so cells render the correct
    /// outside-month flag for each grid.
    #[must_use]
    pub fn cell_attrs_for(&self, date: &CalendarDate, offset: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Cell {
            date: date.clone(),
            offset,
        }
        .data_attrs();

        attrs
            .set(HtmlAttr::Role, "gridcell")
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        if self.is_selected(date) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Selected), "true");
        }

        if self.ctx.is_outside_month_at_offset(date, offset) {
            attrs.set_bool(HtmlAttr::Data("ars-outside-month"), true);
        }

        if self.is_disabled(date) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Attributes for the interactive trigger inside a cell.
    #[must_use]
    pub fn cell_trigger_attrs(&self, date: &CalendarDate) -> AttrMap {
        self.cell_trigger_attrs_inner(date, None)
    }

    /// Like [`Api::cell_trigger_attrs`] but checks "outside month" against
    /// the month at the supplied offset rather than the first visible
    /// month. Multi-month layouts use this so triggers in later grids do
    /// not get flagged `data-ars-outside-month` just because they belong
    /// to a different month than offset 0.
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
            // `type="button"` keeps a click on a date inside a <form>
            // from submitting; without it browsers default to submit.
            .set(HtmlAttr::Type, "button");

        let disabled = self.is_disabled(date);
        let unavailable = self.is_unavailable(date);
        let is_focused = &self.ctx.focused_date == date;
        let is_today = self.is_today(date);
        let selected = self.is_selected(date);

        attrs.set(HtmlAttr::TabIndex, if is_focused { "0" } else { "-1" });

        // `aria-disabled` announces the semantic restriction for both
        // out-of-range (disabled) and unavailable dates. HTML `disabled`
        // is **only** set for the disabled case because it removes the
        // element from the browser focus model; unavailable dates must
        // remain focusable per spec §3 even though they cannot be selected.
        if disabled || unavailable {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if selected {
            attrs.set(HtmlAttr::Aria(AriaAttr::Selected), "true");
        }

        if is_today {
            attrs.set_bool(HtmlAttr::Data("ars-today"), true);
        }

        let outside_month = match offset {
            Some(o) => self.ctx.is_outside_month_at_offset(date, o),
            None => self.ctx.is_outside_visible_month(date),
        };
        if outside_month {
            attrs.set_bool(HtmlAttr::Data("ars-outside-month"), true);
        }

        if unavailable {
            attrs.set_bool(HtmlAttr::Data("ars-unavailable"), true);
        }

        let base_label = format!(
            "{} {}, {}, {}",
            self.ctx
                .intl_backend
                .month_long_name(date.month(), &self.ctx.locale),
            date.day(),
            date.year(),
            self.ctx
                .intl_backend
                .weekday_long_label(date.weekday(), &self.ctx.locale),
        );

        // Status suffix flows through `Messages` so it can be localized
        // (e.g., German `(nicht verfügbar)`); the leading space joins the
        // base label and the suffix automatically.
        let label = if unavailable {
            let suffix = (self.ctx.messages.unavailable_suffix)(&self.ctx.locale);
            format!("{base_label} {suffix}")
        } else if disabled {
            let suffix = (self.ctx.messages.disabled_suffix)(&self.ctx.locale);
            format!("{base_label} {suffix}")
        } else {
            base_label
        };

        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);

        attrs
    }

    // ── Computed state accessors ─────────────────────────────────────

    /// Whether `date` is the currently selected single-select value or
    /// is in the multi-select set, depending on `selection_mode`.
    #[must_use]
    pub fn is_selected(&self, date: &CalendarDate) -> bool {
        self.ctx.is_selected(date)
    }

    /// Whether `date` equals the configured `today`.
    #[must_use]
    pub fn is_today(&self, date: &CalendarDate) -> bool {
        date == &self.ctx.today
    }

    /// Whether `date` is disabled (outside min/max or globally disabled).
    #[must_use]
    pub fn is_disabled(&self, date: &CalendarDate) -> bool {
        self.ctx.is_date_disabled(date)
    }

    /// Whether `date` is marked unavailable by the user predicate.
    #[must_use]
    pub fn is_unavailable(&self, date: &CalendarDate) -> bool {
        self.ctx.is_date_unavailable(date)
    }

    /// Whether `date` belongs to a different month than the first visible
    /// month.
    #[must_use]
    pub const fn is_outside_visible_month(&self, date: &CalendarDate) -> bool {
        self.ctx.is_outside_visible_month(date)
    }

    /// Whether the prev-trigger should be disabled — either the calendar
    /// is globally disabled or stepping back by `page_behavior`'s step
    /// would yield a visible range with no selectable dates ≥ `min`.
    ///
    /// The check honours [`PageBehavior::Single`] vs [`PageBehavior::Visible`]:
    /// with multi-month + `Single`, paging back by one month may still
    /// produce a range that contains selectable dates even if the current
    /// first visible month already starts at-or-before `min`.
    #[must_use]
    pub fn is_prev_disabled(&self) -> bool {
        if self.ctx.disabled {
            return true;
        }

        // Compute the LAST visible month after pressing prev. If the
        // arithmetic fails (calendar boundary), `month_step_plan` will
        // drop the click — so we must surface that as a disabled
        // control rather than leaving a clickable no-op.
        let step = self.nav_step_size();
        let (new_first_month, new_first_year) = advance_month_pair(
            self.ctx.visible_month,
            self.ctx.visible_year,
            -i32::try_from(step).unwrap_or(1),
        );
        let last_offset = self.ctx.visible_months.saturating_sub(1);
        let (new_last_month, new_last_year) =
            grid_month_year_at_offset(new_first_month, new_first_year, last_offset);

        let Ok(first_of_new_last) = CalendarDate::new_gregorian(new_last_year, new_last_month, 1)
        else {
            return true;
        };
        let days_in_month = first_of_new_last.days_in_month();
        let Ok(last_of_new_last) =
            CalendarDate::new_gregorian(new_last_year, new_last_month, days_in_month)
        else {
            return true;
        };

        // Past the representable range, also check the min constraint:
        // if the post-step page is entirely below `min`, disable.
        let Some(min) = &self.ctx.min else {
            return false;
        };
        matches!(last_of_new_last.compare(min), Ordering::Less)
    }

    /// Whether the next-trigger should be disabled — either the calendar
    /// is globally disabled or stepping forward by `page_behavior`'s step
    /// would yield a visible range with no selectable dates ≤ `max`.
    ///
    /// Symmetric to [`Api::is_prev_disabled`]: the check honours
    /// [`PageBehavior::Single`] vs [`PageBehavior::Visible`] so multi-month
    /// + single-step paging doesn't disable too early.
    #[must_use]
    pub fn is_next_disabled(&self) -> bool {
        if self.ctx.disabled {
            return true;
        }

        // Compute the FIRST visible month after pressing next. If the
        // arithmetic fails (calendar boundary), `month_step_plan` will
        // drop the click — so surface that as disabled rather than a
        // clickable no-op.
        let step = self.nav_step_size();
        let (new_first_month, new_first_year) = advance_month_pair(
            self.ctx.visible_month,
            self.ctx.visible_year,
            i32::try_from(step).unwrap_or(1),
        );
        let Ok(first_of_new_first) =
            CalendarDate::new_gregorian(new_first_year, new_first_month, 1)
        else {
            return true;
        };

        // Past representable range, also check the max constraint.
        let Some(max) = &self.ctx.max else {
            return false;
        };
        matches!(first_of_new_first.compare(max), Ordering::Greater)
    }

    /// Returns the configured "today" date.
    #[must_use]
    pub const fn today(&self) -> &CalendarDate {
        &self.ctx.today
    }

    /// Returns the currently focused date.
    #[must_use]
    pub const fn focused_date(&self) -> &CalendarDate {
        &self.ctx.focused_date
    }

    /// Whether the calendar grid currently has focus.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        matches!(self.state, State::Focused)
    }

    /// Number of months visible side-by-side (>= 1).
    #[must_use]
    pub const fn visible_month_count(&self) -> usize {
        self.ctx.visible_months
    }

    /// Iterator of month offsets `0..visible_months` for multi-month
    /// rendering.
    #[must_use]
    pub const fn month_offsets(&self) -> core::ops::Range<usize> {
        0..self.ctx.visible_months
    }

    /// Whether week numbers should be rendered.
    #[must_use]
    pub const fn show_week_numbers(&self) -> bool {
        self.ctx.show_week_numbers
    }

    /// Active selection mode.
    #[must_use]
    pub const fn selection_mode(&self) -> SelectionMode {
        self.ctx.selection_mode
    }

    /// 6-week grid for the first visible month.
    #[must_use]
    pub fn weeks(&self) -> Vec<[CalendarDate; 7]> {
        self.ctx.weeks()
    }

    /// 6-week grid for the month at the supplied offset.
    #[must_use]
    pub fn weeks_for(&self, offset: usize) -> Vec<[CalendarDate; 7]> {
        self.ctx.weeks_for(offset)
    }

    /// Ordered `(weekday, short-label)` pairs for the head row.
    #[must_use]
    pub fn week_day_labels(&self) -> Vec<(Weekday, String)> {
        self.ctx.week_day_labels()
    }

    // ── Event handlers ───────────────────────────────────────────────

    /// Handle a click on a date cell.
    pub fn on_cell_click(&self, date: CalendarDate) {
        let event = match self.ctx.selection_mode {
            SelectionMode::Single => Event::SelectDate { date },
            SelectionMode::Multiple => Event::ToggleDate { date },
        };

        (self.send)(event);
    }

    /// Handle focus entering the grid.
    pub fn on_grid_focusin(&self) {
        (self.send)(Event::FocusIn);
    }

    /// Handle focus leaving the grid. The adapter passes
    /// `focus_leaving_grid = true` only when focus moves outside the grid
    /// element, so the machine stays focused when arrow keys move between
    /// cells.
    pub fn on_grid_focusout(&self, focus_leaving_grid: bool) {
        if focus_leaving_grid {
            (self.send)(Event::FocusOut);
        }
    }

    /// Handle a keydown on the grid. The adapter folds the shift modifier
    /// into the call so the machine can map `Shift+PageUp` / `Shift+PageDown`
    /// to year navigation in a single transition.
    pub fn on_grid_keydown(&self, key: KeyboardKey, shift: bool) {
        (self.send)(Event::KeyDown { key, shift });
    }

    /// Handle a click on the previous-month button.
    pub fn on_prev_click(&self) {
        (self.send)(Event::PrevMonth);
    }

    /// Handle a click on the next-month button.
    pub fn on_next_click(&self) {
        (self.send)(Event::NextMonth);
    }

    // ── Internal helpers ─────────────────────────────────────────────

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
