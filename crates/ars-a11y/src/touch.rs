//! Touch-target sizing and mobile accessibility helpers.

use alloc::format;

use ars_core::{AttrMap, CssProperty, HtmlAttr};

use crate::keyboard::Platform;

/// Minimum recommended touch target size in CSS pixels.
pub const MIN_TOUCH_TARGET_SIZE: f64 = 44.0;

/// Larger touch target for drag-based controls in CSS pixels.
pub const MIN_DRAG_TARGET_SIZE: f64 = 48.0;

/// Returns inline styles that ensure a minimum touch target size while preserving
/// the visual footprint of smaller elements.
///
/// This uses invisible hit-area padding so controls that render smaller than
/// `44x44` CSS pixels still expose an accessible tap target.
#[must_use]
pub fn touch_target_attrs(visual_width: f64, visual_height: f64) -> AttrMap {
    touch_target_attrs_with_min(visual_width, visual_height, MIN_TOUCH_TARGET_SIZE)
}

/// Returns inline styles that ensure a custom minimum touch target size while
/// preserving the visual footprint of smaller elements.
///
/// This uses padding to extend the tap area and a matching negative margin to
/// cancel the layout effect.
#[must_use]
pub fn touch_target_attrs_with_min(visual_width: f64, visual_height: f64, min: f64) -> AttrMap {
    let mut attrs = AttrMap::new();

    let h_padding = ((min - visual_width) / 2.0).max(0.0);
    let v_padding = ((min - visual_height) / 2.0).max(0.0);

    if h_padding > 0.0 || v_padding > 0.0 {
        attrs.set_style(CssProperty::Padding, format!("{v_padding}px {h_padding}px"));

        // Negative margin cancels the padding's layout footprint while keeping
        // the hit area expanded for touch interaction.
        attrs.set_style(
            CssProperty::Margin,
            format!("-{v_padding}px -{h_padding}px"),
        );
    }

    attrs
}

/// Returns whether mobile screen-reader environments should prefer roving
/// tabindex over `aria-activedescendant`.
///
/// `VoiceOver` on iOS does not reliably support `aria-activedescendant`, so ars-ui
/// falls back to roving tabindex when the detected platform is iOS or iPadOS.
#[must_use]
pub const fn should_use_roving_tabindex_for_mobile(platform: Platform) -> bool {
    matches!(platform, Platform::IOS)
}

/// `inputmode` values used to request mobile virtual keyboard layouts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    /// Do not show a virtual keyboard automatically.
    None,
    /// Show the default text keyboard.
    Text,
    /// Show a telephone keypad.
    Tel,
    /// Show a URL-oriented keyboard.
    Url,
    /// Show an email-oriented keyboard.
    Email,
    /// Show a numeric keyboard.
    Numeric,
    /// Show a decimal keypad.
    Decimal,
    /// Show a keyboard optimized for search entry.
    Search,
}

impl InputMode {
    /// Returns the HTML `inputmode` token for this variant.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Text => "text",
            Self::Tel => "tel",
            Self::Url => "url",
            Self::Email => "email",
            Self::Numeric => "numeric",
            Self::Decimal => "decimal",
            Self::Search => "search",
        }
    }

    /// Applies this input mode to an [`AttrMap`] using [`HtmlAttr::InputMode`].
    pub fn apply_to(self, attrs: &mut AttrMap) {
        attrs.set(HtmlAttr::InputMode, self.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style_value<'a>(attrs: &'a AttrMap, property: &CssProperty) -> Option<&'a str> {
        attrs
            .styles()
            .iter()
            .find_map(|(candidate, value)| (candidate == property).then_some(value.as_str()))
    }

    fn parse_spacing_pair(value: &str) -> (f64, f64) {
        let mut parts = value.split_whitespace().map(|part| {
            part.trim_end_matches("px")
                .parse::<f64>()
                .expect("spacing values must be parseable CSS pixels")
        });

        let first = parts.next().expect("spacing value must have first axis");
        let second = parts.next().expect("spacing value must have second axis");
        assert!(
            parts.next().is_none(),
            "spacing value must have exactly two axes"
        );

        (first, second)
    }

    #[test]
    fn min_touch_target_size_is_44_css_pixels() {
        assert_eq!(MIN_TOUCH_TARGET_SIZE, 44.0);
    }

    #[test]
    fn min_drag_target_size_is_48_css_pixels() {
        assert_eq!(MIN_DRAG_TARGET_SIZE, 48.0);
    }

    #[test]
    fn touch_target_attrs_adds_padding_and_negative_margin_for_small_targets() {
        let attrs = touch_target_attrs(24.0, 24.0);

        assert_eq!(
            parse_spacing_pair(style_value(&attrs, &CssProperty::Padding).expect("padding style")),
            (10.0, 10.0)
        );
        assert_eq!(
            parse_spacing_pair(style_value(&attrs, &CssProperty::Margin).expect("margin style")),
            (-10.0, -10.0)
        );
    }

    #[test]
    fn touch_target_attrs_skips_styles_when_target_already_meets_minimum() {
        let attrs = touch_target_attrs(44.0, 44.0);

        assert!(attrs.styles().is_empty());
    }

    #[test]
    fn touch_target_attrs_only_pads_short_axis() {
        let attrs = touch_target_attrs(60.0, 30.0);

        assert_eq!(
            parse_spacing_pair(style_value(&attrs, &CssProperty::Padding).expect("padding style")),
            (7.0, 0.0)
        );
        assert_eq!(
            parse_spacing_pair(style_value(&attrs, &CssProperty::Margin).expect("margin style")),
            (-7.0, 0.0)
        );
    }

    #[test]
    fn touch_target_attrs_only_pads_narrow_axis() {
        let attrs = touch_target_attrs(30.0, 60.0);

        assert_eq!(
            parse_spacing_pair(style_value(&attrs, &CssProperty::Padding).expect("padding style")),
            (0.0, 7.0)
        );
        assert_eq!(
            parse_spacing_pair(style_value(&attrs, &CssProperty::Margin).expect("margin style")),
            (0.0, -7.0)
        );
    }

    #[test]
    fn touch_target_attrs_with_min_uses_custom_minimum_size() {
        let attrs = touch_target_attrs_with_min(24.0, 24.0, 48.0);

        assert_eq!(
            parse_spacing_pair(style_value(&attrs, &CssProperty::Padding).expect("padding style")),
            (12.0, 12.0)
        );
        assert_eq!(
            parse_spacing_pair(style_value(&attrs, &CssProperty::Margin).expect("margin style")),
            (-12.0, -12.0)
        );
    }

    #[test]
    fn touch_target_attrs_with_min_skips_styles_when_target_already_meets_custom_minimum() {
        let attrs = touch_target_attrs_with_min(60.0, 60.0, 48.0);

        assert!(attrs.styles().is_empty());
    }

    #[test]
    fn should_use_roving_tabindex_for_ios() {
        assert!(should_use_roving_tabindex_for_mobile(Platform::IOS));
    }

    #[test]
    fn should_not_use_roving_tabindex_for_macos() {
        assert!(!should_use_roving_tabindex_for_mobile(Platform::MacOs));
    }

    #[test]
    fn should_not_use_roving_tabindex_for_windows() {
        assert!(!should_use_roving_tabindex_for_mobile(Platform::Windows));
    }

    #[test]
    fn should_not_use_roving_tabindex_for_linux() {
        assert!(!should_use_roving_tabindex_for_mobile(Platform::Linux));
    }

    #[test]
    fn should_not_use_roving_tabindex_for_unknown_platforms() {
        assert!(!should_use_roving_tabindex_for_mobile(Platform::Unknown));
    }

    #[test]
    fn input_mode_numeric_as_str_returns_numeric() {
        assert_eq!(InputMode::Numeric.as_str(), "numeric");
    }

    #[test]
    fn input_mode_none_as_str_returns_none() {
        assert_eq!(InputMode::None.as_str(), "none");
    }

    #[test]
    fn input_mode_apply_to_sets_inputmode_attribute() {
        let mut attrs = AttrMap::new();

        InputMode::Tel.apply_to(&mut attrs);

        assert_eq!(attrs.get(&HtmlAttr::InputMode), Some("tel"));
    }

    #[test]
    fn all_input_modes_roundtrip_through_as_str() {
        let cases = [
            (InputMode::None, "none"),
            (InputMode::Text, "text"),
            (InputMode::Tel, "tel"),
            (InputMode::Url, "url"),
            (InputMode::Email, "email"),
            (InputMode::Numeric, "numeric"),
            (InputMode::Decimal, "decimal"),
            (InputMode::Search, "search"),
        ];

        for (mode, expected) in cases {
            assert_eq!(mode.as_str(), expected);
        }
    }
}
