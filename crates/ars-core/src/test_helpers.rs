//! ARIA assertion helpers for validating `connect()` output in tests.
//!
//! These helpers operate on [`AttrMap`] values so component and subsystem tests
//! can validate ARIA contracts without repeating raw attribute lookups.

use alloc::{
    format,
    string::{String, ToString},
};
use core::str::FromStr;

use crate::{AriaAttr, AttrMap, HtmlAttr};

fn required_attr(attrs: &AttrMap, attr: HtmlAttr, missing_message: impl Into<String>) -> &str {
    let missing_message = missing_message.into();

    attrs
        .get(&attr)
        .unwrap_or_else(|| panic!("{missing_message}"))
}

fn parsed_required_attr<T>(
    attrs: &AttrMap,
    attr: HtmlAttr,
    missing_message: &'static str,
    parse_message: &'static str,
) -> T
where
    T: FromStr,
{
    attrs
        .get(&attr)
        .expect(missing_message)
        .parse()
        .unwrap_or_else(|_| panic!("{parse_message}"))
}

fn assert_expected_string_attr(attrs: &AttrMap, attr: HtmlAttr, attr_name: &str, expected: &str) {
    let val = required_attr(
        attrs,
        attr,
        format!("expected {attr_name}=\"{expected}\" but not found"),
    );

    assert_eq!(val, expected, "wrong {attr_name}");
}

fn assert_required_bool_attr(attrs: &AttrMap, attr: HtmlAttr, attr_name: &str, expected: bool) {
    let val = required_attr(attrs, attr, format!("expected {attr_name} but not found"));

    assert_eq!(val, if expected { "true" } else { "false" });
}

fn assert_optional_false_bool_attr(
    attrs: &AttrMap,
    attr: HtmlAttr,
    attr_name: &str,
    expected: bool,
) {
    let val = attrs.get(&attr);

    if expected {
        assert_eq!(val, Some("true"), "expected {attr_name}=\"true\"");
    } else {
        assert!(val.is_none() || val == Some("false"));
    }
}

fn assert_optional_false_bool_attr_with_debug(
    attrs: &AttrMap,
    attr: HtmlAttr,
    attr_name: &str,
    expected: bool,
) {
    if expected {
        assert_eq!(
            attrs.get(&attr),
            Some("true"),
            "expected {attr_name}=\"true\""
        );
    } else {
        let val = attrs.get(&attr);

        assert!(
            val.is_none() || val == Some("false"),
            "expected {attr_name} to be absent or \"false\", got {val:?}"
        );
    }
}

fn assert_present_attr_eq(attrs: &AttrMap, attr: HtmlAttr, attr_name: &str, expected: &str) {
    assert_eq!(
        attrs.get(&attr),
        Some(expected),
        "expected {attr_name}=\"{expected}\"",
    );
}

/// Assert the `AttrMap` contains the expected `role` attribute.
pub fn assert_role(attrs: &AttrMap, expected: &str) {
    let role = attrs
        .get(&HtmlAttr::Role)
        .unwrap_or_else(|| panic!("expected role=\"{expected}\" but no role attribute found"));

    assert_eq!(role, expected, "wrong role");
}

/// Assert `aria-label` matches the expected value.
pub fn assert_aria_label(attrs: &AttrMap, expected: &str) {
    assert_expected_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Label),
        "aria-label",
        expected,
    );
}

/// Assert `aria-expanded` is present and matches the expected boolean string.
pub fn assert_aria_expanded(attrs: &AttrMap, expected: bool) {
    assert_required_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Expanded),
        "aria-expanded",
        expected,
    );
}

/// Assert `aria-selected` is present and matches the expected boolean string.
pub fn assert_aria_selected(attrs: &AttrMap, expected: bool) {
    assert_required_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Selected),
        "aria-selected",
        expected,
    );
}

/// Assert `aria-disabled` matches the expected boolean.
pub fn assert_aria_disabled(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr_with_debug(
        attrs,
        HtmlAttr::Aria(AriaAttr::Disabled),
        "aria-disabled",
        expected,
    );
}

/// Assert `aria-busy` matches the expected boolean.
pub fn assert_aria_busy(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr_with_debug(
        attrs,
        HtmlAttr::Aria(AriaAttr::Busy),
        "aria-busy",
        expected,
    );
}

/// Assert `aria-checked` matches the expected value.
pub fn assert_aria_checked(attrs: &AttrMap, expected: &str) {
    assert_expected_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Checked),
        "aria-checked",
        expected,
    );
}

/// Assert `aria-controls` matches the expected value.
pub fn assert_aria_controls(attrs: &AttrMap, expected: &str) {
    assert_expected_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Controls),
        "aria-controls",
        expected,
    );
}

/// Assert `aria-labelledby` matches the expected value.
pub fn assert_aria_labelledby(attrs: &AttrMap, expected: &str) {
    assert_expected_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::LabelledBy),
        "aria-labelledby",
        expected,
    );
}

/// Assert `aria-describedby` matches the expected value.
pub fn assert_aria_describedby(attrs: &AttrMap, expected: &str) {
    assert_expected_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::DescribedBy),
        "aria-describedby",
        expected,
    );
}

/// Assert `aria-haspopup` matches the expected value.
pub fn assert_aria_haspopup(attrs: &AttrMap, expected: &str) {
    assert_expected_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::HasPopup),
        "aria-haspopup",
        expected,
    );
}

/// Assert `aria-pressed` matches the expected value.
pub fn assert_aria_pressed(attrs: &AttrMap, expected: &str) {
    assert_expected_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Pressed),
        "aria-pressed",
        expected,
    );
}

/// Assert `aria-activedescendant` matches the expected value.
pub fn assert_aria_activedescendant(attrs: &AttrMap, expected: &str) {
    assert_expected_string_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::ActiveDescendant),
        "aria-activedescendant",
        expected,
    );
}

/// Assert `data-ars-state` matches the expected value.
pub fn assert_data_state(attrs: &AttrMap, expected: &str) {
    let val = attrs
        .get(&HtmlAttr::Data("ars-state"))
        .unwrap_or_else(|| panic!("expected data-ars-state but not found"));

    assert_eq!(val, expected);
}

/// Assert `tabindex` matches the expected value.
pub fn assert_tabindex(attrs: &AttrMap, expected: i32) {
    let expected_str = expected.to_string();

    let val = attrs
        .get(&HtmlAttr::TabIndex)
        .unwrap_or_else(|| panic!("expected tabindex but not found"));

    assert_eq!(
        val,
        expected_str.as_str(),
        "expected tabindex=\"{expected}\"",
    );
}

/// Assert `aria-orientation` matches the expected value.
pub fn assert_aria_orientation(attrs: &AttrMap, expected: &str) {
    assert_present_attr_eq(
        attrs,
        HtmlAttr::Aria(AriaAttr::Orientation),
        "aria-orientation",
        expected,
    );
}

/// Assert `aria-valuemin` matches the expected value.
pub fn assert_aria_valuemin(attrs: &AttrMap, expected: &str) {
    assert_present_attr_eq(
        attrs,
        HtmlAttr::Aria(AriaAttr::ValueMin),
        "aria-valuemin",
        expected,
    );
}

/// Assert `aria-valuemax` matches the expected value.
pub fn assert_aria_valuemax(attrs: &AttrMap, expected: &str) {
    assert_present_attr_eq(
        attrs,
        HtmlAttr::Aria(AriaAttr::ValueMax),
        "aria-valuemax",
        expected,
    );
}

/// Assert `aria-valuenow` matches the expected value.
pub fn assert_aria_valuenow(attrs: &AttrMap, expected: &str) {
    assert_present_attr_eq(
        attrs,
        HtmlAttr::Aria(AriaAttr::ValueNow),
        "aria-valuenow",
        expected,
    );
}

/// Assert `aria-valuetext` matches the expected value.
pub fn assert_aria_valuetext(attrs: &AttrMap, expected: &str) {
    assert_present_attr_eq(
        attrs,
        HtmlAttr::Aria(AriaAttr::ValueText),
        "aria-valuetext",
        expected,
    );
}

/// Assert `aria-multiselectable` matches the expected boolean.
pub fn assert_aria_multiselectable(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::MultiSelectable),
        "aria-multiselectable",
        expected,
    );
}

/// Assert `aria-required` matches the expected boolean.
pub fn assert_aria_required(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Required),
        "aria-required",
        expected,
    );
}

/// Assert `aria-invalid` matches the expected value.
pub fn assert_aria_invalid(attrs: &AttrMap, expected: &str) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid));

    if expected == "false" {
        assert!(val.is_none() || val == Some("false"));
    } else {
        assert_eq!(val, Some(expected), "expected aria-invalid=\"{expected}\"");
    }
}

/// Assert `aria-live` matches the expected value.
pub fn assert_aria_live(attrs: &AttrMap, expected: &str) {
    assert_present_attr_eq(attrs, HtmlAttr::Aria(AriaAttr::Live), "aria-live", expected);
}

/// Assert `aria-atomic` matches the expected boolean.
pub fn assert_aria_atomic(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Atomic),
        "aria-atomic",
        expected,
    );
}

/// Assert `aria-owns` matches the expected value.
pub fn assert_aria_owns(attrs: &AttrMap, expected: &str) {
    assert_present_attr_eq(attrs, HtmlAttr::Aria(AriaAttr::Owns), "aria-owns", expected);
}

/// Assert `aria-hidden` matches the expected boolean.
pub fn assert_aria_hidden(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Hidden),
        "aria-hidden",
        expected,
    );
}

/// Assert `aria-modal` matches the expected boolean.
pub fn assert_aria_modal(attrs: &AttrMap, expected: bool) {
    assert_optional_false_bool_attr_with_debug(
        attrs,
        HtmlAttr::Aria(AriaAttr::Modal),
        "aria-modal",
        expected,
    );
}

/// Assert `aria-autocomplete` matches the expected value.
pub fn assert_aria_autocomplete(attrs: &AttrMap, expected: &str) {
    let val = attrs
        .get(&HtmlAttr::Aria(AriaAttr::AutoComplete))
        .expect("aria-autocomplete must be present");

    assert_eq!(
        val, expected,
        "expected aria-autocomplete={expected}, got {val}"
    );
}

/// Assert `aria-errormessage` matches the expected value.
pub fn assert_aria_errormessage(attrs: &AttrMap, expected: &str) {
    let val = attrs
        .get(&HtmlAttr::Aria(AriaAttr::ErrorMessage))
        .expect("aria-errormessage must be present");

    assert_eq!(
        val, expected,
        "expected aria-errormessage={expected}, got {val}"
    );
}

/// Assert `aria-setsize` parses and matches the expected value.
pub fn assert_aria_setsize(attrs: &AttrMap, expected: i32) {
    let val: i32 = parsed_required_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::SetSize),
        "aria-setsize must be present",
        "aria-setsize must be a valid integer",
    );

    assert_eq!(val, expected, "expected aria-setsize={expected}, got {val}");
}

/// Assert `aria-posinset` parses and matches the expected value.
pub fn assert_aria_posinset(attrs: &AttrMap, expected: u32) {
    let val: u32 = parsed_required_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::PosInSet),
        "aria-posinset must be present",
        "aria-posinset must be a valid integer",
    );

    assert_eq!(
        val, expected,
        "expected aria-posinset={expected}, got {val}"
    );
}

/// Assert `aria-level` parses and matches the expected value.
pub fn assert_aria_level(attrs: &AttrMap, expected: u32) {
    let val: u32 = parsed_required_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::Level),
        "aria-level must be present",
        "aria-level must be a valid integer",
    );

    assert_eq!(val, expected, "expected aria-level={expected}, got {val}");
}

/// Assert `aria-roledescription` matches the expected value.
pub fn assert_aria_roledescription(attrs: &AttrMap, expected: &str) {
    let val = attrs
        .get(&HtmlAttr::Aria(AriaAttr::RoleDescription))
        .expect("aria-roledescription must be present");

    assert_eq!(
        val, expected,
        "expected aria-roledescription={expected}, got {val}"
    );
}

/// Assert `aria-current` matches the expected value.
pub fn assert_aria_current(attrs: &AttrMap, expected: &str) {
    let val = attrs
        .get(&HtmlAttr::Aria(AriaAttr::Current))
        .expect("aria-current must be present");

    assert_eq!(
        val, expected,
        "expected aria-current=\"{expected}\", got \"{val}\""
    );
}

/// Assert `aria-rowindex` matches the expected value.
pub fn assert_aria_rowindex(attrs: &AttrMap, expected: u32) {
    let val: u32 = parsed_required_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::RowIndex),
        "aria-rowindex must be present",
        "aria-rowindex must be a valid integer",
    );

    assert_eq!(
        val, expected,
        "expected aria-rowindex=\"{expected}\", got \"{val}\""
    );
}

/// Assert `aria-colindex` matches the expected value.
pub fn assert_aria_colindex(attrs: &AttrMap, expected: u32) {
    let val: u32 = parsed_required_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::ColIndex),
        "aria-colindex must be present",
        "aria-colindex must be a valid integer",
    );

    assert_eq!(
        val, expected,
        "expected aria-colindex=\"{expected}\", got \"{val}\""
    );
}

/// Assert `aria-rowcount` matches the expected value.
pub fn assert_aria_rowcount(attrs: &AttrMap, expected: i32) {
    let val: i32 = parsed_required_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::RowCount),
        "aria-rowcount must be present",
        "aria-rowcount must be a valid integer",
    );

    assert_eq!(
        val, expected,
        "expected aria-rowcount=\"{expected}\", got \"{val}\""
    );
}

/// Assert `aria-colcount` matches the expected value.
pub fn assert_aria_colcount(attrs: &AttrMap, expected: i32) {
    let val: i32 = parsed_required_attr(
        attrs,
        HtmlAttr::Aria(AriaAttr::ColCount),
        "aria-colcount must be present",
        "aria-colcount must be a valid integer",
    );

    assert_eq!(
        val, expected,
        "expected aria-colcount=\"{expected}\", got \"{val}\""
    );
}

/// Assert `aria-sort` matches the expected value.
pub fn assert_aria_sort(attrs: &AttrMap, expected: &str) {
    let val = attrs
        .get(&HtmlAttr::Aria(AriaAttr::Sort))
        .expect("aria-sort must be present");

    assert_eq!(
        val, expected,
        "expected aria-sort=\"{expected}\", got \"{val}\""
    );
}

/// Assert `aria-readonly` matches the expected boolean.
pub fn assert_aria_readonly(attrs: &AttrMap, expected: bool) {
    let val = attrs.get(&HtmlAttr::Aria(AriaAttr::ReadOnly));

    if expected {
        assert_eq!(
            val,
            Some("true"),
            "expected aria-readonly=\"true\", but got {val:?}"
        );
    } else {
        match val {
            None | Some("false") => {}

            Some(other) => {
                panic!("expected aria-readonly to be absent or \"false\", got \"{other}\"")
            }
        }
    }
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

    fn attrs_with(attr: HtmlAttr, value: &str) -> AttrMap {
        let mut attrs = AttrMap::new();

        let _ = attrs.set(attr, value);

        attrs
    }

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

        let result = catch_unwind(AssertUnwindSafe(f));

        set_hook(previous_hook);

        result
    }

    macro_rules! optional_false_helper_tests {
        (
            $true_name:ident,
            $absent_name:ident,
            $false_name:ident,
            $func:ident,
            $attr:expr
        ) => {
            #[test]
            fn $true_name() {
                let attrs = attrs_with($attr, "true");

                $func(&attrs, true);
            }

            #[test]
            fn $absent_name() {
                let attrs = AttrMap::new();

                $func(&attrs, false);
            }

            #[test]
            fn $false_name() {
                let attrs = attrs_with($attr, "false");

                $func(&attrs, false);
            }
        };
    }

    macro_rules! required_string_helper_tests {
        (
            $pass_name:ident,
            $missing_name:ident,
            $func:ident,
            $attr:expr,
            $expected:literal,
            $missing_message:literal
        ) => {
            #[test]
            fn $pass_name() {
                let attrs = attrs_with($attr, $expected);

                $func(&attrs, $expected);
            }

            #[test]
            fn $missing_name() {
                let attrs = AttrMap::new();

                let result = catch_panic_silently(|| $func(&attrs, $expected));

                let message =
                    panic_message(result.expect_err(concat!(stringify!($func), " should panic")));

                assert_eq!(message, $missing_message);
            }
        };
    }

    macro_rules! assert_eq_string_helper_tests {
        (
            $pass_name:ident,
            $missing_name:ident,
            $func:ident,
            $attr:expr,
            $expected:literal,
            $message_fragment:literal
        ) => {
            #[test]
            fn $pass_name() {
                let attrs = attrs_with($attr, $expected);

                $func(&attrs, $expected);
            }

            #[test]
            fn $missing_name() {
                let attrs = AttrMap::new();

                let result = catch_panic_silently(|| $func(&attrs, $expected));

                let message =
                    panic_message(result.expect_err(concat!(stringify!($func), " should panic")));

                assert!(
                    message.contains($message_fragment),
                    "expected panic message to contain {:?}, got {:?}",
                    $message_fragment,
                    message
                );
            }
        };
    }

    macro_rules! required_bool_helper_tests {
        (
            $true_name:ident,
            $false_name:ident,
            $missing_name:ident,
            $func:ident,
            $attr:expr,
            $missing_message:literal
        ) => {
            #[test]
            fn $true_name() {
                let attrs = attrs_with($attr, "true");

                $func(&attrs, true);
            }

            #[test]
            fn $false_name() {
                let attrs = attrs_with($attr, "false");

                $func(&attrs, false);
            }

            #[test]
            fn $missing_name() {
                let attrs = AttrMap::new();

                let result = catch_panic_silently(|| $func(&attrs, true));

                let message =
                    panic_message(result.expect_err(concat!(stringify!($func), " should panic")));

                assert_eq!(message, $missing_message);
            }
        };
    }

    macro_rules! integer_helper_tests {
        (
            $pass_name:ident,
            $missing_name:ident,
            $parse_name:ident,
            $func:ident,
            $attr:expr,
            $expected:expr,
            $missing_message:literal,
            $parse_message:literal
        ) => {
            #[test]
            fn $pass_name() {
                let attrs = attrs_with($attr, stringify!($expected));

                $func(&attrs, $expected);
            }

            #[test]
            fn $missing_name() {
                let attrs = AttrMap::new();

                let result = catch_panic_silently(|| $func(&attrs, $expected));

                let message =
                    panic_message(result.expect_err(concat!(stringify!($func), " should panic")));

                assert_eq!(message, $missing_message);
            }

            #[test]
            fn $parse_name() {
                let attrs = attrs_with($attr, "abc");

                let result = catch_panic_silently(|| $func(&attrs, $expected));

                let message =
                    panic_message(result.expect_err(concat!(stringify!($func), " should panic")));

                assert_eq!(message, $parse_message);
            }
        };
    }

    macro_rules! optional_false_helper_rejects_true_tests {
        ($panic_name:ident, $func:ident, $attr:expr) => {
            #[test]
            fn $panic_name() {
                let attrs = attrs_with($attr, "true");

                let result = catch_panic_silently(|| $func(&attrs, false));

                assert!(result.is_err(), "{} should panic", stringify!($func));
            }
        };
    }

    #[test]
    fn assert_role_passes_for_matching_role() {
        let attrs = attrs_with(HtmlAttr::Role, "button");

        assert_role(&attrs, "button");
    }

    #[test]
    #[should_panic(expected = "wrong role")]
    fn assert_role_panics_for_wrong_role() {
        let attrs = attrs_with(HtmlAttr::Role, "button");

        assert_role(&attrs, "checkbox");
    }

    #[test]
    fn assert_role_panics_with_exact_message_when_role_missing() {
        let attrs = AttrMap::new();

        let result = catch_panic_silently(|| assert_role(&attrs, "button"));

        let message = panic_message(result.expect_err("assert_role should panic"));

        assert_eq!(
            message,
            "expected role=\"button\" but no role attribute found"
        );
    }

    #[test]
    fn assert_aria_expanded_true_passes() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::Expanded), "true");

        assert_aria_expanded(&attrs, true);
    }

    #[test]
    fn assert_aria_expanded_false_passes() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::Expanded), "false");

        assert_aria_expanded(&attrs, false);
    }

    #[test]
    fn assert_aria_expanded_panics_when_missing() {
        let attrs = AttrMap::new();

        let result = catch_panic_silently(|| assert_aria_expanded(&attrs, true));

        let message = panic_message(result.expect_err("assert_aria_expanded should panic"));

        assert_eq!(message, "expected aria-expanded but not found");
    }

    #[test]
    fn assert_aria_disabled_false_passes_when_absent() {
        let attrs = AttrMap::new();

        assert_aria_disabled(&attrs, false);
    }

    #[test]
    fn assert_aria_disabled_false_passes_when_false() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::Disabled), "false");

        assert_aria_disabled(&attrs, false);
    }

    #[test]
    fn assert_aria_disabled_true_passes_when_true() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::Disabled), "true");

        assert_aria_disabled(&attrs, true);
    }

    #[test]
    fn optional_false_helper_panics_when_false_expected_but_true_found() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::Busy), "true");

        let result = catch_panic_silently(|| assert_aria_busy(&attrs, false));

        let message = panic_message(result.expect_err("assert_aria_busy should panic"));

        assert_eq!(
            message,
            "expected aria-busy to be absent or \"false\", got Some(\"true\")"
        );
    }

    #[test]
    fn assert_tabindex_zero_passes() {
        let attrs = attrs_with(HtmlAttr::TabIndex, "0");

        assert_tabindex(&attrs, 0);
    }

    #[test]
    fn assert_tabindex_negative_one_passes() {
        let attrs = attrs_with(HtmlAttr::TabIndex, "-1");

        assert_tabindex(&attrs, -1);
    }

    #[test]
    fn assert_data_state_passes() {
        let attrs = attrs_with(HtmlAttr::Data("ars-state"), "idle");

        assert_data_state(&attrs, "idle");
    }

    #[test]
    fn assert_aria_checked_mixed_passes() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::Checked), "mixed");

        assert_aria_checked(&attrs, "mixed");
    }

    #[test]
    fn assert_aria_setsize_parses_integer() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::SetSize), "5");

        assert_aria_setsize(&attrs, 5);
    }

    #[test]
    fn assert_aria_posinset_parses_integer() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::PosInSet), "3");

        assert_aria_posinset(&attrs, 3);
    }

    #[test]
    fn assert_aria_rowindex_parses_integer() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::RowIndex), "2");

        assert_aria_rowindex(&attrs, 2);
    }

    #[test]
    fn assert_aria_colindex_parses_integer() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::ColIndex), "4");

        assert_aria_colindex(&attrs, 4);
    }

    #[test]
    fn assert_aria_rowcount_parses_integer() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::RowCount), "7");

        assert_aria_rowcount(&attrs, 7);
    }

    #[test]
    fn assert_aria_colcount_parses_integer() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::ColCount), "8");

        assert_aria_colcount(&attrs, 8);
    }

    #[test]
    fn assert_integer_helper_panics_with_exact_parse_message() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::SetSize), "abc");

        let result = catch_panic_silently(|| assert_aria_setsize(&attrs, 5));

        let message = panic_message(result.expect_err("assert_aria_setsize should panic"));

        assert_eq!(message, "aria-setsize must be a valid integer");
    }

    #[test]
    fn assert_aria_invalid_false_passes_when_absent() {
        let attrs = AttrMap::new();

        assert_aria_invalid(&attrs, "false");
    }

    #[test]
    fn assert_aria_invalid_false_panics_when_other_value_is_present() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::Invalid), "grammar");

        let result = catch_panic_silently(|| assert_aria_invalid(&attrs, "false"));

        assert!(result.is_err(), "assert_aria_invalid should panic");
    }

    #[test]
    fn assert_aria_invalid_false_passes_when_false() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::Invalid), "false");

        assert_aria_invalid(&attrs, "false");
    }

    #[test]
    fn assert_aria_invalid_non_false_value_passes() {
        let attrs = attrs_with(HtmlAttr::Aria(AriaAttr::Invalid), "grammar");

        assert_aria_invalid(&attrs, "grammar");
    }

    #[test]
    fn assert_data_state_panics_when_missing() {
        let attrs = AttrMap::new();

        let result = catch_panic_silently(|| assert_data_state(&attrs, "idle"));

        let message = panic_message(result.expect_err("assert_data_state should panic"));

        assert_eq!(message, "expected data-ars-state but not found");
    }

    #[test]
    fn assert_tabindex_panics_when_missing() {
        let attrs = AttrMap::new();

        let result = catch_panic_silently(|| assert_tabindex(&attrs, 0));

        let message = panic_message(result.expect_err("assert_tabindex should panic"));

        assert_eq!(message, "expected tabindex but not found");
    }

    required_bool_helper_tests!(
        assert_aria_selected_true_passes,
        assert_aria_selected_false_passes,
        assert_aria_selected_panics_when_missing,
        assert_aria_selected,
        HtmlAttr::Aria(AriaAttr::Selected),
        "expected aria-selected but not found"
    );

    required_string_helper_tests!(
        assert_aria_label_passes,
        assert_aria_label_panics_when_missing,
        assert_aria_label,
        HtmlAttr::Aria(AriaAttr::Label),
        "Accordion trigger",
        "expected aria-label=\"Accordion trigger\" but not found"
    );

    required_string_helper_tests!(
        assert_aria_checked_passes,
        assert_aria_checked_panics_when_missing,
        assert_aria_checked,
        HtmlAttr::Aria(AriaAttr::Checked),
        "mixed",
        "expected aria-checked=\"mixed\" but not found"
    );

    required_string_helper_tests!(
        assert_aria_controls_passes,
        assert_aria_controls_panics_when_missing,
        assert_aria_controls,
        HtmlAttr::Aria(AriaAttr::Controls),
        "panel-1",
        "expected aria-controls=\"panel-1\" but not found"
    );

    required_string_helper_tests!(
        assert_aria_labelledby_passes,
        assert_aria_labelledby_panics_when_missing,
        assert_aria_labelledby,
        HtmlAttr::Aria(AriaAttr::LabelledBy),
        "trigger-1",
        "expected aria-labelledby=\"trigger-1\" but not found"
    );

    required_string_helper_tests!(
        assert_aria_describedby_passes,
        assert_aria_describedby_panics_when_missing,
        assert_aria_describedby,
        HtmlAttr::Aria(AriaAttr::DescribedBy),
        "hint-1",
        "expected aria-describedby=\"hint-1\" but not found"
    );

    required_string_helper_tests!(
        assert_aria_haspopup_passes,
        assert_aria_haspopup_panics_when_missing,
        assert_aria_haspopup,
        HtmlAttr::Aria(AriaAttr::HasPopup),
        "menu",
        "expected aria-haspopup=\"menu\" but not found"
    );

    required_string_helper_tests!(
        assert_aria_pressed_passes,
        assert_aria_pressed_panics_when_missing,
        assert_aria_pressed,
        HtmlAttr::Aria(AriaAttr::Pressed),
        "mixed",
        "expected aria-pressed=\"mixed\" but not found"
    );

    required_string_helper_tests!(
        assert_aria_activedescendant_passes,
        assert_aria_activedescendant_panics_when_missing,
        assert_aria_activedescendant,
        HtmlAttr::Aria(AriaAttr::ActiveDescendant),
        "item-3",
        "expected aria-activedescendant=\"item-3\" but not found"
    );

    assert_eq_string_helper_tests!(
        assert_aria_orientation_passes,
        assert_aria_orientation_panics_when_missing,
        assert_aria_orientation,
        HtmlAttr::Aria(AriaAttr::Orientation),
        "horizontal",
        "expected aria-orientation=\"horizontal\""
    );

    assert_eq_string_helper_tests!(
        assert_aria_valuemin_passes,
        assert_aria_valuemin_panics_when_missing,
        assert_aria_valuemin,
        HtmlAttr::Aria(AriaAttr::ValueMin),
        "0",
        "expected aria-valuemin=\"0\""
    );

    assert_eq_string_helper_tests!(
        assert_aria_valuemax_passes,
        assert_aria_valuemax_panics_when_missing,
        assert_aria_valuemax,
        HtmlAttr::Aria(AriaAttr::ValueMax),
        "100",
        "expected aria-valuemax=\"100\""
    );

    assert_eq_string_helper_tests!(
        assert_aria_valuenow_passes,
        assert_aria_valuenow_panics_when_missing,
        assert_aria_valuenow,
        HtmlAttr::Aria(AriaAttr::ValueNow),
        "50",
        "expected aria-valuenow=\"50\""
    );

    assert_eq_string_helper_tests!(
        assert_aria_valuetext_passes,
        assert_aria_valuetext_panics_when_missing,
        assert_aria_valuetext,
        HtmlAttr::Aria(AriaAttr::ValueText),
        "50%",
        "expected aria-valuetext=\"50%\""
    );

    assert_eq_string_helper_tests!(
        assert_aria_live_passes,
        assert_aria_live_panics_when_missing,
        assert_aria_live,
        HtmlAttr::Aria(AriaAttr::Live),
        "polite",
        "expected aria-live=\"polite\""
    );

    assert_eq_string_helper_tests!(
        assert_aria_owns_passes,
        assert_aria_owns_panics_when_missing,
        assert_aria_owns,
        HtmlAttr::Aria(AriaAttr::Owns),
        "popup-1",
        "expected aria-owns=\"popup-1\""
    );

    required_string_helper_tests!(
        assert_aria_autocomplete_passes,
        assert_aria_autocomplete_panics_when_missing,
        assert_aria_autocomplete,
        HtmlAttr::Aria(AriaAttr::AutoComplete),
        "list",
        "aria-autocomplete must be present"
    );

    required_string_helper_tests!(
        assert_aria_errormessage_passes,
        assert_aria_errormessage_panics_when_missing,
        assert_aria_errormessage,
        HtmlAttr::Aria(AriaAttr::ErrorMessage),
        "error-1",
        "aria-errormessage must be present"
    );

    required_string_helper_tests!(
        assert_aria_roledescription_passes,
        assert_aria_roledescription_panics_when_missing,
        assert_aria_roledescription,
        HtmlAttr::Aria(AriaAttr::RoleDescription),
        "custom grid",
        "aria-roledescription must be present"
    );

    required_string_helper_tests!(
        assert_aria_current_passes,
        assert_aria_current_panics_when_missing,
        assert_aria_current,
        HtmlAttr::Aria(AriaAttr::Current),
        "page",
        "aria-current must be present"
    );

    required_string_helper_tests!(
        assert_aria_sort_passes,
        assert_aria_sort_panics_when_missing,
        assert_aria_sort,
        HtmlAttr::Aria(AriaAttr::Sort),
        "ascending",
        "aria-sort must be present"
    );

    integer_helper_tests!(
        assert_aria_setsize_parses_integer_value,
        assert_aria_setsize_panics_when_missing,
        assert_aria_setsize_panics_for_invalid_value,
        assert_aria_setsize,
        HtmlAttr::Aria(AriaAttr::SetSize),
        5,
        "aria-setsize must be present",
        "aria-setsize must be a valid integer"
    );

    integer_helper_tests!(
        assert_aria_posinset_parses_integer_value,
        assert_aria_posinset_panics_when_missing,
        assert_aria_posinset_panics_for_invalid_value,
        assert_aria_posinset,
        HtmlAttr::Aria(AriaAttr::PosInSet),
        3,
        "aria-posinset must be present",
        "aria-posinset must be a valid integer"
    );

    integer_helper_tests!(
        assert_aria_level_parses_integer_value,
        assert_aria_level_panics_when_missing,
        assert_aria_level_panics_for_invalid_value,
        assert_aria_level,
        HtmlAttr::Aria(AriaAttr::Level),
        3,
        "aria-level must be present",
        "aria-level must be a valid integer"
    );

    integer_helper_tests!(
        assert_aria_rowindex_parses_integer_value,
        assert_aria_rowindex_panics_when_missing,
        assert_aria_rowindex_panics_for_invalid_value,
        assert_aria_rowindex,
        HtmlAttr::Aria(AriaAttr::RowIndex),
        2,
        "aria-rowindex must be present",
        "aria-rowindex must be a valid integer"
    );

    integer_helper_tests!(
        assert_aria_colindex_parses_integer_value,
        assert_aria_colindex_panics_when_missing,
        assert_aria_colindex_panics_for_invalid_value,
        assert_aria_colindex,
        HtmlAttr::Aria(AriaAttr::ColIndex),
        4,
        "aria-colindex must be present",
        "aria-colindex must be a valid integer"
    );

    integer_helper_tests!(
        assert_aria_rowcount_parses_integer_value,
        assert_aria_rowcount_panics_when_missing,
        assert_aria_rowcount_panics_for_invalid_value,
        assert_aria_rowcount,
        HtmlAttr::Aria(AriaAttr::RowCount),
        7,
        "aria-rowcount must be present",
        "aria-rowcount must be a valid integer"
    );

    integer_helper_tests!(
        assert_aria_colcount_parses_integer_value,
        assert_aria_colcount_panics_when_missing,
        assert_aria_colcount_panics_for_invalid_value,
        assert_aria_colcount,
        HtmlAttr::Aria(AriaAttr::ColCount),
        8,
        "aria-colcount must be present",
        "aria-colcount must be a valid integer"
    );

    optional_false_helper_rejects_true_tests!(
        assert_aria_disabled_panics_when_false_expected_but_true_present,
        assert_aria_disabled,
        HtmlAttr::Aria(AriaAttr::Disabled)
    );

    optional_false_helper_rejects_true_tests!(
        assert_aria_multiselectable_panics_when_false_expected_but_true_present,
        assert_aria_multiselectable,
        HtmlAttr::Aria(AriaAttr::MultiSelectable)
    );

    optional_false_helper_rejects_true_tests!(
        assert_aria_required_panics_when_false_expected_but_true_present,
        assert_aria_required,
        HtmlAttr::Aria(AriaAttr::Required)
    );

    optional_false_helper_rejects_true_tests!(
        assert_aria_atomic_panics_when_false_expected_but_true_present,
        assert_aria_atomic,
        HtmlAttr::Aria(AriaAttr::Atomic)
    );

    optional_false_helper_rejects_true_tests!(
        assert_aria_hidden_panics_when_false_expected_but_true_present,
        assert_aria_hidden,
        HtmlAttr::Aria(AriaAttr::Hidden)
    );

    optional_false_helper_rejects_true_tests!(
        assert_aria_modal_panics_when_false_expected_but_true_present,
        assert_aria_modal,
        HtmlAttr::Aria(AriaAttr::Modal)
    );

    optional_false_helper_rejects_true_tests!(
        assert_aria_readonly_panics_when_false_expected_but_true_present,
        assert_aria_readonly,
        HtmlAttr::Aria(AriaAttr::ReadOnly)
    );

    optional_false_helper_tests!(
        assert_aria_busy_true_passes_when_true,
        assert_aria_busy_false_passes_when_absent,
        assert_aria_busy_false_passes_when_false,
        assert_aria_busy,
        HtmlAttr::Aria(AriaAttr::Busy)
    );

    optional_false_helper_tests!(
        assert_aria_multiselectable_true_passes_when_true,
        assert_aria_multiselectable_false_passes_when_absent,
        assert_aria_multiselectable_false_passes_when_false,
        assert_aria_multiselectable,
        HtmlAttr::Aria(AriaAttr::MultiSelectable)
    );

    optional_false_helper_tests!(
        assert_aria_required_true_passes_when_true,
        assert_aria_required_false_passes_when_absent,
        assert_aria_required_false_passes_when_false,
        assert_aria_required,
        HtmlAttr::Aria(AriaAttr::Required)
    );

    optional_false_helper_tests!(
        assert_aria_atomic_true_passes_when_true,
        assert_aria_atomic_false_passes_when_absent,
        assert_aria_atomic_false_passes_when_false,
        assert_aria_atomic,
        HtmlAttr::Aria(AriaAttr::Atomic)
    );

    optional_false_helper_tests!(
        assert_aria_hidden_true_passes_when_true,
        assert_aria_hidden_false_passes_when_absent,
        assert_aria_hidden_false_passes_when_false,
        assert_aria_hidden,
        HtmlAttr::Aria(AriaAttr::Hidden)
    );

    optional_false_helper_tests!(
        assert_aria_modal_true_passes_when_true,
        assert_aria_modal_false_passes_when_absent,
        assert_aria_modal_false_passes_when_false,
        assert_aria_modal,
        HtmlAttr::Aria(AriaAttr::Modal)
    );

    optional_false_helper_tests!(
        assert_aria_readonly_true_passes_when_true,
        assert_aria_readonly_false_passes_when_absent,
        assert_aria_readonly_false_passes_when_false,
        assert_aria_readonly,
        HtmlAttr::Aria(AriaAttr::ReadOnly)
    );
}
