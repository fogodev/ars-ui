---
component: RangeCalendar
category: date-time
tier: stateful
foundation_deps: [architecture, accessibility, i18n, interactions]
shared_deps: [date-time-types]
related: [calendar, date-range-picker]
references:
    react-aria: RangeCalendar
---

# RangeCalendar

RangeCalendar is a grid-based date range selection component. It renders the same month grid as Calendar but selects a `DateRange` (start + end) instead of a single `CalendarDate`. Selection follows a two-click model: the first click sets an anchor date, the second click completes the range. A hover preview shows the tentative range between clicks.

RangeCalendar has its own state machine with a distinct value type (`Bindable<Option<DateRange>>`). It shares Calendar's grid computation, navigation logic, keyboard handling, and Part structure. For single-date selection, see `calendar.md`. `DateRangePicker` composes RangeCalendar inside a popover -- see `date-range-picker.md` for integration details.

> **Shared logic:** Grid computation (`weeks()`, `weeks_for()`, `month_year_at_offset()`, `advance_month()`), page behavior, week-start resolution, and keyboard navigation are identical to Calendar. This spec references Calendar for those algorithms rather than duplicating them. Implementors should extract a shared `calendar_base` module.

## 1. State Machine

### 1.1 States

```rust
/// States for the RangeCalendar component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Calendar is rendered but no cell has keyboard focus.
    Idle,
    /// A specific date cell has keyboard focus within the grid.
    Focused,
}
```

The state enum is identical to Calendar's. Range selection progress is tracked in `Context.anchor_date`, not in the state enum, because the anchor persists across focus changes.

### 1.2 Events

```rust
/// Events for the RangeCalendar component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Move keyboard focus to a specific date.
    FocusDate { date: CalendarDate },
    /// User selected a date (click or Enter/Space on focused cell).
    /// First click sets the anchor; second click completes the range.
    SelectDate { date: CalendarDate },
    /// Pointer is hovering over a date cell (for range preview).
    HoverDate { date: CalendarDate },
    /// Pointer left the grid (clear hover preview).
    HoverEnd,
    /// Navigate to the next month(s).
    NextMonth,
    /// Navigate to the previous month(s).
    PrevMonth,
    /// Navigate to the next year.
    NextYear,
    /// Navigate to the previous year.
    PrevYear,
    /// Jump to a specific month (1-based).
    SetMonth { month: u8 },
    /// Jump to a specific year.
    SetYear { year: i32 },
    /// Grid received focus.
    FocusIn,
    /// Focus left the grid entirely.
    FocusOut,
    /// Keyboard event on the grid.
    KeyDown { key: KeyboardKey },
}
```

Compared to Calendar, RangeCalendar adds `HoverDate` and `HoverEnd` for the range preview. The `SelectDate` event serves both the first click (set anchor) and the second click (complete range).

### 1.3 Context

```rust
/// Context for the RangeCalendar component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The selected date range. `None` means no range is selected.
    pub value: Bindable<Option<DateRange>>,
    /// Anchor date for range selection (set on first click).
    /// `Some` means a first click has occurred and we are waiting for the second.
    /// `None` means no range selection is in progress.
    pub anchor_date: Option<CalendarDate>,
    /// Date the pointer is currently hovering over (for range preview).
    /// Only meaningful when `anchor_date` is `Some`.
    pub hovering_date: Option<CalendarDate>,
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
    pub provider: Arc<dyn IcuProvider>,
    /// Override for first day of week (falls back to locale default).
    pub first_day_of_week: Weekday,
    /// Right-to-left layout.
    pub is_rtl: bool,
    /// Disabled state -- ignores all interactive events.
    pub disabled: bool,
    /// Read-only state -- can focus/navigate but not select.
    pub readonly: bool,
    /// Static list of unavailable dates (pre-computed for the visible month range).
    pub unavailable_dates: Vec<CalendarDate>,
    /// User-provided predicate for dynamic unavailability checks.
    pub is_date_unavailable_fn: Option<fn(&CalendarDate) -> bool>,
    /// Component IDs.
    pub ids: ComponentIds,
    /// Number of months displayed side-by-side. Default: 2 (common for range pickers).
    pub visible_months: usize,
    /// Navigation step size.
    pub page_behavior: PageBehavior,
    /// Whether to display ISO week numbers.
    pub show_week_numbers: bool,
}
```

Context shares the same navigation fields as Calendar (`focused_date`, `visible_month`, `visible_year`, `page_behavior`, etc.) and the same grid computation methods -- see Calendar `calendar.md` section 1.6 for `weeks()`, `weeks_for()`, `month_year_at_offset()`, `advance_month()`, `sync_visible_to_focused()`, `clamp_date()`, and related helpers. These methods are identical and should be extracted into a shared module by implementors.

The range-specific fields are `anchor_date`, `hovering_date`, and the `Bindable<Option<DateRange>>` value type.

**Date constraint methods** (identical to Calendar):

```rust
impl Context {
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
}
```

### 1.4 Props

```rust
/// Props for the RangeCalendar component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled date range value. `Some(Some(range))` = controlled with a range,
    /// `Some(None)` = controlled with no selection, `None` = uncontrolled.
    pub value: Option<Option<DateRange>>,
    /// Default date range for uncontrolled mode.
    pub default_value: Option<DateRange>,
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
    pub first_day_of_week: Option<Weekday>,
    /// Whether to display ISO week numbers.
    pub show_week_numbers: bool,
    /// Right-to-left layout direction.
    pub is_rtl: bool,
    /// Number of months to display side-by-side. Default: 2.
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
            visible_months: 2,
            page_behavior: PageBehavior::Visible,
            today: CalendarDate::new_gregorian(2024, nzu8(1), nzu8(1)),
        }
    }
}
```

The default `visible_months` is 2 (showing side-by-side months is the common pattern for range calendars).

### 1.5 Guards

```rust
fn is_disabled(ctx: &Context) -> bool { ctx.disabled }
fn is_readonly(ctx: &Context) -> bool { ctx.readonly }
fn is_date_disabled(ctx: &Context, date: &CalendarDate) -> bool { ctx.is_date_disabled(date) }
fn is_date_unavailable(ctx: &Context, date: &CalendarDate) -> bool { ctx.is_date_unavailable(date) }
```

### 1.6 Range Selection Workflow

Range selection follows a two-click workflow:

1. **First click** (`SelectDate`): Sets `ctx.anchor_date` to the clicked date. Clears any previous range value. The calendar enters the "range-pending" visual state where `anchor_date` is `Some` but no completed range exists yet.
2. **Hover** (between clicks): While `anchor_date` is `Some` and the range is not yet completed, hovering over dates updates `ctx.hovering_date`, creating a live preview of the potential range. Cells between the anchor and the hovered date receive the `data-ars-in-hover-range` attribute.
3. **Second click** (`SelectDate`): Completes the range. The clicked date and `anchor_date` are passed through `DateRange::normalized()` to ensure `start <= end` regardless of click order. The completed range is written to `ctx.value`, `anchor_date` is cleared, and `hovering_date` is cleared.
4. **Subsequent click**: Starts a new range -- sets a new `anchor_date`, clears the previous range value, and repeats from step 1.

**Keyboard selection** follows the same two-click model: pressing Enter or Space on a focused date triggers `SelectDate`, functioning identically to a pointer click.

### 1.7 Range Context Helpers

```rust
impl Context {
    /// Whether a date falls within the confirmed range (inclusive).
    /// Returns `false` when no range is selected.
    pub fn is_in_range(&self, date: &CalendarDate) -> bool {
        match self.value.get() {
            Some(ref range) => range.contains(date),
            None => false,
        }
    }

    /// Whether a date is the start of the confirmed range.
    pub fn is_range_start(&self, date: &CalendarDate) -> bool {
        match self.value.get() {
            Some(ref range) => *date == range.start,
            None => false,
        }
    }

    /// Whether a date is the end of the confirmed range.
    pub fn is_range_end(&self, date: &CalendarDate) -> bool {
        match self.value.get() {
            Some(ref range) => *date == range.end,
            None => false,
        }
    }

    /// Whether a date falls within the hover preview range during the pending
    /// state (between first and second click). The preview range spans from
    /// `anchor_date` to `hovering_date`, inclusive, normalized so that start <= end.
    pub fn is_in_hover_range(&self, date: &CalendarDate) -> bool {
        match (self.anchor_date.as_ref(), self.hovering_date.as_ref()) {
            (Some(anchor), Some(hover)) => {
                let range = DateRange::normalized(anchor.clone(), hover.clone());
                range.contains(date)
            }
            _ => false,
        }
    }

    /// Whether a date is the anchor (the first-clicked date in a pending selection).
    pub fn is_anchor(&self, date: &CalendarDate) -> bool {
        self.anchor_date.as_ref() == Some(date)
    }

    /// Whether a range selection is in progress (anchor set, waiting for second click).
    pub fn is_range_pending(&self) -> bool {
        self.anchor_date.is_some()
    }
}
```

### 1.8 Full Machine Implementation

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
        let initial_range = props.value.flatten();
        let focused = initial_range
            .as_ref()
            .map(|r| r.start.clone())
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
            anchor_date: None,
            hovering_date: None,
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
        props: &Self::Props,
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

            // ── Range selection ──────────────────────────────────────────
            Event::SelectDate { date } => {
                if ctx.readonly { return None; }
                if ctx.is_date_disabled(date) || ctx.is_date_unavailable(date) {
                    return None;
                }
                let date = date.clone();
                match ctx.anchor_date {
                    None => {
                        // First click: set anchor, clear any existing range.
                        Some(TransitionPlan::to(State::Focused)
                            .apply(move |ctx| {
                                ctx.anchor_date = Some(date.clone());
                                ctx.value.set(None);
                                ctx.focused_date = date;
                                ctx.hovering_date = None;
                            })
                            .with_effect(PendingEffect::new(
                                "announce-range-start",
                                |ctx, _props, _send| {
                                    if let Some(ref anchor) = ctx.anchor_date {
                                        let platform = use_platform_effects();
                                        let label = (ctx.messages.range_start_label)(
                                            &format_date_label(anchor, &*ctx.provider, &ctx.locale),
                                            &ctx.locale,
                                        );
                                        platform.announce(&label);
                                    }
                                    no_cleanup()
                                },
                            )))
                    }
                    Some(ref anchor) => {
                        // Second click: complete the range.
                        let anchor = anchor.clone();
                        Some(TransitionPlan::to(State::Focused)
                            .apply(move |ctx| {
                                let range = DateRange::normalized(
                                    anchor,
                                    date.clone(),
                                );
                                ctx.value.set(Some(range));
                                ctx.anchor_date = None;
                                ctx.hovering_date = None;
                                ctx.focused_date = date;
                            })
                            .with_effect(PendingEffect::new(
                                "announce-range-complete",
                                |ctx, _props, _send| {
                                    if let Some(ref range) = ctx.value.get() {
                                        let platform = use_platform_effects();
                                        let label = (ctx.messages.range_complete_label)(
                                            &format_date_label(&range.start, &*ctx.provider, &ctx.locale),
                                            &format_date_label(&range.end, &*ctx.provider, &ctx.locale),
                                            &ctx.locale,
                                        );
                                        platform.announce(&label);
                                    }
                                    no_cleanup()
                                },
                            )))
                    }
                }
            }

            // ── Hover preview ────────────────────────────────────────────
            Event::HoverDate { date } => {
                if ctx.anchor_date.is_none() { return None; }
                let date = date.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.hovering_date = Some(date);
                }))
            }

            Event::HoverEnd => {
                if ctx.hovering_date.is_none() { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.hovering_date = None;
                }))
            }

            // ── Month / year navigation ──────────────────────────────────
            // Navigation logic is identical to Calendar (see calendar.md §1.7).
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
                        format!("{} \u{2013} {} {}", first, month_long_name(&*ctx.provider, lm, &ctx.locale), ly)
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
                        format!("{} \u{2013} {} {}", first, month_long_name(&*ctx.provider, lm, &ctx.locale), ly)
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
    /// Identical to Calendar's keyboard handling (see calendar.md §1.7).
    fn handle_keydown(
        state: &State,
        data: &KeyboardEventData,
        ctx: &Context,
    ) -> Option<TransitionPlan<Self>> {
        if *state != State::Focused { return None; }

        let focused = ctx.focused_date.clone();
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
                let wd = focused.weekday() as u8;
                let start = ctx.first_day_of_week as u8;
                let offset = ((wd + 7 - start) % 7) as i32;
                Some(focused.add_days(-offset))
            }
            KeyboardKey::End => {
                let wd = focused.weekday() as u8;
                let start = ctx.first_day_of_week as u8;
                let offset = ((wd + 7 - start) % 7) as i32;
                Some(focused.add_days(6 - offset))
            }
            KeyboardKey::PageUp  => Some(focused.add_months(-1)),
            KeyboardKey::PageDown => Some(focused.add_months(1)),
            _ => None,
        };

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

### 1.9 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "range-calendar"]
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

/// API for the RangeCalendar component.
pub struct Api<'a> {
    state: &'a State,
    ctx:   &'a Context,
    props: &'a Props,
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
        if self.ctx.is_range_pending() {
            attrs.set_bool(HtmlAttr::Data("ars-range-pending"), true);
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

    /// Attributes for the previous trigger button.
    pub fn prev_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::PrevTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("prev-trigger"));
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

    /// Attributes for the next trigger button.
    pub fn next_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::NextTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("next-trigger"));
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
    pub fn heading_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Heading.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("heading"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
        attrs
    }

    /// Formatted heading text: e.g., "January 2024" or "January -- February 2024".
    pub fn heading_text(&self) -> String {
        if self.ctx.visible_months <= 1 {
            let month_name = month_long_name(&*self.ctx.provider, self.ctx.visible_month, &self.ctx.locale);
            return format!("{} {}", month_name, self.ctx.visible_year);
        }
        let first = format!(
            "{} {}",
            month_long_name(&*self.ctx.provider, self.ctx.visible_month, &self.ctx.locale),
            self.ctx.visible_year,
        );
        let (lm, ly) = self.ctx.month_year_at_offset(self.ctx.visible_months - 1);
        format!("{} \u{2013} {} {}", first, month_long_name(&*self.ctx.provider, lm, &self.ctx.locale), ly)
    }

    /// Attributes for the grid group container (role="group").
    /// Only rendered when visible_months > 1.
    pub fn grid_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::GridGroup.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("grid-group"));
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), self.heading_text());
        attrs
    }

    /// Attributes for a grid element (table).
    pub fn grid_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Grid.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("grid"));
        attrs.set(HtmlAttr::Role, "grid");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("heading"));
        attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        attrs
    }

    /// Attributes for the head row element.
    pub fn head_row_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HeadRow.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Attributes for a head cell (weekday column header).
    pub fn head_cell_attrs(&self, weekday: Weekday) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::HeadCell { day: weekday }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Scope, "col");
        attrs.set(HtmlAttr::Abbr, weekday.long_label(&*self.ctx.provider, &self.ctx.locale));
        attrs
    }

    /// Attributes for a row element.
    pub fn row_attrs(&self, week_index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Row { week_index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-week-index"), week_index.to_string());
        attrs
    }

    /// Attributes for a cell element (gridcell).
    pub fn cell_attrs(&self, date: &CalendarDate) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Cell { date: date.clone() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "gridcell");

        if self.ctx.is_range_start(date) || self.ctx.is_range_end(date) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Selected), "true");
        }
        if self.ctx.is_outside_visible_month(date) {
            attrs.set_bool(HtmlAttr::Data("ars-outside-month"), true);
        }
        if self.ctx.is_date_disabled(date) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Attributes for a cell trigger element (the interactive button inside each cell).
    /// Includes range-specific data attributes.
    pub fn cell_trigger_attrs(&self, date: &CalendarDate) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::CellTrigger { date: date.clone() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        let disabled = self.ctx.is_date_disabled(date);
        let unavailable = self.ctx.is_date_unavailable(date);
        let is_focused = self.ctx.focused_date == *date;
        let is_today = *date == self.props.today;

        // tabindex: only the focused cell is tabbable.
        attrs.set(HtmlAttr::TabIndex, if is_focused { "0" } else { "-1" });

        if disabled || unavailable {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        if self.ctx.is_range_start(date) || self.ctx.is_range_end(date) {
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

        // ── Range-specific data attributes ───────────────────────────
        if self.ctx.is_range_start(date) {
            attrs.set_bool(HtmlAttr::Data("ars-range-start"), true);
        }
        if self.ctx.is_range_end(date) {
            attrs.set_bool(HtmlAttr::Data("ars-range-end"), true);
        }
        if self.ctx.is_in_range(date) {
            attrs.set_bool(HtmlAttr::Data("ars-in-range"), true);
        }
        if self.ctx.is_in_hover_range(date) {
            attrs.set_bool(HtmlAttr::Data("ars-in-hover-range"), true);
        }
        if self.ctx.is_anchor(date) {
            attrs.set_bool(HtmlAttr::Data("ars-anchor"), true);
        }

        // aria-label: full date string for screen readers.
        let base_label = format_date_label(date, &*self.ctx.provider, &self.ctx.locale);
        let label = if unavailable {
            format!("{} (unavailable)", base_label)
        } else if disabled {
            format!("{} (disabled)", base_label)
        } else if self.ctx.is_range_start(date) {
            format!("{} (range start)", base_label)
        } else if self.ctx.is_range_end(date) {
            format!("{} (range end)", base_label)
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

    // ── Multi-month API ──────────────────────────────────────────────────

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
        let grid_id = self.ctx.ids.item("grid", offset);
        let heading_id = self.ctx.ids.item("heading", offset);
        attrs.set(HtmlAttr::Id, grid_id);
        attrs.set(HtmlAttr::Role, "grid");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Grid.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), heading_id);
        attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
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
    pub fn heading_attrs_for(&self, offset: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.item("heading", &offset));
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
        if self.ctx.is_outside_month_at_offset(date, offset) {
            attrs.set_bool(HtmlAttr::Data("ars-outside-month"), true);
        }
        attrs
    }

    // ── Computed state accessors ─────────────────────────────────────────

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
    pub fn is_next_disabled(&self) -> bool {
        if let Some(ref max) = self.ctx.max {
            let last_offset = self.ctx.visible_months.saturating_sub(1);
            let (month, year) = self.ctx.month_year_at_offset(last_offset);
            let first_of_last = CalendarDate::new_gregorian(
                year,
                NonZero::new(month).expect("month is 1-based"),
                nzu8(1),
            );
            let last_of_last = CalendarDate::new_gregorian(
                year,
                NonZero::new(month).expect("month is 1-based"),
                NonZero::new(first_of_last.days_in_month()).expect("days_in_month >= 1"),
            );
            last_of_last >= *max
        } else {
            false
        }
    }

    fn state_name(&self) -> &'static str {
        match self.state {
            State::Idle    => "idle",
            State::Focused => "focused",
        }
    }
}

/// Format a date as a human-readable label for screen readers.
fn format_date_label(date: &CalendarDate, provider: &dyn IcuProvider, locale: &Locale) -> String {
    format!(
        "{} {} {}, {}",
        month_long_name(provider, date.month.get(), locale),
        date.day.get(),
        date.year,
        date.weekday().long_label(provider, locale),
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
            Part::Cell { ref date } => self.cell_attrs(date),
            Part::CellTrigger { ref date } => self.cell_trigger_attrs(date),
        }
    }
}
```

## 2. Anatomy

```text
RangeCalendar (en-US, January--February 2024, visible_months=2)
└── Root                                  data-ars-scope="range-calendar"
    ├── Header                            data-ars-part="header"
    │   ├── PrevTrigger                   aria-label="Previous 2 months"
    │   ├── Heading                       "January -- February 2024"  aria-live="polite"
    │   └── NextTrigger                   aria-label="Next 2 months"
    └── GridGroup                         role="group"  aria-label="January -- February 2024"
        ├── Grid[0]                       role="grid"  aria-multiselectable="true"
        │   ├── Heading[0]               "January 2024"  (visually hidden)
        │   ├── HeadRow
        │   │   ├── HeadCell "Su"         scope="col"  abbr="Sunday"
        │   │   ├── … (7 cells)
        │   │   └── HeadCell "Sa"
        │   ├── Row (week 1)
        │   │   ├── Cell > CellTrigger "5"   data-ars-range-start
        │   │   ├── Cell > CellTrigger "6"   data-ars-in-range
        │   │   ├── …
        │   │   └── Cell > CellTrigger "11"
        │   └── Row (week 2–6) …
        └── Grid[1]                       role="grid"  aria-multiselectable="true"
            ├── Heading[1]               "February 2024"  (visually hidden)
            ├── HeadRow … HeadCell x7
            └── Row x6
                └── Cell > CellTrigger "12"  data-ars-range-end
```

When `visible_months == 1`, the `GridGroup` wrapper is omitted and a single `Grid` is rendered directly under `Root`.

| Part          | HTML Element | Key Attributes                                                                                                  |
| ------------- | ------------ | --------------------------------------------------------------------------------------------------------------- |
| `Root`        | `<div>`      | `data-ars-scope="range-calendar"`, `data-ars-part="root"`, `data-ars-state`, `data-ars-range-pending`           |
| `Header`      | `<div>`      | `data-ars-part="header"`                                                                                        |
| `PrevTrigger` | `<button>`   | `aria-label`, `tabindex="-1"`                                                                                   |
| `NextTrigger` | `<button>`   | `aria-label`, `tabindex="-1"`                                                                                   |
| `Heading`     | `<span>`     | `aria-live="polite"`, `aria-atomic="true"`                                                                      |
| `GridGroup`   | `<div>`      | `role="group"`, `aria-label` (only when `visible_months > 1`)                                                   |
| `Grid`        | `<table>`    | `role="grid"`, `aria-labelledby`, `aria-multiselectable="true"`                                                 |
| `HeadRow`     | `<tr>`       | --                                                                                                              |
| `HeadCell`    | `<th>`       | `scope="col"`, `abbr`                                                                                           |
| `Row`         | `<tr>`       | --                                                                                                              |
| `Cell`        | `<td>`       | `role="gridcell"`, `aria-selected`                                                                              |
| `CellTrigger` | `<button>`   | `tabindex`, `aria-label`, `aria-selected`, `aria-disabled`, `data-ars-today`, range data attributes (see below) |

**Range-specific data attributes on CellTrigger:**

| Attribute                 | Value | When                                                          |
| ------------------------- | ----- | ------------------------------------------------------------- |
| `data-ars-range-start`    | `""`  | The cell is the range start date                              |
| `data-ars-range-end`      | `""`  | The cell is the range end date                                |
| `data-ars-in-range`       | `""`  | The cell falls within the confirmed range (inclusive)         |
| `data-ars-in-hover-range` | `""`  | The cell falls within the hover preview range (pending state) |
| `data-ars-anchor`         | `""`  | The cell is the anchor date (first click of pending range)    |

These attributes enable CSS styling for range backgrounds, rounded endpoints, and hover preview effects.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Element              | Role/Attribute          | Details                                                                                     |
| -------------------- | ----------------------- | ------------------------------------------------------------------------------------------- |
| `Grid`               | `role="grid"`           | `aria-labelledby` points to `Heading`; `aria-multiselectable="true"` always set             |
| `HeadCell`           | `<th scope="col">`      | `abbr` attribute holds full weekday name                                                    |
| `Cell`               | `role="gridcell"`       | `aria-selected` on range start/end dates                                                    |
| `CellTrigger`        | `<button>`              | `aria-label` = full date string with range position suffix; `aria-disabled` when restricted |
| `Heading`            | --                      | `aria-live="polite"` + `aria-atomic="true"` for month changes                               |
| `PrevTrigger`        | `<button>`              | `aria-label="Previous month"` (or `"Previous N months"` for multi-step)                     |
| `NextTrigger`        | `<button>`              | `aria-label="Next month"` (or `"Next N months"` for multi-step)                             |
| `GridGroup`          | `role="group"`          | Wraps multiple grids; `aria-label` = month range. Not rendered when `visible_months == 1`   |
| `Grid[n]` (per-grid) | `role="grid"`           | `aria-labelledby` -> per-grid heading                                                       |
| Per-grid `Heading`   | `id="{id}-heading-{n}"` | Visually hidden per-grid heading. No `aria-live`.                                           |

The grid always has `aria-multiselectable="true"` because range selection inherently involves two selected dates (start and end).

### 3.2 Keyboard Interaction

| Key               | Action                                                            |
| ----------------- | ----------------------------------------------------------------- |
| `ArrowLeft`       | Move focus to previous day (next day in RTL)                      |
| `ArrowRight`      | Move focus to next day (previous day in RTL)                      |
| `ArrowUp`         | Move focus to same day in previous week                           |
| `ArrowDown`       | Move focus to same day in next week                               |
| `Home`            | Move focus to start of current week                               |
| `End`             | Move focus to end of current week                                 |
| `PageUp`          | Move focus to same day in previous month                          |
| `PageDown`        | Move focus to same day in next month                              |
| `Shift+PageUp`    | Move focus to same day in previous year                           |
| `Shift+PageDown`  | Move focus to same day in next year                               |
| `Enter` / `Space` | Select the focused date (first or second click in range workflow) |

Keyboard navigation is identical to Calendar. The two-click range selection model applies equally to pointer and keyboard: Enter/Space on a focused date triggers `SelectDate`.

### 3.3 Focus Management

- Only one cell in the grid has `tabindex="0"` at a time (the focused date); all others have `tabindex="-1"`.
- Arrow keys move the focused cell using a roving tabindex pattern.
- When focus moves to a date outside the visible month range, the view auto-scrolls.
- PrevTrigger and NextTrigger have `tabindex="-1"` and are activated by click only.
- When `visible_months > 1` and arrow-key navigation crosses a month boundary within the visible range, the view does not scroll. Only when focus leaves the entire visible range does the calendar paginate.

### 3.4 Screen Reader Announcements

- **Month navigation:** The Heading element has `aria-live="polite"`, so screen readers announce the new month(s) when navigation occurs.
- **Multi-month heading:** When `visible_months > 1`, navigation announces the full range (e.g., "January -- February 2024"). Per-grid headings do NOT have `aria-live` to avoid duplicate announcements.
- **Range start (first click):** An `aria-live` region announces the selected start date and prompts the user to select an end date (e.g., "Selected March 5, 2026 as range start. Select an end date."). The message text is sourced from `Messages.range_start_label`.
- **Range complete (second click):** An `aria-live` region announces the full range (e.g., "Selected March 5 to March 12, 2026"). The message text is sourced from `Messages.range_complete_label`.
- **Cell labels:** Each CellTrigger's `aria-label` includes range position context when applicable: "(range start)" or "(range end)" suffix.

## 4. Internationalization

### 4.1 Messages

```rust
/// Locale-specific labels for the RangeCalendar component.
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
    /// Announcement when the first date is selected (range start).
    /// The `&str` parameter is the formatted start date label.
    pub range_start_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
    /// Announcement when the range is completed.
    /// The two `&str` parameters are the formatted start and end date labels.
    pub range_complete_label: MessageFn<dyn Fn(&str, &str, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            prev_month_label: MessageFn::static_str("Previous month"),
            next_month_label: MessageFn::static_str("Next month"),
            prev_page_label: MessageFn::new(|count, _locale| {
                format!("Previous {} months", count)
            }),
            next_page_label: MessageFn::new(|count, _locale| {
                format!("Next {} months", count)
            }),
            range_start_label: MessageFn::new(|start_date, _locale| {
                format!("Selected {} as range start. Select an end date.", start_date)
            }),
            range_complete_label: MessageFn::new(|start_date, end_date, _locale| {
                format!("Selected {} to {}", start_date, end_date)
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

Month names, weekday labels, and calendar system formatting are inherited from the locale via ICU4X, using the same resolution strategy as Calendar (see `calendar.md` section 4.1). Range formatting uses `DateRange::normalized()` to ensure the announced start date always precedes the end date regardless of click order.

In RTL locales, the calendar grid is mirrored horizontally (week starts from the right) and arrow key semantics are reversed, identical to Calendar's RTL behavior.

## 5. Library Parity

> Compared against: React Aria (`RangeCalendar`).

### 5.1 Props

| Feature                    | ars-ui                                  | React Aria                  | Notes                                        |
| -------------------------- | --------------------------------------- | --------------------------- | -------------------------------------------- |
| Controlled range           | `value` (`Bindable<Option<DateRange>>`) | `value` (`RangeValue`)      | Equivalent                                   |
| Default range              | `default_value`                         | `defaultValue`              | Equivalent                                   |
| Min/max                    | `min`, `max`                            | `minValue`, `maxValue`      | Equivalent                                   |
| Unavailable predicate      | `is_date_unavailable`                   | `isDateUnavailable`         | Equivalent                                   |
| Disabled                   | `disabled`                              | `isDisabled`                | Equivalent                                   |
| Read-only                  | `readonly`                              | `isReadOnly`                | Equivalent                                   |
| First day of week          | `first_day_of_week`                     | `firstDayOfWeek`            | Equivalent                                   |
| Visible months             | `visible_months`                        | `visibleMonths`             | Equivalent; ars-ui defaults to 2             |
| Page behavior              | `page_behavior`                         | `pageBehavior`              | Equivalent                                   |
| Allows non-contiguous      | --                                      | `allowsNonContiguousRanges` | React Aria allows unavailable dates in range |
| Selection alignment        | --                                      | `selectionAlignment`        | React Aria start/end/center alignment        |
| Focused value (controlled) | --                                      | `focusedValue`              | ars-ui uses internal focused_date            |
| Default focused value      | --                                      | `defaultFocusedValue`       | ars-ui derives from value/today              |
| Auto-focus                 | --                                      | `autoFocus`                 | Adapter concern                              |
| Invalid                    | --                                      | `isInvalid`                 | RangeCalendar has no form validation         |

**Gaps:**

- `allowsNonContiguousRanges`: Niche feature that allows ranges spanning unavailable dates. Not adopting; ars-ui prevents selecting unavailable dates which is the safer default.
- `selectionAlignment`: Controls which month is shown when a range is selected. Useful UX but can be implemented by consumers via `SetMonth`/`SetYear` events. Not adopting.

### 5.2 Anatomy

| Part         | ars-ui        | React Aria             | Notes                          |
| ------------ | ------------- | ---------------------- | ------------------------------ |
| Root         | `Root`        | `RangeCalendar`        | Equivalent                     |
| Header       | `Header`      | --                     | ars-ui groups nav controls     |
| Prev trigger | `PrevTrigger` | `Button` (prev)        | Equivalent                     |
| Next trigger | `NextTrigger` | `Button` (next)        | Equivalent                     |
| Heading      | `Heading`     | `Heading`              | Equivalent                     |
| Grid         | `Grid`        | `CalendarGrid`         | Equivalent                     |
| Grid group   | `GridGroup`   | --                     | ars-ui wraps multi-month grids |
| Head row     | `HeadRow`     | `CalendarGridHeader`   | Equivalent                     |
| Head cell    | `HeadCell`    | `CalendarHeaderCell`   | Equivalent                     |
| Row          | `Row`         | --                     | Implicit in React Aria         |
| Cell         | `Cell`        | `CalendarCell` (outer) | Equivalent                     |
| Cell trigger | `CellTrigger` | `CalendarCell` (inner) | ars-ui splits cell/trigger     |

**Gaps:** None.

### 5.3 Events

| Callback     | ars-ui                   | React Aria      | Notes      |
| ------------ | ------------------------ | --------------- | ---------- |
| Range change | `SelectDate` (two-click) | `onChange`      | Equivalent |
| Focus change | `FocusDate`              | `onFocusChange` | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature                   | ars-ui          | React Aria             |
| ------------------------- | --------------- | ---------------------- |
| Two-click range selection | Yes             | Yes                    |
| Hover preview             | Yes             | Yes (via render props) |
| Range normalization       | Yes             | Yes                    |
| Multi-month display       | Yes (default 2) | Yes                    |
| Min/max constraints       | Yes             | Yes                    |
| Unavailable dates         | Yes             | Yes                    |
| Non-contiguous ranges     | No              | Yes                    |
| Selection alignment       | No              | Yes                    |
| Keyboard range selection  | Yes             | Yes                    |
| RTL support               | Yes             | Yes                    |
| Week numbers              | Yes             | No                     |

**Gaps:** None worth adopting.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria supports `allowsNonContiguousRanges` and `selectionAlignment`. ars-ui adds week numbers and an explicit hover preview model with `HoverDate`/`HoverEnd` events.
- **Recommended additions:** None.
