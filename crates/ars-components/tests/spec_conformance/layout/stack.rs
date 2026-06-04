use ars_components::layout::stack;

use super::*;

#[test]
fn stack_anatomy_matches_spec() {
    assert_anatomy("stack", &[(stack::Part::Root, "root")]);
}
