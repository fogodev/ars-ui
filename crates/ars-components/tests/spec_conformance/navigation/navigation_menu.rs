use ars_collections::Key;
use ars_components::navigation::navigation_menu;

use crate::helper::assert_anatomy;

#[test]
fn navigation_menu_anatomy_matches_spec() {
    assert_anatomy(
        "navigation-menu",
        &[
            (navigation_menu::Part::Root, "root"),
            (navigation_menu::Part::List, "list"),
            (
                navigation_menu::Part::Item {
                    item_key: Key::default(),
                },
                "item",
            ),
            (
                navigation_menu::Part::Trigger {
                    item_key: Key::default(),
                    content_id: String::new(),
                },
                "trigger",
            ),
            (
                navigation_menu::Part::Content {
                    item_key: Key::default(),
                },
                "content",
            ),
            (navigation_menu::Part::Link { active: false }, "link"),
            (navigation_menu::Part::Indicator, "indicator"),
            (navigation_menu::Part::Viewport, "viewport"),
            (navigation_menu::Part::Sub, "sub"),
            (navigation_menu::Part::SubList, "sub-list"),
            (
                navigation_menu::Part::SubItem {
                    item_key: Key::default(),
                },
                "sub-item",
            ),
            (
                navigation_menu::Part::SubTrigger {
                    item_key: Key::default(),
                    content_id: String::new(),
                },
                "sub-trigger",
            ),
            (
                navigation_menu::Part::SubContent {
                    item_key: Key::default(),
                },
                "sub-content",
            ),
            (navigation_menu::Part::SubIndicator, "sub-indicator"),
            (navigation_menu::Part::SubViewport, "sub-viewport"),
        ],
    );
}
