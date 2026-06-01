//! Spec-conformance tests for `crates/ars-components/src/specialized/*`.
//!
//! Each test pulls the expected anatomy from the corresponding component
//! spec's §2 anatomy table and asserts the impl's `Part` enum matches.

use ars_components::specialized as specialized_core;

use super::helper::assert_anatomy;

#[test]
fn color_swatch_anatomy_matches_spec() {
    assert_anatomy(
        "color-swatch",
        &[
            (specialized_core::color_swatch::Part::Root, "root"),
            (specialized_core::color_swatch::Part::Inner, "inner"),
        ],
    );
}

#[test]
fn color_field_anatomy_matches_spec() {
    assert_anatomy(
        "color-field",
        &[
            (specialized_core::color_field::Part::Root, "root"),
            (specialized_core::color_field::Part::Label, "label"),
            (specialized_core::color_field::Part::Input, "input"),
            (
                specialized_core::color_field::Part::Description,
                "description",
            ),
            (
                specialized_core::color_field::Part::ErrorMessage,
                "error-message",
            ),
            (
                specialized_core::color_field::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}

#[test]
fn color_area_anatomy_matches_spec() {
    assert_anatomy(
        "color-area",
        &[
            (specialized_core::color_area::Part::Root, "root"),
            (specialized_core::color_area::Part::Background, "background"),
            (specialized_core::color_area::Part::Thumb, "thumb"),
            (
                specialized_core::color_area::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}

#[test]
fn color_slider_anatomy_matches_spec() {
    assert_anatomy(
        "color-slider",
        &[
            (specialized_core::color_slider::Part::Root, "root"),
            (specialized_core::color_slider::Part::Label, "label"),
            (specialized_core::color_slider::Part::Track, "track"),
            (specialized_core::color_slider::Part::Thumb, "thumb"),
            (specialized_core::color_slider::Part::Output, "output"),
            (
                specialized_core::color_slider::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}

#[test]
fn color_wheel_anatomy_matches_spec() {
    assert_anatomy(
        "color-wheel",
        &[
            (specialized_core::color_wheel::Part::Root, "root"),
            (specialized_core::color_wheel::Part::Track, "track"),
            (specialized_core::color_wheel::Part::Thumb, "thumb"),
            (
                specialized_core::color_wheel::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}

#[test]
fn angle_slider_anatomy_matches_spec() {
    assert_anatomy(
        "angle-slider",
        &[
            (specialized_core::angle_slider::Part::Root, "root"),
            (specialized_core::angle_slider::Part::Control, "control"),
            (specialized_core::angle_slider::Part::Track, "track"),
            (specialized_core::angle_slider::Part::Range, "range"),
            (specialized_core::angle_slider::Part::Thumb, "thumb"),
            (
                specialized_core::angle_slider::Part::ValueText,
                "value-text",
            ),
            (
                specialized_core::angle_slider::Part::MarkerGroup,
                "marker-group",
            ),
            (
                specialized_core::angle_slider::Part::Marker { value: 0.0 },
                "marker",
            ),
            (
                specialized_core::angle_slider::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}

#[test]
fn color_swatch_picker_anatomy_matches_spec() {
    assert_anatomy(
        "color-swatch-picker",
        &[
            (specialized_core::color_swatch_picker::Part::Root, "root"),
            (
                specialized_core::color_swatch_picker::Part::Item { index: 0 },
                "item",
            ),
            (
                specialized_core::color_swatch_picker::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}

#[test]
fn clipboard_anatomy_matches_spec() {
    assert_anatomy(
        "clipboard",
        &[
            (specialized_core::clipboard::Part::Root, "root"),
            (specialized_core::clipboard::Part::Label, "label"),
            (specialized_core::clipboard::Part::Trigger, "trigger"),
            (specialized_core::clipboard::Part::Indicator, "indicator"),
            (specialized_core::clipboard::Part::Status, "status"),
            (specialized_core::clipboard::Part::ValueText, "value-text"),
        ],
    );
}

#[test]
fn file_upload_anatomy_matches_spec() {
    assert_anatomy(
        "file-upload",
        &[
            (specialized_core::file_upload::Part::Root, "root"),
            (specialized_core::file_upload::Part::Label, "label"),
            (specialized_core::file_upload::Part::Dropzone, "dropzone"),
            (specialized_core::file_upload::Part::Trigger, "trigger"),
            (specialized_core::file_upload::Part::ItemGroup, "item-group"),
            (
                specialized_core::file_upload::Part::Item { index: 0 },
                "item",
            ),
            (
                specialized_core::file_upload::Part::ItemName { index: 0 },
                "item-name",
            ),
            (
                specialized_core::file_upload::Part::ItemSizeText { index: 0 },
                "item-size-text",
            ),
            (
                specialized_core::file_upload::Part::ItemDeleteTrigger { index: 0 },
                "item-delete-trigger",
            ),
            (
                specialized_core::file_upload::Part::ItemProgress { index: 0 },
                "item-progress",
            ),
            (
                specialized_core::file_upload::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}

#[test]
fn contextual_help_anatomy_matches_spec() {
    assert_anatomy(
        "contextual-help",
        &[
            (specialized_core::contextual_help::Part::Root, "root"),
            (specialized_core::contextual_help::Part::Trigger, "trigger"),
            (specialized_core::contextual_help::Part::Content, "content"),
            (specialized_core::contextual_help::Part::Heading, "heading"),
            (specialized_core::contextual_help::Part::Body, "body"),
            (specialized_core::contextual_help::Part::Footer, "footer"),
            (
                specialized_core::contextual_help::Part::DismissButton,
                "dismiss-button",
            ),
        ],
    );
}

#[test]
fn qr_code_anatomy_matches_spec() {
    assert_anatomy(
        "qr-code",
        &[
            (specialized_core::qr_code::Part::Root, "root"),
            (specialized_core::qr_code::Part::Frame, "frame"),
            (specialized_core::qr_code::Part::Pattern, "pattern"),
            (specialized_core::qr_code::Part::Overlay, "overlay"),
            (
                specialized_core::qr_code::Part::DownloadTrigger,
                "download-trigger",
            ),
        ],
    );
}
