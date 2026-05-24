//! Spec-conformance tests for `crates/ars-components/src/selection/*`.

use ars_collections::Key;
use ars_components::selection::{combobox, listbox, menu, select};

use super::helper::assert_anatomy;

#[test]
fn listbox_anatomy_matches_spec() {
    assert_anatomy(
        "listbox",
        &[
            (listbox::Part::Root, "root"),
            (listbox::Part::Label, "label"),
            (listbox::Part::Content, "content"),
            (
                listbox::Part::ItemGroup {
                    key: Key::default(),
                },
                "item-group",
            ),
            (
                listbox::Part::ItemGroupLabel {
                    key: Key::default(),
                },
                "item-group-label",
            ),
            (
                listbox::Part::Item {
                    key: Key::default(),
                },
                "item",
            ),
            (
                listbox::Part::ItemText {
                    key: Key::default(),
                },
                "item-text",
            ),
            (
                listbox::Part::ItemIndicator {
                    key: Key::default(),
                },
                "item-indicator",
            ),
            (listbox::Part::Description, "description"),
            (listbox::Part::ErrorMessage, "error-message"),
            (listbox::Part::LoadingSentinel, "loading-sentinel"),
        ],
    );
}

#[test]
fn select_anatomy_matches_spec() {
    assert_anatomy(
        "select",
        &[
            (select::Part::Root, "root"),
            (select::Part::Label, "label"),
            (select::Part::Control, "control"),
            (select::Part::Trigger, "trigger"),
            (select::Part::ValueText, "value-text"),
            (select::Part::Indicator, "indicator"),
            (select::Part::ClearTrigger, "clear-trigger"),
            (select::Part::Positioner, "positioner"),
            (select::Part::Content, "content"),
            (
                select::Part::ItemGroup {
                    key: Key::default(),
                },
                "item-group",
            ),
            (
                select::Part::ItemGroupLabel {
                    key: Key::default(),
                },
                "item-group-label",
            ),
            (
                select::Part::Item {
                    key: Key::default(),
                },
                "item",
            ),
            (
                select::Part::ItemText {
                    key: Key::default(),
                },
                "item-text",
            ),
            (
                select::Part::ItemIndicator {
                    key: Key::default(),
                },
                "item-indicator",
            ),
            (select::Part::HiddenInput, "hidden-input"),
            (select::Part::Description, "description"),
            (select::Part::ErrorMessage, "error-message"),
            (select::Part::EmptyState, "empty-state"),
        ],
    );
}

#[test]
fn combobox_anatomy_matches_spec() {
    assert_anatomy(
        "combobox",
        &[
            (combobox::Part::Root, "root"),
            (combobox::Part::Label, "label"),
            (combobox::Part::Control, "control"),
            (combobox::Part::Input, "input"),
            (combobox::Part::Trigger, "trigger"),
            (combobox::Part::ClearTrigger, "clear-trigger"),
            (combobox::Part::Positioner, "positioner"),
            (combobox::Part::Content, "content"),
            (
                combobox::Part::ItemGroup {
                    key: Key::default(),
                },
                "item-group",
            ),
            (
                combobox::Part::ItemGroupLabel {
                    key: Key::default(),
                },
                "item-group-label",
            ),
            (
                combobox::Part::Item {
                    key: Key::default(),
                },
                "item",
            ),
            (
                combobox::Part::ItemText {
                    key: Key::default(),
                },
                "item-text",
            ),
            (
                combobox::Part::ItemIndicator {
                    key: Key::default(),
                },
                "item-indicator",
            ),
            (combobox::Part::Empty, "empty"),
            (combobox::Part::Description, "description"),
            (combobox::Part::ErrorMessage, "error-message"),
            (combobox::Part::LiveRegion, "live-region"),
        ],
    );
}

#[test]
fn menu_anatomy_matches_spec() {
    assert_anatomy(
        "menu",
        &[
            (menu::Part::Root, "root"),
            (menu::Part::Trigger, "trigger"),
            (menu::Part::Positioner, "positioner"),
            (menu::Part::Arrow, "arrow"),
            (menu::Part::Content, "content"),
            (
                menu::Part::ItemGroup {
                    key: Key::default(),
                },
                "item-group",
            ),
            (
                menu::Part::ItemGroupLabel {
                    key: Key::default(),
                },
                "item-group-label",
            ),
            (
                menu::Part::Item {
                    key: Key::default(),
                },
                "item",
            ),
            (
                menu::Part::ItemText {
                    key: Key::default(),
                },
                "item-text",
            ),
            (
                menu::Part::ItemIndicator {
                    key: Key::default(),
                },
                "item-indicator",
            ),
            (menu::Part::Separator, "separator"),
            (
                menu::Part::CheckboxItem {
                    key: Key::default(),
                },
                "checkbox-item",
            ),
            (
                menu::Part::RadioGroup {
                    group: Key::default(),
                },
                "radio-group",
            ),
            (
                menu::Part::RadioItem {
                    key: Key::default(),
                    group: Key::default(),
                },
                "radio-item",
            ),
            (
                menu::Part::SubTrigger {
                    key: Key::default(),
                },
                "sub-trigger",
            ),
            (
                menu::Part::SubPositioner {
                    key: Key::default(),
                },
                "sub-positioner",
            ),
            (
                menu::Part::SubContent {
                    key: Key::default(),
                },
                "sub-content",
            ),
            (
                menu::Part::Shortcut {
                    key: Key::default(),
                },
                "shortcut",
            ),
        ],
    );
}
