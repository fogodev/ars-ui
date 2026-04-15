//! Assertion helpers for validating ARIA contracts in component tests.
//!
//! These helpers intentionally panic with descriptive messages so spec examples
//! and component unit tests can make small, focused assertions against an
//! [`ars_core::AttrMap`] without repeating raw attribute lookups.

use alloc::format;
use core::{
    fmt::{Debug, Display},
    str::FromStr,
};

use ars_core::{AriaAttr, AttrMap, HtmlAttr};

fn required_attr<'a>(attrs: &'a AttrMap, attr: HtmlAttr, name: &str, expected: &str) -> &'a str {
    attrs
        .get(&attr)
        .unwrap_or_else(|| panic!("expected {name}=\"{expected}\" but not found"))
}

fn assert_string_attr(attrs: &AttrMap, attr: HtmlAttr, name: &str, expected: &str) {
    let actual = required_attr(attrs, attr, name, expected);

    assert_eq!(
        actual, expected,
        "expected {name}=\"{expected}\", got \"{actual}\""
    );
}

fn assert_exact_bool_attr(attrs: &AttrMap, attr: HtmlAttr, name: &str, expected: bool) {
    let expected = if expected { "true" } else { "false" };

    let actual = required_attr(attrs, attr, name, expected);

    assert_eq!(
        actual, expected,
        "expected {name}=\"{expected}\", got \"{actual}\""
    );
}

fn assert_optional_false_bool_attr(attrs: &AttrMap, attr: HtmlAttr, name: &str, expected: bool) {
    let actual = attrs.get(&attr);

    if expected {
        assert_eq!(
            actual,
            Some("true"),
            "expected {name}=\"true\", got {actual:?}"
        );
    } else {
        match actual {
            None | Some("false") => {}
            Some(other) => panic!("expected {name} to be absent or \"false\", got \"{other}\""),
        }
    }
}

fn assert_optional_false_token_attr(attrs: &AttrMap, attr: HtmlAttr, name: &str, expected: &str) {
    let actual = attrs.get(&attr);

    if expected == "false" {
        match actual {
            None | Some("false") => {}
            Some(other) => {
                panic!("expected {name} to be absent or \"false\", got \"{other}\"")
            }
        }
    } else {
        assert_eq!(
            actual,
            Some(expected),
            "expected {name}=\"{expected}\", got {actual:?}"
        );
    }
}

fn assert_integer_attr<T>(attrs: &AttrMap, attr: HtmlAttr, name: &str, expected: &T)
where
    T: Copy + FromStr + Debug + Display + PartialEq,
{
    let actual = attrs
        .get(&attr)
        .unwrap_or_else(|| panic!("expected {name}=\"{expected}\" but not found"));

    let parsed = actual
        .parse::<T>()
        .unwrap_or_else(|_| panic!("{name} must be a valid integer, got \"{actual}\""));

    assert_eq!(
        parsed, *expected,
        "expected {name}={expected}, got {parsed}"
    );
}

/// Assert the `AttrMap` contains the expected `role` attribute.
pub fn assert_role(attrs: &AttrMap, expected: &str) {
    assert_string_attr(attrs, HtmlAttr::Role, "role", expected);
}

/// Assert `aria-label` matches the expected value.
pub fn assert_aria_label(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Label),
        "aria-label",
        expected,
    );
}

/// Assert `aria-expanded` is present and matches the expected boolean value.
pub fn assert_aria_expanded(attrs: &AttrMap, expected: bool) {
    assert_exact_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Expanded),
        "aria-expanded",
        expected,
    );
}

/// Assert `aria-selected` is present and matches the expected boolean value.
pub fn assert_aria_selected(attrs: &AttrMap, expected: bool) {
    assert_exact_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Selected),
        "aria-selected",
        expected,
    );
}

/// Assert `aria-disabled` matches the expected boolean contract.
pub fn assert_aria_disabled(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Disabled),
        "aria-disabled",
        expected,
    );
}

/// Assert `aria-busy` matches the expected boolean contract.
pub fn assert_aria_busy(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr(attrs, HtmlAttr::Aria(AriaAttr::Busy), "aria-busy", expected);
}

/// Assert `aria-checked` matches the expected string value.
pub fn assert_aria_checked(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Checked),
        "aria-checked",
        expected,
    );
}

/// Assert `aria-controls` matches the expected ID reference list.
pub fn assert_aria_controls(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Controls),
        "aria-controls",
        expected,
    );
}

/// Assert `aria-labelledby` matches the expected ID reference list.
pub fn assert_aria_labelledby(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::LabelledBy),
        "aria-labelledby",
        expected,
    );
}

/// Assert `aria-describedby` matches the expected ID reference list.
pub fn assert_aria_describedby(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::DescribedBy),
        "aria-describedby",
        expected,
    );
}

/// Assert `aria-haspopup` matches the expected value.
pub fn assert_aria_haspopup(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::HasPopup),
        "aria-haspopup",
        expected,
    );
}

/// Assert `aria-pressed` matches the expected value.
pub fn assert_aria_pressed(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Pressed),
        "aria-pressed",
        expected,
    );
}

/// Assert `aria-activedescendant` matches the expected ID reference.
pub fn assert_aria_activedescendant(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::ActiveDescendant),
        "aria-activedescendant",
        expected,
    );
}

/// Assert `data-ars-state` matches the expected state name.
pub fn assert_data_state(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Data("ars-state"),
        "data-ars-state",
        expected,
    );
}

/// Assert `tabindex` matches the expected integer value.
pub fn assert_tabindex(attrs: &AttrMap, expected: i32) {
    let expected = format!("{expected}");

    assert_string_attr(attrs, HtmlAttr::TabIndex, "tabindex", expected.as_str());
}

/// Assert `aria-orientation` matches the expected value.
pub fn assert_aria_orientation(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Orientation),
        "aria-orientation",
        expected,
    );
}

/// Assert `aria-valuemin` matches the expected value.
pub fn assert_aria_valuemin(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::ValueMin),
        "aria-valuemin",
        expected,
    );
}

/// Assert `aria-valuemax` matches the expected value.
pub fn assert_aria_valuemax(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::ValueMax),
        "aria-valuemax",
        expected,
    );
}

/// Assert `aria-valuenow` matches the expected value.
pub fn assert_aria_valuenow(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::ValueNow),
        "aria-valuenow",
        expected,
    );
}

/// Assert `aria-valuetext` matches the expected value.
pub fn assert_aria_valuetext(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::ValueText),
        "aria-valuetext",
        expected,
    );
}

/// Assert `aria-multiselectable` matches the expected boolean contract.
pub fn assert_aria_multiselectable(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::MultiSelectable),
        "aria-multiselectable",
        expected,
    );
}

/// Assert `aria-required` matches the expected boolean contract.
pub fn assert_aria_required(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Required),
        "aria-required",
        expected,
    );
}

/// Assert `aria-invalid` matches the expected token.
///
/// Passing `"false"` accepts either an absent attribute or an explicit
/// `aria-invalid="false"`, matching the optional-false contract used by the
/// state helpers.
pub fn assert_aria_invalid(attrs: &AttrMap, expected: &str) {
    assert_optional_false_token_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Invalid),
        "aria-invalid",
        expected,
    );
}

/// Assert `aria-live` matches the expected value.
pub fn assert_aria_live(attrs: &AttrMap, expected: &str) {
    assert_string_attr(attrs, HtmlAttr::Aria(AriaAttr::Live), "aria-live", expected);
}

/// Assert `aria-atomic` matches the expected boolean contract.
pub fn assert_aria_atomic(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Atomic),
        "aria-atomic",
        expected,
    );
}

/// Assert `aria-owns` matches the expected ID reference list.
pub fn assert_aria_owns(attrs: &AttrMap, expected: &str) {
    assert_string_attr(attrs, HtmlAttr::Aria(AriaAttr::Owns), "aria-owns", expected);
}

/// Assert `aria-hidden` matches the expected boolean contract.
pub fn assert_aria_hidden(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Hidden),
        "aria-hidden",
        expected,
    );
}

/// Assert `aria-modal` matches the expected boolean contract.
pub fn assert_aria_modal(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Modal),
        "aria-modal",
        expected,
    );
}

/// Assert `aria-autocomplete` matches the expected value.
pub fn assert_aria_autocomplete(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::AutoComplete),
        "aria-autocomplete",
        expected,
    );
}

/// Assert `aria-errormessage` matches the expected ID reference.
pub fn assert_aria_errormessage(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::ErrorMessage),
        "aria-errormessage",
        expected,
    );
}

/// Assert `aria-setsize` matches the expected integer value.
pub fn assert_aria_setsize(attrs: &AttrMap, expected: i32) {
    assert_integer_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::SetSize),
        "aria-setsize",
        &expected,
    );
}

/// Assert `aria-posinset` matches the expected integer value.
pub fn assert_aria_posinset(attrs: &AttrMap, expected: u32) {
    assert_integer_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::PosInSet),
        "aria-posinset",
        &expected,
    );
}

/// Assert `aria-level` matches the expected integer value.
pub fn assert_aria_level(attrs: &AttrMap, expected: u32) {
    assert_integer_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Level),
        "aria-level",
        &expected,
    );
}

/// Assert `aria-roledescription` matches the expected value.
pub fn assert_aria_roledescription(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::RoleDescription),
        "aria-roledescription",
        expected,
    );
}

/// Assert `aria-current` matches the expected value.
pub fn assert_aria_current(attrs: &AttrMap, expected: &str) {
    assert_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Current),
        "aria-current",
        expected,
    );
}

/// Assert `aria-rowindex` matches the expected integer value.
pub fn assert_aria_rowindex(attrs: &AttrMap, expected: u32) {
    assert_integer_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::RowIndex),
        "aria-rowindex",
        &expected,
    );
}

/// Assert `aria-colindex` matches the expected integer value.
pub fn assert_aria_colindex(attrs: &AttrMap, expected: u32) {
    assert_integer_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::ColIndex),
        "aria-colindex",
        &expected,
    );
}

/// Assert `aria-rowcount` matches the expected integer value.
pub fn assert_aria_rowcount(attrs: &AttrMap, expected: i32) {
    assert_integer_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::RowCount),
        "aria-rowcount",
        &expected,
    );
}

/// Assert `aria-colcount` matches the expected integer value.
pub fn assert_aria_colcount(attrs: &AttrMap, expected: i32) {
    assert_integer_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::ColCount),
        "aria-colcount",
        &expected,
    );
}

/// Assert `aria-sort` matches the expected value.
pub fn assert_aria_sort(attrs: &AttrMap, expected: &str) {
    assert_string_attr(attrs, HtmlAttr::Aria(AriaAttr::Sort), "aria-sort", expected);
}

/// Assert `aria-readonly` matches the expected boolean contract.
pub fn assert_aria_readonly(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::ReadOnly),
        "aria-readonly",
        expected,
    );
}

#[cfg(test)]
mod tests {
    extern crate std;

    use alloc::{boxed::Box, string::String};
    use core::panic::AssertUnwindSafe;
    use std::{
        panic::{catch_unwind, set_hook, take_hook},
        sync::{Mutex, OnceLock},
    };

    use super::*;

    fn panic_message(payload: Box<dyn core::any::Any + Send>) -> String {
        match payload.downcast::<String>() {
            Ok(message) => *message,
            Err(payload) => match payload.downcast::<&'static str>() {
                Ok(message) => String::from(*message),
                Err(_) => String::from("non-string panic payload"),
            },
        }
    }

    fn catch_panic_silently(
        f: impl FnOnce() + core::panic::UnwindSafe,
    ) -> Result<(), Box<dyn core::any::Any + Send>> {
        static PANIC_HOOK_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        let lock = PANIC_HOOK_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.lock().expect("panic hook lock poisoned");
        let previous_hook = take_hook();

        set_hook(Box::new(|_| {}));
        let result = catch_unwind(f);
        set_hook(previous_hook);

        result
    }

    macro_rules! string_assert_tests {
        ($success:ident, $missing:ident, $wrong:ident, $func:ident, $attr:expr, $expected:expr, $wrong_value:expr) => {
            #[test]
            fn $success() {
                let mut attrs = AttrMap::new();
                attrs.set($attr, $expected);

                $func(&attrs, $expected);
            }

            #[test]
            fn $missing() {
                let attrs = AttrMap::new();

                let panic = catch_panic_silently(AssertUnwindSafe(|| $func(&attrs, $expected)))
                    .expect_err("missing attribute should panic");

                assert!(panic_message(panic).contains("expected"));
            }

            #[test]
            fn $wrong() {
                let mut attrs = AttrMap::new();
                attrs.set($attr, $wrong_value);

                let panic = catch_panic_silently(AssertUnwindSafe(|| $func(&attrs, $expected)))
                    .expect_err("wrong attribute value should panic");

                assert!(panic_message(panic).contains("expected"));
            }
        };
    }

    macro_rules! exact_bool_assert_tests {
        ($true_success:ident, $false_success:ident, $missing:ident, $func:ident, $attr:expr) => {
            #[test]
            fn $true_success() {
                let mut attrs = AttrMap::new();
                attrs.set($attr, true);

                $func(&attrs, true);
            }

            #[test]
            fn $false_success() {
                let mut attrs = AttrMap::new();
                attrs.set($attr, false);

                $func(&attrs, false);
            }

            #[test]
            fn $missing() {
                let attrs = AttrMap::new();

                let panic = catch_panic_silently(AssertUnwindSafe(|| $func(&attrs, false)))
                    .expect_err("missing attribute should panic");

                assert!(panic_message(panic).contains("expected"));
            }
        };
    }

    macro_rules! optional_false_bool_assert_tests {
        ($true_success:ident, $false_absent:ident, $false_explicit:ident, $invalid_false:ident, $func:ident, $attr:expr, $invalid:expr) => {
            #[test]
            fn $true_success() {
                let mut attrs = AttrMap::new();
                attrs.set($attr, true);

                $func(&attrs, true);
            }

            #[test]
            fn $false_absent() {
                let attrs = AttrMap::new();

                $func(&attrs, false);
            }

            #[test]
            fn $false_explicit() {
                let mut attrs = AttrMap::new();
                attrs.set($attr, false);

                $func(&attrs, false);
            }

            #[test]
            fn $invalid_false() {
                let mut attrs = AttrMap::new();
                attrs.set($attr, $invalid);

                let panic = catch_panic_silently(AssertUnwindSafe(|| $func(&attrs, false)))
                    .expect_err("invalid false representation should panic");

                assert!(panic_message(panic).contains("expected"));
            }
        };
    }

    macro_rules! integer_assert_tests {
        ($success:ident, $wrong:ident, $parse_fail:ident, $missing:ident, $func:ident, $attr:expr, $expected:expr, $wrong_value:expr) => {
            #[test]
            fn $success() {
                let mut attrs = AttrMap::new();
                attrs.set($attr, format!("{}", $expected));

                $func(&attrs, $expected);
            }

            #[test]
            fn $wrong() {
                let mut attrs = AttrMap::new();
                attrs.set($attr, format!("{}", $wrong_value));

                let panic = catch_panic_silently(AssertUnwindSafe(|| $func(&attrs, $expected)))
                    .expect_err("wrong integer value should panic");

                assert!(panic_message(panic).contains("expected"));
            }

            #[test]
            fn $parse_fail() {
                let mut attrs = AttrMap::new();
                attrs.set($attr, "not-a-number");

                let panic = catch_panic_silently(AssertUnwindSafe(|| $func(&attrs, $expected)))
                    .expect_err("invalid integer value should panic");

                assert!(panic_message(panic).contains("valid integer"));
            }

            #[test]
            fn $missing() {
                let attrs = AttrMap::new();

                let panic = catch_panic_silently(AssertUnwindSafe(|| $func(&attrs, $expected)))
                    .expect_err("missing integer attribute should panic");

                assert!(panic_message(panic).contains("expected"));
            }
        };
    }

    #[test]
    fn assert_tabindex_succeeds() {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::TabIndex, "0");

        assert_tabindex(&attrs, 0);
    }

    #[test]
    fn assert_tabindex_panics_when_missing() {
        let attrs = AttrMap::new();

        let panic = catch_panic_silently(AssertUnwindSafe(|| assert_tabindex(&attrs, 0)))
            .expect_err("missing tabindex should panic");

        assert!(panic_message(panic).contains("expected"));
    }

    #[test]
    fn assert_tabindex_panics_when_wrong() {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::TabIndex, "-1");

        let panic = catch_panic_silently(AssertUnwindSafe(|| assert_tabindex(&attrs, 0)))
            .expect_err("wrong tabindex should panic");

        assert!(panic_message(panic).contains("expected"));
    }

    string_assert_tests!(
        assert_role_succeeds,
        assert_role_panics_when_missing,
        assert_role_panics_when_wrong,
        assert_role,
        HtmlAttr::Role,
        "button",
        "dialog"
    );
    string_assert_tests!(
        assert_aria_label_succeeds,
        assert_aria_label_panics_when_missing,
        assert_aria_label_panics_when_wrong,
        assert_aria_label,
        HtmlAttr::Aria(AriaAttr::Label),
        "Save",
        "Cancel"
    );
    exact_bool_assert_tests!(
        assert_aria_expanded_accepts_true,
        assert_aria_expanded_accepts_false,
        assert_aria_expanded_panics_when_missing,
        assert_aria_expanded,
        HtmlAttr::Aria(AriaAttr::Expanded)
    );
    exact_bool_assert_tests!(
        assert_aria_selected_accepts_true,
        assert_aria_selected_accepts_false,
        assert_aria_selected_panics_when_missing,
        assert_aria_selected,
        HtmlAttr::Aria(AriaAttr::Selected)
    );
    optional_false_bool_assert_tests!(
        assert_aria_disabled_accepts_true,
        assert_aria_disabled_accepts_absent_false,
        assert_aria_disabled_accepts_explicit_false,
        assert_aria_disabled_panics_on_invalid_false,
        assert_aria_disabled,
        HtmlAttr::Aria(AriaAttr::Disabled),
        "mixed"
    );
    optional_false_bool_assert_tests!(
        assert_aria_busy_accepts_true,
        assert_aria_busy_accepts_absent_false,
        assert_aria_busy_accepts_explicit_false,
        assert_aria_busy_panics_on_invalid_false,
        assert_aria_busy,
        HtmlAttr::Aria(AriaAttr::Busy),
        "loading"
    );
    string_assert_tests!(
        assert_aria_checked_succeeds,
        assert_aria_checked_panics_when_missing,
        assert_aria_checked_panics_when_wrong,
        assert_aria_checked,
        HtmlAttr::Aria(AriaAttr::Checked),
        "mixed",
        "true"
    );
    string_assert_tests!(
        assert_aria_controls_succeeds,
        assert_aria_controls_panics_when_missing,
        assert_aria_controls_panics_when_wrong,
        assert_aria_controls,
        HtmlAttr::Aria(AriaAttr::Controls),
        "panel-1",
        "panel-2"
    );
    string_assert_tests!(
        assert_aria_labelledby_succeeds,
        assert_aria_labelledby_panics_when_missing,
        assert_aria_labelledby_panics_when_wrong,
        assert_aria_labelledby,
        HtmlAttr::Aria(AriaAttr::LabelledBy),
        "label-1",
        "label-2"
    );
    string_assert_tests!(
        assert_aria_describedby_succeeds,
        assert_aria_describedby_panics_when_missing,
        assert_aria_describedby_panics_when_wrong,
        assert_aria_describedby,
        HtmlAttr::Aria(AriaAttr::DescribedBy),
        "desc-1",
        "desc-2"
    );
    string_assert_tests!(
        assert_aria_haspopup_succeeds,
        assert_aria_haspopup_panics_when_missing,
        assert_aria_haspopup_panics_when_wrong,
        assert_aria_haspopup,
        HtmlAttr::Aria(AriaAttr::HasPopup),
        "dialog",
        "menu"
    );
    string_assert_tests!(
        assert_aria_pressed_succeeds,
        assert_aria_pressed_panics_when_missing,
        assert_aria_pressed_panics_when_wrong,
        assert_aria_pressed,
        HtmlAttr::Aria(AriaAttr::Pressed),
        "mixed",
        "true"
    );
    string_assert_tests!(
        assert_aria_activedescendant_succeeds,
        assert_aria_activedescendant_panics_when_missing,
        assert_aria_activedescendant_panics_when_wrong,
        assert_aria_activedescendant,
        HtmlAttr::Aria(AriaAttr::ActiveDescendant),
        "option-1",
        "option-2"
    );
    string_assert_tests!(
        assert_data_state_succeeds,
        assert_data_state_panics_when_missing,
        assert_data_state_panics_when_wrong,
        assert_data_state,
        HtmlAttr::Data("ars-state"),
        "open",
        "closed"
    );
    string_assert_tests!(
        assert_aria_orientation_succeeds,
        assert_aria_orientation_panics_when_missing,
        assert_aria_orientation_panics_when_wrong,
        assert_aria_orientation,
        HtmlAttr::Aria(AriaAttr::Orientation),
        "vertical",
        "horizontal"
    );
    string_assert_tests!(
        assert_aria_valuemin_succeeds,
        assert_aria_valuemin_panics_when_missing,
        assert_aria_valuemin_panics_when_wrong,
        assert_aria_valuemin,
        HtmlAttr::Aria(AriaAttr::ValueMin),
        "0",
        "1"
    );
    string_assert_tests!(
        assert_aria_valuemax_succeeds,
        assert_aria_valuemax_panics_when_missing,
        assert_aria_valuemax_panics_when_wrong,
        assert_aria_valuemax,
        HtmlAttr::Aria(AriaAttr::ValueMax),
        "100",
        "90"
    );
    string_assert_tests!(
        assert_aria_valuenow_succeeds,
        assert_aria_valuenow_panics_when_missing,
        assert_aria_valuenow_panics_when_wrong,
        assert_aria_valuenow,
        HtmlAttr::Aria(AriaAttr::ValueNow),
        "50",
        "40"
    );
    string_assert_tests!(
        assert_aria_valuetext_succeeds,
        assert_aria_valuetext_panics_when_missing,
        assert_aria_valuetext_panics_when_wrong,
        assert_aria_valuetext,
        HtmlAttr::Aria(AriaAttr::ValueText),
        "Half",
        "Quarter"
    );
    optional_false_bool_assert_tests!(
        assert_aria_multiselectable_accepts_true,
        assert_aria_multiselectable_accepts_absent_false,
        assert_aria_multiselectable_accepts_explicit_false,
        assert_aria_multiselectable_panics_on_invalid_false,
        assert_aria_multiselectable,
        HtmlAttr::Aria(AriaAttr::MultiSelectable),
        "sometimes"
    );
    optional_false_bool_assert_tests!(
        assert_aria_required_accepts_true,
        assert_aria_required_accepts_absent_false,
        assert_aria_required_accepts_explicit_false,
        assert_aria_required_panics_on_invalid_false,
        assert_aria_required,
        HtmlAttr::Aria(AriaAttr::Required),
        "required-ish"
    );
    #[test]
    fn assert_aria_invalid_accepts_true() {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");

        assert_aria_invalid(&attrs, "true");
    }

    #[test]
    fn assert_aria_invalid_accepts_absent_false() {
        let attrs = AttrMap::new();

        assert_aria_invalid(&attrs, "false");
    }

    #[test]
    fn assert_aria_invalid_accepts_explicit_false() {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "false");

        assert_aria_invalid(&attrs, "false");
    }

    #[test]
    fn assert_aria_invalid_accepts_grammar() {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "grammar");

        assert_aria_invalid(&attrs, "grammar");
    }

    #[test]
    fn assert_aria_invalid_accepts_spelling() {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "spelling");

        assert_aria_invalid(&attrs, "spelling");
    }

    #[test]
    fn assert_aria_invalid_panics_when_token_mismatches() {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "spelling");

        let panic =
            catch_panic_silently(AssertUnwindSafe(|| assert_aria_invalid(&attrs, "grammar")))
                .expect_err("mismatched aria-invalid token should panic");

        assert!(panic_message(panic).contains("expected"));
    }

    #[test]
    fn assert_aria_invalid_panics_when_false_is_other_token() {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "grammar");

        let panic = catch_panic_silently(AssertUnwindSafe(|| assert_aria_invalid(&attrs, "false")))
            .expect_err("non-false aria-invalid token should panic when false is expected");

        assert!(panic_message(panic).contains("absent or \"false\""));
    }
    string_assert_tests!(
        assert_aria_live_succeeds,
        assert_aria_live_panics_when_missing,
        assert_aria_live_panics_when_wrong,
        assert_aria_live,
        HtmlAttr::Aria(AriaAttr::Live),
        "polite",
        "assertive"
    );
    optional_false_bool_assert_tests!(
        assert_aria_atomic_accepts_true,
        assert_aria_atomic_accepts_absent_false,
        assert_aria_atomic_accepts_explicit_false,
        assert_aria_atomic_panics_on_invalid_false,
        assert_aria_atomic,
        HtmlAttr::Aria(AriaAttr::Atomic),
        "all"
    );
    string_assert_tests!(
        assert_aria_owns_succeeds,
        assert_aria_owns_panics_when_missing,
        assert_aria_owns_panics_when_wrong,
        assert_aria_owns,
        HtmlAttr::Aria(AriaAttr::Owns),
        "item-1",
        "item-2"
    );
    optional_false_bool_assert_tests!(
        assert_aria_hidden_accepts_true,
        assert_aria_hidden_accepts_absent_false,
        assert_aria_hidden_accepts_explicit_false,
        assert_aria_hidden_panics_on_invalid_false,
        assert_aria_hidden,
        HtmlAttr::Aria(AriaAttr::Hidden),
        "collapsed"
    );
    optional_false_bool_assert_tests!(
        assert_aria_modal_accepts_true,
        assert_aria_modal_accepts_absent_false,
        assert_aria_modal_accepts_explicit_false,
        assert_aria_modal_panics_on_invalid_false,
        assert_aria_modal,
        HtmlAttr::Aria(AriaAttr::Modal),
        "overlay"
    );
    string_assert_tests!(
        assert_aria_autocomplete_succeeds,
        assert_aria_autocomplete_panics_when_missing,
        assert_aria_autocomplete_panics_when_wrong,
        assert_aria_autocomplete,
        HtmlAttr::Aria(AriaAttr::AutoComplete),
        "list",
        "inline"
    );
    string_assert_tests!(
        assert_aria_errormessage_succeeds,
        assert_aria_errormessage_panics_when_missing,
        assert_aria_errormessage_panics_when_wrong,
        assert_aria_errormessage,
        HtmlAttr::Aria(AriaAttr::ErrorMessage),
        "error-1",
        "error-2"
    );
    integer_assert_tests!(
        assert_aria_setsize_succeeds,
        assert_aria_setsize_panics_when_wrong,
        assert_aria_setsize_panics_on_invalid_integer,
        assert_aria_setsize_panics_when_missing,
        assert_aria_setsize,
        HtmlAttr::Aria(AriaAttr::SetSize),
        -1,
        7
    );
    integer_assert_tests!(
        assert_aria_posinset_succeeds,
        assert_aria_posinset_panics_when_wrong,
        assert_aria_posinset_panics_on_invalid_integer,
        assert_aria_posinset_panics_when_missing,
        assert_aria_posinset,
        HtmlAttr::Aria(AriaAttr::PosInSet),
        3,
        4
    );
    integer_assert_tests!(
        assert_aria_level_succeeds,
        assert_aria_level_panics_when_wrong,
        assert_aria_level_panics_on_invalid_integer,
        assert_aria_level_panics_when_missing,
        assert_aria_level,
        HtmlAttr::Aria(AriaAttr::Level),
        2,
        3
    );
    string_assert_tests!(
        assert_aria_roledescription_succeeds,
        assert_aria_roledescription_panics_when_missing,
        assert_aria_roledescription_panics_when_wrong,
        assert_aria_roledescription,
        HtmlAttr::Aria(AriaAttr::RoleDescription),
        "Close dialog",
        "Open dialog"
    );
    string_assert_tests!(
        assert_aria_current_succeeds,
        assert_aria_current_panics_when_missing,
        assert_aria_current_panics_when_wrong,
        assert_aria_current,
        HtmlAttr::Aria(AriaAttr::Current),
        "page",
        "step"
    );
    integer_assert_tests!(
        assert_aria_rowindex_succeeds,
        assert_aria_rowindex_panics_when_wrong,
        assert_aria_rowindex_panics_on_invalid_integer,
        assert_aria_rowindex_panics_when_missing,
        assert_aria_rowindex,
        HtmlAttr::Aria(AriaAttr::RowIndex),
        4,
        5
    );
    integer_assert_tests!(
        assert_aria_colindex_succeeds,
        assert_aria_colindex_panics_when_wrong,
        assert_aria_colindex_panics_on_invalid_integer,
        assert_aria_colindex_panics_when_missing,
        assert_aria_colindex,
        HtmlAttr::Aria(AriaAttr::ColIndex),
        2,
        3
    );
    integer_assert_tests!(
        assert_aria_rowcount_succeeds,
        assert_aria_rowcount_panics_when_wrong,
        assert_aria_rowcount_panics_on_invalid_integer,
        assert_aria_rowcount_panics_when_missing,
        assert_aria_rowcount,
        HtmlAttr::Aria(AriaAttr::RowCount),
        12,
        11
    );
    integer_assert_tests!(
        assert_aria_colcount_succeeds,
        assert_aria_colcount_panics_when_wrong,
        assert_aria_colcount_panics_on_invalid_integer,
        assert_aria_colcount_panics_when_missing,
        assert_aria_colcount,
        HtmlAttr::Aria(AriaAttr::ColCount),
        6,
        5
    );
    string_assert_tests!(
        assert_aria_sort_succeeds,
        assert_aria_sort_panics_when_missing,
        assert_aria_sort_panics_when_wrong,
        assert_aria_sort,
        HtmlAttr::Aria(AriaAttr::Sort),
        "ascending",
        "descending"
    );
    optional_false_bool_assert_tests!(
        assert_aria_readonly_accepts_true,
        assert_aria_readonly_accepts_absent_false,
        assert_aria_readonly_accepts_explicit_false,
        assert_aria_readonly_panics_on_invalid_false,
        assert_aria_readonly,
        HtmlAttr::Aria(AriaAttr::ReadOnly),
        "readonly-ish"
    );
}
