//! Testing helpers for validating ARIA output from connect surfaces.

mod asserts;
mod keyboard;
mod validator;

pub use asserts::{
    assert_aria_activedescendant, assert_aria_atomic, assert_aria_autocomplete, assert_aria_busy,
    assert_aria_checked, assert_aria_colcount, assert_aria_colindex, assert_aria_controls,
    assert_aria_current, assert_aria_describedby, assert_aria_disabled, assert_aria_errormessage,
    assert_aria_expanded, assert_aria_haspopup, assert_aria_hidden, assert_aria_invalid,
    assert_aria_label, assert_aria_labelledby, assert_aria_level, assert_aria_live,
    assert_aria_modal, assert_aria_multiselectable, assert_aria_orientation, assert_aria_owns,
    assert_aria_posinset, assert_aria_pressed, assert_aria_readonly, assert_aria_required,
    assert_aria_roledescription, assert_aria_rowcount, assert_aria_rowindex, assert_aria_selected,
    assert_aria_setsize, assert_aria_sort, assert_aria_valuemax, assert_aria_valuemin,
    assert_aria_valuenow, assert_aria_valuetext, assert_data_state, assert_role, assert_tabindex,
};
pub use keyboard::{FocusZoneTestHarness, NavigationEvent, NavigationRecorder, SimulatedKeyEvent};
pub use validator::{
    AriaValidationContext, AriaValidationError, AriaValidationWarning, AriaValidator,
    required_attributes_for_role, validate_attr_map,
};
