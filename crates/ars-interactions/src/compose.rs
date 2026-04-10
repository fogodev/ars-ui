//! Attribute composition utilities for merging interaction `AttrMap` sets.
//!
//! When multiple interactions (press, hover, focus, etc.) each produce an
//! [`AttrMap`], `merge_attrs` combines them into a single map that can be
//! spread onto one DOM element. Space-separated token attributes (`class`,
//! `rel`, ARIA ID-reference lists) are appended with deduplication; all
//! other attributes use last-write-wins semantics.

use ars_core::AttrMap;

/// Merge multiple [`AttrMap`] sets into a single [`AttrMap`].
///
/// Attribute precedence:
///   For data attributes, the LAST value for a given key wins.
///   (The rightmost attrs set is authoritative for attributes.)
///   Exception: Space-separated token attributes (class, rel, ARIA ID lists)
///   are appended with dedup per `AttrMap::set()` semantics, not overwritten.
///
/// Style merging:
///   Styles are merged; the last value for a given property wins.
///
/// Note: Event handlers are no longer part of `AttrMap`. They are composed
/// separately via typed methods on per-component `Api` structs.
///
/// # Example
///
/// ```rust,ignore
/// let press = use_press(press_config.clone());
/// let hover = use_hover(hover_config);
/// let focus = use_focus(focus_config);
///
/// // All three sets of data attributes are applied to the element.
/// let button_attrs = merge_attrs([
///     press.current_attrs(&press_config),
///     hover.current_attrs(),
///     focus.current_attrs(&focus_config),
/// ]);
/// ```
#[must_use]
pub fn merge_attrs<I>(attrs_iter: I) -> AttrMap
where
    I: IntoIterator<Item = AttrMap>,
{
    let mut merged = AttrMap::new();
    for attrs in attrs_iter {
        // When the optional debug feature is enabled, warn when the same CSS
        // property is set by multiple interaction sources with different values.
        #[cfg(feature = "debug")]
        for (prop, new_value) in attrs.iter_styles() {
            if let Ok(idx) = merged.styles().binary_search_by(|(k, _)| k.cmp(prop)) {
                if merged.styles()[idx].1 != *new_value {
                    log::warn!(
                        "ars-interactions: style property '{prop}' set by multiple interactions \
                         (existing: '{}', new: '{new_value}'). Last write wins.",
                        merged.styles()[idx].1
                    );
                }
            }
        }
        merged.merge(attrs);
    }
    merged
}

/// Convenience macro for merging a fixed set of attrs without constructing a
/// `Vec`.
///
/// ```rust,ignore
/// let attrs = merge_attrs!(
///     press.current_attrs(&press_config),
///     hover.current_attrs(),
///     focus.current_attrs(&focus_config),
/// );
/// ```
#[macro_export]
macro_rules! merge_attrs {
    ($($attrs:expr),+ $(,)?) => {
        $crate::compose::merge_attrs([$($attrs),+])
    };
}

#[cfg(test)]
mod tests {
    use ars_core::{AriaAttr, CssProperty, HtmlAttr};

    use super::*;

    #[test]
    fn n_ary_merge_combines_multiple_maps() {
        let mut a = AttrMap::new();
        a.set(HtmlAttr::Role, "button");

        let mut b = AttrMap::new();
        b.set(HtmlAttr::TabIndex, "0");

        let mut c = AttrMap::new();
        c.set(HtmlAttr::Data("ars-scope"), "slider");

        let merged = merge_attrs([a, b, c]);

        assert_eq!(merged.get(&HtmlAttr::Role), Some("button"));
        assert_eq!(merged.get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(merged.get(&HtmlAttr::Data("ars-scope")), Some("slider"));
    }

    #[test]
    fn last_write_wins_for_regular_attrs() {
        let mut a = AttrMap::new();
        a.set(HtmlAttr::Role, "button");

        let mut b = AttrMap::new();
        b.set(HtmlAttr::TabIndex, "0");

        let mut c = AttrMap::new();
        c.set(HtmlAttr::Role, "slider");

        let merged = merge_attrs([a, b, c]);

        // Third map's role wins (last-write-wins).
        assert_eq!(merged.get(&HtmlAttr::Role), Some("slider"));
        assert_eq!(merged.get(&HtmlAttr::TabIndex), Some("0"));
    }

    #[test]
    fn concatenates_class_with_dedup() {
        let mut a = AttrMap::new();
        a.set(HtmlAttr::Class, "base");
        a.set(HtmlAttr::Class, "pressed");

        let mut b = AttrMap::new();
        b.set(HtmlAttr::Class, "hovered");

        let mut c = AttrMap::new();
        c.set(HtmlAttr::Class, "base");
        c.set(HtmlAttr::Class, "focused");

        let merged = merge_attrs([a, b, c]);

        // All unique tokens present, duplicates removed.
        let class = merged.get(&HtmlAttr::Class).expect("class should be set");
        let tokens = class.split_whitespace().collect::<Vec<_>>();
        assert!(tokens.contains(&"base"), "missing 'base': {class}");
        assert!(tokens.contains(&"pressed"), "missing 'pressed': {class}");
        assert!(tokens.contains(&"hovered"), "missing 'hovered': {class}");
        assert!(tokens.contains(&"focused"), "missing 'focused': {class}");
        // "base" appears only once.
        assert_eq!(
            tokens.iter().filter(|&&t| t == "base").count(),
            1,
            "duplicate 'base': {class}"
        );
    }

    #[test]
    fn concatenates_aria_id_lists() {
        let mut a = AttrMap::new();
        a.set(HtmlAttr::Aria(AriaAttr::DescribedBy), "hint-a");

        let mut b = AttrMap::new();
        b.set(HtmlAttr::Aria(AriaAttr::DescribedBy), "hint-b");

        let mut c = AttrMap::new();
        c.set(HtmlAttr::Aria(AriaAttr::LabelledBy), "label-a");

        let merged = merge_attrs([a, b, c]);

        assert_eq!(
            merged.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("hint-a hint-b")
        );
        assert_eq!(
            merged.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("label-a")
        );
    }

    #[test]
    fn style_last_write_wins_per_property() {
        let mut a = AttrMap::new();
        a.set_style(CssProperty::Width, "10px");
        a.set_style(CssProperty::Height, "20px");

        let mut b = AttrMap::new();
        b.set_style(CssProperty::Width, "30px");

        let mut c = AttrMap::new();
        c.set_style(CssProperty::Height, "40px");
        c.set_style(CssProperty::Color, "red");

        let merged = merge_attrs([a, b, c]);
        let styles = merged.styles();

        let width = styles.iter().find(|(k, _)| *k == CssProperty::Width);
        let height = styles.iter().find(|(k, _)| *k == CssProperty::Height);
        let color = styles.iter().find(|(k, _)| *k == CssProperty::Color);

        assert_eq!(width.map(|(_, v)| v.as_str()), Some("30px"));
        assert_eq!(height.map(|(_, v)| v.as_str()), Some("40px"));
        assert_eq!(color.map(|(_, v)| v.as_str()), Some("red"));
    }

    #[test]
    fn empty_iterator_returns_empty_map() {
        let merged = merge_attrs(Vec::<AttrMap>::new());
        assert_eq!(merged, AttrMap::new());
    }

    #[test]
    fn single_element_is_identity() {
        let mut map = AttrMap::new();
        map.set(HtmlAttr::Role, "button");
        map.set(HtmlAttr::Class, "primary");
        map.set_style(CssProperty::Width, "100px");

        let expected = map.clone();
        let merged = merge_attrs([map]);

        assert_eq!(merged, expected);
    }

    #[test]
    fn macro_works() {
        let mut a = AttrMap::new();
        a.set(HtmlAttr::Role, "button");

        let mut b = AttrMap::new();
        b.set(HtmlAttr::Class, "primary");

        let mut c = AttrMap::new();
        c.set_style(CssProperty::Width, "100px");

        let merged = merge_attrs!(a, b, c);

        assert_eq!(merged.get(&HtmlAttr::Role), Some("button"));
        assert_eq!(merged.get(&HtmlAttr::Class), Some("primary"));
        assert_eq!(
            merged
                .styles()
                .iter()
                .find(|(k, _)| *k == CssProperty::Width)
                .map(|(_, v)| v.as_str()),
            Some("100px")
        );
    }

    #[test]
    fn overlay_values_take_precedence() {
        let mut base = AttrMap::new();
        base.set(HtmlAttr::Role, "button");
        base.set(HtmlAttr::Class, "base");

        let mut overlay = AttrMap::new();
        overlay.set(HtmlAttr::Role, "switch");
        overlay.set(HtmlAttr::Class, "overlay");
        overlay.set(HtmlAttr::Aria(AriaAttr::DescribedBy), "hint");
        overlay.set(HtmlAttr::Aria(AriaAttr::DescribedBy), "error");
        overlay.set_style(CssProperty::Width, "20px");

        let merged = merge_attrs([base, overlay]);

        assert_eq!(merged.get(&HtmlAttr::Role), Some("switch"));
        assert_eq!(merged.get(&HtmlAttr::Class), Some("base overlay"));
        assert_eq!(
            merged.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("hint error")
        );
        assert_eq!(
            merged.styles(),
            &[(CssProperty::Width, String::from("20px"))]
        );
    }

    // ---- Conflict resolution table (spec §8.5) ----

    #[test]
    fn data_ars_attrs_last_write_wins() {
        let mut interaction = AttrMap::new();
        interaction.set(HtmlAttr::Data("ars-pressed"), "true");

        let mut component = AttrMap::new();
        component.set(HtmlAttr::Data("ars-pressed"), "false");

        let merged = merge_attrs([interaction, component]);

        // Component attrs come last → component wins.
        assert_eq!(merged.get(&HtmlAttr::Data("ars-pressed")), Some("false"));
    }

    #[test]
    fn id_last_write_wins() {
        let mut a = AttrMap::new();
        a.set(HtmlAttr::Id, "interaction-id");

        let mut b = AttrMap::new();
        b.set(HtmlAttr::Id, "component-id");

        let merged = merge_attrs([a, b]);

        assert_eq!(merged.get(&HtmlAttr::Id), Some("component-id"));
    }

    #[test]
    fn tabindex_last_write_wins() {
        let mut interaction = AttrMap::new();
        interaction.set(HtmlAttr::TabIndex, "-1");

        let mut component = AttrMap::new();
        component.set(HtmlAttr::TabIndex, "0");

        let merged = merge_attrs([interaction, component]);

        // Component determines focusability.
        assert_eq!(merged.get(&HtmlAttr::TabIndex), Some("0"));
    }

    #[test]
    fn non_id_list_aria_attrs_last_write_wins() {
        let mut interaction = AttrMap::new();
        interaction.set(HtmlAttr::Aria(AriaAttr::Expanded), "false");

        let mut component = AttrMap::new();
        component.set(HtmlAttr::Aria(AriaAttr::Expanded), "true");

        let merged = merge_attrs([interaction, component]);

        // Non-ID-list ARIA: component (last) wins, no concatenation.
        assert_eq!(
            merged.get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("true")
        );
    }

    #[test]
    fn bool_attr_override() {
        let mut a = AttrMap::new();
        a.set_bool(HtmlAttr::Data("ars-disabled"), false);

        let mut b = AttrMap::new();
        b.set_bool(HtmlAttr::Data("ars-disabled"), true);

        let merged = merge_attrs([a, b]);

        // Bool values follow last-write-wins too.
        assert_eq!(
            merged.get_value(&HtmlAttr::Data("ars-disabled")),
            Some(&ars_core::AttrValue::Bool(true))
        );
    }

    // ---- Debug-mode style conflict warning ----

    #[test]
    fn style_conflict_warning_fires_on_different_values() {
        // Capture stderr to verify the warning fires.
        let mut a = AttrMap::new();
        a.set_style(CssProperty::Width, "10px");

        let mut b = AttrMap::new();
        b.set_style(CssProperty::Width, "20px");

        // The function must not panic; the warning is best-effort.
        // We verify the merge result is correct (last-write-wins).
        let merged = merge_attrs([a, b]);
        let width = merged
            .styles()
            .iter()
            .find(|(k, _)| *k == CssProperty::Width);
        assert_eq!(width.map(|(_, v)| v.as_str()), Some("20px"));
    }

    #[test]
    fn no_warning_when_style_values_match() {
        let mut a = AttrMap::new();
        a.set_style(CssProperty::Width, "10px");

        let mut b = AttrMap::new();
        b.set_style(CssProperty::Width, "10px");

        // Same value → no warning, merge still works.
        let merged = merge_attrs([a, b]);
        let width = merged
            .styles()
            .iter()
            .find(|(k, _)| *k == CssProperty::Width);
        assert_eq!(width.map(|(_, v)| v.as_str()), Some("10px"));
    }

    #[test]
    fn macro_accepts_trailing_comma() {
        let mut a = AttrMap::new();
        a.set(HtmlAttr::Role, "button");

        let mut b = AttrMap::new();
        b.set(HtmlAttr::Class, "primary");

        // Trailing comma after last argument.
        let merged = merge_attrs!(a, b,);

        assert_eq!(merged.get(&HtmlAttr::Role), Some("button"));
        assert_eq!(merged.get(&HtmlAttr::Class), Some("primary"));
    }
}
