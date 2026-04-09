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
    /// Move keyboard focus to a specific date.
    FocusDate {
        /// The date to focus on.
        date: CalendarDate,
    },
    /// User selected a date (click or Enter/Space on focused cell).
    SelectDate {
        /// The date to select.
        date: CalendarDate,
    },
    /// Navigate to the next month.
    NextMonth,
    /// Navigate to the previous month.
    PrevMonth,
    /// Navigate to the next year.
    NextYear,
    /// Navigate to the previous year.
    PrevYear,
    /// Jump to a specific month (1-based).
    SetMonth {
        /// The month to jump to.
        month: u8,
    },
    /// Jump to a specific year.
    SetYear {
        /// The year to jump to.
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
    },
}
```

### 1.3 Context

```rust
/// Context for the Calendar component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Selected date (single-select mode).
    pub value: Bindable<Option<CalendarDate>>,
    /// The date that currently holds keyboard focus within the grid.
    pub focused_date: CalendarDate,
    /// The month currently displayed (1-based).
    pub visible_month: u8,
    /// The year currently displayed.
    pub visible_year: i32,
    /// Minimum selectable date.
    pub min: Option<CalendarDate>,
    /// Maximum selectable date.
    pub max: Option<CalendarDate>,
    /// Locale for week start, month/day names.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// ICU data provider for locale-dependent formatting (month/weekday names, etc.).
    pub provider: ArsRc<dyn IcuProvider>,
    /// Override for first day of week (falls back to locale default).
    pub first_day_of_week: Weekday,
    /// Right-to-left layout.
    pub is_rtl: bool,
    /// Disabled state — ignores all interactive events.
    pub disabled: bool,
    /// Read-only state — can focus/navigate but not select.
    pub readonly: bool,
    /// Static list of unavailable dates (pre-computed from Props.is_date_unavailable
    /// for the currently visible month range). Refreshed on month navigation.
    pub unavailable_dates: Vec<CalendarDate>,
    /// Reference to the user-provided predicate from Props.is_date_unavailable.
    /// Used by `is_date_unavailable()` for dynamic evaluation beyond the static list.
    pub is_date_unavailable_fn: Option<fn(&CalendarDate) -> bool>,
    /// Component IDs.
    pub ids: ComponentIds,
    /// Number of months displayed side-by-side.
    pub visible_months: usize,
    /// Navigation step size.
    pub page_behavior: PageBehavior,
    /// Whether to display ISO week numbers.
    pub show_week_numbers: bool,
}

impl Context {
    /// Whether the given date is the currently selected value.
    pub fn is_selected(&self, date: &CalendarDate) -> bool {
        self.value.get().as_ref() == Some(date)
    }

    /// Whether a date is disabled (outside min/max or globally disabled).
    pub fn is_date_disabled(&self, date: &CalendarDate) -> bool {
        if self.disabled { return true; }
        if let Some(ref min) = self.min {
            if date < min { return true; }
        }
        if let Some(ref max) = self.max {
            if date > max { return true; }
        }
        false
    }

    /// Whether a date is marked unavailable by the user-provided predicate.
    /// Checks both the static `unavailable_dates` list and the dynamic
    /// `is_date_unavailable_fn` callback from Props. Unavailable dates are
    /// focusable (for keyboard navigation) but not selectable.
    pub fn is_date_unavailable(&self, date: &CalendarDate) -> bool {
        if self.unavailable_dates.contains(date) {
            return true;
        }
        if let Some(ref predicate) = self.is_date_unavailable_fn {
            return predicate(date);
        }
        false
    }

    /// Clamp a date into the min/max range.
    pub fn clamp_date(&self, date: CalendarDate) -> CalendarDate {
        let date = match &self.min {
            Some(min) if date < *min => min.clone(),
            _ => date,
        };
        match &self.max {
            Some(max) if date > *max => max.clone(),
            _ => date,
        }
    }

    /// Ensure the focused date's month is visible.
    /// Only scrolls when the focused date falls outside all visible months.
    pub fn sync_visible_to_focused(&mut self) {
        if !self.is_in_visible_range(&self.focused_date) {
            self.visible_month = self.focused_date.month.get();
            self.visible_year = self.focused_date.year;
        }
    }
}
```

### 1.4 Props

```rust
/// Props for the Calendar component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled single-date value.
    pub value: Option<Option<CalendarDate>>,
    /// Default date for uncontrolled mode.
    pub default_value: Option<CalendarDate>,
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
    pub is_date_unavailable: Option<fn(&CalendarDate) -> bool>,
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
    /// Number of months to display side-by-side. Default: 1.
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
            today: CalendarDate::new_gregorian(2025, nzu8(1), nzu8(1)),
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

```rust
/// Controls how prev/next navigation advances when multiple months are visible.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
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
        let start = self.first_day_of_week as u8;
        (0..7)
            .map(|i| {
                let wd = Weekday::from_u8((start + i) % 7);
                let label = wd.short_label(&*self.provider, &self.locale);
                (wd, label)
            })
            .collect()
    }

    /// Build the grid for the first visible month. Equivalent to `weeks_for(0)`.
    pub fn weeks(&self) -> Vec<[CalendarDate; 7]> {
        self.weeks_for(0)
    }

    /// Build the grid of weeks for the month at the given offset
    /// from the first visible month. Offset 0 = first visible month.
    pub fn weeks_for(&self, offset: usize) -> Vec<[CalendarDate; 7]> {
        let anchor = CalendarDate::new_gregorian(self.visible_year, NonZero::new(self.visible_month).expect("visible_month is 1-based"), nzu8(1));
        let target = anchor.add_months(offset as i32);
        let first_of_month = CalendarDate::new_gregorian(target.year, target.month, nzu8(1));
        let first_weekday = first_of_month.weekday() as u8;
        let start_offset = ((first_weekday + 7 - self.first_day_of_week as u8) % 7) as i32;
        let grid_start = first_of_month.add_days(-start_offset);

        let mut weeks = Vec::new();
        let mut current = grid_start.clone();
        // Always render 6 weeks (42 days) for consistent grid height.
        for _ in 0..6 {
            let mut week = [current.clone(); 7];
            for d in 0..7 {
                week[d] = current.add_days(d as i32);
            }
            weeks.push(week);
            current = current.add_days(7);
        }
        weeks
    }

    /// Returns (month, year) for the month at the given offset.
    pub fn month_year_at_offset(&self, offset: usize) -> (u8, i32) {
        let anchor = CalendarDate::new_gregorian(self.visible_year, NonZero::new(self.visible_month).expect("visible_month is 1-based"), nzu8(1));
        let target = anchor.add_months(offset as i32);
        (target.month.get(), target.year)
    }

    /// Whether a date is outside the month at the specified offset.
    pub fn is_outside_month_at_offset(&self, date: &CalendarDate, offset: usize) -> bool {
        let (month, year) = self.month_year_at_offset(offset);
        date.month.get() != month || date.year != year
    }

    /// Whether the date's month falls within any of the visible months.
    pub fn is_in_visible_range(&self, date: &CalendarDate) -> bool {
        for offset in 0..self.visible_months {
            let (month, year) = self.month_year_at_offset(offset);
            if date.month.get() == month && date.year == year {
                return true;
            }
        }
        false
    }

    /// Navigate the visible month/year forward by `n` months.
    pub fn advance_month(&mut self, n: i32) {
        let anchor = CalendarDate::new_gregorian(self.visible_year, NonZero::new(self.visible_month).expect("visible_month is 1-based"), nzu8(1));
        let next = anchor.add_months(n);
        self.visible_month = next.month.get().clamp(1, 12);
        self.visible_year = next.year;
    }

    /// Whether the given date is outside the currently visible month.
    pub fn is_outside_visible_month(&self, date: &CalendarDate) -> bool {
        date.month.get() != self.visible_month || date.year != self.visible_year
    }
}
```

### 1.7 Full Machine Implementation

```rust
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let initial_date = props.value.flatten();
        let focused = initial_date
            .clone()
            .unwrap_or_else(|| props.today.clone());

        let value = match props.value {
            Some(v) => Bindable::controlled(v),
            None    => Bindable::uncontrolled(props.default_value.clone()),
        };

        let locale = env.locale.clone();
        let messages = messages.clone();

        let first_day = props
            .first_day_of_week
            .unwrap_or_else(|| locale.first_day_of_week());

        let ctx = Context {
            value,
            focused_date: focused.clone(),
            visible_month: focused.month.get(),
            visible_year: focused.year,
            min: props.min.clone(),
            max: props.max.clone(),
            locale,
            messages,
            provider: env.icu_provider.clone(),
            first_day_of_week: first_day,
            is_rtl: props.is_rtl,
            disabled: props.disabled,
            readonly: props.readonly,
            unavailable_dates: Vec::new(),
            is_date_unavailable_fn: props.is_date_unavailable,
            ids: ComponentIds::from_id(&props.id),
            visible_months: props.visible_months.max(1),
            page_behavior: props.page_behavior.clone(),
            show_week_numbers: props.show_week_numbers,
        };

        (State::Idle, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled { return None; }

        match event {
            // ── Focus management ─────────────────────────────────────────
            Event::FocusIn => {
                Some(TransitionPlan::to(State::Focused))
            }

            Event::FocusOut => {
                Some(TransitionPlan::to(State::Idle))
            }

            Event::FocusDate { date } => {
                let date = ctx.clamp_date(date.clone());
                Some(TransitionPlan::to(State::Focused)
                    .apply(move |ctx| {
                        ctx.focused_date = date;
                        ctx.sync_visible_to_focused();
                    }))
            }

            // ── Date selection ────────────────────────────────────────────
            Event::SelectDate { date } => {
                if ctx.readonly { return None; }
                if ctx.is_date_disabled(date) || ctx.is_date_unavailable(date) {
                    return None;
                }
                let date = date.clone();
                Some(TransitionPlan::to(State::Focused)
                    .apply(move |ctx| {
                        ctx.value.set(Some(date.clone()));
                        ctx.focused_date = date;
                    }))
            }
            // ── Month / year navigation ──────────────────────────────────
            Event::NextMonth => {
                Some(TransitionPlan::context_only(|ctx| {
                    let step = match ctx.page_behavior {
                        PageBehavior::Visible => ctx.visible_months as i32,
                        PageBehavior::Single  => 1,
                    };
                    ctx.advance_month(step);
                }).with_effect(PendingEffect::new("announce-month", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let label = if ctx.visible_months > 1 {
                        let first = format!("{} {}", month_long_name(&*ctx.provider, ctx.visible_month, &ctx.locale), ctx.visible_year);
                        let (lm, ly) = ctx.month_year_at_offset(ctx.visible_months - 1);
                        let sep = (ctx.messages.month_range_separator)(&ctx.locale);
                        format!("{}{}{} {}", first, sep, month_long_name(&*ctx.provider, lm, &ctx.locale), ly)
                    } else {
                        format!("{} {}", month_long_name(&*ctx.provider, ctx.visible_month, &ctx.locale), ctx.visible_year)
                    };
                    platform.announce(&label);
                    no_cleanup()
                })))
            }

            Event::PrevMonth => {
                Some(TransitionPlan::context_only(|ctx| {
                    let step = match ctx.page_behavior {
                        PageBehavior::Visible => ctx.visible_months as i32,
                        PageBehavior::Single  => 1,
                    };
                    ctx.advance_month(-step);
                }).with_effect(PendingEffect::new("announce-month", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let label = if ctx.visible_months > 1 {
                        let first = format!("{} {}", month_long_name(&*ctx.provider, ctx.visible_month, &ctx.locale), ctx.visible_year);
                        let (lm, ly) = ctx.month_year_at_offset(ctx.visible_months - 1);
                        let sep = (ctx.messages.month_range_separator)(&ctx.locale);
                        format!("{}{}{} {}", first, sep, month_long_name(&*ctx.provider, lm, &ctx.locale), ly)
                    } else {
                        format!("{} {}", month_long_name(&*ctx.provider, ctx.visible_month, &ctx.locale), ctx.visible_year)
                    };
                    platform.announce(&label);
                    no_cleanup()
                })))
            }

            Event::NextYear => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.advance_month(12);
                }))
            }

            Event::PrevYear => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.advance_month(-12);
                }))
            }

            Event::SetMonth { month } => {
                let month = *month;
                if !(1..=12).contains(&month) { return None; }
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.visible_month = month;
                }))
            }

            Event::SetYear { year } => {
                let year = *year;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.visible_year = year;
                }))
            }

            // ── Keyboard navigation ──────────────────────────────────────
            Event::KeyDown { ref key } => {
                Self::handle_keydown(state, &KeyboardEventData { key: *key }, ctx)
            }

            // Catch-all for guarded match arms.
            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}

impl Machine {
    /// Keyboard navigation within the calendar grid.
    /// Follows WAI-ARIA Grid pattern with RTL awareness.
    fn handle_keydown(
        state: &State,
        data: &KeyboardEventData,
        ctx: &Context,
    ) -> Option<TransitionPlan<Self>> {
        // Only handle keys when the grid is focused.
        if *state != State::Focused { return None; }

        let focused = ctx.focused_date.clone();
        // RTL-aware directional keys: in RTL, Left means "next" and Right means "prev".
        let (prev_day_key, next_day_key) = if ctx.is_rtl {
            (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
        } else {
            (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
        };

        let new_focus = match data.key {
            k if k == prev_day_key => Some(focused.add_days(-1)),
            k if k == next_day_key => Some(focused.add_days(1)),
            KeyboardKey::ArrowUp              => Some(focused.add_days(-7)),
            KeyboardKey::ArrowDown            => Some(focused.add_days(7)),
            KeyboardKey::Home                 => {
                // Move to start of current week.
                let wd = focused.weekday() as u8;
                let start = ctx.first_day_of_week as u8;
                let offset = ((wd + 7 - start) % 7) as i32;
                Some(focused.add_days(-offset))
            }
            KeyboardKey::End => {
                // Move to end of current week.
                let wd = focused.weekday() as u8;
                let start = ctx.first_day_of_week as u8;
                let offset = ((wd + 7 - start) % 7) as i32;
                Some(focused.add_days(6 - offset))
            }
            KeyboardKey::PageUp  => Some(focused.add_months(-1)),
            KeyboardKey::PageDown => Some(focused.add_months(1)),
            _ => None,
        };

        // Shift+PageUp/PageDown for year navigation is handled separately
        // because the key enum itself doesn't encode the shift modifier.
        // Adapters call `on_grid_keydown(data, shift)` which maps:
        //   Shift+PageUp   -> Event::PrevYear
        //   Shift+PageDown -> Event::NextYear

        match new_focus {
            Some(date) => {
                let clamped = ctx.clamp_date(date);
                Some(TransitionPlan::to(State::Focused)
                    .apply(move |ctx| {
                        ctx.focused_date = clamped;
                        ctx.sync_visible_to_focused();
                    }))
            }
            None => {
                // Enter / Space selects the focused date.
                match data.key {
                    KeyboardKey::Enter | KeyboardKey::Space => {
                        let date = focused;
                        Some(TransitionPlan::context_only(|_ctx| {})
                            .then(Event::SelectDate { date }))
                    }
                    _ => None,
                }
            }
        }
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

    /// Attributes for the heading text element.
    pub fn heading_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, &self.ctx.heading_id);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Heading.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
        attrs
    }

    /// Formatted heading text: e.g. "January 2024".
    pub fn heading_text(&self) -> String {
        let month_name = month_long_name(&*self.ctx.provider, self.ctx.visible_month, &self.ctx.locale);
        format!("{} {}", month_name, self.ctx.visible_year)
    }

    /// Attributes for the grid element.
    pub fn grid_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, &self.ctx.grid_id);
        attrs.set(HtmlAttr::Role, "grid");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Grid.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), &self.ctx.heading_id);
        if self.ctx.is_range {
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

    /// Grid attributes for the month at the given offset.
    pub fn grid_attrs_for(&self, offset: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let grid_id = format!("{}-grid-{}", self.ctx.id, offset);
        let heading_id = format!("{}-heading-{}", self.ctx.id, offset);
        attrs.set(HtmlAttr::Id, grid_id);
        attrs.set(HtmlAttr::Role, "grid");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Grid.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy),  heading_id);
        if self.ctx.is_range {
            attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        }
        if self.ctx.readonly { attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true"); }
        if self.ctx.disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        attrs
    }

    /// Heading text for the month at the given offset.
    pub fn heading_text_for(&self, offset: usize) -> String {
        let (month, year) = self.ctx.month_year_at_offset(offset);
        format!("{} {}", month_long_name(&*self.ctx.provider, month, &self.ctx.locale), year)
    }

    /// Heading text attributes for a per-grid heading.
    pub fn heading_text_attrs_for(&self, offset: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, format!("{}-heading-{}", self.ctx.id, offset));
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Heading.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // No aria-live on per-grid headings; only the main heading has it.
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
    /// Only rendered when visible_months > 1.
    pub fn grid_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, format!("{}-grid-group", self.ctx.id));
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
        attrs.set(HtmlAttr::Abbr, weekday.long_label(&*self.ctx.provider, &self.ctx.locale));
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
        let is_today = *date == self.props.today;
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
            "{} {} {}, {}",
            month_long_name(&*self.ctx.provider, date.month.get(), &self.ctx.locale),
            date.day.get(),
            date.year,
            date.weekday().long_label(&*self.ctx.provider, &self.ctx.locale),
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

    /// Handle click on a day cell.
    pub fn on_cell_click(&self, date: CalendarDate) {
        (self.send)(Event::SelectDate { date });
    }

    /// Handle hover over a day cell (for range preview).
    pub fn on_cell_hover(&self, date: CalendarDate) {
        (self.send)(Event::HoverDate { date });
    }

    /// Handle mouse leaving the grid.
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

    /// Handle keydown on the grid. Shift modifier handled here for year nav.
    pub fn on_grid_keydown(&self, key: KeyboardKey, shift: bool) {
        match (key, shift) {
            (KeyboardKey::PageUp, true)  => (self.send)(Event::PrevYear),
            (KeyboardKey::PageDown, true) => (self.send)(Event::NextYear),
            _ => (self.send)(Event::KeyDown { key }),
        }
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
        *date == self.props.today
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
        if let Some(ref min) = self.ctx.min {
            let first_of_visible = CalendarDate::new_gregorian(
                self.ctx.visible_year,
                NonZero::new(self.ctx.visible_month).expect("visible_month is 1-based"),
                nzu8(1),
            );
            first_of_visible <= *min
        } else {
            false
        }
    }

    /// Whether the next button should be disabled (max constraint).
    /// Checks against the **last** visible month instead of the first.
    pub fn is_next_disabled(&self) -> bool {
        if let Some(ref max) = self.ctx.max {
            let last_offset = self.ctx.visible_months.saturating_sub(1);
            let (month, year) = self.ctx.month_year_at_offset(last_offset);
            let first_of_last = CalendarDate::new_gregorian(year, NonZero::new(month).expect("month is 1-based"), nzu8(1));
            let last_of_last = CalendarDate::new_gregorian(
                year, NonZero::new(month).expect("month is 1-based"), NonZero::new(first_of_last.days_in_month()).expect("days_in_month is >= 1"),
            );
            last_of_last >= *max
        } else {
            false
        }
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
            Part::Heading => self.heading_text_attrs(),
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

/// Locale-specific labels for the Calendar component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the previous month navigation button.
    pub prev_month_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the next month navigation button.
    pub next_month_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for prev button when navigating multiple months.
    /// The `usize` parameter is the step count.
    pub prev_page_label: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
    /// Label for next button when navigating multiple months.
    /// The `usize` parameter is the step count.
    pub next_page_label: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
    /// Separator between month names in multi-month range heading (e.g., " – ").
    pub month_range_separator: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}
// Month names, weekday names, and abbreviations are resolved from the IcuProvider
// (via `month_long_name()`, `wd.short_label()`, etc.) — not stored in Messages.

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

Calendar supports multiple non-contiguous date selection in addition to the default single-date mode.

### 5.1 Additional Props

```rust
/// Added to Props.
pub selection_mode: CalendarSelectionMode,
/// Maximum number of dates that can be selected.
pub max_selected: Option<usize>,

#[derive(Clone, Debug, PartialEq, Default)]
pub enum CalendarSelectionMode {
    /// Select a single date (default behavior).
    #[default]
    Single,
    /// Select multiple non-contiguous dates.
    Multiple,
}
```

### 5.2 Additional Context

```rust
/// Extended CalendarContext for multi-selection.
/// When selection_mode is Multiple:
pub selected_dates: BTreeSet<CalendarDate>,
```

### 5.3 Additional Events

```rust
/// Added to Calendar Event enum.
/// Toggle a date in/out of the selection set.
ToggleDate(CalendarDate),
```

### 5.4 Behavior

| Action   | Click                 | Shift+Click                                 |
| -------- | --------------------- | ------------------------------------------- |
| Multiple | Toggle date selection | Select range from last-clicked to this date |

When `max_selected` is set, the `ToggleDate` transition guards against `selected_dates.len() >= max_selected`. Excess selections are silently prevented.

### 5.5 Keyboard

| Key           | Behavior                                  |
| ------------- | ----------------------------------------- |
| Space / Enter | Toggle focused date in/out of selection   |
| Shift+Space   | Select range from anchor to focused       |
| Ctrl+A        | Select all visible dates in current month |

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
    use ars_core::service::Service;

    fn make_service() -> Service<Machine> {
        Service::new(Props {
            id: "test-cal".into(),
            value: None,
            default_value: None,
            min: None,
            max: None,
            disabled: false,
            readonly: false,
            is_date_unavailable: None,
            first_day_of_week: None,
            is_range: false,
            is_rtl: false,
            visible_months: 1,
            page_behavior: PageBehavior::Visible,
            today: CalendarDate::new_gregorian(2024, nzu8(1), nzu8(15)),
        }, Env::default(), Default::default())
    }

    #[test]
    fn initial_state_is_idle() {
        let svc = make_service();
        assert_eq!(*svc.state(), State::Idle);
    }

    #[test]
    fn focus_in_transitions_to_focused() {
        let mut svc = make_service();
        svc.send(Event::FocusIn);
        assert_eq!(*svc.state(), State::Focused);
    }

    #[test]
    fn arrow_right_advances_focused_date() {
        let mut svc = make_service();
        svc.send(Event::FocusIn);
        let before = svc.context().focused_date.clone();
        svc.send(Event::KeyDown { key: KeyboardKey::ArrowRight });
        assert_eq!(svc.context().focused_date, before.add_days(1));
    }

    #[test]
    fn arrow_up_moves_back_one_week() {
        let mut svc = make_service();
        svc.send(Event::FocusIn);
        let before = svc.context().focused_date.clone();
        svc.send(Event::KeyDown { key: KeyboardKey::ArrowUp });
        assert_eq!(svc.context().focused_date, before.add_days(-7));
    }

    #[test]
    fn select_date_updates_value() {
        let mut svc = make_service();
        let date = CalendarDate::new_gregorian(2024, nzu8(1), nzu8(20));
        svc.send(Event::SelectDate { date: date.clone() });
        assert_eq!(*svc.context().value.get(), Some(date));
    }

    #[test]
    fn next_month_advances_visible_month() {
        let mut svc = make_service();
        assert_eq!(svc.context().visible_month, 1);
        svc.send(Event::NextMonth);
        assert_eq!(svc.context().visible_month, 2);
    }

    #[test]
    fn prev_month_goes_back() {
        let mut svc = make_service();
        assert_eq!(svc.context().visible_month, 1);
        svc.send(Event::PrevMonth);
        assert_eq!(svc.context().visible_month, 12);
        assert_eq!(svc.context().visible_year, 2023);
    }

    #[test]
    fn disabled_calendar_ignores_events() {
        let mut svc = Service::new(Props {
            disabled: true,
            ..make_service().props().clone()
        }, Env::default(), Default::default());
        svc.send(Event::SelectDate {
            date: CalendarDate::new_gregorian(2024, nzu8(1), nzu8(20)),
        });
        assert_eq!(*svc.context().value.get(), None);
    }

    #[test]
    fn range_mode_first_click_sets_start() {
        let mut svc = Service::new(Props {
            is_range: true,
            ..make_service().props().clone()
        }, Env::default(), Default::default());
        let date = CalendarDate::new_gregorian(2024, nzu8(1), nzu8(10));
        svc.send(Event::SelectDate { date: date.clone() });
        assert_eq!(svc.context().range_start, Some(date));
        assert_eq!(svc.context().range_end, None);
    }

    #[test]
    fn range_mode_second_click_completes_range() {
        let mut svc = Service::new(Props {
            is_range: true,
            ..make_service().props().clone()
        }, Env::default(), Default::default());
        let start = CalendarDate::new_gregorian(2024, nzu8(1), nzu8(10));
        let end = CalendarDate::new_gregorian(2024, nzu8(1), nzu8(20));
        svc.send(Event::SelectDate { date: start.clone() });
        svc.send(Event::SelectDate { date: end.clone() });
        assert_eq!(svc.context().range_start, Some(start));
        assert_eq!(svc.context().range_end, Some(end));
    }

    #[test]
    fn range_mode_swaps_if_end_before_start() {
        let mut svc = Service::new(Props {
            is_range: true,
            ..make_service().props().clone()
        }, Env::default(), Default::default());
        let later = CalendarDate::new_gregorian(2024, nzu8(1), nzu8(20));
        let earlier = CalendarDate::new_gregorian(2024, nzu8(1), nzu8(5));
        svc.send(Event::SelectDate { date: later.clone() });
        svc.send(Event::SelectDate { date: earlier.clone() });
        assert_eq!(svc.context().range_start, Some(earlier));
        assert_eq!(svc.context().range_end, Some(later));
    }

    #[test]
    fn min_max_clamps_focused_date() {
        let mut svc = Service::new(Props {
            min: Some(CalendarDate::new_gregorian(2024, nzu8(1), nzu8(10))),
            max: Some(CalendarDate::new_gregorian(2024, nzu8(1), nzu8(25))),
            ..make_service().props().clone()
        }, Env::default(), Default::default());
        svc.send(Event::FocusDate {
            date: CalendarDate::new_gregorian(2024, nzu8(1), nzu8(1)),
        });
        assert_eq!(
            svc.context().focused_date,
            CalendarDate::new_gregorian(2024, nzu8(1), nzu8(10)),
        );
    }

    #[test]
    fn rtl_arrow_keys_reversed() {
        let mut svc = Service::new(Props {
            is_rtl: true,
            ..make_service().props().clone()
        }, Env::default(), Default::default());
        svc.send(Event::FocusIn);
        let before = svc.context().focused_date.clone();
        // In RTL, ArrowLeft should advance (next day).
        svc.send(Event::KeyDown { key: KeyboardKey::ArrowLeft });
        assert_eq!(svc.context().focused_date, before.add_days(1));
    }

    // ── Multi-month tests ───────────────────────────────────────────

    fn make_multi_month_service(months: usize, behavior: PageBehavior) -> Service<Machine> {
        Service::new(Props {
            visible_months: months,
            page_behavior: behavior,
            ..make_service().props().clone()
        }, Env::default(), Default::default())
    }

    #[test]
    fn multi_month_weeks_for_returns_correct_months() {
        let svc = make_multi_month_service(2, PageBehavior::Visible);
        let weeks0 = svc.context().weeks_for(0);
        let weeks1 = svc.context().weeks_for(1);
        // First grid should contain January dates, second should contain February.
        // Check a mid-month date in each grid.
        assert_eq!(weeks0[2][3].month.get(), 1); // mid-January
        assert_eq!(weeks1[2][3].month.get(), 2); // mid-February
    }

    #[test]
    fn next_month_advances_by_visible_months_in_visible_mode() {
        let mut svc = make_multi_month_service(3, PageBehavior::Visible);
        assert_eq!(svc.context().visible_month, 1);
        svc.send(Event::NextMonth);
        // Should jump +3: January → April.
        assert_eq!(svc.context().visible_month, 4);
    }

    #[test]
    fn next_month_advances_by_1_in_single_mode() {
        let mut svc = make_multi_month_service(3, PageBehavior::Single);
        assert_eq!(svc.context().visible_month, 1);
        svc.send(Event::NextMonth);
        // Should jump +1: January → February.
        assert_eq!(svc.context().visible_month, 2);
    }

    #[test]
    fn prev_month_steps_back_by_visible_months() {
        let mut svc = make_multi_month_service(2, PageBehavior::Visible);
        // Advance to March first.
        svc.send(Event::NextMonth); // Jan → Mar (step 2)
        assert_eq!(svc.context().visible_month, 3);
        svc.send(Event::PrevMonth); // Mar → Jan (step -2)
        assert_eq!(svc.context().visible_month, 1);
    }

    #[test]
    fn sync_visible_does_not_scroll_when_focus_in_range() {
        let mut svc = make_multi_month_service(2, PageBehavior::Visible);
        // visible_month = 1 (Jan), visible_months = 2, so Jan+Feb visible.
        // Focus on a Feb date — should NOT scroll.
        let feb_date = CalendarDate::new_gregorian(2024, nzu8(2), nzu8(10));
        svc.send(Event::FocusDate { date: feb_date });
        assert_eq!(svc.context().visible_month, 1); // Still January
    }

    #[test]
    fn sync_visible_scrolls_when_focus_leaves_range() {
        let mut svc = make_multi_month_service(2, PageBehavior::Visible);
        // visible range is Jan+Feb. Focus on March — should scroll.
        let mar_date = CalendarDate::new_gregorian(2024, nzu8(3), nzu8(5));
        svc.send(Event::FocusDate { date: mar_date });
        assert_eq!(svc.context().visible_month, 3); // Scrolled to March
    }

    #[test]
    fn is_in_visible_range_checks_all_months() {
        let svc = make_multi_month_service(3, PageBehavior::Visible);
        let jan = CalendarDate::new_gregorian(2024, nzu8(1), nzu8(15));
        let feb = CalendarDate::new_gregorian(2024, nzu8(2), nzu8(15));
        let mar = CalendarDate::new_gregorian(2024, nzu8(3), nzu8(15));
        let apr = CalendarDate::new_gregorian(2024, nzu8(4), nzu8(1));
        assert!(svc.context().is_in_visible_range(&jan));
        assert!(svc.context().is_in_visible_range(&feb));
        assert!(svc.context().is_in_visible_range(&mar));
        assert!(!svc.context().is_in_visible_range(&apr));
    }

    #[test]
    fn month_year_at_offset_wraps_year() {
        let mut svc = make_multi_month_service(3, PageBehavior::Visible);
        // Set visible to November 2024.
        svc.send(Event::SetMonth { month: 11 });
        let (m, y) = svc.context().month_year_at_offset(2);
        assert_eq!(m, 1);    // January
        assert_eq!(y, 2025); // Next year
    }

    #[test]
    fn is_next_disabled_checks_last_visible_month() {
        let svc = Service::new(Props {
            visible_months: 2,
            page_behavior: PageBehavior::Visible,
            max: Some(CalendarDate::new_gregorian(2024, nzu8(2), nzu8(28))),
            ..make_service().props().clone()
        }, Env::default(), Default::default());
        let api = Machine::connect(
            svc.state(), svc.context(), svc.props(), &|_| {},
        );
        // visible_month=1 (Jan), visible_months=2 → last visible is Feb.
        // max = Feb 28. Last day of Feb = Feb 29 (2024 is a leap year) ≥ Feb 28.
        // But Feb 28 is the max and the last of the last visible month
        // includes Feb 29 which is ≥ max, so next should be disabled.
        assert!(api.is_next_disabled());
    }
}
```
