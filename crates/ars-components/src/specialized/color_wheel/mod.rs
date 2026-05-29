//! `ColorWheel` component state machine and connect API.
//!
//! `ColorWheel` is a circular hue picker: the thumb angle around the ring maps
//! to the hue channel. It owns the hue/angle math, value state, keyboard
//! behavior, and ARIA/data attributes. Live wheel measurement, pointer capture,
//! and the `atan2` angle conversion are adapter concerns: the adapter supplies
//! an already-normalized angle in `0..=1` via [`Api::on_track_pointer_down`]
//! (drag start) and drives [`Event::DragMove`] / [`Event::DragEnd`] from its own
//! pointer listeners. Circular geometry is direction-agnostic, so arrow keys are
//! not mirrored for RTL.

use alloc::{format, string::String};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ColorValue, ComponentIds, ComponentMessages,
    ComponentPart, ConnectApi, CssProperty, Direction, Env, HtmlAttr, KeyboardKey, Locale,
    MessageFn, PendingEffect, TransitionPlan, no_cleanup,
};
use ars_interactions::KeyboardEventData;

/// Label for the wheel thumb.
type LabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// Formats the hue value for `aria-valuetext`.
type ValueTextFn = dyn Fn(f64, &Locale) -> String + Send + Sync;

/// Consumer callback fired on drag-end / pointer release.
type ChangeEndFn = dyn Fn(ColorValue) + Send + Sync;

/// The states for the `ColorWheel` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// No interaction in progress.
    Idle,

    /// The user is dragging the thumb around the ring.
    Dragging,
}

/// The events for the `ColorWheel` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The user started dragging (normalized angle `0..=1` around the ring).
    DragStart {
        /// Normalized ring angle (`0..=1`).
        position: f64,
    },

    /// The user is moving while dragging.
    DragMove {
        /// Normalized ring angle (`0..=1`).
        position: f64,
    },

    /// The user released the drag.
    DragEnd,

    /// Increment the hue by `step` degrees.
    Increment {
        /// The step in degrees.
        step: f64,
    },

    /// Decrement the hue by `step` degrees.
    Decrement {
        /// The step in degrees.
        step: f64,
    },

    /// Snap the hue to its minimum (0°).
    SetToMin,

    /// Snap the hue to its maximum (360°).
    SetToMax,

    /// Focus entered the thumb.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// Focus left the thumb.
    Blur,
}

/// Typed identifier for side effects emitted by the `ColorWheel` machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Invoke `Props::on_change_end`.
    ChangeEnd,
}

/// The context for the `ColorWheel` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The current color value (controlled or uncontrolled).
    pub value: Bindable<ColorValue>,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Whether the thumb is focused.
    pub focused: bool,

    /// Whether focus was via keyboard (for the focus-visible ring).
    pub focus_visible: bool,

    /// Step size in degrees for keyboard adjustment. Default: `1.0`.
    pub step: f64,

    /// Large step size in degrees. Default: `10.0`.
    pub large_step: f64,

    /// Text direction. Retained for parity with other color controls; circular
    /// geometry is direction-agnostic, so arrow keys are not mirrored.
    pub dir: Direction,

    /// Locale for internationalized messages.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component instance IDs.
    pub ids: ComponentIds,
}

/// The props for the `ColorWheel` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<ColorValue>,

    /// Default value for uncontrolled mode.
    pub default_value: ColorValue,

    /// Step size in degrees for keyboard adjustment. Default: `1.0`.
    pub step: f64,

    /// Large step size in degrees. Default: `10.0`.
    pub large_step: f64,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Text direction (retained for parity; circular geometry is direction-agnostic).
    pub dir: Direction,

    /// Name attribute for the hidden form input.
    pub name: Option<String>,

    /// Fired on `Event::DragEnd` / pointer release.
    pub on_change_end: Option<Callback<ChangeEndFn>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: ColorValue::default(),
            step: 1.0,
            large_step: 10.0,
            disabled: false,
            readonly: false,
            dir: Direction::Ltr,
            name: None,
            on_change_end: None,
        }
    }
}

/// The messages for the `ColorWheel` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// The label for the color wheel. Default: `"Hue"`.
    pub label: MessageFn<LabelFn>,

    /// The value text for the color wheel. Default: `"180°"`.
    pub value_text: MessageFn<ValueTextFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Hue"),
            value_text: MessageFn::new(|hue: f64, _locale: &Locale| format!("{hue:.0}\u{00b0}")),
        }
    }
}

impl ComponentMessages for Messages {}

/// Apply a normalized angle (`0..=1`) to the hue value.
fn apply_wheel_angle(ctx: &mut Context, angle: f64) {
    let hue = (angle.clamp(0.0, 1.0) * 360.0) % 360.0;

    let color = *ctx.value.get();

    ctx.value.set(ColorValue { hue, ..color });
}

/// Build the change-end effect that invokes `Props::on_change_end`.
fn change_end_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::ChangeEnd, |ctx: &Context, props: &Props, _send| {
        if let Some(callback) = &props.on_change_end {
            callback(*ctx.value.get());
        }

        no_cleanup()
    })
}

/// The machine for the `ColorWheel` component.
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
        let value = if let Some(v) = &props.value {
            Bindable::controlled(*v)
        } else {
            Bindable::uncontrolled(props.default_value)
        };

        let context = Context {
            value,
            disabled: props.disabled,
            readonly: props.readonly,
            focused: false,
            focus_visible: false,
            step: props.step,
            large_step: props.large_step,
            dir: props.dir,
            locale: env.locale.clone(),
            messages: messages.clone(),
            ids: ComponentIds::from_id(&props.id),
        };

        (State::Idle, context)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // Focus/Blur always pass through regardless of disabled/readonly.
        match event {
            Event::Focus { is_keyboard } => {
                let ik = *is_keyboard;
                return Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = ik;
                }));
            }

            Event::Blur => {
                return Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }));
            }

            _ => {}
        }

        // Disabled and read-only both block value-changing events.
        if ctx.disabled || ctx.readonly {
            return None;
        }

        match (state, event) {
            (State::Idle, Event::DragStart { position }) => {
                let pos = *position;
                Some(
                    TransitionPlan::to(State::Dragging).apply(move |ctx: &mut Context| {
                        apply_wheel_angle(ctx, pos);
                    }),
                )
            }

            (State::Dragging, Event::DragMove { position }) => {
                let pos = *position;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    apply_wheel_angle(ctx, pos);
                }))
            }

            (State::Dragging, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Idle).with_effect(change_end_effect()))
            }

            (_, Event::Increment { step }) => {
                let step_degrees = *step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let color = *ctx.value.get();
                    let new_hue = (color.hue + step_degrees) % 360.0;
                    ctx.value.set(ColorValue {
                        hue: new_hue,
                        ..color
                    });
                }))
            }

            (_, Event::Decrement { step }) => {
                let step_degrees = *step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let color = *ctx.value.get();
                    let new_hue = (color.hue - step_degrees + 360.0) % 360.0;
                    ctx.value.set(ColorValue {
                        hue: new_hue,
                        ..color
                    });
                }))
            }

            (_, Event::SetToMin) => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                let color = *ctx.value.get();
                ctx.value.set(ColorValue { hue: 0.0, ..color });
            })),

            (_, Event::SetToMax) => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                let color = *ctx.value.get();
                ctx.value.set(ColorValue {
                    hue: 360.0,
                    ..color
                });
            })),

            _ => None,
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
}

/// Structural parts exposed by the `ColorWheel` connect API.
#[derive(ComponentPart)]
#[scope = "color-wheel"]
pub enum Part {
    /// Container with `role="group"`.
    Root,

    /// Conic-gradient ring track.
    Track,

    /// Draggable thumb with `role="slider"`.
    Thumb,

    /// `type="hidden"` input that submits the hex value for forms.
    HiddenInput,
}

/// The connect API for the `ColorWheel` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("color_wheel::Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Whether the thumb is currently being dragged.
    #[must_use]
    pub const fn is_dragging(&self) -> bool {
        matches!(self.state, State::Dragging)
    }

    /// The current color value.
    #[must_use]
    pub fn value(&self) -> &ColorValue {
        self.ctx.value.get()
    }

    /// The current hue formatted for display.
    #[must_use]
    pub fn formatted_value(&self) -> String {
        (self.ctx.messages.value_text)(self.ctx.value.get().hue, &self.ctx.locale)
    }

    /// Attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id().to_string())
            .set(HtmlAttr::Role, "group");

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }

        attrs
    }

    /// Attributes for the conic-gradient ring track.
    #[must_use]
    pub fn track_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Track.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set_style(
                CssProperty::Custom("ars-color-wheel-track-bg"),
                "conic-gradient(hsl(0,100%,50%), hsl(60,100%,50%), hsl(120,100%,50%), \
             hsl(180,100%,50%), hsl(240,100%,50%), hsl(300,100%,50%), hsl(360,100%,50%))",
            );

        attrs
    }

    /// Attributes for the draggable thumb element.
    #[must_use]
    pub fn thumb_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Thumb.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("thumb"))
            .set(HtmlAttr::Role, "slider")
            .set(HtmlAttr::TabIndex, "0");

        let hue = self.ctx.value.get().hue;

        attrs
            .set(HtmlAttr::Aria(AriaAttr::ValueNow), format!("{hue:.0}"))
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), "0")
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), "360")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.label)(&self.ctx.locale),
            )
            .set(HtmlAttr::Aria(AriaAttr::ValueText), self.formatted_value())
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            )
            .set_style(
                CssProperty::Custom("ars-color-wheel-thumb-angle"),
                format!("{hue}deg"),
            );

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }

        attrs
    }

    /// Attributes for the hidden form input.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "hidden");

        if let Some(name) = &self.props.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        attrs.set(HtmlAttr::Value, self.ctx.value.get().to_hex(true));

        attrs
    }

    /// Handles a keydown on the thumb. `shift` selects the large step.
    ///
    /// Arrow keys are not mirrored for RTL — the wheel is circular and
    /// direction-agnostic.
    pub fn on_thumb_keydown(&self, data: &KeyboardEventData, shift: bool) {
        let step = if shift {
            self.ctx.large_step
        } else {
            self.ctx.step
        };

        match data.key {
            KeyboardKey::ArrowRight | KeyboardKey::ArrowUp => {
                (self.send)(Event::Increment { step });
            }

            KeyboardKey::ArrowLeft | KeyboardKey::ArrowDown => {
                (self.send)(Event::Decrement { step });
            }

            KeyboardKey::Home => (self.send)(Event::SetToMin),

            KeyboardKey::End => (self.send)(Event::SetToMax),

            KeyboardKey::PageUp => (self.send)(Event::Increment {
                step: self.ctx.large_step,
            }),

            KeyboardKey::PageDown => (self.send)(Event::Decrement {
                step: self.ctx.large_step,
            }),

            _ => {}
        }
    }

    /// Dispatches a drag-start from an adapter-resolved normalized angle.
    pub fn on_track_pointer_down(&self, angle: f64) {
        (self.send)(Event::DragStart { position: angle });
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Track => self.track_attrs(),
            Part::Thumb => self.thumb_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use ars_core::Service;
    use insta::assert_snapshot;

    use super::*;

    fn service(mut props: Props) -> Service<Machine> {
        if props.id.is_empty() {
            props.id = "color-wheel".to_string();
        }

        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn key(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character: None,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
        }
    }

    #[test]
    fn circular_hue_selection_maps_angle_to_hue() {
        let mut svc = service(Props::default());

        drop(svc.send(Event::DragStart { position: 0.5 }));

        assert!((svc.connect(&|_| {}).value().hue - 180.0).abs() < 1e-9);
    }

    #[test]
    fn pointer_drag_around_ring_updates_hue_and_state() {
        let mut svc = service(Props::default());

        drop(svc.send(Event::DragStart { position: 0.0 }));

        assert_eq!(svc.state(), &State::Dragging);

        drop(svc.send(Event::DragMove { position: 0.25 }));

        assert!((svc.connect(&|_| {}).value().hue - 90.0).abs() < 1e-9);

        drop(svc.send(Event::DragEnd));

        assert_eq!(svc.state(), &State::Idle);
    }

    #[test]
    fn keyboard_rotates_hue_with_wraparound() {
        let mut svc = service(Props {
            default_value: ColorValue::from_hsl(0.0, 1.0, 0.5),
            ..Props::default()
        });

        drop(svc.send(Event::Decrement { step: 1.0 }));

        // 0 - 1 wraps to 359.
        assert!((svc.connect(&|_| {}).value().hue - 359.0).abs() < 1e-9);

        drop(svc.send(Event::Increment { step: 1.0 }));

        assert!((svc.connect(&|_| {}).value().hue - 0.0).abs() < 1e-9);
    }

    #[test]
    fn thumb_is_slider_with_hue_valuetext() {
        let svc = service(Props {
            default_value: ColorValue::from_hsl(180.0, 1.0, 0.5),
            ..Props::default()
        });

        let thumb = svc.connect(&|_| {}).thumb_attrs();

        assert_eq!(thumb.get(&HtmlAttr::Role), Some("slider"));
        assert_eq!(thumb.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("180"));
        assert_eq!(thumb.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
        assert_eq!(thumb.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("360"));
        assert_eq!(
            thumb.get(&HtmlAttr::Aria(AriaAttr::ValueText)),
            Some("180°")
        );
        // No aria-orientation on a circular control.
        assert!(!thumb.contains(&HtmlAttr::Aria(AriaAttr::Orientation)));
    }

    #[test]
    fn keydown_dispatches_increment_decrement() {
        let captured = core::cell::RefCell::new(Vec::new());
        let send = |event: Event| captured.borrow_mut().push(event);

        let svc = service(Props::default());

        let api = svc.connect(&send);

        api.on_thumb_keydown(&key(KeyboardKey::ArrowRight), false);
        api.on_thumb_keydown(&key(KeyboardKey::Home), false);

        let events = captured.borrow();

        assert!(matches!(events[0], Event::Increment { .. }));
        assert!(matches!(events[1], Event::SetToMin));
    }

    #[test]
    fn change_end_effect_fires_callback() {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicBool, Ordering};

        use ars_core::{StrongSend, callback};

        let fired = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&fired);
        let mut svc = service(Props {
            on_change_end: Some(callback(move |_c: ColorValue| {
                flag.store(true, Ordering::SeqCst);
            })),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { position: 0.5 }));

        let mut end = svc.send(Event::DragEnd);

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in end.pending_effects.drain(..) {
            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn disabled_blocks_value_but_allows_focus() {
        let mut svc = service(Props {
            disabled: true,
            default_value: ColorValue::from_hsl(0.0, 1.0, 0.5),
            ..Props::default()
        });

        drop(svc.send(Event::Increment { step: 10.0 }));

        assert!((svc.connect(&|_| {}).value().hue - 0.0).abs() < 1e-9);

        drop(svc.send(Event::Focus { is_keyboard: true }));

        assert!(
            svc.connect(&|_| {})
                .thumb_attrs()
                .contains(&HtmlAttr::Data("ars-focus-visible"))
        );
    }

    #[test]
    fn track_uses_conic_gradient() {
        let svc = service(Props::default());

        let track = svc.connect(&|_| {}).track_attrs();

        let bg = track
            .styles()
            .iter()
            .find(|(p, _)| *p == CssProperty::Custom("ars-color-wheel-track-bg"))
            .map(|(_, v)| v.clone())
            .unwrap();

        assert!(bg.starts_with("conic-gradient("));
    }

    #[test]
    fn thumb_snapshot() {
        let svc = service(Props {
            id: "cw".to_string(),
            default_value: ColorValue::from_hsl(270.0, 1.0, 0.5),
            ..Props::default()
        });

        assert_snapshot!(
            "color_wheel_thumb",
            snapshot_attrs(&svc.connect(&|_| {}).thumb_attrs())
        );
    }

    #[test]
    fn root_dragging_snapshot() {
        let mut svc = service(Props {
            id: "cw".to_string(),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { position: 0.25 }));

        assert_snapshot!(
            "color_wheel_root_dragging",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn exhaustive_events_parts_and_helpers() {
        let mut svc = Service::<Machine>::new(
            Props {
                id: "cw".into(),
                value: Some(ColorValue::from_hsl(30.0, 1.0, 0.5)),
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        for ev in [
            Event::Focus { is_keyboard: true },
            Event::DragStart { position: 0.1 },
            Event::DragMove { position: 0.8 },
            Event::DragEnd,
            Event::Increment { step: 5.0 },
            Event::Decrement { step: 5.0 },
            Event::SetToMin,
            Event::SetToMax,
            Event::Blur,
        ] {
            drop(svc.send(ev));
        }

        let api = svc.connect(&|_| {});

        for p in [Part::Root, Part::Track, Part::Thumb, Part::HiddenInput] {
            let _attrs = api.part_attrs(p);
        }

        let _dbg = alloc::format!("{api:?}");

        // Disabled blocks value but allows focus/blur; readonly blocks value.
        let mut dis = Service::<Machine>::new(
            Props {
                id: "cw".into(),
                disabled: true,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(dis.send(Event::Increment { step: 5.0 }));
        drop(dis.send(Event::Blur));

        let mut ro = Service::<Machine>::new(
            Props {
                id: "cw".into(),
                readonly: true,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(ro.send(Event::DragStart { position: 0.5 }));

        // Track-pointer-down dispatch.
        let cap = core::cell::RefCell::new(Vec::new());
        let send = |event: Event| cap.borrow_mut().push(event);

        svc.connect(&send).on_track_pointer_down(0.6);

        assert!(matches!(cap.borrow()[0], Event::DragStart { .. }));
    }
}
