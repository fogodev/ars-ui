//! Typed ARIA attributes, roles, and screen-reader support helpers.
//!
//! This crate provides the accessibility building blocks used by all ars-ui components:
//! typed WAI-ARIA roles and attributes, focus helpers, and live-announcement support.

#![no_std]
#![warn(clippy::std_instead_of_core)]

extern crate alloc;

pub mod announcements;
pub mod announcer;
pub mod aria;
/// Shared focus management contracts consumed by DOM and adapter layers.
pub mod focus;
/// Keyboard shortcut descriptors and platform-normalized modifier matching.
pub mod keyboard;
/// Field labelling, descriptions, and error wiring helpers for form controls.
pub mod label;
/// Testing helpers for ARIA validation, attribute assertions, and keyboard navigation.
#[cfg(any(test, feature = "testing"))]
pub mod testing;
/// Touch-target sizing and mobile accessibility helpers.
pub mod touch;

pub use announcements::Announcements;
pub use announcer::{Announcement, AnnouncementPriority, LiveAnnouncer};
#[cfg(feature = "aria-drag-drop-compat")]
pub use aria::attribute::AriaDropeffect;
pub use aria::{
    apply::{apply_aria, apply_role},
    attribute::{
        AriaAttribute, AriaAutocomplete, AriaChecked, AriaCurrent, AriaHasPopup, AriaIdList,
        AriaIdRef, AriaInvalid, AriaLive, AriaOrientation, AriaPressed, AriaRelevant, AriaSort,
    },
    role::AriaRole,
    state::{
        set_busy, set_checked, set_disabled, set_expanded, set_invalid, set_readonly, set_selected,
    },
};
pub use focus::{
    FocusRing, FocusScopeBehavior, FocusScopeOptions, FocusStrategy, FocusTarget, FocusZone,
    FocusZoneDirection, FocusZoneOptions,
};
pub use keyboard::{DomEvent, KeyModifiers, KeyboardShortcut, Platform};
pub use label::{DescriptionConfig, FieldContext, LabelConfig};
#[cfg(any(test, feature = "testing"))]
pub use testing::{
    AriaValidationContext, AriaValidationError, AriaValidationWarning, AriaValidator,
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
    extract_all_ids, required_attributes_for_role, validate_attr_map,
};
pub use touch::{
    InputMode, MIN_DRAG_TARGET_SIZE, MIN_TOUCH_TARGET_SIZE, should_use_roving_tabindex_for_mobile,
    touch_target_attrs, touch_target_attrs_with_min,
};

/// Custom data attribute used to expose machine state on the root DOM element.
///
/// Components set `data-ars-state` to the current state name, enabling CSS selectors
/// like `[data-ars-state="open"]` for styling and test assertions.
pub const DATA_ARS_STATE: &str = "data-ars-state";

/// Custom data attribute used to expose readonly state on rendered DOM elements.
///
/// Components set `data-ars-readonly` as a presence attribute alongside
/// `aria-readonly="true"` for styling hooks and test assertions.
pub const DATA_ARS_READONLY: &str = "data-ars-readonly";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aria_role_clone_and_equality() {
        let role = AriaRole::Button;
        #[expect(clippy::clone_on_copy, reason = "deliberately testing Clone impl")]
        let cloned = role.clone();
        assert_eq!(role, cloned);
        assert_ne!(AriaRole::Button, AriaRole::Dialog);
    }

    #[test]
    fn aria_attribute_clone_and_equality() {
        let attr = AriaAttribute::Disabled(true);
        let cloned = attr.clone();
        assert_eq!(attr, cloned);
        assert_ne!(
            AriaAttribute::Disabled(true),
            AriaAttribute::Disabled(false)
        );
    }

    #[test]
    fn data_ars_state_constant_value() {
        assert_eq!(DATA_ARS_STATE, "data-ars-state");
    }

    #[test]
    fn data_ars_readonly_constant_value() {
        assert_eq!(DATA_ARS_READONLY, "data-ars-readonly");
    }

    #[test]
    fn announcements_messages_are_available_via_module_path() {
        fn assert_component_messages<M: ars_core::ComponentMessages + Clone + Default>(
            messages: &M,
        ) -> M {
            messages.clone()
        }

        let messages = announcements::Messages::default();
        let cloned = assert_component_messages(&messages);
        let locale = ars_core::Locale::parse("en-US").expect("test locale must parse");

        assert_eq!((cloned.loading)(&locale), "Loading.");
        assert_eq!(Announcements::loading(&locale, &messages), "Loading.");
    }
}
