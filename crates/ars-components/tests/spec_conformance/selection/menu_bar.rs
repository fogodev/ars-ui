use ars_components::selection::menu_bar;

use super::{Key, assert_anatomy};

#[test]
fn menu_bar_anatomy_matches_spec() {
    assert_anatomy(
        "menu-bar",
        &[
            (menu_bar::Part::Root, "root"),
            (
                menu_bar::Part::Menu {
                    key: Key::default(),
                },
                "menu",
            ),
            (
                menu_bar::Part::MenuTrigger {
                    key: Key::default(),
                },
                "menu-trigger",
            ),
            (
                menu_bar::Part::MenuPositioner {
                    key: Key::default(),
                },
                "menu-positioner",
            ),
            (
                menu_bar::Part::MenuContent {
                    key: Key::default(),
                },
                "menu-content",
            ),
        ],
    );
}
