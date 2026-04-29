use std::sync::Arc;
#[cfg(not(feature = "ssr"))]
use std::sync::Mutex;

use ars_core::{
    AriaAttr, AttrMap, ComponentPart, ConnectApi, HasId, HtmlAttr, I18nRegistries, IntlBackend,
    NullPlatformEffects, RenderMode, TransitionPlan,
};
use ars_i18n::{Locale, StubIntlBackend};
use leptos::reactive::traits::Get;

use super::{test_support::*, *};

#[test]
fn toggle_helper_types_and_test_backend_cover_contract_helpers() {
    let mut props = ToggleProps {
        id: String::from("toggle"),
    }
    .with_id(String::from("renamed"));

    let locale = Locale::parse("en-US").expect("locale should parse");

    let backend = TestIntlBackend;

    assert_eq!(props.id(), "renamed");

    props.set_id(String::from("updated"));

    assert_eq!(props.id(), "updated");

    assert_eq!(TogglePart::scope(), "toggle");
    assert_eq!(TogglePart.name(), "root");
    assert_eq!(TogglePart::all(), vec![TogglePart]);

    assert_eq!(format!("{backend:?}"), "TestIntlBackend");
    assert_eq!(
        backend.weekday_short_label(ars_i18n::Weekday::Monday, &locale),
        StubIntlBackend.weekday_short_label(ars_i18n::Weekday::Monday, &locale)
    );
    assert_eq!(
        backend.weekday_long_label(ars_i18n::Weekday::Monday, &locale),
        StubIntlBackend.weekday_long_label(ars_i18n::Weekday::Monday, &locale)
    );
    assert_eq!(
        backend.month_long_name(5, &locale),
        StubIntlBackend.month_long_name(5, &locale)
    );
    assert_eq!(
        backend.day_period_label(true, &locale),
        StubIntlBackend.day_period_label(true, &locale)
    );
    assert_eq!(backend.day_period_from_char('a', &locale), Some(false));
    assert_eq!(
        backend.format_segment_digits(
            9,
            core::num::NonZero::new(2).expect("minimum digits should be non-zero"),
            &locale,
        ),
        "09"
    );
    assert_eq!(
        backend.hour_cycle(&locale),
        StubIntlBackend.hour_cycle(&locale)
    );
    assert_eq!(
        backend.week_info(&locale),
        StubIntlBackend.week_info(&locale)
    );
}

// --- Tests ---

#[test]
#[cfg(feature = "ssr")]
fn current_render_mode_reports_server_for_ssr_builds() {
    let owner = Owner::new();
    owner.with(|| {
        #[cfg(feature = "hydrate")]
        provide_context(IsHydrating(true));

        assert_eq!(current_render_mode(false), RenderMode::Server);
        assert_eq!(current_render_mode(true), RenderMode::Server);
    });
}

#[test]
#[cfg(not(feature = "ssr"))]
fn current_render_mode_reports_client_without_active_hydration() {
    let owner = Owner::new();
    owner.with(|| {
        assert_eq!(current_render_mode(false), RenderMode::Client);

        #[cfg(feature = "hydrate")]
        assert_eq!(current_render_mode(true), RenderMode::Hydrating);

        #[cfg(not(feature = "hydrate"))]
        assert_eq!(current_render_mode(true), RenderMode::Client);
    });
}

#[test]
#[cfg(all(feature = "hydrate", not(feature = "ssr")))]
fn current_render_mode_uses_hydration_context_at_runtime() {
    let owner = Owner::new();
    owner.with(|| {
        provide_context(IsHydrating(true));

        assert_eq!(current_render_mode(false), RenderMode::Hydrating);
    });

    owner.with(|| {
        provide_context(IsHydrating(false));

        assert_eq!(current_render_mode(false), RenderMode::Client);
        assert_eq!(current_render_mode(true), RenderMode::Hydrating);
    });
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
fn use_machine_return_type_clone_delegates_to_copy_fields() {
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        let cloned = <UseMachineReturn<ToggleMachine> as Clone>::clone(&machine);

        cloned.send.run(ToggleEvent::Toggle);

        assert_eq!(machine.state.get_untracked(), ToggleState::On);
        assert_eq!(cloned.state.get_untracked(), ToggleState::On);
    });
}

#[test]
fn use_machine_return_debug_names_the_type() {
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        let debug = format!("{machine:?}");

        assert!(debug.contains("UseMachineReturn"));
        assert!(debug.contains("context_version"));
    });
}

#[test]
fn use_machine_creates_service_with_initial_state() {
    // Test use_machine within a Leptos reactive Owner.
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        // Initial state should be Off
        assert_eq!(machine.state.get_untracked(), ToggleState::Off);

        // Context version starts at 0
        assert_eq!(machine.context_version.get_untracked(), 0);
    });
}

#[test]
fn use_machine_send_updates_state() {
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        assert_eq!(machine.state.get_untracked(), ToggleState::Off);

        machine.send.run(ToggleEvent::Toggle);

        assert_eq!(machine.state.get_untracked(), ToggleState::On);

        machine.send.run(ToggleEvent::Toggle);

        assert_eq!(machine.state.get_untracked(), ToggleState::Off);
    });
}

#[test]
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Method pointers are not general enough for the lifetime-parameterized test API."
)]
fn with_api_snapshot_reads_current_state() {
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        let is_on = machine.with_api_snapshot(|api| api.is_on());

        assert!(!is_on);

        machine.send.run(ToggleEvent::Toggle);

        let is_on = machine.with_api_snapshot(|api| api.is_on());

        assert!(is_on);
    });
}

#[test]
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Method pointers are not general enough for the lifetime-parameterized test API."
)]
fn with_api_snapshot_rejects_callback_sends_events() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            machine.with_api_snapshot(|api| api.trigger_toggle());
        });
    }));

    #[cfg(debug_assertions)]
    assert!(result.is_err());

    #[cfg(not(debug_assertions))]
    assert!(result.is_ok());
}

#[test]
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Method pointers are not general enough for the lifetime-parameterized test API."
)]
fn derive_tracks_connect_output() {
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        let is_on = machine.derive(|api| api.is_on());

        assert!(!is_on.get());

        machine.send.run(ToggleEvent::Toggle);

        assert!(is_on.get());
    });
}

#[test]
fn derive_rejects_callback_sends_events() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            let derived = machine.derive(|api| {
                api.trigger_toggle();
                api.is_on()
            });

            let _ = derived.get();
        });
    }));

    #[cfg(debug_assertions)]
    assert!(result.is_err());

    #[cfg(not(debug_assertions))]
    assert!(result.is_ok());
}

#[test]
fn with_api_ephemeral_reads_current_state() {
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        let is_on = machine.with_api_ephemeral(|api| api.get().is_on());

        assert!(!is_on);

        machine.send.run(ToggleEvent::Toggle);

        let attrs = machine.with_api_ephemeral(|api| api.get().part_attrs(TogglePart));

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Pressed)), Some("true"));
    });
}

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

#[derive(Clone)]
struct EnvContext {
    locale: String,
    intl_backend: Arc<dyn IntlBackend>,
}

impl Debug for EnvContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnvContext")
            .field("locale", &self.locale)
            .field("intl_backend", &"Arc(..)")
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
    type Messages = ();
    type Api<'a> = EnvApi;

    fn init(
        _props: &Self::Props,
        env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            (),
            EnvContext {
                locale: env.locale.to_bcp47(),
                intl_backend: Arc::clone(&env.intl_backend),
            },
        )
    }

    fn transition(
        _state: &Self::State,
        _event: &Self::Event,
        _context: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        None
    }

    fn connect<'a>(
        _state: &'a Self::State,
        _context: &'a Self::Context,
        _props: &'a Self::Props,
        _send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        EnvApi
    }
}

struct EnvApi;

impl ConnectApi for EnvApi {
    type Part = TogglePart;

    fn part_attrs(&self, _part: Self::Part) -> AttrMap {
        AttrMap::new()
    }
}

fn provide_test_context(locale: &str, intl_backend: Arc<dyn IntlBackend>) {
    crate::provide_ars_context(crate::ArsContext::new(
        Locale::parse(locale).expect("locale should parse"),
        ars_i18n::Direction::Ltr,
        ars_core::ColorMode::System,
        false,
        false,
        None,
        None,
        None,
        Arc::new(NullPlatformEffects),
        Arc::new(ars_core::DefaultModalityContext::new()),
        intl_backend,
        Arc::new(I18nRegistries::new()),
        ars_core::StyleStrategy::Inline,
    ));
}

#[test]
fn use_machine_inner_resolves_locale_and_intl_backend_from_context() {
    let owner = Owner::new();
    owner.with(|| {
        let expected_backend: Arc<dyn IntlBackend> = Arc::new(TestIntlBackend);

        provide_test_context("es-ES", Arc::clone(&expected_backend));

        let machine = use_machine::<EnvMachine>(EnvProps { id: String::new() });

        machine.service.with_value(|service| {
            assert!(service.props().id().starts_with("component-"));
            assert_eq!(service.context().locale, "es-ES");
            assert!(Arc::ptr_eq(
                &service.context().intl_backend,
                &expected_backend
            ));
        });
    });
}

#[test]
fn use_machine_injects_generated_id_when_props_id_is_empty() {
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine::<ToggleMachine>(ToggleProps { id: String::new() });

        machine.service.with_value(|service| {
            assert!(service.props().id().starts_with("component-"));
        });
    });
}

#[cfg(feature = "ssr")]
#[test]
fn use_machine_hydrated_preserves_snapshot_state_and_id() {
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine_hydrated::<ToggleMachine>(
            ToggleProps { id: String::new() },
            HydrationSnapshot::<ToggleMachine> {
                state: ToggleState::On,
                id: String::from("toggle-hydrated"),
            },
        );

        assert_eq!(machine.state.get_untracked(), ToggleState::On);
        machine.service.with_value(|service| {
            assert_eq!(service.state(), &ToggleState::On);
            assert_eq!(service.props().id(), "toggle-hydrated");
        });
    });
}

#[cfg(feature = "ssr")]
#[test]
#[should_panic(expected = "HydrationSnapshot id must match Props::id")]
fn use_machine_hydrated_rejects_mismatched_explicit_props_id() {
    let owner = Owner::new();
    owner.with(|| {
        let _machine = use_machine_hydrated::<ToggleMachine>(
            ToggleProps {
                id: String::from("client-id"),
            },
            HydrationSnapshot::<ToggleMachine> {
                state: ToggleState::On,
                id: String::from("server-id"),
            },
        );
    });
}

#[cfg(feature = "ssr")]
#[test]
fn use_machine_with_reactive_props_hydrated_keeps_syncing_props() {
    let owner = Owner::new();
    owner.with(|| {
        let (props, set_props) = signal(PropProps {
            id: String::new(),
            checked: false,
            label: "a",
        });

        let machine = use_machine_with_reactive_props_hydrated::<PropMachine>(
            props.into(),
            HydrationSnapshot::<PropMachine> {
                state: PropState::On,
                id: String::from("prop-hydrated"),
            },
        );

        assert_eq!(machine.state.get_untracked(), PropState::On);
        machine.service.with_value(|service| {
            assert_eq!(service.props().id(), "prop-hydrated");
        });

        set_props.set(PropProps {
            id: String::new(),
            checked: true,
            label: "b",
        });

        assert_eq!(machine.state.get_untracked(), PropState::On);
        assert_eq!(machine.context_version.get_untracked(), 1);

        set_props.set(PropProps {
            id: String::new(),
            checked: false,
            label: "b",
        });

        assert_eq!(machine.state.get_untracked(), PropState::Off);
        assert_eq!(machine.context_version.get_untracked(), 1);

        machine.service.with_value(|service| {
            assert_eq!(service.props().id(), "prop-hydrated");
            assert_eq!(service.context().sync_count, 1);
        });
    });
}

#[test]
fn use_machine_with_reactive_props_syncs_state_and_context_changes() {
    let owner = Owner::new();
    owner.with(|| {
        let (props, set_props) = signal(PropProps {
            id: String::from("toggle"),
            checked: false,
            label: "a",
        });

        let machine = use_machine_with_reactive_props::<PropMachine>(props.into());

        assert_eq!(machine.state.get_untracked(), PropState::Off);
        assert_eq!(machine.context_version.get_untracked(), 0);

        set_props.set(PropProps {
            id: String::from("toggle"),
            checked: true,
            label: "a",
        });

        assert_eq!(machine.state.get_untracked(), PropState::On);
        assert_eq!(machine.context_version.get_untracked(), 0);

        set_props.set(PropProps {
            id: String::from("toggle"),
            checked: true,
            label: "b",
        });

        assert_eq!(machine.state.get_untracked(), PropState::On);
        assert_eq!(machine.context_version.get_untracked(), 1);

        machine.service.with_value(|service| {
            assert_eq!(service.context().sync_count, 1);
        });
    });
}

#[test]
fn reactive_props_sync_preserves_service_id_when_signal_props_omit_id() {
    let owner = Owner::new();
    owner.with(|| {
        let (props, set_props) = signal(PropProps {
            id: String::new(),
            checked: false,
            label: "a",
        });

        let machine = use_machine_with_reactive_props::<PropMachine>(props.into());

        let service_id = machine.service.with_value(|service| {
            let id = service.props().id().to_owned();

            assert!(id.starts_with("component-"));

            id
        });

        set_props.set(PropProps {
            id: String::new(),
            checked: true,
            label: "a",
        });

        assert_eq!(machine.state.get_untracked(), PropState::On);
        assert_eq!(machine.context_version.get_untracked(), 0);

        set_props.set(PropProps {
            id: String::new(),
            checked: true,
            label: "b",
        });

        assert_eq!(machine.context_version.get_untracked(), 1);

        machine.service.with_value(|service| {
            assert_eq!(service.props().id(), service_id);
            assert_eq!(service.context().sync_count, 1);
        });
    });
}

#[test]
fn reactive_props_sync_preserves_explicit_service_id_when_next_props_omit_id() {
    let owner = Owner::new();
    owner.with(|| {
        let (props, set_props) = signal(PropProps {
            id: String::from("stable"),
            checked: false,
            label: "a",
        });

        let machine = use_machine_with_reactive_props::<PropMachine>(props.into());

        set_props.set(PropProps {
            id: String::new(),
            checked: true,
            label: "a",
        });

        assert_eq!(machine.state.get_untracked(), PropState::On);

        machine.service.with_value(|service| {
            assert_eq!(service.props().id(), "stable");
        });
    });
}

#[test]
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Method pointers are not general enough for the lifetime-parameterized test API."
)]
fn derive_recomputes_when_only_context_changes() {
    let owner = Owner::new();
    owner.with(|| {
        let (props, set_props) = signal(PropProps {
            id: String::from("toggle"),
            checked: false,
            label: "a",
        });

        let machine = use_machine_with_reactive_props::<PropMachine>(props.into());

        let sync_count = machine.derive(|api| api.sync_count());

        assert_eq!(sync_count.get(), 0);

        set_props.set(PropProps {
            id: String::from("toggle"),
            checked: false,
            label: "b",
        });

        assert_eq!(machine.state.get_untracked(), PropState::Off);
        assert_eq!(machine.context_version.get_untracked(), 1);
        assert_eq!(sync_count.get(), 1);
    });
}

#[test]
fn context_version_only_increments_on_context_changes() {
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        assert_eq!(machine.context_version.get_untracked(), 0);

        machine.send.run(ToggleEvent::Toggle);

        assert_eq!(machine.context_version.get_untracked(), 0);

        machine.send.run(ToggleEvent::Toggle);

        assert_eq!(machine.context_version.get_untracked(), 0);
    });
}

#[cfg(not(feature = "ssr"))]
#[test]
fn effect_lifecycle_replaces_cancels_and_unmounts_cleanups() {
    let owner = Owner::new();
    owner.with(|| {
        let log = Arc::new(Mutex::new(Vec::new()));

        let machine = use_machine::<EffectMachine>(EffectProps {
            id: String::from("effects"),
            log: Arc::clone(&log),
        });

        machine.send.run(EffectEvent::Start);

        assert_eq!(effect_log(&log), vec!["setup:start"]);

        machine.send.run(EffectEvent::Replace);

        assert_eq!(
            effect_log(&log),
            vec!["setup:start", "cleanup:start", "setup:replace"]
        );

        machine.send.run(EffectEvent::Cancel);

        assert_eq!(
            effect_log(&log),
            vec![
                "setup:start",
                "cleanup:start",
                "setup:replace",
                "cleanup:replace",
            ]
        );

        machine.send.run(EffectEvent::Start);

        assert_eq!(
            effect_log(&log),
            vec![
                "setup:start",
                "cleanup:start",
                "setup:replace",
                "cleanup:replace",
                "setup:start",
            ]
        );

        owner.cleanup();

        assert_eq!(
            effect_log(&log),
            vec![
                "setup:start",
                "cleanup:start",
                "setup:replace",
                "cleanup:replace",
                "setup:start",
                "cleanup:start",
            ]
        );
    });
}

#[cfg(not(feature = "ssr"))]
#[test]
fn state_changes_drain_existing_effect_cleanups() {
    let owner = Owner::new();
    owner.with(|| {
        let log = Arc::new(Mutex::new(Vec::new()));

        let machine = use_machine::<EffectMachine>(EffectProps {
            id: String::from("effects"),
            log: Arc::clone(&log),
        });

        machine.send.run(EffectEvent::Start);
        machine.send.run(EffectEvent::Stop);

        assert_eq!(effect_log(&log), vec!["setup:start", "cleanup:start"]);
        assert_eq!(machine.state.get_untracked(), EffectState::Idle);
    });
}

#[cfg(not(feature = "ssr"))]
#[test]
fn effect_send_handle_dispatches_follow_up_events() {
    let owner = Owner::new();
    owner.with(|| {
        let log = Arc::new(Mutex::new(Vec::new()));

        let machine = use_machine::<EffectMachine>(EffectProps {
            id: String::from("effects"),
            log: Arc::clone(&log),
        });

        machine.send.run(EffectEvent::StartNotify);

        assert_eq!(machine.state.get_untracked(), EffectState::Active);
        assert_eq!(machine.context_version.get_untracked(), 1);

        machine.service.with_value(|service| {
            assert_eq!(service.context().notify_count, 1);
        });

        assert_eq!(effect_log(&log), vec!["setup:notify"]);
    });
}
