#[cfg(not(feature = "ssr"))]
use std::sync::{Arc, Mutex};

#[cfg(not(feature = "ssr"))]
use ars_core::PendingEffect;
use ars_core::{AriaAttr, AttrMap, ComponentPart, ConnectApi, Env, HasId, HtmlAttr, Machine};

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

pub(super) struct ToggleApi {
    pub(super) is_on: bool,
}

impl ConnectApi for ToggleApi {
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
    type Api<'a> = ToggleApi;

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
    ) -> Option<ars_core::TransitionPlan<Self>> {
        match state {
            ToggleState::Off => Some(ars_core::TransitionPlan::to(ToggleState::On)),
            ToggleState::On => Some(ars_core::TransitionPlan::to(ToggleState::Off)),
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        _context: &'a Self::Context,
        _props: &'a Self::Props,
        _send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        ToggleApi {
            is_on: *state == ToggleState::On,
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

pub(super) struct PropApi {
    pub(super) is_on: bool,
    pub(super) sync_count: u32,
}

impl PropApi {
    #[cfg_attr(
        not(all(test, feature = "web", target_arch = "wasm32")),
        expect(
            dead_code,
            reason = "Only the wasm render tests snapshot derived prop state."
        )
    )]
    pub(super) const fn snapshot(&self) -> (bool, u32) {
        (self.is_on, self.sync_count)
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
    ) -> Option<ars_core::TransitionPlan<Self>> {
        match event {
            PropEvent::SetChecked(checked) => Some(ars_core::TransitionPlan::to(if *checked {
                PropState::On
            } else {
                PropState::Off
            })),

            PropEvent::SyncLabel => Some(ars_core::TransitionPlan::new().apply(
                |ctx: &mut PropContext| {
                    ctx.sync_count += 1;
                },
            )),
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

#[cfg_attr(
    not(all(test, feature = "web", target_arch = "wasm32")),
    expect(
        dead_code,
        reason = "Only the wasm render tests use the prop snapshot alias."
    )
)]
pub(super) type PropSnapshot = ((bool, u32), PropState, u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DerivedState {
    Off,
    On,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DerivedEvent {
    Toggle,
    BumpContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DerivedContext {
    pub(super) count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DerivedProps {
    pub(super) id: String,
}

impl HasId for DerivedProps {
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

pub(super) struct DerivedApi {
    pub(super) is_on: bool,
    pub(super) count: u32,
}

impl ConnectApi for DerivedApi {
    type Part = TogglePart;

    fn part_attrs(&self, _part: Self::Part) -> AttrMap {
        AttrMap::new()
    }
}

pub(super) struct DerivedMachine;

impl Machine for DerivedMachine {
    type State = DerivedState;
    type Event = DerivedEvent;
    type Context = DerivedContext;
    type Props = DerivedProps;
    type Messages = ();
    type Api<'a> = DerivedApi;

    fn init(
        _props: &Self::Props,
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (DerivedState::Off, DerivedContext { count: 0 })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        _context: &Self::Context,
        _props: &Self::Props,
    ) -> Option<ars_core::TransitionPlan<Self>> {
        match event {
            DerivedEvent::Toggle => Some(ars_core::TransitionPlan::to(match state {
                DerivedState::Off => DerivedState::On,
                DerivedState::On => DerivedState::Off,
            })),

            DerivedEvent::BumpContext => Some(ars_core::TransitionPlan::new().apply(
                |ctx: &mut DerivedContext| {
                    ctx.count += 1;
                },
            )),
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        context: &'a Self::Context,
        _props: &'a Self::Props,
        _send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        DerivedApi {
            is_on: *state == DerivedState::On,
            count: context.count,
        }
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
    #[cfg_attr(
        not(all(test, feature = "web", target_arch = "wasm32")),
        expect(
            dead_code,
            reason = "Only the wasm render tests exercise queued follow-up events."
        )
    )]
    StartNotify,
    Notify,
}

#[cfg(not(feature = "ssr"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EffectAction {
    None,
    Start,
    Replace,
    Cancel,
}

#[cfg(not(feature = "ssr"))]
#[derive(Clone, Debug)]
pub(super) struct EffectContext {
    pub(super) log: Arc<Mutex<Vec<&'static str>>>,
    pub(super) notify_count: u32,
}

#[cfg(not(feature = "ssr"))]
impl PartialEq for EffectContext {
    fn eq(&self, other: &Self) -> bool {
        self.notify_count == other.notify_count && Arc::ptr_eq(&self.log, &other.log)
    }
}

#[cfg(not(feature = "ssr"))]
impl Eq for EffectContext {}

#[cfg(not(feature = "ssr"))]
#[derive(Clone, Debug)]
pub(super) struct EffectProps {
    pub(super) id: String,
    pub(super) action: EffectAction,
    pub(super) log: Arc<Mutex<Vec<&'static str>>>,
}

#[cfg(not(feature = "ssr"))]
impl PartialEq for EffectProps {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.action == other.action
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
    ) -> Option<ars_core::TransitionPlan<Self>> {
        match event {
            EffectEvent::Start => Some(
                ars_core::TransitionPlan::to(EffectState::Active).with_effect(tracked_effect(
                    "timer",
                    "setup:start",
                    "cleanup:start",
                )),
            ),

            EffectEvent::Replace => {
                Some(ars_core::TransitionPlan::new().with_effect(tracked_effect(
                    "timer",
                    "setup:replace",
                    "cleanup:replace",
                )))
            }

            EffectEvent::Cancel => Some(ars_core::TransitionPlan::new().cancel_effect("timer")),

            EffectEvent::Stop => Some(ars_core::TransitionPlan::to(EffectState::Idle)),

            EffectEvent::StartNotify => Some(
                ars_core::TransitionPlan::to(EffectState::Active).with_effect(PendingEffect::new(
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
                )),
            ),

            EffectEvent::Notify => Some(ars_core::TransitionPlan::new().apply(
                |ctx: &mut EffectContext| {
                    ctx.notify_count += 1;
                },
            )),
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

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old.action == new.action {
            return Vec::new();
        }

        match new.action {
            EffectAction::None => Vec::new(),
            EffectAction::Start => vec![EffectEvent::Start],
            EffectAction::Replace => vec![EffectEvent::Replace],
            EffectAction::Cancel => vec![EffectEvent::Cancel],
        }
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
