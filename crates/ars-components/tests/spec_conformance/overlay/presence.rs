use ars_components::overlay::presence;
use ars_core::{Env, HtmlAttr, Service};

use crate::helper::assert_anatomy;

#[test]
fn presence_anatomy_matches_spec() {
    assert_anatomy("presence", &[(presence::Part::Root, "root")]);
}

#[test]
fn presence_root_attrs_carry_state_and_phase_tokens() {
    let service = Service::<presence::Machine>::new(
        presence::Props::new().id("presence").present(true),
        &Env::default(),
        &presence::Messages,
    );

    let attrs = service.connect(&|_| {}).root_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("open"));
    assert_eq!(attrs.get(&HtmlAttr::Data("ars-presence")), Some("mounted"));
}
