//! NumberInput component state machine and connect API.
//!
//! This module implements the framework-agnostic `NumberInput` machine defined
//! in `spec/components/input/number-input.md`. The native `<input>` is the form
//! participant and carries `role="spinbutton"` with `aria-valuenow/min/max`.
//! Numeric value rounding uses **round-half-up** (away from zero on a tie) as
//! defined in spec §1.5 to match user expectation in interactive UI rather than
//! banker's rounding.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Debug, Display};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, KeyboardKey, Locale, MessageFn, NoEffect, TransitionPlan,
};
use ars_interactions::KeyboardEventData;

/// The states for the `NumberInput` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is in an idle state.
    Idle,

    /// The component is focused (text-editing or keyboard stepping).
    Focused,

    /// The component is being scrubbed (pointer drag adjusts the value).
    Scrubbing,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Idle => "idle",
            Self::Focused => "focused",
            Self::Scrubbing => "scrubbing",
        })
    }
}

/// The events for the `NumberInput` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The component received focus.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// The component lost focus.
    Blur,

    /// The component's value changed (raw text from the native input).
    Change(String),

    /// Step up by `Context::step`.
    Increment,

    /// Step down by `Context::step`.
    Decrement,

    /// Step up by `Context::large_step` (`PageUp`).
    IncrementLarge,

    /// Step down by `Context::large_step` (`PageDown`).
    DecrementLarge,

    /// Jump to `Context::max` (End).
    IncrementToMax,

    /// Jump to `Context::min` (Home).
    DecrementToMin,

    /// Programmatically set the numeric value (clamped + rounded).
    SetValue(f64),

    /// Begin a scrubbing gesture.
    StartScrub,

    /// Adjust the value during a scrubbing gesture. `delta` is a unit-less drag
    /// magnitude multiplied by `Context::step` when applied.
    Scrub(f64),

    /// End a scrubbing gesture.
    EndScrub,

    /// Mouse wheel input. Positive `delta` increments, negative decrements.
    Wheel {
        /// Signed delta — sign determines direction; magnitude is ignored.
        delta: f64,
    },

    /// IME composition started.
    CompositionStart,

    /// IME composition ended.
    CompositionEnd,

    /// Synchronize the externally controlled value prop.
    ///
    /// `Some` switches the component to controlled mode and pushes the new
    /// value; `None` returns the component to uncontrolled mode.
    SyncValue(Option<f64>),

    /// Synchronize output-affecting props (`min` / `max` / `step` /
    /// `large_step` / `precision` / `disabled` / `readonly` / `invalid` /
    /// `required` / `name` / `spin_on_press`) stored in [`Context`] when
    /// [`Service::set_props`] reports a change.
    SetProps,

    /// Track whether a [`Part::Description`] part is rendered (gates
    /// `aria-describedby`).
    SetHasDescription(bool),
}

/// The context for the `NumberInput` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The controlled/uncontrolled numeric value.
    pub value: Bindable<Option<f64>>,

    /// The minimum allowed value (inclusive).
    pub min: f64,

    /// The maximum allowed value (inclusive).
    pub max: f64,

    /// The step size used by `Increment`/`Decrement`.
    pub step: f64,

    /// The large step size used by `IncrementLarge`/`DecrementLarge`.
    pub large_step: f64,

    /// The decimal precision applied via round-half-up after each value mutation.
    pub precision: Option<u32>,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is readonly.
    pub readonly: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// Whether the component is required.
    pub required: bool,

    /// Whether the component is focused.
    pub focused: bool,

    /// Whether the focus is visible (keyboard-initiated).
    pub focus_visible: bool,

    /// The `name` attribute used for form submission.
    pub name: Option<String>,

    /// Whether holding a stepper repeats the action with acceleration.
    pub spin_on_press: bool,

    /// Whether the component is currently being scrubbed.
    pub scrubbing: bool,

    /// True while an IME composition session is active.
    pub is_composing: bool,

    /// Whether a Description part is rendered (gates `aria-describedby`).
    pub has_description: bool,

    /// Resolved locale for i18n.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component IDs for part identification.
    pub ids: ComponentIds,
}

/// Locale-aware formatting and parsing options.
///
/// `NumberInput` unifies on [`ars_i18n::number::FormatOptions`] so all locale-aware
/// formatting and parsing flow through a single source of truth (see
/// `crates/ars-i18n/src/number/mod.rs`). The struct is exposed verbatim via this
/// alias so consumers do not need to depend on `ars-i18n` directly when wiring
/// `NumberInput` props.
pub type FormatOptions = ars_i18n::number::FormatOptions;

/// The props for the `NumberInput` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base ID for the number input root.
    pub id: String,

    /// Controlled value. When `Some`, component is controlled.
    pub value: Option<f64>,

    /// Default value for uncontrolled mode.
    pub default_value: Option<f64>,

    /// The minimum value.
    pub min: f64,

    /// The maximum value.
    pub max: f64,

    /// The step size used by `Increment`/`Decrement`.
    pub step: f64,

    /// The large step size used by `IncrementLarge`/`DecrementLarge`.
    pub large_step: f64,

    /// The decimal precision applied via round-half-up.
    pub precision: Option<u32>,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is readonly.
    pub readonly: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// Whether the component is required.
    pub required: bool,

    /// The `name` attribute for form submission.
    pub name: Option<String>,

    /// The ID of the form element the input is associated with.
    pub form: Option<String>,

    /// Whether mouse-wheel input over the focused field steps the value.
    pub allow_mouse_wheel: bool,

    /// Whether the value is clamped to `[min, max]` on `Blur`.
    pub clamp_value_on_blur: bool,

    /// Whether holding a stepper repeats with acceleration.
    pub spin_on_press: bool,

    /// Locale-aware formatting options used by adapters for paste parsing.
    pub format_options: Option<FormatOptions>,

    /// Locale-aware formatting options applied to the displayed value when the
    /// input is not focused.
    pub display_format: Option<FormatOptions>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            min: f64::NEG_INFINITY,
            max: f64::INFINITY,
            step: 1.0,
            large_step: 10.0,
            precision: None,
            disabled: false,
            readonly: false,
            invalid: false,
            required: false,
            name: None,
            form: None,
            allow_mouse_wheel: false,
            clamp_value_on_blur: true,
            spin_on_press: true,
            format_options: None,
            display_format: None,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id), the adapter-provided base ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`value`](Self::value), switching to controlled mode.
    #[must_use]
    pub const fn value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    /// Clears [`value`](Self::value), switching to uncontrolled mode.
    #[must_use]
    pub const fn uncontrolled(mut self) -> Self {
        self.value = None;
        self
    }

    /// Sets [`default_value`](Self::default_value).
    #[must_use]
    pub const fn default_value(mut self, value: f64) -> Self {
        self.default_value = Some(value);
        self
    }

    /// Sets [`min`](Self::min).
    #[must_use]
    pub const fn min(mut self, value: f64) -> Self {
        self.min = value;
        self
    }

    /// Sets [`max`](Self::max).
    #[must_use]
    pub const fn max(mut self, value: f64) -> Self {
        self.max = value;
        self
    }

    /// Sets [`step`](Self::step).
    #[must_use]
    pub const fn step(mut self, value: f64) -> Self {
        self.step = value;
        self
    }

    /// Sets [`large_step`](Self::large_step).
    #[must_use]
    pub const fn large_step(mut self, value: f64) -> Self {
        self.large_step = value;
        self
    }

    /// Sets [`precision`](Self::precision).
    #[must_use]
    pub const fn precision(mut self, value: u32) -> Self {
        self.precision = Some(value);
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

    /// Sets [`invalid`](Self::invalid).
    #[must_use]
    pub const fn invalid(mut self, value: bool) -> Self {
        self.invalid = value;
        self
    }

    /// Sets [`required`](Self::required).
    #[must_use]
    pub const fn required(mut self, value: bool) -> Self {
        self.required = value;
        self
    }

    /// Sets [`name`](Self::name).
    #[must_use]
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Sets [`form`](Self::form).
    #[must_use]
    pub fn form(mut self, value: impl Into<String>) -> Self {
        self.form = Some(value.into());
        self
    }

    /// Sets [`allow_mouse_wheel`](Self::allow_mouse_wheel).
    #[must_use]
    pub const fn allow_mouse_wheel(mut self, value: bool) -> Self {
        self.allow_mouse_wheel = value;
        self
    }

    /// Sets [`clamp_value_on_blur`](Self::clamp_value_on_blur).
    #[must_use]
    pub const fn clamp_value_on_blur(mut self, value: bool) -> Self {
        self.clamp_value_on_blur = value;
        self
    }

    /// Sets [`spin_on_press`](Self::spin_on_press).
    #[must_use]
    pub const fn spin_on_press(mut self, value: bool) -> Self {
        self.spin_on_press = value;
        self
    }

    /// Sets [`format_options`](Self::format_options).
    #[must_use]
    pub fn format_options(mut self, value: FormatOptions) -> Self {
        self.format_options = Some(value);
        self
    }

    /// Sets [`display_format`](Self::display_format).
    #[must_use]
    pub fn display_format(mut self, value: FormatOptions) -> Self {
        self.display_format = Some(value);
        self
    }
}

/// Locale-specific labels for the `NumberInput` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the increment button.
    pub increment_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the decrement button.
    pub decrement_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            increment_label: MessageFn::static_str("Increment"),
            decrement_label: MessageFn::static_str("Decrement"),
        }
    }
}

impl ComponentMessages for Messages {}

/// The machine for the `NumberInput` component.
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

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (State, Context) {
        (
            State::Idle,
            Context {
                value: if let Some(value) = props.value {
                    Bindable::controlled(Some(value))
                } else {
                    Bindable::uncontrolled(props.default_value)
                },
                min: props.min,
                max: props.max,
                step: props.step,
                large_step: props.large_step,
                precision: props.precision,
                disabled: props.disabled,
                readonly: props.readonly,
                invalid: props.invalid,
                required: props.required,
                focused: false,
                focus_visible: false,
                name: props.name.clone(),
                spin_on_press: props.spin_on_press,
                scrubbing: false,
                is_composing: false,
                has_description: false,
                locale: env.locale.clone(),
                messages: messages.clone(),
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled || ctx.readonly {
            match event {
                Event::Increment
                | Event::Decrement
                | Event::IncrementLarge
                | Event::DecrementLarge
                | Event::IncrementToMax
                | Event::DecrementToMin
                | Event::Change(_)
                | Event::SetValue(_)
                | Event::StartScrub
                | Event::Scrub(_)
                | Event::Wheel { .. } => return None,
                // `EndScrub` is intentionally exempt: if the parent
                // flips disabled/readonly during an active drag, the
                // adapter still fires `EndScrub` on pointer-up and the
                // machine must process it, otherwise state stays stuck
                // at `Scrubbing` with `ctx.scrubbing = true` until some
                // unrelated event clears it.
                _ => {}
            }
        }

        match event {
            Event::Focus { is_keyboard } => {
                let is_keyboard = *is_keyboard;

                Some(
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = is_keyboard;
                    }),
                )
            }

            Event::Blur => {
                let clamp_on_blur = props.clamp_value_on_blur;
                let min = ctx.min;
                let max = ctx.max;
                let precision = ctx.precision;
                let current = *ctx.value.get();

                Some(
                    TransitionPlan::to(State::Idle).apply(move |ctx: &mut Context| {
                        ctx.focused = false;
                        ctx.focus_visible = false;
                        // Clear `scrubbing` here: if Blur fires during an
                        // active scrub gesture (pointer capture loss,
                        // focus transfer), the stale flag would otherwise
                        // leave adapters rendering drag affordances while
                        // the FSM is in `Idle`.
                        ctx.scrubbing = false;

                        if clamp_on_blur && let Some(value) = current {
                            let bounded = round_and_clamp(value, min, max, precision);

                            if (bounded - value).abs() > f64::EPSILON {
                                ctx.value.set(Some(bounded));
                            }
                        }
                    }),
                )
            }

            Event::Increment => stepped_plan(ctx, ctx.step, true),

            Event::Decrement => stepped_plan(ctx, ctx.step, false),

            Event::IncrementLarge => stepped_plan(ctx, ctx.large_step, true),

            Event::DecrementLarge => stepped_plan(ctx, ctx.large_step, false),

            Event::IncrementToMax => {
                // No-op when `max` is not finite: with default props
                // (`max = +∞`) this event would store `+inf` in
                // ctx.value and serialize as `"inf"` into the visible
                // `value` attribute and `aria-valuenow`, violating the
                // spinbutton contract. Storing the bound is only safe
                // when the bound is itself a finite number.
                if !ctx.max.is_finite() {
                    return None;
                }
                // Pass through the same clamp + round pipeline so bounds
                // with a fractional part round consistently AND stay
                // inside `[min, max]` afterwards. Without the second
                // clamp, `max = 1.5, precision = 0` would round up to
                // `2.0` and leave `aria-valuenow > aria-valuemax`.
                let target = round_and_clamp(ctx.max, ctx.min, ctx.max, ctx.precision);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(Some(target));
                }))
            }

            Event::DecrementToMin => {
                if !ctx.min.is_finite() {
                    return None;
                }
                let target = round_and_clamp(ctx.min, ctx.min, ctx.max, ctx.precision);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(Some(target));
                }))
            }

            Event::SetValue(value) => {
                let value = round_and_clamp(*value, ctx.min, ctx.max, ctx.precision);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(Some(value));
                }))
            }

            Event::Change(text) => {
                let text = text.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    if text.is_empty() {
                        ctx.value.set(None);
                    } else if let Ok(value) = text.parse::<f64>()
                        && value.is_finite()
                    {
                        // Reject `NaN`, `±Inf` — they would propagate into
                        // `ctx.value` and produce a non-numeric
                        // `aria-valuenow`, violating the spinbutton
                        // contract.
                        ctx.value.set(Some(value));
                    }
                }))
            }

            Event::StartScrub => Some(TransitionPlan::to(State::Scrubbing).apply(
                |ctx: &mut Context| {
                    ctx.scrubbing = true;
                },
            )),

            Event::Scrub(delta) if matches!(state, State::Scrubbing) => {
                let current = ctx.value.get().unwrap_or(0.0);

                let next =
                    round_and_clamp(current + delta * ctx.step, ctx.min, ctx.max, ctx.precision);

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(Some(next));
                }))
            }

            Event::EndScrub => {
                let target = if ctx.focused {
                    State::Focused
                } else {
                    State::Idle
                };

                Some(TransitionPlan::to(target).apply(|ctx: &mut Context| {
                    ctx.scrubbing = false;
                }))
            }

            Event::Wheel { delta }
                if matches!(state, State::Focused) && props.allow_mouse_wheel =>
            {
                let stepped = if *delta > 0.0 {
                    Event::Increment
                } else if *delta < 0.0 {
                    Event::Decrement
                } else {
                    return None;
                };

                Self::transition(state, &stepped, ctx, props)
            }

            Event::CompositionStart => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.is_composing = true;
            })),

            Event::CompositionEnd => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.is_composing = false;
            })),

            Event::Scrub(_) | Event::Wheel { .. } => None,

            Event::SyncValue(value) => {
                let value = *value;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    match value {
                        // Reject non-finite controlled values for the
                        // same reason `Change` rejects them: storing
                        // `NaN`/`±Inf` produces `"NaN"`/`"inf"` in the
                        // visible `value` attribute (`aria-valuenow` is
                        // clamped separately in `input_attrs`), which
                        // violates the spinbutton contract. The previous
                        // valid value is retained; a parent passing
                        // non-finite is treated as a no-op sync.
                        Some(v) if v.is_finite() => {
                            ctx.value.set(Some(v));
                            ctx.value.sync_controlled(Some(Some(v)));
                        }
                        Some(_non_finite) => {
                            // Intentionally dropped; do not write `NaN`
                            // or `±Inf` into the controlled slot.
                        }
                        None => {
                            ctx.value.sync_controlled(None);
                        }
                    }
                }))
            }

            Event::SetProps => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let bounds_or_precision_changed = (ctx.min - props.min).abs() > f64::EPSILON
                        || (ctx.max - props.max).abs() > f64::EPSILON
                        || ctx.precision != props.precision;

                    ctx.min = props.min;
                    ctx.max = props.max;
                    ctx.step = props.step;
                    ctx.large_step = props.large_step;
                    ctx.precision = props.precision;
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.invalid = props.invalid;
                    ctx.required = props.required;
                    ctx.name = props.name.clone();
                    ctx.spin_on_press = props.spin_on_press;

                    // Reclamp + reround the internal value when the
                    // bounds or precision change — UNCONTROLLED mode
                    // only. In controlled mode the parent owns the
                    // value; self-authoring a clamped controlled value
                    // here would silently override the parent's intent
                    // (next render with the same prop wouldn't emit
                    // `SyncValue` so the desync would persist). Adapters
                    // that want clamped controlled values must pass a
                    // pre-clamped `props.value`. The display-time
                    // `aria-valuenow` clamp in `input_attrs` still
                    // keeps the spinbutton contract intact for AT.
                    if bounds_or_precision_changed
                        && !ctx.value.is_controlled()
                        && let Some(value) = *ctx.value.get()
                    {
                        let bounded = round_and_clamp(value, ctx.min, ctx.max, ctx.precision);
                        ctx.value.set(Some(bounded));
                    }
                }))
            }

            Event::SetHasDescription(has_description) => {
                let has_description = *has_description;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_description = has_description;
                }))
            }
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "number_input::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.value != new.value {
            events.push(Event::SyncValue(new.value));
        }

        if props_output_changed(old, new) {
            events.push(Event::SetProps);
        }

        events
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }
}

/// Maximum precision honoured by [`round_to_precision`]. Caps at the f64
/// significant-digit budget (~15–17 decimal digits); higher precisions
/// cannot be represented anyway, and crucially this cap also prevents
/// the `as i32` cast below from wrapping for arbitrary `u32` inputs and
/// keeps `10.0_f64.powi(p)` finite (`powi(p > 308)` saturates to `+inf`,
/// which would collapse the rounding math).
const MAX_PRECISION: u32 = 15;

/// Round-half-up: ties move away from zero.
///
/// Banker's rounding (`f64::round_ties_even`) is unsuitable for interactive UI
/// because end users expect `2.5` to round to `3` rather than `2`. See
/// `spec/components/input/number-input.md` §1.5 for the rationale.
fn round_to_precision(value: f64, precision: Option<u32>) -> f64 {
    let Some(p) = precision else {
        return value;
    };

    // Cap precision before casting to i32: large `u32` values would
    // wrap negative through `as i32`, and `10.0_f64.powi(p)` saturates
    // to `+inf` for any `p > 308`. Both cases poison the rounding math.
    let p = p.min(MAX_PRECISION);

    let factor = 10_f64.powi(p as i32);

    (value * factor + 0.5_f64.copysign(value)).trunc() / factor
}

const fn clamp(value: f64, min: f64, max: f64) -> f64 {
    let lower = if value > min { value } else { min };

    if lower < max { lower } else { max }
}

/// Combined clamp + round-half-up pipeline used by every value-mutating
/// transition. Order matters and **must terminate with a clamp**: rounding
/// can push a bound-value across the bound itself (e.g. `max = 1.5` under
/// `precision = 0` rounds up to `2.0`, which would otherwise leave
/// `aria-valuenow > aria-valuemax`), so the final clamp pulls the value
/// back into `[min, max]`.
fn round_and_clamp(value: f64, min: f64, max: f64, precision: Option<u32>) -> f64 {
    clamp(
        round_to_precision(clamp(value, min, max), precision),
        min,
        max,
    )
}

fn props_output_changed(old: &Props, new: &Props) -> bool {
    (old.min - new.min).abs() > f64::EPSILON
        || (old.max - new.max).abs() > f64::EPSILON
        || (old.step - new.step).abs() > f64::EPSILON
        || (old.large_step - new.large_step).abs() > f64::EPSILON
        || old.precision != new.precision
        || old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.invalid != new.invalid
        || old.required != new.required
        || old.name != new.name
        || old.form != new.form
        || old.spin_on_press != new.spin_on_press
        || old.allow_mouse_wheel != new.allow_mouse_wheel
        || old.clamp_value_on_blur != new.clamp_value_on_blur
}

fn stepped_plan(ctx: &Context, step: f64, up: bool) -> Option<TransitionPlan<Machine>> {
    let current = ctx.value.get().unwrap_or_else(|| baseline(ctx));
    let raw = if up { current + step } else { current - step };
    let next = round_and_clamp(raw, ctx.min, ctx.max, ctx.precision);

    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.value.set(Some(next));
    }))
}

/// Returns the finite baseline used when stepping/scrubbing from an empty
/// value. Always finite — `0.0` if it falls inside `[min, max]`, otherwise
/// the nearest finite bound, defaulting to `0.0` when both bounds are
/// non-finite (the default `min = -∞`, `max = +∞` config).
fn baseline(ctx: &Context) -> f64 {
    if ctx.min.is_finite() && ctx.max.is_finite() {
        clamp(0.0, ctx.min, ctx.max)
    } else if ctx.min.is_finite() && ctx.min > 0.0 {
        ctx.min
    } else if ctx.max.is_finite() && ctx.max < 0.0 {
        ctx.max
    } else {
        0.0
    }
}

/// Structural parts exposed by the `NumberInput` connect API.
#[derive(ComponentPart)]
#[scope = "number-input"]
pub enum Part {
    /// The root container element.
    Root,

    /// The visible label element.
    Label,

    /// The native `<input>` element (carries `role="spinbutton"`).
    Input,

    /// The optional increment button.
    IncrementTrigger,

    /// The optional decrement button.
    DecrementTrigger,

    /// The optional descriptive help-text element.
    Description,

    /// The optional validation error message element.
    ErrorMessage,
}

/// The API for the `NumberInput` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .field("props", &self.props)
            .field("send", &"<callback>")
            .finish()
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Input => self.input_attrs(),
            Part::IncrementTrigger => self.increment_trigger_attrs(),
            Part::DecrementTrigger => self.decrement_trigger_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}

impl Api<'_> {
    /// Attributes for the root container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Data("ars-state"), self.state.to_string());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Attributes for the label element.
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

    /// Attributes for the native `<input>` element.
    #[must_use]
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("input"))
            .set(HtmlAttr::Role, "spinbutton")
            .set(HtmlAttr::InputMode, "decimal")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        if let Some(value) = self.ctx.value.get() {
            // The visible `value` attribute carries the raw text the
            // user typed (so mid-typing edits are not lost). The ARIA
            // `aria-valuenow` MUST stay inside `[aria-valuemin,
            // aria-valuemax]` to satisfy the spinbutton contract — a
            // user typing `999` with `max=100` must not expose
            // `aria-valuenow="999"` to assistive tech.
            attrs.set(HtmlAttr::Value, value.to_string());
            let bounded = clamp(*value, self.ctx.min, self.ctx.max);
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), bounded.to_string());
        }

        if self.ctx.min.is_finite() {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), self.ctx.min.to_string());
        }

        if self.ctx.max.is_finite() {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), self.ctx.max.to_string());
        }

        set_described_by(&mut attrs, self.ctx);

        if self.ctx.required {
            // Native `required` works with browser constraint validation;
            // `aria-required` announces the requirement to assistive tech.
            attrs.set_bool(HtmlAttr::Required, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
            attrs.set(
                HtmlAttr::Aria(AriaAttr::ErrorMessage),
                self.ctx.ids.part("error-message"),
            );
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
        }

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        if let Some(form) = &self.props.form {
            attrs.set(HtmlAttr::Form, form.clone());
        }

        attrs
    }

    /// Attributes for the increment trigger button.
    ///
    /// Adapters drive press-and-hold spin from [`Context::spin_on_press`]; the
    /// agnostic core only emits the per-click intent.
    #[must_use]
    pub fn increment_trigger_attrs(&self) -> AttrMap {
        self.trigger_attrs(
            &Part::IncrementTrigger,
            &self.ctx.messages.increment_label,
            self.is_at_max(),
        )
    }

    /// Attributes for the decrement trigger button.
    #[must_use]
    pub fn decrement_trigger_attrs(&self) -> AttrMap {
        self.trigger_attrs(
            &Part::DecrementTrigger,
            &self.ctx.messages.decrement_label,
            self.is_at_min(),
        )
    }

    /// Attributes for the description/help text element.
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

    /// Attributes for the validation error message element.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite");

        attrs
    }

    /// Sends [`Event::Focus`] for input focus.
    pub fn on_input_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Sends [`Event::Blur`] for input blur.
    pub fn on_input_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Sends [`Event::Change`] for input changes.
    pub fn on_input_change(&self, value: String) {
        (self.send)(Event::Change(value));
    }

    /// Sends [`Event::Increment`] for increment trigger activation.
    pub fn on_increment_click(&self) {
        (self.send)(Event::Increment);
    }

    /// Sends [`Event::Decrement`] for decrement trigger activation.
    pub fn on_decrement_click(&self) {
        (self.send)(Event::Decrement);
    }

    /// Handles normalized keydown data on the input element.
    ///
    /// Returns `true` when the key was handled by the core machine.
    /// In `disabled` / `readonly` modes the transition arms drop these
    /// events, so the method also returns `false` there — otherwise
    /// adapters would suppress native key behavior on a false positive.
    pub fn on_input_keydown(&self, data: &KeyboardEventData) -> bool {
        if data.is_composing {
            return false;
        }

        // Arrow / Page / Home / End all hit the disabled-or-readonly
        // guard in `transition` — short-circuit here so the return
        // value reflects what the machine actually does.
        if self.ctx.disabled || self.ctx.readonly {
            return false;
        }

        let event = match data.key {
            KeyboardKey::ArrowUp => Event::Increment,
            KeyboardKey::ArrowDown => Event::Decrement,
            KeyboardKey::PageUp => Event::IncrementLarge,
            KeyboardKey::PageDown => Event::DecrementLarge,
            KeyboardKey::Home => Event::DecrementToMin,
            KeyboardKey::End => Event::IncrementToMax,
            _ => return false,
        };

        (self.send)(event);

        true
    }

    fn trigger_attrs(
        &self,
        part: &Part,
        label: &MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
        at_boundary: bool,
    ) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::TabIndex, "-1")
            .set(HtmlAttr::Aria(AriaAttr::Label), label(&self.ctx.locale));

        if self.ctx.disabled || self.ctx.readonly || at_boundary {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    fn is_at_max(&self) -> bool {
        self.ctx
            .value
            .get()
            .is_some_and(|value| value >= self.ctx.max)
    }

    fn is_at_min(&self) -> bool {
        self.ctx
            .value
            .get()
            .is_some_and(|value| value <= self.ctx.min)
    }
}

fn set_described_by(attrs: &mut AttrMap, ctx: &Context) {
    let mut described_by = Vec::new();

    if ctx.has_description {
        described_by.push(ctx.ids.part("description"));
    }

    if ctx.invalid {
        described_by.push(ctx.ids.part("error-message"));
    }

    if !described_by.is_empty() {
        attrs.set(
            HtmlAttr::Aria(AriaAttr::DescribedBy),
            described_by.join(" "),
        );
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use ars_core::{ConnectApi, Env, HtmlAttr, Service};
    use insta::assert_snapshot;

    use super::*;

    fn props() -> Props {
        Props::new().id("num")
    }

    fn bounded_props() -> Props {
        props().min(0.0).max(100.0).step(1.0).large_step(10.0)
    }

    fn fmt_opts() -> FormatOptions {
        FormatOptions {
            use_grouping: false,
            ..FormatOptions::default()
        }
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn keyboard_event(key: KeyboardKey, is_composing: bool) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character: None,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing,
        }
    }

    #[test]
    fn number_input_initial_state_is_idle() {
        let svc = service(props().default_value(5.0));

        assert_eq!(svc.state(), &State::Idle);
        assert_eq!(svc.context().value.get(), &Some(5.0));
        assert!(!svc.context().focused);
        assert!(!svc.context().scrubbing);
    }

    #[test]
    fn number_input_increment_respects_step() {
        let mut svc = service(bounded_props().default_value(10.0).step(2.0));

        drop(svc.send(Event::Increment));

        assert_eq!(svc.context().value.get(), &Some(12.0));
    }

    #[test]
    fn number_input_increment_clamps_to_max() {
        let mut svc = service(bounded_props().default_value(99.0));

        drop(svc.send(Event::Increment));
        drop(svc.send(Event::Increment));

        assert_eq!(svc.context().value.get(), &Some(100.0));
    }

    #[test]
    fn number_input_decrement_clamps_to_min() {
        let mut svc = service(bounded_props().default_value(1.0));

        drop(svc.send(Event::Decrement));
        drop(svc.send(Event::Decrement));

        assert_eq!(svc.context().value.get(), &Some(0.0));
    }

    #[test]
    fn number_input_large_step_increment_decrement() {
        let mut svc = service(bounded_props().default_value(50.0));

        drop(svc.send(Event::IncrementLarge));

        assert_eq!(svc.context().value.get(), &Some(60.0));

        drop(svc.send(Event::DecrementLarge));

        assert_eq!(svc.context().value.get(), &Some(50.0));
    }

    #[test]
    fn number_input_increment_to_max_and_decrement_to_min() {
        let mut svc = service(bounded_props().default_value(50.0));

        drop(svc.send(Event::IncrementToMax));

        assert_eq!(svc.context().value.get(), &Some(100.0));

        drop(svc.send(Event::DecrementToMin));

        assert_eq!(svc.context().value.get(), &Some(0.0));
    }

    #[test]
    fn number_input_change_parses_canonical_decimal() {
        let mut svc = service(bounded_props());

        drop(svc.send(Event::Change("42.5".to_string())));

        assert_eq!(svc.context().value.get(), &Some(42.5));
    }

    #[test]
    fn number_input_change_empty_string_clears_value() {
        let mut svc = service(bounded_props().default_value(7.0));

        drop(svc.send(Event::Change(String::new())));

        assert_eq!(svc.context().value.get(), &None);
    }

    #[test]
    fn number_input_change_rejects_non_numeric() {
        let mut svc = service(bounded_props().default_value(7.0));

        drop(svc.send(Event::Change("abc".to_string())));

        assert_eq!(svc.context().value.get(), &Some(7.0));
    }

    #[test]
    fn number_input_aria_valuenow_is_clamped_when_typed_value_out_of_range() {
        // User typing '999' into a [0, 100] spinbutton must NOT expose
        // `aria-valuenow=999 > aria-valuemax=100`. The visible `value`
        // attribute keeps the raw text so mid-typing edits are visible,
        // but `aria-valuenow` is bounded.
        let mut svc = service(bounded_props().default_value(50.0));

        drop(svc.send(Event::Change("999".to_string())));

        let api = svc.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Value), Some("999"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("100"));
    }

    #[test]
    fn number_input_keydown_returns_false_when_disabled() {
        let svc = service(bounded_props().disabled(true));

        let api = svc.connect(&|_| {});

        for key in [
            KeyboardKey::ArrowUp,
            KeyboardKey::ArrowDown,
            KeyboardKey::PageUp,
            KeyboardKey::PageDown,
            KeyboardKey::Home,
            KeyboardKey::End,
        ] {
            let data = keyboard_event(key, false);

            assert!(
                !api.on_input_keydown(&data),
                "disabled machine must not claim {key:?} as handled"
            );
        }
    }

    #[test]
    fn number_input_keydown_returns_false_when_readonly() {
        let svc = service(bounded_props().readonly(true));

        let api = svc.connect(&|_| {});

        for key in [KeyboardKey::ArrowUp, KeyboardKey::Home] {
            let data = keyboard_event(key, false);

            assert!(!api.on_input_keydown(&data));
        }
    }

    #[test]
    fn number_input_change_rejects_non_finite_values() {
        // f64::parse accepts "inf", "-inf", "NaN", "infinity" — the
        // machine must reject these so `ctx.value` and `aria-valuenow`
        // never carry a non-numeric string.
        let mut svc = service(bounded_props().default_value(7.0));

        for non_finite in ["inf", "-inf", "NaN", "infinity", "-infinity"] {
            drop(svc.send(Event::Change(non_finite.to_string())));

            assert_eq!(
                svc.context().value.get(),
                &Some(7.0),
                "Change({non_finite:?}) must not overwrite finite value"
            );
        }
    }

    #[test]
    fn number_input_increment_to_max_rounds_and_stays_within_bounds() {
        // `1.5` is exactly representable. Round_half_up at 0 decimals gives
        // 2.0 — which would EXCEED `max = 1.5`. The final clamp in
        // `round_and_clamp` pulls the result back to 1.5, preserving the
        // bound contract.
        let mut svc = service(props().min(0.0).max(1.5).precision(0));

        drop(svc.send(Event::IncrementToMax));

        let value = svc.context().value.get().expect("value set");

        assert!(value <= 1.5, "stored value {value} must not exceed max");
        assert_eq!(svc.context().value.get(), &Some(1.5));
    }

    #[test]
    fn number_input_decrement_to_min_rounds_and_stays_within_bounds() {
        // `-1.5` rounded to 0 decimals would be -2.0, which would violate
        // `min = -1.5`. The clamp pulls it back.
        let mut svc = service(props().min(-1.5).max(0.0).precision(0));

        drop(svc.send(Event::DecrementToMin));

        let value = svc.context().value.get().expect("value set");

        assert!(value >= -1.5, "stored value {value} must not undercut min");
        assert_eq!(svc.context().value.get(), &Some(-1.5));
    }

    #[test]
    fn number_input_stepped_plan_clamps_after_rounding() {
        // Step from below the max, then verify rounding never pushes
        // ctx.value past the configured bound.
        let mut svc = service(props().min(0.0).max(1.5).step(0.5).precision(0));

        drop(svc.send(Event::Increment));
        drop(svc.send(Event::Increment));
        drop(svc.send(Event::Increment));
        drop(svc.send(Event::Increment));

        let value = svc.context().value.get().expect("value set");

        assert!(value <= 1.5, "stepped value {value} must respect max");
    }

    #[test]
    fn number_input_blur_clears_scrubbing_marker() {
        // Blur during an active scrub gesture (e.g. pointer-capture loss)
        // must reset `ctx.scrubbing` so adapters don't keep rendering
        // drag affordances while the FSM is back in Idle.
        let mut svc = service(bounded_props().default_value(10.0));

        drop(svc.send(Event::StartScrub));

        assert!(svc.context().scrubbing);

        drop(svc.send(Event::Blur));

        assert_eq!(svc.state(), &State::Idle);
        assert!(!svc.context().scrubbing);
    }

    #[test]
    fn number_input_required_sets_native_required_alongside_aria_required() {
        let svc = service(bounded_props().required(true));

        let api = svc.connect(&|_| {});

        let attrs = api.input_attrs();

        assert!(attrs.contains(&HtmlAttr::Required));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Required)), Some("true"));
    }

    #[test]
    fn number_input_blur_clamps_value_when_enabled() {
        let mut svc = service(
            bounded_props()
                .default_value(50.0)
                .clamp_value_on_blur(true),
        );

        drop(svc.send(Event::Change("999".to_string())));
        drop(svc.send(Event::Blur));

        assert_eq!(svc.context().value.get(), &Some(100.0));
    }

    #[test]
    fn number_input_blur_skips_clamp_when_disabled() {
        let mut svc = service(
            bounded_props()
                .default_value(50.0)
                .clamp_value_on_blur(false),
        );

        drop(svc.send(Event::Change("999".to_string())));
        drop(svc.send(Event::Blur));

        assert_eq!(svc.context().value.get(), &Some(999.0));
    }

    #[test]
    fn number_input_set_value_clamps_and_rounds() {
        let mut svc = service(bounded_props().precision(1));

        drop(svc.send(Event::SetValue(1.25)));

        assert_eq!(svc.context().value.get(), &Some(1.3));
    }

    #[test]
    fn number_input_round_half_up_positive_and_negative() {
        let mut svc = service(props().min(-100.0).max(100.0).precision(0));

        drop(svc.send(Event::SetValue(2.5)));

        assert_eq!(svc.context().value.get(), &Some(3.0));

        drop(svc.send(Event::SetValue(-2.5)));

        assert_eq!(svc.context().value.get(), &Some(-3.0));
    }

    #[test]
    fn number_input_private_numeric_helpers_cover_boundaries() {
        assert_eq!(clamp(0.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp(10.0, 0.0, 10.0), 10.0);
        assert_eq!(clamp(-1.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp(11.0, 0.0, 10.0), 10.0);

        let both_finite = service(props().min(5.0).max(10.0));

        assert_eq!(baseline(both_finite.context()), 5.0);

        let positive_min = service(props().min(5.0));

        assert_eq!(baseline(positive_min.context()), 5.0);

        let negative_max = service(props().max(-5.0));

        assert_eq!(baseline(negative_max.context()), -5.0);

        let negative_min_unbounded_max = service(props().min(-5.0));

        assert_eq!(baseline(negative_min_unbounded_max.context()), 0.0);

        let positive_max_unbounded_min = service(props().max(5.0));

        assert_eq!(baseline(positive_max_unbounded_min.context()), 0.0);

        let unbounded = service(props());

        assert_eq!(baseline(unbounded.context()), 0.0);
    }

    #[test]
    fn number_input_props_output_changed_covers_each_render_field() {
        let old = bounded_props().min(1.0).max(2.0).step(1.0).large_step(1.0);

        assert!(!props_output_changed(&old, &old));

        let mut epsilon_only = old.clone();

        epsilon_only.min += f64::EPSILON;

        assert!(!props_output_changed(&old, &epsilon_only));

        epsilon_only = old.clone();
        epsilon_only.max -= f64::EPSILON;

        assert!(!props_output_changed(&old, &epsilon_only));

        epsilon_only = old.clone();
        epsilon_only.step += f64::EPSILON;

        assert!(!props_output_changed(&old, &epsilon_only));

        epsilon_only = old.clone();
        epsilon_only.large_step -= f64::EPSILON;

        assert!(!props_output_changed(&old, &epsilon_only));

        let mut new = old.clone();

        new.min = 1.5;

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.max = 99.0;

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.step = 2.0;

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.large_step = 20.0;

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.precision = Some(2);

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.disabled = true;

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.readonly = true;

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.invalid = true;

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.required = true;

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.name = Some("amount".to_string());

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.form = Some("order".to_string());

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.spin_on_press = false;

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.allow_mouse_wheel = true;

        assert!(props_output_changed(&old, &new));

        new = old.clone();
        new.clamp_value_on_blur = false;

        assert!(props_output_changed(&old, &new));
    }

    #[test]
    fn number_input_scrub_lifecycle_uses_scrubbing_state() {
        let mut svc = service(bounded_props().step(2.0).default_value(10.0));

        drop(svc.send(Event::StartScrub));

        assert_eq!(svc.state(), &State::Scrubbing);
        assert!(svc.context().scrubbing);

        drop(svc.send(Event::Scrub(3.0)));

        assert_eq!(svc.context().value.get(), &Some(16.0));

        drop(svc.send(Event::EndScrub));

        assert_eq!(svc.state(), &State::Idle);
        assert!(!svc.context().scrubbing);
    }

    #[test]
    fn number_input_scrub_outside_scrubbing_state_is_ignored() {
        let mut svc = service(bounded_props().default_value(10.0));

        let result = svc.send(Event::Scrub(5.0));

        assert!(!result.context_changed);
        assert_eq!(svc.context().value.get(), &Some(10.0));
    }

    #[test]
    fn number_input_increment_from_empty_with_infinite_bounds_uses_finite_baseline() {
        // Default props: min=-inf, max=+inf, value=None.
        // Without the baseline fix, this would produce -inf via unwrap_or(min).
        let mut svc = service(props());

        assert_eq!(svc.context().value.get(), &None);

        drop(svc.send(Event::Increment));

        let value = svc.context().value.get().expect("value set after step");

        assert!(value.is_finite(), "value must remain finite after stepping");
        assert!(
            (value - 1.0).abs() < f64::EPSILON,
            "baseline 0.0 + step 1.0 = 1.0"
        );
    }

    #[test]
    fn number_input_decrement_from_empty_with_infinite_bounds_uses_finite_baseline() {
        let mut svc = service(props());

        drop(svc.send(Event::Decrement));

        let value = svc.context().value.get().expect("value set after step");

        assert!(value.is_finite());
        assert!((value - -1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn number_input_increment_from_empty_with_positive_min_starts_at_min() {
        let mut svc = service(props().min(5.0).max(100.0));

        drop(svc.send(Event::Increment));

        // 0.0 clamped to [5,100] = 5.0, then +1.0 step = 6.0.
        assert_eq!(svc.context().value.get(), &Some(6.0));
    }

    #[test]
    fn number_input_set_props_syncs_controlled_value() {
        let mut svc = service(bounded_props().value(10.0));

        assert_eq!(svc.context().value.get(), &Some(10.0));

        drop(svc.set_props(bounded_props().value(25.0)));

        assert!(svc.context().value.is_controlled());
        assert_eq!(svc.context().value.get(), &Some(25.0));

        drop(svc.set_props(bounded_props().uncontrolled()));

        assert!(!svc.context().value.is_controlled());
    }

    #[test]
    fn number_input_set_props_syncs_output_affecting_fields() {
        let mut svc = service(bounded_props());

        drop(
            svc.set_props(
                bounded_props()
                    .min(-50.0)
                    .max(50.0)
                    .step(5.0)
                    .precision(2)
                    .disabled(true),
            ),
        );

        assert!((svc.context().min - -50.0).abs() < f64::EPSILON);
        assert!((svc.context().max - 50.0).abs() < f64::EPSILON);
        assert!((svc.context().step - 5.0).abs() < f64::EPSILON);
        assert_eq!(svc.context().precision, Some(2));
        assert!(svc.context().disabled);
    }

    #[test]
    fn number_input_increment_to_max_no_op_when_max_infinite() {
        // Default props: max = +∞. IncrementToMax used to write +inf
        // into ctx.value, producing `"inf"` in the visible value and
        // `aria-valuenow`. The arm must no-op when max is not finite.
        let mut svc = service(props().default_value(5.0));

        let result = svc.send(Event::IncrementToMax);

        assert!(!result.context_changed);
        assert_eq!(svc.context().value.get(), &Some(5.0));
    }

    #[test]
    fn number_input_decrement_to_min_no_op_when_min_infinite() {
        let mut svc = service(props().default_value(5.0));

        let result = svc.send(Event::DecrementToMin);

        assert!(!result.context_changed);
        assert_eq!(svc.context().value.get(), &Some(5.0));
    }

    #[test]
    fn number_input_increment_to_max_still_works_when_max_finite() {
        // Sanity check: the no-op guard only kicks in for non-finite
        // bounds; finite bounds must still jump-to-max.
        let mut svc = service(bounded_props().default_value(5.0));

        drop(svc.send(Event::IncrementToMax));

        assert_eq!(svc.context().value.get(), &Some(100.0));
    }

    #[test]
    fn number_input_sync_value_rejects_non_finite_controlled_value() {
        // SyncValue is the controlled-prop sync entry point. NaN/Inf
        // would serialize as `"NaN"`/`"inf"` into the visible input
        // attribute — same defect class `Change` already guards against.
        let mut svc = service(bounded_props().value(10.0));

        assert_eq!(svc.context().value.get(), &Some(10.0));

        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            drop(svc.send(Event::SyncValue(Some(bad))));

            assert_eq!(
                svc.context().value.get(),
                &Some(10.0),
                "non-finite {bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn number_input_sync_value_accepts_finite_value_and_drops_none() {
        let mut svc = service(bounded_props().value(10.0));

        drop(svc.send(Event::SyncValue(Some(42.0))));

        assert_eq!(svc.context().value.get(), &Some(42.0));
        assert!(svc.context().value.is_controlled());

        drop(svc.send(Event::SyncValue(None)));

        assert!(!svc.context().value.is_controlled());
    }

    #[test]
    fn number_input_precision_caps_at_max_precision() {
        // Without capping, `precision = u32::MAX` would wrap to a
        // negative `i32` and `10.0_f64.powi(neg)` would produce a
        // tiny fraction, scrambling the rounding math. Verify the
        // cap clamps to MAX_PRECISION (~15 decimals).
        let mut svc = service(bounded_props().precision(u32::MAX).default_value(0.0));

        drop(svc.send(Event::SetValue(1.5)));

        // Should still produce a finite, sensible result close to 1.5
        // (1.5 itself if precision is capped where it can preserve
        // the value, or a small rounding artefact).
        let value = svc.context().value.get().expect("value set");

        assert!(value.is_finite(), "value {value} must be finite");
        assert!((value - 1.5).abs() < 1.0, "value {value} must be near 1.5");
    }

    #[test]
    fn number_input_set_props_preserves_controlled_value_when_bounds_shrink() {
        // In controlled mode the parent owns the value. SetProps must
        // NOT mutate the controlled slot even when bounds change such
        // that the current value falls outside the new range —
        // otherwise the agnostic core silently diverges from
        // `props.value` and subsequent identical-value renders won't
        // re-sync via `SyncValue` (`old.value == new.value`).
        let mut svc = service(bounded_props().value(75.0));

        assert!(svc.context().value.is_controlled());

        drop(svc.set_props(bounded_props().min(0.0).max(50.0).value(75.0)));

        // The controlled slot must still report 75.0 — the parent is
        // the source of truth. `aria-valuenow` is clamped at
        // display-time via `input_attrs`, so AT still sees a bounded
        // value (covered by the round-7
        // `*_aria_valuenow_is_clamped_*` test).
        assert!(svc.context().value.is_controlled());
        assert_eq!(svc.context().value.get(), &Some(75.0));
    }

    #[test]
    fn number_input_end_scrub_processable_even_when_disabled_or_readonly() {
        // If the parent disables the input during an active scrub
        // gesture, the adapter still fires EndScrub on pointer-up.
        // The machine must process it so state can leave Scrubbing
        // and `ctx.scrubbing` clears.
        let mut svc = service(bounded_props().default_value(10.0));

        drop(svc.send(Event::StartScrub));

        assert_eq!(svc.state(), &State::Scrubbing);

        // Parent disables mid-drag.
        drop(svc.set_props(bounded_props().default_value(10.0).disabled(true)));

        assert!(svc.context().disabled);

        // Pointer-up: adapter fires EndScrub. Must transition out of
        // Scrubbing despite the disabled guard.
        let result = svc.send(Event::EndScrub);

        assert!(result.state_changed);
        assert!(!svc.context().scrubbing);
        assert_ne!(svc.state(), &State::Scrubbing);
    }

    #[test]
    fn number_input_end_scrub_processable_when_readonly() {
        let mut svc = service(bounded_props().default_value(10.0));

        drop(svc.send(Event::StartScrub));

        drop(svc.set_props(bounded_props().default_value(10.0).readonly(true)));

        let result = svc.send(Event::EndScrub);

        assert!(result.state_changed);
        assert!(!svc.context().scrubbing);
    }

    #[test]
    fn number_input_set_props_clamps_value_when_max_shrinks_below_current() {
        // Previous bounds: [0, 100]. Value: 75. New bounds: [0, 50].
        // Without re-clamping, `aria-valuenow` would still be 75 — outside
        // the new `aria-valuemax = 50` — and Increment would still treat
        // 75 as valid. The reclamp must pull it back into range.
        let mut svc = service(bounded_props().default_value(75.0));

        assert_eq!(svc.context().value.get(), &Some(75.0));

        drop(svc.set_props(bounded_props().min(0.0).max(50.0).default_value(75.0)));

        assert_eq!(svc.context().value.get(), &Some(50.0));
    }

    #[test]
    fn number_input_set_props_clamps_value_when_min_rises_above_current() {
        let mut svc = service(bounded_props().default_value(5.0));

        drop(svc.set_props(bounded_props().min(20.0).max(100.0).default_value(5.0)));

        assert_eq!(svc.context().value.get(), &Some(20.0));
    }

    #[test]
    fn number_input_set_props_rerounds_value_when_precision_tightens() {
        // Value 2.345 with precision None preserved literally; SetProps
        // bumps precision to 1 → value must reround to 2.3.
        let mut svc = service(bounded_props().default_value(2.345));

        drop(svc.set_props(bounded_props().precision(1).default_value(2.345)));

        // round_half_up(2.345 → 1 decimal). 2.345 in f64 is 2.345000...004,
        // so the round_to_precision will give 2.3 (5 rounds up away from
        // zero, but the actual stored bits push it just barely below the
        // half-way mark; verify a numerically robust upper bound instead).
        let value = svc.context().value.get().expect("value set");

        assert!(
            (value - 2.3).abs() < f64::EPSILON || (value - 2.4).abs() < f64::EPSILON,
            "value {value} must be reround'd to 2.3 or 2.4 (precision 1)"
        );
    }

    #[test]
    fn number_input_set_props_skips_reclamp_when_bounds_unchanged() {
        // Only `name` changes — bounds/precision are stable so the
        // current value must NOT be touched (otherwise harmless prop
        // updates would silently mutate value).
        let mut svc = service(bounded_props().default_value(7.0));

        drop(svc.set_props(bounded_props().default_value(7.0).name("qty2")));

        assert_eq!(svc.context().value.get(), &Some(7.0));
    }

    #[test]
    fn number_input_blur_and_set_props_ignore_epsilon_only_value_deltas() {
        let edge = 1.0 + f64::EPSILON;

        let mut blur_svc = service(props().min(0.0).max(1.0).default_value(edge));

        drop(blur_svc.send(Event::Focus { is_keyboard: false }));
        drop(blur_svc.send(Event::Blur));

        assert_eq!(blur_svc.context().value.get(), &Some(edge));

        let mut props_svc = service(props().min(0.0).max(1.0).default_value(edge));

        drop(props_svc.set_props(props().min(0.0).max(1.0 - f64::EPSILON).default_value(edge)));

        assert_eq!(props_svc.context().value.get(), &Some(edge));
    }

    #[test]
    fn number_input_set_props_render_change_does_not_reclamp_epsilon_only_bounds() {
        let edge = 2.0 + f64::EPSILON;

        let mut svc = service(props().min(1.0).max(2.0).name("amount").default_value(edge));

        drop(
            svc.set_props(
                props()
                    .min(1.0 + f64::EPSILON)
                    .max(2.0 - f64::EPSILON)
                    .name("total")
                    .default_value(edge),
            ),
        );

        assert_eq!(svc.context().value.get(), &Some(edge));
    }

    #[test]
    fn number_input_zero_wheel_delta_is_noop() {
        let mut svc = service(bounded_props().allow_mouse_wheel(true).default_value(10.0));

        drop(svc.send(Event::Focus { is_keyboard: false }));

        let result = svc.send(Event::Wheel { delta: 0.0 });

        assert!(!result.context_changed);
        assert_eq!(svc.context().value.get(), &Some(10.0));
    }

    #[test]
    fn number_input_set_has_description_flips_context_flag_and_describedby() {
        let mut svc = service(bounded_props().default_value(5.0));

        assert!(!svc.context().has_description);

        drop(svc.send(Event::SetHasDescription(true)));

        assert!(svc.context().has_description);
        assert_eq!(
            svc.connect(&|_| {})
                .input_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("num-description")
        );
    }

    #[test]
    fn number_input_disabled_blocks_value_mutations() {
        let mut svc = service(bounded_props().default_value(10.0).disabled(true));

        for event in [
            Event::Increment,
            Event::Decrement,
            Event::IncrementLarge,
            Event::DecrementLarge,
            Event::IncrementToMax,
            Event::DecrementToMin,
            Event::SetValue(42.0),
            Event::Change("99".to_string()),
            Event::StartScrub,
            Event::Scrub(1.0),
            Event::Wheel { delta: 1.0 },
        ] {
            let result = svc.send(event);

            assert!(!result.context_changed);
        }

        assert_eq!(svc.context().value.get(), &Some(10.0));
    }

    #[test]
    fn number_input_readonly_blocks_value_mutations() {
        let mut svc = service(bounded_props().default_value(10.0).readonly(true));

        let result = svc.send(Event::Increment);

        assert!(!result.context_changed);
        assert_eq!(svc.context().value.get(), &Some(10.0));
    }

    #[test]
    fn number_input_wheel_steps_only_in_focused_state_with_allow_mouse_wheel() {
        let mut svc = service(bounded_props().default_value(10.0).allow_mouse_wheel(true));

        let idle_result = svc.send(Event::Wheel { delta: 1.0 });

        assert!(!idle_result.context_changed);

        drop(svc.send(Event::Focus { is_keyboard: false }));
        drop(svc.send(Event::Wheel { delta: 1.0 }));

        assert_eq!(svc.context().value.get(), &Some(11.0));

        drop(svc.send(Event::Wheel { delta: -1.0 }));

        assert_eq!(svc.context().value.get(), &Some(10.0));
    }

    #[test]
    fn number_input_wheel_disabled_when_allow_mouse_wheel_false() {
        let mut svc = service(bounded_props().default_value(10.0).allow_mouse_wheel(false));

        drop(svc.send(Event::Focus { is_keyboard: true }));

        let result = svc.send(Event::Wheel { delta: 1.0 });

        assert!(!result.context_changed);
    }

    #[test]
    fn number_input_input_attrs_carries_spinbutton_aria_value_now_min_max() {
        let svc = service(bounded_props().default_value(42.0));

        let api = svc.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("spinbutton"));
        assert_eq!(attrs.get(&HtmlAttr::InputMode), Some("decimal"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("42"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("100"));
    }

    #[test]
    fn number_input_format_option_builders_round_trip() {
        let p = props()
            .format_options(fmt_opts())
            .display_format(FormatOptions {
                min_fraction_digits: 2,
                max_fraction_digits: 2,
                ..FormatOptions::default()
            });

        assert_eq!(p.format_options, Some(fmt_opts()));
        assert_eq!(
            p.display_format
                .as_ref()
                .map(|options| options.min_fraction_digits),
            Some(2)
        );
    }

    #[test]
    fn number_input_static_attrs_and_boundary_helpers_are_observable() {
        let min_svc = service(bounded_props().default_value(0.0));

        let min_api = min_svc.connect(&|_| {});

        assert_eq!(min_api.label_attrs().get(&HtmlAttr::Id), Some("num-label"));
        assert_eq!(min_api.label_attrs().get(&HtmlAttr::For), Some("num-input"));
        assert_eq!(
            min_api.description_attrs().get(&HtmlAttr::Id),
            Some("num-description")
        );
        assert_eq!(
            min_api.error_message_attrs().get(&HtmlAttr::Id),
            Some("num-error-message")
        );
        assert_eq!(
            min_api
                .error_message_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Live)),
            Some("polite")
        );
        assert!(
            min_api
                .decrement_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );
        assert!(
            !min_api
                .increment_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );

        let disabled_svc = service(bounded_props().default_value(50.0).disabled(true));
        let disabled_api = disabled_svc.connect(&|_| {});

        assert!(
            disabled_api
                .increment_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );

        let readonly_svc = service(bounded_props().default_value(50.0).readonly(true));
        let readonly_api = readonly_svc.connect(&|_| {});

        assert!(
            readonly_api
                .decrement_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );

        let max_svc = service(bounded_props().default_value(100.0));
        let max_api = max_svc.connect(&|_| {});

        assert!(
            max_api
                .increment_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );
        assert!(
            !max_api
                .decrement_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );
    }

    #[test]
    fn number_input_input_attrs_omits_infinite_bounds() {
        let svc = service(props().default_value(0.0));

        let api = svc.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), None);
    }

    #[test]
    fn number_input_increment_trigger_disabled_at_max() {
        let svc = service(bounded_props().default_value(100.0));

        let api = svc.connect(&|_| {});

        let attrs = api.increment_trigger_attrs();

        assert!(attrs.contains(&HtmlAttr::Disabled));
    }

    #[test]
    fn number_input_decrement_trigger_disabled_at_min() {
        let svc = service(bounded_props().default_value(0.0));

        let api = svc.connect(&|_| {});

        let attrs = api.decrement_trigger_attrs();

        assert!(attrs.contains(&HtmlAttr::Disabled));
    }

    #[test]
    fn number_input_form_carries_name_and_form_id_via_input() {
        let svc = service(
            bounded_props()
                .default_value(7.0)
                .name("qty")
                .form("checkout"),
        );

        let api = svc.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Name), Some("qty"));
        assert_eq!(attrs.get(&HtmlAttr::Form), Some("checkout"));
    }

    #[test]
    fn number_input_keydown_emits_arrow_and_page_and_home_end() {
        let received = core::cell::RefCell::new(Vec::<Event>::new());
        let send = |event: Event| {
            received.borrow_mut().push(event);
        };

        let svc = service(bounded_props());

        let api = svc.connect(&send);

        for key in [
            KeyboardKey::ArrowUp,
            KeyboardKey::ArrowDown,
            KeyboardKey::PageUp,
            KeyboardKey::PageDown,
            KeyboardKey::Home,
            KeyboardKey::End,
        ] {
            let data = keyboard_event(key, false);

            assert!(api.on_input_keydown(&data));
        }

        let events = received.borrow();

        assert_eq!(events.len(), 6);
        assert!(matches!(events[0], Event::Increment));
        assert!(matches!(events[1], Event::Decrement));
        assert!(matches!(events[2], Event::IncrementLarge));
        assert!(matches!(events[3], Event::DecrementLarge));
        assert!(matches!(events[4], Event::DecrementToMin));
        assert!(matches!(events[5], Event::IncrementToMax));
    }

    #[test]
    fn number_input_keydown_ignores_unhandled_keys_and_composition() {
        let received = core::cell::RefCell::new(Vec::<Event>::new());
        let send = |event: Event| {
            received.borrow_mut().push(event);
        };

        let svc = service(bounded_props());

        let api = svc.connect(&send);

        let escape = keyboard_event(KeyboardKey::Escape, false);

        assert!(!api.on_input_keydown(&escape));

        let composing = keyboard_event(KeyboardKey::ArrowUp, true);

        assert!(!api.on_input_keydown(&composing));

        assert!(received.borrow().is_empty());
    }

    #[test]
    fn number_input_composition_lifecycle_tracks_is_composing() {
        let mut svc = service(bounded_props());

        drop(svc.send(Event::CompositionStart));

        assert!(svc.context().is_composing);

        drop(svc.send(Event::CompositionEnd));

        assert!(!svc.context().is_composing);
    }

    #[test]
    fn number_input_part_attrs_delegates_to_each_part_method() {
        let svc = service(bounded_props().default_value(5.0));

        let api = svc.connect(&|_| {});

        for (part, expected) in [
            (Part::Root, snapshot_attrs(&api.root_attrs())),
            (Part::Label, snapshot_attrs(&api.label_attrs())),
            (Part::Input, snapshot_attrs(&api.input_attrs())),
            (
                Part::IncrementTrigger,
                snapshot_attrs(&api.increment_trigger_attrs()),
            ),
            (
                Part::DecrementTrigger,
                snapshot_attrs(&api.decrement_trigger_attrs()),
            ),
            (Part::Description, snapshot_attrs(&api.description_attrs())),
            (
                Part::ErrorMessage,
                snapshot_attrs(&api.error_message_attrs()),
            ),
        ] {
            assert_eq!(snapshot_attrs(&api.part_attrs(part)), expected);
        }
    }

    #[test]
    fn number_input_event_handlers_fan_out_through_send() {
        let received = core::cell::RefCell::new(Vec::<Event>::new());
        let send = |event: Event| {
            received.borrow_mut().push(event);
        };

        let svc = service(bounded_props());

        let api = svc.connect(&send);

        api.on_input_focus(true);
        api.on_input_blur();
        api.on_input_change("9".to_string());
        api.on_increment_click();
        api.on_decrement_click();

        let events = received.borrow();

        assert_eq!(events.len(), 5);
        assert_eq!(events[0], Event::Focus { is_keyboard: true });
        assert_eq!(events[1], Event::Blur);
        assert_eq!(events[2], Event::Change("9".to_string()));
        assert_eq!(events[3], Event::Increment);
        assert_eq!(events[4], Event::Decrement);
    }

    #[test]
    fn number_input_root_idle_snapshot() {
        let svc = service(bounded_props().default_value(5.0));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn number_input_root_focused_snapshot() {
        let mut svc = service(bounded_props().default_value(5.0));

        drop(svc.send(Event::Focus { is_keyboard: true }));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn number_input_root_scrubbing_snapshot() {
        let mut svc = service(bounded_props().default_value(5.0));

        drop(svc.send(Event::StartScrub));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn number_input_input_default_snapshot() {
        let svc = service(bounded_props().default_value(5.0));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs()));
    }

    #[test]
    fn number_input_input_at_max_snapshot() {
        let svc = service(bounded_props().default_value(100.0));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs()));
    }

    #[test]
    fn number_input_input_invalid_required_with_form_snapshot() {
        let svc = service(
            bounded_props()
                .default_value(7.0)
                .invalid(true)
                .required(true)
                .name("qty")
                .form("checkout"),
        );

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs()));
    }

    #[test]
    fn number_input_increment_trigger_default_snapshot() {
        let svc = service(bounded_props().default_value(5.0));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.increment_trigger_attrs()));
    }

    #[test]
    fn number_input_increment_trigger_at_max_snapshot() {
        let svc = service(bounded_props().default_value(100.0));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.increment_trigger_attrs()));
    }

    #[test]
    fn number_input_decrement_trigger_at_min_snapshot() {
        let svc = service(bounded_props().default_value(0.0));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.decrement_trigger_attrs()));
    }
}
