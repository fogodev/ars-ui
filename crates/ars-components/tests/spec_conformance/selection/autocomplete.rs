use ars_components::selection::autocomplete;

use super::{Key, assert_anatomy};

#[test]
fn autocomplete_anatomy_matches_spec() {
    assert_anatomy(
        "autocomplete",
        &[
            (autocomplete::Part::Root, "root"),
            (autocomplete::Part::Input, "input"),
            (autocomplete::Part::ClearTrigger, "clear-trigger"),
            (autocomplete::Part::Content, "content"),
            (
                autocomplete::Part::Item {
                    key: Key::default(),
                },
                "item",
            ),
            (
                autocomplete::Part::ItemText {
                    key: Key::default(),
                },
                "item-text",
            ),
            (autocomplete::Part::EmptyState, "empty-state"),
            (autocomplete::Part::LoadingIndicator, "loading-indicator"),
            (autocomplete::Part::LiveRegion, "live-region"),
        ],
    );
}
