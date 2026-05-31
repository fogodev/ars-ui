use ars_collections::Key;
use ars_components::navigation::accordion;

use crate::helper::assert_anatomy;

#[test]
fn accordion_anatomy_matches_spec() {
    assert_anatomy(
        "accordion",
        &[
            (accordion::Part::Root, "root"),
            (
                accordion::Part::Item {
                    item_key: Key::default(),
                },
                "item",
            ),
            (
                accordion::Part::ItemHeader {
                    item_key: Key::default(),
                },
                "item-header",
            ),
            (
                accordion::Part::ItemTrigger {
                    item_key: Key::default(),
                },
                "item-trigger",
            ),
            (
                accordion::Part::ItemIndicator {
                    item_key: Key::default(),
                },
                "item-indicator",
            ),
            (
                accordion::Part::ItemContent {
                    item_key: Key::default(),
                },
                "item-content",
            ),
        ],
    );
}
