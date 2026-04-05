//! Common ARIA state transition helpers for `connect()` implementations.
//!
//! These functions normalize the most frequent state transitions used inside
//! component connect functions, handling nullable attribute removal and
//! multi-attribute coordination (e.g., `set_invalid` managing both
//! `aria-invalid` and `aria-errormessage`).

use alloc::string::String;

use ars_core::{AriaAttr, AttrMap, AttrValue, HtmlAttr};

use super::attribute::{AriaAttribute, AriaChecked, AriaIdRef, AriaInvalid};

/// Sets `aria-expanded` based on boolean state.
///
/// `None` removes the attribute entirely, for elements that do not inherently
/// have an expanded concept.
#[inline]
pub fn set_expanded(attrs: &mut AttrMap, expanded: Option<bool>) {
    AriaAttribute::Expanded(expanded).apply_to(attrs);
}

/// Sets `aria-selected`.
///
/// `None` removes the attribute entirely, for elements where selection is not
/// applicable in the current context.
#[inline]
pub fn set_selected(attrs: &mut AttrMap, selected: Option<bool>) {
    AriaAttribute::Selected(selected).apply_to(attrs);
}

/// Sets `aria-checked` for checkbox, radio, and switch semantics.
#[inline]
pub fn set_checked(attrs: &mut AttrMap, checked: AriaChecked) {
    AriaAttribute::Checked(checked).apply_to(attrs);
}

/// Sets `aria-disabled`.
///
/// Uses `aria-disabled` rather than the HTML `disabled` attribute for non-form
/// elements. For `<button>` and `<input>`, use the native `disabled` attribute
/// in addition to `aria-disabled`.
///
/// **Note:** `data-ars-disabled` is NOT set by this helper — that attribute is
/// the responsibility of interaction primitives (e.g., `PressResult::current_attrs()`)
/// and component connect functions.
#[inline]
pub fn set_disabled(attrs: &mut AttrMap, disabled: bool) {
    if disabled {
        AriaAttribute::Disabled(true).apply_to(attrs);
    } else {
        attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), AttrValue::None);
    }
}

/// Sets `aria-busy` for loading states.
#[inline]
pub fn set_busy(attrs: &mut AttrMap, busy: bool) {
    AriaAttribute::Busy(busy).apply_to(attrs);
}

/// Sets `aria-invalid` with an optional error message reference.
///
/// **Note on `aria-errormessage` vs `aria-describedby`**: While this function
/// sets `aria-errormessage` per the ARIA 1.2 spec, screen reader support for
/// `aria-errormessage` remains poor as of 2025 (NVDA and JAWS have limited
/// support; `VoiceOver` does not announce it reliably). For maximum
/// compatibility, callers should **also** include the error message element's
/// ID in `aria-describedby`. The `FieldContext` (see spec §5.4) handles
/// this automatically by appending the error ID to `describedby_ids` when the
/// field is invalid. Using both attributes is not harmful — `aria-describedby`
/// provides the fallback announcement while `aria-errormessage` provides
/// the semantic relationship for assistive technologies that support it.
#[inline]
pub fn set_invalid(attrs: &mut AttrMap, invalid: AriaInvalid, error_id: Option<&str>) {
    AriaAttribute::Invalid(invalid).apply_to(attrs);
    if let Some(id) = error_id {
        AriaAttribute::ErrorMessage(AriaIdRef(String::from(id))).apply_to(attrs);
    } else {
        attrs.set(HtmlAttr::Aria(AriaAttr::ErrorMessage), AttrValue::None);
    }
}

#[cfg(test)]
mod tests {
    use ars_core::{AriaAttr, AttrMap, HtmlAttr};

    use super::*;

    #[test]
    fn set_expanded_true() {
        let mut attrs = AttrMap::new();
        set_expanded(&mut attrs, Some(true));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Expanded)), Some("true"));
    }

    #[test]
    fn set_expanded_false() {
        let mut attrs = AttrMap::new();
        set_expanded(&mut attrs, Some(false));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("false")
        );
    }

    #[test]
    fn set_expanded_none_removes() {
        let mut attrs = AttrMap::new();
        set_expanded(&mut attrs, Some(true));
        assert!(attrs.contains(&HtmlAttr::Aria(AriaAttr::Expanded)));
        set_expanded(&mut attrs, None);
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Expanded)));
    }

    #[test]
    fn set_selected_true() {
        let mut attrs = AttrMap::new();
        set_selected(&mut attrs, Some(true));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Selected)), Some("true"));
    }

    #[test]
    fn set_selected_none_removes() {
        let mut attrs = AttrMap::new();
        set_selected(&mut attrs, Some(true));
        assert!(attrs.contains(&HtmlAttr::Aria(AriaAttr::Selected)));
        set_selected(&mut attrs, None);
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Selected)));
    }

    #[test]
    fn set_checked_true() {
        let mut attrs = AttrMap::new();
        set_checked(&mut attrs, AriaChecked::True);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Checked)), Some("true"));
    }

    #[test]
    fn set_checked_mixed() {
        let mut attrs = AttrMap::new();
        set_checked(&mut attrs, AriaChecked::Mixed);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Checked)), Some("mixed"));
    }

    #[test]
    fn set_disabled_true() {
        let mut attrs = AttrMap::new();
        set_disabled(&mut attrs, true);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
    }

    #[test]
    fn set_disabled_false_removes() {
        let mut attrs = AttrMap::new();
        set_disabled(&mut attrs, true);
        assert!(attrs.contains(&HtmlAttr::Aria(AriaAttr::Disabled)));
        set_disabled(&mut attrs, false);
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Disabled)));
    }

    #[test]
    fn set_busy_true() {
        let mut attrs = AttrMap::new();
        set_busy(&mut attrs, true);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Busy)), Some("true"));
    }

    #[test]
    fn set_busy_false() {
        let mut attrs = AttrMap::new();
        set_busy(&mut attrs, false);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Busy)), Some("false"));
    }

    #[test]
    fn set_invalid_with_error_id() {
        let mut attrs = AttrMap::new();
        set_invalid(&mut attrs, AriaInvalid::True, Some("err-1"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ErrorMessage)),
            Some("err-1")
        );
    }

    #[test]
    fn set_invalid_without_error_clears_errormessage() {
        let mut attrs = AttrMap::new();
        // First set with an error ID.
        set_invalid(&mut attrs, AriaInvalid::True, Some("err-1"));
        assert!(attrs.contains(&HtmlAttr::Aria(AriaAttr::ErrorMessage)));

        // Now clear the error ID.
        set_invalid(&mut attrs, AriaInvalid::True, None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::ErrorMessage)));
    }

    #[test]
    fn set_invalid_grammar() {
        let mut attrs = AttrMap::new();
        set_invalid(&mut attrs, AriaInvalid::Grammar, Some("g-1"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)),
            Some("grammar")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ErrorMessage)),
            Some("g-1")
        );
    }
}
