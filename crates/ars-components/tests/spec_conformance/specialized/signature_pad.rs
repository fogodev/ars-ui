use super::{assert_anatomy, specialized_core};

#[test]
fn signature_pad_anatomy_matches_spec() {
    assert_anatomy(
        "signature-pad",
        &[
            (specialized_core::signature_pad::Part::Root, "root"),
            (specialized_core::signature_pad::Part::Canvas, "canvas"),
            (
                specialized_core::signature_pad::Part::ClearTrigger,
                "clear-trigger",
            ),
            (
                specialized_core::signature_pad::Part::UndoTrigger,
                "undo-trigger",
            ),
            (specialized_core::signature_pad::Part::Label, "label"),
            (specialized_core::signature_pad::Part::Guide, "guide"),
            (
                specialized_core::signature_pad::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}
