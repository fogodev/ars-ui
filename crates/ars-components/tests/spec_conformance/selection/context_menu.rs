use ars_components::selection::context_menu;

use super::{Key, assert_anatomy};

#[test]
fn context_menu_anatomy_matches_spec() {
    assert_anatomy(
        "context-menu",
        &[
            (context_menu::Part::Root, "root"),
            (context_menu::Part::Target, "target"),
            (context_menu::Part::Positioner, "positioner"),
            (context_menu::Part::Arrow, "arrow"),
            (context_menu::Part::Content, "content"),
            (
                context_menu::Part::ItemGroup {
                    key: Key::default(),
                },
                "item-group",
            ),
            (
                context_menu::Part::ItemGroupLabel {
                    key: Key::default(),
                },
                "item-group-label",
            ),
            (
                context_menu::Part::Item {
                    key: Key::default(),
                },
                "item",
            ),
            (
                context_menu::Part::ItemText {
                    key: Key::default(),
                },
                "item-text",
            ),
            (
                context_menu::Part::ItemIndicator {
                    key: Key::default(),
                },
                "item-indicator",
            ),
            (context_menu::Part::Separator, "separator"),
            (
                context_menu::Part::CheckboxItem {
                    key: Key::default(),
                },
                "checkbox-item",
            ),
            (
                context_menu::Part::RadioGroup {
                    group: Key::default(),
                },
                "radio-group",
            ),
            (
                context_menu::Part::RadioItem {
                    key: Key::default(),
                    group: Key::default(),
                },
                "radio-item",
            ),
            (
                context_menu::Part::SubTrigger {
                    key: Key::default(),
                },
                "sub-trigger",
            ),
            (
                context_menu::Part::SubPositioner {
                    key: Key::default(),
                },
                "sub-positioner",
            ),
            (
                context_menu::Part::SubContent {
                    key: Key::default(),
                },
                "sub-content",
            ),
            (
                context_menu::Part::Shortcut {
                    key: Key::default(),
                },
                "shortcut",
            ),
        ],
    );
}
