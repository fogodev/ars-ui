//! Spec-conformance tests for `crates/ars-components/src/selection/*`.

use ars_collections::Key;
use ars_components::selection::{
    autocomplete, combobox, context_menu, listbox, menu, menu_bar, segment_group, select,
};

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

#[test]
fn segment_group_anatomy_matches_spec() {
    assert_anatomy(
        "segment-group",
        &[
            (segment_group::Part::Root, "root"),
            (
                segment_group::Part::Item {
                    value: Key::default(),
                },
                "item",
            ),
            (
                segment_group::Part::ItemText {
                    value: Key::default(),
                },
                "item-text",
            ),
            (segment_group::Part::Indicator, "indicator"),
            (segment_group::Part::HiddenInput, "hidden-input"),
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
