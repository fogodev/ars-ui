use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use ars_components::overlay::presence;
use ars_core::{ConnectApi, HtmlAttr};
use dioxus::dioxus_core::{NoOpMutations, ScopeId};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

use super::{test_support, test_support::*, *};

wasm_bindgen_test_configure!(run_in_browser);

type PresenceDerivedSnapshot = (bool, bool, bool, Option<String>, Option<String>);
type PresenceSnapshot = (PresenceDerivedSnapshot, presence::State);

#[wasm_bindgen_test]
fn use_machine_updates_state_on_wasm() {
    let snapshots = Rc::new(RefCell::new(Vec::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(snapshots: Rc<RefCell<Vec<bool>>>) -> Element {
        let machine = use_machine::<ToggleMachine>(ToggleProps {
            id: String::from("toggle"),
        });

        let mut phase = use_signal(|| 0u8);

        snapshots
            .borrow_mut()
            .push(machine.derive(|api| api.is_on)());

        if phase() == 0 {
            phase.set(1);

            machine.send.call(test_support::ToggleEvent::Toggle);
        }

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));

    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    assert_eq!(snapshots.borrow().as_slice(), &[false, true]);
}

#[wasm_bindgen_test]
fn use_machine_injects_generated_id_on_wasm() {
    let snapshots = Rc::new(RefCell::new(Vec::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(snapshots: Rc<RefCell<Vec<String>>>) -> Element {
        let machine = use_machine::<ToggleMachine>(ToggleProps { id: String::new() });

        snapshots
            .borrow_mut()
            .push(machine.service.peek().props().id().to_owned());

        rsx! {
            div {}
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));

    dom.rebuild_in_place();

    assert_eq!(snapshots.borrow().len(), 1);
    assert!(snapshots.borrow()[0].starts_with("component-"));
}

#[wasm_bindgen_test]
fn derive_and_reactive_props_sync_on_wasm() {
    let snapshots = Rc::new(RefCell::new(Vec::<PropSnapshot>::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(snapshots: Rc<RefCell<Vec<PropSnapshot>>>) -> Element {
        let mut props = use_signal(|| PropProps {
            id: String::from("toggle"),
            checked: false,
            label: "a",
        });

        let mut phase = use_signal(|| 0u8);

        let machine = use_machine_with_reactive_props::<PropMachine>(props);

        let derived = machine.derive(PropApi::snapshot);

        snapshots.borrow_mut().push((
            derived(),
            *machine.state.peek(),
            *machine.context_version.peek(),
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
            ((false, 0), PropState::Off, 0),
            ((true, 0), PropState::On, 0),
            ((true, 1), PropState::On, 1),
        ]
    );
}

#[wasm_bindgen_test]
fn presence_machine_exposes_live_presence_attrs_on_wasm() {
    let snapshots = Rc::new(RefCell::new(Vec::<PresenceSnapshot>::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(snapshots: Rc<RefCell<Vec<PresenceSnapshot>>>) -> Element {
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

        let mut phase = use_signal(|| 0u8);

        snapshots
            .borrow_mut()
            .push((derived(), *machine.state.peek()));

        match phase() {
            0 => {
                phase.set(1);
                machine.send.call(presence::Event::Mount);
            }

            1 => {
                phase.set(2);
                machine.send.call(presence::Event::Unmount);
            }

            2 => {
                phase.set(3);

                machine.send.call(presence::Event::AnimationEnd);
            }

            _ => {}
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
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    assert_eq!(
        snapshots.borrow().as_slice(),
        &[
            (
                (
                    false,
                    false,
                    false,
                    Some(String::from("closed")),
                    Some(String::from("mounted")),
                ),
                presence::State::Unmounted,
            ),
            (
                (
                    true,
                    true,
                    false,
                    Some(String::from("open")),
                    Some(String::from("mounted")),
                ),
                presence::State::Mounted,
            ),
            (
                (
                    false,
                    true,
                    true,
                    Some(String::from("closed")),
                    Some(String::from("exiting")),
                ),
                presence::State::UnmountPending,
            ),
            (
                (
                    false,
                    false,
                    false,
                    Some(String::from("closed")),
                    Some(String::from("mounted")),
                ),
                presence::State::Unmounted,
            ),
        ]
    );
}

#[wasm_bindgen_test]
fn derive_recomputes_for_state_and_context_changes_on_wasm() {
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

#[cfg(not(feature = "ssr"))]
#[wasm_bindgen_test]
fn use_sync_props_processes_effect_setup_and_cancel_on_wasm() {
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
#[wasm_bindgen_test]
fn send_effects_run_cleanup_and_follow_up_events_on_wasm() {
    let snapshots = Rc::new(RefCell::new(Vec::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(snapshots: Rc<RefCell<Vec<(EffectState, u32)>>>) -> Element {
        let log = use_hook(|| Arc::new(Mutex::new(Vec::new())));

        let machine = use_machine::<EffectMachine>(EffectProps {
            id: String::from("effects"),
            action: EffectAction::None,
            log: Arc::clone(&log),
        });

        let mut phase = use_signal(|| 0u8);

        let state = *machine.state.peek();

        let notify_count = machine.service.peek().context().notify_count;

        snapshots.borrow_mut().push((state, notify_count));

        if phase() == 0 {
            phase.set(1);

            machine.send.call(EffectEvent::StartNotify);
        } else if phase() == 1 {
            phase.set(2);

            machine.send.call(EffectEvent::Stop);
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
        &[(EffectState::Idle, 0), (EffectState::Active, 1)]
    );
}
