//! Spec-conformance tests for `crates/ars-components/src/input/*`.
//!
//! Each test pulls the expected anatomy from the corresponding component
//! spec's §2 anatomy table and asserts the impl's `Part` enum matches.

use ars_components::input::{
    checkbox, checkbox_group, number_input, password_input, pin_input, radio_group, search_input,
    switch, text_field, textarea,
};

use super::helper::assert_anatomy;

#[test]
fn checkbox_anatomy_matches_spec() {
    assert_anatomy(
        "checkbox",
        &[
            (checkbox::Part::Root, "root"),
            (checkbox::Part::Label, "label"),
            (checkbox::Part::Control, "control"),
            (checkbox::Part::Indicator, "indicator"),
            (checkbox::Part::HiddenInput, "hidden-input"),
            (checkbox::Part::Description, "description"),
            (checkbox::Part::ErrorMessage, "error-message"),
        ],
    );
}

#[test]
fn checkbox_group_anatomy_matches_spec() {
    assert_anatomy(
        "checkbox-group",
        &[
            (checkbox_group::Part::Root, "root"),
            (checkbox_group::Part::Label, "label"),
            (checkbox_group::Part::Description, "description"),
            (checkbox_group::Part::ErrorMessage, "error-message"),
        ],
    );
}

#[test]
fn radio_group_anatomy_matches_spec() {
    assert_anatomy(
        "radio-group",
        &[
            (radio_group::Part::Root, "root"),
            (radio_group::Part::Label, "label"),
            (
                radio_group::Part::Item {
                    item_value: Default::default(),
                },
                "item",
            ),
            (
                radio_group::Part::ItemControl {
                    item_value: Default::default(),
                },
                "item-control",
            ),
            (
                radio_group::Part::ItemIndicator {
                    item_value: Default::default(),
                },
                "item-indicator",
            ),
            (
                radio_group::Part::ItemLabel {
                    item_value: Default::default(),
                },
                "item-label",
            ),
            (
                radio_group::Part::ItemHiddenInput {
                    item_value: Default::default(),
                },
                "item-hidden-input",
            ),
            (radio_group::Part::Description, "description"),
            (radio_group::Part::ErrorMessage, "error-message"),
        ],
    );
}

#[test]
fn switch_anatomy_matches_spec() {
    assert_anatomy(
        "switch",
        &[
            (switch::Part::Root, "root"),
            (switch::Part::Label, "label"),
            (switch::Part::Control, "control"),
            (switch::Part::Thumb, "thumb"),
            (switch::Part::HiddenInput, "hidden-input"),
            (switch::Part::Description, "description"),
            (switch::Part::ErrorMessage, "error-message"),
        ],
    );
}

#[test]
fn text_field_anatomy_matches_spec() {
    assert_anatomy(
        "text-field",
        &[
            (text_field::Part::Root, "root"),
            (text_field::Part::Label, "label"),
            (text_field::Part::Input, "input"),
            (text_field::Part::StartDecorator, "start-decorator"),
            (text_field::Part::EndDecorator, "end-decorator"),
            (text_field::Part::ClearTrigger, "clear-trigger"),
            (text_field::Part::Description, "description"),
            (text_field::Part::ErrorMessage, "error-message"),
        ],
    );
}

#[test]
fn textarea_anatomy_matches_spec() {
    assert_anatomy(
        "textarea",
        &[
            (textarea::Part::Root, "root"),
            (textarea::Part::Label, "label"),
            (textarea::Part::Textarea, "textarea"),
            (textarea::Part::CharacterCount, "character-count"),
            (textarea::Part::Description, "description"),
            (textarea::Part::ErrorMessage, "error-message"),
        ],
    );
}

#[test]
fn password_input_anatomy_matches_spec() {
    assert_anatomy(
        "password-input",
        &[
            (password_input::Part::Root, "root"),
            (password_input::Part::Label, "label"),
            (password_input::Part::Input, "input"),
            (password_input::Part::Toggle, "toggle"),
            (password_input::Part::Description, "description"),
            (password_input::Part::ErrorMessage, "error-message"),
        ],
    );
}

#[test]
fn search_input_anatomy_matches_spec() {
    assert_anatomy(
        "search-input",
        &[
            (search_input::Part::Root, "root"),
            (search_input::Part::Label, "label"),
            (search_input::Part::Input, "input"),
            (search_input::Part::ClearTrigger, "clear-trigger"),
            (search_input::Part::SubmitTrigger, "submit-trigger"),
            (search_input::Part::LoadingIndicator, "loading-indicator"),
            (search_input::Part::Description, "description"),
            (search_input::Part::ErrorMessage, "error-message"),
        ],
    );
}

#[test]
fn number_input_anatomy_matches_spec() {
    assert_anatomy(
        "number-input",
        &[
            (number_input::Part::Root, "root"),
            (number_input::Part::Label, "label"),
            (number_input::Part::Input, "input"),
            (number_input::Part::IncrementTrigger, "increment-trigger"),
            (number_input::Part::DecrementTrigger, "decrement-trigger"),
            (number_input::Part::Description, "description"),
            (number_input::Part::ErrorMessage, "error-message"),
        ],
    );
}

#[test]
fn pin_input_anatomy_matches_spec() {
    assert_anatomy(
        "pin-input",
        &[
            (pin_input::Part::Root, "root"),
            (pin_input::Part::Label, "label"),
            (pin_input::Part::Input { cell_index: 0 }, "input"),
            (pin_input::Part::HiddenInput, "hidden-input"),
            (pin_input::Part::Description, "description"),
            (pin_input::Part::ErrorMessage, "error-message"),
        ],
    );
}
