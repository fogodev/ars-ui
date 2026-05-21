//! Spec-conformance tests for `crates/ars-components/src/selection/*`.

use ars_collections::Key;
use ars_components::selection::{listbox, select};

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
