//! Attr helpers and CSS documentation for visually hidden content.

use ars_core::{AttrMap, HtmlAttr};

/// Returns the `AttrMap` for a visually-hidden wrapper element.
///
/// The CSS technique used (absolute positioning + clip) avoids the following
/// pitfalls of other approaches:
///   - `display: none` / `visibility: hidden`: hidden from screen readers too.
///   - `opacity: 0`: still takes up space, may be clipped by ancestors.
///   - `font-size: 0`: `VoiceOver` on macOS may still read the element.
///   - `text-indent: -9999px`: causes performance issues with long text.
///
/// This implementation is safe for RTL layouts and does not cause scroll issues.
///
/// **Important**: Because this technique uses `position: absolute`, the
/// `VisuallyHidden` element must be placed inside a positioned ancestor
/// (i.e., an element with `position: relative`, `absolute`, `fixed`, or
/// `sticky`). Without a positioned ancestor, the absolutely-positioned
/// element will be placed relative to the initial containing block (the
/// viewport), which can cause unexpected layout shifts and scroll issues.
/// Framework adapters should document this requirement. In practice, most
/// component root elements already have `position: relative` set.
pub fn visually_hidden_attrs() -> AttrMap {
    let mut attrs = AttrMap::new();
    attrs.set(HtmlAttr::Class, "ars-visually-hidden");
    attrs
}

/// CSS for visually-hidden (non-focusable):
///
/// ```css
/// .ars-visually-hidden {
///   position: absolute;
///   width: 1px;
///   height: 1px;
///   padding: 0;
///   margin: -1px;
///   overflow: hidden;
///   clip: rect(0, 0, 0, 0);
///   white-space: nowrap;
///   border-width: 0;
/// }
/// ```
#[derive(Debug)]
pub struct VisuallyHiddenCssDoc;

/// Returns visually hidden attrs for an element that MUST remain visible
/// when it receives focus (e.g., a "Skip to content" link).
/// When focused, the element becomes visible.
///
/// This variant deliberately does not apply the unconditional
/// `ars-visually-hidden` class because that class would keep the element
/// clipped even while focused. The focusable behavior is driven entirely by the
/// `data-ars-visually-hidden-focusable` CSS hook.
pub fn visually_hidden_focusable_attrs() -> AttrMap {
    let mut attrs = AttrMap::new();
    attrs.set_bool(HtmlAttr::Data("ars-visually-hidden-focusable"), true);
    attrs
}

/// CSS for visually-hidden-focusable:
///
/// ```css
/// [data-ars-visually-hidden-focusable]:not(:focus):not(:focus-within) {
///   position: absolute;
///   width: 1px;
///   height: 1px;
///   padding: 0;
///   margin: -1px;
///   overflow: hidden;
///   clip: rect(0, 0, 0, 0);
///   white-space: nowrap;
///   border-width: 0;
/// }
/// ```
#[derive(Debug)]
pub struct VisuallyHiddenFocusableCssDoc;

#[cfg(test)]
mod tests {
    use ars_core::AttrValue;

    use super::*;

    #[test]
    fn visually_hidden_attrs_returns_hidden_class() {
        let attrs = visually_hidden_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Class), Some("ars-visually-hidden"));
    }

    #[test]
    fn visually_hidden_focusable_attrs_adds_focusable_flag() {
        let attrs = visually_hidden_focusable_attrs();

        assert!(!attrs.contains(&HtmlAttr::Class));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-visually-hidden-focusable")),
            Some(&AttrValue::Bool(true))
        );
    }

    #[test]
    fn visually_hidden_focusable_attrs_does_not_apply_hidden_class() {
        let focusable = visually_hidden_focusable_attrs();

        assert_eq!(focusable.get(&HtmlAttr::Class), None);
    }
}
