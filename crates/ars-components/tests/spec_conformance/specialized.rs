//! Spec-conformance tests for `crates/ars-components/src/specialized/*`.
//!
//! Each test pulls the expected anatomy from the corresponding component
//! spec's §2 anatomy table and asserts the impl's `Part` enum matches.

use ars_components::specialized as specialized_core;

use super::helper::assert_anatomy;

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
