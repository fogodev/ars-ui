//! Property-based regression tests for [`ColorValue`] invariants.
//!
//! These tests are `#[ignore]`d so they do not run in the per-PR fast tier.
//! The nightly workflow runs them with
//! `cargo test -p ars-core -- --ignored proptest` and `PROPTEST_CASES=10000`
//! for extended coverage. Run locally with the same command at the default
//! budget to smoke-test the properties.
//!
//! Property coverage:
//! - constructor clamping / hue wrapping always yields in-range components;
//! - hex round-trips losslessly through `to_hex` / `from_hex`;
//! - RGB round-trips through HSL within a 1-unit-per-channel tolerance;
//! - `with_channel` then `channel_value` returns the value that was set
//!   (within the channel's quantization), and never produces out-of-range
//!   stored components.

use ars_core::{ColorChannel, ColorValue, channel_range, channel_value, with_channel};
use proptest::prelude::*;

/// Strategy: an arbitrary, possibly out-of-range HSLA quadruple.
fn raw_hsla() -> impl Strategy<Value = (f64, f64, f64, f64)> {
    (
        -720.0_f64..720.0,
        -0.5_f64..1.5,
        -0.5_f64..1.5,
        -0.5_f64..1.5,
    )
}

/// Strategy: a valid `ColorValue` built from clamped/​wrapped components.
fn any_color() -> impl Strategy<Value = ColorValue> {
    raw_hsla().prop_map(|(hue, saturation, lightness, alpha)| {
        ColorValue::new(hue, saturation, lightness, alpha)
    })
}

/// Strategy: any of the eight color channels.
fn any_channel() -> impl Strategy<Value = ColorChannel> {
    prop_oneof![
        Just(ColorChannel::Hue),
        Just(ColorChannel::Saturation),
        Just(ColorChannel::Lightness),
        Just(ColorChannel::Brightness),
        Just(ColorChannel::Alpha),
        Just(ColorChannel::Red),
        Just(ColorChannel::Green),
        Just(ColorChannel::Blue),
    ]
}

proptest! {
    /// `ColorValue::new` always stores in-range, wrapped components.
    #[test]
    #[ignore = "proptest: run in nightly extended tier"]
    fn new_clamps_and_wraps((hue, saturation, lightness, alpha) in raw_hsla()) {
        let color = ColorValue::new(hue, saturation, lightness, alpha);

        prop_assert!((0.0..360.0).contains(&color.hue), "hue {} out of range", color.hue);
        prop_assert!((0.0..=1.0).contains(&color.saturation));
        prop_assert!((0.0..=1.0).contains(&color.lightness));
        prop_assert!((0.0..=1.0).contains(&color.alpha));
    }

    /// Hex serialization round-trips losslessly (8-digit preserves alpha to
    /// the nearest 1/255).
    #[test]
    #[ignore = "proptest: run in nightly extended tier"]
    fn hex_round_trip(color in any_color()) {
        let hex = color.to_hex(true);

        let parsed = ColorValue::from_hex(&hex).expect("emitted hex must re-parse");

        prop_assert_eq!(parsed.to_rgba(), color.to_rgba());
    }

    /// RGB survives a trip through HSL storage within one unit per channel.
    #[test]
    #[ignore = "proptest: run in nightly extended tier"]
    fn rgb_hsl_round_trip(red in 0u8..=255, green in 0u8..=255, blue in 0u8..=255) {
        let (out_red, out_green, out_blue) = ColorValue::from_rgb(red, green, blue).to_rgb();

        prop_assert!(
            out_red.abs_diff(red) <= 1
                && out_green.abs_diff(green) <= 1
                && out_blue.abs_diff(blue) <= 1,
            "({red},{green},{blue}) -> ({out_red},{out_green},{out_blue})"
        );
    }

    /// `with_channel` keeps the stored color valid and (for the non-derived
    /// channels) sets the requested value within the channel's quantization.
    #[test]
    #[ignore = "proptest: run in nightly extended tier"]
    fn with_channel_keeps_color_valid(color in any_color(), channel in any_channel()) {
        let (min, max) = channel_range(channel);

        let target = (min + max) / 2.0;

        let updated = with_channel(&color, channel, target);

        prop_assert!((0.0..360.0).contains(&updated.hue));
        prop_assert!((0.0..=1.0).contains(&updated.saturation));
        prop_assert!((0.0..=1.0).contains(&updated.lightness));
        prop_assert!((0.0..=1.0).contains(&updated.alpha));

        // Hue/Saturation/Lightness/Alpha are stored directly, so the read-back
        // is exact. RGB/Brightness route through lossy conversions.
        if matches!(
            channel,
            ColorChannel::Hue
                | ColorChannel::Saturation
                | ColorChannel::Lightness
                | ColorChannel::Alpha
        ) {
            let read = channel_value(&updated, channel);

            prop_assert!((read - target).abs() < 1e-9, "set {target}, read {read}");
        }
    }
}
