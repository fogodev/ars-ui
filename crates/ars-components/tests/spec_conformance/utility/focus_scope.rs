use super::*;

#[test]
fn focus_scope_anatomy_matches_spec() {
    // FocusScope's anatomy table (spec §2) declares a single row:
    // `Container`. The container element is also the `ROOT` part —
    // every `Part` enum's first variant is treated as root by the
    // `ComponentPart` derive macro.
    assert_anatomy(
        "focus-scope",
        &[(utility_core::focus_scope::Part::Container, "container")],
    );
}

#[test]
fn focus_scope_props_defaults_match_spec_section_1_4() {
    let props = utility_core::focus_scope::Props::default();

    assert_eq!(props.id, "");
    assert!(!props.trapped, "spec §1.4: trapped defaults to false");
    assert!(!props.contain, "spec §1.4: contain defaults to false");
    assert!(props.auto_focus, "spec §1.4: auto_focus defaults to true");
    assert!(
        props.restore_focus,
        "spec §1.4: restore_focus defaults to true",
    );
}

#[test]
fn focus_scope_event_variants_match_spec_section_1_2() {
    // Spec §1.2 declares seven events. Listing all of them in a typed
    // array here guards against a future drift where a variant is
    // renamed or removed without spec sync.
    let events: [utility_core::focus_scope::Event; 7] = [
        utility_core::focus_scope::Event::Activate {
            trapped: false,
            saved_focus_id: None,
        },
        utility_core::focus_scope::Event::Deactivate {
            restore_focus: false,
        },
        utility_core::focus_scope::Event::TrapFocus,
        utility_core::focus_scope::Event::ReleaseTrap,
        utility_core::focus_scope::Event::RestoreFocus,
        utility_core::focus_scope::Event::FocusFirst,
        utility_core::focus_scope::Event::FocusLast,
    ];

    assert_eq!(events.len(), 7);
}

#[test]
fn focus_scope_state_variants_match_spec_section_1_1() {
    // Spec §1.1: `Inactive` and `Active { trapped: bool }`.
    let states: [utility_core::focus_scope::State; 3] = [
        utility_core::focus_scope::State::Inactive,
        utility_core::focus_scope::State::Active { trapped: false },
        utility_core::focus_scope::State::Active { trapped: true },
    ];

    assert_eq!(states.len(), 3);
}

#[test]
fn focus_scope_effect_variants_match_spec_section_1_8() {
    // Spec §1.8 (Effect Contract): four typed effect intents.
    let effects: [utility_core::focus_scope::Effect; 4] = [
        utility_core::focus_scope::Effect::FocusTrapListener,
        utility_core::focus_scope::Effect::FocusFirst,
        utility_core::focus_scope::Effect::FocusLast,
        utility_core::focus_scope::Effect::RestoreFocus,
    ];

    assert_eq!(effects.len(), 4);
}

#[test]
fn focus_scope_connect_api_emits_data_ars_trapped_not_focus_trapped() {
    // Spec §2 anatomy table declares the trap data attribute as
    // `data-ars-trapped`. GitHub issue #212 test 9 has a typo that
    // calls it `data-ars-focus-trapped`. This regression test pins
    // the spec-correct name so a future refactor cannot drift to
    // the issue's wording without a deliberate spec edit.
    let mut service = Service::<utility_core::focus_scope::Machine>::new(
        utility_core::focus_scope::Props::new().id("trap"),
        &Env::default(),
        &utility_core::focus_scope::Messages,
    );

    drop(service.send(utility_core::focus_scope::Event::Activate {
        trapped: true,
        saved_focus_id: None,
    }));

    let attrs = service.connect(&|_| {}).container_attrs();

    assert_eq!(
        attrs.get(&HtmlAttr::Data("ars-trapped")),
        Some("true"),
        "spec §2 names the trap data attribute `data-ars-trapped`",
    );
    assert_eq!(
        attrs.get(&HtmlAttr::Data("ars-focus-trapped")),
        None,
        "spec §2 does NOT define `data-ars-focus-trapped`; issue #212 \
         test 9 has a typo that the spec is authoritative against",
    );
}
