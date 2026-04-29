#[cfg(not(feature = "ssr"))]
use std::sync::{Arc, Mutex};

#[cfg(not(feature = "ssr"))]
use ars_core::PendingEffect;
use ars_core::{
    AriaAttr, AttrMap, ComponentPart, ConnectApi, Env, HasId, HtmlAttr, Machine, TransitionPlan,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToggleState {
    Off,
    On,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToggleEvent {
    Toggle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ToggleContext;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ToggleProps {
    pub(super) id: String,
}

impl HasId for ToggleProps {
    fn id(&self) -> &str {
        &self.id
    }

    fn with_id(self, id: String) -> Self {
        Self { id }
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct TogglePart;

impl ComponentPart for TogglePart {
    const ROOT: Self = Self;

    fn scope() -> &'static str {
        "toggle"
    }

    fn name(&self) -> &'static str {
        "root"
    }

    fn all() -> Vec<Self> {
        vec![Self]
    }
}

pub(super) struct ToggleApi<'a> {
    pub(super) is_on: bool,
    pub(super) send: &'a dyn Fn(ToggleEvent),
}

impl ToggleApi<'_> {
    pub(super) fn is_on(&self) -> bool {
        self.is_on
    }

    pub(super) fn trigger_toggle(&self) {
        (self.send)(ToggleEvent::Toggle);
    }
}

impl ConnectApi for ToggleApi<'_> {
    type Part = TogglePart;

    fn part_attrs(&self, _part: Self::Part) -> AttrMap {
        let mut attrs = AttrMap::new();

        attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), self.is_on.to_string());

        attrs
    }
}

pub(super) struct ToggleMachine;

impl Machine for ToggleMachine {
    type State = ToggleState;
    type Event = ToggleEvent;
    type Context = ToggleContext;
    type Props = ToggleProps;
    type Messages = ();
    type Api<'a> = ToggleApi<'a>;

    fn init(
        _props: &Self::Props,
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (ToggleState::Off, ToggleContext)
    }

    fn transition(
        state: &Self::State,
        _event: &Self::Event,
        _context: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match state {
            ToggleState::Off => Some(TransitionPlan::to(ToggleState::On)),
            ToggleState::On => Some(TransitionPlan::to(ToggleState::Off)),
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        _context: &'a Self::Context,
        _props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        ToggleApi {
            is_on: *state == ToggleState::On,
            send,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PropState {
    Off,
    On,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PropEvent {
    SetChecked(bool),
    SyncLabel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PropContext {
    pub(super) sync_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PropProps {
    pub(super) id: String,
    pub(super) checked: bool,
    pub(super) label: &'static str,
}

impl HasId for PropProps {
    fn id(&self) -> &str {
        &self.id
    }

    fn with_id(self, id: String) -> Self {
        Self { id, ..self }
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

pub(super) struct PropMachine;

impl Machine for PropMachine {
    type State = PropState;
    type Event = PropEvent;
    type Context = PropContext;
    type Props = PropProps;
    type Messages = ();
    type Api<'a> = PropApi;

    fn init(
        props: &Self::Props,
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            if props.checked {
                PropState::On
            } else {
                PropState::Off
            },
            PropContext { sync_count: 0 },
        )
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        _context: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            PropEvent::SetChecked(checked) => Some(TransitionPlan::to(if *checked {
                PropState::On
            } else {
                PropState::Off
            })),
            PropEvent::SyncLabel => Some(TransitionPlan::new().apply(|ctx: &mut PropContext| {
                ctx.sync_count += 1;
            })),
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        context: &'a Self::Context,
        _props: &'a Self::Props,
        _send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        PropApi {
            is_on: *state == PropState::On,
            sync_count: context.sync_count,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "PropMachine id cannot change after initialization"
        );

        let mut events = Vec::new();

        if old.checked != new.checked {
            events.push(PropEvent::SetChecked(new.checked));
        }

        if old.label != new.label {
            events.push(PropEvent::SyncLabel);
        }

        events
    }
}

pub(super) struct PropApi {
    pub(super) is_on: bool,
    pub(super) sync_count: u32,
}

impl PropApi {
    pub(super) const fn sync_count(&self) -> u32 {
        self.sync_count
    }
}

impl ConnectApi for PropApi {
    type Part = TogglePart;

    fn part_attrs(&self, _part: Self::Part) -> AttrMap {
        let mut attrs = AttrMap::new();

        attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), self.is_on.to_string());

        attrs
    }
}

#[cfg(not(feature = "ssr"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EffectState {
    Idle,
    Active,
}

#[cfg(not(feature = "ssr"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EffectEvent {
    Start,
    Replace,
    Cancel,
    Stop,
    StartNotify,
    Notify,
}

#[cfg(not(feature = "ssr"))]
#[derive(Clone, Debug)]
pub(super) struct EffectContext {
    pub(super) log: Arc<Mutex<Vec<&'static str>>>,
    pub(super) notify_count: u32,
}

#[cfg(not(feature = "ssr"))]
#[derive(Clone, Debug)]
pub(super) struct EffectProps {
    pub(super) id: String,
    pub(super) log: Arc<Mutex<Vec<&'static str>>>,
}

#[cfg(not(feature = "ssr"))]
impl PartialEq for EffectProps {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[cfg(not(feature = "ssr"))]
impl Eq for EffectProps {}

#[cfg(not(feature = "ssr"))]
impl HasId for EffectProps {
    fn id(&self) -> &str {
        &self.id
    }

    fn with_id(self, id: String) -> Self {
        Self { id, ..self }
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

#[cfg(not(feature = "ssr"))]
pub(super) struct EffectApi;

#[cfg(not(feature = "ssr"))]
impl ConnectApi for EffectApi {
    type Part = TogglePart;

    fn part_attrs(&self, _part: Self::Part) -> AttrMap {
        AttrMap::new()
    }
}

#[cfg(not(feature = "ssr"))]
pub(super) struct EffectMachine;

#[cfg(not(feature = "ssr"))]
impl Machine for EffectMachine {
    type State = EffectState;
    type Event = EffectEvent;
    type Context = EffectContext;
    type Props = EffectProps;
    type Messages = ();
    type Api<'a> = EffectApi;

    fn init(
        props: &Self::Props,
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            EffectState::Idle,
            EffectContext {
                log: Arc::clone(&props.log),
                notify_count: 0,
            },
        )
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        _context: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            EffectEvent::Start => Some(
                TransitionPlan::to(EffectState::Active).with_effect(tracked_effect(
                    "timer",
                    "setup:start",
                    "cleanup:start",
                )),
            ),

            EffectEvent::Replace => Some(TransitionPlan::new().with_effect(tracked_effect(
                "timer",
                "setup:replace",
                "cleanup:replace",
            ))),

            EffectEvent::Cancel => Some(TransitionPlan::new().cancel_effect("timer")),

            EffectEvent::Stop => Some(TransitionPlan::to(EffectState::Idle)),

            EffectEvent::StartNotify => Some(TransitionPlan::to(EffectState::Active).with_effect(
                PendingEffect::new(
                    "notify",
                    |ctx: &EffectContext, _props: &EffectProps, send| {
                        ctx.log
                            .lock()
                            .expect("log mutex should not be poisoned")
                            .push("setup:notify");

                        send.call_if_alive(EffectEvent::Notify);

                        let log = Arc::clone(&ctx.log);

                        Box::new(move || {
                            log.lock()
                                .expect("log mutex should not be poisoned")
                                .push("cleanup:notify");
                        })
                    },
                ),
            )),

            EffectEvent::Notify => Some(TransitionPlan::new().apply(|ctx: &mut EffectContext| {
                ctx.notify_count += 1;
            })),
        }
    }

    fn connect<'a>(
        _state: &'a Self::State,
        _context: &'a Self::Context,
        _props: &'a Self::Props,
        _send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        EffectApi
    }
}

#[cfg(not(feature = "ssr"))]
pub(super) fn tracked_effect(
    name: &'static str,
    setup_label: &'static str,
    cleanup_label: &'static str,
) -> PendingEffect<EffectMachine> {
    PendingEffect::new(
        name,
        move |ctx: &EffectContext, _props: &EffectProps, _send| {
            ctx.log
                .lock()
                .expect("log mutex should not be poisoned")
                .push(setup_label);

            let log = Arc::clone(&ctx.log);

            Box::new(move || {
                log.lock()
                    .expect("log mutex should not be poisoned")
                    .push(cleanup_label);
            })
        },
    )
}

#[cfg(not(feature = "ssr"))]
pub(super) fn effect_log(log: &Arc<Mutex<Vec<&'static str>>>) -> Vec<&'static str> {
    log.lock()
        .expect("log mutex should not be poisoned")
        .clone()
}
