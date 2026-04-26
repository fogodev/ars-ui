use std::sync::{Arc, Mutex};

use ars_components::overlay::presence;
use ars_core::{ConnectApi, HtmlAttr};
use leptos::reactive::traits::GetUntracked;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

use super::{test_support::*, *};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn callback_to_strong_send_uses_wasm_send_handle() {
    let calls = Arc::new(Mutex::new(Vec::new()));

    let callback_calls = Arc::clone(&calls);

    let callback = Callback::new(move |event: i32| {
        callback_calls
            .lock()
            .expect("mutex should not be poisoned")
            .push(event);
    });

    let strong = callback_to_strong_send(callback);

    strong(7);
    strong(9);

    assert_eq!(
        calls
            .lock()
            .expect("mutex should not be poisoned")
            .as_slice(),
        &[7, 9]
    );
}

#[wasm_bindgen_test]
fn use_machine_updates_state_on_wasm() {
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        assert_eq!(machine.state.get_untracked(), ToggleState::Off);

        machine.send.run(ToggleEvent::Toggle);

        assert_eq!(machine.state.get_untracked(), ToggleState::On);
    });
}

#[wasm_bindgen_test]
fn reactive_props_sync_state_and_context_on_wasm() {
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

        assert_eq!(machine.context_version.get_untracked(), 1);

        machine.service.with_value(|service| {
            assert_eq!(service.context().sync_count, 1);
        });
    });
}

#[wasm_bindgen_test]
fn presence_machine_exposes_live_presence_attrs_on_wasm() {
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine::<presence::Machine>(presence::Props {
            id: String::from("presence"),
            present: false,
            lazy_mount: false,
            skip_animation: false,
            reduce_motion: false,
        });

        let derived = machine.derive(|api| {
            (
                api.is_present(),
                api.is_mounted(),
                api.is_unmounting(),
                api.root_attrs()
                    .get(&HtmlAttr::Data("ars-state"))
                    .map(str::to_owned),
                api.part_attrs(presence::Part::Root)
                    .get(&HtmlAttr::Data("ars-presence"))
                    .map(str::to_owned),
            )
        });

        assert_eq!(
            derived.get_untracked(),
            (
                false,
                false,
                false,
                Some(String::from("closed")),
                Some(String::from("mounted")),
            )
        );
        assert_eq!(machine.state.get_untracked(), presence::State::Unmounted);

        machine.send.run(presence::Event::Mount);

        assert_eq!(machine.state.get_untracked(), presence::State::Mounted);
        assert_eq!(
            derived.get_untracked(),
            (
                true,
                true,
                false,
                Some(String::from("open")),
                Some(String::from("mounted")),
            )
        );

        machine.send.run(presence::Event::Unmount);

        assert_eq!(
            machine.state.get_untracked(),
            presence::State::UnmountPending
        );
        assert_eq!(
            derived.get_untracked(),
            (
                false,
                true,
                true,
                Some(String::from("closed")),
                Some(String::from("exiting")),
            )
        );

        machine.send.run(presence::Event::AnimationEnd);

        assert_eq!(machine.state.get_untracked(), presence::State::Unmounted);
        assert_eq!(
            derived.get_untracked(),
            (
                false,
                false,
                false,
                Some(String::from("closed")),
                Some(String::from("mounted")),
            )
        );
    });
}

#[wasm_bindgen_test]
fn use_machine_injects_generated_id_on_wasm() {
    let owner = Owner::new();
    owner.with(|| {
        let machine = use_machine::<ToggleMachine>(ToggleProps { id: String::new() });

        machine.service.with_value(|service| {
            assert!(service.props().id().starts_with("component-"));
        });
    });
}

#[wasm_bindgen_test]
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Method pointers are not general enough for the lifetime-parameterized test API."
)]
fn derive_recomputes_when_only_context_changes_on_wasm() {
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

#[cfg(not(feature = "ssr"))]
#[wasm_bindgen_test]
fn effect_lifecycle_replaces_cancels_and_unmounts_cleanups_on_wasm() {
    let owner = Owner::new();
    owner.with(|| {
        let log = Arc::new(Mutex::new(Vec::new()));

        let machine = use_machine::<EffectMachine>(EffectProps {
            id: String::from("effects"),
            log: Arc::clone(&log),
        });

        machine.send.run(EffectEvent::Start);
        machine.send.run(EffectEvent::Replace);
        machine.send.run(EffectEvent::Cancel);
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
#[wasm_bindgen_test]
fn effect_send_handle_dispatches_follow_up_events_on_wasm() {
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
