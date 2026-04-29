#[cfg(not(feature = "ssr"))]
use std::sync::Mutex;
use std::{cell::RefCell, rc::Rc, sync::Arc};

use ars_core::{HasId, I18nRegistries, MessageFn, NullPlatformEffects};
use ars_i18n::{Direction, IntlBackend, Locale, StubIntlBackend};
use dioxus::dioxus_core::{NoOpMutations, ScopeId};

use super::{test_support, test_support::*, *};
use crate::provider::{ArsContext, NullPlatform};

type PropIdSnapshot = (String, PropState, u64, u32);

#[derive(Clone)]
struct TestIntlBackend;

impl Debug for TestIntlBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TestIntlBackend")
    }
}

impl IntlBackend for TestIntlBackend {
    fn weekday_short_label(&self, weekday: ars_i18n::Weekday, locale: &Locale) -> String {
        StubIntlBackend.weekday_short_label(weekday, locale)
    }

    fn weekday_long_label(&self, weekday: ars_i18n::Weekday, locale: &Locale) -> String {
        StubIntlBackend.weekday_long_label(weekday, locale)
    }

    fn month_long_name(&self, month: u8, locale: &Locale) -> String {
        StubIntlBackend.month_long_name(month, locale)
    }

    fn day_period_label(&self, is_pm: bool, locale: &Locale) -> String {
        StubIntlBackend.day_period_label(is_pm, locale)
    }

    fn day_period_from_char(&self, ch: char, locale: &Locale) -> Option<bool> {
        StubIntlBackend.day_period_from_char(ch, locale)
    }

    fn format_segment_digits(
        &self,
        value: u32,
        min_digits: core::num::NonZero<u8>,
        locale: &Locale,
    ) -> String {
        StubIntlBackend.format_segment_digits(value, min_digits, locale)
    }

    fn hour_cycle(&self, locale: &Locale) -> ars_i18n::HourCycle {
        StubIntlBackend.hour_cycle(locale)
    }

    fn week_info(&self, locale: &Locale) -> ars_i18n::WeekInfo {
        StubIntlBackend.week_info(locale)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct EnvMessages {
    label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for EnvMessages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Default"),
        }
    }
}

impl ars_core::ComponentMessages for EnvMessages {}

#[derive(Clone)]
struct EnvContext {
    locale: String,
    intl_backend: Arc<dyn IntlBackend>,
    label: String,
}

impl Debug for EnvContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnvContext")
            .field("locale", &self.locale)
            .field("intl_backend", &"Arc(..)")
            .field("label", &self.label)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EnvProps {
    id: String,
}

impl HasId for EnvProps {
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

struct EnvMachine;

impl Machine for EnvMachine {
    type State = ();
    type Event = ();
    type Context = EnvContext;
    type Props = EnvProps;
    type Messages = EnvMessages;
    type Api<'a> = ToggleApi;

    fn init(
        _props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            (),
            EnvContext {
                locale: env.locale.to_bcp47(),
                intl_backend: Arc::clone(&env.intl_backend),
                label: (messages.label)(&env.locale),
            },
        )
    }

    fn transition(
        _state: &Self::State,
        _event: &Self::Event,
        _context: &Self::Context,
        _props: &Self::Props,
    ) -> Option<ars_core::TransitionPlan<Self>> {
        None
    }

    fn connect<'a>(
        _state: &'a Self::State,
        _context: &'a Self::Context,
        _props: &'a Self::Props,
        _send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        ToggleApi { is_on: false }
    }
}

fn provide_test_context(
    locale: &str,
    intl_backend: Arc<dyn IntlBackend>,
    registries: Arc<I18nRegistries>,
) -> ArsContext {
    ArsContext::new(
        Locale::parse(locale).expect("locale should parse"),
        Direction::Ltr,
        ars_core::ColorMode::System,
        false,
        false,
        None,
        None,
        None,
        Arc::new(NullPlatformEffects),
        Arc::new(ars_core::DefaultModalityContext::new()),
        intl_backend,
        registries,
        Arc::new(NullPlatform),
        ars_core::StyleStrategy::Inline,
    )
}

#[test]
fn use_machine_return_type_is_copy() {
    // Verify the struct is Copy by checking that all field types are Copy.
    // This is a compile-time check — if UseMachineReturn<ToggleMachine> is
    // not Copy, this function won't compile.
    fn assert_copy<T: Copy>() {}

    assert_copy::<UseMachineReturn<ToggleMachine>>();
}

#[test]
fn use_machine_return_clone_and_debug_impls_work() {
    fn app() -> Element {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        #[expect(
            clippy::clone_on_copy,
            reason = "This test intentionally exercises the manual Clone impl."
        )]
        let clone = machine.clone();

        assert_eq!(*clone.state.peek(), ToggleState::Off);
        assert!(format!("{machine:?}").contains("UseMachineReturn"));

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();
}

#[test]
fn use_machine_creates_service_with_initial_state() {
    fn app() -> Element {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        // Initial state should be Off
        assert_eq!(*machine.state.peek(), ToggleState::Off);

        // Context version starts at 0
        assert_eq!(*machine.context_version.peek(), 0);

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();
}

#[test]
fn use_machine_send_updates_state() {
    fn app() -> Element {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        assert_eq!(*machine.state.peek(), ToggleState::Off);

        machine.send.call(test_support::ToggleEvent::Toggle);

        assert_eq!(*machine.state.peek(), ToggleState::On);

        machine.send.call(test_support::ToggleEvent::Toggle);

        assert_eq!(*machine.state.peek(), ToggleState::Off);

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();
}

#[test]
fn with_api_snapshot_reads_current_state() {
    fn app() -> Element {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        let is_on = machine.with_api_snapshot(|api| api.is_on);

        assert!(!is_on);

        machine.send.call(test_support::ToggleEvent::Toggle);

        let is_on = machine.with_api_snapshot(|api| api.is_on);

        assert!(is_on);

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();
}

#[test]
fn with_api_ephemeral_reads_current_state() {
    fn app() -> Element {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        let is_on = machine.with_api_ephemeral(|api| api.get().is_on);

        assert!(!is_on);

        machine.send.call(test_support::ToggleEvent::Toggle);

        let is_on = machine.with_api_ephemeral(|api| api.get().is_on);

        assert!(is_on);

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();
}

#[test]
fn context_version_increments_on_transition() {
    fn app() -> Element {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        assert_eq!(*machine.context_version.peek(), 0);

        machine.send.call(test_support::ToggleEvent::Toggle);

        assert_eq!(*machine.context_version.peek(), 0);

        machine.send.call(test_support::ToggleEvent::Toggle);

        assert_eq!(*machine.context_version.peek(), 0);

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();
}

#[test]
fn context_version_increments_on_context_only_transition() {
    fn app() -> Element {
        let machine = use_machine::<DerivedMachine>(DerivedProps {
            id: String::from("derived"),
        });

        assert_eq!(*machine.context_version.peek(), 0);

        machine.send.call(DerivedEvent::BumpContext);

        assert_eq!(*machine.context_version.peek(), 1);
        assert_eq!(*machine.state.peek(), DerivedState::Off);

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();
}

#[test]
fn derive_recomputes_for_state_and_context_changes() {
    let snapshots = Rc::new(RefCell::new(Vec::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(snapshots: Rc<RefCell<Vec<(bool, u32)>>>) -> Element {
        let machine = use_machine::<DerivedMachine>(DerivedProps {
            id: String::from("derived"),
        });

        let derived = machine.derive(|api| (api.is_on, api.count));

        let mut phase = use_signal(|| 0u8);

        snapshots.borrow_mut().push(derived());

        if phase() == 0 {
            phase.set(1);

            machine.send.call(DerivedEvent::BumpContext);
        } else if phase() == 1 {
            phase.set(2);

            machine.send.call(DerivedEvent::Toggle);
        }

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));

    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    assert_eq!(
        snapshots.borrow().as_slice(),
        &[(false, 0), (false, 1), (true, 1)]
    );
}

#[test]
fn use_machine_inner_resolves_locale_icu_and_messages_from_context() {
    fn app() -> Element {
        let expected_backend: Arc<dyn IntlBackend> = Arc::new(TestIntlBackend);

        let mut registries = I18nRegistries::new();

        registries.register(
            ars_core::MessagesRegistry::new(EnvMessages::default()).register(
                "es",
                EnvMessages {
                    label: MessageFn::static_str("Hola"),
                },
            ),
        );

        let ctx =
            provide_test_context("es-ES", Arc::clone(&expected_backend), Arc::new(registries));

        use_context_provider(|| ctx);

        let machine = use_machine::<EnvMachine>(EnvProps { id: String::new() });

        let service = machine.service.peek();

        assert!(service.props().id().starts_with("component-"));
        assert_eq!(service.context().locale, "es-ES");
        assert!(Arc::ptr_eq(
            &service.context().intl_backend,
            &expected_backend
        ));
        assert_eq!(service.context().label, "Hola");

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();
}

#[test]
fn use_machine_syncs_external_prop_changes_on_rerender() {
    let snapshots = Rc::new(RefCell::new(Vec::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(snapshots: Rc<RefCell<Vec<(PropState, u64, u32)>>>) -> Element {
        let mut props = use_signal(|| PropProps {
            id: String::from("toggle"),
            checked: false,
            label: "a",
        });

        let mut phase = use_signal(|| 0u8);

        let machine = use_machine::<PropMachine>(props());

        let sync_count = machine.service.peek().context().sync_count;

        snapshots.borrow_mut().push((
            *machine.state.peek(),
            *machine.context_version.peek(),
            sync_count,
        ));

        if phase() == 0 {
            phase.set(1);

            props.set(PropProps {
                id: String::from("toggle"),
                checked: true,
                label: "a",
            });
        } else if phase() == 1 {
            phase.set(2);

            props.set(PropProps {
                id: String::from("toggle"),
                checked: true,
                label: "b",
            });
        }

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));

    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    assert_eq!(
        snapshots.borrow().as_slice(),
        &[
            (PropState::Off, 0, 0),
            (PropState::On, 0, 0),
            (PropState::On, 1, 1),
        ]
    );
}

#[test]
fn generated_id_stays_stable_across_rerenders() {
    let snapshots = Rc::new(RefCell::new(Vec::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(snapshots: Rc<RefCell<Vec<String>>>) -> Element {
        let mut phase = use_signal(|| 0u8);

        let machine = use_machine::<ToggleMachine>(ToggleProps { id: String::new() });

        snapshots
            .borrow_mut()
            .push(machine.service.peek().props().id().to_owned());

        if phase() < 2 {
            phase += 1;
        }

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));

    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    let snapshots = snapshots.borrow();

    assert_eq!(snapshots.len(), 3);
    assert!(snapshots[0].starts_with("component-"));
    assert_eq!(snapshots[0], snapshots[1]);
    assert_eq!(snapshots[1], snapshots[2]);
}

#[test]
fn use_machine_with_reactive_props_syncs_external_prop_changes() {
    let snapshots = Rc::new(RefCell::new(Vec::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(snapshots: Rc<RefCell<Vec<(PropState, u64, u32)>>>) -> Element {
        let mut props = use_signal(|| PropProps {
            id: String::from("toggle"),
            checked: false,
            label: "a",
        });

        let mut phase = use_signal(|| 0u8);

        let machine = use_machine_with_reactive_props::<PropMachine>(props);

        let sync_count = machine.service.peek().context().sync_count;

        snapshots.borrow_mut().push((
            *machine.state.peek(),
            *machine.context_version.peek(),
            sync_count,
        ));

        if phase() == 0 {
            phase.set(1);

            props.set(PropProps {
                id: String::from("toggle"),
                checked: true,
                label: "a",
            });
        } else if phase() == 1 {
            phase.set(2);

            props.set(PropProps {
                id: String::from("toggle"),
                checked: true,
                label: "b",
            });
        }

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));

    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    assert_eq!(
        snapshots.borrow().as_slice(),
        &[
            (PropState::Off, 0, 0),
            (PropState::On, 0, 0),
            (PropState::On, 1, 1),
        ]
    );
}

#[test]
fn reactive_props_sync_preserves_service_id_when_signal_props_omit_id() {
    let snapshots = Rc::new(RefCell::new(Vec::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(snapshots: Rc<RefCell<Vec<PropIdSnapshot>>>) -> Element {
        let mut props = use_signal(|| PropProps {
            id: String::new(),
            checked: false,
            label: "a",
        });

        let mut phase = use_signal(|| 0u8);

        let machine = use_machine_with_reactive_props::<PropMachine>(props);

        snapshots.borrow_mut().push((
            machine.service.peek().props().id().to_owned(),
            *machine.state.peek(),
            *machine.context_version.peek(),
            machine.service.peek().context().sync_count,
        ));

        if phase() == 0 {
            phase.set(1);

            props.set(PropProps {
                id: String::new(),
                checked: true,
                label: "a",
            });
        } else if phase() == 1 {
            phase.set(2);

            props.set(PropProps {
                id: String::new(),
                checked: true,
                label: "b",
            });
        }

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));

    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    let snapshots = snapshots.borrow();

    assert_eq!(snapshots.len(), 3);
    assert!(snapshots[0].0.starts_with("component-"));
    assert_eq!(snapshots[0].0, snapshots[1].0);
    assert_eq!(snapshots[1].0, snapshots[2].0);
    assert_eq!(
        snapshots
            .iter()
            .map(|(_, state, version, sync_count)| (*state, *version, *sync_count))
            .collect::<Vec<_>>()
            .as_slice(),
        &[
            (PropState::Off, 0, 0),
            (PropState::On, 0, 0),
            (PropState::On, 1, 1),
        ]
    );
}

#[test]
fn reactive_props_sync_preserves_explicit_service_id_when_next_props_omit_id() {
    let snapshots = Rc::new(RefCell::new(Vec::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(snapshots: Rc<RefCell<Vec<String>>>) -> Element {
        let mut props = use_signal(|| PropProps {
            id: String::from("stable"),
            checked: false,
            label: "a",
        });

        let mut phase = use_signal(|| 0u8);

        let machine = use_machine_with_reactive_props::<PropMachine>(props);

        snapshots
            .borrow_mut()
            .push(machine.service.peek().props().id().to_owned());

        if phase() == 0 {
            phase.set(1);

            props.set(PropProps {
                id: String::new(),
                checked: true,
                label: "a",
            });
        }

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));

    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    assert_eq!(
        snapshots.borrow().as_slice(),
        &[String::from("stable"), String::from("stable")]
    );
}

#[cfg(not(feature = "ssr"))]
#[test]
fn use_sync_props_processes_effect_setup_and_cancel() {
    let log = Arc::new(Mutex::new(Vec::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(log: Arc<Mutex<Vec<&'static str>>>) -> Element {
        let mut props = use_signal(|| EffectProps {
            id: String::from("effects"),
            action: EffectAction::None,
            log: Arc::clone(&log),
        });

        let mut phase = use_signal(|| 0u8);

        let _machine = use_machine_with_reactive_props::<EffectMachine>(props);

        if phase() == 0 {
            phase.set(1);

            props.set(EffectProps {
                id: String::from("effects"),
                action: EffectAction::Start,
                log: Arc::clone(&log),
            });
        } else if phase() == 1 {
            phase.set(2);

            props.set(EffectProps {
                id: String::from("effects"),
                action: EffectAction::Replace,
                log: Arc::clone(&log),
            });
        } else if phase() == 2 {
            phase.set(3);

            props.set(EffectProps {
                id: String::from("effects"),
                action: EffectAction::Cancel,
                log: Arc::clone(&log),
            });
        }

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Arc::clone(&log));

    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    assert_eq!(
        effect_log(&log),
        vec![
            "setup:start",
            "cleanup:start",
            "setup:replace",
            "cleanup:replace",
        ]
    );
}

#[cfg(not(feature = "ssr"))]
#[test]
fn send_effects_run_cleanup_on_state_change() {
    let log = Arc::new(Mutex::new(Vec::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(log: Arc<Mutex<Vec<&'static str>>>) -> Element {
        let machine = use_machine::<EffectMachine>(EffectProps {
            id: String::from("effects"),
            action: EffectAction::None,
            log: Arc::clone(&log),
        });

        let state = *machine.state.peek();

        let notify_count = machine.service.peek().context().notify_count;

        if state == EffectState::Idle && notify_count == 0 {
            machine.send.call(EffectEvent::Start);
        } else if state == EffectState::Active && notify_count == 0 {
            machine.send.call(EffectEvent::Notify);

            machine.send.call(EffectEvent::Stop);
        }

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Arc::clone(&log));

    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    drop(dom);

    assert_eq!(effect_log(&log), vec!["setup:start", "cleanup:start"]);
}
