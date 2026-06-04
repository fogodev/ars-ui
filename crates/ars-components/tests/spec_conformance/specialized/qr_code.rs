use super::{assert_anatomy, specialized_core};

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
