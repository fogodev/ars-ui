use super::{assert_anatomy, specialized_core};

#[test]
fn image_cropper_anatomy_matches_spec() {
    use specialized_core::image_cropper::{CropHandle, Part};

    assert_anatomy(
        "image-cropper",
        &[
            (Part::Root, "root"),
            (Part::Image, "image"),
            (Part::Overlay, "overlay"),
            (Part::CropArea, "crop-area"),
            (Part::Grid, "grid"),
            (
                Part::Handle {
                    position: CropHandle::TopLeft,
                },
                "handle",
            ),
            (Part::ZoomSlider, "zoom-slider"),
            (Part::RotationSlider, "rotation-slider"),
            (Part::ResetTrigger, "reset-trigger"),
            (Part::Label, "label"),
        ],
    );
}
