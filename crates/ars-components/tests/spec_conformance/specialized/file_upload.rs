use super::{assert_anatomy, specialized_core};

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
