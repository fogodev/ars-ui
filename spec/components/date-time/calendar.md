---
component: Calendar
category: date-time
tier: complex
foundation_deps: [architecture, accessibility, i18n, interactions]
shared_deps: [date-time-types]
related: [range-calendar, date-picker, date-range-picker]
references:
    react-aria: Calendar
---

# Calendar

`Calendar` is a grid-based date selection component following the WAI-ARIA Grid pattern. It renders a month view with navigable day cells, supports single-date selection, locale-aware week start, min/max date constraints, and arbitrary unavailability predicates.

> **First Day of Week Rendering:** The calendar grid header row and week rows MUST start on the locale's first day of week, as determined by `WeekInfo::for_locale()` (see `04-internationalization.md` §5.3). The `ordered_weekdays()` method provides the correctly-ordered weekday array for rendering column headers. Test expectations: `ar-SA` → first column is Saturday; `en-US` → first column is Sunday; `de-DE` → first column is Monday. The `u-fw-*` locale extension (e.g., `en-US-u-fw-mon`) overrides the default first day for the locale.

Calendar is used standalone for inline date picking and is embedded inside `DatePicker` and `DateRangePicker` as the popover content. For range selection, see `RangeCalendar` which provides a separate state machine composing Calendar's navigation logic.

## 1. State Machine

### 1.1 States

```rust
/// States for the Calendar component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Calendar is rendered but no cell has keyboard focus.
    Idle,
    /// A specific date cell has keyboard focus within the grid.
    Focused,
}
```

### 1.2 Events

```rust
/// Events for the Calendar component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Move keyboard focus to a specific date (clamped to min/max).
    FocusDate {
        /// The date to focus on.
        date: CalendarDate,
    },
    /// User selected a date via click or `Enter`/`Space` on the focused cell.
    ///
    /// In [`SelectionMode::Single`] this sets the value. In
    /// [`SelectionMode::Multiple`] it falls through to [`Event::ToggleDate`] so
    /// adapters that do not know the active mode can emit one canonical
    /// "select this date" event.
    SelectDate {
        /// The date to select.
        date: CalendarDate,
    },
    /// Toggle the date in or out of the multi-selection set.
    ///
    /// Only meaningful when `selection_mode == Multiple`. A toggle that
    /// would add a date when `selected_dates.len() >= max_selected` is
    /// silently dropped.
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
    /// The `shift` flag is folded into the event so adapters do not need a
    /// side-channel to communicate `Shift+PageUp` / `Shift+PageDown` for
    /// year navigation — the machine maps those to `PrevYear`/`NextYear`
    /// inside its transition function.
    KeyDown {
        /// The key that was pressed.
        key: KeyboardKey,
        /// Whether the shift modifier was held.
        shift: bool,
    },
}
```

### 1.3 Context

[`CalendarDate`] implements neither `Ord` nor `PartialOrd` — different
calendar systems share the same value type so global ordering is undefined,
but it does expose chronological [`CalendarDate::compare`](https://docs.rs/ars-i18n).
Both `is_date_disabled` and `clamp_date` compare via `compare(...)` rather
than `<`/`>`, and the multi-select set lives in a [`SelectedDates`] newtype
(§1.4) that keeps a sorted `Vec<CalendarDate>` instead of a `BTreeSet`.

```rust
/// Context for the Calendar component.
#[derive(Clone, Debug)]
pub struct Context {
    /// Selected date (single-select mode).
    pub value: Bindable<Option<CalendarDate>>,
    /// Selected date set (multi-select mode).
    pub selected_dates: Bindable<SelectedDates>,
    /// Active selection mode.
    pub selection_mode: SelectionMode,
    /// Cap on `selected_dates.len()` in multi-select mode.
    pub max_selected: Option<usize>,
    /// The date that currently holds keyboard focus within the grid.
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
    /// Resolved first day of week (locale default unless overridden by Props).
    pub first_day_of_week: Weekday,
    /// Whether the calendar is rendered right-to-left.
    pub is_rtl: bool,
    /// Whether the calendar is globally disabled.
    pub disabled: bool,
    /// Whether the calendar is read-only (focus OK, selection blocked).
    pub readonly: bool,
    /// Whether the head row exposes ISO week numbers.
    pub show_week_numbers: bool,
    /// Date passed in as "today" by the adapter; used to mark today's cell
    /// with `data-ars-today`.
    pub today: CalendarDate,
    /// User-supplied predicate marking dates unavailable. Unavailable dates
    /// remain focusable but are not selectable. Evaluated lazily — there is
    /// no pre-computed static cache.
    pub is_date_unavailable_fn: Option<fn(&CalendarDate) -> bool>,
    /// Resolved locale (from `Env`).
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Backend used for locale-dependent labels and digits.
    pub intl_backend: Arc<dyn IntlBackend>,
    /// Derived part IDs (`ids.part("heading")`, `ids.part("grid")`, …).
    pub ids: ComponentIds,
}

impl Context {
    /// Whether `date` is selected under the active selection mode.
    pub fn is_selected(&self, date: &CalendarDate) -> bool {
        match self.selection_mode {
            SelectionMode::Single => self.value.get().as_ref() == Some(date),
            SelectionMode::Multiple => self.selected_dates.get().contains(date),
        }
    }

    /// Whether a date is disabled (outside min/max or globally disabled).
    pub fn is_date_disabled(&self, date: &CalendarDate) -> bool {
        if self.disabled { return true; }
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

    /// Whether the user predicate marks `date` as unavailable.
    pub fn is_date_unavailable(&self, date: &CalendarDate) -> bool {
        self.is_date_unavailable_fn
            .is_some_and(|predicate| predicate(date))
    }

    /// Clamp `date` into the configured `[min, max]` range.
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

    /// Ensure the focused date's month is visible.
    /// Only scrolls when the focused date falls outside all visible months.
    pub fn sync_visible_to_focused(&mut self) {
        if !self.is_in_visible_range(&self.focused_date) {
            self.visible_month = self.focused_date.month();
            self.visible_year = self.focused_date.year();
        }
    }
}
```

### 1.4 Props

Multi-select Props (`selection_mode`, `max_selected`, `selected_dates`,
`default_selected_dates`) live directly on `Props`, not in a §5 extension —
the machine is a single unified type that switches behaviour on
`selection_mode` rather than two parallel `Machine` types.

```rust
/// Ordered set of unique calendar dates, sorted by `CalendarDate::compare`.
///
/// `CalendarDate` implements neither `Ord` nor `PartialOrd`, so `BTreeSet`
/// is unavailable. `SelectedDates` is a thin newtype around
/// `Vec<CalendarDate>` that maintains chronological order through binary
/// search with the type's `compare` method.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectedDates {
    dates: Vec<CalendarDate>,
}

impl SelectedDates {
    pub const fn new() -> Self;
    pub const fn len(&self) -> usize;
    pub const fn is_empty(&self) -> bool;
    pub fn iter(&self) -> core::slice::Iter<'_, CalendarDate>;
    pub fn contains(&self, date: &CalendarDate) -> bool;
    pub fn insert(&mut self, date: CalendarDate) -> bool;
    pub fn remove(&mut self, date: &CalendarDate) -> bool;
    pub fn as_slice(&self) -> &[CalendarDate];
}

impl FromIterator<CalendarDate> for SelectedDates { /* … */ }
impl<'a> IntoIterator for &'a SelectedDates { /* … */ }

/// Whether the calendar selects a single date or an unordered set of dates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    #[default]
    Single,
    Multiple,
}

/// Predicate type for marking dates unavailable.
///
/// Wrapped in [`Callback`] so applications can supply closures with
/// captured state (allowlists, computed holiday tables, etc.). `Callback`
/// is `Arc`-backed and compares by pointer identity, so `Props` derives
/// `Clone + PartialEq` cleanly. Construct via
/// [`Callback::new_ref`](ars_core::Callback::new_ref), which accepts a
/// higher-ranked closure over a reference argument — the generic
/// [`Callback::new`](ars_core::Callback::new) constructor cannot satisfy
/// the implicit HRTB lifetime because its `Args: 'static` bound excludes
/// reference types.
pub type IsDateUnavailableFn = Callback<dyn for<'a> Fn(&'a CalendarDate) -> bool + Send + Sync>;

/// Props for the Calendar component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled single-date value.
    pub value: Option<Option<CalendarDate>>,
    /// Default date for uncontrolled single-select mode.
    pub default_value: Option<CalendarDate>,
    /// Controlled multi-selection set.
    pub selected_dates: Option<SelectedDates>,
    /// Default selection set for uncontrolled multi-select mode.
    pub default_selected_dates: SelectedDates,
    /// Selection mode — single by default. Multi-select unlocks
    /// `selected_dates` and the `ToggleDate` event.
    pub selection_mode: SelectionMode,
    /// Maximum number of dates that can be selected in `Multiple` mode.
    /// `None` removes the cap; excess toggles are silently dropped.
    pub max_selected: Option<usize>,
    /// Minimum selectable date.
    pub min: Option<CalendarDate>,
    /// Maximum selectable date.
    pub max: Option<CalendarDate>,
    /// Whether the entire calendar is non-interactive.
    pub disabled: bool,
    /// Whether the calendar allows navigation but not selection.
    pub readonly: bool,
    /// Predicate returning true for dates that should be marked unavailable.
    /// Unavailable dates are focusable but not selectable.
    pub is_date_unavailable: Option<IsDateUnavailableFn>,
    /// Explicit override of the locale's default first day of week.
    /// When `Some`, overrides the locale default. When `None`, derives from
    /// `ars_i18n::Locale` via `WeekInfo::first_day`.
    ///
    /// **CLDR Limitations**: The CLDR `WeekInfo::first_day` data covers most locales but
    /// may not account for religious or cultural contexts where the first day differs from
    /// the national default (e.g., Islamic calendars where Friday is the start of the week
    /// in religious contexts, even though Saturday is the CLDR default for `ar-SA`).
    /// Applications serving these contexts SHOULD use `first_day_of_week: Some(Weekday::Friday)`
    /// explicitly rather than relying on CLDR locale data.
    pub first_day_of_week: Option<Weekday>,
    /// Whether to display ISO week numbers at the start of each calendar row.
    pub show_week_numbers: bool,
    /// Right-to-left layout direction.
    pub is_rtl: bool,
    /// Number of months to display side-by-side. Values below 1 are
    /// clamped to 1 by `Machine::init`. Default: 1.
    pub visible_months: usize,
    /// Controls navigation step size. Default: `PageBehavior::Visible`.
    pub page_behavior: PageBehavior,
    /// The "today" date, injected by the adapter for testability.
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
```

### 1.5 Guards

```rust
fn is_disabled(ctx: &Context) -> bool { ctx.disabled }
fn is_readonly(ctx: &Context) -> bool { ctx.readonly }
fn is_date_disabled(ctx: &Context, date: &CalendarDate) -> bool { ctx.is_date_disabled(date) }
fn is_date_unavailable(ctx: &Context, date: &CalendarDate) -> bool { ctx.is_date_unavailable(date) }
```

### 1.6 Grid Computation

All `CalendarDate` arithmetic in this section uses the real `ars-i18n` API:
`new_gregorian(year, month, day)` takes plain `u8`s and returns
`Result<CalendarDate, CalendarError>`; `add_days(i32)` / `add(DateDuration)`
also return `Result`. Grid anchoring calls cannot fail by construction —
the spec uses `.expect("…")` with explicit reasons where overflow is
impossible. Adapters propagate other failures by treating the grid as
empty for that boundary month rather than panicking.

`Weekday` is the ISO-numbered enum from `ars-i18n` (Monday=1..Sunday=7) and
cannot be cast to `u8` directly. The grid module exposes a local
`weekday_sunday_zero(wd) -> u8` helper that maps to the Sunday-zero
convention the modulo arithmetic below requires.

```rust
/// Controls how prev/next navigation advances when multiple months are visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageBehavior {
    /// Prev/next advances by the full `visible_months` count.
    #[default]
    Visible,
    /// Prev/next advances by exactly 1 month, regardless of visible_months.
    Single,
}

impl Context {
    /// Returns the first day of the week for this calendar's locale/override.
    pub fn week_start(&self) -> Weekday {
        self.first_day_of_week
    }

    /// Ordered day-of-week labels starting from the configured first day.
    pub fn week_day_labels(&self) -> Vec<(Weekday, String)> {
        ordered_weekdays(self.first_day_of_week)
            .into_iter()
            .map(|wd| (wd, self.intl_backend.weekday_short_label(wd, &self.locale)))
            .collect()
    }

    /// Build the grid for the first visible month. Equivalent to `weeks_for(0)`.
    pub fn weeks(&self) -> Vec<[CalendarDate; 7]> {
        self.weeks_for(0)
    }

    /// Build the grid of weeks for the month at the given offset
    /// from the first visible month. Offset 0 = first visible month.
    pub fn weeks_for(&self, offset: usize) -> Vec<[CalendarDate; 7]> {
        let (target_month, target_year) =
            month_year_at_offset(self.visible_month, self.visible_year, offset);

        let Ok(first_of_month) = CalendarDate::new_gregorian(target_year, target_month, 1) else {
            return Vec::new();
        };

        let first_weekday_index = weekday_sunday_zero(first_of_month.weekday());
        let start_index = weekday_sunday_zero(self.first_day_of_week);
        let leading = i32::from((first_weekday_index + 7 - start_index) % 7);

        let Ok(grid_start) = first_of_month.add_days(-leading) else {
            return Vec::new();
        };

        let mut weeks: Vec<[CalendarDate; 7]> = Vec::with_capacity(6);
        let mut current = grid_start;
        // Always render 6 weeks (42 days) for consistent grid height.
        for _ in 0..6 {
            let Some(row) = build_week(&current) else { return weeks; };
            weeks.push(row);
            let Ok(next_week_start) = current.add_days(7) else { return weeks; };
            current = next_week_start;
        }
        weeks
    }

    /// Returns (month, year) for the month at the given offset, normalised
    /// into the 1..=12 month range with a year adjustment.
    pub fn month_year_at_offset(&self, offset: usize) -> (u8, i32) {
        month_year_at_offset(self.visible_month, self.visible_year, offset)
    }

    /// Whether a date is outside the month at the specified offset.
    pub fn is_outside_month_at_offset(&self, date: &CalendarDate, offset: usize) -> bool {
        let (month, year) = self.month_year_at_offset(offset);
        date.month() != month || date.year() != year
    }

    /// Whether the date's month falls within any of the visible months.
    pub fn is_in_visible_range(&self, date: &CalendarDate) -> bool {
        (0..self.visible_months).any(|offset| {
            let (month, year) = self.month_year_at_offset(offset);
            date.month() == month && date.year() == year
        })
    }

    /// Navigate the visible month/year forward by `n` months.
    pub fn advance_month(&mut self, n: i32) {
        let (month, year) = advance_month(self.visible_month, self.visible_year, n);
        self.visible_month = month;
        self.visible_year = year;
    }

    /// Whether the given date is outside the currently visible month.
    pub fn is_outside_visible_month(&self, date: &CalendarDate) -> bool {
        date.month() != self.visible_month || date.year() != self.visible_year
    }
}

/// Sunday-zero index for a weekday (Sunday=0..Saturday=6).
const fn weekday_sunday_zero(weekday: Weekday) -> u8 { /* … */ }

/// `(month, year)` at the given offset from `(visible_month, visible_year)`,
/// normalised into the 1..=12 month range with a year adjustment.
const fn month_year_at_offset(visible_month: u8, visible_year: i32, offset: usize) -> (u8, i32) {
    let raw_index = visible_month as i64 - 1 + offset as i64;
    let normalised_month = raw_index.rem_euclid(12) as u8 + 1;
    let year_delta = raw_index.div_euclid(12) as i32;
    (normalised_month, visible_year + year_delta)
}

/// `advance_month` shares the same modulo arithmetic — see
/// `month_year_at_offset`; signed `n` advances or retreats by `|n|` months.
const fn advance_month(visible_month: u8, visible_year: i32, n: i32) -> (u8, i32);
```

### 1.7 Full Machine Implementation

The named-effect intent for month/year navigation is a typed enum, not a
bare `&'static str`. The setup closure is a no-op marker
([`PendingEffect::named`]); the adapter resolves the announce intent based
on the typed name and calls into its platform announce path — core code
never reaches for a framework hook like `use_platform_effects()`.

```rust
/// Typed identifier for every named effect intent the `calendar` machine emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Announce the newly visible month (or month range) to assistive tech
    /// via the adapter's platform announcer. Triggered by every
    /// month/year navigation transition.
    AnnounceMonth,
}

pub struct Machine;

impl ars_core::Machine for Machine {
    type State    = State;
    type Event    = Event;
    type Context  = Context;
    type Props    = Props;
    type Messages = Messages;
    type Effect   = Effect;
    type Api<'a>  = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let value = match &props.value {
            Some(controlled) => Bindable::controlled(controlled.clone()),
            None             => Bindable::uncontrolled(props.default_value.clone()),
        };
        let selected_dates = match &props.selected_dates {
            Some(set) => Bindable::controlled(set.clone()),
            None      => Bindable::uncontrolled(props.default_selected_dates.clone()),
        };

        let initial_date = match props.selection_mode {
            SelectionMode::Single   => value.get().clone().unwrap_or_else(|| props.today.clone()),
            SelectionMode::Multiple => selected_dates.get().iter().next().cloned()
                .unwrap_or_else(|| props.today.clone()),
        };

        let locale = env.locale.clone();
        let first_day = props
            .first_day_of_week
            .unwrap_or_else(|| locale.first_day_of_week(&*env.intl_backend));

        let ctx = Context {
            value,
            selected_dates,
            selection_mode: props.selection_mode,
            max_selected: props.max_selected,
            focused_date: initial_date.clone(),
            visible_month: initial_date.month(),
            visible_year: initial_date.year(),
            visible_months: props.visible_months.max(1),
            page_behavior: props.page_behavior,
            min: props.min.clone(),
            max: props.max.clone(),
            first_day_of_week: first_day,
            is_rtl: props.is_rtl,
            disabled: props.disabled,
            readonly: props.readonly,
            show_week_numbers: props.show_week_numbers,
            today: props.today.clone(),
            is_date_unavailable_fn: props.is_date_unavailable,
            locale,
            messages: messages.clone(),
            intl_backend: Arc::clone(&env.intl_backend),
            ids: ComponentIds::from_id(&props.id),
        };

        (State::Idle, ctx)
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled { return None; }

        match event {
            // ── Focus management ─────────────────────────────────────────
            Event::FocusIn  => Some(TransitionPlan::to(State::Focused)),
            Event::FocusOut => Some(TransitionPlan::to(State::Idle)),

            Event::FocusDate { date } => {
                let clamped = ctx.clamp_date(date.clone());
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_date = clamped;
                    ctx.sync_visible_to_focused();
                }))
            }

            // ── Date selection ────────────────────────────────────────────
            // `SelectDate` and `ToggleDate` both route through the active
            // selection mode: Single uses `apply_select_single`, Multiple
            // uses `apply_toggle_multi`. Read the spec wording in §1.2 for
            // why they share an arm.
            Event::SelectDate { date } | Event::ToggleDate { date } => {
                match ctx.selection_mode {
                    SelectionMode::Single   => apply_select_single(ctx, date.clone()),
                    SelectionMode::Multiple => apply_toggle_multi(ctx, date.clone()),
                }
            }

            // ── Month / year navigation ──────────────────────────────────
            Event::NextMonth => Some(month_step_plan(step_for_page_behavior(ctx))),
            Event::PrevMonth => Some(month_step_plan(-step_for_page_behavior(ctx))),
            Event::NextYear  => Some(month_step_plan(12)),
            Event::PrevYear  => Some(month_step_plan(-12)),

            Event::SetMonth { month } => {
                let month = *month;
                if !(1..=12).contains(&month) { return None; }
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.visible_month = month;
                }).with_effect(announce_month_effect()))
            }

            Event::SetYear { year } => {
                let year = *year;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.visible_year = year;
                }).with_effect(announce_month_effect()))
            }

            // ── Keyboard navigation ──────────────────────────────────────
            Event::KeyDown { key, shift } => handle_keydown(*key, *shift, ctx),
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Event),
    ) -> Self::Api<'a> {
        Api::new(state, ctx, props, send)
    }
}

fn step_for_page_behavior(ctx: &Context) -> i32 {
    match ctx.page_behavior {
        PageBehavior::Visible => i32::try_from(ctx.visible_months).unwrap_or(1).max(1),
        PageBehavior::Single  => 1,
    }
}

fn month_step_plan(step: i32) -> TransitionPlan<Machine> {
    TransitionPlan::context_only(move |ctx: &mut Context| { ctx.advance_month(step); })
        .with_effect(announce_month_effect())
}

fn announce_month_effect() -> ars_core::PendingEffect<Machine> {
    ars_core::PendingEffect::named(Effect::AnnounceMonth)
}

fn apply_select_single(ctx: &Context, date: CalendarDate)
    -> Option<TransitionPlan<Machine>>
{
    if ctx.readonly { return None; }
    if ctx.is_date_disabled(&date) || ctx.is_date_unavailable(&date) { return None; }
    Some(TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
        ctx.value.set(Some(date.clone()));
        ctx.focused_date = date;
    }))
}

fn apply_toggle_multi(ctx: &Context, date: CalendarDate)
    -> Option<TransitionPlan<Machine>>
{
    if ctx.readonly { return None; }
    if ctx.is_date_disabled(&date) || ctx.is_date_unavailable(&date) { return None; }
    let already_selected = ctx.selected_dates.get().contains(&date);
    let at_cap = !already_selected && ctx.max_selected
        .is_some_and(|cap| ctx.selected_dates.get().len() >= cap);
    if at_cap { return None; } // §5.4 silent drop
    Some(TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
        let mut next = ctx.selected_dates.get().clone();
        if already_selected { next.remove(&date); } else { next.insert(date.clone()); }
        ctx.selected_dates.set(next);
        ctx.focused_date = date;
    }))
}

/// Keyboard navigation within the calendar grid.
/// Follows WAI-ARIA Grid pattern with RTL awareness.
///
/// `Shift+PageUp` / `Shift+PageDown` are handled here directly because
/// `Event::KeyDown` carries the `shift` flag — adapters do not need a
/// side-channel mapping. The `Enter`/`Space` arm fans out to
/// `Event::SelectDate` (Single) or `Event::ToggleDate` (Multiple) so the
/// active selection mode determines the follow-up behavior.
fn handle_keydown(key: KeyboardKey, shift: bool, ctx: &Context)
    -> Option<TransitionPlan<Machine>>
{
    if shift {
        match key {
            KeyboardKey::PageUp   => return Some(month_step_plan(-12)),
            KeyboardKey::PageDown => return Some(month_step_plan(12)),
            _ => {}
        }
    }

    let focused = ctx.focused_date.clone();
    // RTL-aware directional keys: in RTL, Left means "next" and Right means "prev".
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
            KeyboardKey::ArrowUp   => focused.add_days(-7).ok(),
            KeyboardKey::ArrowDown => focused.add_days(7).ok(),
            KeyboardKey::Home => {
                // Move to start of current week.
                let wd = weekday_sunday_zero(focused.weekday());
                let start = weekday_sunday_zero(ctx.first_day_of_week);
                let offset = i32::from((wd + 7 - start) % 7);
                focused.add_days(-offset).ok()
            }
            KeyboardKey::End => {
                // Move to end of current week.
                let wd = weekday_sunday_zero(focused.weekday());
                let start = weekday_sunday_zero(ctx.first_day_of_week);
                let offset = i32::from((wd + 7 - start) % 7);
                focused.add_days(6 - offset).ok()
            }
            KeyboardKey::PageUp   => focused.add(DateDuration { months: -1, ..Default::default() }).ok(),
            KeyboardKey::PageDown => focused.add(DateDuration { months:  1, ..Default::default() }).ok(),
            _ => None,
        }
    };

    if let Some(date) = new_focus {
        let clamped = ctx.clamp_date(date);
        return Some(TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
            ctx.focused_date = clamped;
            ctx.sync_visible_to_focused();
        }));
    }

    // Enter / Space selects (Single) or toggles (Multiple) the focused date.
    match key {
        KeyboardKey::Enter | KeyboardKey::Space => {
            let date = focused;
            let select_event = match ctx.selection_mode {
                SelectionMode::Single   => Event::SelectDate { date },
                SelectionMode::Multiple => Event::ToggleDate { date },
            };
            Some(TransitionPlan::context_only(|_ctx: &mut Context| {}).then(select_event))
        }
        _ => None,
    }
}
```

### 1.8 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "calendar"]
pub enum Part {
    Root,
    Header,
    PrevTrigger,
    NextTrigger,
    Heading,
    Grid,
    GridGroup,
    HeadRow,
    HeadCell { day: Weekday },
    Row { week_index: usize },
    Cell { date: CalendarDate },
    CellTrigger { date: CalendarDate },
}

/// API for the Calendar component.
pub struct Api<'a> {
    /// The state of the calendar.
    state: &'a State,
    /// The context of the calendar.
    ctx:   &'a Context,
    /// The props of the calendar.
    props: &'a Props,
    /// The send function for the calendar.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    // ── AttrMap getters ──────────────────────────────────────────────────

    /// Attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Data("ars-state"), self.state_name());
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

    /// Attributes for the header element.
    pub fn header_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Header.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("header"));
        attrs
    }

    /// Attributes for the previous trigger element.
    pub fn prev_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let id = format!("{}-prev-trigger", self.ctx.id);
        attrs.set(HtmlAttr::Id, id);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::PrevTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let step = match self.ctx.page_behavior {
            PageBehavior::Visible => self.ctx.visible_months,
            PageBehavior::Single => 1,
        };
        let label = if step > 1 {
            (self.ctx.messages.prev_page_label)(step, &self.ctx.locale)
        } else {
            (self.ctx.messages.prev_month_label)(&self.ctx.locale)
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.is_prev_disabled() {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        attrs
    }

    /// Attributes for the next trigger element.
    pub fn next_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let id = format!("{}-next-trigger", self.ctx.id);
        attrs.set(HtmlAttr::Id, id);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::NextTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let step = match self.ctx.page_behavior {
            PageBehavior::Visible => self.ctx.visible_months,
            PageBehavior::Single => 1,
        };
        let label = if step > 1 {
            (self.ctx.messages.next_page_label)(step, &self.ctx.locale)
        } else {
            (self.ctx.messages.next_month_label)(&self.ctx.locale)
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.is_next_disabled() {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        attrs
    }

    /// Attributes for the main heading element. The heading is the
    /// `aria-live="polite"` announcer for month navigation.
    pub fn heading_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("heading"));
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Heading.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
        attrs
    }

    /// Formatted heading text: e.g. "January 2024".
    pub fn heading_text(&self) -> String {
        let month_name = self.ctx.intl_backend.month_long_name(self.ctx.visible_month, &self.ctx.locale);
        format!("{} {}", month_name, self.ctx.visible_year)
    }

    /// Attributes for the grid element (first visible month).
    pub fn grid_attrs(&self) -> AttrMap {
        self.grid_attrs_with_ids(self.ctx.ids.part("grid"), self.ctx.ids.part("heading"))
    }

    fn grid_attrs_with_ids(&self, grid_id: String, heading_id: String) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Grid.data_attrs();
        attrs.set(HtmlAttr::Id, grid_id);
        attrs.set(HtmlAttr::Role, "grid");
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), heading_id);
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

    // ── Multi-month API ───────────────────────────────────────────────

    /// Number of months currently visible.
    pub fn visible_month_count(&self) -> usize {
        self.ctx.visible_months
    }

    /// Iterator of month offsets: 0..visible_months.
    pub fn month_offsets(&self) -> std::ops::Range<usize> {
        0..self.ctx.visible_months
    }

    /// Grid attributes for the month at the given offset. Multi-month
    /// layouts use the per-grid heading id rather than the shared live
    /// region.
    pub fn grid_attrs_for(&self, offset: usize) -> AttrMap {
        let grid_id    = format!("{}-grid-{}",    self.ctx.ids.id(), offset);
        let heading_id = format!("{}-heading-{}", self.ctx.ids.id(), offset);
        self.grid_attrs_with_ids(grid_id, heading_id)
    }

    /// Heading text for the month at the given offset.
    pub fn heading_text_for(&self, offset: usize) -> String {
        let (month, year) = self.ctx.month_year_at_offset(offset);
        format!("{} {}", self.ctx.intl_backend.month_long_name(month, &self.ctx.locale), year)
    }

    /// Heading attributes for a per-grid heading. Per-grid headings are
    /// visually hidden and intentionally have no `aria-live` — only the
    /// main `heading_attrs` heading announces month changes to avoid
    /// duplicate announcements.
    pub fn heading_attrs_for(&self, offset: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, format!("{}-heading-{}", self.ctx.ids.id(), offset));
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Heading.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Grid data for the month at the given offset.
    pub fn weeks_for(&self, offset: usize) -> Vec<[CalendarDate; 7]> {
        self.ctx.weeks_for(offset)
    }

    /// Whether a date is outside the month at the given offset.
    pub fn is_outside_month_for(&self, date: &CalendarDate, offset: usize) -> bool {
        self.ctx.is_outside_month_at_offset(date, offset)
    }

    /// Cell attributes using the offset-specific outside-month check.
    pub fn cell_attrs_for(&self, date: &CalendarDate, offset: usize) -> AttrMap {
        let mut attrs = self.cell_attrs(date);
        // Override the outside-month attribute for this specific grid's month.
        if self.ctx.is_outside_month_at_offset(date, offset) {
            attrs.set_bool(HtmlAttr::Data("ars-outside-month"), true);
        }
        attrs
    }

    /// Attributes for the grid group container (role="group").
    /// Only meaningful when `visible_months > 1`.
    pub fn grid_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, format!("{}-grid-group", self.ctx.ids.id()));
        attrs.set(HtmlAttr::Role, "group");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::GridGroup.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), self.range_heading_text());
        attrs
    }

    /// Heading text spanning all visible months.
    /// E.g., "January – February 2024" or "December 2023 – January 2024".
    pub fn range_heading_text(&self) -> String {
        if self.ctx.visible_months <= 1 {
            return self.heading_text();
        }
        let first = self.heading_text_for(0);
        let last = self.heading_text_for(self.ctx.visible_months - 1);
        let sep = (self.ctx.messages.month_range_separator)(&self.ctx.locale);
        format!("{}{}{}", first, sep, last)
    }

    // ── Single-month grid helpers (backward compat) ─────────────────

    /// Attributes for the head row element.
    pub fn head_row_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HeadRow.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Attributes for the head cell element.
    pub fn head_cell_attrs(&self, weekday: Weekday) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HeadCell { day: weekday }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Scope, "col");
        attrs.set(HtmlAttr::Abbr, self.ctx.intl_backend.weekday_long_label(weekday, &self.ctx.locale));
        attrs
    }

    /// Attributes for the row element.
    pub fn row_attrs(&self, week_index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Row { week_index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-week-index"), week_index.to_string());
        attrs
    }

    /// Attributes for the cell element.
    pub fn cell_attrs(&self, date: &CalendarDate) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Role, "gridcell");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Cell { date: date.clone() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

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

    /// Attributes for the cell trigger element.
    pub fn cell_trigger_attrs(&self, date: &CalendarDate) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CellTrigger { date: date.clone() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        let disabled = self.is_disabled(date);
        let unavailable = self.is_unavailable(date);
        let is_focused = self.ctx.focused_date == *date;
        let is_today = *date == self.ctx.today;
        let selected = self.is_selected(date);

        // tabindex: only the focused cell or the first selectable cell is tabbable.
        attrs.set(HtmlAttr::TabIndex, if is_focused { "0" } else { "-1" });

        if disabled || unavailable {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        if selected {
            attrs.set(HtmlAttr::Aria(AriaAttr::Selected), "true");
        }
        if is_today {
            attrs.set_bool(HtmlAttr::Data("ars-today"), true);
        }
        if self.ctx.is_outside_visible_month(date) {
            attrs.set_bool(HtmlAttr::Data("ars-outside-month"), true);
        }
        if unavailable {
            attrs.set_bool(HtmlAttr::Data("ars-unavailable"), true);
        }

        // aria-label: full date string for screen readers.
        // Unavailable and disabled dates append a suffix so screen readers announce
        // the restriction without requiring the user to attempt selection.
        let base_label = format!(
            "{} {}, {}, {}",
            self.ctx.intl_backend.month_long_name(date.month(), &self.ctx.locale),
            date.day(),
            date.year(),
            self.ctx.intl_backend.weekday_long_label(date.weekday(), &self.ctx.locale),
        );
        let label = if unavailable {
            format!("{} (unavailable)", base_label)
        } else if disabled {
            format!("{} (disabled)", base_label)
        } else {
            base_label
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);

        attrs
    }

    // ── Typed handler methods ────────────────────────────────────────────

    /// Handle click on a day cell. Routes to `SelectDate` in Single mode
    /// and `ToggleDate` in Multiple mode.
    pub fn on_cell_click(&self, date: CalendarDate) {
        let event = match self.ctx.selection_mode {
            SelectionMode::Single   => Event::SelectDate { date },
            SelectionMode::Multiple => Event::ToggleDate { date },
        };
        (self.send)(event);
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

    /// Handle keydown on the grid. The adapter folds the `shift` modifier
    /// into the call so the machine can map `Shift+PageUp` /
    /// `Shift+PageDown` to year navigation inside its single transition
    /// arm. There is no side-channel dispatch — `Event::KeyDown` carries
    /// `shift` directly.
    pub fn on_grid_keydown(&self, key: KeyboardKey, shift: bool) {
        (self.send)(Event::KeyDown { key, shift });
    }

    /// Handle click on the previous month button.
    pub fn on_prev_click(&self) {
        (self.send)(Event::PrevMonth);
    }

    /// Handle click on the next month button.
    pub fn on_next_click(&self) {
        (self.send)(Event::NextMonth);
    }

    // ── Computed state accessors ─────────────────────────────────────────

    /// Whether a date is currently selected.
    pub fn is_selected(&self, date: &CalendarDate) -> bool {
        self.ctx.is_selected(date)
    }

    /// Whether a date is today.
    pub fn is_today(&self, date: &CalendarDate) -> bool {
        *date == self.ctx.today
    }

    /// Whether a date is disabled (out of min/max range or globally disabled).
    pub fn is_disabled(&self, date: &CalendarDate) -> bool {
        self.ctx.is_date_disabled(date)
    }

    /// Whether a date is unavailable (user-defined predicate).
    pub fn is_unavailable(&self, date: &CalendarDate) -> bool {
        self.ctx.is_date_unavailable(date)
    }

    /// Whether a date is outside the currently visible month.
    pub fn is_outside_month(&self, date: &CalendarDate) -> bool {
        self.ctx.is_outside_visible_month(date)
    }

    /// Whether the prev button should be disabled (min constraint).
    pub fn is_prev_disabled(&self) -> bool {
        let Some(min) = &self.ctx.min else { return false; };
        let Ok(first_of_visible) =
            CalendarDate::new_gregorian(self.ctx.visible_year, self.ctx.visible_month, 1)
        else { return false; };
        !matches!(first_of_visible.compare(min), Ordering::Greater)
    }

    /// Whether the next button should be disabled (max constraint).
    /// Checks against the **last** visible month so multi-month layouts
    /// behave correctly.
    pub fn is_next_disabled(&self) -> bool {
        let Some(max) = &self.ctx.max else { return false; };
        let last_offset = self.ctx.visible_months.saturating_sub(1);
        let (month, year) = self.ctx.month_year_at_offset(last_offset);
        let Ok(first_of_last) = CalendarDate::new_gregorian(year, month, 1) else {
            return false;
        };
        let days_in_month = first_of_last.days_in_month();
        let Ok(last_of_last) = CalendarDate::new_gregorian(year, month, days_in_month) else {
            return false;
        };
        !matches!(last_of_last.compare(max), Ordering::Less)
    }

    /// The grid data: weeks and day labels.
    pub fn weeks(&self) -> Vec<[CalendarDate; 7]> {
        self.ctx.weeks()
    }

    /// Ordered day-of-week labels for the head row.
    pub fn week_day_labels(&self) -> Vec<(Weekday, String)> {
        self.ctx.week_day_labels()
    }

    /// The currently focused date.
    pub fn focused_date(&self) -> &CalendarDate {
        &self.ctx.focused_date
    }

    /// Whether the calendar grid has focus.
    pub fn is_focused(&self) -> bool {
        *self.state == State::Focused
    }

    fn state_name(&self) -> &'static str {
        match self.state {
            State::Idle    => "idle",
            State::Focused => "focused",
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match &part {
            Part::Root => self.root_attrs(),
            Part::Header => self.header_attrs(),
            Part::PrevTrigger => self.prev_trigger_attrs(),
            Part::NextTrigger => self.next_trigger_attrs(),
            Part::Heading => self.heading_attrs(),
            Part::Grid => self.grid_attrs(),
            Part::GridGroup => self.grid_group_attrs(),
            Part::HeadRow => self.head_row_attrs(),
            Part::HeadCell { day } => self.head_cell_attrs(day),
            Part::Row { week_index } => self.row_attrs(*week_index),
            Part::Cell { date } => self.cell_attrs(date),
            Part::CellTrigger { date } => self.cell_trigger_attrs(date),
        }
    }
}
```

## 2. Anatomy

```text
Calendar (en-US, January 2024)
└── Root                                  data-ars-scope="calendar"
    ├── Header                            data-ars-part="header"
    │   ├── PrevTrigger                   aria-label="Previous month"
    │   ├── HeadingText                   "January 2024"
    │   └── NextTrigger                   aria-label="Next month"
    └── Grid                              role="grid"  aria-labelledby="{id}-heading"
        ├── HeadRow
        │   ├── HeadCell "Su"             scope="col"  abbr="Sunday"
        │   ├── HeadCell "Mo"             scope="col"  abbr="Monday"
        │   ├── … (7 cells total)
        │   └── HeadCell "Sa"             scope="col"  abbr="Saturday"
        ├── Row (week 1)
        │   ├── Cell > CellTrigger "31"   data-ars-outside-month  aria-disabled="true"
        │   ├── Cell > CellTrigger "1"    tabindex="0"  aria-selected="false"
        │   ├── …
        │   └── Cell > CellTrigger "6"
        ├── Row (week 2) … Row (week 5)
        └── Row (week 6)
            └── Cell > CellTrigger "3"    data-ars-outside-month
```

| Part          | HTML Element | Key Attributes                                                               |
| ------------- | ------------ | ---------------------------------------------------------------------------- |
| `Root`        | `<div>`      | `data-ars-scope="calendar"`, `data-ars-part="root"`, `data-ars-state`        |
| `Header`      | `<div>`      | `data-ars-part="header"`                                                     |
| `PrevTrigger` | `<button>`   | `aria-label`, `tabindex="-1"`                                                |
| `NextTrigger` | `<button>`   | `aria-label`, `tabindex="-1"`                                                |
| `Heading`     | `<span>`     | `aria-live="polite"`, `aria-atomic="true"`                                   |
| `GridGroup`   | `<div>`      | `role="group"`, `aria-label` (only when `visible_months > 1`)                |
| `Grid`        | `<table>`    | `role="grid"`, `aria-labelledby`                                             |
| `HeadRow`     | `<tr>`       | —                                                                            |
| `HeadCell`    | `<th>`       | `scope="col"`, `abbr`                                                        |
| `Row`         | `<tr>`       | —                                                                            |
| `Cell`        | `<td>`       | `role="gridcell"`, `aria-selected`                                           |
| `CellTrigger` | `<button>`   | `tabindex`, `aria-label`, `aria-selected`, `aria-disabled`, `data-ars-today` |

**Multi-month anatomy** (`visible_months=2`):

```text
Calendar (en-US, January–February 2024, visible_months=2)
└── Root                                  data-ars-scope="calendar"
    ├── Header                            data-ars-part="header"
    │   ├── PrevTrigger                   aria-label="Previous 2 months"
    │   ├── Heading                       "January – February 2024"
    │   └── NextTrigger                   aria-label="Next 2 months"
    └── GridGroup                         role="group"  aria-label="January – February 2024"
        ├── Grid[0]                       role="grid"  aria-labelledby="{id}-heading-0"
        │   ├── Heading[0]               "January 2024"  (visually hidden)
        │   ├── HeadRow … HeadCell×7
        │   └── Row×6 > Cell > CellTrigger
        └── Grid[1]                       role="grid"  aria-labelledby="{id}-heading-1"
            ├── Heading[1]               "February 2024"  (visually hidden)
            ├── HeadRow … HeadCell×7
            └── Row×6 > Cell > CellTrigger
```

When `visible_months == 1`, anatomy is identical to the single-month tree above (no `GridGroup` wrapper).

## 3. Accessibility

- Each selected cell: `aria-selected="true"`.
- Screen readers announce selection state changes via `aria-live` region.
- In Multiple mode: `aria-multiselectable="true"` on the grid.

### 3.1 ARIA Roles, States, and Properties

| Element                | Role/Attribute          | Details                                                                                    |
| ---------------------- | ----------------------- | ------------------------------------------------------------------------------------------ |
| `Grid`                 | `role="grid"`           | `aria-labelledby` points to `HeadingText`                                                  |
| `HeadCell`             | `<th scope="col">`      | `abbr` attribute holds full weekday name                                                   |
| `Cell`                 | `role="gridcell"`       | `aria-selected` on selected date(s)                                                        |
| `CellTrigger`          | `<button>`              | `aria-label` = full date string; `aria-disabled` on disabled/unavailable                   |
| `HeadingText`          | —                       | `aria-live="polite"` + `aria-atomic="true"` for month changes                              |
| `PrevTrigger`          | `<button>`              | `aria-label="Previous month"` (or `"Previous N months"` when step > 1)                     |
| `NextTrigger`          | `<button>`              | `aria-label="Next month"` (or `"Next N months"` when step > 1)                             |
| `GridGroup`            | `role="group"`          | Wraps multiple grids; `aria-label` = month range. Not rendered when `visible_months == 1`. |
| `Grid[n]` (per-grid)   | `role="grid"`           | `aria-labelledby` → per-grid heading `{id}-heading-{n}`                                    |
| Per-grid `HeadingText` | `id="{id}-heading-{n}"` | Visually hidden per-grid heading. No `aria-live` (only the main heading has it).           |

### 3.2 Keyboard Interaction

| Key               | Action                                       |
| ----------------- | -------------------------------------------- |
| `ArrowLeft`       | Move focus to previous day (next day in RTL) |
| `ArrowRight`      | Move focus to next day (previous day in RTL) |
| `ArrowUp`         | Move focus to same day in previous week      |
| `ArrowDown`       | Move focus to same day in next week          |
| `Home`            | Move focus to start of current week          |
| `End`             | Move focus to end of current week            |
| `PageUp`          | Move focus to same day in previous month     |
| `PageDown`        | Move focus to same day in next month         |
| `Shift+PageUp`    | Move focus to same day in previous year      |
| `Shift+PageDown`  | Move focus to same day in next year          |
| `Enter` / `Space` | Select the focused date                      |

### 3.3 Focus Management

- Only one cell in the grid is in the tab order at a time (`tabindex="0"`); all others have `tabindex="-1"`.
- Arrow keys move the focused cell roving within the grid.
- When focus moves to a date outside the visible month, the visible month auto-scrolls.
- The PrevTrigger and NextTrigger buttons have `tabindex="-1"` and are activated by click only; they do not participate in the roving tab order of the grid.
- When `visible_months > 1` and arrow-key navigation crosses a month boundary that remains within the visible range, the view does **not** scroll. Only when focus leaves the entire visible range does the calendar paginate.

### 3.4 Screen Reader Announcements

- Month navigation: HeadingText has `aria-live="polite"` so screen readers announce "February 2024" when the month changes.
- When `visible_months > 1`, month navigation announces the full range (e.g., "January – February 2024"). Per-grid headings do NOT have `aria-live` to avoid duplicate announcements.
- Date selection: Cell's `aria-selected` change is announced.
- Each CellTrigger has an `aria-label` like "January 15, 2024, Monday" for unambiguous date reading.

## 4. Internationalization

### 4.1 Messages

```rust
// ars-i18n/src/calendar/i18n.rs

/// `MessageFn` carrying a locale-only label closure.
pub type LocaleLabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// `MessageFn` carrying a step-count plus locale label closure (used by the
/// multi-month prev/next page labels).
pub type PageLabelFn = dyn Fn(usize, &Locale) -> String + Send + Sync;

/// Locale-specific labels for the Calendar component.
#[derive(Clone)]
pub struct Messages {
    /// Accessible label for the previous month navigation button.
    pub prev_month_label: MessageFn<LocaleLabelFn>,
    /// Accessible label for the next month navigation button.
    pub next_month_label: MessageFn<LocaleLabelFn>,
    /// Label for prev button when navigating multiple months.
    /// The `usize` parameter is the step count.
    pub prev_page_label: MessageFn<PageLabelFn>,
    /// Label for next button when navigating multiple months.
    /// The `usize` parameter is the step count.
    pub next_page_label: MessageFn<PageLabelFn>,
    /// Separator between month names in multi-month range heading (e.g., " – ").
    pub month_range_separator: MessageFn<LocaleLabelFn>,
}
// Month names, weekday names, and abbreviations are resolved from the
// `IntlBackend` via `intl_backend.month_long_name(month, &locale)`,
// `intl_backend.weekday_short_label(wd, &locale)`, and
// `intl_backend.weekday_long_label(wd, &locale)` — not stored in Messages.

impl Default for Messages {
    fn default() -> Self {
        Self {
            prev_month_label: MessageFn::static_str("Previous month"),
            next_month_label: MessageFn::static_str("Next month"),
            prev_page_label: MessageFn::new(|count, _locale| format!("Previous {} months", count)),
            next_page_label: MessageFn::new(|count, _locale| format!("Next {} months", count)),
            month_range_separator: MessageFn::static_str(" \u{2013} "),
        }
    }
}

impl ComponentMessages for Messages {}
```

**Locale-aware week start:**

| Locale                                               | First Day of Week |
| ---------------------------------------------------- | ----------------- |
| `en-US`, `en-CA`, `ja-JP`, `ko-KR`, `zh-TW`          | Sunday            |
| `ar-SA`, `he-IL`                                     | Sunday            |
| `en-GB`, `de-DE`, `fr-FR`, `es-ES`, `ru-RU`, `it-IT` | Monday            |
| `fa-IR`                                              | Saturday          |

The `first_day_of_week` prop overrides the locale default when provided.

### 4.2 First Day of Week

1. Default derived from locale via ICU4X `WeekCalculator` (US: Sunday, most of Europe: Monday, parts of Middle East: Saturday).
2. Overridable via `first_day_of_week: Option<Weekday>` prop.
3. In RTL locales, the calendar grid is mirrored horizontally (week starts from the right).
4. DateField weekday segment typeahead matches locale-appropriate day names.

### 4.3 Week Numbers

1. Optional `show_week_numbers: bool` prop (default `false`).
2. Week numbering follows locale convention: ISO 8601 (most of world) vs US week numbering via ICU4X `WeekCalculator`.
3. Week number column header has `aria-label="Week number"` (localized).
4. Each week number cell has `aria-label="Week {n}"` (localized).
5. Week numbers are not focusable or selectable by default.

## 5. Variant: Multiple Selection

Calendar supports multiple non-contiguous date selection alongside the
default single-date mode. The Props, Context, and Event surface for
multi-select live directly in §1.2-§1.4 (not as "additional"
declarations) — the machine is one unified type that switches behaviour
on `selection_mode` rather than two parallel `Machine` types.

### 5.1 Active surface

The unified surface relevant to multi-select:

- [`SelectionMode { Single, Multiple }`](#14-props) on `Props` and
  `Context`.
- `Props.selected_dates: Option<SelectedDates>` (controlled) and
  `Props.default_selected_dates: SelectedDates` (uncontrolled), backing
  `Context.selected_dates: Bindable<SelectedDates>`.
- `Props.max_selected: Option<usize>` and `Context.max_selected`.
- [`Event::ToggleDate { date }`](#12-events) struct variant. `SelectDate`
  is accepted in `Multiple` mode and routes to the same toggle handler so
  adapters that do not know the active mode can emit one canonical
  "select this date" event.

### 5.2 Behavior

| Action     | Click                 |
| ---------- | --------------------- |
| `Multiple` | Toggle date selection |

When `max_selected` is set, the `ToggleDate` transition guards against
`selected_dates.len() >= max_selected`. Toggles that would add a date
beyond the cap are silently dropped; toggles that remove an already-selected
date always succeed.

### 5.3 Keyboard

| Key               | Behavior                                |
| ----------------- | --------------------------------------- |
| `Space` / `Enter` | Toggle focused date in/out of selection |

Advanced multi-select keyboard operations (`Shift+Click`,
`Shift+Space` range fill from an anchor, `Ctrl+A` select-all visible)
are not part of this implementation. They require an "anchor" concept
not currently in the spec or machine state, and are tracked as a
follow-up that will refine §5 once a concrete anchor design lands.

## 6. Library Parity

> Compared against: React Aria (`Calendar`).

### 6.1 Props

| Feature                    | ars-ui                                | React Aria                    | Notes                                                             |
| -------------------------- | ------------------------------------- | ----------------------------- | ----------------------------------------------------------------- |
| Controlled value           | `value: Option<Option<CalendarDate>>` | `value: DateValue`            | Equivalent                                                        |
| Default value              | `default_value`                       | `defaultValue`                | Equivalent                                                        |
| Focused value (controlled) | --                                    | `focusedValue`                | ars-ui has `focused_date` in Context but not as a controlled prop |
| Default focused value      | --                                    | `defaultFocusedValue`         | ars-ui derives from value/today                                   |
| Min/max                    | `min`, `max`                          | `minValue`, `maxValue`        | Equivalent                                                        |
| Unavailable predicate      | `is_date_unavailable`                 | `isDateUnavailable`           | Equivalent                                                        |
| Disabled                   | `disabled`                            | `isDisabled`                  | Equivalent                                                        |
| Read-only                  | `readonly`                            | `isReadOnly`                  | Equivalent                                                        |
| Invalid                    | --                                    | `isInvalid`                   | Calendar has no `invalid` prop                                    |
| First day of week          | `first_day_of_week`                   | `firstDayOfWeek`              | Equivalent                                                        |
| Visible months             | `visible_months`                      | `visibleDuration: { months }` | Equivalent concept, different shape                               |
| Page behavior              | `page_behavior`                       | `pageBehavior`                | Equivalent                                                        |
| Auto-focus                 | --                                    | `autoFocus`                   | Not present in ars-ui                                             |
| Create calendar            | --                                    | `createCalendar`              | ars-ui uses `CalendarSystem` enum instead                         |

**Gaps:** None worth adopting. `autoFocus` is an adapter concern (the adapter can set focus on mount). `focusedValue`/`defaultFocusedValue` as controlled props add complexity without clear benefit since focus is a transient UI state. `isInvalid` on Calendar is not needed since Calendar does not participate in form validation directly.

### 6.2 Anatomy

| Part         | ars-ui        | React Aria                    | Notes                                                    |
| ------------ | ------------- | ----------------------------- | -------------------------------------------------------- |
| Root         | `Root`        | `Calendar`                    | Equivalent                                               |
| Header       | `Header`      | --                            | ars-ui groups nav controls; React Aria uses flat buttons |
| Prev trigger | `PrevTrigger` | `Button` (prev)               | Equivalent                                               |
| Next trigger | `NextTrigger` | `Button` (next)               | Equivalent                                               |
| Heading      | `Heading`     | `Heading`                     | Equivalent                                               |
| Grid         | `Grid`        | `CalendarGrid`                | Equivalent                                               |
| Grid group   | `GridGroup`   | --                            | ars-ui wraps multi-month grids                           |
| Head row     | `HeadRow`     | `CalendarGridHeader`          | Equivalent                                               |
| Head cell    | `HeadCell`    | `CalendarHeaderCell`          | Equivalent                                               |
| Body         | --            | `CalendarGridBody`            | ars-ui has no explicit body part                         |
| Row          | `Row`         | --                            | Implicit in React Aria                                   |
| Cell         | `Cell`        | `CalendarCell` (outer)        | Equivalent                                               |
| Cell trigger | `CellTrigger` | `CalendarCell` (inner button) | ars-ui splits cell and trigger                           |

**Gaps:** None. The anatomy is structurally equivalent with minor naming differences.

### 6.3 Events

| Callback     | ars-ui             | React Aria      | Notes      |
| ------------ | ------------------ | --------------- | ---------- |
| Value change | `SelectDate` event | `onChange`      | Equivalent |
| Focus change | `FocusDate` event  | `onFocusChange` | Equivalent |

**Gaps:** None.

### 6.4 Features

| Feature                 | ars-ui                    | React Aria           |
| ----------------------- | ------------------------- | -------------------- |
| Single date selection   | Yes                       | Yes                  |
| Multiple date selection | Yes (variant)             | No                   |
| Min/max constraints     | Yes                       | Yes                  |
| Unavailable dates       | Yes                       | Yes                  |
| Multi-month display     | Yes                       | Yes                  |
| Page behavior           | Yes                       | Yes                  |
| Locale-aware week start | Yes                       | Yes                  |
| Week numbers            | Yes                       | No                   |
| RTL support             | Yes                       | Yes (via locale)     |
| Keyboard navigation     | Yes (WAI-ARIA Grid)       | Yes (WAI-ARIA Grid)  |
| Calendar system support | Yes (CalendarSystem enum) | Yes (createCalendar) |

**Gaps:** None. ars-ui is a superset (week numbers, multiple selection variant).

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui supports week numbers and multi-date selection which React Aria does not. React Aria uses `createCalendar` factory while ars-ui uses a `CalendarSystem` enum.
- **Recommended additions:** None.

## Appendix: Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ars_core::Service;

    fn date(year: i32, month: u8, day: u8) -> CalendarDate {
        CalendarDate::new_gregorian(year, month, day).expect("valid test date")
    }

    fn props() -> Props {
        Props::new().id("cal").today(date(2024, 1, 15))
    }

    fn service() -> Service<Machine> {
        Service::<Machine>::new(props(), &Env::default(), &Messages::default())
    }

    #[test]
    fn initial_state_is_idle() {
        assert_eq!(*service().state(), State::Idle);
    }

    #[test]
    fn focus_in_transitions_to_focused() {
        let mut svc = service();
        drop(svc.send(Event::FocusIn));
        assert_eq!(*svc.state(), State::Focused);
    }

    #[test]
    fn arrow_right_advances_focused_date() {
        let mut svc = service();
        drop(svc.send(Event::FocusIn));
        let before = svc.context().focused_date.clone();
        drop(svc.send(Event::KeyDown { key: KeyboardKey::ArrowRight, shift: false }));
        assert_eq!(svc.context().focused_date, before.add_days(1).unwrap());
    }

    #[test]
    fn select_date_updates_value() {
        let mut svc = service();
        let d = date(2024, 1, 20);
        drop(svc.send(Event::SelectDate { date: d.clone() }));
        assert_eq!(*svc.context().value.get(), Some(d));
    }

    #[test]
    fn shift_page_down_advances_one_year() {
        let mut svc = service();
        drop(svc.send(Event::FocusIn));
        let before = svc.context().visible_year;
        drop(svc.send(Event::KeyDown { key: KeyboardKey::PageDown, shift: true }));
        assert_eq!(svc.context().visible_year, before + 1);
    }

    #[test]
    fn multi_select_toggle_round_trip() {
        let mut svc = Service::<Machine>::new(
            props().selection_mode(SelectionMode::Multiple),
            &Env::default(),
            &Messages::default(),
        );
        drop(svc.send(Event::ToggleDate { date: date(2024, 1, 10) }));
        assert!(svc.context().selected_dates.get().contains(&date(2024, 1, 10)));
        drop(svc.send(Event::ToggleDate { date: date(2024, 1, 10) }));
        assert!(!svc.context().selected_dates.get().contains(&date(2024, 1, 10)));
    }

    #[test]
    fn next_month_emits_announce_month_effect() {
        let mut svc = service();
        let result = svc.send(Event::NextMonth);
        assert!(result.pending_effects.iter().any(|e| e.name == Effect::AnnounceMonth));
    }

    #[test]
    fn is_next_disabled_checks_last_visible_month() {
        let svc = Service::<Machine>::new(
            props().visible_months(2).max(Some(date(2024, 1, 25))),
            &Env::default(),
            &Messages::default(),
        );
        let api = svc.connect(&|_| {});
        assert!(api.is_next_disabled());
    }
}
```
