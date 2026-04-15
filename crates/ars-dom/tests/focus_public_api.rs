//! Public API coverage for `ars_dom::focus`.

use ars_dom::focus::{FOCUSABLE_SELECTOR, TABBABLE_SELECTOR, get_tabbable_elements_selector};

#[test]
fn focus_selectors_are_publicly_accessible() {
    assert!(FOCUSABLE_SELECTOR.contains("button:not([disabled])"));
    assert!(FOCUSABLE_SELECTOR.contains("[tabindex]:not([disabled])"));
    assert!(!FOCUSABLE_SELECTOR.contains("[tabindex]:not([tabindex='-1'])"));
    assert!(TABBABLE_SELECTOR.contains("[tabindex]:not([tabindex='-1'])"));
    assert_eq!(get_tabbable_elements_selector(), TABBABLE_SELECTOR);
}
