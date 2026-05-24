//! Spec-conformance tests for `crates/ars-components/src/utility/*`.
//!
//! Each test pulls the expected anatomy from the corresponding component
//! spec's §2 anatomy table and asserts the impl's `Part` enum matches.

use ars_collections::Key;
#[cfg(feature = "i18n")]
use ars_components::utility::highlight;
use ars_components::utility::{
    action_group, download_trigger, focus_scope, group, live_region, swap, toggle, toggle_button,
    toggle_group,
};
use ars_core::{Env, HtmlAttr, Service};

use super::helper::assert_anatomy;

#[test]
fn group_anatomy_matches_spec() {
    // Group's anatomy table (spec §2) declares a single row: `Root`.
    // Children are not parts — they are an unenumerated subtree that
    // inherits state through `GroupContext`, so the `Part` enum stays
    // single-variant.
    assert_anatomy("group", &[(group::Part::Root, "root")]);
}

#[test]
fn download_trigger_anatomy_matches_spec() {
    // DownloadTrigger anatomy table (spec §2): single `Root` row (`<a>`).
    assert_anatomy(
        "download-trigger",
        &[(download_trigger::Part::Root, "root")],
    );
}

#[test]
fn toggle_anatomy_matches_spec() {
    assert_anatomy(
        "toggle",
        &[
            (toggle::Part::Root, "root"),
            (toggle::Part::Indicator, "indicator"),
        ],
    );
}

#[test]
fn toggle_button_anatomy_matches_spec() {
    assert_anatomy("toggle-button", &[(toggle_button::Part::Root, "root")]);
}

#[test]
fn toggle_group_anatomy_matches_spec() {
    assert_anatomy(
        "toggle-group",
        &[
            (toggle_group::Part::Root, "root"),
            (
                toggle_group::Part::Item {
                    value: Key::default(),
                },
                "item",
            ),
            (toggle_group::Part::Indicator, "indicator"),
        ],
    );
}

#[test]
fn action_group_anatomy_matches_spec() {
    assert_anatomy(
        "action-group",
        &[
            (action_group::Part::Root, "root"),
            (
                action_group::Part::Item {
                    item_id: Key::default(),
                },
                "item",
            ),
            (action_group::Part::OverflowTrigger, "overflow-trigger"),
        ],
    );
}

#[test]
fn swap_anatomy_matches_spec() {
    assert_anatomy(
        "swap",
        &[
            (swap::Part::Root, "root"),
            (swap::Part::OnContent, "on-content"),
            (swap::Part::OffContent, "off-content"),
        ],
    );
}

#[test]
fn live_region_anatomy_matches_spec() {
    assert_anatomy("live-region", &[(live_region::Part::Root, "root")]);
}

#[test]
fn focus_scope_anatomy_matches_spec() {
    // FocusScope's anatomy table (spec §2) declares a single row:
    // `Container`. The container element is also the `ROOT` part —
    // every `Part` enum's first variant is treated as root by the
    // `ComponentPart` derive macro.
    assert_anatomy(
        "focus-scope",
        &[(focus_scope::Part::Container, "container")],
    );
}

#[test]
fn focus_scope_props_defaults_match_spec_section_1_4() {
    let props = focus_scope::Props::default();

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
    let events: [focus_scope::Event; 7] = [
        focus_scope::Event::Activate {
            trapped: false,
            saved_focus_id: None,
        },
        focus_scope::Event::Deactivate {
            restore_focus: false,
        },
        focus_scope::Event::TrapFocus,
        focus_scope::Event::ReleaseTrap,
        focus_scope::Event::RestoreFocus,
        focus_scope::Event::FocusFirst,
        focus_scope::Event::FocusLast,
    ];

    assert_eq!(events.len(), 7);
}

#[test]
fn focus_scope_state_variants_match_spec_section_1_1() {
    // Spec §1.1: `Inactive` and `Active { trapped: bool }`.
    let states: [focus_scope::State; 3] = [
        focus_scope::State::Inactive,
        focus_scope::State::Active { trapped: false },
        focus_scope::State::Active { trapped: true },
    ];

    assert_eq!(states.len(), 3);
}

#[test]
fn focus_scope_effect_variants_match_spec_section_1_8() {
    // Spec §1.8 (Effect Contract): four typed effect intents.
    let effects: [focus_scope::Effect; 4] = [
        focus_scope::Effect::FocusTrapListener,
        focus_scope::Effect::FocusFirst,
        focus_scope::Effect::FocusLast,
        focus_scope::Effect::RestoreFocus,
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
    let mut service = Service::<focus_scope::Machine>::new(
        focus_scope::Props::new().id("trap"),
        &Env::default(),
        &focus_scope::Messages,
    );

    drop(service.send(focus_scope::Event::Activate {
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

#[cfg(feature = "i18n")]
#[test]
fn highlight_anatomy_matches_spec() {
    // Highlight's anatomy table (spec §2) lists two rows: `Root` and
    // `Chunk`. Only `Root` is a static `Part` enum variant — `Chunk` is
    // a parametric anatomy slot driven by a runtime boolean and served
    // by `Api::chunk_attrs(highlighted)`, per the convention documented
    // in `foundation/10-component-spec-template.md` §4.2.
    assert_anatomy("highlight", &[(highlight::Part::Root, "root")]);
}
