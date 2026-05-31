//! `DatePicker` component state machine and connect API.
//!
//! Framework-agnostic implementation of the composite date picker defined in
//! `spec/components/date-time/date-picker.md`. The machine pairs a date text
//! input with an embedded [`Calendar`](super::calendar) shown in a popover and
//! owns the popover open/close lifecycle, the selected value, input
//! parsing/validation, the value↔calendar synchronisation point, and the
//! ARIA/`data-ars-*` attribute surface.
//!
//! ## Spec-vs-implementation reconciliation
//!
//! The spec's §1.7/§1.8 code examples predate the current core engine and
//! sibling components. This implementation keeps the spec's semantics while
//! using the real APIs:
//!
//! - **Named effects, not platform calls.** Focus is adapter-driven. The
//!   machine declares a typed [`Effect`] enum and emits
//!   [`PendingEffect::named`](ars_core::PendingEffect::named) intents
//!   ([`Effect::FocusCalendar`], [`Effect::RestoreFocusToTrigger`],
//!   [`Effect::RestoreFocusToInput`]); the adapter performs the DOM focus from
//!   live element handles. This matches `popover`/`dialog` and the issue's
//!   element/ref handling note. The spec's `use_platform_effects()` /
//!   `focus_element_by_id` closures are not used in core.
//! - **Real [`CalendarDate`] API.** Year/month/day are `year()`/`month()`/
//!   `day()` methods returning plain integers, `new_gregorian` is fallible, and
//!   `CalendarDate` has no `Ord` — range checks use
//!   [`CalendarDate::compare`].
//! - **Forwarded predicate type.** `is_date_unavailable` reuses
//!   [`calendar::IsDateUnavailableFn`] so it forwards into [`Api::calendar_props`].
//! - **Controlled-prop sync.** A [`Event::SyncProps`] event plus
//!   [`Machine::on_props_changed`](ars_core::Machine::on_props_changed) and
//!   [`Machine::initial_effects`](ars_core::Machine::initial_effects) keep
//!   controlled `value`/`open`/`min`/`max`/`disabled` live, matching
//!   Calendar/DateField/Popover.
//! - **Format-aware parse/format.** [`format_date`]/[`parse_date`] honour the
//!   resolved `format` pattern (field order + separator) instead of a hardcoded
//!   `MM/dd/yyyy` placeholder.

#[cfg(test)]
mod tests;

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString as _},
    vec,
    vec::Vec,
};
use core::{
    cmp::Ordering,
    fmt::{self, Debug, Write as _},
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Env, HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan,
};
use ars_i18n::{CalendarDate, DateOrder, date_order};
use ars_interactions::{KeyboardEventData, KeyboardKey};

use super::calendar;

// ────────────────────────────────────────────────────────────────────
// State / Event / Effect
// ────────────────────────────────────────────────────────────────────

/// States for the `DatePicker` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Popover is closed; the input field may or may not have focus.
    Closed,

    /// Popover is open and the calendar is visible.
    Open,
}

/// Events for the `DatePicker` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open the calendar popover.
    Open,

    /// Close the calendar popover.
    Close,

    /// Toggle the popover.
    Toggle,

    /// A date was selected from the embedded calendar.
    SelectDate {
        /// The newly selected date.
        date: CalendarDate,
    },

    /// The text input value changed (user typing in the field).
    InputChange {
        /// The raw text now present in the input.
        value: String,
    },

    /// Focus entered the input field (the date field). With `open_on_click`,
    /// this opens the calendar. Adapters dispatch it from input focus only — not
    /// trigger focus, which the trigger's own click/keydown handlers manage.
    FocusIn,

    /// Focus left the date picker entirely.
    FocusOut,

    /// Keyboard event on the input or trigger.
    KeyDown {
        /// The key that was pressed.
        key: KeyboardKey,
    },

    /// Re-apply context-backed prop fields after a props change.
    ///
    /// Emitted by [`Machine::on_props_changed`](ars_core::Machine::on_props_changed)
    /// so controlled `value`/`open` and the cached `min`/`max`/`disabled`/
    /// `format` fields follow parent-driven prop updates.
    SyncProps(Box<Props>),
}

/// Typed identifier for every named effect intent the `date_picker` machine
/// emits.
///
/// Each variant is a stable identifier the adapter routes on with an
/// exhaustive `match` to perform the live, element-handle-based focus
/// operation the agnostic core cannot perform itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Notify the consumer that the open state changed. Emitted on every
    /// open ↔ closed transition (and on a non-closed initial mount). The
    /// adapter reads the current open state from the connect API and invokes
    /// [`Props::on_open_change`] with it — the channel a controlled-`open`
    /// parent uses to reconcile after a user-driven open/close.
    OpenChange,

    /// Notify the consumer that the selected value changed. Emitted whenever a
    /// selection, accepted typed entry, or clear changes the requested value.
    /// The adapter reads [`Context::requested_value`] (the requested date — not
    /// `value.get()`, which a controlled bindable holds at the parent's value)
    /// and forwards it, so a controlled-`value` parent learns which date to feed
    /// back. Mirrors the `requested_value` + `ValueChange` convention used by
    /// other value-bearing components (e.g. `rating_group`).
    ValueChange,

    /// Move focus into the embedded calendar (its grid, falling back to the
    /// content container). Emitted when the popover opens.
    FocusCalendar,

    /// Return focus to the trigger button. Emitted when the popover closes via
    /// [`Event::Close`] or an Escape key press.
    RestoreFocusToTrigger,

    /// Return focus to the text input. Emitted when a calendar selection closes
    /// the popover (`close_on_select`).
    RestoreFocusToInput,
}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Props for the `DatePicker` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// The ID of the date picker. The adapter supplies a hydration-stable base;
    /// [`ComponentIds`] derives part IDs from it.
    pub id: String,

    /// Controlled date value. `None` = uncontrolled, `Some(None)` =
    /// controlled-and-empty, `Some(Some(date))` = controlled with a date.
    pub value: Option<Option<CalendarDate>>,

    /// Default date for uncontrolled mode.
    pub default_value: Option<CalendarDate>,

    /// Minimum selectable date (forwarded to the embedded Calendar).
    pub min: Option<CalendarDate>,

    /// Maximum selectable date (forwarded to the embedded Calendar).
    pub max: Option<CalendarDate>,

    /// Whether the date picker is non-interactive.
    pub disabled: bool,

    /// Whether the date picker allows viewing but not editing.
    pub readonly: bool,

    /// Predicate for unavailable dates (forwarded to the embedded Calendar).
    pub is_date_unavailable: Option<calendar::IsDateUnavailableFn>,

    /// Date format pattern. Defaults to a locale-appropriate format.
    pub format: Option<String>,

    /// Placeholder text for the input field.
    pub placeholder: Option<String>,

    /// Form field name for hidden input submission.
    pub name: Option<String>,

    /// Whether the field is required.
    pub required: bool,

    /// Right-to-left layout direction.
    pub is_rtl: bool,

    /// Label text.
    pub label: String,

    /// Description/help text.
    pub description: Option<String>,

    /// Error message.
    pub error_message: Option<String>,

    /// Whether the field is in an invalid state.
    pub invalid: bool,

    /// Whether to close the popover after a date is selected. Default: `true`.
    pub close_on_select: bool,

    /// Controlled open state. `Some(true)` = forced open, `Some(false)` =
    /// forced closed, `None` = uncontrolled.
    pub open: Option<bool>,

    /// Default open state for uncontrolled mode.
    pub default_open: bool,

    /// When `true` (the default), clicking or focusing the date field opens the
    /// calendar popover. When `false`, only the trigger button opens it.
    pub open_on_click: bool,

    /// Number of months to display side-by-side in the calendar popover.
    /// Default: `1`. Forwarded to the embedded Calendar's `visible_months`.
    pub visible_months: usize,

    /// The "today" date, injected by the adapter for testability and SSR
    /// determinism. Forwarded to the embedded Calendar's `today` so an empty
    /// picker opens on the current month and marks the correct day. Defaults to
    /// a fixed date (matching `calendar::Props::default().today`); adapters
    /// inject the real today.
    pub today: CalendarDate,

    /// Called whenever the open state changes. Fired by the adapter from the
    /// [`Effect::OpenChange`] intent with the new open value. A controlled-`open`
    /// parent uses this to reconcile its state after a user-driven open/close
    /// (mirrors `popover`/`dialog`).
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
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
            format: None,
            placeholder: None,
            name: None,
            required: false,
            is_rtl: false,
            label: String::new(),
            description: None,
            error_message: None,
            invalid: false,
            close_on_select: true,
            open: None,
            default_open: false,
            open_on_click: true,
            visible_months: 1,
            today: CalendarDate::new_gregorian(2025, 1, 1)
                .expect("2025-01-01 is a valid Gregorian date"),
            on_open_change: None,
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// Context for the `DatePicker` component.
#[derive(Clone, Debug)]
pub struct Context {
    /// The selected date value (controlled/uncontrolled).
    pub value: Bindable<Option<CalendarDate>>,

    /// Raw text in the input field.
    pub input_text: String,

    /// Last successfully parsed date from input text.
    pub parsed_date: Option<CalendarDate>,

    /// The most recently requested value, carried to the adapter's
    /// `on_value_change` wiring via [`Effect::ValueChange`].
    ///
    /// This is distinct from [`value`](Self::value): when `value` is controlled,
    /// the bindable's `get()` returns the parent's committed value, so the
    /// requested date (from a selection or accepted typed entry) would otherwise
    /// be invisible to the parent. The adapter reads this field — not
    /// `value.get()` — when an `Effect::ValueChange` fires. `None` represents a
    /// requested clear.
    pub requested_value: Option<CalendarDate>,

    /// Most recently requested open state, carried to the adapter's
    /// `on_open_change` wiring via [`Effect::OpenChange`].
    ///
    /// When `open` is controlled the machine does not mutate [`State`] on user
    /// events — it records the request here and lets the parent reconcile by
    /// updating the `open` prop. The adapter reads this field (not `is_open()`,
    /// which still reflects the committed state) when an `Effect::OpenChange`
    /// fires.
    pub requested_open: bool,

    /// Focus move to perform the next time an open/close lands, recorded per
    /// user event so a controlled-`open` reconciliation (which lands the state
    /// change in `SyncProps`, not at the user event) reproduces the originating
    /// intent. `None` means "no pending override" — `SyncProps` then uses the
    /// per-direction default (calendar on open, trigger on close).
    requested_focus: Option<OpenFocus>,

    /// Locale for formatting and parsing.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Date format pattern (e.g. `"MM/dd/yyyy"`).
    pub format: String,

    /// Minimum selectable date (forwarded to Calendar).
    pub min: Option<CalendarDate>,

    /// Maximum selectable date (forwarded to Calendar).
    pub max: Option<CalendarDate>,

    /// Adapter-injected "today" date, forwarded to the embedded Calendar.
    pub today: CalendarDate,

    /// Disabled state.
    pub disabled: bool,

    /// Read-only state.
    pub readonly: bool,

    /// Whether clicking/focusing the input opens the popover.
    pub open_on_click: bool,

    /// Whether the input has been interacted with.
    pub is_touched: bool,

    /// Form field name.
    pub name: Option<String>,

    /// Whether the field is required.
    pub required: bool,

    /// Right-to-left layout.
    pub is_rtl: bool,

    /// Component IDs.
    pub ids: ComponentIds,
}

impl Context {
    /// Formats the current value as a display string for the input field.
    #[must_use]
    pub fn formatted_value(&self) -> String {
        self.value
            .get()
            .as_ref()
            .map(|date| format_date(date, &self.format))
            .unwrap_or_default()
    }

    /// Attempts to parse a text string into a [`CalendarDate`] using the
    /// resolved format pattern.
    #[must_use]
    pub fn parse_input(&self, text: &str) -> Option<CalendarDate> {
        parse_date(text, &self.format)
    }

    /// Syncs the input text to reflect the current value.
    pub fn sync_input_text(&mut self) {
        self.input_text = self.formatted_value();
    }
}

// ────────────────────────────────────────────────────────────────────
// Date format / parse helpers
// ────────────────────────────────────────────────────────────────────

/// One of the three ordered fields in a date format pattern.
#[derive(Clone, Copy)]
enum DateField {
    Year,
    Month,
    Day,
}

/// Returns the locale-appropriate default date format pattern.
///
/// Field order comes from [`ars_i18n::date_order`], which uses real locale data
/// — CLDR via ICU4X or the browser `Intl` API where a backend is compiled in,
/// and a `(language, region)` heuristic otherwise — so every locale gets the
/// correct month/day/year order (the sibling [`date_field`](super::date_field)
/// uses the same source). The field separator stays a small locale heuristic
/// (`.` for German and Korean, `/` elsewhere), mirroring `date_field`'s literal.
fn default_format_for_locale(locale: &Locale) -> String {
    let separator = match (locale.language(), locale.region()) {
        ("de", Some("DE")) | ("ko", Some("KR")) => '.',
        _ => '/',
    };

    let order: [&str; 3] = match date_order(locale) {
        DateOrder::MonthDayYear => ["MM", "dd", "yyyy"],
        DateOrder::DayMonthYear => ["dd", "MM", "yyyy"],
        DateOrder::YearMonthDay => ["yyyy", "MM", "dd"],
    };

    format!("{}{separator}{}{separator}{}", order[0], order[1], order[2])
}

/// Parses a format pattern into its field separator and ordered fields.
///
/// The separator is the first non-alphabetic character (defaulting to `'/'`);
/// each token's leading letter selects the field (`y`→year, `M`→month,
/// `d`→day). Patterns that do not split into three tokens fall back to
/// month/day/year.
fn parse_format(format: &str) -> (char, [DateField; 3]) {
    let sep = format
        .chars()
        .find(|c| !c.is_ascii_alphabetic())
        .unwrap_or('/');

    let mut order = [DateField::Month, DateField::Day, DateField::Year];

    let tokens = format.split(sep).collect::<Vec<_>>();

    if tokens.len() == 3 {
        for (index, token) in tokens.iter().enumerate() {
            order[index] = match token.chars().next() {
                Some('y' | 'Y') => DateField::Year,
                Some('d' | 'D') => DateField::Day,
                _ => DateField::Month,
            };
        }
    }

    (sep, order)
}

/// Formats a [`CalendarDate`] according to the given pattern.
///
/// Month and day are zero-padded to two digits, year to four, in the field
/// order described by `format`.
fn format_date(date: &CalendarDate, format: &str) -> String {
    let (sep, order) = parse_format(format);

    let mut out = String::new();

    for (index, field) in order.iter().enumerate() {
        if index > 0 {
            out.push(sep);
        }

        match field {
            DateField::Year => {
                let _ = write!(out, "{:04}", date.year());
            }

            DateField::Month => {
                let _ = write!(out, "{:02}", date.month());
            }

            DateField::Day => {
                let _ = write!(out, "{:02}", date.day());
            }
        }
    }

    out
}

/// Parses a date string according to the given pattern.
///
/// Returns `None` when the text does not split into three numeric tokens in the
/// pattern's field order or does not form a valid Gregorian date.
fn parse_date(text: &str, format: &str) -> Option<CalendarDate> {
    let (sep, order) = parse_format(format);

    let tokens = text.split(sep).collect::<Vec<_>>();

    if tokens.len() != 3 {
        return None;
    }

    let (mut year, mut month, mut day) = (None, None, None);

    for (index, field) in order.iter().enumerate() {
        let parsed: i64 = tokens[index].trim().parse().ok()?;

        match field {
            DateField::Year => year = Some(parsed),
            DateField::Month => month = Some(parsed),
            DateField::Day => day = Some(parsed),
        }
    }

    let year = i32::try_from(year?).ok()?;
    let month = u8::try_from(month?).ok()?;
    let day = u8::try_from(day?).ok()?;

    CalendarDate::new_gregorian(year, month, day).ok()
}

/// Classification of input text for [`Event::InputChange`].
enum InputClass {
    /// A complete, valid date.
    Valid(CalendarDate),
    /// A complete numeric entry (three numeric fields) that does not form a
    /// valid date, e.g. `02/30/2024`. Treated as a rejected commit (clears the
    /// value) rather than as in-progress typing.
    CompleteInvalid,
    /// In-progress / non-numeric text the user is still editing.
    Partial,
}

/// Classifies input text as a valid date, a complete-but-invalid date, or
/// in-progress partial text.
///
/// A "complete" entry is exactly three numeric fields in the format's separator;
/// only such entries can be rejected as invalid. Anything else (too few fields,
/// non-numeric) is partial and leaves the committed value untouched.
fn classify_input(text: &str, format: &str) -> InputClass {
    let (separator, _order) = parse_format(format);
    let tokens = text.split(separator).collect::<Vec<_>>();
    let complete_numeric = tokens.len() == 3
        && tokens.iter().all(|token| {
            let token = token.trim();
            !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_digit())
        });

    if !complete_numeric {
        return InputClass::Partial;
    }

    match parse_date(text, format) {
        Some(date) => InputClass::Valid(date),
        None => InputClass::CompleteInvalid,
    }
}

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// `MessageFn` carrying a locale-only label closure.
pub type LocaleLabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// `MessageFn` carrying a formatted-date plus locale label closure.
pub type SelectedDateLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// Locale-specific labels for the `DatePicker` component.
///
/// Calendar navigation labels (prev/next month, today, unavailable) belong to
/// the embedded [`Calendar`](super::calendar)'s messages and are not duplicated
/// here.
#[derive(Clone)]
pub struct Messages {
    /// Trigger button label (default: `"Open calendar"`).
    pub trigger_label: MessageFn<LocaleLabelFn>,

    /// Clear button label (default: `"Clear date"`).
    pub clear_label: MessageFn<LocaleLabelFn>,

    /// Content dialog label (default: `"Choose date"`).
    pub content_label: MessageFn<LocaleLabelFn>,

    /// Announces the selected date (default: `"Selected date: {date}"`). Used
    /// as `aria-description` on the input when a date is selected.
    pub selected_date_label: MessageFn<SelectedDateLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Open calendar"),
            clear_label: MessageFn::static_str("Clear date"),
            content_label: MessageFn::static_str("Choose date"),
            selected_date_label: MessageFn::new(|date: &str, _locale: &Locale| {
                format!("Selected date: {date}")
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
        self.trigger_label == other.trigger_label
            && self.clear_label == other.clear_label
            && self.content_label == other.content_label
            && self.selected_date_label == other.selected_date_label
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Anatomy
// ────────────────────────────────────────────────────────────────────

/// Anatomy parts for the `DatePicker` component.
#[derive(ars_core::ComponentPart)]
#[scope = "date-picker"]
pub enum Part {
    /// Outermost container.
    Root,

    /// Associated label pointing to the input.
    Label,

    /// Wrapper around the input and trigger button.
    Control,

    /// Date text input.
    Input,

    /// Calendar icon button that toggles the popover.
    Trigger,

    /// Button that clears the selected date; hidden when empty.
    ClearTrigger,

    /// Floating positioner for the popover content.
    Positioner,

    /// Popover content (`role="dialog"`) containing the embedded Calendar.
    Content,

    /// Optional help text.
    Description,

    /// Validation error text (`role="alert"`).
    ErrorMessage,

    /// Hidden input carrying the ISO date string for form submission.
    HiddenInput,
}

// ────────────────────────────────────────────────────────────────────
// Effect builders
// ────────────────────────────────────────────────────────────────────

fn open_change_effect() -> PendingEffect<Machine> {
    PendingEffect::named(Effect::OpenChange)
}

fn value_change_effect() -> PendingEffect<Machine> {
    PendingEffect::named(Effect::ValueChange)
}

fn focus_calendar_effect() -> PendingEffect<Machine> {
    PendingEffect::named(Effect::FocusCalendar)
}

fn restore_focus_to_trigger_effect() -> PendingEffect<Machine> {
    PendingEffect::named(Effect::RestoreFocusToTrigger)
}

fn restore_focus_to_input_effect() -> PendingEffect<Machine> {
    PendingEffect::named(Effect::RestoreFocusToInput)
}

/// The focus move that should accompany an open/close once it actually lands.
///
/// Recorded per user event so a controlled-`open` reconciliation (which lands
/// the state change later, in `SyncProps`) can reproduce the *originating*
/// intent instead of always focusing the calendar on open / the trigger on
/// close. Internal: the adapter only ever sees the resulting focus `Effect`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenFocus {
    /// Move focus into the calendar (trigger / `ArrowDown` open).
    Calendar,
    /// Restore focus to the trigger (`Close` / Escape).
    Trigger,
    /// Restore focus to the input (`SelectDate` close).
    Input,
    /// Do not move focus (input-focus open, focus-out close).
    None,
}

impl OpenFocus {
    fn effect(self) -> Option<PendingEffect<Machine>> {
        match self {
            OpenFocus::Calendar => Some(focus_calendar_effect()),
            OpenFocus::Trigger => Some(restore_focus_to_trigger_effect()),
            OpenFocus::Input => Some(restore_focus_to_input_effect()),
            OpenFocus::None => None,
        }
    }
}

/// Builds the plan for a user-driven request to open or close the popover.
///
/// When `open` is **controlled** (`props.open.is_some()`), the [`State`] is not
/// mutated: the request is recorded in [`Context::requested_open`] (and the
/// `focus` intent in `requested_focus`) and signalled via [`Effect::OpenChange`]
/// so the parent reconciles by updating the `open` prop — the focus effect then
/// fires from `SyncProps` once the state actually lands. When **uncontrolled**,
/// the `State` transitions and the `focus` effect fires immediately.
///
/// Returns `None` when the popover is already in the requested open state.
fn open_request(
    state: &State,
    ctx: &Context,
    props: &Props,
    target_open: bool,
    focus: OpenFocus,
) -> Option<TransitionPlan<Machine>> {
    if (*state == State::Open) == target_open {
        // Already in the requested open-state, so there is nothing to
        // reconcile. A controlled open the parent vetoed (left `open`
        // unchanged) leaves a deferred focus intent in `requested_focus` with no
        // `SyncProps` to consume it; a subsequent no-op *close* (e.g. `FocusOut`
        // after the input lost focus) cancels that stale intent so a later
        // programmatic open uses the per-direction default (`FocusCalendar`)
        // instead of suppressing it. (`requested_focus` is only ever set under
        // controlled `open`, so the uncontrolled no-op path stays a pure `None`.)
        if !target_open && ctx.requested_focus.is_some() {
            return Some(TransitionPlan::new().apply(|ctx: &mut Context| {
                ctx.requested_focus = None;
            }));
        }
        return None;
    }

    let controlled = props.open.is_some();
    let mut plan = if controlled {
        TransitionPlan::new()
    } else {
        TransitionPlan::to(if target_open {
            State::Open
        } else {
            State::Closed
        })
    };

    plan = plan
        .apply(move |ctx: &mut Context| {
            ctx.requested_open = target_open;
            if controlled {
                // Defer the focus move to `SyncProps`, which lands the state.
                ctx.requested_focus = Some(focus);
            }
        })
        .with_effect(open_change_effect());

    if !controlled && let Some(focus) = focus.effect() {
        plan = plan.with_effect(focus);
    }

    Some(plan)
}

/// Re-applies the controlled value and cached prop fields onto `ctx`.
///
/// The controlled `value` flows through its [`Bindable`] via
/// [`Bindable::sync_controlled`] (`Some(_)` stays controlled, `None` reveals the
/// uncontrolled internal value), then the cached scalar fields are refreshed.
/// Open/closed is owned by [`State`] (the [`Event::SyncProps`] transition derives
/// the target state from `props.open`), so there is no `open` field to refresh
/// here.
///
/// The input text is re-synced **only when the displayed value or format
/// actually changed** — an unrelated prop change (e.g. `invalid`, `description`,
/// `disabled`) must not clobber a partial/invalid date the user is mid-way
/// through typing.
fn sync_props_into_ctx(ctx: &mut Context, props: &Props) {
    let previous_display = ctx.formatted_value();

    ctx.value.sync_controlled(props.value.clone());

    ctx.min = props.min.clone();
    ctx.max = props.max.clone();
    ctx.today = props.today.clone();
    ctx.disabled = props.disabled;
    ctx.readonly = props.readonly;
    ctx.open_on_click = props.open_on_click;
    ctx.name = props.name.clone();
    ctx.required = props.required;
    ctx.is_rtl = props.is_rtl;
    ctx.format = props
        .format
        .clone()
        .unwrap_or_else(|| default_format_for_locale(&ctx.locale));

    let next_display = ctx.formatted_value();
    if next_display != previous_display {
        ctx.input_text = next_display;
    }
}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// The `DatePicker` state machine.
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

        let locale = env.locale.clone();

        let format = props
            .format
            .clone()
            .unwrap_or_else(|| default_format_for_locale(&locale));

        let input_text = value
            .get()
            .as_ref()
            .map(|date| format_date(date, &format))
            .unwrap_or_default();

        let open = props.open.unwrap_or(props.default_open);
        let initial_state = if open { State::Open } else { State::Closed };

        // Seed the `requested_*` mirrors with the initial state; they are only
        // read by the adapter in response to an `Effect::ValueChange` /
        // `Effect::OpenChange`, neither of which fires at init.
        let requested_value = value.get().clone();
        let requested_open = open;

        let ctx = Context {
            value,
            input_text,
            parsed_date: None,
            requested_value,
            requested_open,
            requested_focus: None,
            locale,
            messages: messages.clone(),
            format,
            min: props.min.clone(),
            max: props.max.clone(),
            today: props.today.clone(),
            disabled: props.disabled,
            readonly: props.readonly,
            open_on_click: props.open_on_click,
            is_touched: false,
            name: props.name.clone(),
            required: props.required,
            is_rtl: props.is_rtl,
            ids: ComponentIds::from_id(&props.id),
        };

        (initial_state, ctx)
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        debug_assert_eq!(
            old.id, new.id,
            "date_picker::Props.id must remain stable after init",
        );

        if old == new {
            Vec::new()
        } else {
            vec![Event::SyncProps(Box::new(new.clone()))]
        }
    }

    fn initial_effects(
        state: &Self::State,
        _context: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        // A picker that boots in `Open` (controlled `open` or `default_open`)
        // moves focus into the calendar. It does NOT emit `OpenChange`: that is
        // a user-interaction signal, and the initial open state is the parent's
        // own configuration (mirrors not firing `onOpenChange` on mount).
        if *state == State::Open {
            vec![focus_calendar_effect()]
        } else {
            Vec::new()
        }
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // `SyncProps` flows through even when disabled so parent-driven changes
        // that take the picker *out of* the disabled state can land.
        if let Event::SyncProps(new_props) = event {
            let new_props = new_props.as_ref().clone();

            let was_open = *state == State::Open;

            let target = new_props
                .open
                .map(|open| if open { State::Open } else { State::Closed });

            let opening = !was_open && target.as_ref() == Some(&State::Open);
            let closing = was_open && target.as_ref() == Some(&State::Closed);

            // The focus intent recorded by the user event that prompted this
            // reconciliation (read before the apply closure consumes it). When a
            // controlled change is purely programmatic (no preceding user event),
            // it is `None` and the per-direction default applies.
            let pending_focus = ctx.requested_focus;

            let mut plan = if let Some(next) = target {
                TransitionPlan::to(next)
            } else {
                TransitionPlan::new()
            };

            plan = plan.apply(move |ctx: &mut Context| {
                if let Some(open) = new_props.open {
                    ctx.requested_open = open;
                }
                // Consume the pending focus intent.
                ctx.requested_focus = None;
                sync_props_into_ctx(ctx, &new_props);
            });

            // No `OpenChange` here: a controlled `open` prop change is the
            // parent's own doing, so re-notifying would double-fire (and loop
            // against a user request that prompted the prop change). Focus
            // reproduces the originating user intent (or the per-direction
            // default for a programmatic change).
            if opening && let Some(focus) = pending_focus.unwrap_or(OpenFocus::Calendar).effect() {
                plan = plan.with_effect(focus);
            } else if closing
                && let Some(focus) = pending_focus.unwrap_or(OpenFocus::Trigger).effect()
            {
                plan = plan.with_effect(focus);
            }

            return Some(plan);
        }

        if ctx.disabled {
            return None;
        }

        match event {
            // Trigger / `ArrowDown` open: when uncontrolled this opens and moves
            // focus into the calendar; when controlled it only requests the open
            // (see `open_request`). `readonly` blocks opening entirely.
            Event::Open => {
                if ctx.readonly {
                    return None;
                }
                open_request(state, ctx, props, true, OpenFocus::Calendar)
            }

            Event::Close => open_request(state, ctx, props, false, OpenFocus::Trigger),

            Event::Toggle => match state {
                State::Closed => Self::transition(state, &Event::Open, ctx, props),
                State::Open => Self::transition(state, &Event::Close, ctx, props),
            },

            Event::SelectDate { date } => {
                if ctx.readonly {
                    return None;
                }

                // Defense-in-depth: reject a selection the picker disallows. The
                // embedded calendar should never offer an out-of-range or
                // unavailable date, but a scripted/stale/buggy `SelectDate` must
                // not be able to commit a value the component forbids (the typed
                // path enforces the same constraints).
                let in_range = ctx
                    .min
                    .as_ref()
                    .is_none_or(|m| date.compare(m) != Ordering::Less)
                    && ctx
                        .max
                        .as_ref()
                        .is_none_or(|m| date.compare(m) != Ordering::Greater);
                let available = props
                    .is_date_unavailable
                    .as_ref()
                    .is_none_or(|predicate| !predicate(date));
                if !(in_range && available) {
                    return None;
                }

                let date = date.clone();
                let should_close = props.close_on_select;
                let open_controlled = props.open.is_some();
                // Only commit the close when open is uncontrolled; a controlled
                // picker requests the close and lets the parent reconcile.
                let commit_close = should_close && !open_controlled;
                // Suppress a no-op value-change notification when the selected
                // date already matches the committed value.
                let value_changes = ctx.value.get().as_ref() != Some(&date);

                // When `open` is controlled, never mutate `State` here — the
                // parent owns it (a stale/scripted selection must not locally
                // reopen or move the picker). Uncontrolled: close on select, or
                // *preserve the current state* when `close_on_select` is false —
                // a selection must never reopen an already-closed picker (a
                // queued/stale calendar click that races with an outside-close
                // or focus-out must not bring the popover back).
                let next_state = if !open_controlled && should_close {
                    State::Closed
                } else {
                    state.clone()
                };

                let mut plan = TransitionPlan::to(next_state).apply(move |ctx: &mut Context| {
                    // Record the requested date so the adapter can forward it
                    // even when `value` is controlled (where `get()` returns the
                    // parent's value).
                    ctx.requested_value = Some(date.clone());
                    ctx.value.set(Some(date));
                    // Derive the visible text and `parsed_date` from the value
                    // the bindable actually exposes — for a controlled `value`
                    // that is still the parent's value, so the display never
                    // optimistically diverges from `selected_date()` / the
                    // hidden input / the composed calendar props.
                    ctx.parsed_date = ctx.value.get().clone();
                    ctx.input_text = ctx.formatted_value();
                    ctx.is_touched = true;
                    if should_close {
                        ctx.requested_open = false;
                        if !commit_close {
                            // Controlled close: defer the focus restore to the
                            // input until the parent echoes the close.
                            ctx.requested_focus = Some(OpenFocus::Input);
                        }
                    }
                });

                if value_changes {
                    plan = plan.with_effect(value_change_effect());
                }

                if should_close {
                    plan = plan.with_effect(open_change_effect());
                    if commit_close {
                        plan = plan.with_effect(restore_focus_to_input_effect());
                    }
                }

                Some(plan)
            }

            Event::InputChange { value } => {
                if ctx.readonly {
                    return None;
                }

                let text = value.clone();

                // Decide the committed-value outcome at plan-build time so the
                // `ValueChange` effect is emitted exactly when the value changes.
                //
                // - A complete, valid date in range and available commits.
                // - A complete date that is rejected (out of range, unavailable,
                //   or not a real date such as `02/30/2024`) clears the value, so
                //   the hidden input / calendar never submit a stale date that
                //   contradicts the visible field.
                // - Empty text clears.
                // - Partial / non-numeric text leaves the committed value
                //   untouched (the user is still typing).
                let (committed_value, value_changed): (Option<CalendarDate>, bool) =
                    match classify_input(&text, &ctx.format) {
                        InputClass::Valid(date) => {
                            let in_range = ctx
                                .min
                                .as_ref()
                                .is_none_or(|m| date.compare(m) != Ordering::Less)
                                && ctx
                                    .max
                                    .as_ref()
                                    .is_none_or(|m| date.compare(m) != Ordering::Greater);
                            let available = props
                                .is_date_unavailable
                                .as_ref()
                                .is_none_or(|predicate| !predicate(&date));
                            if in_range && available {
                                (Some(date), true)
                            } else {
                                (None, true)
                            }
                        }
                        InputClass::CompleteInvalid => (None, true),
                        InputClass::Partial if text.is_empty() => (None, true),
                        InputClass::Partial => (None, false),
                    };

                // Suppress a no-op value-change notification when the committed
                // value would not actually change.
                let value_changes =
                    value_changed && committed_value.as_ref() != ctx.value.get().as_ref();
                // When `value` is controlled, `value.set` only records a pending
                // request — the field must reflect the bindable (the parent's
                // committed value) for accepts *and* clears/rejections, never
                // optimistically showing typed/empty text the rest of the API
                // contradicts.
                let value_controlled = props.value.is_some();

                let mut plan = TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.is_touched = true;
                    if value_changed {
                        let accepted = committed_value.is_some();
                        ctx.parsed_date = committed_value.clone();
                        ctx.requested_value = committed_value.clone();
                        ctx.value.set(committed_value);
                        ctx.input_text = if value_controlled || accepted {
                            // Reflect the bindable's value: for a controlled
                            // `value` the parent hasn't echoed yet (any change), or
                            // an uncontrolled accept (normalized), keep the visible
                            // field from diverging from `selected_date()` / the
                            // hidden input / calendar props.
                            ctx.formatted_value()
                        } else {
                            // Uncontrolled rejected complete date or explicit
                            // clear: keep what the user sees in the field.
                            text
                        };
                    } else {
                        // In-progress (partial/unparseable) typing is preserved.
                        ctx.input_text = text;
                    }
                });

                if value_changes {
                    plan = plan.with_effect(value_change_effect());
                }

                Some(plan)
            }

            Event::FocusIn => {
                // `readonly` blocks opening through the focus path just as it
                // blocks the explicit `Open` event. Open via focus does NOT move
                // focus into the calendar (the user focused the input to type),
                // so no focus effect is passed — only explicit trigger/ArrowDown
                // opens move focus into the grid.
                if *state == State::Closed && ctx.open_on_click && !ctx.readonly {
                    open_request(state, ctx, props, true, OpenFocus::None)
                } else {
                    None
                }
            }

            Event::FocusOut => open_request(state, ctx, props, false, OpenFocus::None),

            Event::KeyDown { key } => match key {
                KeyboardKey::Escape if *state == State::Open => {
                    Self::transition(state, &Event::Close, ctx, props)
                }

                KeyboardKey::ArrowDown if *state == State::Closed => {
                    Self::transition(state, &Event::Open, ctx, props)
                }

                // Already open (e.g. opened by input focus, which keeps focus in
                // the input): ArrowDown moves focus into the calendar grid so the
                // documented input→calendar keyboard path works in the focus-open
                // flow. No state change.
                KeyboardKey::ArrowDown => {
                    Some(TransitionPlan::new().with_effect(focus_calendar_effect()))
                }

                _ => None,
            },

            // Handled above the disabled guard.
            Event::SyncProps(_) => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Connect API
// ────────────────────────────────────────────────────────────────────

/// Connect API for the `DatePicker` component.
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
    // ── AttrMap getters ───────────────────────────────────────────────

    /// Returns the root attributes.
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

        if self.ctx.is_rtl {
            attrs.set(HtmlAttr::Dir, "rtl");
        }

        attrs
    }

    /// Returns the label attributes.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("label"))
            .set(HtmlAttr::For, self.ctx.ids.part("input"));

        attrs
    }

    /// Returns the control attributes.
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("control"));

        attrs
    }

    /// Returns the input attributes.
    #[must_use]
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("input"))
            .set(HtmlAttr::Type, "text")
            .set(HtmlAttr::Value, self.ctx.input_text.clone())
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.is_open() { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.part("content"),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        // Announce the selected date from the live value so the description is
        // present for `default_value` and controlled values too — not only for
        // dates typed or picked during this session (`parsed_date`).
        if let Some(date) = self.ctx.value.get() {
            let formatted = format_date(date, &self.ctx.format);

            attrs.set(
                HtmlAttr::Aria(AriaAttr::Description),
                (self.ctx.messages.selected_date_label)(&formatted, &self.ctx.locale),
            );
        }

        let mut describedby = Vec::new();

        if self.props.description.is_some() {
            describedby.push(self.ctx.ids.part("description"));
        }

        if self.props.invalid {
            describedby.push(self.ctx.ids.part("error-message"));
        }

        if !describedby.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby.join(" "));
        }

        if let Some(placeholder) = &self.props.placeholder {
            attrs.set(HtmlAttr::Placeholder, placeholder.clone());
        }

        if self.ctx.disabled {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs
                .set_bool(HtmlAttr::ReadOnly, true)
                .set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.ctx.required {
            // Native `required` on the visible control so the browser enforces
            // constraint validation (ARIA alone does not), matching the
            // text-like inputs in this crate. The hidden input is `type=hidden`
            // and cannot participate in validation.
            attrs.set_bool(HtmlAttr::Required, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.props.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        attrs
    }

    /// Returns the trigger button attributes.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("trigger"))
            // Explicit `type="button"` so activating the trigger never submits a
            // surrounding form (the HTML default button type is `submit`).
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
            )
            .set(HtmlAttr::TabIndex, "0");

        // `readonly` blocks every opening path, so the trigger advertises that
        // it is non-actionable (disabled + aria-disabled) rather than rendering
        // as an enabled button that does nothing — matching the input and
        // clear-trigger, which already expose the readonly/disabled state.
        if self.ctx.disabled || self.ctx.readonly {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Returns the clear-trigger button attributes.
    #[must_use]
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("clear-trigger"))
            // Explicit `type="button"` so clearing never submits a surrounding
            // form (the HTML default button type is `submit`).
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.clear_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::TabIndex, "-1");

        if self.ctx.disabled || self.ctx.readonly {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.value.get().is_none() {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Returns the positioner attributes.
    #[must_use]
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("positioner"));

        if !self.is_open() {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Returns the content attributes.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("content"))
            .set(HtmlAttr::Role, "dialog")
            .set(HtmlAttr::Data("ars-state"), self.state_name())
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.content_label)(&self.ctx.locale),
            );

        attrs
    }

    /// Returns the description attributes.
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

    /// Returns the error-message attributes.
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

    /// Returns the hidden input attributes (ISO 8601 value for form submission).
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

        // A disabled picker must not submit: a disabled form control is excluded
        // from submission, mirroring the disabled visible input and buttons.
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        // Always the canonical ISO 8601 string (`to_iso8601` reads the date's
        // ISO slots, not the display-calendar `year()/month()/day()` fields), so
        // a non-Gregorian value still submits the correct calendar-independent
        // date.
        let value = self
            .ctx
            .value
            .get()
            .as_ref()
            .map(CalendarDate::to_iso8601)
            .unwrap_or_default();

        attrs.set(HtmlAttr::Value, value);

        attrs
    }

    // ── Typed handler methods ──────────────────────────────────────────

    /// Handles a click on the trigger button.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// Handles a keydown on the trigger button.
    ///
    /// Returns `true` when the key was handled so adapters can prevent the
    /// follow-up native button-activation click — without this, Enter/Space
    /// would toggle once on keydown and again on the synthesized click, leaving
    /// the popover back in its original state (mirrors
    /// [`collapsible::Api::on_trigger_keydown`](crate::layout::collapsible)).
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) -> bool {
        match data.key {
            KeyboardKey::Enter | KeyboardKey::Space => {
                (self.send)(Event::Toggle);
                true
            }

            KeyboardKey::ArrowDown => {
                (self.send)(Event::Open);
                true
            }

            _ => false,
        }
    }

    /// Handles a click on the clear-trigger button.
    pub fn on_clear_trigger_click(&self) {
        (self.send)(Event::InputChange {
            value: String::new(),
        });
    }

    /// Handles an input text change.
    pub fn on_input_change(&self, value: &str) {
        (self.send)(Event::InputChange {
            value: value.to_string(),
        });
    }

    /// Handles a keydown on the input field.
    pub fn on_input_keydown(&self, key: KeyboardKey) {
        (self.send)(Event::KeyDown { key });
    }

    /// Handles focus entering the **input field**.
    ///
    /// Wire this to the `Input` element only — not the `Trigger`. The
    /// `open_on_click` behavior ("focusing the date field opens the calendar")
    /// applies to the input; the trigger has its own
    /// [`on_trigger_click`](Self::on_trigger_click) /
    /// [`on_trigger_keydown`](Self::on_trigger_keydown) handlers. Wiring focus-in
    /// on the trigger would open the popover on trigger focus and then the
    /// activating click/Enter would immediately toggle it closed.
    pub fn on_focusin(&self) {
        (self.send)(Event::FocusIn);
    }

    /// Handles focus leaving the entire component.
    pub fn on_focusout(&self, focus_leaving_component: bool) {
        if focus_leaving_component {
            (self.send)(Event::FocusOut);
        }
    }

    /// Handles a keydown within the popover content (Escape closes).
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Escape {
            (self.send)(Event::Close);
        }
    }

    // ── Calendar composition ───────────────────────────────────────────

    /// Builds [`Calendar`](super::calendar) props from the current state.
    ///
    /// The adapter creates a Calendar machine with these props inside the
    /// [`Part::Content`] element and wires its `SelectDate` event back to this
    /// machine as [`Event::SelectDate`].
    #[must_use]
    pub fn calendar_props(&self) -> calendar::Props {
        calendar::Props {
            id: format!("{}-calendar", self.ctx.ids.id()),
            value: Some(self.ctx.value.get().clone()),
            min: self.ctx.min.clone(),
            max: self.ctx.max.clone(),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            is_date_unavailable: self.props.is_date_unavailable.clone(),
            is_rtl: self.ctx.is_rtl,
            visible_months: self.props.visible_months,
            // Forward the adapter-injected "today" so the calendar opens on the
            // current month and marks the correct day (otherwise it would fall
            // back to `calendar::Props::default().today`, a fixed test date).
            today: self.ctx.today.clone(),
            ..calendar::Props::default()
        }
    }

    // ── Computed state accessors ───────────────────────────────────────

    /// Whether the popover is currently open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        *self.state == State::Open
    }

    /// The currently selected date.
    #[must_use]
    pub fn selected_date(&self) -> Option<&CalendarDate> {
        self.ctx.value.get().as_ref()
    }

    /// The formatted display value of the selected date.
    #[must_use]
    pub fn formatted_value(&self) -> String {
        self.ctx.formatted_value()
    }

    /// Opens the popover programmatically.
    pub fn open(&self) {
        (self.send)(Event::Open);
    }

    /// Closes the popover programmatically.
    pub fn close(&self) {
        (self.send)(Event::Close);
    }

    /// Toggles the popover programmatically.
    pub fn toggle(&self) {
        (self.send)(Event::Toggle);
    }

    const fn state_name(&self) -> &'static str {
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
            Part::Input => self.input_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}
