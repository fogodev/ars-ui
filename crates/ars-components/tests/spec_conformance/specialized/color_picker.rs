use ars_core::ColorChannel;

use super::{assert_anatomy, specialized_core};

#[test]
fn color_picker_anatomy_matches_spec() {
    use specialized_core::color_picker::Part;

    assert_anatomy(
        "color-picker",
        &[
            (Part::Root, "root"),
            (Part::Label, "label"),
            (Part::Control, "control"),
            (Part::Trigger, "trigger"),
            (Part::Content, "content"),
            (Part::Area, "area"),
            (Part::AreaThumb, "area-thumb"),
            (
                Part::ChannelSlider {
                    channel: ColorChannel::Hue,
                },
                "channel-slider",
            ),
            (
                Part::ChannelSliderThumb {
                    channel: ColorChannel::Hue,
                },
                "channel-slider-thumb",
            ),
            (Part::AlphaSlider, "alpha-slider"),
            (Part::SwatchGroup, "swatch-group"),
            (Part::Swatch { index: 0 }, "swatch"),
            (Part::FormatSelect, "format-select"),
            (
                Part::ChannelInput {
                    channel: ColorChannel::Hue,
                    index: 0,
                },
                "channel-input",
            ),
            (Part::HexInput, "hex-input"),
            (Part::EyeDropperTrigger, "eye-dropper-trigger"),
            (Part::HiddenInput, "hidden-input"),
        ],
    );
}
