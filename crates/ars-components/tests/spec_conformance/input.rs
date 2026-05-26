//! Spec-conformance tests for `crates/ars-components/src/input/*`.
//!
//! Each test pulls the expected anatomy from the corresponding component
//! spec's §2 anatomy table and asserts the impl's `Part` enum matches.

use ars_components::input::{
    checkbox, checkbox_group, editable, file_trigger, number_input, password_input, pin_input,
    radio_group, range_slider, search_input, slider, switch, text_field, textarea,
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
fn editable_anatomy_matches_spec() {
    assert_anatomy(
        "editable",
        &[
            (editable::Part::Root, "root"),
            (editable::Part::Label, "label"),
            (editable::Part::Preview, "preview"),
            (editable::Part::Input, "input"),
            (editable::Part::EditTrigger, "edit-trigger"),
            (editable::Part::SubmitTrigger, "submit-trigger"),
            (editable::Part::CancelTrigger, "cancel-trigger"),
        ],
    );
}

#[test]
fn file_trigger_anatomy_matches_spec() {
    assert_anatomy(
        "file-trigger",
        &[
            (file_trigger::Part::Root, "root"),
            (file_trigger::Part::Trigger, "trigger"),
            (file_trigger::Part::Input, "input"),
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
fn slider_anatomy_matches_spec() {
    assert_anatomy(
        "slider",
        &[
            (slider::Part::Root, "root"),
            (slider::Part::Label, "label"),
            (slider::Part::Track, "track"),
            (slider::Part::Range, "range"),
            (slider::Part::Thumb, "thumb"),
            (slider::Part::Output, "output"),
            (slider::Part::MarkerGroup, "marker-group"),
            (slider::Part::Marker { value: 0.0 }, "marker"),
            (slider::Part::HiddenInput, "hidden-input"),
            (slider::Part::DraggingIndicator, "dragging-indicator"),
            (slider::Part::Description, "description"),
            (slider::Part::ErrorMessage, "error-message"),
        ],
    );
}

#[test]
fn range_slider_anatomy_matches_spec() {
    assert_anatomy(
        "range-slider",
        &[
            (range_slider::Part::Root, "root"),
            (range_slider::Part::Label, "label"),
            (range_slider::Part::Track, "track"),
            (range_slider::Part::Range, "range"),
            (
                range_slider::Part::Thumb {
                    thumb: range_slider::ThumbIndex::Start,
                },
                "thumb",
            ),
            (range_slider::Part::Output, "output"),
            (range_slider::Part::MarkerGroup, "marker-group"),
            (range_slider::Part::Marker { value: 0.0 }, "marker"),
            (
                range_slider::Part::HiddenInput {
                    thumb: range_slider::ThumbIndex::Start,
                },
                "hidden-input",
            ),
            (range_slider::Part::DraggingIndicator, "dragging-indicator"),
            (range_slider::Part::Description, "description"),
            (range_slider::Part::ErrorMessage, "error-message"),
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
